//! Insert one scissor mask into the gauge row, with the two storage expansions it needs.
//!
//! The native contracts this rests on, all read from the decompiles recorded in
//! work/world-sharpness-gauge/PHASE2_RESEARCH.md:
//!
//! - The row's object array at `instance+0xd8` (aliased at `root+0xa0`) is indexed by
//!   main+0x5456e4 with main+0xa47d2c's node count + 1, dispatching vtable slot 0x50 on each
//!   entry. It holds one word per object with the controller first, so each added node needs
//!   its own word: the inserted tree's 6 nodes take 7. It is grown through the `cGUIInstance` DTI's MT allocator, the Key Quest
//!   Markers pattern: Forge's malloc cannot free what the game's destructor frees.
//! - The draw block is sized by main+0xa49b28 as `(objects << 4 | 8) + sequences * 12` from the
//!   halfwords at row+8, allocated through the root's vtable slot 0x58, with the walker's
//!   pointer at `alloc + sequences * 12`. This row is sized for 3 objects and 16 sequences, so
//!   its record area holds 7 records, and the inserted tree emits 13: the insertion has to grow
//!   the block.
//! - The mask's rectangle at `+0xb0..+0xbc` is derived: the enter handler main+0xa4ba94 rebuilds
//!   it from the node's own transform and authored size while `+0xa4` bit 1 is set, then applies
//!   it. So its inputs are written at that handler, in the same call that reads them, and the
//!   rectangle words themselves are never written.
//!
//! `run` executes from `forge_onUpdate` through `gui::tick`, and `enter` inside the mask's enter
//! hook during the draw walk. No game pointer is kept between calls: addresses are plain integers,
//! cleared by the liveness checks in `check_liveness` and `enter`.
use crate::{gui, memory, sampling};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU8, Ordering};
use forge::mt::dti::MtDti;
use sharpness_gauge_core::{
    cut_box, dim_box, NODE_CHILD, NODE_DRAWABLE, NODE_PARENT, NODE_RECT, NODE_SIBLING,
    NODE_TEXTURE_RECT,
};

/// Written to the mask's mId at +0x50, where the factory leaves -1: it identifies our mask in
/// a tree the game owns, so a re-entry never inserts a second one and the enter hook can tell
/// our mask from every other scissor mask in the game.
pub const MARKER: u32 = 0x4d534731;
const MASK_ID: u32 = 0x50;
const MASK_OWNER: u32 = 0x74; // the owning cGUI, taken from the bar
const MASK_RESOURCE: u32 = 0x78; // the borrowed rGUI resource, taken from the bar
const MASK_X_SCALE: u32 = 0x10; // the transform's x row: the builder multiplies the width by it
const MASK_TRANSLATION: u32 = 0x40; // the transform's translation pair: the box's corner, as floats
/// The class's DTI `Enable` setter (main+0xa4ba20, from the DTI record), the native way to
/// stop a scissor mask clipping.
const ENABLE_SETTER: u32 = 0xa4ba20;
const MASK_ANCHORS: u32 = 0xa0; // bits 0-1 anchor +0xa8, bits 2-3 anchor +0xac
const MASK_FLAGS: u32 = 0xa4; // bit 0 enable the scissor, bit 1 rebuild the rectangle
const MASK_SIZE: u32 = 0xa8; // authored width/height, signed integers
/// bit 0 applies it. The rebuild is the game's own arithmetic, and its inputs are the fields
/// written above it.
const REBUILD_AND_APPLY: u32 = 3;
/// The halfwords main+0xa49b28 sizes the draw cache from: objects at row+8, sequences at
/// row+8+2. The pair at row+4 is the row's own reported counts; the sizer does not read it.
const ROW_SIZING: u32 = 0x08;

