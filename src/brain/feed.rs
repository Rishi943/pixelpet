//! Feed (spec §5, from the hand): the food follows the cursor as if held
//! out. The pet comes over; once the food is in reach it eats, pulling the
//! cursor (and the food) slowly in to its mouth, in three bites with
//! crumbs. Pull the food away and it stops and follows. Then it is happy:
//! Alex wags and licks his lips, the bun does a binky. Where the cursor
//! can't be moved, the user holds the food to the mouth.

use super::{Brain, FoodShow, Inputs, Mode, Play};
use crate::platform::Rect;
use crate::rig::{self, Anim, Food, Pet};

/// Rig units from the mouth within which the pet reaches for the food, and
/// beyond which a meal stops (the food was pulled away).
const REACH: f64 = 1.6;
const LOSE: f64 = 2.2;
/// The food is "at the mouth" within this (rig units): bites happen then.
const AT_MOUTH: f64 = 0.4;
/// How fast the pet pulls the food in (rig units per second).
const PULL_SPEED: f64 = 1.2;
/// Eating time at the mouth, and one bite per BITE seconds of it.
const EAT: f64 = 2.4;
const BITE: f64 = 0.6;
const WAG: f64 = 1.6;
/// The pet stands this far behind the point under the food (rig units).
const STAND_OFF: f64 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Coming over to the food, or waiting for it to come in reach.
    Follow,
    /// Eating: seconds eaten so far with the food at the mouth.
    Eat { eaten: f64 },
    React { until: f64 },
}

impl Brain {
    /// The mouth while eating (head down at the food), screen px.
    pub fn eat_mouth(&self, unit_px: f64, work: Rect) -> (f64, f64) {
        let ground = work.bottom() as f64;
        let (fwd, up) = match self.pet {
            Pet::Dog => (0.8, 0.3),
            Pet::Cat => (0.7, 0.25),
            Pet::Rabbit => (0.6, 0.25),
        };
        (self.x + self.facing as f64 * fwd * unit_px, ground - up * unit_px)
    }

    pub(super) fn feed(&mut self, food: Food, st: State, inp: &Inputs, now: f64, dt: f64, lo: f64, hi: f64) {
        let u = inp.unit_px;
        let ground = inp.work.bottom() as f64;
        let clamp = |x: f64| x.clamp(lo, hi.max(lo));
        if let State::React { until } = st {
            if now >= until {
                return self.end_play(now);
            }
            self.mode = Mode::Play(Play::Feed { food, st });
            return;
        }
        let Some(mut f) = self.props.food else { return self.end_play(now) };
        let Some((cx, cy)) = inp.cursor else {
            self.mode = Mode::Play(Play::Feed { food, st });
            return;
        };
        // the food is in the hand
        let mut c = (cx as f64, cy as f64);
        (f.x, f.lift) = (c.0, (ground - c.1).max(0.0));
        let (mx, my) = self.eat_mouth(u, inp.work);
        let dist = ((c.0 - mx).powi(2) + (c.1 - my).powi(2)).sqrt();
        let st = match st {
            State::Follow => {
                // stand just behind the point under the food, facing it
                let side = if c.0 >= self.x { 1.0 } else { -1.0 };
                let tx = clamp(c.0 - side * STAND_OFF * u);
                let there = (tx - self.x).abs() <= 0.15 * u || self.go_to(tx, 0.0, Anim::Walk, now, dt, u);
                if there {
                    self.facing = side as i8;
                    if self.anim != Anim::Stand {
                        self.show_anim(Anim::Stand, now);
                    }
                }
                if there && dist <= REACH * u {
                    self.show_anim(Anim::Eat, now);
                    State::Eat { eaten: 0.0 }
                } else {
                    State::Follow
                }
            }
            State::Eat { eaten } => {
                if dist > LOSE * u {
                    // pulled away: stop and follow
                    State::Follow
                } else {
                    let mut eaten = eaten;
                    self.engaged_at = now;
                    if dist <= AT_MOUTH * u {
                        eaten += dt;
                    } else if inp.can_warp {
                        // pull it in, slowly
                        let step = (PULL_SPEED * u * dt).min(dist);
                        let (dx, dy) = ((mx - c.0) / dist * step, (my - c.1) / dist * step);
                        c = (c.0 + dx, c.1 + dy);
                        self.warp = Some((c.0.round() as i32, c.1.round() as i32));
                        (f.x, f.lift) = (c.0.round(), (ground - c.1.round()).max(0.0));
                    }
                    let bites = (eaten / BITE).floor().clamp(0.0, 3.0) as u8;
                    if bites > f.bites {
                        self.props.crumbs = Some(now);
                    }
                    f.bites = bites;
                    if eaten >= EAT {
                        self.props.food = None;
                        let (anim, secs) = match self.pet {
                            Pet::Dog | Pet::Cat => (Anim::Wag, WAG),
                            Pet::Rabbit => (Anim::Hop, rig::period(Pet::Rabbit, Anim::Hop)),
                        };
                        self.show_anim(anim, now);
                        self.mode = Mode::Play(Play::Feed { food, st: State::React { until: now + secs } });
                        return;
                    }
                    State::Eat { eaten }
                }
            }
            State::React { .. } => st,
        };
        self.props.food = Some(f);
        self.mode = Mode::Play(Play::Feed { food, st });
    }
}

/// The food as it first appears: in the hand at the cursor.
pub fn in_hand(food: Food, at: (i32, i32), ground: f64) -> FoodShow {
    FoodShow { food, x: at.0 as f64, lift: (ground - at.1 as f64).max(0.0), bites: 0 }
}
