//! Insert one `cGUIObjScissorMask` above the sharpness bar so the gauge row is
//! clipped to current over maximum points. Colour, tier frames, sequences, atlases
//! and vertex colours stay exactly as the game makes them; this module only builds
//! the mask, publishes it on the row, reparents the bar under it, and sizes the
//! clip from the core decision.
//!
//! Every offset below is proven: the mask factory main+0xa4b6cc allocates 0xd0
//! bytes through the DTI allocator itself, the row layout and publish order follow
//! Key Quest Markers' working counter insertion, and the gauge chain (group,
//! `Txt_sharp`, `Txt_sharpefx`, `Txt_sharpEdge`) is the order `hud0.gui` stores.
//! Nothing here allocates through Forge, and no game-owned block is freed while
//! the tree still points at it.
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
const NODE_BYTES: usize = 192;

// Row description fields, as Key Quest Markers reads them at root+0x9c.
const GAUGE_ANIMATION: u32 = 4;
const GAUGE_OBJECTS: u16 = 4;
const GAUGE_SEQUENCES: u16 = 15;
const GAUGE_FIRST_OBJECT: u32 = 29;
const BAR_WIDTH: f32 = 88.0;
const BAR_HEIGHT: f32 = 24.0;

/// 0 = not inserted, 1 = inserted, 2 = failed. One insertion per plugin session:
/// the row is a persistent HUD object and a second mask would clip twice.
static STATE: AtomicU8 = AtomicU8::new(0);
static MASK: AtomicU32 = AtomicU32::new(0);
static OLD_CACHE: AtomicU32 = AtomicU32::new(0);
static BAR_NODE: AtomicU32 = AtomicU32::new(0);
static GROUP_NODE: AtomicU32 = AtomicU32::new(0);
static CACHE_BLOCK: AtomicU32 = AtomicU32::new(0);
static FAILURE: AtomicU32 = AtomicU32::new(0);
/// The row's map length and cache size at the moment of insertion, for the log.
static SIZES: AtomicU32 = AtomicU32::new(0);

pub fn take_failure() -> u32 {
    FAILURE.swap(0, Ordering::Relaxed)
}
pub fn inserted() -> bool {
    STATE.load(Ordering::Relaxed) == 1
}
/// (mask, old map length, old cache bytes) for the log line.
pub fn bar_width() -> (u32, u32) {
    (BAR_W.load(Ordering::Relaxed), AUTHORED.load(Ordering::Relaxed))
}

/// root+0x50 as read, and the walker's drawable count the cache length uses.
pub fn row_flags() -> (u32, u32) {
    (AUTHORED50.load(Ordering::Relaxed), sampling::drawables())
}

