//! Insert one scissor mask into the gauge row, with the two storage expansions it needs.
//!
//! The native contracts this rests on, all read from the decompiles recorded in
//! work/modern-sharpness-gauge/PHASE2_RESEARCH.md:
//!
//! - The row's object array at `instance+0xd8` (aliased at `root+0xa0`) is indexed by
//!   main+0x5456e4 with main+0xa47d2c's node count + 1, dispatching vtable slot 0x50 on each
//!   entry. It holds one word per object with the controller first, so an added node needs a
//!   fifth word. It is grown through the `cGUIInstance` DTI's MT allocator, the Key Quest
//!   Markers pattern: Forge's malloc cannot free what the game's destructor frees.
//! - The draw block is sized by main+0xa49b28 as `(objects << 4 | 8) + sequences * 12` from the
//!   halfwords at row+8, allocated through the root's vtable slot 0x58, with the walker's
//!   pointer at `alloc + sequences * 12`. This row is sized for 3 objects and 16 sequences, so
//!   its record area holds 7 records, and the mask's tree emits 9: the insertion has to grow
//!   the block.
//! - The mask's rectangle at `+0xb0..+0xbc` is owned by the game's layout, which rebuilds it
//!   from the node's own quad (position + authored size). A node the animation row has no entry
//!   for comes out with width 0, so the box is written at the mask's enter handler
//!   main+0xa4ba94 in the same call that reads it.
//!
//! Everything here runs inside a native callback on the game's GUI thread. No game pointer is
//! kept past the callback: addresses are plain integers, cleared by the root destructor hook
//! and by the mask's own liveness check in `enter`.
use crate::{gui, memory, sampling};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use forge::mt::dti::MtDti;
use sharpness_gauge_core::{
    basis_points, cut_box, dim_shape, NODE_AUTHORED_WIDTH, NODE_CHILD, NODE_EFFECTIVE_RECT, NODE_PARENT,
    NODE_RECT, NODE_SIBLING, NODE_TEXTURE_RECT,
};

/// Written to the mask's mId at +0x50, where the factory leaves -1: it identifies our mask in
/// a tree the game owns, so a re-entry never inserts a second one and the enter hook can tell
/// our mask from every other scissor mask in the game.
pub const MARKER: u32 = 0x4d534731;
const MASK_ID: u32 = 0x50;
const MASK_OWNER: u32 = 0x74; // the owning cGUI, taken from the bar
const MASK_RESOURCE: u32 = 0x78; // the borrowed rGUI resource, taken from the bar
const MASK_X_SCALE: u32 = 0x10; // the transform's x row: the builder multiplies the width by it
/// The class's DTI `Enable` setter (main+0xa4ba20, from the DTI record), the native way to
/// stop a scissor mask clipping.
const ENABLE_SETTER: u32 = 0xa4ba20;
const MASK_ANCHORS: u32 = 0xa0; // bits 0-1 y anchor mode, bits 2-3 x anchor mode
const MASK_FLAGS: u32 = 0xa4; // bit 0 enable the scissor, bit 1 rebuild the rectangle
const MASK_SIZE: u32 = 0xa8; // authored width/height, signed integers
/// bit 1 set asks the game's enter handler to rebuild the rectangle from the node's quad, and
/// bit 0 applies it. The rebuild is the game's own arithmetic, and its inputs are the fields
/// written above it.
const REBUILD_AND_APPLY: u32 = 3;
/// The halfwords main+0xa49b28 sizes from: objects at row+8, sequences at row+8+2. The facts
/// line's `counts` is the pair at row+4, which the sizer does not read.
const ROW_SIZING: u32 = 0x08;

