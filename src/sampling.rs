//! Bounded observation copied out inside the hot draw-walker hook.
//!
//! The hook budget is fixed per process: at most `MAX_SAMPLES` gauge samples,
//! after which the hook reads no game memory at all. Sampling is state-driven:
//! once the gauge controller is known, a walker call is followed up only when
//! the sharpness words differ from the previous sample, or on a coarse periodic
//! tick, so one run can cover a whole session's transitions instead of a short
//! window. All formatting and logging happens later in `forge_onUpdate`.
use crate::memory;
use core::sync::atomic::{AtomicU32, Ordering};
use sharpness_gauge_core::{FIELD_CURRENT, FIELD_MAXIMUM, FIELD_TIER,
    FIELDS, MAX_SHARPNESS, Memory, SHARPNESS, SHARPNESS_TYPE, sample_controller};

pub const MAX_SAMPLES: u32 = 320;
/// Walker calls between discovery attempts, before the gauge controller is
/// known. The walker runs for every animation controller each frame.
pub const SAMPLE_EVERY: u32 = 60;
/// Periodic tick that guarantees log progress even when nothing changed. The
/// walker is called hundreds of times a second, so this is a few seconds.
pub const PERIODIC_EVERY: u32 = 2_400;
/// Never emit two samples closer than this, so a busy fight cannot flood the
/// log faster than the budget.
pub const MIN_CALLS_APART: u32 = 120;
/// A hit only removes a few sharpness points, so the raw value is not a useful
/// trigger: it is shifted by this much, which is 8 points per step. A full
/// 300-point gauge therefore costs about 38 lines instead of 300.
pub const BUCKET_SHIFT: u32 = 3;
pub const WORDS: usize = FIELDS.len();
/// Slots cover the whole budget, so no slot is overwritten before it is read.
pub const RING: usize = MAX_SAMPLES as usize;

struct Machine;
impl Memory for Machine {
    fn word(&self, address: u32) -> Option<u32> {
        memory::word(address)
    }
}

static CALLS: AtomicU32 = AtomicU32::new(0);
static TAKEN: AtomicU32 = AtomicU32::new(0);
static READ: AtomicU32 = AtomicU32::new(0);
/// The gauge controller and local player found so far, plus the last sharpness
/// triple seen (current, maximum, tier). Zero CTRL means "not found yet".
static CTRL: AtomicU32 = AtomicU32::new(0);
static PLAYER: AtomicU32 = AtomicU32::new(0);
/// Walker call count of the last emitted sample, so the rate floor can work.
static LAST_CALLS: AtomicU32 = AtomicU32::new(0);
/// The last accepted sample's sharpness points, packed 10 bits current, 10 bits
/// maximum, 6 bits tier. A gauge of 0/0 is not a sharpness weapon.
static LAST_GAUGE: AtomicU32 = AtomicU32::new(0);
/// Last bucketed sharpness, maximum, tier and player pointer seen. Zero means
/// "nothing seen yet", which makes the first sample a change.
static LAST: [AtomicU32; 4] = const { [const { AtomicU32::new(0) }; 4] };
static RING_WORDS: [AtomicU32; RING * WORDS] =
    const { [const { AtomicU32::new(0) }; RING * WORDS] };

