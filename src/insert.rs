//! Insert one scissor mask into the gauge row, with the two storage expansions it needs.
//!
//! The native contracts this rests on, all read from the decompiles and the runs recorded in
//! work/modern-sharpness-gauge/PHASE2_RESEARCH.md:
//!
//! - The row's object array at `instance+0xd8` (aliased at `root+0xa0`) is indexed by
//!   main+0x5456e4 with main+0xa47d2c's node count + 1, dispatching vtable slot 0x50 on each
//!   entry. It holds one word per object with the controller first, so an added node needs a
//!   fifth word. It is grown through the `cGUIInstance` DTI's MT allocator, the accepted
//!   Key Quest Markers pattern: Forge's malloc cannot free what the game's destructor frees.
//! - The draw block is sized by main+0xa49b28 as `(objects << 4 | 8) + sequences * 12` from the
//!   halfwords at row+8, allocated through the root's vtable slot 0x58, with the walker's
//!   pointer at `alloc + sequences * 12`. This row measured 3 objects and 16 sequences, so its
//!   record area is 56 = 7 records and the mask's tree emits 9 = 72: the insertion has to grow
//!   it, which is r14's cache overflow.
//! - The mask's rectangle at `+0xb0..+0xbc` is owned by the game's layout, which rebuilds it
//!   from the node's own quad (position + authored size). A node the animation row has no entry
//!   for comes out with width 0, so the box is written at the mask's enter handler main+0xa4ba94
//!   in the same call that reads it: r18 wrote it from the draw walker's entry and the run
//!   showed 376 of 383 probe lines with left == right, the gauge clipped away.
//!
//! Everything here runs inside a native callback on the game's GUI thread. No game pointer is
//! kept past the callback: addresses are plain integers, cleared by the root destructor hook
//! and by the mask's own liveness check in `enter`.
use crate::{
    gui, memory,
    sampling,
};
use core::sync::atomic::{AtomicU32, Ordering};
use forge::mt::dti::MtDti;
use sharpness_gauge_core::{
    basis_points, mask_box, Gauge, NODE_CHILD, NODE_PARENT, NODE_SIBLING,
};

/// Written to the mask's mId at +0x50, where the factory leaves -1: it identifies our mask in
/// a tree the game owns, so a re-entry never inserts a second one and the enter hook can tell
/// our mask from every other scissor mask in the game.
pub const MARKER: u32 = 0x4d534731;
const MASK_ID: u32 = 0x50;
const MASK_OWNER: u32 = 0x74; // the owning cGUI, taken from the bar
const MASK_RESOURCE: u32 = 0x78; // the borrowed rGUI resource, taken from the bar
const MASK_POS: u32 = 0x80; // PosX/PosY floats, a DTI property the layout resolves per animation
const MASK_X_SCALE: u32 = 0x10; // the transform's x row: the builder multiplies the width by it
/// The class's DTI `Enable` setter (main+0xa4ba20, from the DTI record), the native way to
/// stop a scissor mask clipping.
const ENABLE_SETTER: u32 = 0xa4ba20;
const MASK_ANCHORS: u32 = 0xa0; // bits 0-1 y anchor mode, bits 2-3 x anchor mode
const MASK_FLAGS: u32 = 0xa4; // bit 0 enable the scissor, bit 1 rebuild the rectangle
const MASK_SIZE: u32 = 0xa8; // authored width/height, signed integers
const MASK_RECT: u32 = 0xb0; // left, top, right, bottom, four integers
/// bit 1 set asks the game's enter handler to rebuild the rectangle from the node's quad, and
/// bit 0 applies it. The rebuild is the game's own arithmetic, and its inputs are the fields
/// written above it.
const REBUILD_AND_APPLY: u32 = 3;
/// The halfwords main+0xa49b28 sizes from: objects at row+8, sequences at row+8+2. The facts
/// line's `counts` is the pair at row+4, which the sizer does not read.
const ROW_SIZING: u32 = 0x08;

/// Times the enter hook found the mask's links gone: the count the log prints, and the only
/// teardown signal this build can prove (the root destructor hook has fired zero times).
static MISSES: AtomicU32 = AtomicU32::new(0);
static MASK: AtomicU32 = AtomicU32::new(0);
static BAR: AtomicU32 = AtomicU32::new(0);
static ROOT: AtomicU32 = AtomicU32::new(0);
// The r25 run: the gauge row is never rebuilt after an area change. Which link broke at the
// drop, and what the kept addresses read as afterwards, are the facts no log line carries.
static LINK_LOST: [AtomicU32; 6] = const { [const { AtomicU32::new(0) }; 6] };
static KEEP: [AtomicU32; 3] = const { [const { AtomicU32::new(0) }; 3] };
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
    KEEP[0].store(mask, Ordering::Relaxed);
    KEEP[1].store(bar, Ordering::Relaxed);
    KEEP[2].store(root, Ordering::Relaxed);
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

