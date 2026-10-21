//! Modern sharpness gauge: a read-only diagnostic candidate.
//!
//! One hook, at the draw walker entry (main+0xa494a0), observes the native
//! sharpness gauge controller, the local player's sharpness fields and the
//! gauge node tree. It never writes game memory, never creates GUI objects and
//! never changes what is drawn. Forge supplies lifecycle, allocation and
//! logging; stock Forge filters log levels below ERROR.
#![no_std]
extern crate alloc;
use alloc::string::String;
use core::fmt::Write as _;
use core::sync::atomic::{AtomicBool, Ordering};
use sharpness_gauge_core::{FIELDS, gauge_of, ratio_of_words};
mod hooks;
mod memory;
mod sampling;
mod signatures;

/// The number of samples drained per frame. Formatting is cheap compared to
/// the log line itself, and the budget is 60 lines per process.
const DRAIN_PER_FRAME: usize = 2;
/// Walker calls without a gauge controller that count as "the HUD is absent".
const ABSENT_AFTER_CALLS: u32 = 6_000;
static REPORTED_FIRST: AtomicBool = AtomicBool::new(false);
static REPORTED_ABSENT: AtomicBool = AtomicBool::new(false);
static REPORTED_FULL: AtomicBool = AtomicBool::new(false);

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
                " loaded (diagnostic; no drawing changes)"
            ));
        }
        Err(reason) => log::error!("Modern Sharpness Gauge disabled: {reason}"),
    }
}

/// Pointers print in hexadecimal, counts and gauge values in decimal, so a
/// log line is readable without losing any bits.
fn append_field(line: &mut String, name: &str, value: u32) {
    let hex = matches!(
        name,
        "ctrl" | "row" | "owner" | "cache" | "root" | "node" | "node_flags" | "player"
            | "cache_flags"
    );
    if hex {
        let _ = write!(line, " {name}={value:#x}");
    } else {
        let _ = write!(line, " {name}={value}");
    }
}

fn sample_line(number: u32, words: &[u32; sampling::WORDS]) -> String {
    let mut line = String::with_capacity(256);
    let _ = write!(
        line,
        concat!("MSG ", env!("MSG_VERSION"), " sample {number}/{MAX}: "),
        number = number,
        MAX = sampling::MAX_SAMPLES
    );
    for (name, value) in FIELDS.iter().zip(words) {
        append_field(&mut line, name, *value);
    }
    match (gauge_of(words), ratio_of_words(words)) {
        (Some(gauge), Some(points)) => {
            let _ = write!(line, " ratio_bp={points} tier={}", gauge.tier);
        }
        _ => {
            // Unsupported state: no player, no gauge fields, or a maximum that
            // is not verified positive. Nothing is drawn from it.
            let _ = write!(line, " ratio=unsupported");
        }
    }
    line
}

/// Logging runs here, outside the hook, formatted from the copied words.
#[unsafe(no_mangle)]
pub extern "C" fn forge_onUpdate(_delta: f32) {
    for _ in 0..DRAIN_PER_FRAME {
        let Some((number, words)) = sampling::take() else {
            break;
        };
        if !REPORTED_FIRST.swap(true, Ordering::Relaxed) {
            log::error!(concat!(
                "MSG ",
                env!("MSG_VERSION"),
                " gauge controller observed; sampling the native gauge"
            ));
        }
        log::error!("{}", sample_line(number, &words));
    }
    let (calls, taken, matched) = sampling::stats();
    if matched && taken == sampling::MAX_SAMPLES && !REPORTED_FULL.swap(true, Ordering::Relaxed) {
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " sample budget complete; the hook no longer reads game memory"
        ));
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

/// Forge supplies the live ImGui context and its matching allocator pair. They
/// stay loader-owned. This mod has no settings, so the panel below is empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forge_onImGuiInit(
    ctx: *mut core::ffi::c_void,
    alloc: *mut core::ffi::c_void,
    free: *mut core::ffi::c_void,
) {
    if ctx.is_null() || alloc.is_null() || free.is_null() {
        return;
    }
    unsafe {
        forge::imgui::set_current_context(&mut *ctx.cast());
        forge::imgui::set_allocator_functions(
            core::mem::transmute(alloc),
            core::mem::transmute(free),
        );
    }
}

#[forge::imgui_render]
fn panel() {}
