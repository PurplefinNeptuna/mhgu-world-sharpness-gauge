//! Four Forge-owned trampolines: the draw walker (main+0xa494a0), the instance draw entry
//! that hands it a controller (main+0xa39300, its first argument is the cGUIInstance owning
//! the row), the row root's destructor (main+0xa486fc), and the scissor mask's enter handler
//! (main+0xa4ba94). Installation is all-or-nothing: every signature window is checked before
//! any write, and a failure restores the originals it touched.
use crate::{
    gui,
    insert,
    memory,
    sampling,
    signatures::{HOOK, INSTANCE_HOOK, MASK_ENTER_HOOK, ROOT_DESTRUCTOR_HOOK, SIGNATURES},
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
     .hidden msg_root_dtor\n.hidden msg_root_dtor_original\n.hidden msg_root_dtor_active\n\
     .hidden msg_mask_enter\n.hidden msg_mask_enter_original\n.hidden msg_mask_enter_active"
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
#[unsafe(no_mangle)]
pub static msg_mask_enter_original: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub static msg_mask_enter_active: AtomicBool = AtomicBool::new(false);
// The instance whose +0xdc is the gauge controller, as plain integers: readers use them
// synchronously on the thread the game called them on.
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

/// The walker takes (controller, context). The observation runs before the native walker so
/// the copied values are the state native code will draw with; it allocates, formats, logs,
/// does floating point and, past its sample budget, reads no game memory.
///
/// Flags: the relocated window main+0xa494a0..0xa494ac keeps `cmp r1,#0` and the
/// flag-consuming `popeq` in the same unit; this callback restores NZCVQ through the normal
/// AAPCS return and never writes APSR (verify.py scans the disassembly; the window has no
/// SEL/USAD8/MRS/MSR, so no APSR_ge producer exists).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msg_walker(controller: u32, context: u32) {
    let original = msg_walker_original.load(Ordering::Acquire);
    if original == 0 {
        return;
    } // Installation never publishes an original of zero.
    if msg_active.load(Ordering::Acquire) {
        sampling::observe(controller, context);
        // Three word reads while a mask is inserted, and the only reacquire signal that runs
        // when the inserted mask dies with its row.
        insert::check_liveness();
    }
    let f: unsafe extern "C" fn(u32, u32) = unsafe { core::mem::transmute(original as usize) };
    unsafe { f(controller, context) };
}

/// The instance draw entry takes (instance, context) and reads the controller at instance+0xdc
/// before calling the walker. Every HUD animation instance reaches it, so the pair is recorded
/// only when the controller is the gauge controller the sampler identified.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msg_instance(instance: u32, context: u32) {
    let original = msg_instance_original.load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    if msg_instance_active.load(Ordering::Acquire) {
        FIRES.fetch_add(1, Ordering::Relaxed);
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

/// The row root's destructor (cGUIObjRoot vtable slot 0x04, main+0xa486fc) frees the row's
/// draw cache and then deletes its child chain, which is where the inserted mask and its
/// children go. The capture is dropped before the original runs. Roots other than the captured
/// controller are untouched. It has fired zero times in six sessions: the mask's liveness is
/// checked at its enter hook, which is the teardown this build can prove.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msg_root_dtor(root: u32) {
    let original = msg_root_dtor_original.load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    // The argument is the row's root node: the r28 tree lines show the root's parent is the
    // controller, so the controller is never this argument, and the old comparison was dead.
    if msg_root_dtor_active.load(Ordering::Acquire)
        && root != 0
        && root == insert::row_root()
    {
        TEARDOWNS.fetch_add(1, Ordering::Relaxed);
        teardown_gauge_row(sampling::gauge_controller());
    }
    let f: unsafe extern "C" fn(u32) = unsafe { core::mem::transmute(original as usize) };
    unsafe { f(root) };
}

