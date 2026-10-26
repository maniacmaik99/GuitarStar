//! Bink videos (logos, intro, lesson videos). For now this records which
//! videos the game opens, so the intro videos can be told apart from the
//! lesson videos before anything gets skipped.
//!
//! The hooks are installed by rewriting bink2w32.dll's export table in
//! DllMain: the game resolves its imports only after its startup stub runs,
//! so it receives our functions, which then call the original ones. Neither
//! the game nor bink2w32.dll is changed on disk.
//!
//! `GuitarStar.ini`:
//! ```ini
//! [Video]
//! Log=1   ; record BinkOpen/BinkClose calls in GuitarStar-window.log
//! ```

use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::window::{ini_int, log_line, open_log, sibling};

type Bool = i32;
type Hbink = *mut c_void;

const PAGE_READWRITE: u32 = 0x04;
/// BINKFILEHANDLE: `name` is a Win32 file handle instead of a path.
const BINK_FILE_HANDLE: u32 = 0x0080_0000;

#[link(name = "kernel32", kind = "raw-dylib", import_name_type = "undecorated")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn VirtualProtect(address: *mut c_void, size: usize, protect: u32, old: *mut u32) -> Bool;
    fn GetFinalPathNameByHandleW(file: *mut c_void, buffer: *mut u16, len: u32, flags: u32) -> u32;
}

static ORIG_OPEN: AtomicUsize = AtomicUsize::new(0);
static ORIG_OPEN_OPTIONS: AtomicUsize = AtomicUsize::new(0);
static ORIG_CLOSE: AtomicUsize = AtomicUsize::new(0);
static OPEN_COUNT: AtomicU32 = AtomicU32::new(0);

pub unsafe fn start(module: *mut c_void) {
    let mut config = [0u16; 600];
    unsafe {
        if !sibling(module, crate::wide!("GuitarStar.ini"), &mut config)
            || ini_int(crate::wide!("Video"), crate::wide!("Log"), &config) == 0
        {
            return;
        }
        open_log(module);
        let bink = GetModuleHandleW(crate::wide!("bink2w32.dll").as_ptr());
        log_line(b"video start bink=", bink as usize, b"", 0);
        if bink.is_null() {
            return;
        }
        let base = bink as usize;
        ORIG_OPEN.store(patch_export(base, b"_BinkOpen@8", bink_open as BinkOpen as usize), Ordering::SeqCst);
        ORIG_OPEN_OPTIONS.store(
            patch_export(base, b"_BinkOpenWithOptions@12", bink_open_options as BinkOpenOptions as usize),
            Ordering::SeqCst,
        );
        ORIG_CLOSE.store(patch_export(base, b"_BinkClose@4", bink_close as BinkClose as usize), Ordering::SeqCst);
        log_line(b"video hooks open=", ORIG_OPEN.load(Ordering::SeqCst), b" close=", ORIG_CLOSE.load(Ordering::SeqCst));
    }
}

unsafe fn read_u32(addr: usize) -> u32 {
    unsafe { core::ptr::read_unaligned(addr as *const u32) }
}

unsafe fn read_u16(addr: usize) -> u16 {
    unsafe { core::ptr::read_unaligned(addr as *const u16) }
}

/// Points the export `name` of the module at `base` to `hook` and returns the
/// original function address, or 0 if the export was not found.
unsafe fn patch_export(base: usize, name: &[u8], hook: usize) -> usize {
    unsafe {
        let nt = base + read_u32(base + 0x3C) as usize;
        // PE32: the optional header starts at nt+24, data directory 0 at +96.
        let export_rva = read_u32(nt + 24 + 96) as usize;
        if export_rva == 0 {
            return 0;
        }
        let dir = base + export_rva;
        let count = read_u32(dir + 24) as usize;
        let functions = base + read_u32(dir + 28) as usize;
        let names = base + read_u32(dir + 32) as usize;
        let ordinals = base + read_u32(dir + 36) as usize;
        for i in 0..count {
            let s = base + read_u32(names + i * 4) as usize;
            if !c_str_eq(s, name) {
                continue;
            }
            let entry = functions + read_u16(ordinals + i * 2) as usize * 4;
            let original = base.wrapping_add(read_u32(entry) as usize);
            let mut old = 0u32;
            if VirtualProtect(entry as *mut c_void, 4, PAGE_READWRITE, &mut old) == 0 {
                return 0;
            }
            // Export entries are RVAs; on 32-bit Windows the subtraction wraps correctly.
            core::ptr::write_unaligned(entry as *mut u32, hook.wrapping_sub(base) as u32);
            VirtualProtect(entry as *mut c_void, 4, old, &mut old);
            return original;
        }
        0
    }
}