// The dim blade (plan Phase 5): a second copy of the blade sprite, outside the mask, in a flat
// dark colour. Its class is `cGUIObjTexture`, size 304, constructor at its vftable slot 0x00;
// the field offsets are from the DTI record and the setter decompiles, all recorded in
// work/modern-sharpness-gauge/PHASE5_RESEARCH.md.
pub const DIM_MARKER: u32 = 0x4d534732;
/// Flat dark grey for all four corner colours. The MT `color` word's byte order is not proven in
/// this workspace, so the grey is byte-symmetric and the alpha byte's position is an inference:
/// a run that shows the copy too transparent, or dark in the wrong channels, is this constant,
/// not the mechanism.
const DIM_COLOUR: u32 = 0x73404040;
const TEXTURE_SIZE: u32 = 304;
const TEXTURE_CTOR: u32 = 0x866a4;
const TEXTURE_SETTER: u32 = 0xa4faf0;
const TEX_MID: u32 = 0x50;
const TEX_STATE: u32 = 0x30; // the kind byte (+0x33) and the word at +0x34 prepaint reads
const TEX_FLAGS: u32 = 0x54; // bit 0 visible, 0x0200_0000 drawable, +0x56 bit 6 prepaint skip
const TEX_OWNER: u32 = 0x74;
const TEX_AUTHORED: u32 = 0xb4;
const TEX_COLOURS: u32 = 0xa4; // four corner words: LT, RT, LB, RB
const TEX_TEXTURE_ID: u32 = 0x100;
static DIM: AtomicU32 = AtomicU32::new(0);

/// Times the enter hook found the mask's links gone: the count the log prints, and the only
/// teardown signal this build can prove (the root destructor hook has fired zero times).
static MISSES: AtomicU32 = AtomicU32::new(0);
static MASK: AtomicU32 = AtomicU32::new(0);
static BAR: AtomicU32 = AtomicU32::new(0);
static ROOT: AtomicU32 = AtomicU32::new(0);

/// The plugin thread's own update tick, used only to rate-limit the cut's log line.
/// shortcut: pacing the log by update count, not seconds; 30 updates is a fraction of a second.
static UPDATES: AtomicU32 = AtomicU32::new(0);

/// One tick of the log clock, called from `forge_onUpdate`: a plain counter, no game pointer,
/// no allocation and no formatting.
pub fn tick_clock() {
    UPDATES.fetch_add(1, Ordering::Relaxed);
}

/// The cut line queued by `enter`: ratio_bp, the box written, the bar's own rect at
/// +0xe0, its effective rect at +0xf0, its atlas rect at +0x10c (core's NODE_RECT,
/// NODE_EFFECTIVE_RECT and NODE_TEXTURE_RECT), the authored width read at +0xb4 and the rect
/// width the box was built from. The ratio is always the player's own sharpness and the
/// endpoints are core's accepted offsets, so neither is repeated on the line.
pub const CUT_WORDS: usize = 19;
static CUT: [AtomicU32; CUT_WORDS] = const { [const { AtomicU32::new(0) }; CUT_WORDS] };
static CUT_READY: AtomicBool = AtomicBool::new(false);
/// u32::MAX is not a ratio (they are 0..=10_000), so the first box written queues a line.
static CUT_RATIO: AtomicU32 = AtomicU32::new(u32::MAX);
/// The last clock tick that queued a line, for the rate limit above.
static CUT_LOG_AT: AtomicU32 = AtomicU32::new(0);

/// The queued line, taken once per change of the cut's ratio. `forge_onUpdate` prints it, so
/// nothing is formatted in the hook and nothing is logged per frame.
pub fn take_cut_line() -> Option<[u32; CUT_WORDS]> {
    if !CUT_READY.swap(false, Ordering::Acquire) {
        return None;
    }
    let mut out = [0u32; CUT_WORDS];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = CUT[index].load(Ordering::Relaxed);
    }
    Some(out)
}

/// The four words of one rectangle field on a node.
fn rect_at(node: u32, offset: u32) -> [i32; 4] {
    let mut out = [0i32; 4];
    for edge in 0..4 {
        out[edge] = memory::word(node + offset + edge as u32 * 4).unwrap_or(0) as i32;
    }
    out
}

