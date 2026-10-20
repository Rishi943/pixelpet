//! The brain: decides which anim plays and where the pet is. Pure logic, no
//! OS and no pixels. Coordinates are physical pixels; `x` is the pet's ground
//! point and `lift` its height above the ground.

pub mod feed;
pub mod fetch;
pub mod pet;
pub mod wander;

use crate::platform::Rect;
use crate::rig::{self, Anim, Food, Look, Pet, Toy};
use crate::settings::{FullscreenMode, Settings};
use wander::Wander;

/// Idle seconds before the pet lies down for a nap.
pub const NAP_AFTER: f64 = 180.0;
/// Hovering this long opens the bubble (spec §4).
pub const BUBBLE_AFTER: f64 = 0.4;
/// The bubble stays this long after the cursor leaves the pet and the bubble.
pub const BUBBLE_LINGER: f64 = 0.6;
/// Strokes before the pet settles in (Alex leans, the bun flops).
pub const STROKES_TO_SETTLE: u32 = 6;
/// Play ends after this long with no mouse input.
pub const PLAY_IDLE_END: f64 = 60.0;
/// ... or this long without actually playing (no throw, bite or stroke):
/// moving the mouse for other work puts the toy or food away.
pub const PLAY_UNENGAGED: f64 = 30.0;
const HEART_LIFE: f64 = 1.2;
/// Petting ends once the cursor has been off the pet this long.
const PET_LEAVE: f64 = 0.5;
const DRAG_START_PX: i32 = 4;
pub(crate) const GRAVITY: f64 = 2400.0;
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
    /// The cursor is over the open bubble.
    pub over_bubble: bool,
    /// The cursor is over the toy.
    pub over_toy: bool,
    /// End play now (right-click on the pet or Esc during play), for one tick.
    pub cancel: bool,
    /// How fast the cursor was moving upwards when the button was let go
    /// (px/s, over the last 100 ms); the app sets it with `Mouse::Up`.
    pub flick_up: f64,
    /// The real cursor can be moved (feeding pulls it); false on Wayland.
    pub can_warp: bool,
}

