//! Spike: prove the layered pet window on Windows. Shows a red disc with a
//! dark outline, bobbing near the bottom-centre of the primary screen for
//! 10 s, and self-tests click-through with `WindowFromPoint`.

#[cfg(windows)]
fn main() {
    use pixelpet::window::{PetWindow, Rgba};
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, WindowFromPoint, SM_CXSCREEN, SM_CYSCREEN,
    };
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::window::WindowId;

    const N: usize = 96;

    fn disc() -> Rgba {
        let mut img = Rgba::new(N, N);
        let c = (N as f64 - 1.0) / 2.0;
        for y in 0..N {
            for x in 0..N {
                let d = ((x as f64 - c).powi(2) + (y as f64 - c).powi(2)).sqrt();
                let px = &mut img.px[(y * N + x) * 4..][..4];
                if d <= 39.0 {
                    px.copy_from_slice(&[220, 40, 40, 255]);
                } else if d <= 40.0 {
                    px.copy_from_slice(&[36, 18, 10, 255]);
                }
            }
        }
        img
    }

    struct Spike {
        win: Option<PetWindow>,
        img: Rgba,
        start: Instant,
        frames: u32,
        result: Option<String>,
    }

    impl ApplicationHandler for Spike {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.win.is_none() {
                match PetWindow::new(el, false) {
                    Ok(w) => self.win = Some(w),
                    Err(e) => {
                        self.result = Some(format!("SPIKE FAIL: create: {e}"));
                        el.exit();
                    }
                }
            }
        }

        fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, _ev: WindowEvent) {}

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            let Some(win) = self.win.as_mut() else { return };
            let t = self.start.elapsed().as_secs_f64();
            if t >= 10.0 {
                el.exit();
                return;
            }
            let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
            let bob = (8.0 * (t * std::f64::consts::TAU).sin()).round() as i32;
            let x = sw / 2 - N as i32 / 2;
            let y = sh - 200 + bob;
            if let Err(e) = win.present(&self.img, x, y, 255) {
                self.result = Some(format!("SPIKE FAIL: present: {e}"));
                el.exit();
                return;
            }
            self.frames += 1;
            if self.frames == 3 && self.result.is_none() {
                let me = win.hwnd();
                let at = |px: i32, py: i32| unsafe { WindowFromPoint(POINT { x: px, y: py }) } as isize;
                let centre = at(x + N as i32 / 2, y + N as i32 / 2);
                let corner = at(x + 1, y + 1);
                let mut fails = vec![];
                if centre != me {
                    fails.push("disc centre is not our window");
                }
                if corner == me {
                    fails.push("transparent corner hits our window");
                }
                println!("ex-style: {:#010x}", win.ex_style());
                self.result = Some(if fails.is_empty() {
                    "SPIKE PASS".into()
                } else {
                    format!("SPIKE FAIL: {}", fails.join(", "))
                });
                println!("{}", self.result.as_ref().unwrap());
            }
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(33)));
        }
    }

    let el = EventLoop::new().expect("event loop");
    let mut app = Spike { win: None, img: disc(), start: Instant::now(), frames: 0, result: None };
    el.run_app(&mut app).expect("run");
    match &app.result {
        Some(r) if r == "SPIKE PASS" => {}
        Some(r) => {
            if app.frames < 3 {
                println!("{r}");
            }
            std::process::exit(1);
        }
        None => {
            println!("SPIKE FAIL: never presented");
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("SPIKE FAIL: Windows only in M1");
}
