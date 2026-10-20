//! The brain: decides which anim plays and where the pet is. Pure logic, no
//! OS and no pixels. Coordinates are physical pixels; `x` is the pet's ground
//! point and `lift` its height above the ground.

pub mod wander;

use crate::platform::Rect;
use crate::rig::{self, Anim, Look, Pet};
use crate::settings::{FullscreenMode, Settings};
use wander::Wander;

/// Idle seconds before the pet lies down for a nap.
pub const NAP_AFTER: f64 = 180.0;
const WAKE_STAND: f64 = 1.5;
const DRAG_START_PX: i32 = 4;
const GRAVITY: f64 = 2400.0;
const LANDED_STAND: f64 = 1.0;
const MAX_DT: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mouse {
    None,
    Down(i32, i32),
    Move(i32, i32),
    Up(i32, i32),
}

#[derive(Clone, Copy, Debug)]
pub struct Inputs {
    pub now: f64,
    pub work: Rect,
    /// Physical px per rig unit (rig `R` x zoom).
    pub unit_px: f64,
    pub cursor: Option<(i32, i32)>,
    pub over_pet: bool,
    pub mouse: Mouse,
    pub idle: Option<f64>,
    pub fullscreen: Option<bool>,
}

/// What to show this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Show {
    pub anim: Anim,
    pub anim_start: f64,
    pub x: f64,
    pub lift: f64,
    pub facing: i8,
    pub visible: bool,
    /// Percent, after the fullscreen rules.
    pub opacity: u8,
    pub zzz: bool,
    /// Seconds the cursor has rested on the pet (0 when it isn't).
    pub hover_for: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Wander,
    Nap,
    Waking { until: f64 },
    Seek,
    Drag,
    Fall { v: f64 },
    Landed { until: f64 },
}

pub struct Brain {
    pet: Pet,
    look: &'static Look,
    wander: Wander,
    mode: Mode,
    // the wander script's current anim
    w_anim: Anim,
    w_start: f64,
    w_end: f64,
    // what is on screen
    anim: Anim,
    anim_start: f64,
    x: f64,
    facing: i8,
    lift: f64,
    hover_start: Option<f64>,
    hover_left: f64,
    down: Option<(i32, i32)>,
    pinned: Option<Anim>,
    pin_start: Option<f64>,
    last_now: f64,
}

impl Brain {
    pub fn new(pet: Pet, seed: u64, now: f64, work: Rect) -> Brain {
        let mut b = Brain {
            pet,
            // v1 ships look B only
            look: rig::look("classic"),
            wander: Wander::new(seed),
            mode: Mode::Wander,
            w_anim: Anim::Stand,
            w_start: now,
            w_end: now,
            anim: Anim::Stand,
            anim_start: now,
            x: work.x as f64 + work.w as f64 / 2.0,
            facing: 1,
            lift: 0.0,
            hover_start: None,
            hover_left: 0.0,
            down: None,
            pinned: None,
            pin_start: None,
            last_now: now,
        };
        b.next_wander(now);
        b
    }

    /// Hold one anim (or release with None). Drag still works; nothing else
    /// changes the anim. Used by the measuring tool.
    pub fn pin(&mut self, anim: Option<Anim>) {
        self.pinned = anim;
        self.pin_start = None;
    }

    fn next_wander(&mut self, now: f64) {
        let n = self.wander.next();
        if n.turn {
            self.facing = -self.facing;
        }
        self.mode = Mode::Wander;
        self.w_anim = n.anim;
        self.w_start = now;
        self.w_end = now + n.secs;
        self.show_anim(n.anim, now);
    }

    fn show_anim(&mut self, anim: Anim, start: f64) {
        self.anim = anim;
        self.anim_start = start;
    }

    /// Move along the ground if the current anim travels. Returns true when
    /// the pet was stopped by an edge.
    fn travel(&mut self, now: f64, dt: f64, unit_px: f64, lo: f64, hi: f64, turn: bool) -> bool {
        let sp = rig::speed(self.pet, self.look, self.anim);
        if sp <= 0.0 {
            return false;
        }
        let phase = ((now - self.anim_start) / rig::period(self.pet, self.anim)).rem_euclid(1.0);
        self.x += self.facing as f64 * sp * rig::move_profile(self.pet, self.anim, phase) * unit_px * dt;
        if self.x <= lo {
            self.x = lo;
            if turn {
                self.facing = 1;
            }
            return true;
        }
        if self.x >= hi {
            self.x = hi;
            if turn {
                self.facing = -1;
            }
            return true;
        }
        false
    }

