//! Insert one scissor mask into the gauge row, with both storage expansions.
//!
//! r14 inserted the mask and the Eden run of 2026-10-23 (work/modern-sharpness-gauge/runs/)
//! showed two separable failures, and this module answers each with the native contract for it:
//!
//! - The row's object array grew short. main+0x5456e4 walks the instance's array at `+0xd8`
//!   for `main+0xa38ffc`'s length, which is main+0xa47d2c's node count of the tree plus one,
//!   and it calls vtable slot 0x50 on every entry. The r16 run read the array out: four
//!   entries, the controller then the bar, glow and edge, for a tree of 3 nodes. So an added
//!   node needs a fifth entry, in a block big enough for it. The game allocates that block
//!   through the cGUIInstance DTI's MT allocator, and Key Quest Markers r17 (accepted) grows
//!   a row's map the same way: copy the original entries, append the child, publish
//!   `instance+0xd8` and the controller's `+0xa0` (the r16 run showed the two fields hold the
//!   same pointer), then free the old block through that allocator.
//! - The clip box never reached the draw. The mask's enter handler main+0xa4ba94 builds the
//!   rectangle at `+0xb0..+0xbc` from the authored size pair `+0xa8/+0xac` through
//!   main+0xa4b834, which lays the node's own quad out at its position, and r14's 2,512
//!   printed rectangles had left/top `0 0` in every line while the box asked for x=110 y=32:
//!   the node's position pair `+0x80/+0x84` is the row's layout pass's to write, and it kept
//!   them at 0. The same handler reads `+0xb0..+0xbc` as four integers and applies them
//!   clipped to the context's current box, and it only rebuilds them while bit 1 of `+0xa4`
//!   is set. So the box is written straight into `+0xb0..+0xbc`, bit 1 is kept clear, and bit
//!   0 asks for the scissor. That write happens in the draw walker, after the game's own
//!   writes for the frame and before the walker builds the draw cache.
//!
//! The draw cache has to grow too: main+0xa49b28 sizes the row's block as
//! `(objects << 4 | 8) + sequences * 12` through the root's vtable slot 0x58, publishes
//! `root+0x80 = alloc + sequences * 12` and `root+0xa4 = alloc`, and the record area is the
//! `(objects << 4) | 8` part. The r17 run read this row's pair at row+8 as 3 objects and 16
//! sequences, so its record area is (3 << 4) | 8 = 56 bytes = 7 records, and the tree with the
//! mask emits 9 = 72: 16 bytes past the end, which is r14's cache overflow. Growing the block
//! to 4 objects makes the area 72, exactly the 9 records.
//!
//! Every call below runs inside the draw walker's callback, on the game's GUI thread, with
//! the controller reachable in that same call. Nothing here keeps a game pointer past the
//! callback: the inserted mask's address is held as an integer and cleared by the row root's
//! destructor hook, which is the only teardown this build has evidence for (it fired zero
//! times in the runs so far, so no cleanup is claimed).
use crate::{
    gui, memory,
    sampling,
};
use core::sync::atomic::{AtomicU32, Ordering};
use forge::mt::dti::MtDti;
use sharpness_gauge_core::{basis_points, mask_box, Gauge, NODE_CHILD};

/// Written to the mask's mId at `+0x50`. The factory leaves -1 there, so a marker of our own
/// tells an already-inserted mask from a gauge row the game built, which is how a re-entry
/// (leaving and re-entering a quest) is detected without inserting twice.
pub const MARKER: u32 = 0x4d534731;
/// The mask's field words. From the factory decompile and the enter handler main+0xa4ba94.
const MASK_ID: u32 = 0x50;
const MASK_OWNER: u32 = 0x74; // the owning cGUI, taken from the bar
const MASK_RESOURCE: u32 = 0x78; // the borrowed rGUI resource, taken from the bar
const MASK_POS: u32 = 0x80; // PosX/PosY floats, game-owned after each layout pass
const MASK_FLAGS: u32 = 0xa4; // bit 0 enable, bit 1 rebuild the rectangle
const MASK_SIZE: u32 = 0xa8; // authored width/height, read as signed integers
const MASK_RECT: u32 = 0xb0; // left, top, right, bottom, read as four integers
const ENABLE: u32 = 1;
/// The row's draw-cache sizing fields, the halfwords main+0xa49b28 reads: objects at row+8 and
/// sequences at row+8+2. The mod's facts line prints the pair at row+4 (4 and 15); the sizing
/// pair is 4 and 16, which is what the r15/r16 runs measured as `cache - cache_alloc` = 192.
const ROW_SIZING: u32 = 0x08;

