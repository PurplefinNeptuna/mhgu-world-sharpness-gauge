//! Find the gauge row and copy out the row/map facts the insertion needs.
//!
//! r14 inserted a `cGUIObjScissorMask` as the row root's first child and clipped the
//! bar with it. The Eden run of 2026-10-23 (its own NRO hash, `work/modern-sharpness-gauge/
//! runs/r14-eden-2026-10-23.txt`) shows what that cost. r17 inserted one again, grown
//! map and cache and all, and its own gate stopped it before a write
//! (`runs/r17-eden-2026-10-24.txt`); r18 is that gate's fix. What r14's log showed:
//! - the rectangle the game built at `mask+0xb0..+0xbc` was `0 0 W 32` in every one of
//!   the 2512 lines that printed it, while the box asked for was `x=110 y=32` in 1352
//!   lines. The mask's position writes at `+0x80/+0x84` never reached the build, and the
//!   bar's own rect at `+0xe0` is `110,38,238,166`, so the clip and the bar never
//!   overlapped: the gauge was clipped away, which is the user's "gauge disappeared".
//! - from t=642.30 the built rectangle is `0 0 0 0` for the remaining 1119 lines while
//!   the plugin kept writing widths 18/33/49/80. At the same line the bar's rect became
//!   the authored `0,-6,88,18` and the player's `cmd_tbl_id` changed (1000 → 9999 →
//!   2003). What the log proves is that a game pass took the mask's authored size away
//!   when that layout change ran; that the pass is the row's authored-layout pass,
//!   reached because the mask has no entry in the gauge row's map, is inference.
//! - the crash at t=1015.539 (1011 dynarmic exceptions, then the emulator's own SIGSEGV)
//!   is the same map fact: main+0x5456e4 loops over the instance's object array at `+0xd8`
//!   with the length `main+0xa38ffc` = main+0xa47d2c's node count + 1. That array's 4
//!   entries are the controller plus the 3 tree nodes (the r16 and r17 runs read them out:
//!   `obj0` is the controller, `obj1` the bar, `obj2` the glow, `obj3` the edge); the
//!   inserted mask made 4 tree nodes, so the loop walks 5 entries through a 4-entry array
//!   and dispatches vtable slot 0x50 on what it finds there. The logged first argument is `0x727272`, a freed block's fill,
//!   the frame at main+0x5457c8 is that loop, and the frame below it is in the mask
//!   class's destructor region (main+0xa56758, main+0xa56828, and main+0xa568a4).
//!
//! The gates below are r14's gates, widened once (the inserted mask becomes the textures'
//! parent); the facts below are printed once per gauge row, and they are what told r18 the
//! row's real sizing pair. Nothing here writes the row: the insertion lives in `insert.rs`.
//!
//! What the code below relies on, all read from the hash-checked main NSO and recorded in
//! `work/modern-sharpness-gauge/PHASE2_RESEARCH.md`:
//! - the draw walker main+0xa494a0 writes 8-byte records (kind word, node pointer) into
//!   `controller+0x80`, and main+0xa49b28 sizes that area from the halfwords at **row+8** as
//!   `objects * 16 + 8 + sequences * 12`, publishing `cache = allocation + sequences * 12`.
//!   That pair, not the one at `row+4` this file prints as `counts`, is the sizing pair: the
//!   r17 run read `row+8` as 3 objects (the tree's node count) and 16 sequences, with the
//!   published cache 192 = 16 * 12 into the allocation, so the record area is 56 = 7 records;
//! - main+0xa47d2c counts the nodes in the child and sibling chains under `+0x68` with no
//!   visibility test, main+0xa38ffc returns that + 1, and main+0x5456e4 uses the sum as the
//!   length of the instance's array at `+0xd8`;
//! - the root destructor main+0xa486fc frees `root+0xa4`, then walks `root+0x68` and each
//!   `+0x6c`, calling slot 0x04 of each, then frees the root.
use crate::{
    hooks,
    insert,
    memory,
    sampling,
};
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use forge::mt::dti::MtDti;

