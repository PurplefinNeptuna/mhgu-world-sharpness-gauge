//! Two Forge-owned trampolines: the draw walker entry main+0xa494a0, and the
//! instance draw entry main+0xa39300 that hands the walker its controller. The
//! second one only records the pair (instance, controller) so the insertion
//! code can reach the cGUIInstance that owns the gauge row, which is not
//! derivable from the controller. Installation is all-or-nothing: every
//! signature window is checked before any write, and a failure restores the
//! originals it touched.
use crate::{
    gui,
    memory,
    sampling,
    signatures::{HOOK, INSTANCE_HOOK, ROOT_DESTRUCTOR_HOOK, SIGNATURES},
};
use alloc::vec::Vec;
use core::{
    ffi::c_void,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use forge::patch::Patch;

core::arch::global_asm!(
    ".hidden msg_walker\n.hidden msg_walker_original\n.hidden msg_active\n\
     .hidden msg_instance\n.hidden msg_instance_original\n.hidden msg_instance_active\n\
     .hidden msg_root_dtor\n.hidden msg_root_dtor_original\n.hidden msg_root_dtor_active"
);
#[unsafe(no_mangle)]
pub static msg_walker_original: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub static msg_active: AtomicBool = AtomicBool::new(false);
#[unsafe(no_mangle)]
pub static msg_instance_original: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub static msg_instance_active: AtomicBool = AtomicBool::new(false);
#[unsafe(no_mangle)]
pub static msg_root_dtor_original: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub static msg_root_dtor_active: AtomicBool = AtomicBool::new(false);
// The instance whose +0xdc is the gauge controller, recorded as plain integers.
// Game pointers never leave this module; readers only use them synchronously
// on the same thread that the game called them on.
static INSTANCE: AtomicU32 = AtomicU32::new(0);
static CONTROLLER: AtomicU32 = AtomicU32::new(0);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<forge::sys::hook::Hook>() == 24);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(
    core::mem::offset_of!(forge::sys::hook::Hook, context) == 12
        && core::mem::offset_of!(forge::sys::hook::Hook, ctx_cell) == 16
        && core::mem::align_of::<forge::sys::hook::Hook>() == 4
);

/// The walker takes (controller, context): the animation controller whose draw
/// cache is about to be built and consumed. The observation runs before the
/// native walker so the copied values are the state native code will draw with,
/// and it performs no allocation, formatting, file access, floating point or
/// logging. Once its sample budget is spent it reads no game memory at all.
///
/// Flags: the relocated window main+0xa494a0..0xa494ac keeps `cmp r1,#0` and
/// the flag-consuming `popeq` in the same unit; this callback restores NZCVQ
/// through the normal AAPCS return and never writes APSR (verified by
/// scripts/verify.py disassembly scan; the window has no SEL/USAD8/MRS/MSR, so
/// no APSR_ge producer exists).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msg_walker(controller: u32, context: u32) {
    let original = msg_walker_original.load(Ordering::Acquire);
    if original == 0 {
        return;
    } // Installation never publishes an original of zero.
    if msg_active.load(Ordering::Acquire) {
        sampling::observe(controller, context);
    }
    let f: unsafe extern "C" fn(u32, u32) = unsafe { core::mem::transmute(original as usize) };
    unsafe { f(controller, context) };
}

/// The instance draw entry takes (instance, context) and reads the controller at
/// instance+0xdc before calling the walker. It is the only caller of the walker in
/// the whole program (one `bl` in .text), but every animation instance on the HUD
/// reaches it, so the pair is recorded only when its controller is the gauge
/// controller the sampler identified. Anything else is not the gauge row and is
/// ignored, which is what makes the recorded instance unambiguous.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msg_instance(instance: u32, context: u32) {
    let original = msg_instance_original.load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    if msg_instance_active.load(Ordering::Acquire) {
        let gauge = sampling::gauge_controller();
        if gauge != 0 {
            if let Some(controller) = memory::word(instance.wrapping_add(0xdc)) {
                if controller == gauge {
                    CONTROLLER.store(controller, Ordering::Relaxed);
                    INSTANCE.store(instance, Ordering::Relaxed);
                }
            }
        }
    }
    let f: unsafe extern "C" fn(u32, u32) = unsafe { core::mem::transmute(original as usize) };
    unsafe { f(instance, context) };
}

