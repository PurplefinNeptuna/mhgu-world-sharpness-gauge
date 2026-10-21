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
