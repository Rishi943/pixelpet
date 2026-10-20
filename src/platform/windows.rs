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
