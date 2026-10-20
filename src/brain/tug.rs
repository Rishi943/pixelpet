//! Tug-of-war (spec §5): a rope of fixed length from the pet's mouth to the
//! cursor. Pull the cursor away and the pet is dragged towards you; every
//! second or so the pet braces and steps back, dragging the real cursor with
//! it. Drag the pet 2.5 units your way and you win (a happy jump); let it
//! back 2.5 units and it wins (a victory shake); let the cursor reach the
//! pet and you've given up. Where the cursor can't be moved the pet's step
//! back stretches the rope instead.

use super::{Brain, Inputs, Mode, Play};
use crate::platform::Rect;
use crate::rig::{self, Anim, Pet};

const BURST: f64 = 0.35;
/// Rig units the pet steps back over one burst.
const PULL: f64 = 0.6;
/// The rope's length: the starting distance, kept within these (rig units).
const ROPE_MIN: f64 = 2.0;
const ROPE_MAX: f64 = 4.0;
/// Rig units either side of the start that win it.
const WIN: f64 = 2.5;
const VICTORY: f64 = 1.2;
/// No input from the user for this long ends it with no winner (a
/// stalemate, say with the pet backed against the screen edge).
const UNTOUCHED: f64 = 20.0;
/// Cursor movement under this (rig units) is not the user pulling.
const STILL: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    next_burst: f64,
    /// (start time, px stepped back so far, px the cursor was moved so far)
    burst: Option<(f64, f64, i32)>,
    /// set on the first tick: where the pet started, and the rope's length (px)
    start_x: Option<f64>,
    rope: f64,
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
        State { next_burst: now, burst: None, start_x: None, rope: 0.0, anchor: None, still_since: now, end: None, armed: false }
    }
}

impl Brain {
    /// The mouth, as tug measures the rope from it (screen px). The app
    /// draws the rope from the rig's own mouth point; this is close to it.
    pub fn tug_mouth(&self, unit_px: f64, work: Rect) -> (f64, f64) {
        let ground = work.bottom() as f64;
        let (fwd, up) = match self.pet {
            Pet::Dog => (0.5, 0.85),
            Pet::Rabbit => (0.45, 0.5),
        };
        (self.x + self.facing as f64 * fwd * unit_px, ground - up * unit_px)
    }

    fn rope_dist(&self, c: (f64, f64), u: f64, work: Rect) -> f64 {
        let (mx, my) = self.tug_mouth(u, work);
        ((c.0 - mx).powi(2) + (c.1 - my).powi(2)).sqrt()
    }

    pub(super) fn tug(&mut self, mut st: State, inp: &Inputs, now: f64, lo: f64, hi: f64) {
        let u = inp.unit_px;
        let clamp = |x: f64| x.clamp(lo, hi.max(lo));
        if let Some(until) = st.end {
            if now >= until {
                return self.end_play(now);
            }
            self.mode = Mode::Play(Play::Tug { st });
            return;
        }
        if self.anim != Anim::Tug {
            self.show_anim(Anim::Tug, now);
        }
        let Some((cx, cy)) = inp.cursor else {
            self.mode = Mode::Play(Play::Tug { st });
            return;
        };
        let mut c = (cx as f64, cy as f64);
        if st.start_x.is_none() {
            // face the cursor; the rope is as long as it is now
            self.facing = if c.0 >= self.x { 1 } else { -1 };
            st.start_x = Some(self.x);
            st.rope = self.rope_dist(c, u, inp.work).clamp(ROPE_MIN * u, ROPE_MAX * u);
            st.next_burst = now + self.rng.range(0.9, 1.6);
        }
        let start = st.start_x.unwrap();
        let toward = self.facing as f64; // the cursor's side
        let finish = |b: &mut Brain, mut st: State, anim: Anim, secs: f64| {
            b.props.rope_to = None;
            b.props.taut = false;
            if b.anim != anim {
                b.show_anim(anim, now);
            }
            st.end = Some(now + secs);
            b.mode = Mode::Play(Play::Tug { st });
        };

        // the cursor onto the pet: the user has given up
        if !inp.over_pet {
            st.armed = true;
        }
        if inp.over_pet && st.armed {
            return finish(self, st, Anim::Tug, VICTORY);
        }
        // the user pulling: past the rope's length drags the pet along
        let dist = self.rope_dist(c, u, inp.work);
        if dist > st.rope + 0.5 {
            let (mx, my) = self.tug_mouth(u, inp.work);
            let dy = (c.1 - my).abs().min(st.rope);
            let reach = (st.rope * st.rope - dy * dy).sqrt();
            let want_mx = c.0 - toward * reach;
            self.x = clamp(self.x + (want_mx - mx));
        }
        if (self.x - start) * toward >= WIN * u {
            self.show_anim(Anim::Hop, now);
            return finish(self, st, Anim::Hop, rig::period(self.pet, Anim::Hop));
        }
        // the user moving the mouse at all (our own pulls aside)
        let anchor = *st.anchor.get_or_insert(c);
        if ((c.0 - anchor.0).powi(2) + (c.1 - anchor.1).powi(2)).sqrt() > STILL * u {
            st.anchor = Some(c);
            st.still_since = now;
        } else if now - st.still_since >= UNTOUCHED {
            return self.end_play(now);
        }
        // the pet pulling: in bursts it steps back, and the cursor comes too
        if st.burst.is_none() && now >= st.next_burst {
            st.burst = Some((now, 0.0, 0));
        }
        if let Some((t0, done, moved)) = st.burst {
            let p = ((now - t0) / BURST).min(1.0);
            let want = PULL * u * p;
            let before = self.x;
            self.x = clamp(self.x - toward * (want - done));
            let stepped = (before - self.x).abs();
            let mut moved = moved;
            if stepped > 0.0 {
                let taut = self.rope_dist(c, u, inp.work) >= st.rope - 2.0;
                if inp.can_warp && taut {
                    // keep the rope its length: the cursor follows the pet
                    let total = (done + stepped).round() as i32;
                    let d = total - moved;
                    if d != 0 {
                        c.0 -= toward * d as f64;
                        moved = total;
                        self.warp = Some((c.0.round() as i32, cy));
                        st.anchor = st.anchor.map(|(x, y)| (x - toward * d as f64, y));
                    }
                } else if !inp.can_warp {
                    // the cursor can't follow: the rope stretches
                    st.rope = st.rope.max(self.rope_dist(c, u, inp.work));
                }
            }
            st.burst = if p >= 1.0 {
                st.next_burst = now + self.rng.range(0.9, 1.6);
                None
            } else {
                Some((t0, want, moved))
            };
        }
        if (start - self.x) * toward >= WIN * u - 1.0 {
            return finish(self, st, Anim::Tug, VICTORY);
        }
        self.props.rope_to = Some((c.0.round() as i32, cy));
        self.props.taut = st.burst.is_some();
        self.mode = Mode::Play(Play::Tug { st });
    }
}
