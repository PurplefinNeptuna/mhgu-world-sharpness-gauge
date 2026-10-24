//! Modern sharpness gauge: the game's sharpness bar, clipped to the live ratio.
//!
//! Four hooks: the draw walker (main+0xa494a0) observes the gauge controller, the player's
//! sharpness fields and the node tree; the instance draw entry (main+0xa39300) records the row
//! instance; the row root's destructor (main+0xa486fc) clears that capture; the scissor mask's
//! enter handler (main+0xa4ba94) writes the clip box in the call that applies it. The build
//! writes game memory and inserts one GUI object, so it changes what is drawn. `src/gui.rs`
//! names the log lines and the native code behind them. Forge supplies lifecycle, allocation
//! and logging; stock Forge filters levels below ERROR, hence log::error! for diagnostics.
#![no_std]
extern crate alloc;
use alloc::string::String;
use core::fmt::Write as _;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use sharpness_gauge_core::{FIELDS, HEAD_WORDS, MAX_NODES, NODE_WORDS, FIELD_VISITED, ratio_of_words};
mod gui;
mod hooks;
mod insert;
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
static LAST_TEARDOWNS: AtomicU32 = AtomicU32::new(0);
/// A 120-update divider (about two seconds of frames) for the one clip probe line.
const CLIP_LOG_EVERY: u32 = 120;
static CLIP_LOG: Counter = Counter::new(CLIP_LOG_EVERY);

struct Counter {
    value: AtomicU32,
    every: u32,
}
impl Counter {
    const fn new(every: u32) -> Self {
        Self {
            value: AtomicU32::new(0),
            every,
        }
    }
    /// True once every `every` calls, so a hot path logs at a fixed low cadence.
    fn tick(&self) -> bool {
        let next = self.value.fetch_add(1, Ordering::Relaxed) + 1;
        next % self.every == 0
    }
}

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
/// which is also the leaf-exit bit the walker tests for a childless node,
/// 0x0001_0000 cleared on exit, 0x0000_4000 prepaint skip), the object kind, then
/// the own rectangle and the rectangle prepaint leaves after the clip intersection.
fn node_line(number: u32, index: usize, words: &[u32; sampling::WORDS]) -> String {
    let base = HEAD_WORDS + index * NODE_WORDS;
    let mut line = String::with_capacity(160);
    let _ = write!(
        line,
        concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " tree {number} n{index}: addr={a:#x} flags={f:#x} kind={k}",
            " own={l} {t} {r} {b} eff={el} {et} {er} {eb} tex={tu} {tv} {tw} {th}",
            " child={ch:#x} sibling={sb:#x} parent={pa:#x}"
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
        th = words[base + 14] as i32,
        ch = words[base + 15],
        sb = words[base + 16],
        pa = words[base + 17]
    );
    line
}

/// Logging runs here, outside the hook, formatted from the copied words.
#[unsafe(no_mangle)]
pub extern "C" fn forge_onUpdate(delta: f32) {
    let _ = delta;
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
            Ok(done) => log::error!(concat!(
                "MSG ",
                env!("MSG_VERSION"),
                " scissor mask inserted: mask={mask:#x} map={map:#x} entries={entries} cache={cache:#x} records={records}"
            ),
            mask = done.mask,
            map = done.map,
            entries = done.entries,
            cache = done.cache,
            records = done.records),
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
    if stage != 0 {
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
    // The rectangle the layout pass left in the mask between frames, next to the bar's rect.
    let probe = insert::probe();
    if probe[0] | probe[1] | probe[2] | probe[3] != 0 && CLIP_LOG.tick() {
        log::error!(concat!(
            "MSG ",
            env!("MSG_VERSION"),
            " clip mask rect={a} {b} {c} {d} flags={flags:#x} bar rect={l} {t} {r} {bb}"
        ),
        a = probe[0] as i32, b = probe[1] as i32, c = probe[2] as i32, d = probe[3] as i32,
        flags = probe[4],
        l = probe[5] as i32, t = probe[6] as i32, r = probe[7] as i32, bb = probe[8] as i32);
    }
    let (calls, taken, matched) = sampling::stats();
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
