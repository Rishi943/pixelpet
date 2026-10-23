//! Windows: cursor, work area, idle time, fullscreen detection, and the
//! launch-on-startup value in the registry `Run` key.

use super::{Platform, Rect};
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows_sys::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows_sys::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowRect, SetCursorPos,
};

pub struct Win;

/// A named mutex held for the life of the process (`Local\<name>`, so one
/// per signed-in user).
pub struct Mutex(windows_sys::Win32::Foundation::HANDLE);

impl Mutex {
    pub fn claim(name: &str) -> Option<Mutex> {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexW;
        let n = wide(&format!("Local\\{name}"));
        unsafe {
            let h = CreateMutexW(std::ptr::null(), 1, n.as_ptr());
            if h == 0 {
                return None;
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(h);
                return None;
            }
            Some(Mutex(h))
        }
    }
}

impl Drop for Mutex {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Threading::ReleaseMutex(self.0);
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn rect_of(r: RECT) -> Rect {
    Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
}

impl Platform for Win {
    fn cursor_pos(&self) -> Option<(i32, i32)> {
        let mut p = POINT { x: 0, y: 0 };
        (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
    }

    fn warp_cursor(&self, x: i32, y: i32) -> bool {
        unsafe { SetCursorPos(x, y) != 0 }
    }

    fn work_area(&self) -> Rect {
        unsafe {
            let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(mon, &mut mi) == 0 {
                return Rect { x: 0, y: 0, w: 1920, h: 1040 };
            }
            // an auto-hide taskbar stays inside rcWork: ride on it when shown
            let tray = wide("Shell_TrayWnd");
            let bar = FindWindowW(tray.as_ptr(), std::ptr::null());
            let mut br: RECT = std::mem::zeroed();
            let bar = (bar != 0 && GetWindowRect(bar, &mut br) != 0).then(|| rect_of(br));
            super::ground_with_taskbar(rect_of(mi.rcWork), bar)
        }
    }

    fn idle_seconds(&self) -> Option<f64> {
        let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        if unsafe { GetLastInputInfo(&mut li) } == 0 {
            return None;
        }
        let now = unsafe { GetTickCount() };
        Some(now.wrapping_sub(li.dwTime) as f64 / 1000.0)
    }

    fn fullscreen_app(&self) -> Option<bool> {
        unsafe {
            let mut state = 0;
            if SHQueryUserNotificationState(&mut state) == 0
                && (state == QUNS_BUSY || state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE)
            {
                return Some(true);
            }
            // Borderless "fullscreen": the foreground window covers its monitor.
            let fg = GetForegroundWindow();
            if fg == 0 {
                return Some(false);
            }
            let mut buf = [0u16; 64];
            let n = GetClassNameW(fg, buf.as_mut_ptr(), buf.len() as i32);
            let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
            if class == "Progman" || class == "WorkerW" {
                return Some(false);
            }
            let mut wr: RECT = std::mem::zeroed();
            if GetWindowRect(fg, &mut wr) == 0 {
                return Some(false);
            }
            let mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(mon, &mut mi) == 0 {
                return Some(false);
            }
            Some(rect_of(wr) == rect_of(mi.rcMonitor))
        }
    }

    fn autostart(&self) -> Option<PathBuf> {
        get_autostart_named("PixelPet")
    }

    fn set_autostart(&self, target: Option<&Path>) -> std::io::Result<()> {
        set_autostart_named("PixelPet", target)
    }

    fn esc_down(&self) -> bool {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE};
        (unsafe { GetAsyncKeyState(VK_ESCAPE as i32) } as u16 & 0x8000) != 0
    }