// The dim blade: a second copy of the blade sprite, outside the mask, in a flat
// dark colour. Its class is `cGUIObjTexture`, size 304. Construction and allocation follow the
// contract the Key Quest Markers badge runs on: the class's DTI has three virtual slots and no
// placement constructor, so the standalone constructor main+0xa4f60c takes the storage in r0 and
// returns it, and the DTI's allocator allocates through slot 0x20 with (bytes, align, dti id).
// The field offsets are from the DTI record and the setter decompiles, recorded in
// work/world-sharpness-gauge/PHASE5_RESEARCH.md.
pub const DIM_MARKER: u32 = 0x4d534732;
/// The shadow colour at all four corners: `#202326`. The corner word is alpha, blue, green, red - a
/// copy drawn with `0xFFFF00FF` comes out magenta on screen, and the bar's own `0xff5efaff` is the
/// yellow tier - so the colour is `0xFF262320`.
const DIM_COLOUR: u32 = 0xFF26_2320;
/// The shade's clip needs one more node: drawing is gated by the render state's scissor box, set
/// only by a mask's enter or a texture's enter under `Mask` 3 or 6 (main+0xa37c1c). A mask builds its
/// box over its own transform at `+0x10..+0x48` and its authored pair: `corner .. corner +
/// authored`, with the corner the node's translation at `+0x40/+0x44` (main+0xa4b390, called by the
/// mask's enter handler main+0xa4ba94 with the node itself; it reads no parent field and no
/// rectangle at `+0xe0`, which is past the 0xd0-byte mask object). So the corner is a write: the
/// clip mask's translation goes to the cut's right edge and its authored width spans forward from
/// there to the blade's end, which clips the copy to the part of the blade
/// past the cut.
const CLIP_MARKER: u32 = 0x4d53_4734;
const MASK_FACTORY: u32 = 0xa4b6cc;
/// The style setters Key Quest Markers' badge runs on this class, in its order and with its
/// values, each through the two-integer ABI `(object, value)`. `Mask = 0` is the one that draws
/// the corner colours: `Mask = 1` enters the native mask-writing pass. The last one, SamplerState,
/// takes the bar's own value, not the badge's 0: see `build_dim`.
const STYLE_SETTERS: [(u32, u32); 12] = [
    (0xa4dc24, 1), // BlendState
    (0xa4a9f4, 0), // ControlPoint
    (0xa47a94, 0), // FixFrame
    (0xa4dca0, 0), // Mask
    (0xa47a04, 2), // Priority
    (0xa4aa24, 1), // ReferencePosition
    (0xa4aa5c, 1), // ReferenceRotation
    (0xa4aa94, 3), // ReferenceScaleX
    (0xa4aac8, 3), // ReferenceScaleY
    (0xa4aafc, 3), // ReferenceScaleZ
    (0xa4dc54, 0), // Reverse
    (0xa4fa5c, 0), // SamplerState, replaced by the bar's live low nibble in build_dim
];
/// The `mTextureRect` setter (main+0xa4fb58): `(object, *const f32[4])`. It returns at once when
/// the four floats already match. Otherwise it copies the four floats into `+0x10c`, calls the
/// class's vtable slot 0x28 when the flag byte at +0x108 has bits 4/5 set, and always tail-calls
/// slot 0x50 (main+0xa4fc68) - the only writer of the +0xc0 bits 24..26, the vertex cache dirty
/// mark the draw slot reads (core's `NODE_TEXTURE_RECT`).
const TEXTURE_RECT_SETTER: u32 = 0xa4fb58;
const TEXTURE_SIZE: u32 = 304;
const TEXTURE_CTOR: u32 = 0xa4f60c;
const TEXTURE_ALLOC_SLOT: u32 = 0x20;
const TEXTURE_SETTER: u32 = 0xa4faf0;
const TEX_MID: u32 = 0x50;
/// The `cGUI` whose texture list the `Texture` setter resolves an id through: main+0xb80fd0
/// scans `*(obj+0x64) + 0xe8` for `count` entries of 16 bytes at `+0x70`. The class's base
/// constructor zeroes this word, so a standalone copy has to be given the bar's.
const TEX_GUI: u32 = 0x64;
/// The flag word the draw walk reads: bit 17 and bit 19 below, seen live as
/// `bar_flags=41943045` on the bar. Neither one is the drawable bit; that is bit 25, core's
/// `NODE_DRAWABLE`.
const TEX_FLAGS: u32 = 0x54;
/// Bit 17, transform dirty. The transform walk (main+0xa4964c, started by root+0x8c bit 1) calls a
/// node's setMatrix - vtable slot 0x44, main+0xa4e604 for a texture, which composes the parent's
/// `+0x10` matrix with the node's own position and `Reference` flags - only on a node with this bit
/// set (main+0xa4969c), and it passes the bit to the children only of a node that has it itself
/// (main+0xa496e0..0xa496f0). A node without it keeps the matrix its constructor left, so it is
/// never composed with its parent and its subtree keeps the bar's transform, not the gauge row's.
const TRANSFORM_DIRTY: u32 = 0x0002_0000;
/// Bits 17 and 19: transform dirty, and the child re-sort bit - main+0xa4915c uses bit 19 to re-sort
/// a node's children by Priority (`+0x60`) and then clears it, so it is not a layout bit. Set on the
/// nodes the walk has to compose with their parent: the accepted mask and the copy at the build.
const DRAWABLE_AND_DIRTY: u32 = 0x000a_0000;
const TEX_OWNER: u32 = 0x74; // the owning cGUI, taken from the bar
const TEX_RESOURCE: u32 = 0x78; // the borrowed rGUI resource, taken from the bar
const TEX_POSITION: u32 = 0x80; // DTI PosX/PosY, float pixels
const TEX_AUTHORED: u32 = 0xb4; // authored width/height, float pixels
const TEX_COLOURS: u32 = 0xa4; // four corner words: LT, RT, LB, RB
const TEX_TEXTURE_ID: u32 = 0x100; // the DTI setter's texture id
const TEX_TEXTURE_OBJECT: u32 = 0x104; // the object the setter resolves from that id
/// The style byte the `SamplerState` setter (main+0xa4fa5c) writes its value into, low nibble, and
/// whose bits 4/5 the `mTextureRect` setter tests. The bar's own value is what its draw filters with.
const TEX_SAMPLER: u32 = 0x108;
static DIM: AtomicU32 = AtomicU32::new(0);
/// The shade's clip mask node, 0 when the shade has no clip.
static CLIP: AtomicU32 = AtomicU32::new(0);

