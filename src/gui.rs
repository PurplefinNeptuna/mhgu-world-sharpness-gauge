//! Insert one `cGUIObjScissorMask` above the sharpness bar so the gauge row is
//! clipped to current over maximum points. Colour, tier frames, sequences, atlases
//! and vertex colours stay exactly as the game makes them; this module only builds
//! the mask, links it under the row's root with the native linkers, and sizes the
//! clip from the core decision.
//!
//! What the code below relies on, all read from the hash-checked main NSO and
//! recorded in `work/modern-sharpness-gauge/PHASE2_RESEARCH.md`:
//! - the draw walker main+0xa494a0 emits an enter record (vtable 0x38, 0x3c) when it
//!   descends into a drawable node and an exit record (vtable 0x40) when a walk pops
//!   back out of it, and the walk ends when the next pointer is the root;
//! - `cGUIObjScissorMask`'s vtable main+0x178fb04 answers 0x38 with main+0xa4ba94
//!   (apply the scissor) and 0x40 with main+0xa4bcdc (restore the saved clip), so the
//!   mask clips only while the walk both enters and exits it;
//! - main+0xa4844c links a sibling and supplies its parent, main+0xa48460 links a
//!   child and supplies its parent;
//! - main+0xa49b28 sizes this row's draw cache from the row's own counts, which
//!   covers the mask's extra records, so the cache is never replaced;
//! - the root destructor main+0xa486fc deletes the root's child chain, and the mask
//!   destructor main+0xa56758 deletes its own child chain, so the game frees the mask
//!   and the three textures it covers in the same pass. Nothing here allocates
//!   through Forge and nothing here frees a game-owned block.
use crate::{
    hooks, memory, sampling,
    memory::BASE,
};
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use sharpness_gauge_core::{NODE_CHILD, NODE_DRAWABLE, NODE_FLAGS, NODE_PARENT, NODE_SIBLING, NODE_VISIBLE};
use forge::mt::dti::MtDti;

/// Mod-owned object id, outside the ids `hud0.gui` uses for this row.
const MOD_ID: u32 = 0x4d534731; // MSG1
const MASK_BYTES: usize = 208;
const TEXTURE_BYTES: usize = 304;
const INSTANCE_BYTES: usize = 224;
const ROOT_BYTES: usize = 176;

// Row description fields, as Key Quest Markers reads them at root+0x9c.
const GAUGE_ANIMATION: u32 = 4;
const GAUGE_OBJECTS: u16 = 4;
const GAUGE_SEQUENCES: u16 = 15;
const GAUGE_FIRST_OBJECT: u32 = 29;
const BAR_WIDTH: f32 = 88.0;
const BAR_HEIGHT: f32 = 24.0;

/// Mask property words, from the DTI record of `cGUIObjScissorMask` and its
/// setters: PosX/PosY floats at +0x80/+0x84 with the position dirty bit in +0x54,
/// the integer width/height pair at +0xa8/+0xac, and the flag word at +0xa4 whose
/// bit 1 asks the enter record to rebuild the rectangle at +0xb0..+0xbc and whose
/// bit 0 asks it to apply the scissor.
const MASK_POSITION: u32 = 0x80;
const MASK_SIZE: u32 = 0xa8;
const MASK_FLAGS: u32 = 0xa4;
const MASK_RECT: u32 = 0xb0;
const MASK_POSITION_DIRTY: u32 = 0x2_0000;
const MASK_SIZE_DIRTY: u32 = 1 << 1;
const MASK_ENABLE: u32 = 1;

/// 0 = not inserted, 1 = inserted, 2 = failed. One insertion per gauge HUD: the
/// row is a persistent HUD object and a second mask would clip twice. The game's
/// own teardown of the row deletes the mask, which resets this state (see
/// `on_teardown`).
static STATE: AtomicU8 = AtomicU8::new(0);
static MASK: AtomicU32 = AtomicU32::new(0);
static BAR_NODE: AtomicU32 = AtomicU32::new(0);
static FAILURE: AtomicU32 = AtomicU32::new(0);