    fn pick_photos(&self) -> Option<Vec<PathBuf>> {
        unsafe { open_dialog() }
    }
}

// COM by hand: windows-sys has the CLSID and the option flags but no vtables
// or IIDs. Every interface is a pointer to a pointer to a table of fns, the
// first argument of each being the interface itself. Slots counted from
// IUnknown (QueryInterface 0, AddRef 1, Release 2) down the inheritance
// chain.
type Com = *mut *const usize;
type Hr = i32;

const IID_IFILEOPENDIALOG: windows_sys::core::GUID = windows_sys::core::GUID::from_u128(0xd57c7288_d4ad_4768_be02_9d969532d960);
const HR_CANCELLED: Hr = 0x800704C7u32 as Hr; // HRESULT_FROM_WIN32(ERROR_CANCELLED)
const RPC_E_CHANGED_MODE: Hr = 0x80010106u32 as Hr;

#[repr(C)]
struct FilterSpec {
    name: *const u16,
    spec: *const u16,
}

/// Call slot `n` of the interface `o` as `fn(o, args...) -> Hr`.
macro_rules! com {
    ($o:expr, $n:expr, fn($($t:ty),*), $($a:expr),*) => {{
        let f: unsafe extern "system" fn(Com $(, $t)*) -> Hr = std::mem::transmute(*(*$o).add($n));
        f($o $(, $a)*)
    }};
}

unsafe fn release(o: Com) {
    com!(o, 2, fn(),);
}

/// The Open dialog for several JPEG/PNG files, with no owner window (the
/// maker is topmost, and the dialog opens in front of it). Blocks until it
/// closes. None if the dialog could not be made; Some(empty) on Cancel.
unsafe fn open_dialog() -> Option<Vec<PathBuf>> {
    use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::UI::Shell::{FileOpenDialog, FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, SIGDN_FILESYSPATH};
    // S_OK and S_FALSE (already initialised the same way) both need a
    // matching CoUninitialize; a thread already in another mode needs none.
    let init = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
    if init < 0 && init != RPC_E_CHANGED_MODE {
        return None;
    }
    let result = (|| {
        let mut dlg: *mut std::ffi::c_void = std::ptr::null_mut();
        if CoCreateInstance(&FileOpenDialog, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_IFILEOPENDIALOG, &mut dlg) < 0 || dlg.is_null() {
            return None;
        }
        let dlg = dlg as Com;
        let picked = (|| {
            // IFileDialog: SetFileTypes 4, GetOptions 10, SetOptions 9, SetTitle 17
            let (name, spec) = (wide("Photos (*.jpg;*.jpeg;*.png;*.webp)"), wide("*.jpg;*.jpeg;*.png;*.webp"));
            let filter = FilterSpec { name: name.as_ptr(), spec: spec.as_ptr() };
            com!(dlg, 4, fn(u32, *const FilterSpec), 1, &filter as *const FilterSpec);
            let title = wide("Choose photos");
            com!(dlg, 17, fn(*const u16), title.as_ptr());
            let mut opts = 0u32;
            com!(dlg, 10, fn(*mut u32), &mut opts as *mut u32);
            com!(dlg, 9, fn(u32), opts | FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST);
            // IModalWindow::Show 3
            let shown = com!(dlg, 3, fn(isize), 0);
            if shown == HR_CANCELLED {
                return Some(Vec::new());
            }
            if shown < 0 {
                return None;
            }
            // IFileOpenDialog::GetResults 27 -> IShellItemArray (GetCount 7, GetItemAt 8)
            let mut items: Com = std::ptr::null_mut();
            if com!(dlg, 27, fn(*mut Com), &mut items as *mut Com) < 0 || items.is_null() {
                return None;
            }
            let (mut count, mut paths) = (0u32, Vec::new());
            com!(items, 7, fn(*mut u32), &mut count as *mut u32);
            for i in 0..count {
                let mut item: Com = std::ptr::null_mut();
                if com!(items, 8, fn(u32, *mut Com), i, &mut item as *mut Com) < 0 || item.is_null() {
                    continue;
                }
                // IShellItem::GetDisplayName 5, a CoTaskMem string
                let mut s: *mut u16 = std::ptr::null_mut();
                if com!(item, 5, fn(i32, *mut *mut u16), SIGDN_FILESYSPATH, &mut s as *mut *mut u16) >= 0 && !s.is_null() {
                    let len = (0..).take_while(|&n| *s.add(n) != 0).count();
                    paths.push(PathBuf::from(<std::ffi::OsString as std::os::windows::ffi::OsStringExt>::from_wide(std::slice::from_raw_parts(s, len))));
                    CoTaskMemFree(s as *const _);
                }
                release(item);
            }
            release(items);
            Some(paths)
        })();
        release(dlg);
        picked
    })();
    if init >= 0 {
        CoUninitialize();
    }
    result
}

fn os_err(code: u32) -> std::io::Error {
    std::io::Error::from_raw_os_error(code as i32)
}

/// Read `HKCU\...\Run\<name>`; the stored data is the quoted exe path.
fn get_autostart_named(name: &str) -> Option<PathBuf> {
    let (key, val) = (wide(RUN_KEY), wide(name));
    let mut buf = vec![0u16; 1024];
    let mut bytes = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            val.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if r as u32 != ERROR_SUCCESS {
        return None;
    }
    let n = (bytes as usize / 2).min(buf.len());
    let s = String::from_utf16_lossy(&buf[..n]);
    let s = s.trim_end_matches('\0').trim();
    let s = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(s);
    Some(PathBuf::from(s))
}

/// Write (or with None, remove) `HKCU\...\Run\<name>`.
fn set_autostart_named(name: &str, target: Option<&Path>) -> std::io::Result<()> {
    let (key, val) = (wide(RUN_KEY), wide(name));
    let r = match target {
        Some(p) => {
            let data = wide(&format!("\"{}\"", p.display()));
            unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    key.as_ptr(),
                    val.as_ptr(),
                    REG_SZ,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                )
            }
        }
        None => {
            let r = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), val.as_ptr()) };
            if r as u32 == ERROR_FILE_NOT_FOUND {
                return Ok(());
            }
            r
        }
    };
    if r as u32 == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(os_err(r as u32))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    #[test]
    fn work_area_is_inside_primary_monitor() {
        let r = Win.work_area();
        let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        assert!(r.w > 0 && r.h > 0, "{r:?}");
        assert!(r.x >= 0 && r.y >= 0 && r.x + r.w <= sw && r.bottom() <= sh, "{r:?} vs {sw}x{sh}");
    }

    #[test]
    fn idle_is_small_while_testing() {
        let s = Win.idle_seconds().expect("idle known on Windows");
        assert!((0.0..3600.0).contains(&s), "{s}");
    }

    /// Known either way on Windows. (It is `Some(true)` whenever a fullscreen
    /// video or game is in front, so the value itself depends on the desktop;
    /// `examples/probe_fullscreen.rs` shows what it sees.)
    #[test]
    fn fullscreen_is_known() {
        assert!(Win.fullscreen_app().is_some());
    }

    #[test]
    fn only_one_instance_holds_the_lock() {
        let name = format!("PixelPet-test-{}", std::process::id());
        let first = crate::platform::claim_single_instance(&name);
        assert!(first.is_some(), "the first claim gets the lock");
        assert!(crate::platform::claim_single_instance(&name).is_none(), "a second one is refused");
        drop(first);
        assert!(crate::platform::claim_single_instance(&name).is_some(), "free again once the first is gone");
    }

    #[test]
    fn cursor_is_known() {
        assert!(Win.cursor_pos().is_some());
    }

    /// Writes and then removes the registry value
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\PixelPet-test`.
    #[test]
    #[ignore = "touches the real registry; run on request"]
    fn autostart_round_trip() {
        let target = Path::new("C:\\tmp\\a b\\PixelPet.exe");
        set_autostart_named("PixelPet-test", Some(target)).unwrap();
        assert_eq!(get_autostart_named("PixelPet-test").as_deref(), Some(target));
        set_autostart_named("PixelPet-test", None).unwrap();
        assert_eq!(get_autostart_named("PixelPet-test"), None);
    }
}
