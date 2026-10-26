//! Keeps Rocksmith visible in "fullscreen window" mode when another window
//! (e.g. on a second monitor) gets the focus. The game minimises itself when
//! it is told it lost the focus, so those notifications are kept from it.
//!
//! Settings come from `GuitarStar.ini` next to this DLL, written by the app:
//!
//! ```ini
//! [Window]
//! KeepVisible=1   ; suppress focus-loss notifications
//! Log=1           ; write GuitarStar-window.log for diagnosis
//! ```
//!
//! Exclusive fullscreen (Rocksmith.ini `Fullscreen=2`) is left alone: there the
//! game has to give the monitor back to Windows when it loses the focus.

use core::ffi::c_void;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

type Hwnd = *mut c_void;
type Handle = *mut c_void;
type Bool = i32;
type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

const GWL_WNDPROC: i32 = -4;
const WM_SIZE: u32 = 0x0005;
const WM_ACTIVATE: u32 = 0x0006;
const WM_SETFOCUS: u32 = 0x0007;
const WM_KILLFOCUS: u32 = 0x0008;
const WM_SHOWWINDOW: u32 = 0x0018;
const WM_ACTIVATEAPP: u32 = 0x001C;
const WM_NCACTIVATE: u32 = 0x0086;
const WM_SYSCOMMAND: u32 = 0x0112;
const WA_INACTIVE: usize = 0;
const SC_MINIMIZE: usize = 0xF020;
const MAX_LOG_LINES: u32 = 5000;

#[link(name = "kernel32", kind = "raw-dylib", import_name_type = "undecorated")]
unsafe extern "system" {
    fn CreateThread(
        attributes: *mut c_void,
        stack: usize,
        start: unsafe extern "system" fn(*mut c_void) -> u32,
        param: *mut c_void,
        flags: u32,
        thread_id: *mut u32,
    ) -> Handle;
    fn CloseHandle(handle: Handle) -> Bool;
    fn Sleep(ms: u32);
    fn GetCurrentProcessId() -> u32;
    fn GetModuleFileNameW(module: *mut c_void, buffer: *mut u16, size: u32) -> u32;
    fn GetPrivateProfileIntW(section: *const u16, key: *const u16, default: i32, file: *const u16) -> u32;
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *mut c_void,
        disposition: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn WriteFile(file: Handle, buffer: *const u8, len: u32, written: *mut u32, overlapped: *mut c_void) -> Bool;
    fn GetTickCount() -> u32;
}

#[link(name = "user32", kind = "raw-dylib", import_name_type = "undecorated")]
unsafe extern "system" {
    fn EnumWindows(callback: unsafe extern "system" fn(Hwnd, isize) -> Bool, param: isize) -> Bool;
    fn GetWindowThreadProcessId(hwnd: Hwnd, pid: *mut u32) -> u32;
    fn IsWindow(hwnd: Hwnd) -> Bool;
    fn IsWindowVisible(hwnd: Hwnd) -> Bool;
    fn IsWindowUnicode(hwnd: Hwnd) -> Bool;
    fn GetWindowTextW(hwnd: Hwnd, buffer: *mut u16, max: i32) -> i32;
    fn GetWindowLongW(hwnd: Hwnd, index: i32) -> isize;
    fn SetWindowLongW(hwnd: Hwnd, index: i32, value: isize) -> isize;
    fn SetWindowLongA(hwnd: Hwnd, index: i32, value: isize) -> isize;
    fn CallWindowProcW(prev: isize, hwnd: Hwnd, msg: u32, w: usize, l: isize) -> isize;
    fn CallWindowProcA(prev: isize, hwnd: Hwnd, msg: u32, w: usize, l: isize) -> isize;
    fn DefWindowProcW(hwnd: Hwnd, msg: u32, w: usize, l: isize) -> isize;
    fn DefWindowProcA(hwnd: Hwnd, msg: u32, w: usize, l: isize) -> isize;
}