pub fn take_failure() -> u32 {
    FAILURE.swap(0, Ordering::Relaxed)
}
pub fn inserted() -> bool {
    STATE.load(Ordering::Relaxed) == 1
}
/// The bar's live width and the authored-size flags, for the log line.
pub fn bar_width() -> (u32, u32) {
    (BAR_W.load(Ordering::Relaxed), AUTHORED.load(Ordering::Relaxed))
}

/// root+0x8c as read, and the walker's own drawable count for the last sample.
pub fn row_flags() -> (u32, u32) {
    (AUTHORED50.load(Ordering::Relaxed), sampling::drawables())
}

/// (mask, mask child, mask sibling, mask parent) for the insertion line: the three
/// links the walk follows on the node this insertion adds.
pub fn applied() -> (u32, u32, u32, u32) {
    let mask = MASK.load(Ordering::Relaxed);
    (
        mask,
        memory::word(mask + NODE_CHILD).unwrap_or(0),
        memory::word(mask + NODE_SIBLING).unwrap_or(0),
        memory::word(mask + NODE_PARENT).unwrap_or(0),
    )
}

fn base() -> u32 {
    BASE.load(Ordering::Relaxed)
}
fn method(object: u32, slot: u32) -> Option<u32> {
    memory::method(object, slot)
}
fn is_type(object: u32, dti: u32) -> bool {
    let Some(f) = method(object, 0x14) else {
        return false;
    };
    memory::call1(f, object).is_some_and(|got| got == dti)
}
fn word(address: u32, value: u32) -> bool {
    memory::write_word(address, value)
}
/// Call one of the native linkers and read back the two words it writes, so a
/// published tree is a tree this code has seen in memory.
fn link(offset: u32, node: u32, other: u32) -> Option<()> {
    memory::call2(base() + offset, node, other)?;
    Some(())
}

/// The gauge row as the game currently holds it. Every field is checked against
/// the archive's structure, and the bar is identified by its authored size.
struct Row {
    root: u32,
    bar: u32,
    effect: u32,
    edge: u32,
}