/// The row's root is being destroyed. cGUIObjRoot's vtable slot 0x04 is main+
/// +0xa486fc: it frees the row's draw cache, then walks root+0x68 and each +0x6c
/// sibling and calls slot 0x04 on every one of them, which is how the mask and the
/// three textures this insertion linked into that chain get freed. The gauge HUD's
/// teardown is therefore this call, on this thread, with the object as its argument.
/// The capture is dropped before the original runs, so nothing in this plugin
/// touches the tree, the mask or the cache once the game owns them again. Roots
/// other than the captured controller are untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msg_root_dtor(root: u32) {
    let original = msg_root_dtor_original.load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    if msg_root_dtor_active.load(Ordering::Acquire)
        && root != 0
        && root == sampling::gauge_controller()
    {
        TEARDOWNS.fetch_add(1, Ordering::Relaxed);
        gui::on_teardown();
        clear_capture(root);
        sampling::clear_controller();
    }
    let f: unsafe extern "C" fn(u32) = unsafe { core::mem::transmute(original as usize) };
    unsafe { f(root) };
}

/// Times the captured gauge row's root destructor ran. Zero for a whole session
/// means this hook never proved the row's teardown, and the plugin kept no
/// assumption that it did.
pub fn teardowns() -> u32 {
    TEARDOWNS.load(Ordering::Relaxed)
}
static TEARDOWNS: AtomicU32 = AtomicU32::new(0);

/// The raw recorded pair, for the log line. Zeros mean the instance hook never
/// saw a controller matching the gauge controller.
pub fn capture() -> (u32, u32) {
    (INSTANCE.load(Ordering::Relaxed), CONTROLLER.load(Ordering::Relaxed))
}

/// The recorded pair, only handed out when the gauge controller is the controller
/// this instance draws. Returns (instance, controller).
pub fn gauge_instance() -> Option<(u32, u32)> {
    let instance = INSTANCE.load(Ordering::Relaxed);
    let controller = CONTROLLER.load(Ordering::Relaxed);
    let gauge = sampling::gauge_controller();
    if instance == 0 || controller == 0 || gauge == 0 || controller != gauge {
        return None;
    }
    Some((instance, controller))
}

/// Drop a capture whose object is gone. Called when a row read fails (the row
/// pointer is unreadable) or the row is no longer the sharpness animation, which
/// is how the log shows a captured controller after its scene was torn down. The
/// pair is only cleared when it is the pair that failed, so a fresh capture taken
/// by the instance hook in the meantime stays.
pub fn clear_capture(controller: u32) {
    if controller == 0 {
        return;
    }
    let _ = CONTROLLER.compare_exchange(controller, 0, Ordering::AcqRel, Ordering::Relaxed);
    if CONTROLLER.load(Ordering::Relaxed) == 0 {
        INSTANCE.store(0, Ordering::Release);
    }
}

/// Restore only our branch words (or a partially installed original/branch
/// pair). Foreign modifications keep their ownership; Forge keeps allocated
/// continuations reachable for the process lifetime.
fn rollback(patch: &mut Patch) -> bool {
    msg_active.store(false, Ordering::Release);
    let mut current = [0; 8];
    if !memory::copy(patch.addr(), &mut current) {
        return false;
    }
    let original: [u8; 8] = patch.original_bytes().try_into().unwrap();
    let branch: [u8; 8] = patch.patch_bytes().try_into().unwrap();
    if !(0..8)
        .step_by(4)
        .all(|i| current[i..i + 4] == original[i..i + 4] || current[i..i + 4] == branch[i..i + 4])
    {
        return false;
    }
    if current != original {
        patch.enable();
        patch.disable();
    }
    memory::sync_code(patch.addr(), 8)
        && memory::copy(patch.addr(), &mut current)
        && current == original
}

