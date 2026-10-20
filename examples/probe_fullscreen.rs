//! Read-only probe: prints what fullscreen detection sees right now
//! (notification state, foreground window class, its rect and its monitor).

#[cfg(windows)]
fn main() {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW};
    unsafe {
        let mut state = 0;
        let hr = SHQueryUserNotificationState(&mut state);
        println!("SHQueryUserNotificationState hr={hr} state={state} (2 busy, 3 d3d fullscreen, 4 presentation, 5 accepts notifications)");
        let fg = GetForegroundWindow();
        let mut buf = [0u16; 256];
        let n = GetClassNameW(fg, buf.as_mut_ptr(), buf.len() as i32);
        let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        let n = GetWindowTextW(fg, buf.as_mut_ptr(), buf.len() as i32);
        let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        let mut wr: RECT = std::mem::zeroed();
        GetWindowRect(fg, &mut wr);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST), &mut mi);
        let r = |r: RECT| (r.left, r.top, r.right, r.bottom);
        println!("foreground class={class:?} title={title:?}");
        println!("window rect {:?}  monitor {:?}  work {:?}", r(wr), r(mi.rcMonitor), r(mi.rcWork));
        println!("pixelpet says fullscreen = {:?}", pixelpet::platform::native().fullscreen_app());
    }
}

#[cfg(not(windows))]
fn main() {}