/// The link state as the game left it at a drop: mask marker, mask.parent, mask.child,
/// bar.parent, bar.sibling, root.child. Which link broke is the fact the log line names, and
/// no later read can recover it.
static LINK_LOST: [AtomicU32; 6] = const { [const { AtomicU32::new(0) }; 6] };
const SNAPSHOT: [(usize, u32); 6] = [
    (0, MASK_ID),       // the mask's own marker word
    (0, NODE_PARENT),   // mask.parent
    (0, NODE_CHILD),    // mask.child
    (1, NODE_PARENT),   // bar.parent
    (1, NODE_SIBLING),  // bar.sibling
    (2, NODE_CHILD),    // root.child
];

/// Read the link state at the drop, and keep the addresses to re-read it later. Read-only.
fn snapshot(mask: u32, bar: u32, root: u32) {
    let bases = [mask, bar, root];
    for (index, (which, offset)) in SNAPSHOT.iter().enumerate() {
        let value = memory::word(bases[*which].wrapping_add(*offset)).unwrap_or(0);
        LINK_LOST[index].store(value, Ordering::Relaxed);
    }
}

/// The snapshot: mask marker, mask.parent, mask.child, bar.parent, bar.sibling, root.child.
pub fn link_lost() -> [u32; 6] {
    let mut out = [0u32; 6];
    for (index, slot) in LINK_LOST.iter().enumerate() {
        out[index] = slot.load(Ordering::Relaxed);
    }
    out
}

/// What one insertion published, as integers for the log line.
pub struct Outcome {
    pub mask: u32,
    pub map: u32,
    pub cache: u32,
    pub entries: u32,
    pub records: u32,
    /// The dim blade's node, 0 when it was not built.
    pub dim: u32,
    /// The stage its build stopped at, 0 when it was built: 50 the allocation, 51 the
    /// constructor and class test, 52 the fields. A failed build leaves the accepted cut alone.
    pub dim_stage: u32,
}

/// The row is going away: the game owns the mask, the map and the cache. Called from the root
/// destructor hook and from the mask's own link check.
pub fn on_teardown() {
    ROOT.store(0, Ordering::Release);
    MASK.store(0, Ordering::Release);
    BAR.store(0, Ordering::Release);
    DIM.store(0, Ordering::Release);
}

/// The captured row root, for the root destructor hook's gate. The destructor's argument is the
/// root node, whose parent is the controller, so the controller is never that argument.
pub fn row_root() -> u32 {
    ROOT.load(Ordering::Relaxed)
}

/// Times the mask's links were found gone at its enter hook.
pub fn misses() -> u32 {
    MISSES.load(Ordering::Relaxed)
}

fn base() -> u32 {
    memory::BASE.load(Ordering::Relaxed)
}

fn is_type(object: u32, dti: u32) -> bool {
    let Some(f) = memory::method(object, 0x14) else {
        return false;
    };
    memory::call1(f, object).is_some_and(|got| got == dti)
}

/// A DTI's MT allocator and its free (main+0x7aa47c is `MtDti::getAllocator`). Game-owned GUI
/// memory must come from and go back to this allocator, not Forge's malloc.
struct Allocator {
    handle: u32,
    free: u32,
}

impl Allocator {
    fn find(name: &str) -> Option<Self> {
        let dti = MtDti::find(name)?;
        let getter: u32 = base() + 0x7aa47c;
        let handle = memory::call1(getter, dti as *const _ as u32)?;
        let free = memory::method(handle, 0x34)?;
        Some(Self { handle, free })
    }
    /// Slot 0x1c is `memAlloc(bytes, align)`, the Key Quest Markers-verified signature: both
    /// arguments, align 16, which is also what the published block's &15 gate asks for.
    fn alloc(&self, bytes: u32) -> Option<u32> {
        let f = memory::method(self.handle, 0x1c)?;
        memory::call3(f, self.handle, bytes, 16).filter(|p| *p != 0)
    }
    fn release(&self, pointer: u32) {
        if pointer != 0 {
            let _ = memory::call2(self.free, self.handle, pointer);
        }
    }
}