/// The kept dropped-row addresses: mask, bar, root. Zero before the first drop.
pub fn kept() -> (u32, u32, u32) {
    (
        KEEP[0].load(Ordering::Relaxed),
        KEEP[1].load(Ordering::Relaxed),
        KEEP[2].load(Ordering::Relaxed),
    )
}

/// Re-read the kept addresses with the snapshot's fields, from the plugin thread. All zero
/// before the first drop, so callers gate on `kept().0 != 0`.
pub fn probe_kept() -> [u32; 6] {
    let (mask, bar, root) = kept();
    let bases = [mask, bar, root];
    let mut out = [0u32; 6];
    for (index, (which, offset)) in SNAPSHOT.iter().enumerate() {
        out[index] = memory::word(bases[*which].wrapping_add(*offset)).unwrap_or(0);
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
}

/// The row root's destructor ran: the game owns the mask, the map and the cache now. Called
/// from the root destructor hook and from the mask's own link check.
pub fn on_teardown() {
    ROOT.store(0, Ordering::Release);
    MASK.store(0, Ordering::Release);
    BAR.store(0, Ordering::Release);
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
/// tree gains a node: that pairing is r14's crash.
fn grow_map(instance: u32, root: u32, entries: u32, node: u32) -> Result<u32, u32> {
    let old = memory::word(instance + 0xd8).ok_or(43u32)?;
    if old == 0 || memory::word(root + 0xa0).ok_or(43u32)? != old {
        return Err(43);
    }
    let allocator = Allocator::find("cGUIInstance").ok_or(42u32)?;
    let bytes = (entries + 1) * 4;
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
    // allocator's free list. r26's run: at the drop the row's objects read as freed-heap fill
    // (0xffeaeaea/0xffe3e3e3), the memory is reused by the game within seconds, and no gauge
    // controller is matched in 25,000 discovery attempts over the following 160 s. r27 tests
    // the leak. If the game rebuilds its row in the next area again, read who allocated the
    // old block and free it there.
    Ok(fresh)
}

/// Resize the row's draw block for the extended object count, through the root's own
/// allocate/free pair (slots 0x58/0x5c) so the root's destructor frees an allocation the root
/// made.
fn grow_cache(root: u32, objects: u32, sequences: u32) -> Result<(u32, u32), u32> {
    let (size, prefix, area) = sharpness_gauge_core::cache_layout(objects, sequences);
    let allocate = memory::method(root, 0x58).ok_or(44u32)?;
    let release = memory::method(root, 0x5c).ok_or(44u32)?;
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
    // shortcut: the old cache block leaks (72 bytes per insertion). Slot 0x5c is proven as a
    // free pair on Key Quest Markers' counter row only; it has never been observed freeing
    // this row, and freeing through the owning allocator's wrong counterpart corrupts its
    // free list. r27's run proved the map-block leak is not the poison behind the missing
    // post-transition rebuild; this is the last release of game-owned memory in the build.
    // If the next area's row returns, read slot 0x5c's body and free it correctly.
    Ok((fresh, area / 8))
}

/// Read-only reacquire signal, for the draw walker's callback. A mask freed with its row stops
/// calling its enter handler (the r20 run: freed-object reads from t=630 and no links-lost
/// line), and quests cross areas through loading screens, so the walk of the next area is where
/// discovery has to start again. Nothing is written here: the mask is reached through links this
/// plugin captured, so a stale read can only drop the addresses, never write into freed memory.
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
/// ours, and drops the addresses when the links that made it ours are gone: the r18 run showed
/// the plugin writing into a freed mask for 90 seconds after the row's scene ended, because the
/// root destructor never fires.
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
        // it to stop clipping here is the one disable this build can prove safe. r24's run left
        // the mask applying its last rectangle into the next area.
        MISSES.fetch_add(1, Ordering::Relaxed);
        snapshot(mask, bar, root);
        let _ = memory::call2(base() + ENABLE_SETTER, mask, 0);
        crate::hooks::teardown_gauge_row(root);
        return;
    }
    // The tier travels with the points; each tier is its own segment of the bar.
    let Some((current, maximum, tier)) = sampling::gauge_points() else {
        return;
    };
    let Some(ratio) = basis_points(Gauge {
        current: current as i32,
        maximum: maximum as i32,
        tier,
    }) else {
        return;
    };
    let rect = [
        memory::word(bar + 0xe0).unwrap_or(0) as i32,
        memory::word(bar + 0xe4).unwrap_or(0) as i32,
        memory::word(bar + 0xe8).unwrap_or(0) as i32,
        memory::word(bar + 0xec).unwrap_or(0) as i32,
    ];
    let Some(width) = memory::word(bar + MASK_SIZE).map(|w| w as i32) else {
        return;
    };
    let box_words = mask_box(rect, ratio, width);
    // The rectangle is the game's own arithmetic, and main+0xa4b390 shows its inputs, so these
    // are the writes that matter. The r23 run read the rectangle 225 times with width 0 and
    // height 32 at the parent's origin (110,38), against authored 128x32, which pins each input:

    // 1. The anchor modes, bits 0-3 of +0xa0: `& 0xc` is x and `& 3` is y, mode 0 spans
    //    0..size, 1 centres, 2 spans -size..0, 3 collapses the axis. The factory writes
    //    (word & 0xfffff000) | 0xff0 and leaves bits 0-3 as the block came from the allocator,
    //    and the class has no Anchor property in its DTI record, so bits 0-3 are recycled.
    //    r23 cleared them (its run read anchors=0xfffffff0) and the width stayed 0.
    if let Some(anchors) = memory::word(mask + MASK_ANCHORS) {
        if anchors & 0xf != 0 {
            let _ = memory::write_word(mask + MASK_ANCHORS, anchors & !0xf);
        }
    }
    // 2. The transform's x row, at +0x10/+0x14. The builder multiplies the authored width by
    //    +0x10 for x and by +0x14 for y, and the authored height by +0x20/+0x24. The r23 numbers
    //    (width 0 from an authored 128, height 32 from an authored 32) say the height row is
    //    (0,1) and the width row is (0,0): the factory copies its matrix template, and that
    //    template carries no x scale. Set the x scale to 1.0 and the authored width reaches the
    //    rectangle.
    let _ = memory::write_word(mask + MASK_X_SCALE, 1.0f32.to_bits());
    let _ = memory::write_word(mask + MASK_X_SCALE + 4, 0f32.to_bits());
    // 3. The authored pair. The height is the bar's own rect height, so the clip spans the bar
    //    top to bottom; the width is the ratio's share of it. The position at +0x80 is a DTI
    //    property that the row's layout resolves into the matrix for animated nodes, and this
    //    node has no animation entry, so writing it does nothing: the rectangle's origin comes
    //    from the parent's transform, which is the gauge's own corner (110,38 in r22/r23).
    let height = (rect[3] - rect[1]).max(1);
    let _ = memory::write_word(mask + MASK_SIZE, box_words[2] as u32);
    let _ = memory::write_word(mask + MASK_SIZE + 4, height as u32);
    let _ = memory::write_word(mask + MASK_FLAGS, REBUILD_AND_APPLY);
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
    // The r17 run read the row's two pairs: row+4 is 4 and 15, row+8 is 3 and 16, and the cache
    // sits 192 = 16 * 12 into the allocation. The sizing pair is row+8, and its objects field
    // equals the tree's node count; the map is the field that is one longer.
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
    // constructs it; r14's run shows the object it returns passes the class test and draws.
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
    let map = match grow_map(instance, root, nodes + 1, mask) {
        Ok(map) => map,
        Err(stage) => {
            if let Some(destructor) = owned {
                let _ = memory::call1(destructor, mask);
            }
            return Err(stage);
        }
    };
    let cache = match grow_cache(root, objects + 1, sequences) {
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
        && memory::call2(link_sibling, row.effect, row.edge).is_some();
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
    Ok(Outcome {
        mask,
        map,
        cache: cache.0,
        entries: nodes + 2,
        records: cache.1,
    })
}

/// The quad builder's inputs as the game holds them between frames, plus the bar's same fields
/// for comparison (the bar draws correctly, so its values are the working ones): the transform's
/// first two rows at +0x10, the position floats at +0x80, the authored pair at +0xa8, the anchor
/// word, the flags, and the built rectangle. Zeros mean nothing is inserted.
pub fn probe() -> [u32; 20] {
    let mask = MASK.load(Ordering::Relaxed);
    let bar = BAR.load(Ordering::Relaxed);
    let mut out = [0u32; 20];
    if mask == 0 {
        return out;
    }
    let mut at = |fields: &[(usize, u32, u32)], base: u32| {
        for (index, offset, _) in fields {
            out[*index] = memory::word(base.wrapping_add(*offset)).unwrap_or(0);
        }
    };
    const MASK_FIELDS: [(usize, u32, u32); 16] = [
        (0, 0x10, 0), (1, 0x14, 0), (2, 0x18, 0), (3, 0x1c, 0),
        (4, MASK_POS, 0), (5, MASK_POS + 4, 0), (6, MASK_SIZE, 0), (7, MASK_SIZE + 4, 0),
        (8, MASK_ANCHORS, 0), (9, MASK_FLAGS, 0),
        (10, MASK_RECT, 0), (11, MASK_RECT + 4, 0), (12, MASK_RECT + 8, 0), (13, MASK_RECT + 12, 0),
        (14, 0x20, 0), (15, 0x24, 0),
    ];
    const BAR_FIELDS: [(usize, u32, u32); 4] = [(16, 0x10, 0), (17, MASK_POS, 0), (18, MASK_SIZE, 0), (19, MASK_ANCHORS, 0)];
    at(&MASK_FIELDS, mask);
    if bar != 0 {
        at(&BAR_FIELDS, bar);
    }
    out
}

struct Read;
impl sharpness_gauge_core::Memory for Read {
    fn word(&self, address: u32) -> Option<u32> {
        memory::word(address)
    }
}