/// The atlas window of a node, in float pixels: the bar's live blade window, and the authored
/// pair, are stored as floats.
fn window_at(node: u32, offset: u32) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for edge in 0..4 {
        out[edge] = f32::from_bits(memory::word(node + offset + edge as u32 * 4).unwrap_or(0));
    }
    out
}

/// Times a check found the mask's links gone. The enter hook catches a mask still being entered
/// after its row's links broke, the one place the game proves it live; the draw walker's check
/// catches a mask freed with its row, whose enter is no longer called.
static MISSES: AtomicU32 = AtomicU32::new(0);
static MASK: AtomicU32 = AtomicU32::new(0);
static BAR: AtomicU32 = AtomicU32::new(0);
static ROOT: AtomicU32 = AtomicU32::new(0);
/// The bar's box as last drawn, when the draw's matrix differs from the node's: a plugin rescaled it.
static DRAWN: [AtomicI32; 4] = [const { AtomicI32::new(0) }; 4];
static DRAWN_SET: AtomicBool = AtomicBool::new(false);
/// One report per row of the modelled box against the game's rect: 0 due, 1 ready to log, 2 logged.
static RECT_REPORT: AtomicU32 = AtomicU32::new(0);
static REPORT_RECT: [AtomicI32; 4] = [const { AtomicI32::new(0) }; 4];
static REPORT_PLAIN: AtomicBool = AtomicBool::new(false);
/// Whether `sampling::fill` refused this row: 0 not refused, 1 refused and not logged yet, 2
/// logged. One log line per row, so a row stuck on the stock bar cannot flood the log.
static FILL_REFUSED: AtomicU8 = AtomicU8::new(0);

/// The four words of a rectangle field as signed pixels (the bar's screen rect).
pub fn is_bar(node: u32) -> bool {
    node != 0 && BAR.load(Ordering::Relaxed) == node
}

/// The matrix block of the context's latest upload: main+0xa4eb9c and main+0xa3e3b0 store its address
/// at `*(context+4)+0x18bc`. Zero when unreadable.
pub fn uploaded_block(context: u32) -> u32 {
    memory::word(context + 4)
        .and_then(|target| memory::word(target + 0x18bc))
        .unwrap_or(0)
}

/// Record where the bar really drew. A draw that uploaded nothing leaves the block address as it
/// was, and that block belongs to another node. The block's words 0..13 are the node's `+0x10`
/// matrix unless a plugin edited them, so equal words mean a plain draw and the game's rect stays
/// the source. Only a rescaled bar reaches the model, so a plain game's cut never depends on it.
pub fn capture_drawn(node: u32, context: u32, before: u32) {
    let block = uploaded_block(context);
    if block == 0 || block == before {
        return;
    }
    let mut drawn = [0u32; 6];
    let mut native = [0u32; 6];
    for (slot, index) in [0u32, 1, 4, 5, 12, 13].into_iter().enumerate() {
        match (memory::word(block + index * 4), memory::word(node + 0x10 + index * 4)) {
            (Some(d), Some(n)) => (drawn[slot], native[slot]) = (d, n),
            _ => return,
        }
    }
    let plain = drawn == native;
    let (Some(anchors), Some(flags), Some(width), Some(height)) = (
        memory::word(node + 0xa0),
        memory::word(node + 0x54),
        memory::word(node + 0xd8),
        memory::word(node + 0xdc),
    ) else {
        return;
    };
    let box_words = sharpness_gauge_core::drawn_rect(
        drawn.map(f32::from_bits),
        anchors,
        flags & 0x0040_0000 != 0,
        [width as i32, height as i32],
    );
    if RECT_REPORT.load(Ordering::Relaxed) == 0 {
        for (cell, value) in REPORT_RECT.iter().zip(box_words.unwrap_or([0; 4])) {
            cell.store(value, Ordering::Relaxed);
        }
        REPORT_PLAIN.store(plain, Ordering::Relaxed);
        RECT_REPORT.store(1, Ordering::Release);
    }
    match box_words {
        Some(words) if !plain => {
            for (cell, value) in DRAWN.iter().zip(words) {
                cell.store(value, Ordering::Relaxed);
            }
            DRAWN_SET.store(true, Ordering::Release);
        }
        _ => DRAWN_SET.store(false, Ordering::Release),
    }
}