/// Grow the row's object array by one entry and publish both aliases. `entries` is the length
/// main+0x5456e4 indexes with, so the block and the new entry have to be in place before the
/// tree gains a node: a fifth node against a four-word array is read past its end.
fn grow_map(instance: u32, root: u32, entries: u32, node: u32, extra: u32) -> Result<u32, u32> {
    let old = memory::word(instance + 0xd8).ok_or(43u32)?;
    if old == 0 || memory::word(root + 0xa0).ok_or(43u32)? != old {
        return Err(43);
    }
    let allocator = Allocator::find("cGUIInstance").ok_or(42u32)?;
    let appended = if extra == 0 { 1 } else { 2 };
    let bytes = (entries + appended) * 4;
    let fresh = match allocator.alloc(bytes) {
        Some(pointer) if pointer & 15 == 0 => pointer,
        Some(pointer) => {
            allocator.release(pointer);
            return Err(42);
        }
        None => return Err(42),
    };
    if !memory::copy_words(old, fresh, entries as usize)
        || !memory::write_word(fresh + entries * 4, node)
        || (extra != 0 && !memory::write_word(fresh + entries * 4 + 4, extra))
    {
        allocator.release(fresh);
        return Err(43);
    }
    if !memory::write_word(instance + 0xd8, fresh) || !memory::write_word(root + 0xa0, fresh) {
        allocator.release(fresh);
        return Err(43);
    }
    // shortcut: the old 4-word block leaks (16 bytes per insertion). No code read so far
    // allocates it, and handing a block to an allocator that never owned it corrupts that
    // allocator's free list. At a row's drop its objects read as freed-heap fill
    // (0xffeaeaea/0xffe3e3e3), the memory is reused by the game within seconds, and no gauge
    // controller is matched afterwards. If the game rebuilds its row in the next area again,
    // read who allocated the old block and free it there.
    Ok(fresh)
}

/// Resize the row's draw block for the extended object count, through the root's own
/// allocate/free pair (slots 0x58/0x5c) so the root's destructor frees an allocation the root
/// made.
fn grow_cache(root: u32, objects: u32, sequences: u32) -> Result<(u32, u32), u32> {
    let (size, prefix, area) = sharpness_gauge_core::cache_layout(objects, sequences);
    let allocate = memory::method(root, 0x58).ok_or(44u32)?;
    let release = memory::method(root, 0x5c).ok_or(44u32)?;
    let old_alloc = memory::word(root + 0xa4).unwrap_or(0);
    let fresh = memory::call2(allocate, root, size).ok_or(45u32)?;
    if fresh == 0 || fresh & 15 != 0 || !memory::writable(fresh, size as usize) {
        if fresh != 0 {
            let _ = memory::call2(release, root, fresh);
        }
        return Err(45);
    }
    if !memory::zero_words(fresh, (size / 4) as usize) {
        let _ = memory::call2(release, root, fresh);
        return Err(46);
    }
    if !memory::write_word(root + 0xa4, fresh) || !memory::write_word(root + 0x80, fresh + prefix)
    {
        let _ = memory::call2(release, root, fresh);
        return Err(46);
    }
    // The root's own destructor frees root+0xa4 through slot 0x5c, so the block it replaces
    // goes back the same way. Leaving it unreleased stops the bar drawing in the quest's first
    // area, so the replacement only works with the old block returned to the root's allocator.
    if old_alloc != 0 && old_alloc != fresh {
        let _ = memory::call2(release, root, old_alloc);
    }
    Ok((fresh, area / 8))
}

