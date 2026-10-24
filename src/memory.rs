//! Checked native memory access and cache synchronization. Forge owns the
//! hooks and its own allocations. Forge's public API exposes no mapping query,
//! program-id query or process cache flush, so these three narrow SVC wrappers
//! cover that verified gap; the selected libnx wrappers are in `svc.S`.
use core::sync::atomic::AtomicU32;

pub static BASE: AtomicU32 = AtomicU32::new(0);
// Horizon's pseudo-handle denotes this running process; it needs no ownership
// and must not be closed.
const CURRENT_PROCESS: u32 = 0xffff8001;
// Match the kernel's query-memory output layout exactly, padding included.
#[repr(C)]
#[derive(Default)]
struct MemoryInfo {
    address: u64,
    size: u64,
    kind: u32,
    attributes: u32,
    permissions: u32,
    ipc: u32,
    device: u32,
    padding: u32,
}
const _: () = assert!(core::mem::size_of::<MemoryInfo>() == 40);
unsafe extern "C" {
    fn msg_svc_query(info: *mut MemoryInfo, page: *mut u32, address: u64) -> u32;
    fn msg_svc_get_info(out: *mut u64, id: u32, handle: u32, sub_id: u64) -> u32;
    fn msg_svc_flush(handle: u32, address: u64, bytes: u64) -> u32;
}
core::arch::global_asm!(include_str!("svc.S"), options(raw));

/// GetInfo selector 18 is the program ID; it distinguishes MHGU 1.4.0 from any
/// other title before a single game byte is read.
pub fn correct_program() -> bool {
    let mut title = 0;
    // SAFETY: writable output, current-process pseudo-handle, pinned SVC ABI.
    unsafe { msg_svc_get_info(&mut title, 18, CURRENT_PROCESS, 0) == 0 && title == 0x0100770008dd8000 }
}

fn mapped(address: u32, bytes: usize, permissions: u32) -> bool {
    if address == 0 || address as u64 + bytes as u64 > 0x1_0000_0000 {
        return false;
    }
    let mut info = MemoryInfo::default();
    let mut page = 0;
    // SAFETY: outputs are initialized, naturally aligned, kernel-sized.
    unsafe {
        msg_svc_query(&mut info, &mut page, address as u64) == 0
            && (info.permissions & permissions) == permissions
            && info.address <= address as u64
            && address as u64 + bytes as u64 <= info.address + info.size
    }
}

pub fn readable(address: u32, bytes: usize) -> bool {
    mapped(address, bytes, 1)
}

// Permission bit 1 (read) plus 4 (execute) must cover the whole Forge
// trampoline; the RX alias stays live until process exit.
pub fn executable(address: u32, bytes: usize) -> bool {
    mapped(address, bytes, 5)
}

pub fn sync_code(address: u32, bytes: usize) -> bool {
    // SAFETY: caller supplies a validated executable span; this is the tested
    // ARM32 process cache-flush contract.
    let result = unsafe { msg_svc_flush(CURRENT_PROCESS, address as u64, bytes as u64) };
    // SAFETY: barriers move no memory; they publish the flushed instructions.
    unsafe { core::arch::asm!("dsb sy", "isb", options(nostack, preserves_flags)) };
    result == 0
}

/// Read one word without forming a reference to a game-owned object. The
/// mapping query runs per read, so callers use it sparingly and synchronously.
pub fn word(address: u32) -> Option<u32> {
    if address & 3 != 0 || !readable(address, 4) {
        return None;
    }
    // SAFETY: alignment and mapping checked; the synchronous native caller
    // holds the object's lifetime for the whole call.
    Some(unsafe { core::ptr::read_volatile(address as *const u32) })
}

pub fn copy(address: u32, out: &mut [u8]) -> bool {
    if !readable(address, out.len()) {
        return false;
    }
    // SAFETY: source lifetime is held by the synchronous native caller; the
    // destination is plugin-owned.
    unsafe { core::ptr::copy_nonoverlapping(address as *const u8, out.as_mut_ptr(), out.len()) };
    true
}

