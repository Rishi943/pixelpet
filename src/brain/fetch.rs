//! Fetch (spec §5): the toy appears at the cursor; grab it and flick it up
//! and it flies into the screen (depth); the pet runs after it, brings it
//! back (Alex in his mouth, the bun nosing it along the ground) and sits by
//! it, ready for another throw.

use super::{Brain, Inputs, Mode, Mouse, Play, ToyShow, GRAVITY};
use crate::rig::{self, Anim, Pet, Toy};

/// How far into the screen a flick of `up_px_per_s` (upwards) sends the toy.
pub fn throw_depth(up_px_per_s: f64, unit_px: f64) -> f64 {
    (up_px_per_s / unit_px / 25.0).clamp(0.25, 3.0)
}

/// A slower flick than this (rig units per second, upwards) just drops it.
const MIN_FLICK: f64 = 4.0;
/// An untouched toy drops to the ground after this long.
const WAIT_DROP: f64 = 2.0;
const PICKUP: f64 = 0.4;
/// Rig units: the toy sits this far in front of a sitting pet, the mouth
/// stops this far short of it, the bun's nose this far behind it.
const READY_GAP: f64 = 0.6;
const MOUTH_GAP: f64 = 0.5;
const NOSE_GAP: f64 = 0.55;
/// The flight's arc height per unit of depth, in rig units.
const ARC: f64 = 3.0;
/// One unit of depth counts as this many rig units of distance to run.
const DEPTH_COST: f64 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The toy hangs where the cursor was when fetch was chosen.
    Waiting { since: f64, at: Option<(f64, f64)> },
    Held,
    Flying { t0: f64, dur: f64, from_lift: f64, target: f64 },
    Falling { v: f64, then_ready: bool },
    Chase,
    Pickup { until: f64 },
    Return { to: f64 },
    Ready,
}

impl Brain {
    /// Move the pet's (x, depth) towards (`tx`, `td`) at `anim`'s speed.
    /// True on arrival.
    pub(super) fn go_to(&mut self, tx: f64, td: f64, anim: Anim, now: f64, dt: f64, u: f64) -> bool {
        // Alex runs into the screen seen from behind, and back out of it
        // seen from the front, when the run is mostly into or out of it
        let into = DEPTH_COST * (td - self.depth).abs() > (tx - self.x).abs() / u;
        let anim = match (self.pet, anim, into) {
            (Pet::Dog, Anim::Run, true) if td > self.depth => Anim::RunAway,
            (Pet::Dog, Anim::Run, true) => Anim::RunBack,
            (Pet::Dog, Anim::Carry, true) if td < self.depth => Anim::CarryBack,
            _ => anim,
        };
        if (tx - self.x).abs() > 1.0 {
            self.facing = if tx > self.x { 1 } else { -1 };
        }
        if self.anim != anim {
            self.show_anim(anim, now);
        }
        let dist = (tx - self.x).abs() / u + DEPTH_COST * (td - self.depth).abs();
        let phase = ((now - self.anim_start) / rig::period(self.pet, anim)).rem_euclid(1.0);
        let step = rig::speed(self.pet, self.look, anim) * rig::move_profile(self.pet, anim, phase) * dt;
        if dist <= step.max(0.02) {
            self.x = tx;
            self.depth = td;
            return true;
        }
        let k = step / dist;
        self.x += (tx - self.x) * k;
        self.depth += (td - self.depth) * k;
        false
    }

