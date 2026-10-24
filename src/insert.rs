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
    basis_points, mask_box, Gauge, NODE_CHILD, NODE_PARENT,
};

/// Written to the mask's mId at +0x50, where the factory leaves -1: it identifies our mask in
/// a tree the game owns, so a re-entry never inserts a second one and the enter hook can tell
/// our mask from every other scissor mask in the game.
pub const MARKER: u32 = 0x4d534731;
const MASK_ID: u32 = 0x50;
const MASK_OWNER: u32 = 0x74; // the owning cGUI, taken from the bar
const MASK_RESOURCE: u32 = 0x78; // the borrowed rGUI resource, taken from the bar
const MASK_POS: u32 = 0x80; // PosX/PosY floats
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
    /// Slot 0x1c: `memAlloc(bytes, align)`, the map allocation Key Quest Markers uses.
    fn alloc(&self, bytes: u32) -> Option<u32> {
        let f = memory::method(self.handle, 0x1c)?;
        memory::call2(f, self.handle, bytes).filter(|p| *p != 0)
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
    // The published block is what the game's destructor frees, so the old one goes now.
    allocator.release(old);
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
    if old_alloc != 0 && old_alloc != fresh {
        let _ = memory::call2(release, root, old_alloc);
    }
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
        // The links that made this mask ours are gone: an area's loading screen has taken the
        // row. Drop the addresses and re-arm discovery, so the new HUD row gets its own mask.
        MISSES.fetch_add(1, Ordering::Relaxed);
        crate::hooks::teardown_gauge_row(root);
        return;
    }
    let Some((current, maximum)) = sampling::gauge_points() else {
        return;
    };
    let Some(ratio) = basis_points(Gauge {
        current: current as i32,
        maximum: maximum as i32,
        tier: 0,
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
    // The anchor modes decide what the game's quad builder (main+0xa4b390) makes of the authored
    // pair: it reads `mask+0xa0 & 3` as the y mode and `& 0xc` as the x mode, where 0 spans
    // 0..size, 1 centres, 2 spans -size..0 and 3 collapses the extent to zero. The class factory
    // forces bits 4-11 and leaves bits 0-3 as the allocator handed them, and the game's own masks
    // get those bits from the row builder's resource record, which a factory-built mask has
    // none of. Bits 0-3 clear, the authored pair and the position are what the rebuild reads,
    // and the rectangle the handler applies comes out of that.
    //
    // This is the cause of the four failed runs: r14, r18, r20 and r22 all printed the same
    // signature (left == right, top/bottom = pos and pos + our height), which is the x axis
    // collapsed and the y axis spanning 0..size. It is also why writing the rectangle itself
    // did not help: the handler rebuilds it from the quad every frame.
    if let Some(anchors) = memory::word(mask + MASK_ANCHORS) {
        if anchors & 0xf != 0 {
            let _ = memory::write_word(mask + MASK_ANCHORS, anchors & !0xf);
        }
    }
    // No cache of the last box: the game rewrites its derived rectangle every frame, so the
    // inputs have to be written every frame. r20 wrote them once, because a remembered box
    // skipped the rest, and the gauge stayed hidden.
    let _ = memory::write_word(mask + MASK_SIZE, box_words[2] as u32);
    let _ = memory::write_word(mask + MASK_SIZE + 4, box_words[3] as u32);
    let _ = memory::write_bits(mask + MASK_POS, box_words[0] as f32);
    let _ = memory::write_bits(mask + MASK_POS + 4, box_words[1] as f32);
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

/// The mask's rectangle, flags and anchor modes as the game holds them between frames, next to
/// the bar's rect: the readback that shows what the layout pass left behind after the enter hook
/// wrote. Zeros mean nothing is inserted.
pub fn probe() -> [u32; 10] {
    let mask = MASK.load(Ordering::Relaxed);
    let bar = BAR.load(Ordering::Relaxed);
    let mut out = [0u32; 10];
    if mask == 0 {
        return out;
    }
    for index in 0..4 {
        out[index] = memory::word(mask + MASK_RECT + index as u32 * 4).unwrap_or(0);
    }
    out[4] = memory::word(mask + MASK_FLAGS).unwrap_or(0);
    // The anchor modes as the game holds them: bits 0-1 y, bits 2-3 x, both 0 for a full quad.
    out[9] = memory::word(mask + MASK_ANCHORS).unwrap_or(0);
    if bar != 0 {
        for index in 0..4 {
            out[5 + index] = memory::word(bar + 0xe0 + index as u32 * 4).unwrap_or(0);
        }
    }
    out
}

struct Read;
impl sharpness_gauge_core::Memory for Read {
    fn word(&self, address: u32) -> Option<u32> {
        memory::word(address)
    }
}