/// The box the cut and the shade work from.
fn bar_rect(bar: u32) -> [i32; 4] {
    if DRAWN_SET.load(Ordering::Acquire) {
        return core::array::from_fn(|edge| DRAWN[edge].load(Ordering::Relaxed));
    }
    rect_at(bar, NODE_RECT)
}

/// The modelled box, the game's rect and whether the draw was plain, once per row, for the log.
pub fn take_rect_report() -> Option<([i32; 4], [i32; 4], bool)> {
    let bar = BAR.load(Ordering::Relaxed);
    if bar == 0
        || RECT_REPORT
            .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
    {
        return None;
    }
    Some((
        core::array::from_fn(|edge| REPORT_RECT[edge].load(Ordering::Relaxed)),
        rect_at(bar, NODE_RECT),
        REPORT_PLAIN.load(Ordering::Relaxed),
    ))
}

fn rect_at(node: u32, offset: u32) -> [i32; 4] {
    let mut out = [0i32; 4];
    for edge in 0..4 {
        out[edge] = memory::word(node + offset + edge as u32 * 4).unwrap_or(0) as i32;
    }
    out
}

/// What one insertion published: the nodes, the map and cache sizes, and the stage a failed shade build stopped at.
pub struct Outcome {
    pub mask: u32,
    pub map: u32,
    pub cache: u32,
    pub entries: u32,
    pub records: u32,
    /// The dim blade's node, 0 when it was not built.
    pub dim: u32,
    /// The shade's clip mask node, 0 when the shade has no clip: its transform translation is the
    /// box's corner and its authored width is the part of the blade past the cut, and the copy is
    /// drawn inside it.
    pub clip: u32,
    /// The stage its build stopped at, 0 when it was built: 50 the allocation, 51 the
    /// constructor and class test, 52 the fields. A failed build leaves the accepted cut alone.
    pub dim_stage: u32,
}

/// The row is going away: the game owns the mask, the map and the cache. Called from the mask's
/// link checks.
pub fn on_teardown() {
    FILL_REFUSED.store(0, Ordering::Relaxed);
    ROOT.store(0, Ordering::Release);
    MASK.store(0, Ordering::Release);
    BAR.store(0, Ordering::Release);
    DRAWN_SET.store(false, Ordering::Release);
    RECT_REPORT.store(0, Ordering::Release);
    DIM.store(0, Ordering::Release);
    CLIP.store(0, Ordering::Release);
}