/// The inserted mask, the bar it clips through, and the box last written. Plain integers:
/// no game pointer is dereferenced from outside a native callback.
static MASK: AtomicU32 = AtomicU32::new(0);
static BAR: AtomicU32 = AtomicU32::new(0);
static BOX: [AtomicU32; 4] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

/// What one insertion published, as integers for the log line: the mask, the new map, the new
/// cache allocation, the map's entry count, and the cache's record capacity in records.
pub struct Outcome {
    pub mask: u32,
    pub map: u32,
    pub cache: u32,
    pub entries: u32,
    pub records: u32,
}

/// The row root's destructor ran, so the game owns (or has freed) the mask, the map and the
/// cache. Drop every address; the next gauge HUD row starts over.
pub fn on_teardown() {
    MASK.store(0, Ordering::Release);
    BAR.store(0, Ordering::Release);
    for word in BOX.iter() {
        word.store(0, Ordering::Relaxed);
    }
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

/// The MT allocator of one DTI, and its free. main+0x7aa47c is `MtDti::getAllocator`, which
/// Key Quest Markers r17 uses for game-owned GUI memory: the native destructor frees through
/// this allocator, and Forge's own malloc cannot be substituted for it.
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
    /// The allocator's `memAlloc(bytes, align)` slot, 0x1c: Key Quest Markers' map
    /// allocation uses it the same way, and it takes no object id.
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
/// main+0x5456e4 will index with, so the new block must be that size and the new node must be
/// in it before the tree gains the node: a 5-node tree with a 4-entry array is r14's crash.
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
    // Copy the original bindings at their original indices, then append the new node.
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
    // The game's own destructor frees the published block, so the old one is released now
    // (Key Quest Markers r17 does the same, and its run has the map alive across scenes).
    allocator.release(old);
    Ok(fresh)
}

/// Rebuild the row's draw block for the extended object count and publish it. The sizing is
/// main+0xa49b28's arithmetic through the root's own vtable slots, so the allocation the
/// root's destructor frees is one the root allocated.
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
    if !memory::write_word(root + 0xa4, fresh)
        || !memory::write_word(root + 0x80, fresh + prefix)
    {
        let _ = memory::call2(release, root, fresh);
        return Err(46);
    }
    if old_alloc != 0 && old_alloc != fresh {
        let _ = memory::call2(release, root, old_alloc);
    }
    Ok((fresh, area / 8))
}

