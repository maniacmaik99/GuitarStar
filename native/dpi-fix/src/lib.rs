//! A `dinput8.dll` placed next to Rocksmith2014.exe.
//!
//! Rocksmith is not DPI aware, and it ignores the HIGHDPIAWARE compatibility
//! flag, so with display scaling above 100 % Windows stretches its window past
//! the monitor, in fullscreen modes too. The game imports dinput8.dll, so
//! Windows loads this DLL before the game creates its window. On load it
//! marks the process per-monitor DPI aware; every DirectInput call is passed
//! on unchanged to the real dinput8.dll from the system folder.
//!
//! Optionally it also keeps the game from minimising on focus loss, see `window`.

#![no_std]
#![no_main]

mod window;

use core::ffi::c_void;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

type Hmodule = *mut c_void;
type Bool = i32;
type Hresult = i32;

const DLL_PROCESS_ATTACH: u32 = 1;
const E_FAIL: Hresult = 0x8000_4005_u32 as i32;
const S_FALSE: Hresult = 1;
/// DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
const PER_MONITOR_AWARE_V2: isize = -4;

#[link(name = "kernel32", kind = "raw-dylib", import_name_type = "undecorated")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> Hmodule;
    fn GetModuleHandleW(name: *const u16) -> Hmodule;
    fn GetProcAddress(module: Hmodule, name: *const u8) -> *mut c_void;
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
    fn DisableThreadLibraryCalls(module: Hmodule) -> Bool;
}

/// Builds a NUL-terminated UTF-16 string from ASCII at compile time.
#[macro_export]
macro_rules! wide {
    ($s:literal) => {{
        const S: &[u8] = $s.as_bytes();
        const W: [u16; S.len() + 1] = {
            let mut w = [0u16; S.len() + 1];
            let mut i = 0;
            while i < S.len() {
                w[i] = S[i] as u16;
                i += 1;
            }
            w
        };
        &W
    }};
}

unsafe fn make_dpi_aware() {
    unsafe {
        let mut user32 = GetModuleHandleW(wide!("user32.dll").as_ptr());
        if user32.is_null() {
            user32 = LoadLibraryW(wide!("user32.dll").as_ptr());
        }
        if user32.is_null() {
            return;
        }
        // Windows 10 1703+; older systems only know the system-wide variant.
        let ctx = GetProcAddress(user32, c"SetProcessDpiAwarenessContext".as_ptr().cast());
        if !ctx.is_null() {
            let f: unsafe extern "system" fn(isize) -> Bool = core::mem::transmute(ctx);
            if f(PER_MONITOR_AWARE_V2) != 0 {
                return;
            }
        }
        let legacy = GetProcAddress(user32, c"SetProcessDPIAware".as_ptr().cast());
        if !legacy.is_null() {
            let f: unsafe extern "system" fn() -> Bool = core::mem::transmute(legacy);
            f();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(module: Hmodule, reason: u32, _reserved: *mut c_void) -> Bool {
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            DisableThreadLibraryCalls(module);
            make_dpi_aware();
            window::start(module);
        }
    }
    1
}

static REAL: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

/// The system's dinput8.dll, loaded on first use rather than in DllMain.
/// For a 32-bit process Windows redirects System32 to SysWOW64 by itself.
unsafe fn real_proc(name: &core::ffi::CStr) -> *mut c_void {
    unsafe {
        let mut module = REAL.load(Ordering::Acquire);
        if module.is_null() {
            let mut path = [0u16; 300];
            let len = GetSystemDirectoryW(path.as_mut_ptr(), 260) as usize;
            if len == 0 || len > 260 {
                return null_mut();
            }
            let file = wide!("\\dinput8.dll");
            path[len..len + file.len()].copy_from_slice(file);
            module = LoadLibraryW(path.as_ptr());
            if module.is_null() {
                return null_mut();
            }
            REAL.store(module, Ordering::Release);
        }
        GetProcAddress(module, name.as_ptr().cast())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DirectInput8Create(
    instance: Hmodule,
    version: u32,
    riid: *const c_void,
    out: *mut *mut c_void,
    outer: *mut c_void,
) -> Hresult {
    unsafe {
        let p = real_proc(c"DirectInput8Create");
        if p.is_null() {
            return E_FAIL;
        }
        let f: unsafe extern "system" fn(Hmodule, u32, *const c_void, *mut *mut c_void, *mut c_void) -> Hresult =
            core::mem::transmute(p);
        f(instance, version, riid, out, outer)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllCanUnloadNow() -> Hresult {
    unsafe {
        let p = real_proc(c"DllCanUnloadNow");
        if p.is_null() {
            return S_FALSE;
        }
        let f: unsafe extern "system" fn() -> Hresult = core::mem::transmute(p);
        f()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(clsid: *const c_void, riid: *const c_void, out: *mut *mut c_void) -> Hresult {
    unsafe {
        let p = real_proc(c"DllGetClassObject");
        if p.is_null() {
            return E_FAIL;
        }
        let f: unsafe extern "system" fn(*const c_void, *const c_void, *mut *mut c_void) -> Hresult =
            core::mem::transmute(p);
        f(clsid, riid, out)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllRegisterServer() -> Hresult {
    E_FAIL
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllUnregisterServer() -> Hresult {
    E_FAIL
}

// Without the C runtime the compiler's calls for array copies need these.
// Volatile writes keep LLVM from turning the loops back into memset/memcpy calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dest: *mut c_void, c: i32, n: usize) -> *mut c_void {
    let d = dest.cast::<u8>();
    for i in 0..n {
        unsafe { core::ptr::write_volatile(d.add(i), c as u8) };
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let (d, s) = (dest.cast::<u8>(), src.cast::<u8>());
    for i in 0..n {
        unsafe { core::ptr::write_volatile(d.add(i), *s.add(i)) };
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    let (a, b) = (a.cast::<u8>(), b.cast::<u8>());
    for i in 0..n {
        let (x, y) = unsafe { (core::ptr::read_volatile(a.add(i)), core::ptr::read_volatile(b.add(i))) };
        if x != y {
            return x as i32 - y as i32;
        }
    }
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
