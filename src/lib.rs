//! World sharpness gauge: the game's sharpness bar, clipped to the local player's fill inside the
//! current tier's band, computed in the mask's enter hook. The box is built from the accepted cut
//! endpoints, as offsets on the bar's own rect, scaled by that rect's live width. There are no
//! settings and therefore no Plugin UI panel.
//!
//! Three hooks: the draw walker (main+0xa494a0) finds the gauge controller and the local player;
//! the instance draw entry (main+0xa39300) records the row instance; the scissor mask's enter
//! handler (main+0xa4ba94) writes the clip box in the call that applies it. The build writes game memory and inserts one GUI
//! object, so it changes what is drawn. `src/gui.rs` names the native code its row gates rest on. Forge supplies lifecycle, allocation and logging; stock Forge filters levels
//! below ERROR, hence log::error! for diagnostics.
#![no_std]
extern crate alloc;
use core::sync::atomic::{AtomicU32, Ordering};
mod gui;
mod hooks;
mod insert;
mod memory;
mod sampling;
mod signatures;

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
    let label = concat!("World Sharpness Gauge ", env!("WSG_VERSION")).as_bytes();
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
                "World Sharpness Gauge ",
                env!("WSG_VERSION"),
                " loaded (clips the sharpness gauge to the fill inside its tier)"
            ));
        }
        Err(reason) => log::error!("World Sharpness Gauge disabled: {reason}"),
    }
}

/// Logging runs here, outside the hook: one line per lifecycle event - a mask inserted, an
/// insertion stopped at a stage, a gauge row torn down - and nothing per frame. Stock Forge keeps
/// levels below ERROR out of the log, so these are error-level.
#[unsafe(no_mangle)]
pub extern "C" fn forge_onUpdate(_delta: f32) {
    let _ = _delta;
    // One insertion per gauge row, driven by the row check's own slow retry cadence; 40..49 are
    // the insertion's own codes.
    if let Some(reading) = gui::tick() {
        match reading.outcome {
            Ok(done) => {
                log::error!(concat!(
                    "WSG ",
                    env!("WSG_VERSION"),
                    " scissor mask inserted: mask={mask:#x} map={map:#x} entries={entries} cache={cache:#x} records={records} dim={dim:#x} clip={clip:#x}"
                ),
                mask = done.mask, map = done.map, entries = done.entries, cache = done.cache,
                records = done.records, dim = done.dim, clip = done.clip);
                // The shade blade is additive: its failure leaves the accepted cut as it is, so it
                // is reported on its own line, once per row. 50 the allocation, 51 the constructor
                // and class test, 52 the fields.
                if done.dim_stage != 0 {
                    log::error!(concat!(
                        "WSG ",
                        env!("WSG_VERSION"),
                        " shade blade not added at stage {stage}"
                    ), stage = done.dim_stage);
                }
            }
            Err(49) => log::error!(concat!(
                "WSG ",
                env!("WSG_VERSION"),
                " gauge row already carries the inserted mask marker; not inserting again"
            )),
            Err(stage) => log::error!(concat!(
                "WSG ",
                env!("WSG_VERSION"),
                " mask insertion failed at stage {stage}: instance={instance} controller={controller} gauge={gauge}"
            ),
            stage = stage,
            instance = hooks::capture().0,
            controller = hooks::capture().1,
            gauge = sampling::gauge_controller()),
        }
    }
    // The row check retries while the gauge HUD is absent, so the stage is printed when it
    // changes, not on every retry.
    let stage = gui::take_failure();
    if stage != 0 && stage != LAST_STAGE.swap(stage, Ordering::Relaxed) {
        log::error!(concat!(
            "WSG ",
            env!("WSG_VERSION"),
            " gauge row check stopped at stage {stage}: instance={instance} controller={controller} gauge={gauge}"
        ),
        stage = stage,
        instance = hooks::capture().0,
        controller = hooks::capture().1,
        gauge = sampling::gauge_controller());
    }
    // Whether the bar was drawn as the game placed it, and the box modelled from that draw against
    // the game's own rect: in a plain game the two must agree.
    if let Some((drawn, rect, plain)) = insert::take_rect_report() {
        log::error!(concat!(
            "WSG ",
            env!("WSG_VERSION"),
            " bar drawn {kind}: drawn={drawn:?} rect={rect:?}"
        ), kind = if plain { "plain" } else { "rescaled" }, drawn = drawn, rect = rect);
    }
    // The fill's gates refused, or a native call was refused, so the stock bar is showing.
    if insert::take_fill_refused() {
        log::error!(concat!(
            "WSG ",
            env!("WSG_VERSION"),
            " fill refused; the stock bar shows while it stays refused"
        ));
    }
    // The inserted mask's links went away, which is a gauge row ending with its quest.
    let misses = insert::misses();
    if misses > LAST_MISSES.swap(misses, Ordering::Relaxed) {
        log::error!(concat!(
            "WSG ",
            env!("WSG_VERSION"),
            " inserted mask's links lost ({misses} times); captured addresses cleared"
        ), misses = misses);
    }
}