/// Build the dim blade: a `cGUIObjTexture` allocated through its own DTI's MT allocator and
/// constructed by the class's slot-0x00 constructor, then given the bar's render fields so it
/// draws the same sprite in the flat dark colour. Nothing is linked and no list head is copied:
/// the node has no animation entry, so no keyframe writes to it, which is what keeps it constant
/// and non-pulsing and leaves `Txt_sharp`'s own keyframes untouched. Stages 50..52.
fn build_dim(bar: u32) -> Result<u32, u32> {
    let dti = MtDti::find("cGUIObjTexture").ok_or(50u32)?;
    let allocator = Allocator::find("cGUIObjTexture").ok_or(50u32)?;
    let node = match allocator.alloc(TEXTURE_SIZE) {
        Some(pointer) if pointer & 15 == 0 => pointer,
        Some(pointer) => {
            allocator.release(pointer);
            return Err(50);
        }
        None => return Err(50),
    };
    let built = memory::call1(base() + TEXTURE_CTOR, node).is_some_and(|got| got == node);
    if !built
        || !is_type(node, dti as *const _ as u32)
        || !memory::writable(node, TEXTURE_SIZE as usize)
    {
        allocator.release(node);
        return Err(51);
    }
    // The render fields, read live from the bar: the transform rows, the kind and the word
    // prepaint reads, the visible and drawable bits, owner and resource, the authored pair, the
    // texture id and object, and the atlas rectangle.
    let copied = (0..8usize).all(|index| {
        memory::word(bar + 0x10 + index as u32 * 4)
            .is_some_and(|word| memory::write_word(node + 0x10 + index as u32 * 4, word))
    }) && memory::copy_words(bar + TEX_STATE, node + TEX_STATE, 2)
        && memory::word(bar + TEX_FLAGS).is_some_and(|flags| {
            memory::write_word(node + TEX_FLAGS, flags & 0x0200_0001)
        })
        && memory::copy_words(bar + TEX_OWNER, node + TEX_OWNER, 2)
        && memory::copy_words(bar + TEX_AUTHORED, node + TEX_AUTHORED, 2)
        && memory::copy_words(bar + TEX_TEXTURE_ID, node + TEX_TEXTURE_ID, 2)
        && memory::copy_words(bar + NODE_TEXTURE_RECT, node + NODE_TEXTURE_RECT, 4);
    let texture = memory::word(bar + TEX_TEXTURE_ID)
        .and_then(|id| memory::call2(base() + TEXTURE_SETTER, node, id))
        .is_some();
    let marked = memory::write_word(node + TEX_MID, DIM_MARKER)
        && (0..4usize).all(|corner| memory::write_word(node + TEX_COLOURS + corner as u32 * 4, DIM_COLOUR));
    if !copied || !texture || !marked {
        if let Some(destructor) = memory::method(node, 0x04) {
            let _ = memory::call1(destructor, node);
        }
        allocator.release(node);
        return Err(52);
    }
    Ok(node)
}

/// Write the dim blade's rectangle and atlas for this frame's cut. It starts where the kept part
/// ends and runs to the bar rect's right edge, with the atlas `u` at the same fraction of the
/// authored 88 columns, so the copy keeps the blade's scale. With no valid gauge the width is
/// zero and nothing is drawn, which is the accepted shipped appearance.
fn write_dim(dim: u32, rect: [i32; 4], ratio_bp: u32) {
    let atlas = rect_at(dim, NODE_TEXTURE_RECT);
    let (box_rect, box_atlas) = if ratio_bp == u32::MAX {
        ([rect[0], rect[1], rect[0], rect[3]], atlas)
    } else {
        dim_shape(rect, ratio_bp, atlas)
    };
    for edge in 0..4usize {
        let _ = memory::write_word(dim + NODE_RECT + edge as u32 * 4, box_rect[edge] as u32);
        let _ = memory::write_word(
            dim + NODE_TEXTURE_RECT + edge as u32 * 4,
            box_atlas[edge] as u32,
        );
    }
}

/// Read-only reacquire signal, for the draw walker's callback. A mask freed with its row stops
/// calling its enter handler, and quests cross areas through loading screens, so the walk of
/// the next area is where discovery has to start again. Nothing is written here: the mask is
/// reached through links this plugin captured, so a stale read can only drop the addresses,
/// never write into freed memory.
pub fn check_liveness() {
    let (mask, root) = (MASK.load(Ordering::Relaxed), ROOT.load(Ordering::Relaxed));
    if mask == 0 {
        return;
    }
    if memory::word(mask + MASK_ID) == Some(MARKER)
        && memory::word(mask + NODE_PARENT) == Some(root)
        && memory::word(root + NODE_CHILD) == Some(mask)
    {
        return;
    }
    MISSES.fetch_add(1, Ordering::Relaxed);
    snapshot(mask, BAR.load(Ordering::Relaxed), root);
    crate::hooks::teardown_gauge_row(root);
}

