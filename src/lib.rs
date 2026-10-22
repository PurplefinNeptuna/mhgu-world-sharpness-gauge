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
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use sharpness_gauge_core::{FIELDS, HEAD_WORDS, MAX_NODES, NODE_WORDS, FIELD_VISITED, ratio_of_words};
mod gui;
mod hooks;
mod memory;
mod sampling;
mod signatures;

/// Samples drained per frame. Each sample prints one summary line plus one
/// line per walked node, so the drain rate is bounded well below the budget.
const DRAIN_PER_FRAME: usize = 1;
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

/// Pointers print in hexadecimal, screen coordinates and the signed gauge
/// words in decimal, counts in decimal, so a log line is readable without
/// losing any bits.
fn append_field(line: &mut String, name: &str, word: u32) {
    if matches!(
        name,
        "ctrl" | "row" | "owner" | "cache" | "cache_flags" | "root" | "player"
            | "context" | "input_flags"
    ) {
        let _ = write!(line, " {name}={word:#x}");
    } else if matches!(
        name,
        "current" | "maximum" | "clip_l" | "clip_t" | "clip_r" | "clip_b"
    ) {
        let _ = write!(line, " {name}={value}", value = word as i32);
    } else {
        let _ = write!(line, " {name}={word}");
    }
}

/// One sample: identity, chain, gauge words, the live clip and the walk counts.
/// The node blocks print separately, one line each, in `node_line`.
fn sample_line(number: u32, words: &[u32; sampling::WORDS]) -> String {
    let mut line = String::with_capacity(512);
    let _ = write!(
        line,
        concat!("MSG ", env!("MSG_VERSION"), " sample {number}/{MAX}: "),
        number = number,
        MAX = sampling::MAX_SAMPLES
    );
    for index in 0..HEAD_WORDS {
        append_field(&mut line, FIELDS[index], words[index]);
    }
    match ratio_of_words(words) {
        Some(points) => {
            let _ = write!(line, " ratio_bp={points}");
        }
        None => {
            // Unsupported state: no player, no gauge fields, or a maximum that
            // is not verified positive. Nothing is drawn from it.
            let _ = write!(line, " ratio=unsupported");
        }
    }
    line
}

/// One node: address, the whole flag word (bit 0 visible, 0x0200_0000 drawable,
/// 0x0040_0000 leaf exit, 0x0000_4000 prepaint skip), the object kind, then the
/// own rectangle and the rectangle prepaint leaves after the clip intersection.
fn node_line(number: u32, index: usize, words: &[u32; sampling::WORDS]) -> String {
    let base = HEAD_WORDS + index * NODE_WORDS;
    let mut line = String::with_capacity(160);
    let _ = write!(
        line,
        concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " tree {number} n{index}: addr={a:#x} flags={f:#x} kind={k}",
            " own={l} {t} {r} {b} eff={el} {et} {er} {eb} tex={tu} {tv} {tw} {th}"
        ),
        number = number,
        index = index,
        a = words[base],
        f = words[base + 1],
        k = words[base + 2],
        l = words[base + 3] as i32,
        t = words[base + 4] as i32,
        r = words[base + 5] as i32,
        b = words[base + 6] as i32,
        el = words[base + 7] as i32,
        et = words[base + 8] as i32,
        er = words[base + 9] as i32,
        eb = words[base + 10] as i32,
        tu = words[base + 11] as i32,
        tv = words[base + 12] as i32,
        tw = words[base + 13] as i32,
        th = words[base + 14] as i32
    );
    line
}

/// The prototype ratio cycle runs here, outside the hook: the mask is inserted once
/// and its clip is written from the elapsed wall clock of the plugin's own frames.
static ELAPSED: AtomicU32 = AtomicU32::new(0);
static LAST_BOX: AtomicU32 = AtomicU32::new(0);
static REPORTED_INSERT: AtomicBool = AtomicBool::new(false);
const GRACE_SECONDS: u32 = 6;

/// Logging runs here, outside the hook, formatted from the copied words.
#[unsafe(no_mangle)]
pub extern "C" fn forge_onUpdate(delta: f32) {
    let seconds = ELAPSED.fetch_add((delta * 1_000.0).max(0.0) as u32, Ordering::Relaxed) / 1_000;
    gui::tick();
    let stage = gui::take_failure();
    if stage != 0 {
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " mask insertion failed at stage {stage}: instance={instance} controller={controller} gauge={gauge}"
        ),
        stage = stage,
        instance = hooks::capture().0,
        controller = hooks::capture().1,
        gauge = sampling::gauge_controller());
    }
    if gui::inserted() && !REPORTED_INSERT.swap(true, Ordering::Relaxed) {
        let (mask, objects, cache) = gui::applied();
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " scissor mask inserted; row map {objects} bindings, draw cache {cache} bytes, mask {mask}"
        ), objects = objects, cache = cache, mask = mask);
    }
    if gui::inserted() && seconds > GRACE_SECONDS {
        let ratio = sharpness_gauge_core::prototype_ratio(seconds - GRACE_SECONDS);
        if let Some(box_) = gui::apply(ratio) {
            let key = (box_[2] as u32) << 16 | (ratio & 0xffff);
            if LAST_BOX.swap(key, Ordering::Relaxed) != key {
                log::error!(concat!(
                    "MSG ",
                    env!("MSG_VERSION"),
                    " clip x={x} y={y} width={width} height={height} ratio={ratio}bp seconds={seconds} bar_width={bar_width} authored={authored}"
                ),
                x = box_[0],
                y = box_[1],
                width = box_[2],
                height = box_[3],
                ratio = ratio,
                seconds = seconds,
                bar_width = gui::bar_width().0,
                authored = gui::bar_width().1);
            }
        }
    }
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
        let walked = words[FIELD_VISITED];
        let limit = if walked > MAX_NODES as u32 {
            MAX_NODES
        } else {
            walked as usize
        };
        for index in 0..limit {
            log::error!("{}", node_line(number, index, &words));
        }
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