/// A choice from the bubble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    Pet,
    Fetch(Toy),
    Feed(Food),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToyShow {
    pub toy: Toy,
    /// ground x (physical px, at depth 0 coordinates), height above the
    /// ground and depth
    pub x: f64,
    pub lift: f64,
    pub depth: f64,
    pub in_mouth: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoodShow {
    pub food: Food,
    pub x: f64,
    pub lift: f64,
    pub bites: u8,
}

/// Things drawn with the pet.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Props {
    pub toy: Option<ToyShow>,
    pub food: Option<FoodShow>,
    /// When each heart was floated (shown for 1.2 s).
    pub hearts: [Option<f64>; 6],
    /// When the crumbs started falling.
    pub crumbs: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Play {
    Pet { strokes: pet::Strokes, off_since: Option<f64> },
    Fetch { toy: Toy, st: fetch::State },
    Feed { food: Food, st: feed::State },
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
    /// The bubble is open.
    pub bubble: bool,
    /// How far into the screen the pet is (0 = on the ground line).
    pub depth: f64,
    /// The x of the vanishing point things at depth shrink towards (the
    /// throw's x); its y is 0.3 of the work area's height above the ground.
    pub vp_x: f64,
    pub eyes_closed: bool,
    /// Pet, fetch, tug or feed is going on.
    pub playing: bool,
    pub props: Props,
    /// Move the real cursor here this frame (feeding pulls the food in).
    pub warp: Option<(i32, i32)>,
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
    Play(Play),
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
    last_idle: Option<f64>,
    /// the work area's middle, which a watching pet faces
    centre: f64,
    bubble: bool,
    bubble_off_since: Option<f64>,
    props: Props,
    heart_i: usize,
    eyes_closed: bool,
    depth: f64,
    vp_x: f64,
    warp: Option<(i32, i32)>,
    /// from the last update: px per rig unit, and the ground line
    last_u: f64,
    last_ground: f64,
    /// last throw, fetch, bite or stroke (for PLAY_UNENGAGED)
    engaged_at: f64,
    /// last mouse input during play (for PLAY_IDLE_END)
    play_input_at: f64,
    last_cursor: Option<(i32, i32)>,
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
            last_idle: None,
            centre: work.x as f64 + work.w as f64 / 2.0,
            bubble: false,
            bubble_off_since: None,
            props: Props::default(),
            heart_i: 0,
            eyes_closed: false,
            depth: 0.0,
            vp_x: work.x as f64 + work.w as f64 / 2.0,
            warp: None,
            last_u: 66.0,
            last_ground: work.bottom() as f64,
            play_input_at: now,
            engaged_at: now,
            last_cursor: None,
        };
        b.next_wander(now);
        b
    }

    /// Put the pet at ground x (clamped into the work area on the next
    /// update). Used when the pet is swapped, so the new one appears in place.
    pub fn place(&mut self, x: f64) {
        self.x = x;
    }

    /// A choice from the bubble. Closes the bubble and starts that play.
    pub fn choose(&mut self, c: Choice, now: f64) {
        self.bubble = false;
        self.bubble_off_since = None;
        self.hover_start = None;
        self.play_input_at = now;
        self.engaged_at = now;
        match c {
            Choice::Pet => {
                self.mode = Mode::Play(Play::Pet { strokes: pet::Strokes::default(), off_since: None });
                self.show_anim(Anim::Stand, now);
            }
            Choice::Fetch(toy) => {
                self.mode = Mode::Play(Play::Fetch { toy, st: fetch::State::Waiting { since: now, at: None } });
                self.show_anim(Anim::Stand, now);
            }
            Choice::Feed(food) => {
                // in the hand; it follows the cursor from the next tick
                let at = self.last_cursor.unwrap_or((self.x.round() as i32, (self.last_ground - 1.6 * self.last_u) as i32));
                self.props.food = Some(feed::in_hand(food, at, self.last_ground));
                self.mode = Mode::Play(Play::Feed { food, st: feed::State::Follow });
                self.show_anim(Anim::Stand, now);
            }
        }
    }

    /// Back to wandering: props gone, eyes open, on the ground line.
    fn end_play(&mut self, now: f64) {
        self.props = Props::default();
        self.eyes_closed = false;
        self.depth = 0.0;
        self.next_wander(now);
    }

    fn play(&mut self, p: Play, inp: &Inputs, now: f64, dt: f64, lo: f64, hi: f64) {
        match p {
            Play::Fetch { toy, st } => self.fetch(toy, st, inp, now, dt, lo, hi),
            Play::Feed { food, st } => self.feed(food, st, inp, now, dt, lo, hi),
            Play::Pet { mut strokes, mut off_since } => {
                if inp.over_pet {
                    off_since = None;
                } else if off_since.is_none() {
                    off_since = Some(now);
                }
                if off_since.is_some_and(|t| now - t >= PET_LEAVE) {
                    return self.end_play(now);
                }
                if let (Some((cx, _)), true) = (inp.cursor, inp.over_pet) {
                    if strokes.feed(cx as f64, inp.unit_px) {
                        self.engaged_at = now;
                        self.props.hearts[self.heart_i % 6] = Some(now);
                        self.heart_i += 1;
                    }
                }
                let n = strokes.count;
                self.eyes_closed = n >= 1;
                let anim = match (self.pet, n) {
                    (_, 0) => Anim::Stand,
                    (Pet::Dog, n) if n >= STROKES_TO_SETTLE => Anim::Lean,
                    (Pet::Dog, _) => Anim::Wag,
                    (Pet::Rabbit, n) if n >= STROKES_TO_SETTLE => Anim::Flop,
                    (Pet::Rabbit, _) => Anim::Stand,
                };
                if anim != self.anim {
                    self.show_anim(anim, now);
                }
                self.mode = Mode::Play(Play::Pet { strokes, off_since });
            }
        }
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
        if n.anim == Anim::Watch {
            self.facing = if self.centre >= self.x { 1 } else { -1 };
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
        self.warp = None;
        self.last_u = inp.unit_px;
        self.last_ground = inp.work.bottom() as f64;
        self.last_now = now;
        let u = inp.unit_px;
        let lo = inp.work.x as f64 + 1.2 * u;
        let hi = (inp.work.x + inp.work.w) as f64 - 1.2 * u;
        self.x = self.x.clamp(lo, hi.max(lo));
        self.centre = inp.work.x as f64 + inp.work.w as f64 / 2.0;

        // ---- mouse: click vs drag
        match inp.mouse {
            Mouse::Down(px, py) if inp.over_pet && !inp.over_toy && !matches!(self.mode, Mode::Drag | Mode::Fall { .. }) => {
                self.down = Some((px, py));
            }
            Mouse::Move(px, py) => {
                if let (Some((dx, dy)), false) = (self.down, self.mode == Mode::Drag) {
                    let (ddx, ddy) = (px - dx, py - dy);
                    if ddx * ddx + ddy * ddy > DRAG_START_PX * DRAG_START_PX {
                        self.end_hover(now);
                        // picking the pet up ends any play
                        self.props = Props::default();
                        self.eyes_closed = false;
                        self.depth = 0.0;
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
        // idle is polled, so input between polls shows up as idle going down
        let input_since = match (idle, self.last_idle) {
            (Some(i), Some(prev)) => i < 1.0 || i < prev - 1e-3,
            (Some(i), None) => i < 1.0,
            (None, _) => true,
        };
        self.last_idle = idle;
        // play input: any mouse event or cursor movement
        if inp.mouse != Mouse::None || inp.cursor != self.last_cursor {
            self.play_input_at = now;
        }
        self.last_cursor = inp.cursor;
        if let Mode::Play(_) = self.mode {
            if inp.cancel || now - self.play_input_at >= PLAY_IDLE_END || now - self.engaged_at >= PLAY_UNENGAGED {
                self.end_play(now);
            }
        }
        // hearts float for HEART_LIFE
        for h in self.props.hearts.iter_mut() {
            if h.is_some_and(|t| now - t >= HEART_LIFE) {
                *h = None;
            }
        }
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
                self.props = Props::default();
                self.eyes_closed = false;
                self.depth = 0.0;
                self.mode = Mode::Nap;
                self.wander.note(Anim::Sleep);
                self.show_anim(Anim::Sleep, now);
            }
            Mode::Nap => {
                if input_since {
                    // wakes, stretches, then walks to the cursor (spec §4)
                    self.mode = Mode::Waking { until: now + rig::period(self.pet, Anim::Stretch) };
                    self.wander.note(Anim::Stand);
                    self.show_anim(Anim::Stretch, now);
                }
            }
            Mode::Play(p) => self.play(p, inp, now, dt, lo, hi),
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
        // the bubble: after a hover, while wandering; stays while the cursor
        // is on the pet or the bubble, lingers a moment after
        let can_bubble = matches!(self.mode, Mode::Wander | Mode::Seek) && self.pinned.is_none();
        if !can_bubble {
            self.bubble = false;
        } else if hover_for >= BUBBLE_AFTER {
            self.bubble = true;
        }
        if self.bubble {
            if inp.over_pet || inp.over_bubble {
                self.bubble_off_since = None;
            } else if now - *self.bubble_off_since.get_or_insert(now) >= BUBBLE_LINGER {
                self.bubble = false;
                self.bubble_off_since = None;
            }
        }
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
            bubble: self.bubble,
            depth: self.depth,
            vp_x: self.vp_x,
            eyes_closed: self.eyes_closed,
            playing: matches!(self.mode, Mode::Play(_)),
            props: self.props,
            warp: self.warp,
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
