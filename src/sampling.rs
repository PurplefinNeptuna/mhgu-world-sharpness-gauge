//! Finding the gauge row inside the hot draw-walker hook.
//!
//! The fill reads the player's sharpness in the mask's enter handler, so the walker's job here is
//! discovery only: find the gauge controller and the local player, once per HUD. Attempts run on a
//! slow cadence until the row is known, and after that one walker call costs two loads and one
//! integer compare. All formatting and logging happens later in `forge_onUpdate`.
use crate::memory;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use sharpness_gauge_core::{Gauge, Memory, sample_controller};

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
/// The gauge controller and local player found so far. Zero CTRL means "not found yet".
static CTRL: AtomicU32 = AtomicU32::new(0);
static PLAYER: AtomicU32 = AtomicU32::new(0);
/// The last accepted sample carried a sharpness gauge: a positive maximum, and points that do not
/// pass it. A gauge of 0/0 is not a sharpness weapon, and a weapon with no sharpness (a bowgun,
/// the prowler rig) never builds the gauge row at all.
static GAUGE_PRESENT: AtomicBool = AtomicBool::new(false);

/// Called from the hook with the walker's animation-controller argument: the controller about to
/// build its draw cache.
pub fn observe(controller: u32) {
    let calls = CALLS.fetch_add(1, Ordering::Relaxed);
    let gauge = CTRL.load(Ordering::Relaxed);
    if gauge != 0 && controller == gauge {
        return; // The row is known; the fill reads the player where the clip is written.
    }
    if calls % SAMPLE_EVERY != 0 {
        return;
    }
    let base = memory::BASE.load(Ordering::Relaxed);
    let Some(sample) = sample_controller(&Machine, controller, base) else {
        return;
    };
    GAUGE_PRESENT.store(
        sample
            .gauge
            .is_some_and(|gauge| gauge.maximum > 0 && gauge.current <= gauge.maximum),
        Ordering::Relaxed,
    );
    CTRL.store(controller, Ordering::Relaxed);
    PLAYER.store(sample.local.player, Ordering::Relaxed);
}

/// The controller of the gauge row, as recorded by the last accepted sample.
pub fn gauge_controller() -> u32 {
    CTRL.load(Ordering::Relaxed)
}

/// Drop the recorded gauge controller and its player. Called on the gauge row's teardown, where
/// the row the controller belongs to is going away, so the sampler stops comparing walker
/// arguments against a freed address. Discovery starts again when a new HUD is built.
pub fn clear_controller() {
    CTRL.store(0, Ordering::Relaxed);
    GAUGE_PRESENT.store(false, Ordering::Relaxed);
    // The player pointer goes with it: reading a captured player after its HUD is gone is the
    // stale capture the plan's Phase 2 blocker names, and the fill reads this pointer per frame.
    PLAYER.store(0, Ordering::Relaxed);
}

/// The gauge to fill from, read fresh from the captured local player: the live sharpness points
/// through the same checked mapping discovery uses. Nothing captured yet, or an unsupported
/// player state, is None, and the caller leaves the native gauge alone.
pub fn live_gauge() -> Option<Gauge> {
    let player = PLAYER.load(Ordering::Relaxed);
    if player == 0 {
        return None;
    }
    sharpness_gauge_core::read_gauge(&Machine, player)
}

/// Whether the gauge row of a weapon with sharpness has been found, which is what the row check
/// in `src/gui.rs` needs before it reads that row.
pub fn gauge_present() -> bool {
    GAUGE_PRESENT.load(Ordering::Relaxed)
}