/// The mask's enter handler is about to read `+0xb0..+0xbc` and apply the scissor, so the box
/// is written here, in this call. Returns without touching anything for a mask that is not
/// ours, and drops the addresses when the links that made it ours are gone: a mask left over
/// from a row whose scene ended keeps being entered, and writing into it then writes into
/// freed memory.
pub fn enter(mask: u32) {
    if memory::word(mask + MASK_ID) != Some(MARKER) {
        return;
    }
    let (bar, root) = (BAR.load(Ordering::Relaxed), ROOT.load(Ordering::Relaxed));
    let alive = bar != 0
        && root != 0
        && memory::word(mask + NODE_PARENT) == Some(root)
        && memory::word(root + NODE_CHILD) == Some(mask)
        && memory::word(bar + NODE_PARENT) == Some(mask);
    if !alive {
        // The links that made this mask are gone: the row is going away at an area's loading
        // screen. The game is calling this mask's handler right now, so it is live, and asking
        // it to stop clipping here is the one disable this build can prove safe. Left alone, the
        // mask applies its last rectangle into the next area.
        MISSES.fetch_add(1, Ordering::Relaxed);
        snapshot(mask, bar, root);
        let _ = memory::call2(base() + ENABLE_SETTER, mask, 0);
        crate::hooks::teardown_gauge_row(root);
        return;
    }
    // The live fill: the local player's own sharpness points, read fresh in this call through
    // the same checked mapping discovery uses. No valid gauge - no player yet, a zero maximum,
    // points above the maximum - is unsupported state, so the clip is turned off here and the
    // native gauge is left as the game draws it.
    let dim = DIM.load(Ordering::Relaxed);
    let ratio = match sampling::live_gauge().and_then(|gauge| basis_points(gauge)) {
        Some(bp) => bp,
        None => {
            let _ = memory::call2(base() + ENABLE_SETTER, mask, 0);
            // No gauge to cut by: the copy is hidden to width 0, the accepted appearance.
            if dim != 0 {
                write_dim(dim, rect_at(bar, NODE_RECT), u32::MAX);
            }
            return;
        }
    };
    let updates = UPDATES.load(Ordering::Relaxed);
    let rect = rect_at(bar, NODE_RECT);
    // The bar's authored-size field (bar+0xa8) is animation-driven and layout-zeroed at mask
    // enter, so it reads 0 here and a box built from it cuts the blade near the tip. The bar's
    // screen rect is the proven draw width: 110..238, 128 px in the gauge row's tree.
    if rect[2] <= rect[0] {
        return;
    }
    let authored = memory::word(bar + NODE_AUTHORED_WIDTH)
        .map(|word| f32::from_bits(word) as i32)
        .unwrap_or(0);
    // The box runs between the accepted endpoints, which core scales by this rect's width, so a
    // bar laid out wider or narrower cuts the same fraction of its blade. The authored read
    // stays in the log line as the proof of that difference.
    let box_words = cut_box(rect, ratio);
    // One line per change of the driving ratio, rate-limited so a fast sequence of ratio
    // changes prints one line, not one per frame.
    if ratio != CUT_RATIO.swap(ratio, Ordering::Relaxed)
        && updates.wrapping_sub(CUT_LOG_AT.load(Ordering::Relaxed)) >= 30
    {
        CUT_LOG_AT.store(updates, Ordering::Relaxed);
        // Queued for the plugin thread to print: the box written, then the bar's own rect, the
        // rect prepaint leaves after the clip intersection, the rect inside the atlas, and the
        // width the box was built from.
        let effective = rect_at(bar, NODE_EFFECTIVE_RECT);
        let texture = rect_at(bar, NODE_TEXTURE_RECT);
        CUT[0].store(ratio, Ordering::Relaxed);
        for (edge, word) in box_words
            .iter()
            .chain(rect.iter())
            .chain(effective.iter())
            .chain(texture.iter())
            .enumerate()
        {
            CUT[1 + edge].store(*word as u32, Ordering::Relaxed);
        }
        CUT[17].store(authored as u32, Ordering::Relaxed);
        CUT[18].store((rect[2] - rect[0]) as u32, Ordering::Relaxed);
        CUT_READY.store(true, Ordering::Release);
    }
    // The rectangle is the game's own arithmetic, and main+0xa4b390 shows its inputs, so these
    // are the writes that matter. A rectangle read as width 0, height 32 at the parent's origin
    // (110,38), against authored 128x32, pins each input:

    // 1. The anchor modes, bits 0-3 of +0xa0: `& 0xc` is x and `& 3` is y, mode 0 spans
    //    0..size, 1 centres, 2 spans -size..0, 3 collapses the axis. The factory writes
    //    (word & 0xfffff000) | 0xff0 and leaves bits 0-3 as the block came from the allocator,
    //    and the class has no Anchor property in its DTI record, so bits 0-3 are recycled. They
    //    have to read 0 for the width to reach the rectangle.
    if let Some(anchors) = memory::word(mask + MASK_ANCHORS) {
        if anchors & 0xf != 0 {
            let _ = memory::write_word(mask + MASK_ANCHORS, anchors & !0xf);
        }
    }
    // 2. The transform's x row, at +0x10/+0x14. The builder multiplies the authored width by
    //    +0x10 for x and by +0x14 for y, and the authored height by +0x20/+0x24. A width of 0
    //    from an authored 128, with a height of 32 from an authored 32, says the height row is
    //    (0,1) and the width row is (0,0): the factory copies its matrix template, and that
    //    template carries no x scale. Set the x scale to 1.0 and the authored width reaches the
    //    rectangle.
    let _ = memory::write_word(mask + MASK_X_SCALE, 1.0f32.to_bits());
    let _ = memory::write_word(mask + MASK_X_SCALE + 4, 0f32.to_bits());
    // 3. The authored pair. The height is the bar's own rect height, so the clip spans the bar
    //    top to bottom; the width is the ratio's share of it. The position at +0x80 is a DTI
    //    property that the row's layout resolves into the matrix for animated nodes, and this
    //    node has no animation entry, so writing it does nothing: the rectangle's origin comes
    //    from the parent's transform, which is the gauge's own corner (110,38).
    let height = (rect[3] - rect[1]).max(1);
    let _ = memory::write_word(mask + MASK_SIZE, box_words[2] as u32);
    let _ = memory::write_word(mask + MASK_SIZE + 4, height as u32);
    let _ = memory::write_word(mask + MASK_FLAGS, REBUILD_AND_APPLY);
    if dim != 0 {
        write_dim(dim, rect, ratio);
    }
}