unsafe fn c_str_eq(addr: usize, name: &[u8]) -> bool {
    for (i, &b) in name.iter().enumerate() {
        if unsafe { *((addr + i) as *const u8) } != b {
            return false;
        }
    }
    unsafe { *((addr + name.len()) as *const u8) == 0 }
}

type BinkOpen = unsafe extern "system" fn(*const u8, u32) -> Hbink;
type BinkOpenOptions = unsafe extern "system" fn(*const u8, *const c_void, u32) -> Hbink;
type BinkClose = unsafe extern "system" fn(Hbink);

/// Logs the file name (or the file behind a handle) and the opened video's size.
fn log_open(kind: &[u8], name: *const u8, flags: u32, bink: Hbink) {
    let n = OPEN_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    log_line(kind, n as usize, b" flags=", flags as usize);
    let mut text = [0u8; 300];
    let mut len = 0;
    if flags & BINK_FILE_HANDLE != 0 || (name as usize) < 0x10000 {
        let mut wide = [0u16; 300];
        let got = unsafe { GetFinalPathNameByHandleW(name as *mut c_void, wide.as_mut_ptr(), 299, 0) } as usize;
        for &c in &wide[..got.min(299)] {
            text[len] = if c < 0x80 { c as u8 } else { b'?' };
            len += 1;
        }
    } else {
        while len < 299 {
            let b = unsafe { *name.add(len) };
            if b == 0 {
                break;
            }
            text[len] = b;
            len += 1;
        }
    }
    crate::window::log_text(b"  file ", &text[..len]);
    if !bink.is_null() {
        // BINK starts with Width, Height, Frames, FrameNum, FrameRate, FrameRateDiv.
        let p = bink as usize;
        unsafe {
            log_line(b"  size w=", read_u32(p) as usize, b" h=", read_u32(p + 4) as usize);
            log_line(b"  frames=", read_u32(p + 8) as usize, b" rate=", read_u32(p + 16) as usize);
            log_line(b"  ratediv=", read_u32(p + 20) as usize, b" handle=", p);
        }
    }
}

unsafe extern "system" fn bink_open(name: *const u8, flags: u32) -> Hbink {
    let orig: BinkOpen = unsafe { core::mem::transmute(ORIG_OPEN.load(Ordering::SeqCst)) };
    let bink = unsafe { orig(name, flags) };
    log_open(b"BinkOpen #", name, flags, bink);
    bink
}

unsafe extern "system" fn bink_open_options(name: *const u8, options: *const c_void, flags: u32) -> Hbink {
    let orig: BinkOpenOptions = unsafe { core::mem::transmute(ORIG_OPEN_OPTIONS.load(Ordering::SeqCst)) };
    let bink = unsafe { orig(name, options, flags) };
    log_open(b"BinkOpenWithOptions #", name, flags, bink);
    bink
}

unsafe extern "system" fn bink_close(bink: Hbink) {
    if !bink.is_null() {
        let p = bink as usize;
        unsafe { log_line(b"BinkClose handle=", p, b" at frame=", read_u32(p + 12) as usize) };
    }
    let orig: BinkClose = unsafe { core::mem::transmute(ORIG_CLOSE.load(Ordering::SeqCst)) };
    unsafe { orig(bink) }
}