/// The mask's own enter handler (main+0xa4ba94), reached through the class vtable during the
/// draw walk: the call that reads the mask's rectangle at +0xb0..+0xbc and applies the scissor.
/// The game's layout rebuilds that rectangle from the node's own quad, and a node the animation
/// row has no entry for comes out with width 0, which is r18's missing gauge: 376 of 383 probe
/// lines read left == right with flags 0x1. The box is written here, in this same call,
/// immediately before the original reads it. Only the mask carrying this plugin's marker is
/// touched; every other scissor mask pays one word read and a compare.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msg_mask_enter(mask: u32, _context: u32) {
    let original = msg_mask_enter_original.load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    if msg_mask_enter_active.load(Ordering::Acquire) {
        insert::enter(mask);
    }
    let f: unsafe extern "C" fn(u32, u32) = unsafe { core::mem::transmute(original as usize) };
    unsafe { f(mask, _context) };
}

/// The gauge row is gone: drop every address this plugin holds into it, so the sampler, the
/// instance capture, the row facts and the insertion all start over on the HUD the game builds
/// next. Quests are split into areas with a loading screen between them (the tester's account,
/// 2026-10-24, matching the logs: HUD-absent stretches, a bar rect that changes layout, then
/// freed objects), so a row's death is a normal event of a hunt, not an edge case. Two paths
/// reach this: the root destructor hook, and the inserted mask's own link check.
pub fn teardown_gauge_row(controller: u32) {
    gui::on_teardown();
    clear_capture(controller);
    sampling::clear_controller();
}

/// Times the root destructor ran, and times the instance draw entry fired. Both counts are
/// diagnostics: a HUD that is never rebuilt shows zero instance fires after a row's drop.
pub fn teardowns() -> u32 {
    TEARDOWNS.load(Ordering::Relaxed)
}
pub fn instance_fires() -> u32 {
    FIRES.load(Ordering::Relaxed)
}
static TEARDOWNS: AtomicU32 = AtomicU32::new(0);
static FIRES: AtomicU32 = AtomicU32::new(0);

/// The recorded pair, for the log line; zeros mean it was never captured.
pub fn capture() -> (u32, u32) {
    (INSTANCE.load(Ordering::Relaxed), CONTROLLER.load(Ordering::Relaxed))
}

/// The recorded pair, only when its controller is still the gauge controller.
pub fn gauge_instance() -> Option<(u32, u32)> {
    let instance = INSTANCE.load(Ordering::Relaxed);
    let controller = CONTROLLER.load(Ordering::Relaxed);
    let gauge = sampling::gauge_controller();
    if instance == 0 || controller == 0 || gauge == 0 || controller != gauge {
        return None;
    }
    Some((instance, controller))
}

/// Drop a capture whose object is gone. Only the failing pair is cleared, so a fresh capture
/// taken in the meantime stays.
pub fn clear_capture(controller: u32) {
    if controller == 0 {
        return;
    }
    let _ = CONTROLLER.compare_exchange(controller, 0, Ordering::AcqRel, Ordering::Relaxed);
    if CONTROLLER.load(Ordering::Relaxed) == 0 {
        INSTANCE.store(0, Ordering::Release);
    }
}

/// Restore only our branch words; foreign modifications keep their ownership.
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

/// Plant one Forge hook over the first eight bytes of a function entry. Every target in this
/// mod starts with a `push` plus a stack-pointer arithmetic word, so no relocated word is
/// PC-relative. The continuation must show the original eight bytes, then the branch back to
/// target+8.
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
    // The walker goes first (the row must be identifiable before the instance pair means
    // anything), the mask enter handler last (it only matters once an insertion exists). A
    // failure restores every patch this call planted, newest first, and publishes nothing.
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
        (
            MASK_ENTER_HOOK,
            msg_mask_enter as *const () as u32,
            &msg_mask_enter_original,
            &msg_mask_enter_active,
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
    msg_mask_enter_active.store(true, Ordering::Release);
    Ok(planted)
}