/// Insert the mask under the row root, over the bar, glow and edge, after the row's map and
/// cache have been grown for it. Stage codes 40..49, kept apart from the row gates 21..34.
pub fn run(row: &gui::Row, instance: u32) -> Result<Outcome, u32> {
    if MASK.load(Ordering::Relaxed) != 0 {
        return Err(49);
    }
    let root = row.root;
    if memory::word(root + NODE_CHILD).is_some_and(|child| {
        child != 0 && memory::word(child + MASK_ID) == Some(MARKER)
    }) {
        return Err(49);
    }
    // The row's two count pairs: row+4 is 4 and 15, row+8 is 3 and 16, and the cache sits
    // 192 = 16 * 12 into the allocation. The sizing pair is row+8, and its objects field equals
    // the tree's node count; the map is the field that is one longer.
    let objects = memory::half(row.description + ROW_SIZING).ok_or(48u32)? as u32;
    let sequences = memory::half(row.description + ROW_SIZING + 2).ok_or(48u32)? as u32;
    let nodes = sharpness_gauge_core::count_nodes(&Read, root).0;
    if objects != nodes || objects == 0 || objects > 64 || sequences > 4096 {
        return Err(48);
    }
    // The mask borrows the bar's owning cGUI and resource, read live.
    let owner = memory::word(row.bar + MASK_OWNER).ok_or(41u32)?;
    let resource = memory::word(row.bar + MASK_RESOURCE).ok_or(41u32)?;
    if owner == 0 || resource == 0 {
        return Err(41);
    }
    let dti = MtDti::find("cGUIObjScissorMask").ok_or(40u32)?;
    // The class factory allocates the 0xd0-byte object through the mask DTI's MT allocator and
    // constructs it; the object it returns passes the class test and draws.
    let mask = memory::call0(base() + 0xa4b6cc).ok_or(40u32)?;
    if mask == 0 || !is_type(mask, dti as *const _ as u32) {
        return Err(40);
    }
    let owned = memory::method(mask, 0x04);
    let ready = memory::write_word(mask + MASK_ID, MARKER)
        && memory::write_word(mask + MASK_OWNER, owner)
        && memory::write_word(mask + MASK_RESOURCE, resource)
        && memory::write_word(mask + MASK_FLAGS, REBUILD_AND_APPLY);
    if !ready {
        if let Some(destructor) = owned {
            let _ = memory::call1(destructor, mask);
        }
        return Err(41);
    }
    // The dim blade is built before any of the tree exists, so its failure costs the accepted
    // cut nothing: the mask insertion below is the accepted path exactly.
    let (dim, dim_stage) = match build_dim(row.bar) {
        Ok(node) => (node, 0u32),
        Err(stage) => (0u32, stage),
    };
    let map = match grow_map(instance, root, nodes + 1, mask, dim) {
        Ok(map) => map,
        Err(stage) => {
            if let Some(destructor) = owned {
                let _ = memory::call1(destructor, mask);
            }
            return Err(stage);
        }
    };
    // The records the walker emits over the tree this builds: 9 with the mask alone, 11 once the
    // dim blade joins the root's chain (edge glyph visible; 9 with it hidden, as the looping
    // tiers keep it). The grown area is (5 << 4) | 8 = 88 bytes = 11 records, so the 11 fit
    // exactly; without the dim blade the row grows to 4 objects, area 72 = 9 records.
    let cache = match grow_cache(root, objects + 1 + (dim != 0) as u32, sequences) {
        Ok(done) => done,
        Err(stage) => {
            // The map is published and the tree untouched, so main+0x5456e4 never reaches the
            // extra entry. The mask gets its class destructor; its block stays with the mask
            // DTI's allocator, which is the only owner this build has evidence for.
            if let Some(destructor) = owned {
                let _ = memory::call1(destructor, mask);
            }
            return Err(stage);
        }
    };
    // Link last, so a 5-node tree never exists against the old array. The native linkers write
    // the parent links too: 0xa48460(parent, child), 0xa4844c(node, sibling).
    let link_child = base() + 0xa48460;
    let link_sibling = base() + 0xa4844c;
    let linked = memory::call2(link_child, root, mask).is_some()
        && memory::call2(link_child, mask, row.bar).is_some()
        && memory::call2(link_sibling, row.bar, row.effect).is_some()
        && memory::call2(link_sibling, row.effect, row.edge).is_some()
        // One inbound chain per node, the invariant every native teardown walk relies on:
        // the bar chain moved under the mask, so the root's sibling chain has to end.
        && memory::write_word(root + NODE_SIBLING, 0)
        // The dim blade sits on the root's chain as the mask's sibling, so it is drawn after the
        // mask's subtree and outside the scissor.
        && (dim == 0 || memory::call2(link_sibling, mask, dim).is_some());
    if !linked {
        let _ = memory::call2(link_child, root, row.bar);
        return Err(47);
    }
    if let Some(flags) = memory::word(root + 0x8c) {
        let _ = memory::write_word(root + 0x8c, flags | 3); // ask the walker to rebuild
    }
    BAR.store(row.bar, Ordering::Release);
    ROOT.store(root, Ordering::Release);
    MASK.store(mask, Ordering::Release);
    DIM.store(dim, Ordering::Release);
    Ok(Outcome {
        mask,
        map,
        cache: cache.0,
        entries: nodes + 2 + (dim != 0) as u32,
        records: cache.1,
        dim,
        dim_stage,
    })
}

struct Read;
impl sharpness_gauge_core::Memory for Read {
    fn word(&self, address: u32) -> Option<u32> {
        memory::word(address)
    }
}