// Row description fields, as Key Quest Markers reads them at root+0x9c.
const GAUGE_ANIMATION: u32 = 4;
const GAUGE_OBJECTS: u16 = 4;
const GAUGE_SEQUENCES: u16 = 15;
const GAUGE_FIRST_OBJECT: u32 = 29;
const OBJECT_BYTES: usize = 304;
const INSTANCE_BYTES: usize = 224;
const ROOT_BYTES: usize = 176;

/// 0 = nothing reported for the current gauge row, 1 = the facts line was printed.
/// Reset by `on_teardown`, so the next quest's HUD is reported again.
static STATE: AtomicU8 = AtomicU8::new(0);
static FAILURE: AtomicU32 = AtomicU32::new(0);

pub fn take_failure() -> u32 {
    FAILURE.swap(0, Ordering::Relaxed)
}

/// The row, map and object facts, in `FACT_NAMES` order. Pointers are words as read;
/// the authored sizes are printed as the integers the game's floats hold. `tree_nodes`
/// is main+0xa47d2c's count for this controller: main+0xa38ffc adds one to it and
/// main+0x5456e4 uses that sum as the length of the instance's object array at +0xd8.
pub const FACTS_WORDS: usize = 29;
pub const FACT_NAMES: [&str; FACTS_WORDS] = [
    "ctrl", "tree_root", "cache", "ctrl_flags", "row", "instance", "cache_alloc", "anim", "counts",
    "counts8", "first_object", "row14", "obj_list", "inst_ctrl", "bar_id", "bar_w", "bar_h", "glow_id",
    "glow_w", "glow_h", "edge_id", "edge_w", "edge_h", "bar_flags", "obj0", "obj1", "obj2",
    "obj3", "tree_nodes",
];
pub const FACT_HEX: [&str; 12] = [
    "ctrl", "tree_root", "cache", "row", "instance", "cache_alloc", "row14", "obj_list", "obj0",
    "obj1", "obj2", "obj3",
];

fn word(address: u32) -> u32 {
    memory::word(address).unwrap_or(0)
}

fn is_type(object: u32, dti: u32) -> bool {
    let Some(f) = memory::method(object, 0x14) else {
        return false;
    };
    memory::call1(f, object).is_some_and(|got| got == dti)
}

/// The gauge row as the game currently holds it. Every field is checked against the
/// archive's structure, and the bar is identified by its authored size. `holder` is what the
/// three textures hang off: the controller, or the inserted mask once this build has inserted
/// one, whose own links the gates then check. `description` is the animation row the
/// controller was built from, which is where the sizing pair lives.
pub(crate) struct Row {
    pub(crate) root: u32,
    pub(crate) bar: u32,
    pub(crate) effect: u32,
    pub(crate) edge: u32,
    pub(crate) description: u32,
}