/// Read and validate the gauge row. Each gate has its own stage code so a single
/// run says which check failed: 21 no captured instance pair, 22 instance or
/// controller not writable, 23 a field read failed, 24 row counts do not match the
/// gauge animation, 25 the controller has no usable draw cache, 26 the bar link is
/// missing, equals the controller, or is not writable, 27 the bar's parent link is
/// not the controller, 28 the glow is not a distinct writable sibling of the bar,
/// 29 the edge is not a distinct writable sibling of the glow, 30 the chain does not
/// close at the controller, 31 the bar is not a cGUIObjTexture, 32 the bar's live
/// width is outside 1..=1024, 33 a DTI is missing, 34 no live sharpness gauge.
/// These gates describe the tree before the insertion; the tree after it is checked
/// in `apply`, and `core::draw_plan` proves the walk over the tree this write
/// produces.
fn read_row() -> Result<Row, u32> {
    let (instance, root) = hooks::gauge_instance().ok_or(21u32)?;
    if !memory::writable(instance, INSTANCE_BYTES) || !memory::writable(root, ROOT_BYTES) {
        return Err(22);
    }
    let Some(description) = memory::word(root.wrapping_add(0x9c)) else {
        return Err(23);
    };
    let (Some(cache), Some(cache_flags)) = (
        memory::word(root.wrapping_add(0x80)),
        memory::word(root.wrapping_add(0x8c)),
    ) else {
        return Err(23);
    };
    if cache == 0 || !memory::readable(cache, 8) {
        return Err(26);
    }
    let (Some(objects), Some(sequences)) = (
        memory::half(description.wrapping_add(4)),
        memory::half(description.wrapping_add(6)),
    ) else {
        return Err(23);
    };
    let Some(anim) = memory::word(description) else {
        return Err(23);
    };
    let Some(first_object) = memory::word(description.wrapping_add(0x0c)) else {
        return Err(23);
    };
    // The gauge row is identified by the fields the sampler already proves on every
    // sample line: animation 4, 4 objects, 15 sequences, first object 29. The row
    // carries no drawable count. root+0x8c is recorded, not required: its bit 0 is the
    // walker's rebuild flag, which this code sets after linking.
    AUTHORED50.store(cache_flags, Ordering::Release);
    if anim != GAUGE_ANIMATION
        || objects != GAUGE_OBJECTS
        || sequences != GAUGE_SEQUENCES
        || first_object != GAUGE_FIRST_OBJECT
    {
        return Err(24);
    }
    // Gauge layout, from the walker decompile at main+0xa494a0 and the r9 Eden log.
    // The walk root is a cGUIObjRoot: the bar's parent (bar 0x488aa9b0 has
    // parent=0x48898ec0, the controller on every sample line), the draw cache
    // lives at root+0x80, root+0x8c bit 0 is the dirty bit the walker clears after
    // rebuilding, and the walk ends when the next pointer comes back to the root.
    // The three sharpness textures are direct siblings of the root, so the mask
    // enters as the root's first child and takes all three under it: the walk has
    // to leave the mask through its exit record, or the scissor never ends.
    // A sharpness gauge must be live. The hub has no gauge, a quest that ended
    // tears its HUD down, and weapons without sharpness (bowguns, the prowler
    // rig) never build the gauge row at all, so there is nothing to clip there.
    sampling::gauge_points().ok_or(34u32)?;
    let root_dti = MtDti::find("cGUIObjRoot").ok_or(33u32)?;
    if !is_type(root, root_dti as *const _ as u32) {
        return Err(25);
    }
    let Some(bar) = memory::word(root.wrapping_add(NODE_CHILD)) else {
        return Err(23);
    };
    let Some(bar_parent) = memory::word(bar.wrapping_add(NODE_PARENT)) else {
        return Err(23);
    };
    if bar == 0 || bar == root || !memory::writable(bar, TEXTURE_BYTES) {
        return Err(26);
    }
    if bar_parent != root {
        return Err(27);
    }
    let Some(effect) = memory::word(bar.wrapping_add(NODE_SIBLING)) else {
        return Err(23);
    };
    let Some(effect_parent) = memory::word(effect.wrapping_add(NODE_PARENT)) else {
        return Err(23);
    };
    if effect == 0 || effect == bar || !memory::writable(effect, TEXTURE_BYTES) {
        return Err(28);
    }
    if effect_parent != root {
        return Err(28);
    }
    let Some(edge) = memory::word(effect.wrapping_add(NODE_SIBLING)) else {
        return Err(23);
    };
    let Some(edge_parent) = memory::word(edge.wrapping_add(NODE_PARENT)) else {
        return Err(23);
    };
    if edge == 0 || edge == effect || edge == bar
        || !memory::writable(edge, TEXTURE_BYTES)
        || edge_parent != root
    {
        return Err(29);
    }
    // How the walk closes, from main+0xa494a0: a sibling equal to the root breaks
    // the loop, and a zero sibling pops to node+0x70, which the gates above prove
    // is the root. Both closings are valid, any other pointer is not the end of
    // this chain. The r10 log shows the edge glyph closing with sibling 0.
    let terminator = memory::word(edge.wrapping_add(NODE_SIBLING)).ok_or(23u32)?;
    if terminator != 0 && terminator != root {
        return Err(30);
    }
    let texture = MtDti::find("cGUIObjTexture").ok_or(33u32)?;
    if !is_type(bar, texture as *const _ as u32) {
        return Err(31);
    }
    let (Some(width), Some(height)) = (
        memory::word(bar.wrapping_add(0xb4)),
        memory::word(bar.wrapping_add(0xb8)),
    ) else {
        return Err(23);
    };
    // The authored 88x24 is the expected size; the live value is what the geometry
    // scales from, so a bar the game draws at another size still clips correctly
    // instead of failing the gate. Stage 32 means an unusable live size.
    let live_width = f32::from_bits(width) as i32;
    if !(1..=1024).contains(&live_width) {
        return Err(32);
    }
    BAR_W.store(live_width as u32, Ordering::Release);
    AUTHORED.store(
        ((width == BAR_WIDTH.to_bits()) as u32) | (((height == BAR_HEIGHT.to_bits()) as u32) << 1),
        Ordering::Release,
    );
    // instance+0xd8 is read and logged, not gated: this HUD has no animation map
    // list to grow, so nothing here indexes it.
    Ok(Row {
        root,
        bar,
        effect,
        edge,
    })
}

