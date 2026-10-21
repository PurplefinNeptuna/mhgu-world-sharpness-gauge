//! Bounded observation copied out inside the hot draw-walker hook.
//!
//! The hook budget is fixed per process: at most `MAX_SAMPLES` gauge samples,
//! each taken every `SAMPLE_EVERY` walker calls. After that budget the hook
//! performs no game reads at all, so the steady-state cost is two atomic loads.
//! All formatting and logging happens later in `forge_onUpdate` (src/lib.rs).
use crate::memory;
use core::sync::atomic::{AtomicU32, Ordering};
use sharpness_gauge_core::{FIELDS, Memory, sample_controller};

pub const MAX_SAMPLES: u32 = 60;
/// Walker calls between samples. The walker runs per animation controller per
/// frame, so this is roughly "once per second" for the gauge.
pub const SAMPLE_EVERY: u32 = 60;
pub const WORDS: usize = FIELDS.len();
/// Slots must cover the whole budget; no slot is overwritten before it is read.
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
static MATCHED: AtomicU32 = AtomicU32::new(0);
static RING_WORDS: [AtomicU32; RING * WORDS] = const { [const { AtomicU32::new(0) }; RING * WORDS] };

/// Called from the hook with the animation controller that is about to build
/// and consume its draw cache. Returns without touching game memory when the
/// sample budget is spent, so the per-frame cost after sampling is negligible.
pub fn observe(controller: u32) {
    let taken = TAKEN.load(Ordering::Relaxed);
    if taken >= MAX_SAMPLES {
        return;
    }
    let calls = CALLS.fetch_add(1, Ordering::Relaxed);
    if calls % SAMPLE_EVERY != 0 {
        return;
    }
    let base = memory::BASE.load(Ordering::Relaxed);
    let Some(sample) = sample_controller(&Machine, controller, base) else {
        return;
    };
    let words = sample.words();
    // Publish the slot words first, then the write cursor, so a reader that
    // sees the cursor bump always sees a complete sample.
    for (index, word) in words.iter().enumerate() {
        RING_WORDS[taken as usize * WORDS + index].store(*word, Ordering::Relaxed);
    }
    MATCHED.store(1, Ordering::Relaxed);
    TAKEN.store(taken + 1, Ordering::Release);
}

/// Oldest unread sample, with its 1-based number. Formatting happens here, in
/// a Forge callback, never inside the hook.
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
        MATCHED.load(Ordering::Relaxed) != 0,
    )
}