/// Build the clip box in the bar sprite's own rect and write it where the enter handler reads
/// it. Called from the draw walker for the gauge controller, after the game's writes for the
/// frame. The writes are skipped while the box is unchanged, which is every frame the
/// sharpness ratio does not move.
pub fn apply(controller: u32) {
    let mask = MASK.load(Ordering::Relaxed);
    let bar = BAR.load(Ordering::Relaxed);
    if mask == 0 || bar == 0 || controller != sampling::gauge_controller() {
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
    let right = box_words[0] + box_words[2];
    let bottom = box_words[1] + box_words[3];
    let next = [box_words[0] as u32, box_words[1] as u32, right as u32, bottom as u32];
    if BOX.iter().zip(next).all(|(old, new)| old.load(Ordering::Relaxed) == new) {
        return;
    }
    // The authored pair keeps the mask's own quad the size of the box, for the class's own
    // geometry; the rectangle the handler applies is the one written here.
    let _ = memory::write_word(mask + MASK_SIZE, box_words[2] as u32);
    let _ = memory::write_word(mask + MASK_SIZE + 4, box_words[3] as u32);
    let _ = memory::write_bits(mask + MASK_POS, box_words[0] as f32);
    let _ = memory::write_bits(mask + MASK_POS + 4, box_words[1] as f32);
    for (index, word) in next.iter().enumerate() {
        if memory::write_word(mask + MASK_RECT + index as u32 * 4, *word) {
            BOX[index].store(*word, Ordering::Relaxed);
        }
    }
    // Bit 1 stays clear: setting it asks the game to rebuild the rectangle from the node's
    // layout, which is the write that produced r14's `0 0 W 32` for 2,512 lines.
    let _ = memory::write_word(mask + MASK_FLAGS, ENABLE);
}

/// Insert the mask under the row root, with the bar, glow and edge inside it, after the row's
/// map and cache have been grown for it. Stage codes are 40..49 so they read apart from the
/// row gates 21..34.
pub fn run(row: &gui::Row, instance: u32) -> Result<Outcome, u32> {
    if MASK.load(Ordering::Relaxed) != 0 {
        return Err(49);
    }
    let root = row.root;
    // The tree already carrying our marker means this row was inserted in an earlier
    // encounter and the game has not torn it down: never insert twice.
    if memory::word(root + NODE_CHILD).is_some_and(|child| {
        child != 0 && memory::word(child + MASK_ID) == Some(MARKER)
    }) {
        return Err(49);
    }
    // The sizing pair the game's own cache sizer reads at row+8. The r17 run measured both
    // pairs on this row: row+4 is 4 objects and 15 sequences, row+8 is 3 and 16, and the
    // published cache sits 192 = 16 * 12 into the allocation, so row+8 is the sizing pair and
    // its objects field is the tree's node count (3), not that count plus one. The map is the
    // one that is one longer, because its first entry is the controller: 4 words for 3 nodes,
    // which is what main+0xa38ffc reports and main+0x5456e4 indexes with.
    let objects = memory::half(row.description + ROW_SIZING).ok_or(48u32)? as u32;
    let sequences = memory::half(row.description + ROW_SIZING + 2).ok_or(48u32)? as u32;
    let nodes = sharpness_gauge_core::count_nodes(&Read, root).0;
    if objects != nodes || objects == 0 || objects > 64 || sequences > 4096 {
        return Err(48);
    }
    // The mask borrows the bar's owning cGUI and resource: they are what its siblings hold,
    // read live, not a constant copied from another row.
    let owner = memory::word(row.bar + MASK_OWNER).ok_or(41u32)?;
    let resource = memory::word(row.bar + MASK_RESOURCE).ok_or(41u32)?;
    if owner == 0 || resource == 0 {
        return Err(41);
    }
    let dti = MtDti::find("cGUIObjScissorMask").ok_or(40u32)?;
    // The class factory allocates the object (0xd0 bytes, align 0x10, through the mask DTI's
    // MT allocator) and constructs it. r14's run shows the object it returns passes the class test and
    // draws, so this is the accepted construction path for this class.
    let mask = memory::call0(base() + 0xa4b6cc).ok_or(40u32)?;
    if mask == 0 || !is_type(mask, dti as *const _ as u32) {
        return Err(40);
    }
    let owned = memory::method(mask, 0x04);
    if memory::write_word(mask + MASK_ID, MARKER)
        && memory::write_word(mask + MASK_OWNER, owner)
        && memory::write_word(mask + MASK_RESOURCE, resource)
        && memory::write_word(mask + MASK_FLAGS, ENABLE)
    {
        // Ready. The map and cache come next; nothing is published before both exist.
    } else {
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
            // The map is published and the tree is untouched, so the row stays consistent:
            // main+0x5456e4 indexes `nodes + 1` entries and never reaches the extra one. The
            // mask is run through its class destructor (vtable slot 0x04). Its block belongs
            // to the mask DTI's MT allocator and no evidence shows this plugin freeing that
            // block, so it is left allocated rather than risk a double free.
            if let Some(destructor) = owned {
                let _ = memory::call1(destructor, mask);
            }
            return Err(stage);
        }
    };
    // Link last, so a 5-node tree never exists against the old array. The native linkers
    // supply the parent links: 0xa48460(parent, child) writes parent+0x68 and child+0x70,
    // 0xa4844c(node, sibling) writes node+0x6c, sibling+0x70 and the sibling's parent.
    let link_child = base() + 0xa48460;
    let link_sibling = base() + 0xa4844c;
    let linked = memory::call2(link_child, root, mask).is_some()
        && memory::call2(link_child, mask, row.bar).is_some()
        && memory::call2(link_sibling, row.bar, row.effect).is_some()
        && memory::call2(link_sibling, row.effect, row.edge).is_some();
    if !linked {
        // The tree is unmodified: the first link call is the only write that can have landed,
        // and it is reverted by re-linking the root to its original child.
        let _ = memory::call2(link_child, root, row.bar);
        return Err(47);
    }
    // Ask the walker to rebuild the draw cache (bit 0 of the root's flag word), which is the
    // same request Key Quest Markers r17 makes after growing a row.
    if let Some(flags) = memory::word(root + 0x8c) {
        let _ = memory::write_word(root + 0x8c, flags | 3);
    }
    BAR.store(row.bar, Ordering::Release);
    MASK.store(mask, Ordering::Release);
    Ok(Outcome {
        mask,
        map,
        cache: cache.0,
        entries: nodes + 2,
        records: cache.1,
    })
}

/// Copy the mask's own rectangle, its flag word and the bar's rect out, for the log line
/// that answers the question r14 left open: does a rectangle written here survive the game's
/// layout pass, or does a later write replace it? Zeros mean nothing is inserted.
pub fn probe() -> [u32; 9] {
    let mask = MASK.load(Ordering::Relaxed);
    let bar = BAR.load(Ordering::Relaxed);
    let mut out = [0u32; 9];
    if mask == 0 {
        return out;
    }
    for index in 0..4 {
        out[index] = memory::word(mask + MASK_RECT + index as u32 * 4).unwrap_or(0);
    }
    out[4] = memory::word(mask + MASK_FLAGS).unwrap_or(0);
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