/// Plant one Forge hook over the first eight bytes of a function entry. Both
/// targets start with a `push` plus a stack-pointer arithmetic word, so neither
/// relocated word contains a PC-relative operand. The continuation must show the
/// original eight bytes, then the same branch form back to target+8.
fn plant(base: u32, offset: u32, detour: u32, published: &AtomicU32) -> Result<Patch, &'static str> {
    let target = base + offset;
    let mut branch = [0; 8];
    branch[..4].copy_from_slice(&0xe51ff004u32.to_le_bytes());
    branch[4..].copy_from_slice(&detour.to_le_bytes());
    let mut patch = Patch::new(target, &branch, false);
    let mut original = core::ptr::null();
    let hook = unsafe {
        forge::sys::hook::forge_hook_create(target as *const c_void, detour as *const c_void, &mut original)
    };
    let continuation = original as u32;
    let mut code = [0; 16];
    let valid = continuation != 0
        && continuation & 3 == 0
        && memory::executable(continuation, 16)
        && memory::copy(continuation, &mut code)
        && &code[..8] == patch.original_bytes()
        && code[8..12] == 0xe51ff004u32.to_le_bytes()
        && code[12..] == (target + 8).to_le_bytes();
    if valid {
        published.store(continuation, Ordering::Release);
    }
    let mut actual = [0; 8];
    if !valid
        || hook.original != original
        || hook.target as u32 != target
        || hook.detour as u32 != detour
        || !memory::copy(target, &mut actual)
        || actual != patch.patch_bytes()
        || !memory::sync_code(target, 8)
    {
        let restored = rollback(&mut patch);
        published.store(0, Ordering::Release);
        return Err(if restored {
            "Forge hook failed; restored original"
        } else {
            "hook rollback failed; restart required"
        });
    }
    Ok(patch)
}

pub fn install() -> Result<Vec<Patch>, &'static str> {
    let base = forge::mem::text_addr();
    memory::BASE.store(base, Ordering::Relaxed);
    if base == 0 || base > 0xfe00_0000 || base & 3 != 0 || !memory::correct_program() {
        return Err("unsupported title/address");
    }
    for &(offset, bytes) in SIGNATURES {
        let mut actual = [0; 32];
        if !memory::executable(base + offset, 32)
            || !memory::copy(base + offset, &mut actual)
            || actual != bytes
        {
            return Err("MHGU 1.4.0 code mismatch at a required function");
        }
    }
    // The walker is planted first: the gauge row must be identifiable before the
    // instance pair is of any use, and the instance capture only records pairs. The
    // root destructor is last, because clearing a capture is only useful once the
    // other two hooks can produce one. A failure restores every patch this call
    // planted, newest first, and publishes nothing.
    let plan = [
        (
            HOOK,
            msg_walker as *const () as u32,
            &msg_walker_original as &AtomicU32,
            &msg_active as &AtomicBool,
        ),
        (
            INSTANCE_HOOK,
            msg_instance as *const () as u32,
            &msg_instance_original,
            &msg_instance_active,
        ),
        (
            ROOT_DESTRUCTOR_HOOK,
            msg_root_dtor as *const () as u32,
            &msg_root_dtor_original,
            &msg_root_dtor_active,
        ),
    ];
    let mut planted: Vec<Patch> = Vec::new();
    for (offset, detour, published, armed) in plan {
        if let Err(message) = plant(base, offset, detour, published) {
            armed.store(false, Ordering::Release);
            let mut restored = true;
            for patch in planted.iter_mut().rev() {
                restored = rollback(patch) && restored;
            }
            return Err(if restored {
                message
            } else {
                "hook rollback failed; restart required"
            });
        }
        if let Some(patch) = planted.last() {
            if !memory::sync_code(patch.addr(), 8) {
                return Err("hook continuation not live; restart required");
            }
        }
    }
    msg_active.store(true, Ordering::Release);
    msg_instance_active.store(true, Ordering::Release);
    msg_root_dtor_active.store(true, Ordering::Release);
    Ok(planted)
}
