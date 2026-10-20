//! A real pet window must show all of itself and answer "client area" to
//! the hit-test question for points on it, after any resize or move. (Seen
//! on the desktop: a clipping region stuck at an old size cut the pet off at
//! L and XL, and the default handler answered HTNOWHERE.)
//! No test harness: winit wants the main thread.

#[cfg(windows)]
fn main() {
    use pixelpet::window::{PetWindow, Rgba};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, HTCLIENT, WM_NCHITTEST};
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::WindowId;

    struct T {
        failures: Vec<String>,
        checked: usize,
        done: bool,
    }

    impl ApplicationHandler for T {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.done {
                return;
            }
            self.done = true;
            let mut win = PetWindow::new(el, false).expect("window");
            let (sw, sh) = unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
                (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
            };
            // the pet's life: grow and shrink (lift, z's), move, hang past
            // the bottom edge as he does on the ground line
            let steps = [(120, 120, 400, 300), (207, 246, 400, sh - 243), (207, 300, 900, 200), (150, 180, sw / 2, sh - 177), (207, 246, sw / 3, sh - 243)];
            for (k, &(w, h, x0, y0)) in steps.iter().enumerate() {
                if k == 1 {
                    // something on the desktop sets a clipping region the
                    // size of the window as it is (seen live: 207x246 at M,
                    // still there after growing to L); set one the same way
                    let (pw, ph) = (steps[0].0 as i32, steps[0].1 as i32);
                    unsafe {
                        use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, SetWindowRgn};
                        SetWindowRgn(win.hwnd() as _, CreateRectRgn(0, 0, pw, ph), 0);
                    }
                }
                let mut img = Rgba::new(w, h);
                for p in img.px.chunks_mut(4) {
                    p.copy_from_slice(&[200, 120, 40, 255]);
                }
                win.present(&img, x0, y0, 255).expect("present");
                // a clipping region left over from a smaller size cuts the
                // pet off (only the ear tips showed at L and XL)
                let mut rb: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
                let kind = unsafe { windows_sys::Win32::Graphics::Gdi::GetWindowRgnBox(win.hwnd() as _, &mut rb) };
                if kind != 0 && (rb.right - rb.left, rb.bottom - rb.top) != (w as i32, h as i32) {
                    self.failures.push(format!("step {k}: region {}x{} on a {w}x{h} window", rb.right - rb.left, rb.bottom - rb.top));
                }
                for dy in (0..h as i32).step_by(10) {
                    for dx in (0..w as i32).step_by(10) {
                        let (x, y) = (x0 + dx, y0 + dy);
                        if y >= sh {
                            continue;
                        }
                        let lp = ((y as u32 & 0xFFFF) << 16 | (x as u32 & 0xFFFF)) as isize;
                        let r = unsafe { SendMessageW(win.hwnd() as _, WM_NCHITTEST, 0, lp) };
                        self.checked += 1;
                        if r != HTCLIENT as isize {
                            self.failures.push(format!("step {k} ({x},{y}) -> {r}"));
                        }
                    }
                }
            }
            el.exit();
        }
        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    }

    let el = EventLoop::new().expect("event loop");
    let mut t = T { failures: vec![], checked: 0, done: false };
    el.run_app(&mut t).expect("run");
    if t.failures.is_empty() {
        println!("test window_hit ... ok ({} points answer HTCLIENT)", t.checked);
    } else {
        println!("test window_hit ... FAILED: {} of {} points: {:?}", t.failures.len(), t.checked, &t.failures[..t.failures.len().min(8)]);
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {}