/// True once per row, the first time `forge_onUpdate` asks after a refused fill.
pub fn take_fill_refused() -> bool {
    FILL_REFUSED
        .compare_exchange(1, 2, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
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
    /// Slot 0x20 is the allocate the class's objects are built with: `(bytes, align, dti id)`,
    /// the id the destructor frees the block by.
    fn alloc_text(&self, bytes: u32, id: u32) -> Option<u32> {
        let f = memory::method(self.handle, TEXTURE_ALLOC_SLOT)?;
        memory::call4(f, self.handle, bytes, 16, id).filter(|p| *p != 0)
    }
    fn release(&self, pointer: u32) {
        if pointer != 0 {
            let _ = memory::call2(self.free, self.handle, pointer);
        }
    }
}

/// Build the node the shade's clip needs. It comes from the class factory the accepted mask build
/// uses (main+0xa4b6cc: it allocates the 0xd0-byte object through the mask DTI's MT allocator and
/// constructs it), borrows the bar's owning cGUI and resource, and carries its own marker word so
/// the accepted mask's enter hook ignores it.
///
/// It gets the accepted rebuild-and-apply pair, so its box is rebuilt from its translation and
/// authored pair every frame, and its scope clips whatever is drawn inside it. Its flags carry bit 17
/// (main+0xa4964c / main+0xa4969c): the transform walk composes a node with its parent's matrix only
/// while that bit is set, and passes it down, so without it the clip node and the copy under it keep
/// the identity matrix their constructors left and never take the gauge row's transform.
fn build_clip_node(owner: u32, resource: u32) -> Result<u32, u32> {
    let dti = MtDti::find("cGUIObjScissorMask").ok_or(53u32)?;
    let node = memory::call0(base() + MASK_FACTORY).ok_or(53u32)?;
    if node == 0 || !is_type(node, dti as *const _ as u32) {
        return Err(53);
    }
    let owned = memory::method(node, 0x04);
    let ready = memory::write_word(node + MASK_ID, CLIP_MARKER)
        && memory::write_word(node + MASK_OWNER, owner)
        && memory::write_word(node + MASK_RESOURCE, resource)
        // The node has to stay drawable, or the walker skips the subtree under it; the scissor is a
        // separate word at +0xa4. Bit 17 is the walk's own gate (main+0xa4964c / main+0xa4969c): with
        // it the walk calls this node's setMatrix with the root's matrix and pushes it down to the
        // copy; without it this node and the copy keep their constructor's matrix.
        && memory::word(node + TEX_FLAGS).is_some_and(|word| {
            memory::write_word(node + TEX_FLAGS, word | NODE_DRAWABLE | 1 | TRANSFORM_DIRTY)
        })
        && memory::write_word(node + MASK_FLAGS, REBUILD_AND_APPLY);
    if !ready {
        if let Some(destructor) = owned {
            let _ = memory::call1(destructor, node);
        }
        return Err(54);
    }
    Ok(node)
}

/// Grow the row's object array by one entry and publish both aliases. `entries` is the length
/// main+0x5456e4 indexes with, so the block and the new entry have to be in place before the
/// tree gains a node: a fifth node against a four-word array is read past its end.
fn grow_map(instance: u32, root: u32, entries: u32, node: u32, extra: [u32; 3]) -> Result<u32, u32> {
    let old = memory::word(instance + 0xd8).ok_or(43u32)?;
    if old == 0 || memory::word(root + 0xa0).ok_or(43u32)? != old {
        return Err(43);
    }
    let allocator = Allocator::find("cGUIInstance").ok_or(42u32)?;
    let appended = 1 + extra.iter().filter(|word| **word != 0).count() as u32;
    let bytes = (entries + appended) * 4;
    let fresh = match allocator.alloc(bytes) {
        Some(pointer) if pointer & 15 == 0 => pointer,
        Some(pointer) => {
            allocator.release(pointer);
            return Err(42);
        }
        None => return Err(42),
    };
    let mut published = memory::copy_words(old, fresh, entries as usize)
        && memory::write_word(fresh + entries * 4, node);
    for slot in 0..3usize {
        if extra[slot] != 0 {
            published = published && memory::write_word(fresh + (entries + slot as u32 + 1) * 4, extra[slot]);
        }
    }
    if !published {
        allocator.release(fresh);
        return Err(43);
    }
    if !memory::write_word(instance + 0xd8, fresh) || !memory::write_word(root + 0xa0, fresh) {
        allocator.release(fresh);
        return Err(43);
    }
    // shortcut: the old 4-word block leaks, 16 bytes per quest. The game allocates a row's map
    // through this DTI's allocator (FUN_01401bc0, align 16) and frees it the same way in
    // setAnimationId main+0xa38828, so releasing it here should be sound, but no run has tried
    // it. Upgrade: release it through `allocator` in a version that gets a run.
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
    // goes back the same way. Leaving it unreleased stops the bar drawing, so the replacement
    // only works with the old block returned to the root's allocator.
    if old_alloc != 0 && old_alloc != fresh {
        let _ = memory::call2(release, root, old_alloc);
    }
    Ok((fresh, area / 8))
}

/// Build the dim blade: a `cGUIObjTexture` allocated through its own DTI's MT allocator (slot
/// 0x20 with the DTI id) and constructed by the class's standalone constructor on a zeroed block,
/// then given the bar's owner, resource, texture id, sampler state, authored size, position and flat
/// colours. Nothing is linked and no field is copied from the bar: the node has no animation entry, so
/// no keyframe writes to it, which is what keeps it constant and non-pulsing and leaves `Txt_sharp`'s
/// own keyframes untouched. Stages 50..52.
fn build_dim(bar: u32) -> Result<u32, u32> {
    let dti = MtDti::find("cGUIObjTexture").ok_or(50u32)?;
    let (owner, resource, gui, texture) = (
        memory::word(bar + TEX_OWNER).ok_or(50u32)?,
        memory::word(bar + TEX_RESOURCE).ok_or(50u32)?,
        memory::word(bar + TEX_GUI).ok_or(50u32)?,
        memory::word(bar + TEX_TEXTURE_ID).ok_or(50u32)?,
    );
    if owner == 0 || resource == 0 || texture == 0 {
        return Err(50);
    }
    // No cGUI to resolve a texture id through: the setter would leave the texture object 0, so
    // say so at the stage that reports it instead of building a node that cannot draw.
    if gui == 0 {
        return Err(52);
    }
    // The bar's live quad, in floats: its authored pair (88.0 x 24.0 on the measured bar) and its
    // position pair. The shadow keeps this quad for its whole life; the per-frame write moves the
    // clip mask over it, which is what limits the part that shows.
    let (authored, bar_position) = (window_at(bar, TEX_AUTHORED), window_at(bar, TEX_POSITION));
    let (copy_position, copy_size) = ([bar_position[0], bar_position[1]], [authored[0], authored[1]]);
    // The bar's own sampler state, the low nibble of the style byte at +0x108 where main+0xa4fa5c
    // stores it. `hud0-sharpness.json` authors Txt_sharp with SamplerState 2; on the badge's 0 the
    // copy's edges are filtered by a different sampler than the blade part it sits beside.
    let sampler = memory::word(bar + TEX_SAMPLER).ok_or(50u32)? & 0xf;
    let allocator = Allocator::find("cGUIObjTexture").ok_or(50u32)?;
    let node = match allocator.alloc_text(TEXTURE_SIZE, dti.id()) {
        Some(pointer) if pointer & 15 == 0 => pointer,
        Some(pointer) => {
            allocator.release(pointer);
            return Err(50);
        }
        None => return Err(50),
    };
    // The block comes from a recycled pool, so it arrives as freed-heap fill: constructor inputs
    // and every field the constructor leaves alone have to read 0 first.
    if !memory::zero_words(node, (TEXTURE_SIZE / 4) as usize) {
        allocator.release(node);
        return Err(50);
    }
    let built = memory::call1(base() + TEXTURE_CTOR, node).is_some_and(|got| got == node);
    if !built
        || !is_type(node, dti as *const _ as u32)
        || !memory::writable(node, TEXTURE_SIZE as usize)
    {
        // Not constructed: the block is still ours, so it goes back to its allocator.
        allocator.release(node);
        return Err(51);
    }
    // Constructed: the native destructor owns the block from here. A failure calls the
    // destructor and never the allocator's free - the destructor frees through this allocator.
    let owned = memory::method(node, 0x04);
    // The Key Quest Markers order: identity and ownership, the quad, the flags, the style setters,
    // the colours, the texture id last (its first test is the resource at +0x78, so it runs after
    // that word is written), then the atlas rectangle through its setter, given the bar's own four
    // floats at `+0x10c` so the copy starts on the band the bar is drawing.
    let ready = memory::write_word(node + TEX_MID, DIM_MARKER)
        && memory::write_word(node + TEX_GUI, gui)
        && memory::write_word(node + TEX_OWNER, owner)
        && memory::write_word(node + TEX_RESOURCE, resource)
        && memory::write_word(node + TEX_POSITION, copy_position[0].to_bits())
        && memory::write_word(node + TEX_POSITION + 4, copy_position[1].to_bits())
        && memory::write_word(node + TEX_AUTHORED, copy_size[0].to_bits())
        && memory::write_word(node + TEX_AUTHORED + 4, copy_size[1].to_bits())
        && memory::word(node + TEX_FLAGS).is_some_and(|flags| {
            memory::write_word(node + TEX_FLAGS, flags | DRAWABLE_AND_DIRTY)
        })
        && STYLE_SETTERS
            .iter()
            .enumerate()
            .all(|(index, (setter, value))| {
                // The last setter is SamplerState, and it takes the bar's own state.
                let value = if index + 1 == STYLE_SETTERS.len() {
                    sampler
                } else {
                    *value
                };
                memory::call2(base() + setter, node, value).is_some()
            })
        && (0..4usize).all(|corner| {
            memory::write_word(node + TEX_COLOURS + corner as u32 * 4, DIM_COLOUR)
        })
        && memory::call2(base() + TEXTURE_SETTER, node, texture)
            .is_some_and(|_| memory::word(node + TEX_TEXTURE_OBJECT).is_some_and(|p| p != 0))
        && memory::call2(base() + TEXTURE_RECT_SETTER, node, bar + NODE_TEXTURE_RECT).is_some();
    if !ready {
        if let Some(destructor) = owned {
            let _ = memory::call1(destructor, node);
        }
        return Err(52);
    }
    Ok(node)
}

/// Write the shade for this frame's cut. The clip mask's translation (`+0x40/+0x44`) is the shade
/// box's left edge and the bar rect's top, and its authored pair (`+0xa8/+0xac`) is the box's width
/// and height, so the box it builds spans the part of the blade past the cut, and the copy drawn
/// inside it is clipped to that. The copy's atlas window is the bar's own four floats at `+0x10c`,
/// given to their setter every frame, so the shade draws the tier frame the bar draws: the notched,
/// worn blade at orange and red, the straight-bottomed one at yellow and above. The copy's own quad,
/// rect and flags are not touched: its rect is the bar's layout's output, and a node's rect does not
/// gate its drawing - the scissor box is.
fn write_shade(dim: u32, clip: u32, bar: u32, rect: [i32; 4], ratio_bp: u32) {
    // The setter is the only way to the copy's window. main+0xa4fb58 stores the four floats at
    // +0x10c and tail-calls slot 0x50 (main+0xa4fc68), the only writer of the +0xc0 bits 24..26;
    // it returns at once when the floats already match, so the mark is set exactly when the bar's
    // band moves. The texture's draw slot main+0xa4fc9c rebuilds its quad from +0x10c..+0x118 and
    // the colours at +0xa4 only while (global+0x98 & (node+0xc0 >> 24) & 7) is nonzero, and clears
    // those bits (main+0xa4feb4): the drawn quad carries the window as of its last rebuild, so a
    // raw write to +0x10c marks nothing and never repaints - the copy keeps the band that was in
    // +0x10c when its quad was first built.
    let _ = memory::call2(base() + TEXTURE_RECT_SETTER, dim, bar + NODE_TEXTURE_RECT);
    let box_words = if ratio_bp == u32::MAX {
        [rect[2], rect[1], rect[2], rect[3]]
    } else {
        dim_box(rect, ratio_bp)
    };
    let width = (box_words[2] - box_words[0]).max(1) as u32;
    let height = (rect[3] - rect[1]).max(1) as u32;
    // The box's corner is the mask node's own matrix translation: main+0xa4b390 reads the node's
    // transform and nothing else, so this is the only lever that moves the shade along the blade.
    // The authored width spans forward from it, so the translation is the box's left edge.
    let _ = memory::write_word(clip + MASK_TRANSLATION, (box_words[0] as f32).to_bits());
    let _ = memory::write_word(clip + MASK_TRANSLATION + 4, (box_words[1] as f32).to_bits());
    if let Some(anchors) = memory::word(clip + MASK_ANCHORS) {
        if anchors & 0xf != 0 {
            let _ = memory::write_word(clip + MASK_ANCHORS, anchors & !0xf);
        }
    }
    let _ = memory::write_word(clip + MASK_X_SCALE, 1.0f32.to_bits());
    let _ = memory::write_word(clip + MASK_X_SCALE + 4, 0f32.to_bits());
    let _ = memory::write_word(clip + MASK_SIZE, width);
    let _ = memory::write_word(clip + MASK_SIZE + 4, height);
    let _ = memory::write_word(clip + MASK_FLAGS, REBUILD_AND_APPLY);
}

/// Read-only reacquire signal, for the draw walker's callback. A mask freed with its row stops
/// calling its enter handler, so the walker is where the next quest's discovery has to start
/// again. Nothing is written here: the mask is
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
    crate::hooks::teardown_gauge_row(root);
}