static KEEP_VISIBLE: AtomicBool = AtomicBool::new(false);
static PREV_PROC: AtomicIsize = AtomicIsize::new(0);
static HOOKED: AtomicIsize = AtomicIsize::new(0);
static UNICODE: AtomicBool = AtomicBool::new(true);
static FOUND: AtomicIsize = AtomicIsize::new(0);
static LOG: AtomicIsize = AtomicIsize::new(0);
static LOG_LINES: AtomicU32 = AtomicU32::new(0);

/// Path of `name` in the folder of this DLL, NUL-terminated.
unsafe fn sibling(module: *mut c_void, name: &[u16], out: &mut [u16; 600]) -> bool {
    let len = unsafe { GetModuleFileNameW(module, out.as_mut_ptr(), 520) } as usize;
    if len == 0 || len >= 520 {
        return false;
    }
    let Some(slash) = out[..len].iter().rposition(|&c| c == b'\\' as u16) else {
        return false;
    };
    let end = slash + 1 + name.len();
    if end > out.len() {
        return false;
    }
    out[slash + 1..end].copy_from_slice(name);
    true
}

fn ini_int(section: &[u16], key: &[u16], file: &[u16; 600]) -> u32 {
    unsafe { GetPrivateProfileIntW(section.as_ptr(), key.as_ptr(), 0, file.as_ptr()) }
}

/// Called from DllMain: reads the settings and starts watching for the game window.
pub unsafe fn start(module: *mut c_void) {
    let mut config = [0u16; 600];
    let mut game_ini = [0u16; 600];
    unsafe {
        if !sibling(module, crate::wide!("GuitarStar.ini"), &mut config) {
            return;
        }
        let keep = ini_int(crate::wide!("Window"), crate::wide!("KeepVisible"), &config) != 0;
        let log = ini_int(crate::wide!("Window"), crate::wide!("Log"), &config) != 0;
        let exclusive = sibling(module, crate::wide!("Rocksmith.ini"), &mut game_ini)
            && ini_int(crate::wide!("Renderer.Win32"), crate::wide!("Fullscreen"), &game_ini) == 2;
        if !keep && !log {
            return;
        }
        KEEP_VISIBLE.store(keep && !exclusive, Ordering::Relaxed);
        if log {
            let mut path = [0u16; 600];
            if sibling(module, crate::wide!("GuitarStar-window.log"), &mut path) {
                // GENERIC_WRITE, FILE_SHARE_READ, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL
                let h = CreateFileW(path.as_ptr(), 0x4000_0000, 1, null_mut(), 2, 0x80, null_mut());
                if h as isize != -1 {
                    LOG.store(h as isize, Ordering::Relaxed);
                }
            }
            log_line(b"start keep_visible=", keep as usize, b" exclusive=", exclusive as usize);
        }
        let thread = CreateThread(null_mut(), 0, watch, null_mut(), 0, null_mut());
        if !thread.is_null() {
            CloseHandle(thread);
        }
    }
}

unsafe extern "system" fn find_window(hwnd: Hwnd, _: isize) -> Bool {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != GetCurrentProcessId() || IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut title = [0u16; 32];
        let n = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) as usize;
        let prefix = crate::wide!("Rocksmith");
        let prefix = &prefix[..prefix.len() - 1];
        if n >= prefix.len() && title[..prefix.len()] == *prefix {
            FOUND.store(hwnd as isize, Ordering::Relaxed);
            return 0;
        }
        1
    }
}

