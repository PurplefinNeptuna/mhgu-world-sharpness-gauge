//! Find the gauge row, validate it, and publish the row the insertion needs.
//!
//! The gauge row is the animation controller the walker reports, its root at `root+0x68`, and
//! the bar, glow and edge as its child/sibling chain, identified by authored size. What the
//! gates rest on, from the decompiles recorded in
//! work/world-sharpness-gauge/PHASE2_RESEARCH.md:
//! - main+0xa494a0 writes 8-byte records (kind, node) into `controller+0x80`; main+0xa49b28
//!   sizes that area from the halfwords at **row+8** as `objects * 16 + 8 + sequences * 12` and
//!   publishes `cache = allocation + sequences * 12`. This row is sized for 3 objects and 16
//!   sequences: a 7-record area, so the inserted tree's 13 records do not fit and the insertion
//!   has to grow the block.
//! - main+0xa47d2c counts the nodes under `+0x68`/`+0x6c` with no visibility test;
//!   main+0xa38ffc returns that + 1 and main+0x5456e4 uses it as the length of the instance's
//!   array at `+0xd8`, dispatching slot 0x50 per entry. The gauge row's four entries are the
//!   controller, bar, glow and edge, so each inserted node needs its own entry: more indexed
//!   reads than the array has words run past it and end in that loop.
//! - the rectangle the game builds for a mask at `mask+0xb0..+0xbc` is derived from the node's
//!   own transform and authored pair, and a factory-built mask's anchors and x scale collapse it
//!   to zero width, which clips the gauge away. Those inputs are written at the mask's enter
//!   handler, not the rectangle: see `src/insert.rs`.
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

/// 0 = no insertion made for the current gauge row, 1 = the row was read and the insertion
/// attempted. Reset by `on_teardown`, so the next quest's HUD is inserted again.
static STATE: AtomicU8 = AtomicU8::new(0);
static FAILURE: AtomicU32 = AtomicU32::new(0);

pub fn take_failure() -> u32 {
    FAILURE.swap(0, Ordering::Relaxed)
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
    // The gauge row is identified by the fields the sampler already checks:
    // animation 4, 4 objects, 15 sequences, first object 29. The row
    // carries no drawable count. root+0x8c is recorded, not required: its bit 0 is the
    // walker's rebuild flag, which no code in this build sets.
    if anim != GAUGE_ANIMATION
        || objects != GAUGE_OBJECTS
        || sequences != GAUGE_SEQUENCES
        || first_object != GAUGE_FIRST_OBJECT
    {
        return Err(24);
    }
    // Gauge layout, from the walker decompile at main+0xa494a0.
    // The walk root is a cGUIObjRoot: the bar's parent, the draw cache lives at
    // root+0x80, and the walk ends when the next pointer comes back to the root. The
    // three sharpness textures are direct siblings of the root.
    // A sharpness gauge must be live. The hub has no gauge, a quest that ended
    // tears its HUD down, and weapons without sharpness (bowguns, the prowler
    // rig) never build the gauge row at all.
    if !sampling::gauge_present() {
        return Err(34);
    }
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
    // this chain. The gauge row's edge glyph closes it with sibling 0.
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




/// One row read: the insertion's outcome. Its stage is 0 when the mask was inserted, and a
/// 40..49 code when it was not, so one report says which of the two paths stopped and why. A
/// row that is not present yet is not a failure: the gauge HUD row exists only while a
/// weapon-owning player is in a quest, so stage 21 reports and keeps retrying at a slow cadence.
pub struct Reading {
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
    let outcome = insert::run(&row, instance);
    STATE.store(1, Ordering::Release);
    Some(Reading { outcome })
}

/// The gauge row is going away. The game owns the tree, the mask, the map and the cache
/// from here on: this clears this build's own state, so one insertion is made per gauge HUD and
/// the sampler stops comparing walker arguments against a freed address.
pub fn on_teardown() {
    insert::on_teardown();
    STATE.store(0, Ordering::Release);
    FAILURE.store(0, Ordering::Release);
    TRIES.store(0, Ordering::Relaxed);
}