/// The mask's enter handler is about to read `+0xb0..+0xbc` and apply the scissor, so the box
/// is written here, in this call. Returns without touching anything for a mask that is not
/// ours, and drops the addresses when the links that made it ours are gone: a mask whose row's
/// links broke can still be entered, and it would keep applying its last box. A mask freed with
/// its row is not entered at all, which is `check_liveness`'s case.
pub fn enter(mask: u32) -> Option<[u32; 2]> {
    if memory::word(mask + MASK_ID) != Some(MARKER) {
        return None;
    }
    let (bar, root) = (BAR.load(Ordering::Relaxed), ROOT.load(Ordering::Relaxed));
    let alive = bar != 0
        && root != 0
        && memory::word(mask + NODE_PARENT) == Some(root)
        && memory::word(root + NODE_CHILD) == Some(mask)
        && memory::word(bar + NODE_PARENT) == Some(mask);
    if !alive {
        // The links that made this mask are gone: the row is going away. The game is calling
        // this mask's handler right now, so it is live, and asking it to stop clipping here is
        // the one disable this build can prove safe. Left alone, the mask keeps applying its
        // last rectangle.
        MISSES.fetch_add(1, Ordering::Relaxed);
        let _ = memory::call2(base() + ENABLE_SETTER, mask, 0);
        crate::hooks::teardown_gauge_row(root);
        return None;
    }
    let (dim, clip) = (DIM.load(Ordering::Relaxed), CLIP.load(Ordering::Relaxed));
    // The live fill: the local player's sharpness points measured inside the tier band they sit
    // in, so the bar refills at every colour change while the game keeps colouring it. No fill -
    // no player the session still names, a zero maximum, a tier the native rule cannot answer -
    // is unsupported state, so the clip is turned off here and the native gauge is left as the
    // game draws it. `sampling::fill` runs its safety gates before any native call.
    let ratio = match sampling::fill() {
        Some(bp) => bp,
        None => {
            let _ = FILL_REFUSED.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed);
            let _ = memory::call2(base() + ENABLE_SETTER, mask, 0);
            // No gauge to cut by: the shadow goes to width 0, the accepted appearance.
            if dim != 0 && clip != 0 {
                write_shade(dim, clip, bar, bar_rect(bar), u32::MAX);
            }
            return None;
        }
    };
    let rect = bar_rect(bar);
    // The box is a fraction of the bar's rect, a layout box and not the sprite: the accepted
    // endpoints were measured on it. The bar's own size fields do not work as a width: +0xa8 is an
    // animated word that reads 0 here, and the authored 88 at +0xb4 cuts the blade near the tip.
    if rect[2] <= rect[0] {
        return None;
    }
    // The box runs between the accepted endpoints, which core scales by this rect's width, so a
    // bar laid out wider or narrower cuts the same fraction of its blade.
    let box_words = cut_box(rect, ratio);
    // The rectangle is the game's own arithmetic, and main+0xa4b390 shows its inputs, so these
    // are the writes that matter: the anchor modes, the transform's x row, the authored pair and
    // the corner.

    // 1. The anchor modes, bits 0-3 of +0xa0: `& 3` is +0xa8's and `& 0xc` is +0xac's, mode 0 spans
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
    //    node has no animation entry, so writing it does nothing: the rectangle's origin is this
    //    node's translation at +0x40/+0x44.
    let height = (rect[3] - rect[1]).max(1);
    let _ = memory::write_word(mask + MASK_SIZE, box_words[2] as u32);
    let _ = memory::write_word(mask + MASK_SIZE + 4, height as u32);
    let _ = memory::write_word(mask + MASK_FLAGS, REBUILD_AND_APPLY);
    // 4. The corner. The walk fills the translation with the gauge's unscaled corner, which is the
    //    rect's corner in a plain game. For a rescaled bar `bar_rect` is where it was last drawn, and
    //    that box also sizes this one, so the corner comes from it too. The children's setMatrix
    //    reads this matrix (main+0xa496b0), so the walk's value goes back once the original has
    //    built the box.
    let corner = [
        memory::word(mask + MASK_TRANSLATION)?,
        memory::word(mask + MASK_TRANSLATION + 4)?,
    ];
    let _ = memory::write_word(mask + MASK_TRANSLATION, (rect[0] as f32).to_bits());
    let _ = memory::write_word(mask + MASK_TRANSLATION + 4, (rect[1] as f32).to_bits());
    // The shade follows the cut from here: its clip mask's translation goes to the cut's right edge,
    // its authored width spans forward from there to the blade's end, and the copy's atlas window
    // goes through its setter as the bar's own live four floats. The copy's quad, rect and flags
    // are not written.
    if dim != 0 && clip != 0 {
        write_shade(dim, clip, bar, rect, ratio);
    }
    Some(corner)
}