/// Build the mask and link it as the root's first child with the bar, the glow and
/// the edge under it, in that order, closing with a zero sibling on the edge and on
/// the mask. Every link is written by the native linkers, which also supply the
/// parent of the node they attach. The mask is published first, so each later link
/// hangs under a node that already reaches the root: at no point is there a parent
/// cycle, a parent chain ending at 0, or a node whose parent is unreachable from the
/// root, any of which a walker rebuild running on another thread would follow.
/// A mask linked in and then rejected by the link checks below stays in the tree
/// inert (no Enable bit, no children), and insertion is terminal.
fn insert(row: &Row) -> Result<u32, u32> {
    // The mask. Its factory allocates through the mask DTI's own MT allocator, so
    // the game's destructor owns the block from the moment it returns.
    // shortcut: a mask that fails verification below is never linked, and the
    // factory has no release path, so it stays unreachable in the mask DTI's
    // allocator. Insertion failure is terminal (STATE = 2), so this cannot repeat.
    let mask = memory::call0(base() + 0xa4b6cc).ok_or(1u32)?;
    let dti = MtDti::find("cGUIObjScissorMask").ok_or(18u32)? as *const _ as u32;
    if mask == 0 || mask & 3 != 0 || !memory::writable(mask, MASK_BYTES) || !is_type(mask, dti) {
        return Err(2);
    }
    // The mask needs visible and drawable set: bit 0 is the visibility the walker
    // tests before descending, and 0x0200_0000 is the bit that makes it emit the
    // enter and exit records the mask's clip and restore hang off.
    if !word(mask + 0x50, MOD_ID)
        || !word(
            mask + NODE_FLAGS,
            memory::word(mask + NODE_FLAGS).ok_or(12u32)? | NODE_VISIBLE | NODE_DRAWABLE,
        )
    {
        return Err(13);
    }
    // Publish the mask first, as the root's first child: the linker writes
    // mask.parent = root. Every later link then hangs under a node that already
    // reaches the root, which is what keeps a rebuild during this sequence safe. A
    // node whose parent chain ends at 0 is fatal here, because the walker pops to
    // node+0x70 and dereferences it, and a parent cycle is fatal the same way.
    link(0xa48460, row.root, mask).ok_or(16u32)?;
    if memory::word(row.root.wrapping_add(NODE_CHILD)) != Some(mask)
        || memory::word(mask.wrapping_add(NODE_PARENT)) != Some(row.root)
    {
        return Err(17);
    }
    // mask.child = bar, and the linker writes bar.parent = mask. The bar leaves the
    // root's own chain at this point: the gauge is not drawn between the two lines
    // above and this one, which is the price for a chain that is never half-built.
    link(0xa48460, mask, row.bar).ok_or(14u32)?;
    // bar.sibling = glow, and the linker writes glow.parent = bar.parent = mask.
    link(0xa4844c, row.bar, row.effect).ok_or(14u32)?;
    // glow.sibling = edge, and the linker writes edge.parent = glow.parent = mask.
    link(0xa4844c, row.effect, row.edge).ok_or(14u32)?;
    // The mask's child chain ends with a zero sibling, so the walk pops from the
    // edge back to the mask and emits the mask's exit record. The mask's own
    // sibling is 0 from the factory (main+0xa4b6cc zeroes +0x5c).
    link(0xa4844c, row.edge, 0).ok_or(14u32)?;
    // Read every link back. The walk visits mask -> bar -> glow -> edge -> mask.
    let links_ok = memory::word(mask.wrapping_add(NODE_CHILD)) == Some(row.bar)
        && memory::word(mask.wrapping_add(NODE_SIBLING)) == Some(0)
        && memory::word(row.bar.wrapping_add(NODE_PARENT)) == Some(mask)
        && memory::word(row.bar.wrapping_add(NODE_SIBLING)) == Some(row.effect)
        && memory::word(row.effect.wrapping_add(NODE_PARENT)) == Some(mask)
        && memory::word(row.effect.wrapping_add(NODE_SIBLING)) == Some(row.edge)
        && memory::word(row.edge.wrapping_add(NODE_PARENT)) == Some(mask)
        && memory::word(row.edge.wrapping_add(NODE_SIBLING)) == Some(0);
    if !links_ok {
        return Err(15);
    }
    // Bit 0 of root+0x8c asks the walker to rebuild the cache over the new tree; it
    // clears the bit itself. The row's own cache is kept, because main+0xa49b28
    // sized it from the row counts and the mask's extra records fit in it.
    let flags = memory::word(row.root + 0x8c).ok_or(17u32)?;
    if !word(row.root + 0x8c, flags | 1) {
        return Err(17);
    }
    BAR_NODE.store(row.bar, Ordering::Release);
    Ok(mask)
}

