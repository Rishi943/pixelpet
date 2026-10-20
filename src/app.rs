//! Wires it together on a winit event loop:
//! tick -> read platform -> brain update -> rig render (cached) -> present.

use crate::brain::{Brain, Inputs, Mouse, Show};
use crate::ui::bubble::Bubble;
use crate::platform::{self, Platform, Rect};
use crate::rig::{self, Anim, Look};
use crate::settings::{self, Settings};
use crate::scene::{Hit, Part, Scene, Sprite};
use crate::ui::menu::{self, Action, Menu};
use crate::view::View;
use crate::window::PetWindow;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

/// Idle and fullscreen are polled this often (seconds).
const POLL_EVERY: f64 = 2.0;
/// The menu closes when the cursor has been away from it this long.
const MENU_AWAY_CLOSE: f64 = 4.0;

/// Crumbs fall this many rig units over CRUMB_LIFE seconds.
const CRUMB_FALL: f64 = 0.5;
const CRUMB_LIFE: f64 = 0.8;

/// Hearts rise this many rig units over their life, fading at the end.
const HEART_RISE: f64 = 1.5;
const HEART_LIFE: f64 = 1.2;
const HEART_FADE: f64 = 0.4;

/// The open hover bubble.
struct BubbleOpen {
    win: PetWindow,
    left: i32,
    top: i32,
    w: i32,
    h: i32,
}

/// The open right-click menu.
struct MenuOpen {
    win: PetWindow,
    menu: Menu,
    scale: u32,
    left: i32,
    top: i32,
    last_over: f64,
}

pub struct App {
    platform: Box<dyn Platform>,
    look: &'static Look,
    settings: Settings,
    settings_path: PathBuf,
    exe: PathBuf,
    start: Instant,
    exit_after: Option<f64>,
    pin: Option<Anim>,
    win: Option<PetWindow>,
    menu: Option<MenuOpen>,
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
    /// what was last shown, for hit-testing
    scene: Option<Scene>,
    last_x: f64,
    hidden: bool,
    /// whether the cursor was on the pet last tick (hover stickiness)
    was_over: bool,
    /// PIXELPET_SCRIPT: `t:action;...` dev actions replayed at t seconds
    script: Vec<(f64, String)>,
    next_tick: Instant,
    /// PIXELPET_OPEN_MENU: open the menu (without focus) once the pet is shown
    open_menu_at_start: bool,
    bubble: Bubble,
    bubble_open: Option<BubbleOpen>,
    /// right-click or Esc during play, passed to the brain on the next tick
    cancel_pending: bool,
    esc_was_down: bool,
    last_show: Option<Show>,
    /// recent (time, cursor y) while the button is held, for the flick speed
    drag_trail: VecDeque<(f64, i32)>,
    /// the OS let us move the cursor (tug); false after the first refusal
    can_warp: bool,
}

fn seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
}

