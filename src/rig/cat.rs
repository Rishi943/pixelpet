//! The cat species: the dog's skeleton and gaits (a cat walks, trots and
//! sits much like a small dog), with its own head, ears, tail carriage,
//! coat and eyes. Mellow's coat, sampled from his photos; Oreo is a design
//! over it (breeds.rs).

use super::core::*;
use super::dog::{self, Body, Skel, View};
use super::{Anim, Toy};

// ---------------------------------------------------------------- coat

/// Mellow: a long-haired cream cat with copper eyes.
pub static MELLOW: Coat = Coat {
    fams: &[
        (fam::TAN, [0x8E5E34, 0xC28C58, 0xE4B482, 0xF4D3A8]),
        (fam::CREAM, [0xB48E66, 0xDCBC96, 0xF2DDBF, 0xFCF1E0]),
        (fam::EAR_IN, [0x9A5A52, 0xC27C70, 0xE2A29A, 0xF4C6C0]),
        (fam::NOSE, [0x8A4650, 0xB8646E, 0xE08C96, 0xF4B2B8]),
        (fam::MOUTH, [0x22080B, 0x341015, 0x46141B, 0x5E1E26]),
        (fam::COLLAR, [0x132C66, 0x1E4494, 0x2F64C8, 0x5E8FE6]),
    ],
    eye: 0x120A06,
    iris: 0xE8862A,
    outline: 0x34200F,
    edge_drop: 1,
};

// ---------------------------------------------------------------- body

/// The look's dog body reshaped into a cat (shorter legs and muzzle, small
/// upright ears, a long tail, long hair), then a pet's shape multipliers.
pub fn body_for(look: &Look, s: &super::design::Shape) -> Body {
    let b = dog::body_for(look, s);
    Body {
        leg_len: b.leg_len * 0.80,
        body_len: b.body_len * 0.92,
        head: b.head,
        ear: b.ear * 0.90,
        muzzle: b.muzzle * 0.55,
        tail: b.tail * 1.45,
        fluff: b.fluff * 1.15,
        limb: b.limb * 0.88,
    }
}

// ---------------------------------------------------------------- anims

pub fn pose_with(anim: Anim, b: &Body, t: f64, toy: Option<Toy>) -> Skel {
    let mut s = dog::pose_with(anim, b, t, toy);
    // no panting: the mouth stays shut unless it holds a toy or eats
    s.tongue = 0.0;
    if anim == Anim::Eat {
        s.mouth *= 0.5;
    } else if s.toy.is_none() {
        s.mouth = 0.0;
    }
    s.ear_back *= 0.5;
    if let Some(angs) = tail_angles(anim, t) {
        s.tail = tail_chain(s.tail[0], angs, 0.19 * b.tail);
        if matches!(anim, Anim::Sit | Anim::Tilt | Anim::Lean | Anim::Watch) {
            // wrapped round the paws, lying on the ground
            for p in &mut s.tail[1..] {
                p[1] = js_max(p[1], PAW_Y + 0.04);
            }
        }
    }
    s
}

/// The tail's four segment angles (0 = forward, PI/2 = up), or None to keep
/// the dog's (lying asleep, and the views from the front and behind).
fn tail_angles(anim: Anim, t: f64) -> Option<[f64; 4]> {
    let sway = |i: f64, speed: f64, amp: f64| amp * (t * TAU * speed + 0.6 * i).sin() * (i + 1.0) * 0.5;
    let mut a = match anim {
        // up, with the tip curled forward
        Anim::Stand | Anim::Walk | Anim::Hop | Anim::Carry | Anim::Stretch | Anim::Tug | Anim::Nudge => [2.3, 1.85, 1.5, 1.0],
        // happy: straight up, quivering
        Anim::Wag => [1.95, 1.72, 1.6, 1.3],
        Anim::Run => [2.75, 2.95, 3.05, 2.95],
        // down and round to the front, along the ground
        Anim::Sit | Anim::Tilt | Anim::Lean | Anim::Watch => [-1.95, -0.55, -0.05, 0.55],
        Anim::Eat => [-2.85, -3.05, 3.1, 2.75],
        Anim::Sleep | Anim::Flop | Anim::RunAway | Anim::RunBack | Anim::CarryBack => return None,
    };
    let (speed, amp) = match anim {
        Anim::Wag => (6.0, 0.03),
        Anim::Run | Anim::Walk | Anim::Carry => (1.2, 0.07),
        Anim::Sit | Anim::Tilt | Anim::Lean | Anim::Watch => (0.5, 0.04),
        _ => (0.6, 0.06),
    };
    for (i, x) in a.iter_mut().enumerate() {
        *x += sway(i as f64, speed, amp);
    }
    Some(a)
}

