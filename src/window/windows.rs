//! Windows: a winit window switched to a layered window and fed with
//! `UpdateLayeredWindow`, so per-pixel alpha shows and clicks pass through
//! fully transparent pixels natively.

use super::Rgba;
use std::ptr;
use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
    DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, UpdateLayeredWindow, GWL_EXSTYLE, SW_HIDE,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

/// A DIB section kept between presents; recreated only on size change.
struct Dib {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    w: usize,
    h: usize,
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bmp);
            DeleteDC(self.dc);
        }
    }
}

pub struct PetWindow {
    win: Window,
    hwnd: HWND,
    dib: Option<Dib>,
    shown: bool,
}

impl PetWindow {
    pub fn new(el: &ActiveEventLoop, focusable: bool) -> Result<PetWindow, String> {
        let attrs = WindowAttributes::default()
            .with_title("PixelPet")
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_skip_taskbar(true)
            .with_active(false)
            .with_transparent(false)
            .with_visible(false);
        let win = el.create_window(attrs).map_err(|e| e.to_string())?;
        let hwnd = match win.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::Win32(h) => h.hwnd.get() as HWND,
            _ => return Err("not a Win32 window".into()),
        };
        unsafe {
            let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            // No taskbar button: tool window, and never an app window.
            ex &= !WS_EX_APPWINDOW;
            ex |= WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
            if !focusable {
                ex |= WS_EX_NOACTIVATE;
            }
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
        }
        hook(hwnd);
        Ok(PetWindow { win, hwnd, dib: None, shown: false })
    }

    pub fn id(&self) -> WindowId {
        self.win.id()
    }

    pub fn hwnd(&self) -> isize {
        self.hwnd as isize
    }

    /// The window's extended style bits, for diagnostics.
    pub fn ex_style(&self) -> u32 {
        unsafe { GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32 }
    }

    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
    }

    /// Ask for keyboard focus (the menu window, so Esc and focus-lost work).
    pub fn focus(&self) {
        self.win.focus_window();
    }

    fn dib(&mut self, w: usize, h: usize) -> Result<&mut Dib, String> {
        if self.dib.as_ref().map_or(true, |d| d.w != w || d.h != h) {
            self.dib = None;
            unsafe {
                let screen = GetDC(0);
                let dc = CreateCompatibleDC(screen);
                ReleaseDC(0, screen);
                if dc == 0 {
                    return Err("CreateCompatibleDC failed".into());
                }
                let mut bi: BITMAPINFO = std::mem::zeroed();
                bi.bmiHeader = BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    biHeight: -(h as i32), // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB as u32,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                };
                let mut bits: *mut core::ffi::c_void = ptr::null_mut();
                let bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, 0, 0);
                if bmp == 0 || bits.is_null() {
                    DeleteDC(dc);
                    return Err("CreateDIBSection failed".into());
                }
                let old = SelectObject(dc, bmp);
                self.dib = Some(Dib { dc, bmp, old, bits: bits as *mut u8, w, h });
            }
        }
        Ok(self.dib.as_mut().unwrap())
    }

    /// Show `img` with its top-left at physical `(x, y)`. `opacity` is 0–255
    /// and applies on top of the per-pixel alpha.
    pub fn present(&mut self, img: &Rgba, x: i32, y: i32, opacity: u8) -> Result<(), String> {
        if img.w == 0 || img.h == 0 || img.px.len() != img.w * img.h * 4 {
            return Err("bad image size".into());
        }
        let hwnd = self.hwnd;
        let dib = self.dib(img.w, img.h)?;
        let dst = unsafe { std::slice::from_raw_parts_mut(dib.bits, img.w * img.h * 4) };
        for (d, s) in dst.chunks_exact_mut(4).zip(img.px.chunks_exact(4)) {
            let a = s[3] as u32;
            let pm = |c: u8| ((c as u32 * a + 127) / 255) as u8;
            d[0] = pm(s[2]);
            d[1] = pm(s[1]);
            d[2] = pm(s[0]);
            d[3] = s[3];
        }
        let dc = dib.dc;
        let pt_dst = POINT { x, y };
        let size = SIZE { cx: img.w as i32, cy: img.h as i32 };
        let pt_src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: opacity,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let ok = unsafe {
            UpdateLayeredWindow(hwnd, 0, &pt_dst, &size, dc, &pt_src, 0, &blend, ULW_ALPHA)
        };
        if ok == 0 {
            return Err(format!("UpdateLayeredWindow failed: {}", std::io::Error::last_os_error()));
        }
        if !self.shown {
            self.show();
            self.shown = true;
        }
        Ok(())
    }
}

/// Window procedure hook on every pet and menu window.
///
/// Hit-test: once `UpdateLayeredWindow` has resized and moved the window,
/// the default handler answers HTNOWHERE for points inside it and Windows
/// drops the clicks (`tests/window_hit.rs`). Fully transparent pixels never
/// get this far (the layered window passes them through first), so any
/// point inside the window rect is ours: answer HTCLIENT.
fn hook(hwnd: HWND) {
    use windows_sys::Win32::UI::Shell::SetWindowSubclass;
    unsafe {
        SetWindowSubclass(hwnd, Some(hook_proc), 1, 0);
    }
}

unsafe extern "system" fn hook_proc(
    h: HWND,
    msg: u32,
    wp: windows_sys::Win32::Foundation::WPARAM,
    lp: windows_sys::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::Shell::DefSubclassProc;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, HTCLIENT, HTNOWHERE, WM_NCHITTEST};
    let mut r = DefSubclassProc(h, msg, wp, lp);
    if msg == WM_NCHITTEST && r == HTNOWHERE as isize {
        let (x, y) = ((lp & 0xFFFF) as i16 as i32, ((lp >> 16) & 0xFFFF) as i16 as i32);
        let mut wr: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
        if GetWindowRect(h, &mut wr) != 0 && x >= wr.left && x < wr.right && y >= wr.top && y < wr.bottom {
            r = HTCLIENT as isize;
        }
    }
    r
}
