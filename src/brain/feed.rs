//! Feed (spec §5): the food drops from the bubble to the ground; the pet
//! walks over, eats it in three bites with crumbs, then is happy: Alex wags
//! and licks his lips, the bun does a binky.

use super::{Brain, FoodShow, Inputs, Mode, Play, GRAVITY};
use crate::rig::{self, Anim, Food, Pet};

/// The food lands this many rig units to the side of the pet.
const LAND_GAP: f64 = 1.5;
/// The mouth stops this far short of the food (rig units).
const MOUTH_GAP: f64 = 0.45;
const EAT: f64 = 2.4;
const BITE: f64 = 0.6;
const WAG: f64 = 1.6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Dropping from the bubble; `side` is which side of the pet it lands.
    Falling { v: f64, side: f64, landed_x: Option<f64> },
    Walk,
    Eat { t0: f64 },
    React { until: f64 },
}

impl Brain {
    pub(super) fn feed(&mut self, food: Food, st: State, inp: &Inputs, now: f64, dt: f64, lo: f64, hi: f64) {
        let u = inp.unit_px;
        let clamp = |x: f64| x.clamp(lo, hi.max(lo));
        // the reaction comes after the food is gone
        if let State::React { until } = st {
            if now >= until {
                return self.end_play(now);
            }
            self.mode = Mode::Play(Play::Feed { food, st });
            return;
        }
        let Some(mut f) = self.props.food else { return self.end_play(now) };
        let st = match st {
            State::Falling { v, side, mut landed_x } => {
                // lands beside the pet, kept on the ground (and on screen)
                let x = *landed_x.get_or_insert_with(|| {
                    let x = self.x + side * LAND_GAP * u;
                    if x < lo || x > hi {
                        self.x - side * LAND_GAP * u
                    } else {
                        x
                    }
                });
                f.x = clamp(x);
                let v = v + GRAVITY * dt;
                f.lift = (f.lift - v * dt).max(0.0);
                if f.lift <= 0.0 {
                    State::Walk
                } else {
                    State::Falling { v, side, landed_x: Some(x) }
                }
            }
            State::Walk => {
                let side = if f.x >= self.x { 1.0 } else { -1.0 };
                let tx = clamp(f.x - side * MOUTH_GAP * u);
                let arrived = (tx - self.x).abs() < 1.0 || self.go_to(tx, 0.0, Anim::Walk, now, dt, u);
                if arrived {
                    self.facing = side as i8;
                    self.show_anim(Anim::Eat, now);
                    State::Eat { t0: now }
                } else {
                    State::Walk
                }
            }
            State::Eat { t0 } => {
                let bites = ((now - t0) / BITE).floor().clamp(0.0, 3.0) as u8;
                if bites > f.bites {
                    self.props.crumbs = Some(now);
                }
                f.bites = bites;
                if now - t0 >= EAT {
                    self.props.food = None;
                    let (anim, secs) = match self.pet {
                        Pet::Dog => (Anim::Wag, WAG),
                        Pet::Rabbit => (Anim::Hop, rig::period(Pet::Rabbit, Anim::Hop)),
                    };
                    self.show_anim(anim, now);
                    self.mode = Mode::Play(Play::Feed { food, st: State::React { until: now + secs } });
                    return;
                }
                State::Eat { t0 }
            }
            State::React { .. } => st,
        };
        self.props.food = Some(f);
        self.mode = Mode::Play(Play::Feed { food, st });
    }
}

/// The food as it first appears: at `from` (screen px), above the ground.
pub fn dropped(food: Food, from: (i32, i32), ground: f64) -> FoodShow {
    FoodShow { food, x: from.0 as f64, lift: (ground - from.1 as f64).max(0.0), bites: 0 }
}
