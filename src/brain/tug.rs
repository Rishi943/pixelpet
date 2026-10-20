//! Tug-of-war (spec §5): a rope from the pet's mouth to the cursor. The pet
//! pulls in bursts that move the real cursor a little towards it; the user
//! pulls back by moving the mouse away. Held far for 2 s, the user wins (a
//! happy jump); dragged onto the pet, the pet wins (a victory shake). Where
//! the cursor can't be moved, the same rules apply by distance.

use super::{Brain, Inputs, Mode, Play};
use crate::platform::Rect;
use crate::rig::{self, Anim, Pet};

const BURST: f64 = 0.35;
/// Rig units the cursor is pulled over one burst.
const PULL: f64 = 0.6;
/// Rig units from the mouth the user must hold the cursor, for WIN_HOLD s.
const WIN_DIST: f64 = 5.0;
const WIN_HOLD: f64 = 2.0;
const VICTORY: f64 = 1.2;
/// No pull from the user for this long ends it with no winner.
const UNTOUCHED: f64 = 20.0;
/// Cursor movement under this (rig units) is not the user pulling.
const STILL: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    next_burst: f64,
    /// (start time, unit vector towards the mouth, px pulled so far)
    burst: Option<(f64, (f64, f64), f64)>,
    far_since: Option<f64>,
    /// where the cursor was when the user last moved it (our pulls included)
    anchor: Option<(f64, f64)>,
    still_since: f64,
    /// someone won: the anim plays until then, then play ends
    end: Option<f64>,
    /// the cursor has been off the pet since tug began (it starts on the
    /// bubble, right over the pet, which still counts as on it)
    armed: bool,
}

impl State {
    pub fn new(now: f64) -> State {
        State { next_burst: now, burst: None, far_since: None, anchor: None, still_since: now, end: None, armed: false }
    }
}

impl Brain {
    /// The mouth, as tug measures distance from it (screen px). The app
    /// draws the rope from the rig's own mouth point; this is close to it.
    pub fn tug_mouth(&self, unit_px: f64, work: Rect) -> (f64, f64) {
        let ground = work.bottom() as f64;
        let (fwd, up) = match self.pet {
            Pet::Dog => (0.5, 0.85),
            Pet::Rabbit => (0.45, 0.5),
        };
        (self.x + self.facing as f64 * fwd * unit_px, ground - up * unit_px)
    }

    pub(super) fn tug(&mut self, mut st: State, inp: &Inputs, now: f64) {
        let u = inp.unit_px;
        if let Some(until) = st.end {
            if now >= until {
                return self.end_play(now);
            }
            self.mode = Mode::Play(Play::Tug { st });
            return;
        }
        if st.next_burst <= now && st.burst.is_none() && st.anchor.is_none() {
            // first tick: face the cursor, first burst a moment later
            if let Some((cx, _)) = inp.cursor {
                self.facing = if cx as f64 >= self.x { 1 } else { -1 };
            }
            st.next_burst = now + self.rng.range(0.9, 1.6);
        }
        if self.anim != Anim::Tug {
            self.show_anim(Anim::Tug, now);
        }
        let Some((cx, cy)) = inp.cursor else {
            self.mode = Mode::Play(Play::Tug { st });
            return;
        };
        let c = (cx as f64, cy as f64);
        let (mx, my) = self.tug_mouth(u, inp.work);
        let dist = ((c.0 - mx).powi(2) + (c.1 - my).powi(2)).sqrt();

        // dragged onto the pet: the pet wins
        if !inp.over_pet {
            st.armed = true;
        }
        if inp.over_pet && st.armed {
            self.props.rope_to = None;
            st.end = Some(now + VICTORY);
            self.mode = Mode::Play(Play::Tug { st });
            return;
        }
        // held far enough for long enough: the user wins
        if dist > WIN_DIST * u {
            let since = *st.far_since.get_or_insert(now);
            if now - since >= WIN_HOLD {
                self.props.rope_to = None;
                self.show_anim(Anim::Hop, now);
                st.end = Some(now + rig::period(self.pet, Anim::Hop));
                self.mode = Mode::Play(Play::Tug { st });
                return;
            }
        } else {
            st.far_since = None;
        }
        // the user pulling, or not
        let anchor = *st.anchor.get_or_insert(c);
        if ((c.0 - anchor.0).powi(2) + (c.1 - anchor.1).powi(2)).sqrt() > STILL * u {
            st.anchor = Some(c);
            st.still_since = now;
        } else if now - st.still_since >= UNTOUCHED {
            return self.end_play(now);
        }
        // the pet pulling, in bursts
        if st.burst.is_none() && now >= st.next_burst {
            let len = dist.max(1.0);
            st.burst = Some((now, ((mx - c.0) / len, (my - c.1) / len), 0.0));
        }
        let mut cursor = (cx, cy);
        if let Some((t0, dir, done)) = st.burst {
            let p = ((now - t0) / BURST).min(1.0);
            let want = PULL * u * p;
            if inp.can_warp {
                let step = |k: f64| ((dir.0 * k).round() as i32, (dir.1 * k).round() as i32);
                let (a, b) = (step(want), step(done));
                let d = (a.0 - b.0, a.1 - b.1);
                if d != (0, 0) {
                    cursor = (cx + d.0, cy + d.1);
                    self.warp = Some(cursor);
                    // our own pull is not the user moving
                    st.anchor = st.anchor.map(|(x, y)| (x + d.0 as f64, y + d.1 as f64));
                }
            }
            st.burst = if p >= 1.0 {
                st.next_burst = now + self.rng.range(0.9, 1.6);
                None
            } else {
                Some((t0, dir, want))
            };
        }
        self.props.rope_to = Some(cursor);
        self.props.taut = st.burst.is_some();
        self.mode = Mode::Play(Play::Tug { st });
    }
}