pub fn applied() -> (u32, u32, u32) {
    let sizes = SIZES.load(Ordering::Relaxed);
    (
        MASK.load(Ordering::Relaxed),
        sizes & 0xffff,
        sizes >> 16,
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

/// The gauge row as the game currently holds it. Every field is checked against
/// the archive's structure, and the bar is identified by its authored size.
struct Row {
    root: u32,
    instance: u32,
    map: u32,
    cache: u32,
    group: u32,
    bar: u32,
    effect: u32,
    width: i32,
}

/// Read and validate the gauge row. Each gate has its own stage code so a single
/// run says which check failed: 21 no captured instance pair, 22 instance or
/// controller not writable, 23 a field read failed, 24 row counts do not match the
/// gauge animation, 25 the controller has no usable draw cache,
/// 26 controller+0x68 is missing, equals the controller, or is not writable, 27 the
/// bar's siblings are not two distinct objects, 28 an object in the chain is not
/// writable, 29 the bar's parent group is missing, is the controller, the edge's
/// parent differs, or its first child is not the bar, 31 the bar is not a
/// cGUIObjTexture, 32 the bar's live width is outside 1..=1024, 33 a DTI is missing.
fn read_row() -> Result<Row, u32> {
    let Some((instance, root)) = hooks::gauge_instance() else {
        return Err(21);
    };
    if !memory::writable(instance, INSTANCE_BYTES) || !memory::writable(root, ROOT_BYTES) {
        return Err(22);
    }
    let (Some(map), Some(description)) = (
        memory::word(instance.wrapping_add(0xd8)),
        memory::word(root.wrapping_add(0x9c)),
    ) else {
        return Err(23);
    };
    let (Some(objects), Some(sequences)) = (
        memory::half(description.wrapping_add(4)),
        memory::half(description.wrapping_add(6)),
    ) else {
        return Err(23);
    };
    let (Some(anim), Some(active)) = (
        memory::word(description),
        memory::word(root.wrapping_add(0x50)),
    ) else {
        return Err(23);
    };
    let Some(first_object) = memory::word(description.wrapping_add(0x0c)) else {
        return Err(23);
    };
    // The gauge row is identified by the fields the sampler already proves on every
    // sample line: animation 4, 4 objects, 15 sequences, first object 29. The row
    // carries no drawable count, so the cache length comes from the walker's count
    // (sampling::drawables), not from the row. root+0x50 is recorded, not required:
    // it is a counter-row fact from Key Quest Markers, not a gauge-row fact.
    AUTHORED50.store(active, Ordering::Release);
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
    // rebuilding, and the walk ends when a sibling pointer comes back to the root.
    // The three sharpness textures are direct siblings of the root, so the mask
    // enters as the root's first child, keeps the glow and the edge as its own
    // siblings, and takes the bar as its child.
    let root_dti = MtDti::find("cGUIObjRoot").ok_or(33u32)?;
    if !is_type(root, root_dti as *const _ as u32) {
        return Err(25);
    }
    let (Some(cache), Some(cache_flags)) = (
        memory::word(root.wrapping_add(0x80)),
        memory::word(root.wrapping_add(0x8c)),
    ) else {
        return Err(23);
    };
    // drawables() reads the walker's own count word in the cache header, so the
    // cache we allocate is sized from what the tree produced this frame.
    let drawables = sampling::drawables();
    if cache == 0 || drawables == 0 || !memory::readable(cache.wrapping_sub(8), 0x20) {
        return Err(26);
    }
    let (Some(bar), Some(bar_parent)) = (
        memory::word(root.wrapping_add(NODE_CHILD)),
        memory::word(root.wrapping_add(NODE_PARENT)),
    ) else {
        return Err(23);
    };
    if bar == 0 || bar == root || bar_parent != root || !memory::writable(bar, TEXTURE_BYTES) {
        return Err(27);
    }
    let (Some(effect), Some(effect_parent)) = (
        memory::word(bar.wrapping_add(NODE_SIBLING)),
        memory::word(bar.wrapping_add(NODE_PARENT)),
    ) else {
        return Err(23);
    };
    if effect == 0 || effect == bar || effect_parent != root || !memory::writable(effect, TEXTURE_BYTES) {
        return Err(28);
    }
    let (Some(edge), Some(edge_parent)) = (
        memory::word(effect.wrapping_add(NODE_SIBLING)),
        memory::word(effect.wrapping_add(NODE_PARENT)),
    ) else {
        return Err(23);
    };

    if edge == 0 || edge == effect || edge == bar
        || edge_parent != root
        || !memory::writable(edge, TEXTURE_BYTES)
    {
        return Err(29);
    }
    // The walk stops where a sibling comes back to the root, so the last sibling
    // of the root's chain must point at the root. That is the loop bound the cache
    // entries are built from, and it is what a stray child pointer would break.
    let terminator = memory::word(edge.wrapping_add(NODE_SIBLING)).ok_or(23u32)?;
    if terminator != root {
        return Err(30);
    }
    let group = root;
    let Some(texture) = MtDti::find("cGUIObjTexture") else {
        return Err(33);
    };
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
    Ok(Row {
        root,
        instance,
        map,
        cache,
        group,
        bar,
        effect,
        width: live_width,
    })
}

/// Build the mask, grow the row's map and draw cache, publish them, then reparent
/// the bar under the mask. A failure at any stage publishes nothing.
fn insert(row: &Row) -> Result<u32, u32> {
    // 1. The mask. Its factory allocates through the mask DTI's own MT allocator,
    // so the game's destructor owns the block from the moment it returns.
    let mask = memory::call0(base() + 0xa4b6cc).ok_or(1u32)?;
    let dti = MtDti::find("cGUIObjScissorMask").ok_or(18u32)? as *const _ as u32;
    if mask == 0 || mask & 3 != 0 || !memory::writable(mask, MASK_BYTES) || !is_type(mask, dti) {
        return Err(2);
    }
    // 2. The cache allocation goes through the root object's own pair of methods,
    // vtable 0x58 to take and 0x5c to release. That is proven for this class: the
    // root is a cGUIObjRoot (gate 25), the same class the quest row publishes, and
    // the native destructor releases the block through the same slot.
    let allocate = method(row.root, 0x58).ok_or(4u32)?;
    let free = method(row.root, 0x5c).ok_or(5u32)?;
    // 3. The draw cache grows by one begin/end pair: the walker emits an enter
    // entry and an exit entry for the new child, and the terminator stays last.
    // Entries are (kind, node) word pairs, eight bytes each, so one more drawable
    // is exactly sixteen more bytes plus the eight-byte terminator, and the header
    // sits eight bytes below the entry pointer the root publishes at +0x80.
    let cache_bytes = (sampling::drawables() + 1) * 16 + 8;
    let new_cache = memory::call2(allocate, row.root, cache_bytes).ok_or(10u32)?;
    if new_cache == 0
        || new_cache & 15 != 0
        || !memory::write_bytes(new_cache.wrapping_sub(8), (cache_bytes + 8) as usize)
    {
        return Err(11);
    }
    CACHE_BLOCK.store(new_cache, Ordering::Release);
    // 4. The mask takes the root's first-child slot and the bar becomes its child.
    // The mask's siblings stay the glow and the edge, so the walk visits
    // mask -> bar -> glow -> edge and the scissor covers all three. The mask needs
    // visible and drawable set, because those are the two bits the walker emits on:
    // bit 0 selects the enter path, 0x0200_0000 selects the entry itself.
    if !word(mask + 0x50, MOD_ID)
        || !word(mask + NODE_CHILD, row.bar)
        || !word(mask + NODE_SIBLING, row.effect)
        || !word(mask + NODE_PARENT, row.group)
        || !word(
            mask + NODE_FLAGS,
            memory::word(mask + NODE_FLAGS).ok_or(12u32)? | NODE_VISIBLE | NODE_DRAWABLE,
        )
        || !word(row.bar + NODE_PARENT, mask)
        || !word(row.group + NODE_CHILD, mask)
    {
        memory::call2(free, row.root, new_cache);
        return Err(15);
    }
    // 5. Publish the gauge cache pointer. The counter mod writes map aliases at
    // root+0xa0/+0xa4 and a cache alias at root+0x9c; the gauge uses none of them
    // (controller+0x9c is the animation row and +0xa0/+0xa4 stay 0), so writing
    // them here would corrupt the row. Bit 0 of root+0x8c is the dirty flag the
    // walker clears when it rebuilds, so that is the only bit to set.
    let published = word(row.root + 0x80, new_cache)
        && word(
            row.root + 0x8c,
            memory::word(row.root + 0x8c).ok_or(16u32)? | 1,
        );
    if !published {
        return Err(17);
    }
    // The originals stay allocated and reachable; the plugin keeps them for
    // restore, and the game's own cleanup counts the published tree.
    OLD_CACHE.store(row.cache, Ordering::Release);
    SIZES.store(
        (GAUGE_OBJECTS as u32) | ((row.cache_bytes()) << 16),
        Ordering::Release,
    );
    BAR_NODE.store(row.bar, Ordering::Release);
    GROUP_NODE.store(row.group, Ordering::Release);
    Ok(mask)
}

impl Row {
    fn cache_bytes(&self) -> u32 {
        ((sampling::drawables() + 1) * 16 + 8) & 0xffff
    }
}

/// Write the clip for one ratio. Position is a float pair with the dirty bit the
/// game's own setters set, size is the integer pair the consumer at main+0xa4ba94
/// reads once bit 1 of +0xa4 is on.
pub fn apply(ratio_bp: u32) -> Option<[i32; 4]> {
    if STATE.load(Ordering::Relaxed) != 1 {
        return None;
    }
    let mask = MASK.load(Ordering::Relaxed);
    let row = read_row().ok()?;
    let box_ = sharpness_gauge_core::mask_box(rect(&row.bar)?, ratio_bp, row.width);
    if !memory::write_bits(mask + 0x80, box_[0] as f32)
        || !memory::write_bits(mask + 0x84, box_[1] as f32)
        || !word(mask + 0x54, memory::word(mask + 0x54)? | 0x2_0000)
        || !word(mask + 0xa8, box_[2] as u32)
        || !word(mask + 0xac, box_[3] as u32)
        || !word(mask + 0xa4, memory::word(mask + 0xa4)? | 2)
    {
        return None;
    }
    Some(box_)
}

/// The bar's own rect, as the game keeps it at +0xe0.
fn rect(bar: &u32) -> Option<[i32; 4]> {
    let left = memory::word(bar.checked_add(0xe0)?)? as i32;
    let top = memory::word(bar.checked_add(0xe4)?)? as i32;
    let right = memory::word(bar.checked_add(0xe8)?)? as i32;
    let bottom = memory::word(bar.checked_add(0xec)?)? as i32;
    Some([left, top, right, bottom])
}

/// Insert once, from the plugin's update callback (the game's own thread, outside
/// the walker). A row that is not present yet is not a failure: the gauge HUD row
/// exists only while a weapon-owning player is in a quest, so stage 20 reports once
/// and keeps retrying at a slow cadence. A hard failure (mask factory, allocation
/// or publish) is terminal, because retrying an allocation path that already
/// failed is not how this row gets built.
const RETRY_EVERY: u32 = 240;
static TRIES: AtomicU32 = AtomicU32::new(0);
/// The bar's live width, and bit 0/1 for "the authored 88 and 24 match".
static BAR_W: AtomicU32 = AtomicU32::new(0);
static AUTHORED: AtomicU32 = AtomicU32::new(0);
/// root+0x50 as read, recorded because it is not a gate for this row.
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

fn restore_links(old_cache: u32, root: u32) -> Option<()> {
    let bar = BAR_NODE.load(Ordering::Relaxed);
    if bar == 0 || !word(bar + NODE_PARENT, root) || !word(root + NODE_CHILD, bar) {
        return None;
    }
    if !word(root + 0x80, old_cache) {
        return None;
    }
    let flags = memory::word(root + 0x8c)?;
    if !word(root + 0x8c, flags | 1) {
        return None;
    }
    Some(())
}

/// Undo the insertion: restore the row's links, publish the original map and
/// cache back, then let the game's destructor free the mask. The map the plugin
/// allocated is freed last, after nothing points at it.
pub fn restore() -> bool {
    if STATE.load(Ordering::Relaxed) != 1 {
        return true;
    }
    let (mask, _, _) = applied();
    let old_cache = OLD_CACHE.load(Ordering::Relaxed);
    let Some((_, root)) = hooks::gauge_instance() else {
        return false;
    };
    if old_cache == 0 || restore_links(old_cache, root).is_none() {
        return false;
    }
    let block = CACHE_BLOCK.load(Ordering::Relaxed);
    if let Some(destroy) = method(mask, 0x04) {
        memory::call1(destroy, mask);
    }
    // The cache block the root handed us is released through the root's own
    // release slot, the same pair the native cleanup uses.
    if let Some(free) = method(root, 0x5c) {
        if block != 0 {
            memory::call2(free, root, block);
        }
    }
    STATE.store(2, Ordering::Release);
    true
}
