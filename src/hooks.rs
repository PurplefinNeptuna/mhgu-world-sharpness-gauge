//! One Forge-owned trampoline at the draw walker entry main+0xa494a0.
//! Installation is all-or-nothing: every signature window is checked before
//! any write, and a failure restores the originals it touched.
use crate::{
    memory,
    sampling,
    signatures::{HOOK, SIGNATURES},
};
use alloc::vec::Vec;
use core::{
    ffi::c_void,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use forge::patch::Patch;

core::arch::global_asm!(
    ".hidden msg_walker\n.hidden msg_walker_original\n.hidden msg_active"
);
#[unsafe(no_mangle)]
pub static msg_walker_original: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub static msg_active: AtomicBool = AtomicBool::new(false);
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
    // The entry prologue push {r4,r5,r11,lr} plus mov r4,r1 contains no
    // PC-relative operand, so Forge can relocate the first eight bytes.
    let target = base + HOOK;
    let detour = msg_walker as *const () as u32;
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
        msg_walker_original.store(continuation, Ordering::Release);
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
        return Err(if restored {
            "Forge hook failed; restored original"
        } else {
            "hook rollback failed; restart required"
        });
    }
    msg_active.store(true, Ordering::Release);
    Ok(alloc::vec::Vec::from_iter([patch]))
}