fn tail_chain(base: P, angs: [f64; 4], seg: f64) -> [P; 5] {
    let mut pts = [base; 5];
    let mut p = base;
    for (i, a) in angs.iter().enumerate() {
        p = add(p, [a.cos() * seg, a.sin() * seg]);
        pts[i + 1] = p;
    }
    pts
}

// ---------------------------------------------------------------- shapes

/// The dog's body and legs, with a cat's head and tail in their places.
pub fn shapes(s: &Skel, b: &Body, look: &Look) -> Vec<Shape> {
    let mut out = Vec::with_capacity(40);
    let (mut head, mut tail) = (false, false);
    for sh in dog::shapes(s, b, look) {
        // the dog's far ear comes first, behind the body: the cat's is part of its head
        if sh.part == Part::EarF && sh.grp != 6 {
            continue;
        }
        if sh.grp == 6 || sh.part == Part::Tongue {
            if !head {
                head_shapes(&mut out, s, b, look);
                head = true;
            }
            continue;
        }
        if sh.part == Part::Tail {
            if !tail {
                tail_shapes(&mut out, s, b, look);
                tail = true;
            }
            continue;
        }
        out.push(sh);
    }
    out
}

fn tail_shapes(out: &mut Vec<Shape>, s: &Skel, b: &Body, look: &Look) {
    let f = b.fluff;
    let r = |x: f64| js_max(x, look.min_r / look.r);
    // bushy all the way, a little thinner at the root and the tip
    let tr = [0.05, 0.065, 0.07, 0.068, 0.05];
    for i in 0..4 {
        let sh = cap(s.tail[i], s.tail[i + 1], r(tr[i] * f), r(tr[i + 1] * f), Part::Tail).idx(i as u8).grp(5);
        out.push(if s.view == View::Side { sh.flip() } else { sh });
    }
}

fn head_shapes(out: &mut Vec<Shape>, s: &Skel, b: &Body, look: &Look) {
    let from = out.len();
    let r = |x: f64| js_max(x, look.min_r / look.r);
    let (f, eb) = (b.fluff, s.ear_back);
    let up = |k: f64| k * b.ear * (1.0 - 0.3 * eb);
    if s.view == View::Back {
        let hs = b.head;
        let hp = |p: P| add(s.head, mul(p, hs));
        for sgn in [-1.0, 1.0] {
            out.push(tri(hp([0.14 * sgn, 0.04]), hp([0.04 * sgn, 0.12]), hp([0.14 * sgn, 0.12 + up(0.17)]), Part::EarF));
        }
        out.push(ell(s.head, 0.16 * hs * f, 0.14 * hs, 0.0, Part::BackHead));
    } else if s.head_front {
        let hs = b.head * 1.15;
        let ang = s.tilt;
        let hp = |p: P| add(s.head, rot(mul(p, hs), ang));
        for sgn in [-1.0, 1.0] {
            out.push(tri(hp([0.16 * sgn, 0.05]), hp([0.04 * sgn, 0.12]), hp([(0.15 + 0.04 * eb) * sgn, 0.12 + up(0.19)]), Part::EarFront));
        }
        // the ruff: wide fluffy cheeks under a round head
        out.push(ell(hp([0.0, -0.06]), 0.20 * hs * f, 0.11 * hs * f, ang, Part::FCheek));
        out.push(ell(hp([0.0, 0.01]), 0.165 * hs, 0.145 * hs, ang, Part::FSkull));
        out.push(ell(hp([0.0, -0.065]), 0.065 * hs, 0.045 * hs, ang, Part::FMuzzle));
        if s.mouth > 0.08 {
            out.push(ell(hp([0.0, -0.10]), 0.03 * hs, 0.02 * hs * s.mouth + 0.01, ang, Part::Mouth));
        }
        out.push(ell(hp([0.0, -0.04]), r(0.025 * hs), r(0.018 * hs), ang, Part::Nose));
    } else {
        let hs = b.head;
        let ang = s.head_rot;
        let hp = |p: P| add(s.head, rot(mul(p, hs), ang));
        // muzzle length: 1 for the built-in cats
        let m = b.muzzle / (0.94 * 0.55);
        out.push(tri(hp([-0.14, 0.07]), hp([-0.03, 0.12]), hp([-0.11 - 0.05 * eb, 0.12 + up(0.16)]), Part::EarF).far(true));
        out.push(ell(hp([-0.05, -0.07]), 0.15 * hs * f, 0.11 * hs * f, ang, Part::Jaw));
        out.push(ell(hp([0.0, 0.0]), 0.165 * hs, 0.145 * hs, ang, Part::Skull));
        if s.mouth > 0.08 {
            out.push(tri(hp([0.07, -0.08]), hp([0.15, -0.075]), hp([0.12, -0.08 - 0.07 * s.mouth]), Part::Mouth));
        }
        out.push(ell(hp([0.10 + 0.03 * m, -0.055]), 0.07 * hs * m.sqrt(), 0.05 * hs, ang, Part::Muzzle));
        out.push(ell(hp([0.165 + 0.04 * (m - 1.0), -0.03]), r(0.022 * hs), r(0.018 * hs), ang, Part::Nose));
        out.push(tri(hp([-0.07, 0.11]), hp([0.07, 0.12]), hp([-0.01 - 0.06 * eb, 0.12 + up(0.19)]), Part::EarN));
    }
    for sh in &mut out[from..] {
        sh.grp = 6;
    }
}

