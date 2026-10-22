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
use sharpness_gauge_core::{
    FIELDS, MAX_SHARPNESS, Memory, SHARPNESS, SHARPNESS_TYPE, sample_controller,
};

pub const MAX_SAMPLES: u32 = 240;
/// Walker calls between discovery attempts, before the gauge controller is
/// known. The walker runs for every animation controller each frame.
pub const SAMPLE_EVERY: u32 = 60;
/// Periodic tick that guarantees log progress even when nothing changed.
pub const PERIODIC_EVERY: u32 = 1_200;
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
static LAST: [AtomicU32; 3] = const { [const { AtomicU32::new(0) }; 3] };
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
        let changed = player != 0 && match (
            memory::word(player.wrapping_add(SHARPNESS)),
            memory::word(player.wrapping_add(MAX_SHARPNESS)),
            memory::word(player.wrapping_add(SHARPNESS_TYPE)),
        ) {
            // One changed word is enough to deserve a line; a failed read is
            // not a change, and the periodic tick still covers it.
            (Some(current), Some(maximum), Some(tier)) =>
                LAST[0].swap(current, Ordering::Relaxed) != current
                    || LAST[1].swap(maximum, Ordering::Relaxed) != maximum
                    || LAST[2].swap(tier, Ordering::Relaxed) != tier,
            _ => false,
        };
        if !changed && calls % PERIODIC_EVERY != 0 {
            return;
        }
    } else if calls % SAMPLE_EVERY != 0 {
        // Another controller, or the gauge is not found yet: look only on the
        // discovery cadence so the common case stays cheap.
        return;
    }
    let base = memory::BASE.load(Ordering::Relaxed);
    let Some(sample) = sample_controller(&Machine, controller, context, base) else {
        return;
    };
    let words = sample.words();
    let slot = taken as usize;
    for (index, word) in words.iter().enumerate() {
        RING_WORDS[slot * WORDS + index].store(*word, Ordering::Relaxed);
    }
    CTRL.store(controller, Ordering::Relaxed);
    PLAYER.store(sample.local.player, Ordering::Relaxed);
    // Publish the slot words first, then the write cursor, so a reader that
    // sees the cursor bump always sees a complete sample.
    TAKEN.store(taken + 1, Ordering::Release);
}

/// Oldest unread sample, with its 1-based number.
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

pub fn stats() -> (u32, u32, bool) {
    (
        CALLS.load(Ordering::Relaxed),
        TAKEN.load(Ordering::Relaxed),
        CTRL.load(Ordering::Relaxed) != 0,
    )
}
