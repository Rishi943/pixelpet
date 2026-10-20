//! A real pet window must answer "client area" to the hit-test question
//! for points on it, or Windows drops the clicks (seen on the desktop: the
//! default handler answered HTNOWHERE for points inside the window).
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
                let mut img = Rgba::new(w, h);
                for p in img.px.chunks_mut(4) {
                    p.copy_from_slice(&[200, 120, 40, 255]);
                }
                win.present(&img, x0, y0, 255).expect("present");
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