// ---------------------------------------------------------------- markings

/// Coat markings: a lighter chest, belly, muzzle and paws; pink inner ears.
pub fn paint(s: &Shape, _x: f64, _y: f64, look: &Look, nz: f64, hh: &Hit) -> i8 {
    use fam::*;
    let fr = if look.detail != 0 { nz * 0.10 * look.detail as f64 } else { 0.0 };
    let (ux, uy, v, t) = (hh.ux, hh.uy, hh.v, hh.t);
    match s.part {
        Part::Torso => if v < -0.5 + fr { CREAM } else { TAN },
        Part::Rump => if uy < -0.55 + fr { CREAM } else { TAN },
        Part::Chest => if ux > 0.1 && uy < 0.3 + fr { CREAM } else { TAN },
        Part::Brisket => if uy < 0.4 + fr { CREAM } else { TAN },
        Part::Neck => {
            if s.collar && t > 0.5 && t < 0.68 {
                COLLAR
            } else if v < -0.2 + fr {
                CREAM
            } else {
                TAN
            }
        }
        Part::Jaw => if uy < 0.0 + fr { CREAM } else { TAN },
        Part::Muzzle | Part::FMuzzle | Part::Paw => CREAM,
        Part::Nose => NOSE,
        Part::Mouth => MOUTH,
        Part::EarN | Part::EarFront => {
            if look.detail == 0 || hh.e0 < 0.035 || hh.e1 < 0.03 || hh.h > 0.75 {
                TAN
            } else {
                EAR_IN
            }
        }
        Part::FCheek => if ux.abs() < 0.45 && uy < 0.3 + fr { CREAM } else { TAN },
        Part::Ball | Part::Coco | Part::FBall => paint_toy(s, hh),
        _ => TAN,
    }
}

/// Big round eyes: the iris colour round a dark pupil.
pub fn eyes(out: &mut [u8], s: &Skel, b: &Body, look: &Look, fs: FrameSize, pal: &Palette) {
    if s.view == View::Back {
        return;
    }
    let hs = b.head * if s.head_front { 1.15 } else { 1.0 };
    let ang = if s.head_front { s.tilt } else { s.head_rot };
    let pts: &[P] = if s.head_front { &[[-0.075, 0.02], [0.075, 0.02]] } else { &[[0.085, 0.025]] };
    for p in pts {
        let (ix, iy) = px_of(fs, look, add(s.head, rot(mul(*p, hs), ang)));
        if s.eyes > 0.5 {
            put(out, fs, ix, iy, pal.iris);
            put(out, fs, ix, iy - 1, pal.iris);
            put(out, fs, if s.head_front { ix } else { ix + 1 }, if s.head_front { iy - 1 } else { iy }, pal.eye);
            if !s.head_front {
                put(out, fs, ix + 1, iy - 1, pal.eye);
            }
        } else {
            put(out, fs, ix, iy, pal.eye);
            put(out, fs, ix + 1, iy, pal.eye);
        }
    }
}

/// Where food or a toy meets the mouth, in rig units.
pub fn mouth(s: &Skel, b: &Body) -> P {
    dog::mouth(s, b)
}

/// Units per second for the travelling anims (none of them slide).
pub fn speed(b: &Body, anim: Anim) -> f64 {
    let walk = dog::walk_speed(b);
    match anim {
        Anim::Walk | Anim::Nudge => walk,
        Anim::Run | Anim::RunAway | Anim::RunBack => dog::RUN_SPEEDUP * walk,
        Anim::Carry | Anim::CarryBack => dog::CARRY_SPEEDUP * walk,
        _ => 0.0,
    }
}
