//! Finding the gauge row inside the hot draw-walker hook, and the bar's fill for each frame.
//!
//! The fill reads the player in the mask's enter handler, where the clip is written; the walker's
//! job here is discovery only: find the gauge controller and the local player, once per HUD.
//! Attempts run on a slow cadence until the row is known, and after that one walker call costs two
//! loads and one integer compare. All formatting and logging happens later in `forge_onUpdate`.
use crate::memory;
use crate::signatures::CALCULATE_SHARPNESS_TYPE;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use sharpness_gauge_core::{Memory, band_bp, local_player, sample_controller};

/// Walker calls between discovery attempts. The walker runs for every animation controller each
/// frame, so this is a fraction of a second.
pub const SAMPLE_EVERY: u32 = 60;

/// `uPlayerBase::calculateSharpnessType` (main+0x29cdfc) dereferences `player + 0x13c4` at
/// main+0x29ce34, and the gate it calls first at main+0x27c7ec reads the byte at that pointer
/// plus `0x4d4` (main+0x27c848, main+0x27c84c). Neither is checked by the game, so `fill` reads
/// both through the mapping-checked reader before its first native call.
const TIER_PARAM_CELL: u32 = 0x13c4;
const TIER_STATE_BYTE: u32 = 0x4d4;

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

/// The bar's fill for this frame, in basis points inside the tier's band: the local player's live
/// points, banded by the game's own points-to-tier function.
///
/// `calculateSharpnessType` is native code called with the player, and it dereferences two
/// pointers of its own with no check, so every gate below runs before the first native call and
/// the player is re-derived from the session rather than trusted from a capture:
///
/// 1. `local_player` answers with the same player the sampler captured. A gauge row whose player
///    is gone, moved or not the session's local hunter stops the fill here.
/// 2. `read_gauge` succeeds with a positive `mMaxSharpness`. A gauge of 0/0 is not a sharpness
///    weapon, and the band search divides by a width derived from the maximum.
/// 3. Both pointers the native function dereferences, `player + 0x13c4` and that plus `0x4d4`,
///    read through the mapping check.
///
/// Any failure, and any probe whose native call is refused, yields `None`: the caller's existing
/// unsupported state, which turns the clip off, leaves the native gauge alone and gives the shade
/// width 0. The searches run over at most `0..=maximum` points, so a frame costs a couple of
/// dozen checked native calls; nothing is cached, so no invalidation is needed on a weapon
/// change. Upgrade: cache the band, keyed by points, maximum and tier, if a run shows a cost.
pub fn fill() -> Option<u32> {
    let base = memory::BASE.load(Ordering::Relaxed);
    let captured = PLAYER.load(Ordering::Relaxed);
    let live = local_player(&Machine, base)?;
    if captured == 0 || live.player != captured {
        return None;
    }
    let gauge = sharpness_gauge_core::read_gauge(&Machine, live.player)?;
    if gauge.maximum <= 0 {
        return None;
    }
    let params = memory::word(live.player.checked_add(TIER_PARAM_CELL)?)?;
    memory::word(params.checked_add(TIER_STATE_BYTE)?)?;
    band_bp(gauge.current, gauge.maximum, &|points| {
        memory::call2(base.checked_add(CALCULATE_SHARPNESS_TYPE)?, live.player, points as u32)
    })
}

/// Whether the gauge row of a weapon with sharpness has been found, which is what the row check
/// in `src/gui.rs` needs before it reads that row.
pub fn gauge_present() -> bool {
    GAUGE_PRESENT.load(Ordering::Relaxed)
}
