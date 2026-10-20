//! Wires it together on a winit event loop:
//! tick -> read platform -> brain update -> rig render (cached) -> present.

use crate::brain::{Brain, Inputs, Mouse, Show};
use crate::platform::{self, Platform};
use crate::rig::{self, Anim, Look};
use crate::settings::{self, Settings};
use crate::view::View;
use crate::window::PetWindow;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

/// Idle and fullscreen are polled this often (seconds).
const POLL_EVERY: f64 = 2.0;

pub struct App {
    platform: Box<dyn Platform>,
    look: &'static Look,
    settings: Settings,
    start: Instant,
    exit_after: Option<f64>,
    win: Option<PetWindow>,
    brain: Brain,
    view: View,
    mouse: VecDeque<Mouse>,
    button_down: bool,
    idle: Option<f64>,
    fullscreen: Option<bool>,
    last_poll: Option<f64>,
    /// (image generation, left, top, opacity) last presented
    presented: Option<(u64, i32, i32, u8)>,
    /// top-left of the image on screen, for hit-testing the cursor
    placed: Option<(i32, i32)>,
    hidden: bool,
    next_tick: Instant,
}

fn seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
}

impl App {
    pub fn new() -> App {
        let platform = platform::native();
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("PixelPet.exe"));
        let path = settings::settings_path(settings::current_os(), &exe, &|k| std::env::var(k).ok());
        let settings = settings::load(&path);
        // launch on startup follows the program if it moved
        let current = platform.autostart();
        if settings::autostart_needs_update(current.as_deref(), &exe, settings.autostart) {
            let _ = platform.set_autostart(settings.autostart.then_some(exe.as_path()));
        }
        let look = rig::look("classic");
        let pet = settings.pet.pet();
        let mut brain = Brain::new(pet, seed(), 0.0, platform.work_area());
        let pin = std::env::var("PIXELPET_PIN").ok().and_then(|v| Anim::from_name(v.trim()));
        brain.pin(pin);
        let exit_after = std::env::var("PIXELPET_EXIT_AFTER").ok().and_then(|v| v.trim().parse().ok());
        App {
            platform,
            look,
            settings,
            start: Instant::now(),
            exit_after,
            win: None,
            brain,
            view: View::new(pet, look),
            mouse: VecDeque::new(),
            button_down: false,
            idle: None,
            fullscreen: None,
            last_poll: None,
            presented: None,
            placed: None,
            hidden: true,
            next_tick: Instant::now(),
        }
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Whether the cursor is over an opaque pixel of the pet's image.
    fn over_pet(&self, cursor: Option<(i32, i32)>) -> bool {
        let (Some((cx, cy)), Some((left, top)), false) = (cursor, self.placed, self.hidden) else { return false };
        let img = self.view.image();
        let (ix, iy) = (cx - left, cy - top);
        if ix < 0 || iy < 0 || ix as usize >= img.w || iy as usize >= img.h {
            return false;
        }
        img.px[(iy as usize * img.w + ix as usize) * 4 + 3] > 0
    }

    fn tick(&mut self, el: &ActiveEventLoop) {
        let now = self.now();
        if self.exit_after.is_some_and(|t| now >= t) {
            el.exit();
            return;
        }
        if self.last_poll.map_or(true, |t| now - t >= POLL_EVERY) {
            self.idle = self.platform.idle_seconds();
            self.fullscreen = self.platform.fullscreen_app();
            self.last_poll = Some(now);
        }
        let cursor = self.platform.cursor_pos();
        let work = self.platform.work_area();
        let mouse = match self.mouse.pop_front() {
            Some(m) => m,
            None if self.button_down => cursor.map_or(Mouse::None, |(x, y)| Mouse::Move(x, y)),
            None => Mouse::None,
        };
        let zoom = self.settings.size.zoom();
        let inp = Inputs {
            now,
            work,
            unit_px: self.look.r * zoom as f64,
            cursor,
            over_pet: self.over_pet(cursor),
            mouse,
            idle: self.idle,
            fullscreen: self.fullscreen,
        };
        let show = self.brain.update(&inp, &self.settings);
        self.present(&show, now, zoom, work.bottom());
        let busy = self.button_down || show.lift > 0.0 || !self.mouse.is_empty();
        let hz = if busy { 30.0 } else { 12.0 };
        self.next_tick = Instant::now() + Duration::from_secs_f64(1.0 / hz);
        el.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }

    fn present(&mut self, show: &Show, now: f64, zoom: u32, ground: i32) {
        let Some(win) = self.win.as_mut() else { return };
        if !show.visible {
            if !self.hidden {
                win.hide();
                self.hidden = true;
            }
            return;
        }
        let (_, dx, dy) = self.view.frame(show, now, zoom, false);
        let z = zoom as i32;
        // snap to whole art pixels in x
        let left = (show.x.round() as i32 + dx).div_euclid(z) * z;
        let top = ground + dy - show.lift.round() as i32;
        let opacity = (show.opacity as f64 * 255.0 / 100.0).round() as u8;
        let key = (self.view.image_generation(), left, top, opacity);
        if self.presented == Some(key) && !self.hidden {
            return;
        }
        if let Err(e) = win.present(self.view.image(), left, top, opacity) {
            eprintln!("present failed: {e}");
            return;
        }
        if self.hidden {
            win.show();
            self.hidden = false;
        }
        self.presented = Some(key);
        self.placed = Some((left, top));
    }
}

impl Default for App {
    fn default() -> App {
        App::new()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.win.is_some() {
            return;
        }
        match PetWindow::new(el, false) {
            Ok(w) => self.win = Some(w),
            Err(e) => {
                eprintln!("PixelPet: could not open the pet window: {e}");
                el.exit();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let Some((x, y)) = self.platform.cursor_pos() else { return };
                match state {
                    ElementState::Pressed => {
                        self.button_down = true;
                        self.mouse.push_back(Mouse::Down(x, y));
                    }
                    ElementState::Released => {
                        self.button_down = false;
                        self.mouse.push_back(Mouse::Up(x, y));
                    }
                }
            }
            WindowEvent::CloseRequested => el.exit(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.win.is_none() {
            return;
        }
        // other window events (cursor moves) wake the loop too: tick only when due
        if self.mouse.is_empty() && Instant::now() < self.next_tick {
            el.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
            return;
        }
        self.tick(el);
    }
}

/// Run the pet until it quits.
pub fn run() -> Result<(), String> {
    let el = EventLoop::new().map_err(|e| e.to_string())?;
    let mut app = App::new();
    el.run_app(&mut app).map_err(|e| e.to_string())
}
