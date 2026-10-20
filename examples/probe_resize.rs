//! Probe: does a PetWindow keep showing its image after the image changes
//! size? Shows a 96 px disc for 2 s, then a 192 px disc, then a 96 px one,
//! and self-tests each with WindowFromPoint. Prints one line per stage.

#[cfg(windows)]
fn main() {
    use pixelpet::window::{PetWindow, Rgba};
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, WindowFromPoint};
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::window::WindowId;

    fn disc(n: usize) -> Rgba {
        let mut img = Rgba::new(n, n);
        let c = (n as f64 - 1.0) / 2.0;
        for y in 0..n {
            for x in 0..n {
                if ((x as f64 - c).powi(2) + (y as f64 - c).powi(2)).sqrt() <= c - 2.0 {
                    img.px[(y * n + x) * 4..][..4].copy_from_slice(&[40, 120, 220, 255]);
                }
            }
        }
        img
    }

    struct P {
        win: Option<PetWindow>,
        start: Instant,
        stage: usize,
    }
    const SIZES: [usize; 3] = [96, 192, 96];

    impl ApplicationHandler for P {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.win.is_none() {
                self.win = Some(PetWindow::new(el, false).expect("window"));
            }
        }
        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            let Some(win) = self.win.as_mut() else { return };
            let t = self.start.elapsed().as_secs_f64();
            let stage = (t / 2.0) as usize;
            if stage >= SIZES.len() {
                el.exit();
                return;
            }
            let n = SIZES[stage];
            let (x, y) = (600, 300);
            let r = win.present(&disc(n), x, y, 255);
            if stage != self.stage || t < 0.05 {
                self.stage = stage;
            }
            // test half a second into each stage
            let into = t - stage as f64 * 2.0;
            if (0.5..0.55).contains(&into) {
                let me = win.hwnd();
                let hit = unsafe { WindowFromPoint(POINT { x: x + n as i32 / 2, y: y + n as i32 / 2 }) } as isize;
                let mut wr = unsafe { std::mem::zeroed() };
                unsafe { GetWindowRect(me as _, &mut wr) };
                let rect: windows_sys::Win32::Foundation::RECT = wr;
                println!(
                    "stage {stage} size {n}: present={r:?} centre-hit-ours={} window=({},{})-({},{}) = {}x{}",
                    hit == me,
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom,
                    rect.right - rect.left,
                    rect.bottom - rect.top
                );
            }
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(40)));
        }
    }

    let el = EventLoop::new().unwrap();
    let mut p = P { win: None, start: Instant::now(), stage: 0 };
    el.run_app(&mut p).unwrap();
}

#[cfg(not(windows))]
fn main() {}