    pub fn update(&mut self, inp: &Inputs, s: &Settings) -> Show {
        let now = inp.now;
        let dt = (now - self.last_now).clamp(0.0, MAX_DT);
        self.last_now = now;
        let u = inp.unit_px;
        let lo = inp.work.x as f64 + 1.2 * u;
        let hi = (inp.work.x + inp.work.w) as f64 - 1.2 * u;
        self.x = self.x.clamp(lo, hi.max(lo));

        // ---- mouse: click vs drag
        match inp.mouse {
            Mouse::Down(px, py) if inp.over_pet && !matches!(self.mode, Mode::Drag | Mode::Fall { .. }) => {
                self.down = Some((px, py));
            }
            Mouse::Move(px, py) => {
                if let (Some((dx, dy)), false) = (self.down, self.mode == Mode::Drag) {
                    let (ddx, ddy) = (px - dx, py - dy);
                    if ddx * ddx + ddy * ddy > DRAG_START_PX * DRAG_START_PX {
                        self.end_hover(now);
                        self.mode = Mode::Drag;
                    }
                }
                if self.mode == Mode::Drag {
                    self.x = (px as f64).clamp(lo, hi.max(lo));
                    self.lift = (inp.work.bottom() as f64 - py as f64 - 0.3 * u).max(0.0);
                }
            }
            Mouse::Up(..) => {
                self.down = None;
                if self.mode == Mode::Drag {
                    self.mode = Mode::Fall { v: 0.0 };
                }
            }
            _ => {}
        }

        let idle = inp.idle;
        match self.mode {
            Mode::Drag => {
                if self.anim != Anim::Stand {
                    self.show_anim(Anim::Stand, now);
                }
            }
            Mode::Fall { v } => {
                let v = v + GRAVITY * dt;
                self.lift -= v * dt;
                if self.lift <= 0.0 {
                    self.lift = 0.0;
                    self.mode = Mode::Landed { until: now + LANDED_STAND };
                } else {
                    self.mode = Mode::Fall { v };
                }
                if self.anim != Anim::Stand {
                    self.show_anim(Anim::Stand, now);
                }
            }
            _ if self.pinned.is_some() => {
                let a = self.pinned.unwrap();
                let start = *self.pin_start.get_or_insert(now);
                self.mode = Mode::Wander;
                self.show_anim(a, start);
                self.travel(now, dt, u, lo, hi, true);
            }
            Mode::Landed { until } => {
                if now >= until {
                    self.next_wander(now);
                }
            }
            _ if self.mode != Mode::Nap && idle.is_some_and(|i| i >= NAP_AFTER) => {
                self.end_hover(now);
                self.mode = Mode::Nap;
                self.wander.note(Anim::Sleep);
                self.show_anim(Anim::Sleep, now);
            }
            Mode::Nap => {
                if idle.map_or(true, |i| i < 1.0) {
                    self.mode = Mode::Waking { until: now + WAKE_STAND };
                    self.wander.note(Anim::Stand);
                    self.show_anim(Anim::Stand, now);
                }
            }
            _ if inp.over_pet => {
                let start = *self.hover_start.get_or_insert_with(|| {
                    self.hover_left = (self.w_end - now).max(0.0);
                    now
                });
                self.show_anim(Anim::Tilt, start);
            }
            _ => {
                self.end_hover(now);
                match self.mode {
                    Mode::Waking { until } => {
                        if now >= until {
                            match inp.cursor {
                                Some((cx, _)) => {
                                    self.facing = if cx as f64 >= self.x { 1 } else { -1 };
                                    self.mode = Mode::Seek;
                                    self.show_anim(Anim::Walk, now);
                                }
                                None => self.next_wander(now),
                            }
                        }
                    }
                    Mode::Seek => match inp.cursor {
                        Some((cx, _)) if (cx as f64 - self.x).abs() > 1.5 * u => {
                            self.facing = if cx as f64 >= self.x { 1 } else { -1 };
                            if self.travel(now, dt, u, lo, hi, false) {
                                self.next_wander(now); // the cursor is past the edge
                            }
                        }
                        _ => self.next_wander(now),
                    },
                    _ => {
                        if now >= self.w_end {
                            self.next_wander(now);
                        }
                        self.travel(now, dt, u, lo, hi, true);
                    }
                }
            }
        }

        let hover_for = self.hover_start.map_or(0.0, |h| now - h);
        let fs = inp.fullscreen == Some(true);
        let (visible, opacity) = match (fs, s.fullscreen) {
            (true, FullscreenMode::Hide) => (false, s.opacity),
            (true, FullscreenMode::Stay) => (true, s.fullscreen_opacity),
            _ => (true, s.opacity),
        };
        Show {
            anim: self.anim,
            anim_start: self.anim_start,
            x: self.x,
            lift: self.lift,
            facing: self.facing,
            visible,
            opacity,
            zzz: self.anim == Anim::Sleep,
            hover_for,
        }
    }

    /// The cursor left the pet: whatever it was doing resumes, fresh.
    fn end_hover(&mut self, now: f64) {
        if self.hover_start.take().is_none() {
            return;
        }
        match self.mode {
            Mode::Wander => {
                self.w_start = now;
                self.w_end = now + self.hover_left;
                self.show_anim(self.w_anim, now);
            }
            Mode::Seek => self.show_anim(Anim::Walk, now),
            _ => self.show_anim(Anim::Stand, now),
        }
    }
}
