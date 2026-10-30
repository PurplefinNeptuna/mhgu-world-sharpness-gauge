//! Finding the gauge row inside the hot draw-walker hook.
//!
//! The fill reads the player's sharpness in the mask's enter handler, so the walker's job here is
//! discovery only: find the gauge controller and the local player, once per HUD. Attempts run on a
//! slow cadence until the row is known, and after that one walker call costs two loads and one
//! integer compare. All formatting and logging happens later in `forge_onUpdate`.
use crate::memory;
use core::sync::atomic::{AtomicU32, Ordering};
use sharpness_gauge_core::{FIELD_CURRENT, FIELD_MAXIMUM, FIELD_TIER, Memory, sample_controller};

/// Walker calls between discovery attempts. The walker runs for every animation controller each
/// frame, so this is a fraction of a second.
pub const SAMPLE_EVERY: u32 = 60;

struct Machine;
impl Memory for Machine {
    fn word(&self, address: u32) -> Option<u32> {
        memory::word(address)
    }
}

static CALLS: AtomicU32 = AtomicU32::new(0);
static TAKEN: AtomicU32 = AtomicU32::new(0);
/// The gauge controller and local player found so far. Zero CTRL means "not found yet".
static CTRL: AtomicU32 = AtomicU32::new(0);
static PLAYER: AtomicU32 = AtomicU32::new(0);
/// The sharpness triple of the accepted discovery, packed 10 bits current, 10 bits maximum, 6 bits
/// tier. A gauge of 0/0 is not a sharpness weapon.
static LAST_GAUGE: AtomicU32 = AtomicU32::new(0);

/// Called from the hook with the walker's own two arguments: the animation controller about to
/// build its draw cache, and the draw context whose live clip every drawable in that cache is
/// intersected with.
pub fn observe(controller: u32, context: u32) {
    let calls = CALLS.fetch_add(1, Ordering::Relaxed);
    let gauge = CTRL.load(Ordering::Relaxed);
    if gauge != 0 && controller == gauge {
        return; // The row is known; the fill reads the player where the clip is written.
    }
    if calls % SAMPLE_EVERY != 0 {
        return;
    }
    let base = memory::BASE.load(Ordering::Relaxed);
    let Some(sample) = sample_controller(&Machine, controller, context, base) else {
        return;
    };
    let words = sample.words();
    LAST_GAUGE.store(
        (words[FIELD_CURRENT as usize] & 0x3ff)
            | ((words[FIELD_MAXIMUM as usize] & 0x3ff) << 10)
            | ((words[FIELD_TIER as usize] & 0x3f) << 20),
        Ordering::Relaxed,
    );
    CTRL.store(controller, Ordering::Relaxed);
    PLAYER.store(sample.local.player, Ordering::Relaxed);
    TAKEN.fetch_add(1, Ordering::Release);
}

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
    // The player pointer goes with it: reading a captured player after its HUD is gone is the
    // stale capture the plan's Phase 2 blocker names, and the fill reads this pointer per frame.
    PLAYER.store(0, Ordering::Relaxed);
}

/// The gauge to fill from, read fresh from the captured local player: the live sharpness points
/// through the same checked mapping the sampler logs. Nothing captured yet, or an unsupported
/// player state, is None, and the caller leaves the native gauge alone.
pub fn live_gauge() -> Option<sharpness_gauge_core::Gauge> {
    let player = PLAYER.load(Ordering::Relaxed);
    if player == 0 {
        return None;
    }
    sharpness_gauge_core::read_gauge(&Machine, player)
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