    pub(super) fn fetch(&mut self, toy: Toy, st: State, inp: &Inputs, now: f64, dt: f64, lo: f64, hi: f64) {
        let u = inp.unit_px;
        let ground = inp.work.bottom() as f64;
        let clamp = |x: f64| x.clamp(lo, hi.max(lo));
        let cursor_x = inp.cursor.map_or(self.x, |c| clamp(c.0 as f64));
        let grab = matches!(inp.mouse, Mouse::Down(..)) && inp.over_toy;
        let mut t = self.props.toy.unwrap_or(ToyShow { toy, x: self.x, lift: 0.0, depth: 0.0, in_mouth: false });
        let (run, back) = match self.pet {
            Pet::Dog => (Anim::Run, Anim::Carry),
            Pet::Rabbit => (Anim::Walk, Anim::Nudge),
        };
        let st = match st {
            State::Waiting { since, at } => {
                let at = at.or_else(|| inp.cursor.map(|(cx, cy)| (clamp(cx as f64), (ground - cy as f64).max(0.0))));
                if let Some((x, lift)) = at {
                    (t.x, t.lift) = (x, lift);
                }
                if self.anim != Anim::Stand {
                    self.show_anim(Anim::Stand, now);
                }
                if grab {
                    State::Held
                } else if now - since >= WAIT_DROP {
                    State::Falling { v: 0.0, then_ready: true }
                } else {
                    State::Waiting { since, at }
                }
            }
            State::Held => {
                if let Some((cx, cy)) = inp.cursor {
                    (t.x, t.lift, t.depth) = (clamp(cx as f64), (ground - cy as f64).max(0.0), 0.0);
                }
                if let Mouse::Up(..) = inp.mouse {
                    if inp.flick_up >= MIN_FLICK * u {
                        let target = throw_depth(inp.flick_up, u);
                        self.vp_x = t.x;
                        State::Flying { t0: now, dur: 0.6 + 0.3 * target, from_lift: t.lift, target }
                    } else {
                        State::Falling { v: 0.0, then_ready: false }
                    }
                } else {
                    State::Held
                }
            }
            State::Flying { t0, dur, from_lift, target } => {
                let p = ((now - t0) / dur).min(1.0);
                t.depth = target * p;
                t.lift = from_lift * (1.0 - p) + 4.0 * target * p * (1.0 - p) * ARC * u;
                if p >= 1.0 {
                    (t.depth, t.lift) = (target, 0.0);
                    State::Chase
                } else {
                    State::Flying { t0, dur, from_lift, target }
                }
            }
            State::Falling { v, then_ready } => {
                let v = v + GRAVITY * dt;
                t.lift -= v * dt;
                if t.lift <= 0.0 {
                    t.lift = 0.0;
                    if then_ready {
                        State::Ready
                    } else {
                        State::Chase
                    }
                } else {
                    State::Falling { v, then_ready }
                }
            }
            State::Chase => {
                let side = if t.x >= self.x { 1.0 } else { -1.0 };
                let gap = MOUTH_GAP * u * rig::depth_scale(t.depth);
                let tx = clamp(t.x - side * gap);
                if self.go_to(tx, t.depth, run, now, dt, u) {
                    match self.pet {
                        Pet::Dog => {
                            self.show_anim(Anim::Eat, now);
                            State::Pickup { until: now + PICKUP }
                        }
                        Pet::Rabbit => State::Return { to: cursor_x },
                    }
                } else {
                    State::Chase
                }
            }
            State::Pickup { until } => {
                if now >= until {
                    t.in_mouth = true;
                    State::Return { to: cursor_x }
                } else {
                    State::Pickup { until }
                }
            }
            State::Return { to } => {
                let arrived = self.go_to(to, 0.0, back, now, dt, u);
                let s = rig::depth_scale(self.depth);
                if t.in_mouth {
                    (t.x, t.depth, t.lift) = (self.x, self.depth, 0.0);
                } else {
                    (t.x, t.depth, t.lift) = (self.x + self.facing as f64 * NOSE_GAP * u * s, self.depth, 0.0);
                }
                if arrived {
                    t.in_mouth = false;
                    (t.x, t.depth, t.lift) = (clamp(self.x + self.facing as f64 * READY_GAP * u), 0.0, 0.0);
                    self.show_anim(Anim::Sit, now);
                    State::Ready
                } else {
                    State::Return { to }
                }
            }
            State::Ready => {
                // walk over to the toy if it isn't right in front, then sit
                let side = if t.x >= self.x { 1.0 } else { -1.0 };
                let tx = clamp(t.x - side * READY_GAP * u);
                if (tx - self.x).abs() > 0.3 * u || self.depth > 0.0 {
                    self.go_to(tx, 0.0, Anim::Walk, now, dt, u);
                } else {
                    self.facing = side as i8;
                    if self.anim != Anim::Sit {
                        self.show_anim(Anim::Sit, now);
                    }
                }
                if grab {
                    State::Held
                } else {
                    State::Ready
                }
            }
        };
        self.props.toy = Some(t);
        self.mode = Mode::Play(Play::Fetch { toy, st });
    }
}
