//! Modern sharpness gauge: the game's sharpness bar, clipped to the local player's own sharpness
//! ratio, read in the mask's enter hook. The box is built from the accepted cut endpoints, as
//! offsets on the bar's own rect, scaled by that rect's live width. There are no settings and
//! therefore no Plugin UI panel.
//!
//! Four hooks: the draw walker (main+0xa494a0) finds the gauge controller and the local player;
//! the instance draw entry (main+0xa39300) records the row instance; the row root's destructor
//! (main+0xa486fc) clears that capture; the scissor mask's enter handler (main+0xa4ba94) writes
//! the clip box in the call that applies it. The build writes game memory and inserts one GUI
//! object, so it changes what is drawn. `src/gui.rs` names the log lines and the native code
//! behind them. Forge supplies lifecycle, allocation and logging; stock Forge filters levels
//! below ERROR, hence log::error! for diagnostics.
#![no_std]
extern crate alloc;
use alloc::string::String;
use core::fmt::Write as _;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
mod gui;
mod hooks;
mod insert;
mod memory;
mod sampling;
mod signatures;

/// Walker calls without a gauge controller that count as "the HUD is absent".
const ABSENT_AFTER_CALLS: u32 = 6_000;
static REPORTED_FIRST: AtomicBool = AtomicBool::new(false);
static REPORTED_ABSENT: AtomicBool = AtomicBool::new(false);
static LAST_TEARDOWNS: AtomicU32 = AtomicU32::new(0);
static LAST_MISSES: AtomicU32 = AtomicU32::new(0);
/// The last reported row-check stage, so the retry cadence prints a change, not a stream.
static LAST_STAGE: AtomicU32 = AtomicU32::new(0);