/// Finds the game window and installs the hook; repeats if the window is recreated.
unsafe extern "system" fn watch(_: *mut c_void) -> u32 {
    loop {
        unsafe {
            let hooked = HOOKED.load(Ordering::Relaxed) as Hwnd;
            if hooked.is_null() || IsWindow(hooked) == 0 {
                FOUND.store(0, Ordering::Relaxed);
                EnumWindows(find_window, 0);
                let hwnd = FOUND.load(Ordering::Relaxed) as Hwnd;
                if !hwnd.is_null() {
                    let unicode = IsWindowUnicode(hwnd) != 0;
                    UNICODE.store(unicode, Ordering::Relaxed);
                    // Publish the previous procedure before the hook can run.
                    PREV_PROC.store(GetWindowLongW(hwnd, GWL_WNDPROC), Ordering::SeqCst);
                    let proc = hook as WndProc as usize as isize;
                    let prev = if unicode {
                        SetWindowLongW(hwnd, GWL_WNDPROC, proc)
                    } else {
                        SetWindowLongA(hwnd, GWL_WNDPROC, proc)
                    };
                    PREV_PROC.store(prev, Ordering::SeqCst);
                    HOOKED.store(hwnd as isize, Ordering::Relaxed);
                    log_line(b"hooked window=", hwnd as usize, b" unicode=", unicode as usize);
                }
            }
            Sleep(if HOOKED.load(Ordering::Relaxed) == 0 { 200 } else { 1000 });
        }
    }
}

unsafe extern "system" fn hook(hwnd: Hwnd, msg: u32, w: usize, l: isize) -> isize {
    let name: &[u8] = match msg {
        WM_SIZE => b"WM_SIZE",
        WM_ACTIVATE => b"WM_ACTIVATE",
        WM_SETFOCUS => b"WM_SETFOCUS",
        WM_KILLFOCUS => b"WM_KILLFOCUS",
        WM_SHOWWINDOW => b"WM_SHOWWINDOW",
        WM_ACTIVATEAPP => b"WM_ACTIVATEAPP",
        WM_NCACTIVATE => b"WM_NCACTIVATE",
        WM_SYSCOMMAND => b"WM_SYSCOMMAND",
        _ => b"",
    };
    let suppress = KEEP_VISIBLE.load(Ordering::Relaxed)
        && match msg {
            WM_ACTIVATEAPP => w == 0,
            WM_ACTIVATE => (w & 0xFFFF) == WA_INACTIVE,
            WM_KILLFOCUS => true,
            WM_SYSCOMMAND => (w & 0xFFF0) == SC_MINIMIZE,
            _ => false,
        };
    if !name.is_empty() {
        log_line(name, w, if suppress { b" suppressed" } else { b" passed" }, l as usize);
    }
    let unicode = UNICODE.load(Ordering::Relaxed);
    unsafe {
        if suppress {
            // The game never hears about the focus loss; Windows still gets its default handling,
            // except for minimising, which is dropped entirely.
            return match msg {
                WM_ACTIVATEAPP | WM_SYSCOMMAND => 0,
                _ if unicode => DefWindowProcW(hwnd, msg, w, l),
                _ => DefWindowProcA(hwnd, msg, w, l),
            };
        }
        let prev = PREV_PROC.load(Ordering::SeqCst);
        if unicode {
            CallWindowProcW(prev, hwnd, msg, w, l)
        } else {
            CallWindowProcA(prev, hwnd, msg, w, l)
        }
    }
}

/// Writes `t=<ms> <a><hex b><c><hex d>` to the log, if logging is on.
fn log_line(a: &[u8], b: usize, c: &[u8], d: usize) {
    let h = LOG.load(Ordering::Relaxed);
    if h == 0 || LOG_LINES.fetch_add(1, Ordering::Relaxed) >= MAX_LOG_LINES {
        return;
    }
    let mut buf = [0u8; 160];
    let mut n = 0;
    let mut put = |bytes: &[u8]| {
        for &x in bytes {
            if n < buf.len() {
                buf[n] = x;
                n += 1;
            }
        }
    };
    put(b"t=");
    put(&hex(unsafe { GetTickCount() } as usize));
    put(b" ");
    put(a);
    put(b" w=");
    put(&hex(b));
    put(c);
    put(b" l=");
    put(&hex(d));
    put(b"\r\n");
    let mut written = 0u32;
    unsafe { WriteFile(h as Handle, buf.as_ptr(), n as u32, &mut written, null_mut()) };
}

fn hex(v: usize) -> [u8; 10] {
    let mut out = *b"0x00000000";
    for i in 0..8 {
        let digit = (v >> ((7 - i) * 4)) & 0xF;
        out[2 + i] = b"0123456789abcdef"[digit];
    }
    out
}