/// Put back the translation `enter` replaced, after the original mask enter has built its box.
pub fn restore_corner(mask: u32, corner: [u32; 2]) {
    let _ = memory::write_word(mask + MASK_TRANSLATION, corner[0]);
    let _ = memory::write_word(mask + MASK_TRANSLATION + 4, corner[1]);
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
    // The shade's clip mask, built with the copy: its translation and authored pair are the box
    // around the part of the blade past the cut, and the copy is drawn inside that box. Without the
    // copy there is nothing to clip, so it is not built and the accepted shape stands alone.
    let clip = match if dim == 0 {
        Ok(0u32)
    } else {
        build_clip_node(owner, resource)
    } {
        Ok(node) => node,
        Err(stage) => {
            if let Some(destructor) = owned {
                let _ = memory::call1(destructor, mask);
            }
            return Err(stage);
        }
    };
    let shade = (dim != 0 && clip != 0) as u32;
    let map = match grow_map(instance, root, nodes + 1, mask, [dim, clip, 0]) {
        Ok(map) => map,
        Err(stage) => {
            if let Some(destructor) = owned {
                let _ = memory::call1(destructor, mask);
            }
            return Err(stage);
        }
    };
    // The records the walker emits over the tree this builds: 9 with the mask alone, 11 once the
    // dim blade joins the root's chain as the mask's sibling, and 13 with the shade's clip mask
    // (mask enter, bar/glow/edge enter+exit, mask exit, clip enter, copy enter+exit, clip exit,
    // terminator). The grown area is (6 << 4) | 8 = 104 bytes = 13 records, so they fit exactly;
    // without the shade the row grows to 4 objects, area 72 = 9 records.
    let cache = match grow_cache(root, objects + 1 + shade * 2, sequences) {
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
        // The shade hangs off the root's chain as the mask's sibling, so it is walked after the
        // mask's subtree and outside the accepted scissor: the clip mask, then the copy inside that
        // clip. The accepted mask carries the two flag bits because the accepted badge's live flags
        // carry them: bit 17 is the transform gate, and bit 19 asks main+0xa4915c to re-sort the
        // mask's children by Priority once, on the next walk.
        && (shade == 0
            || (memory::word(mask + TEX_FLAGS)
                .is_some_and(|flags| memory::write_word(mask + TEX_FLAGS, flags | DRAWABLE_AND_DIRTY))
                && memory::call2(link_sibling, mask, clip).is_some()
                && memory::call2(link_child, clip, dim).is_some()));
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
    CLIP.store(clip, Ordering::Release);
    Ok(Outcome {
        mask,
        map,
        cache: cache.0,
        entries: nodes + 2 + shade * 2,
        records: cache.1,
        dim,
        dim_stage,
        clip,
    })
}

struct Read;
impl sharpness_gauge_core::Memory for Read {
    fn word(&self, address: u32) -> Option<u32> {
        memory::word(address)
    }
}