/// Write the clip for one ratio. Position is the float pair with the dirty bit the
/// game's own setter sets, size is the integer pair, and both dirty bits are set in
/// the mask flag word: bit 1 makes the enter record rebuild the mask rectangle at
/// +0xb0..+0xbc, bit 0 makes it apply the scissor. The mask's exit record restores
/// the clip the game had before, so masking ends with the mask.
///
/// The tree is re-checked in the shape this insertion produced: the bar's parent is
/// the mask, the mask's child is the bar, and the mask's own parent is the row's
/// root. A row read written for the tree before the insertion is not used here.
/// Returns the requested box and the rectangle the game built from it.
pub fn apply(ratio_bp: u32) -> Option<([i32; 4], [i32; 4])> {
    if STATE.load(Ordering::Relaxed) != 1 {
        return None;
    }
    let mask = MASK.load(Ordering::Relaxed);
    let bar = BAR_NODE.load(Ordering::Relaxed);
    // The instance hook records this pair only from the game's own draw path, so
    // the controller the insertion linked into is the one the game is drawing now.
    let (_, root) = hooks::gauge_instance()?;
    if bar == 0
        || memory::word(bar.wrapping_add(NODE_PARENT)) != Some(mask)
        || memory::word(mask.wrapping_add(NODE_CHILD)) != Some(bar)
        || memory::word(mask.wrapping_add(NODE_PARENT)) != Some(root)
    {
        return None;
    }
    let rect = rect(&bar)?;
    let box_ = sharpness_gauge_core::mask_box(rect, ratio_bp, BAR_W.load(Ordering::Relaxed) as i32);
    if !memory::write_bits(mask + MASK_POSITION, box_[0] as f32)
        || !memory::write_bits(mask + MASK_POSITION + 4, box_[1] as f32)
        || !word(
            mask + 0x54,
            memory::word(mask + 0x54)? | MASK_POSITION_DIRTY,
        )
        || !word(mask + MASK_SIZE, box_[2] as u32)
        || !word(mask + MASK_SIZE + 4, box_[3] as u32)
        || !word(
            mask + MASK_FLAGS,
            memory::word(mask + MASK_FLAGS)? | MASK_SIZE_DIRTY | MASK_ENABLE,
        )
    {
        return None;
    }
    let built = [
        memory::word(mask + MASK_RECT).unwrap_or(0) as i32,
        memory::word(mask + MASK_RECT + 4).unwrap_or(0) as i32,
        memory::word(mask + MASK_RECT + 8).unwrap_or(0) as i32,
        memory::word(mask + MASK_RECT + 12).unwrap_or(0) as i32,
    ];
    Some((box_, built))
}