/// Called from the hook with the walker's own two arguments: the animation
/// controller about to build its draw cache, and the draw context whose live
/// clip every drawable in that cache is intersected with.
pub fn observe(controller: u32, context: u32) {
    let taken = TAKEN.load(Ordering::Relaxed);
    if taken >= MAX_SAMPLES {
        return; // Budget spent: two loads and nothing else.
    }
    let calls = CALLS.fetch_add(1, Ordering::Relaxed);
    let gauge = CTRL.load(Ordering::Relaxed);
    if gauge != 0 && controller == gauge {
        let player = PLAYER.load(Ordering::Relaxed);
        // A tier, maximum or player change is the event worth measuring to the
        // point, and it happens rarely, so it is measured at full resolution
        // with no rate floor. The sharpness word itself is bucketed, because a
        // hit costs a few points and one line per hit would spend the budget
        // inside a single fight.
        let mut stepped = false;
        let mut exact = false;
        if player != 0 {
            if let (Some(current), Some(maximum), Some(tier)) = (
                memory::word(player.wrapping_add(SHARPNESS)),
                memory::word(player.wrapping_add(MAX_SHARPNESS)),
                memory::word(player.wrapping_add(SHARPNESS_TYPE)),
            ) {
                let bucket = current >> BUCKET_SHIFT;
                stepped = LAST[0].swap(bucket, Ordering::Relaxed) != bucket;
                exact = LAST[1].swap(maximum, Ordering::Relaxed) != maximum
                    || LAST[2].swap(tier, Ordering::Relaxed) != tier
                    || LAST[3].swap(player, Ordering::Relaxed) != player;
            }
        }
        if !stepped && !exact && calls % PERIODIC_EVERY != 0 {
            return;
        }
        if stepped
            && !exact
            && calls.wrapping_sub(LAST_CALLS.load(Ordering::Relaxed)) < MIN_CALLS_APART
        {
            // Only a bucket step, and it is too close to the previous line. The
            // periodic tick guarantees progress, and the next boundary comes
            // soon.
            return;
        }
    } else if calls % SAMPLE_EVERY != 0 {
        // Another controller, or the gauge is not found yet: look only on the
        // discovery cadence so the common case stays cheap.
        return;
    }
    let base = memory::BASE.load(Ordering::Relaxed);
    LAST_CALLS.store(calls, Ordering::Relaxed);
    let Some(sample) = sample_controller(&Machine, controller, context, base) else {
        return;
    };
    let words = sample.words();
    let slot = taken as usize;
    for (index, word) in words.iter().enumerate() {
        RING_WORDS[slot * WORDS + index].store(*word, Ordering::Relaxed);
    }
    LAST_GAUGE.store(
        (words[FIELD_CURRENT as usize] & 0x3ff)
            | ((words[FIELD_MAXIMUM as usize] & 0x3ff) << 10)
            | ((words[FIELD_TIER as usize] & 0x3f) << 20),
        Ordering::Relaxed,
    );
    CTRL.store(controller, Ordering::Relaxed);
    PLAYER.store(sample.local.player, Ordering::Relaxed);
    // Publish the slot words first, then the write cursor, so a reader that
    // sees the cursor bump always sees a complete sample.
    TAKEN.store(taken + 1, Ordering::Release);
}

/// Oldest unread sample, with its 1-based number.
/// The controller of the gauge row, as recorded by the last accepted sample.
pub fn gauge_controller() -> u32 {
    CTRL.load(Ordering::Relaxed)
}

/// Drop the recorded gauge controller. Called from the row's root destructor
/// (main+0xa486fc), where the object being destroyed is that controller, so the
/// sampler stops comparing walker arguments against a freed address. Discovery
/// starts again when a new HUD is built.
pub fn clear_controller() {
    CTRL.store(0, Ordering::Relaxed);
}

pub fn take() -> Option<(u32, [u32; WORDS])> {
    let taken = TAKEN.load(Ordering::Acquire);
    let read = READ.load(Ordering::Relaxed);
    if read >= taken {
        return None;
    }
    let mut words = [0u32; WORDS];
    for index in 0..WORDS {
        words[index] = RING_WORDS[read as usize * WORDS + index].load(Ordering::Relaxed);
    }
    READ.store(read + 1, Ordering::Relaxed);
    Some((read + 1, words))
}

/// Current and maximum sharpness points of the last accepted sample, when that
/// sample carried a sharpness gauge at all. Weapons without sharpness (bowguns,
/// the prowler rig) produce no gauge row, so there is nothing to accept.
/// The last sampled sharpness triple. The tier is part of it: each tier is its own segment of
/// points, so the fill ratio inside the bar is a function of the tier as much as the points.
pub fn gauge_points() -> Option<(u32, u32, u32)> {
    let packed = LAST_GAUGE.load(Ordering::Relaxed);
    let current = packed & 0x3ff;
    let maximum = (packed >> 10) & 0x3ff;
    let tier = (packed >> 20) & 0x3f;
    (maximum != 0 && current <= maximum).then_some((current, maximum, tier))
}


pub fn stats() -> (u32, u32, bool) {
    (
        CALLS.load(Ordering::Relaxed),
        TAKEN.load(Ordering::Relaxed),
        CTRL.load(Ordering::Relaxed) != 0,
    )
}