/// Read one halfword; the row description stores object and sequence counts as
/// 16-bit fields, so they need their own checked read.
pub fn half(address: u32) -> Option<u16> {
    if address & 1 != 0 || !readable(address, 2) {
        return None;
    }
    // SAFETY: alignment and mapping checked; the caller holds the native
    // object's lifetime for the whole call.
    Some(unsafe { core::ptr::read_volatile(address as *const u16) })
}

/// A write target is accepted only where the kernel reports the span as writable, so a
/// stray pointer can never hit code or a read-only page.
pub fn writable(address: u32, bytes: usize) -> bool {
    mapped(address, bytes, 3)
}

/// Read a virtual method out of a game object's vtable, checked executable.
pub fn method(object: u32, slot: u32) -> Option<u32> {
    let vft = word(object)?;
    let f = word(vft.checked_add(slot)?)?;
    executable(f, 4).then_some(f)
}

/// Write one word after alignment and writable-mapping checks. No reference is formed to
/// a game-owned object, so no aliasing assumption is imposed on it. The caller must be
/// inside a native callback that made the object reachable in the same call.
pub fn write_word(address: u32, value: u32) -> bool {
    if address & 3 != 0 || !writable(address, 4) {
        return false;
    }
    // SAFETY: mapping checked above; the caller holds the native object's lifetime for
    // the whole write sequence.
    unsafe { core::ptr::write_volatile(address as *mut u32, value) };
    true
}

/// Copy `words` words from `source` to `target`, both checked for their span first. Used
/// to copy a row's object array into a larger block; a source the plugin cannot read is
/// a hard failure, never a partial copy.
pub fn copy_words(source: u32, target: u32, words: usize) -> bool {
    let Some(bytes) = words.checked_mul(4) else {
        return false;
    };
    if source & 3 != 0
        || target & 3 != 0
        || !readable(source, bytes)
        || !writable(target, bytes)
    {
        return false;
    }
    // SAFETY: spans checked above; the two blocks never overlap (the target is a fresh
    // allocation), and the source object's lifetime is held by the native caller.
    unsafe { core::ptr::copy_nonoverlapping(source as *const u32, target as *mut u32, words) };
    true
}

pub fn zero_words(target: u32, words: usize) -> bool {
    let Some(bytes) = words.checked_mul(4) else {
        return false;
    };
    if target & 3 != 0 || !writable(target, bytes) {
        return false;
    }
    // SAFETY: span checked writable above; the block is plugin-owned.
    unsafe { core::ptr::write_bytes(target as *mut u8, 0, bytes) };
    true
}

pub fn call0(function: u32) -> Option<u32> {
    if !executable(function, 4) {
        return None;
    }
    let f: unsafe extern "C" fn() -> u32 = unsafe { core::mem::transmute(function as usize) };
    Some(unsafe { f() })
}

pub fn call1(function: u32, a: u32) -> Option<u32> {
    if !executable(function, 4) {
        return None;
    }
    let f: unsafe extern "C" fn(u32) -> u32 = unsafe { core::mem::transmute(function as usize) };
    Some(unsafe { f(a) })
}

pub fn call2(function: u32, a: u32, b: u32) -> Option<u32> {
    if !executable(function, 4) {
        return None;
    }
    let f: unsafe extern "C" fn(u32, u32) -> u32 = unsafe { core::mem::transmute(function as usize) };
    Some(unsafe { f(a, b) })
}

pub fn call3(function: u32, a: u32, b: u32, c: u32) -> Option<u32> {
    if !executable(function, 4) {
        return None;
    }
    let f: unsafe extern "C" fn(u32, u32, u32) -> u32 =
        unsafe { core::mem::transmute(function as usize) };
    Some(unsafe { f(a, b, c) })
}