impl App {
    pub fn new() -> App {
        let platform = platform::native();
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("PixelPet.exe"));
        let settings_path = settings::settings_path(settings::current_os(), &exe, &|k| std::env::var(k).ok());
        let settings = settings::load(&settings_path);
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
            settings_path,
            exe,
            start: Instant::now(),
            exit_after,
            pin,
            win: None,
            menu: None,
            brain,
            view: View::new(pet, look),
            bubble: Bubble::new(pet),
            bubble_open: None,
            cancel_pending: false,
            esc_was_down: false,
            last_show: None,
            drag_trail: VecDeque::new(),
            can_warp: true,
            mouse: VecDeque::new(),
            button_down: false,
            idle: None,
            fullscreen: None,
            last_poll: None,
            presented: None,
            placed: None,
            scene: None,
            last_x: 0.0,
            hidden: true,
            was_over: false,
            script: std::env::var("PIXELPET_SCRIPT")
                .map(|v| {
                    v.split(';')
                        .filter_map(|e| e.split_once(':'))
                        .filter_map(|(t, a)| Some((t.trim().parse().ok()?, a.trim().to_string())))
                        .collect()
                })
                .unwrap_or_default(),
            next_tick: Instant::now(),
            open_menu_at_start: std::env::var("PIXELPET_OPEN_MENU").is_ok_and(|v| v.trim() == "1"),
        }
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Whether the cursor is on the pet in the scene last shown.
    fn over_pet(&self, cursor: Option<(i32, i32)>) -> bool {
        let (Some(c), Some(scene), false) = (cursor, self.scene.as_ref(), self.hidden) else { return false };
        scene.hit(c.0, c.1, self.was_over) == Hit::Pet
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
        self.menu_tick(now, cursor);
        while self.script.first().is_some_and(|(t, _)| now >= *t) {
            let (_, a) = self.script.remove(0);
            match a.as_str() {
                "menu" => self.open_menu(el, false),
                "menu-focus" => self.open_menu(el, true),
                "close" => self.close_menu(),
                "size=S" => self.act(Action::SetSize(crate::settings::Size::S), el),
                "size=M" => self.act(Action::SetSize(crate::settings::Size::M), el),
                "size=L" => self.act(Action::SetSize(crate::settings::Size::L), el),
                "size=XL" => self.act(Action::SetSize(crate::settings::Size::XL), el),
                "ask-quit" => self.act(Action::AskQuit, el),
                "cancel-quit" => self.act(Action::CancelQuit, el),
                _ => {}
            }
        }
        // Esc ends play; read only while playing, and only Esc
        if self.last_show.is_some_and(|s| s.playing) {
            let down = self.platform.esc_down();
            if down && !self.esc_was_down {
                self.cancel_pending = true;
            }
            self.esc_was_down = down;
        } else {
            self.esc_was_down = false;
        }
        let mouse = match self.mouse.pop_front() {
            Some(m) => m,
            None if self.button_down => cursor.map_or(Mouse::None, |(x, y)| Mouse::Move(x, y)),
            None => Mouse::None,
        };
        // the flick: how fast the cursor rose over the last 100 ms before letting go
        if let (true, Some((_, y))) = (self.button_down, cursor) {
            self.drag_trail.push_back((now, y));
        }
        while self.drag_trail.front().is_some_and(|(t, _)| now - t > 0.1) {
            self.drag_trail.pop_front();
        }
        let flick_up = match (mouse, self.drag_trail.front(), self.drag_trail.back()) {
            (Mouse::Up(..), Some(&(t0, y0)), Some(&(t1, y1))) if t1 > t0 => (y0 - y1) as f64 / (t1 - t0),
            _ => 0.0,
        };
        if let Mouse::Up(..) = mouse {
            self.drag_trail.clear();
        }
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
            over_bubble: self.over_bubble(cursor),
            over_toy: self.over_toy(cursor),
            cancel: std::mem::take(&mut self.cancel_pending),
            flick_up,
            can_warp: self.can_warp,
        };
        let show = self.brain.update(&inp, &self.settings);
        self.was_over = inp.over_pet;
        self.last_x = show.x;
        self.last_show = Some(show);
        if let Some((x, y)) = show.warp {
            if !self.platform.warp_cursor(x, y) {
                self.can_warp = false; // the rope stretches instead
            }
        }
        self.present(&show, now, zoom, work);
        self.bubble_tick(el, &show, work);
        if self.open_menu_at_start && self.placed.is_some() {
            self.open_menu_at_start = false;
            self.open_menu(el, false);
        }
        let hearts = show.props.hearts.iter().any(|h| h.is_some());
        let busy = self.button_down || show.lift > 0.0 || !self.mouse.is_empty() || show.playing || show.bubble || hearts;
        let hz = if busy { 30.0 } else { 12.0 };
        self.next_tick = Instant::now() + Duration::from_secs_f64(1.0 / hz);
        el.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }

    fn present(&mut self, show: &Show, now: f64, zoom: u32, work: Rect) {
        let ground = work.bottom();
        // things at depth shrink towards the vanishing point
        let vp = (show.vp_x, ground as f64 - 0.3 * work.h as f64);
        let Some(win) = self.win.as_mut() else { return };
        if !show.visible {
            if !self.hidden {
                win.hide();
                self.hidden = true;
            }
            return;
        }
        let (_, dx, dy) = self.view.frame(show, now, zoom, false, show.depth);
        let z = zoom as i32;
        let (px, py, _) = crate::scene::project(show.x, show.depth, vp, ground as f64);
        // snap to whole art pixels in x
        let left = (px.round() as i32 + dx).div_euclid(z) * z;
        let top = py.round() as i32 + dy - show.lift.round() as i32;
        let opacity = (show.opacity as f64 * 255.0 / 100.0).round() as u8;
        let key = (self.view.image_generation(), left, top, opacity);
        let hearts = show.props.hearts.iter().any(|h| h.is_some());
        if self.presented == Some(key) && !self.hidden && !hearts && !show.playing {
            return;
        }
        let mut sprites = vec![Sprite { img: self.view.image().clone(), x: left, y: top, part: Part::Pet }];
        let u = self.look.r * zoom as f64;
        if let Some(t) = show.props.toy.filter(|t| !t.in_mouth) {
            let (tx, ty, s) = crate::scene::project(t.x, t.depth, vp, ground as f64);
            let f = rig::render_toy(t.toy, rig::look_scaled(self.look, s));
            let img = zoomed(&f, zoom);
            let x = tx.round() as i32 - f.ox * z;
            let y = (ty - t.lift * s).round() as i32 - (f.gy + 1) * z;
            // the toy goes in front of the pet unless it is behind (deeper)
            let sprite = Sprite { img, x, y, part: Part::Toy };
            if t.depth > show.depth + 1e-6 {
                sprites.insert(0, sprite);
            } else {
                sprites.push(sprite);
            }
        }
        for t0 in show.props.hearts.iter().flatten() {
            let age = now - t0;
            let img = heart_image(zoom, age);
            let hx = show.x + show.facing as f64 * 0.3 * u - img.w as f64 / 2.0;
            let hy = ground as f64 - 1.5 * u - HEART_RISE * u * (age / HEART_LIFE) - img.h as f64;
            sprites.push(Sprite { img, x: hx.round() as i32, y: hy.round() as i32, part: Part::Effect });
        }
        if let Some(f) = show.props.food {
            let fr = rig::props::food(f.food, f.bites);
            if fr.w > 0 {
                let x = f.x.round() as i32 - fr.ox * z;
                let y = (ground as f64 - f.lift).round() as i32 - (fr.gy + 1) * z;
                sprites.insert(0, Sprite { img: zoomed(&fr, zoom), x, y, part: Part::Food });
            }
            if let Some(t0) = show.props.crumbs {
                let age = now - t0;
                if age < CRUMB_LIFE {
                    let c = rig::props::crumb(f.food);
                    for i in 0..6 {
                        // a little spread, falling to the ground
                        let dx = (i as f64 - 2.5) * 0.12 * u + ((i * 37 % 7) as f64 - 3.0) * 0.02 * u;
                        let y = ground as f64 - 0.6 * u + CRUMB_FALL * u * (age / CRUMB_LIFE);
                        let mut img = crate::window::Rgba::new(z as usize, z as usize);
                        for p in img.px.chunks_mut(4) {
                            p.copy_from_slice(&c);
                        }
                        let x = (f.x + dx).round() as i32;
                        sprites.push(Sprite { img, x, y: y.min(ground as f64 - z as f64).round() as i32, part: Part::Effect });
                    }
                }
            }
        }
        let rope = match (show.props.rope_to, self.view.mouth()) {
            (Some(to), Some((mx, my))) => {
                let sag = if show.props.taut { 0.2 } else { 0.4 } * u;
                Some(((left + mx, top + my), to, zoom, sag))
            }
            _ => None,
        };
        let scene = Scene { sprites, rope };
        let (img, sx, sy) = scene.compose();
        if let Err(e) = win.present(&img, sx, sy, opacity) {
            eprintln!("present failed: {e}");
            return;
        }
        self.scene = Some(scene);
        if self.hidden {
            win.show();
            self.hidden = false;
        }
        self.presented = Some(key);
        self.placed = Some((left, top));
    }

    // ---------------------------------------------------------------- bubble

    fn over_bubble(&self, cursor: Option<(i32, i32)>) -> bool {
        let (Some((x, y)), Some(b)) = (cursor, self.bubble_open.as_ref()) else { return false };
        x >= b.left && y >= b.top && x < b.left + b.w && y < b.top + b.h
    }

    fn over_toy(&self, cursor: Option<(i32, i32)>) -> bool {
        let (Some(c), Some(scene), false) = (cursor, self.scene.as_ref(), self.hidden) else { return false };
        scene.hit(c.0, c.1, false) == Hit::Toy
    }

    /// Open the bubble when the brain says so, above the pet's head; close it
    /// when it says not.
    fn bubble_tick(&mut self, el: &ActiveEventLoop, show: &Show, work: Rect) {
        if !show.bubble || !show.visible || self.menu.is_some() {
            if self.bubble_open.take().is_some() {
                self.bubble.reset();
            }
            return;
        }
        if self.bubble_open.is_none() {
            match PetWindow::new(el, false) {
                Ok(win) => self.bubble_open = Some(BubbleOpen { win, left: 0, top: 0, w: 0, h: 0 }),
                Err(e) => {
                    eprintln!("PixelPet: could not open the bubble: {e}");
                    return;
                }
            }
            self.draw_bubble(work);
        }
    }

    /// Draw the bubble at its current page, centred over the pet, its tail
    /// just above the pet's head, kept inside the work area.
    fn draw_bubble(&mut self, work: Rect) {
        let scale = self.settings.size.zoom();
        let img = self.bubble.draw(scale);
        let (w, h) = (img.w as i32, img.h as i32);
        let (Some((_, pt)), Some(show)) = (self.placed, self.last_show) else { return };
        let pet = self.view.image();
        let head = (0..pet.h).find(|&y| (0..pet.w).any(|x| pet.px[(y * pet.w + x) * 4 + 3] > 0)).unwrap_or(0) as i32;
        let left = (show.x.round() as i32 - w / 2).clamp(work.x, (work.x + work.w - w).max(work.x));
        let top = (pt + head - h - 2).max(work.y);
        let Some(b) = self.bubble_open.as_mut() else { return };
        if let Err(e) = b.win.present(&img, left, top, 255) {
            eprintln!("PixelPet: bubble present failed: {e}");
        }
        (b.left, b.top, b.w, b.h) = (left, top, w, h);
    }

    fn is_bubble(&self, id: WindowId) -> bool {
        self.bubble_open.as_ref().is_some_and(|b| b.win.id() == id)
    }

    fn bubble_click(&mut self) {
        let (Some(b), Some((cx, cy))) = (self.bubble_open.as_ref(), self.platform.cursor_pos()) else { return };
        let (x, y) = (cx - b.left, cy - b.top);
        let scale = self.settings.size.zoom();
        match self.bubble.click(x, y, scale) {
            Some(choice) => {
                // food drops from the bubble's tail
                let from = (b.left + b.w / 2, b.top + b.h);
                self.brain.choose_at(choice, self.now(), from);
                self.bubble.reset();
                self.bubble_open = None;
            }
            None => self.draw_bubble(self.platform.work_area()),
        }
    }

    // ---------------------------------------------------------------- menu

    /// Open the menu beside the pet, kept inside the work area.
    fn open_menu(&mut self, el: &ActiveEventLoop, focus: bool) {
        self.close_menu();
        let win = match PetWindow::new(el, true) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("PixelPet: could not open the menu: {e}");
                return;
            }
        };
        let work = self.platform.work_area();
        let menu = Menu::new(&self.settings);
        let scale = self.settings.size.zoom();
        let (left, top) = self.menu_place(&menu, scale, work);
        self.menu = Some(MenuOpen { win, menu, scale, left, top, last_over: self.now() });
        self.draw_menu();
        if let (Some(m), true) = (&self.menu, focus) {
            m.win.focus();
        }
    }

    fn menu_place(&self, menu: &Menu, scale: u32, work: Rect) -> (i32, i32) {
        let (mw, mh) = menu.size();
        let (mw, mh) = ((mw as u32 * scale) as i32, (mh as u32 * scale) as i32);
        let pl = self.placed.map_or(work.x + work.w / 2, |p| p.0);
        let pw = self.view.image().w as i32;
        let right = work.x + work.w;
        // beside the pet: right of it if there is room, else left of it;
        // its bottom on the ground line
        let left = if pl + pw + mw <= right { pl + pw } else { pl - mw };
        let left = left.clamp(work.x, (right - mw).max(work.x));
        let top = (work.bottom() - mh).max(work.y);
        (left, top)
    }

    fn draw_menu(&mut self) {
        if let Some(m) = self.menu.as_mut() {
            let img = m.menu.draw(m.scale);
            if let Err(e) = m.win.present(&img, m.left, m.top, 255) {
                eprintln!("PixelPet: menu present failed: {e}");
            }
        }
    }

    fn close_menu(&mut self) {
        self.menu = None; // dropping the window closes it
    }

    fn menu_tick(&mut self, now: f64, cursor: Option<(i32, i32)>) {
        let Some(m) = self.menu.as_mut() else { return };
        let (mw, mh) = m.menu.size();
        let (w, h) = ((mw as u32 * m.scale) as i32, (mh as u32 * m.scale) as i32);
        if let Some((cx, cy)) = cursor {
            if cx >= m.left && cy >= m.top && cx < m.left + w && cy < m.top + h {
                m.last_over = now;
            }
        }
        if now - m.last_over > MENU_AWAY_CLOSE {
            self.close_menu();
        }
    }

    fn menu_click(&mut self, el: &ActiveEventLoop) {
        let (Some(m), Some((cx, cy))) = (self.menu.as_ref(), self.platform.cursor_pos()) else { return };
        let Some(action) = m.menu.hit(cx - m.left, cy - m.top, m.scale) else { return };
        self.act(action, el);
    }

    /// Apply a menu choice: settings change, side effects, save, redraw.
    fn act(&mut self, action: Action, el: &ActiveEventLoop) {
        if action == Action::Quit {
            self.close_menu();
            el.exit();
            return;
        }
        if matches!(action, Action::AskQuit | Action::CancelQuit) {
            let menu = if action == Action::AskQuit {
                Menu::confirming_quit(&self.settings)
            } else {
                Menu::new(&self.settings)
            };
            if let Some(scale) = self.menu.as_ref().map(|m| m.scale) {
                let (left, top) = self.menu_place(&menu, scale, self.platform.work_area());
                let m = self.menu.as_mut().unwrap();
                (m.menu, m.left, m.top) = (menu, left, top);
            }
            self.draw_menu();
            return;
        }
        let old_pet = self.settings.pet;
        menu::apply(&action, &mut self.settings);
        if action == Action::ToggleAutostart {
            let target = self.settings.autostart.then_some(self.exe.as_path());
            if let Err(e) = self.platform.set_autostart(target) {
                eprintln!("PixelPet: launch on startup: {e}");
                self.settings.autostart = !self.settings.autostart; // didn't happen
            }
        }
        if let Err(e) = settings::save(&self.settings_path, &self.settings) {
            eprintln!("PixelPet: could not save settings: {e}");
        }
        if self.settings.pet != old_pet {
            let pet = self.settings.pet.pet();
            let mut brain = Brain::new(pet, seed(), self.now(), self.platform.work_area());
            brain.place(self.last_x);
            brain.pin(self.pin);
            self.brain = brain;
            self.view = View::new(pet, self.look);
            // the bubble's Play and Feed pages are per pet
            self.bubble = Bubble::new(pet);
            self.bubble_open = None;
            self.presented = None;
        }
        if let Some(m) = self.menu.as_mut() {
            m.menu = Menu::new(&self.settings);
            m.scale = self.settings.size.zoom();
        }
        if matches!(action, Action::SetSize(_)) {
            self.presented = None;
            if let Some(m) = &self.menu {
                let (left, top) = self.menu_place(&m.menu, m.scale, self.platform.work_area());
                let m = self.menu.as_mut().unwrap();
                m.left = left;
                m.top = top;
            }
        }
        self.draw_menu();
    }

    fn is_menu(&self, id: WindowId) -> bool {
        self.menu.as_ref().is_some_and(|m| m.win.id() == id)
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

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, ev: WindowEvent) {
        if self.is_bubble(id) {
            if let WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } = ev {
                self.bubble_click();
            }
            return;
        }
        if self.is_menu(id) {
            match ev {
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.menu_click(el),
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => self.close_menu(),
                WindowEvent::KeyboardInput { event, .. } if event.logical_key == Key::Named(NamedKey::Escape) => self.close_menu(),
                WindowEvent::Focused(false) => self.close_menu(),
                _ => {}
            }
            return;
        }
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
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => {
                if self.last_show.is_some_and(|s| s.playing) {
                    // right-click ends play rather than opening the menu
                    self.cancel_pending = true;
                } else if self.menu.is_some() {
                    self.close_menu();
                } else {
                    self.open_menu(el, true);
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

/// A rig frame scaled up by `zoom`, nearest neighbour.
fn zoomed(f: &rig::Frame, zoom: u32) -> crate::window::Rgba {
    let z = zoom.max(1) as usize;
    let mut img = crate::window::Rgba::new(f.w * z, f.h * z);
    for y in 0..f.h * z {
        for x in 0..f.w * z {
            let s = ((y / z) * f.w + x / z) * 4;
            let d = (y * f.w * z + x) * 4;
            img.px[d..d + 4].copy_from_slice(&f.rgba[s..s + 4]);
        }
    }
    img
}

/// A heart at `zoom`, faded by `age` over its last HEART_FADE seconds.
fn heart_image(zoom: u32, age: f64) -> crate::window::Rgba {
    let f = rig::props::heart();
    let z = zoom.max(1) as usize;
    let fade = ((HEART_LIFE - age) / HEART_FADE).clamp(0.0, 1.0);
    let mut img = crate::window::Rgba::new(f.w * z, f.h * z);
    for y in 0..f.h * z {
        for x in 0..f.w * z {
            let s = ((y / z) * f.w + x / z) * 4;
            let d = (y * f.w * z + x) * 4;
            img.px[d..d + 3].copy_from_slice(&f.rgba[s..s + 3]);
            img.px[d + 3] = (f.rgba[s + 3] as f64 * fade).round() as u8;
        }
    }
    img
}

/// Run the pet until it quits.
pub fn run() -> Result<(), String> {
    // one pet at a time: a second copy just leaves quietly
    let Some(_guard) = platform::claim_single_instance("PixelPet") else {
        eprintln!("PixelPet is already running");
        return Ok(());
    };
    let el = EventLoop::new().map_err(|e| e.to_string())?;
    let mut app = App::new();
    el.run_app(&mut app).map_err(|e| e.to_string())
}