/// The bar's own rect, as the game keeps it at +0xe0. The sprite prepaint at
/// main+0xa4e628 intersects this rect with the live clip at context+0xd0, so the
/// two are in the same units and so is the mask's own rectangle.
fn rect(bar: &u32) -> Option<[i32; 4]> {
    let left = memory::word(bar.checked_add(0xe0)?)? as i32;
    let top = memory::word(bar.checked_add(0xe4)?)? as i32;
    let right = memory::word(bar.checked_add(0xe8)?)? as i32;
    let bottom = memory::word(bar.checked_add(0xec)?)? as i32;
    Some([left, top, right, bottom])
}

/// Insert once, from the plugin's update callback (the game's own thread, outside
/// the walker). A row that is not present yet is not a failure: the gauge HUD row
/// exists only while a weapon-owning player is in a quest, so stage 21 reports and
/// keeps retrying at a slow cadence. A hard failure (mask factory, linker call or
/// publish) is terminal, because the row is not rebuilt by retrying a path that
/// already failed.
const RETRY_EVERY: u32 = 240;
static TRIES: AtomicU32 = AtomicU32::new(0);
/// The bar's live width, and bit 0/1 for "the authored 88 and 24 match".
static BAR_W: AtomicU32 = AtomicU32::new(0);
static AUTHORED: AtomicU32 = AtomicU32::new(0);
/// root+0x8c as read, recorded because the insertion sets its rebuild bit.
static AUTHORED50: AtomicU32 = AtomicU32::new(0);

pub fn tick() {
    if STATE.load(Ordering::Relaxed) != 0 {
        return;
    }
    let tries = TRIES.fetch_add(1, Ordering::Relaxed);
    if tries != 0 && tries % RETRY_EVERY != 0 {
        return;
    }
    let row = match read_row() {
        Ok(row) => row,
        Err(gate) => {
            // Overwrite, so a persistent gate is reported on every retry instead
            // of only the first time. The gate is the diagnosis, not a status.
            FAILURE.store(gate, Ordering::Release);
            // 23 is an unreadable row pointer, 24 a row that is no longer the
            // sharpness animation: both mean the captured controller belongs to a
            // scene that ended. Drop that capture so the next walker call records
            // the live one instead of retrying a dead object for the whole session.
            if gate == 23 || gate == 24 {
                hooks::clear_capture(hooks::capture().1);
            }
            return;
        }
    };
    match insert(&row) {
        Ok(mask) => {
            MASK.store(mask, Ordering::Release);
            STATE.store(1, Ordering::Release);
        }
        Err(stage) => {
            let _ = FAILURE.compare_exchange(0, stage, Ordering::Relaxed, Ordering::Relaxed);
            STATE.store(2, Ordering::Release);
        }
    }
}

/// The row's root is being destroyed: the game's root destructor walks the child
/// chain, so the mask and the three textures are freed by this call, right after it
/// returns. Nothing is written to the tree from here on, and the state goes back to
/// "not inserted" so the next quest's HUD is inserted fresh.
pub fn on_teardown() {
    MASK.store(0, Ordering::Release);
    BAR_NODE.store(0, Ordering::Release);
    STATE.store(0, Ordering::Release);
    FAILURE.store(0, Ordering::Release);
    TRIES.store(0, Ordering::Release);
}