/// Read and validate the gauge row. Each gate has its own stage code so a single
/// run says which check failed: 21 no captured instance pair, 22 instance or
/// controller not writable, 23 a field read failed, 24 row counts do not match the
/// gauge animation, 25 the controller is not a cGUIObjRoot, 26 the draw cache at
/// `+0x80` is missing or unreadable, or the bar link under the controller is missing,
/// equals the controller, or is not writable, 27 the bar's parent link is not the
/// controller, 28 the glow is not a distinct writable sibling of the bar whose parent
/// is the controller, 29 the edge is not a distinct writable sibling of the glow whose
/// parent is the controller, 30 the chain does not close at the controller, 31 the bar
/// is not a cGUIObjTexture, 32 the bar's authored width at `+0xb4` is outside 1..=1024,
/// 33 a DTI is missing, 34 no live sharpness gauge.
fn read_row() -> Result<Row, u32> {
    let (instance, root) = hooks::gauge_instance().ok_or(21u32)?;
    if !memory::writable(instance, INSTANCE_BYTES) || !memory::writable(root, ROOT_BYTES) {
        return Err(22);
    }
    let Some(description) = memory::word(root.wrapping_add(0x9c)) else {
        return Err(23);
    };
    let (Some(cache), Some(_)) = (
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
    // walker's rebuild flag, which no code in this build sets.
    if anim != GAUGE_ANIMATION
        || objects != GAUGE_OBJECTS
        || sequences != GAUGE_SEQUENCES
        || first_object != GAUGE_FIRST_OBJECT
    {
        return Err(24);
    }
    // Gauge layout, from the walker decompile at main+0xa494a0 and the r9 Eden log.
    // The walk root is a cGUIObjRoot: the bar's parent, the draw cache lives at
    // root+0x80, and the walk ends when the next pointer comes back to the root. The
    // three sharpness textures are direct siblings of the root.
    // A sharpness gauge must be live. The hub has no gauge, a quest that ended
    // tears its HUD down, and weapons without sharpness (bowguns, the prowler
    // rig) never build the gauge row at all.
    sampling::gauge_points().ok_or(34u32)?;
    let root_dti = MtDti::find("cGUIObjRoot").ok_or(33u32)?;
    if !is_type(root, root_dti as *const _ as u32) {
        return Err(25);
    }
    let first = memory::word(root.wrapping_add(sharpness_gauge_core::NODE_CHILD)).ok_or(23u32)?;
    // Our own marker at the head of the chain means the mask is in this row, so the three
    // textures are its children and its siblings, not the controller's. Every parent check
    // below is against that holder: gates written for the pre-insertion tree would stop a
    // row that has been inserted, which is the failure mode the playbook names.
    let inserted = first != 0
        && memory::word(first.wrapping_add(0x50)) == Some(insert::MARKER);
    let (holder, bar) = if inserted {
        (
            first,
            memory::word(first.wrapping_add(sharpness_gauge_core::NODE_CHILD)).ok_or(26u32)?,
        )
    } else {
        (root, first)
    };
    let Some(bar_parent) = memory::word(bar.wrapping_add(sharpness_gauge_core::NODE_PARENT))
    else {
        return Err(23);
    };
    if bar == 0 || bar == holder || !memory::writable(bar, OBJECT_BYTES) {
        return Err(26);
    }
    if bar_parent != holder {
        return Err(27);
    }
    let Some(effect) = memory::word(bar.wrapping_add(sharpness_gauge_core::NODE_SIBLING)) else {
        return Err(23);
    };
    let Some(effect_parent) = memory::word(effect.wrapping_add(sharpness_gauge_core::NODE_PARENT))
    else {
        return Err(23);
    };
    if effect == 0 || effect == bar || !memory::writable(effect, OBJECT_BYTES) {
        return Err(28);
    }
    if effect_parent != holder {
        return Err(28);
    }
    let Some(edge) = memory::word(effect.wrapping_add(sharpness_gauge_core::NODE_SIBLING)) else {
        return Err(23);
    };
    let Some(edge_parent) = memory::word(edge.wrapping_add(sharpness_gauge_core::NODE_PARENT))
    else {
        return Err(23);
    };
    if edge == 0 || edge == effect || edge == bar
        || !memory::writable(edge, OBJECT_BYTES)
        || edge_parent != holder
    {
        return Err(29);
    }
    // How the walk closes, from main+0xa494a0: a sibling equal to the root breaks
    // the loop, and a zero sibling pops to node+0x70, which the gates above prove
    // is the root. Both closings are valid, any other pointer is not the end of
    // this chain. The r10 log shows the edge glyph closing with sibling 0.
    let terminator = memory::word(edge.wrapping_add(sharpness_gauge_core::NODE_SIBLING)).ok_or(23u32)?;
    if terminator != 0 && terminator != holder {
        return Err(30);
    }
    let texture = MtDti::find("cGUIObjTexture").ok_or(33u32)?;
    if !is_type(bar, texture as *const _ as u32) {
        return Err(31);
    }
    let (Some(width), Some(_)) = (
        memory::word(bar.wrapping_add(0xb4)),
        memory::word(bar.wrapping_add(0xb8)),
    ) else {
        return Err(23);
    };
    // +0xb4/+0xb8 is the authored size pair the layout pass writes, 88x24 for this bar.
    // Stage 32 means that width is not a size a box can be built from.
    if !(1..=1024).contains(&(f32::from_bits(width) as i32)) {
        return Err(32);
    }
    Ok(Row {
        root,
        bar,
        effect,
        edge,
        description,
    })
}

struct Read;
impl sharpness_gauge_core::Memory for Read {
    fn word(&self, address: u32) -> Option<u32> {
        memory::word(address)
    }
}

/// Copy the row, map and object words out, read-only. The array at `instance+0xd8` is
/// read for its four entries: main+0x5456e4 indexes it with `main+0xa47d2c`'s node count
/// plus one, so this line prints both numbers. The r15 run of 2026-10-23 (163.5 s, gauge
/// controller 0x4800b250) printed tree_nodes=3 against this row's 4 objects, and showed
/// that `+0xd8` is only an array on the captured HUD instance: read from the controller's
/// owner field at `root+0xa0` it is the freed block's fill 0x727272.
fn facts(row: &Row, instance: u32) -> [u32; FACTS_WORDS] {
    let root = row.root;
    let row_word = word(root + 0x9c);
    let bar = row.bar;
    let list = word(instance + 0xd8);
    let objects = readable_words(list, 4);
    let nodes = sharpness_gauge_core::count_nodes(&Read, root).0;
    let authored = |object: u32| -> (u32, u32, u32) {
        (
            word(object + 0x50),
            (f32::from_bits(word(object + 0xb4)) as i32) as u32,
            (f32::from_bits(word(object + 0xb8)) as i32) as u32,
        )
    };
    let (bar_id, bar_w, bar_h) = authored(bar);
    let (glow_id, glow_w, glow_h) = authored(row.effect);
    let (edge_id, edge_w, edge_h) = authored(row.edge);
    [
        root,
        word(root + sharpness_gauge_core::NODE_CHILD),
        word(root + 0x80),
        word(root + 0x8c),
        row_word,
        instance,
        word(root + 0xa4),
        word(row_word),
        word(row_word + 4),
        word(row_word + 0x08),
        word(row_word + 0x0c),
        word(row_word + 0x14),
        list,
        word(instance + 0xdc),
        bar_id,
        bar_w,
        bar_h,
        glow_id,
        glow_w,
        glow_h,
        edge_id,
        edge_w,
        edge_h,
        word(bar + sharpness_gauge_core::NODE_FLAGS),
        objects[0],
        objects[1],
        objects[2],
        objects[3],
        nodes,
    ]
}

fn readable_words(list: u32, count: usize) -> [u32; 4] {
    let mut out = [0u32; 4];
    if list == 0 || !memory::readable(list, 16) {
        return out;
    }
    for (index, slot) in out.iter_mut().enumerate().take(count) {
        *slot = word(list + 4 * index as u32);
    }
    out
}

/// One row read: the facts, and the insertion's outcome. `stage` is 0 when the mask was
/// inserted, and a 40..49 code when it was not, so one run says which of the two paths
/// stopped and why. A row that is not present yet is not a failure: the gauge HUD row
/// exists only while a weapon-owning player is in a quest, so stage 21 reports and keeps
/// retrying at a slow cadence.
pub struct Reading {
    pub facts: [u32; FACTS_WORDS],
    pub outcome: Result<insert::Outcome, u32>,
}
const RETRY_EVERY: u32 = 240;
static TRIES: AtomicU32 = AtomicU32::new(0);

pub fn tick() -> Option<Reading> {
    if STATE.load(Ordering::Relaxed) != 0 {
        return None;
    }
    let tries = TRIES.fetch_add(1, Ordering::Relaxed);
    if tries != 0 && tries % RETRY_EVERY != 0 {
        return None;
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
            return None;
        }
    };
    let Some((instance, _)) = hooks::gauge_instance() else {
        return None;
    };
    let words = facts(&row, instance);
    let outcome = insert::run(&row, instance);
    STATE.store(1, Ordering::Release);
    Some(Reading {
        facts: words,
        outcome,
    })
}

/// The row's root is being destroyed. Nothing in this build touches the tree, so this
/// only clears the state: the facts line is printed once per gauge HUD, and the sampler
/// stops comparing walker arguments against a freed address.
pub fn on_teardown() {
    insert::on_teardown();
    STATE.store(0, Ordering::Release);
    FAILURE.store(0, Ordering::Release);
    TRIES.store(0, Ordering::Relaxed);
}