// forge-rs exposes only the six-byte version prefix. The matching native loader
// also owns name[64] at offset 6 and hides empty names in its Loaded Plugins
// list, so this narrow ABI extension mirrors the accepted mod's record.
#[repr(C)]
struct PluginMetadata {
    version: forge::ForgeVersion,
    name: [u8; 64],
}
const _: () = assert!(
    core::mem::size_of::<PluginMetadata>() == 70
        && core::mem::offset_of!(PluginMetadata, name) == 6
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn forge_onLoad(params: *mut forge::PluginInitParams) {
    if params.is_null() {
        return;
    }
    let mut name = [0; 64];
    let label = concat!("Modern Sharpness Gauge ", env!("MSG_VERSION")).as_bytes();
    name[..label.len()].copy_from_slice(label);
    // SAFETY: the pinned Forge loader provides the complete writable 70-byte
    // record, as verified by the accepted mod's runtime evidence.
    unsafe {
        params.cast::<PluginMetadata>().write(PluginMetadata {
            version: forge::ForgeVersion {
                major: 1,
                minor: 5,
                patch: 1,
            },
            name,
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn forge_onInit() {
    forge::log::init().expect("Failed to initialize the Forge logger");
    match hooks::install() {
        Ok(owner) => {
            // Forge keeps the patches live for the process lifetime; rollback
            // is only reachable from the installation path above.
            core::mem::forget(owner);
            log::info!(concat!(
                "Modern Sharpness Gauge ",
                env!("MSG_VERSION"),
                " loaded (clips the sharpness gauge to the player's sharpness)"
            ));
        }
        Err(reason) => log::error!("Modern Sharpness Gauge disabled: {reason}"),
    }
}

/// Logging runs here, outside the hook. What is printed is the run's evidence: one line per gauge
/// row read (with the insertion's own outcome), one line per mask teardown, one line each time the
/// cut's ratio steps, and one line when the gauge HUD is absent. The row-check stage prints when
/// it changes, not on every retry.
#[unsafe(no_mangle)]
pub extern "C" fn forge_onUpdate(delta: f32) {
    let _ = delta;
    // The clip box follows the player's sharpness; one line each time that ratio steps.
    insert::tick_clock();
    if let Some(cut) = insert::take_cut_line() {
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " fill ratio_bp={ratio} box={bl} {bt} {bw} {bh} bar={l} {t} {r} {b} eff={el} {et} {er} {eb} tex={tu} {tv} {tw} {th} bar_w={bw2} feed={feed}"
        ),
        ratio = cut[0],
        bl = cut[1] as i32, bt = cut[2] as i32, bw = cut[3] as i32, bh = cut[4] as i32,
        l = cut[5] as i32, t = cut[6] as i32, r = cut[7] as i32, b = cut[8] as i32,
        el = cut[9] as i32, et = cut[10] as i32, er = cut[11] as i32, eb = cut[12] as i32,
        tu = cut[13] as i32, tv = cut[14] as i32, tw = cut[15] as i32, th = cut[16] as i32,
        bw2 = cut[17] as i32, feed = cut[18] as i32);
        let quad = insert::take_dim_quad();
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " fill copy rect={x} {y} {x2} {y2} pos={pos} authored={width}"
        ),
        x = quad[0] as i32, y = quad[1] as i32, x2 = quad[2] as i32, y2 = quad[3] as i32,
        pos = quad[4] as i32, width = quad[5] as i32);
    }
    if let Some(reading) = gui::tick() {
        let words = reading.facts;
        let mut line = String::with_capacity(512);
        for index in 0..gui::FACTS_WORDS {
            let name = gui::FACT_NAMES[index];
            let value = words[index];
            if gui::FACT_HEX.contains(&name) {
                let _ = write!(line, " {name}={value:#x}");
            } else {
                let _ = write!(line, " {name}={value}");
            }
        }
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " gauge row facts:{line}"
        ), line = line);
        // One insertion per gauge row; 40..49 are the insertion's own codes.
        match reading.outcome {
            Ok(done) => {
                log::error!(concat!(
                    "MSG ",
                    env!("MSG_VERSION"),
                    " scissor mask inserted: mask={mask:#x} map={map:#x} entries={entries} cache={cache:#x} records={records} dim={dim:#x} dim_tex={tex:#x} dim_flags={flags:#x}"
                ),
                mask = done.mask,
                map = done.map,
                entries = done.entries,
                cache = done.cache,
                records = done.records,
                dim = done.dim,
                tex = done.dim_texture,
                flags = done.dim_flags);
                // The dim blade is additive: its failure leaves the accepted cut as it is, so it
                // is reported on its own line, once per row.
                if done.dim_stage != 0 {
                    log::error!(concat!(
                        "MSG ",
                        env!("MSG_VERSION"),
                        " dim blade not added at stage {stage}"
                    ), stage = done.dim_stage);
                }
            }
            Err(49) => log::error!(concat!(
                "MSG ",
                env!("MSG_VERSION"),
                " gauge row already carries the inserted mask marker; not inserting again"
            )),
            Err(stage) => log::error!(concat!(
                "MSG ",
                env!("MSG_VERSION"),
                " mask insertion failed at stage {stage}: instance={instance} controller={controller} gauge={gauge}"
            ),
            stage = stage,
            instance = hooks::capture().0,
            controller = hooks::capture().1,
            gauge = sampling::gauge_controller()),
        }
    }
    let stage = gui::take_failure();
    // The row check retries at a slow cadence while the gauge HUD is absent, so the stage is
    // printed when it changes, not on every retry.
    if stage != 0 && stage != LAST_STAGE.swap(stage, Ordering::Relaxed) {
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " gauge row check stopped at stage {stage}: instance={instance} controller={controller} gauge={gauge}"
        ),
        stage = stage,
        instance = hooks::capture().0,
        controller = hooks::capture().1,
        gauge = sampling::gauge_controller());
    }
    let (calls, matched) = sampling::stats();
    if matched && !REPORTED_FIRST.swap(true, Ordering::Relaxed) {
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " gauge controller observed; the fill follows the local player"
        ));
    }
    // Each fire means the game freed the gauge row; it has fired zero times in six sessions,
    // so the mask's own liveness check is the teardown this build relies on.
    let tears = hooks::teardowns();
    if tears > LAST_TEARDOWNS.swap(tears, Ordering::Relaxed) {
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " gauge row root destroyed ({tears} times); row facts cleared"
        ), tears = tears);
    }
    // The inserted mask's links went away, which is a row dying at an area's loading screen.
    // The snapshot names which link broke; the instance fire count separates a HUD the game
    // stops drawing from one this plugin lost track of.
    let misses = insert::misses();
    if misses > LAST_MISSES.swap(misses, Ordering::Relaxed) {
        let lost = insert::link_lost();
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " inserted mask's links lost ({misses} times): marker={m:#x} mask.parent={mp:#x} mask.child={mc:#x} bar.parent={bp:#x} bar.sibling={bs:#x} root.child={rc:#x} instance hook fires={fires}"
        ),
        misses = misses, m = lost[0], mp = lost[1], mc = lost[2],
        bp = lost[3], bs = lost[4], rc = lost[5], fires = hooks::instance_fires());
        REPORTED_ABSENT.store(false, Ordering::Release);
    }
    // The walker runs for every animation controller, so a large call count
    // without a gauge match means the gauge HUD is absent in this state.
    if !matched && calls > ABSENT_AFTER_CALLS && !REPORTED_ABSENT.swap(true, Ordering::Relaxed) {
        log::error!(
            concat!(
                "MSG ",
                env!("MSG_VERSION"),
                " no sharpness gauge controller seen in {calls} walker calls (HUD absent?)"
            ),
            calls = calls
        );
    }
}
