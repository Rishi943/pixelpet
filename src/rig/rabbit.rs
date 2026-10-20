//! The rabbit species and Alex the bun's coat, ported from
//! `drafts/looks/rabbit.js`. White "broken" coat with chestnut patches, a
//! brown butterfly face with a white blaze, long agouti ears with dark tips.
//! Units: head-top height when sitting in a loaf = ~0.9, y up, facing +x.

use super::core::*;
use super::{Anim, Toy};

pub static BUN: Coat = Coat {
    fams: &[
        (fam::WHITE, [0xA39A92, 0xD3CCC4, 0xF3F0EB, 0xFFFFFF]),
        (fam::BROWN, [0x4C3218, 0x70502C, 0x94693B, 0xB48852]), // agouti chestnut, not orange
        (fam::EAR_R, [0x54402C, 0x78603F, 0x9A8058, 0xB89C74]),
        (fam::PINK, [0x9E6F62, 0xBC8A7C, 0xD6A799, 0xE8C2B4]), // ear insides read pink-tan in the photos
        (fam::MASK, [0x20160F, 0x2E2118, 0x3F2E22, 0x584334]),
        (fam::BLACK, [0x0E0B0A, 0x171311, 0x221C19, 0x3A322E]),
        (fam::TAN, [0x5A381C, 0x86582E, 0xAC7B45, 0xD0A26C]),
    ],
    eye: 0x140C09,
    iris: 0xFFFFFF,
    outline: 0x2B1F17,
    edge_drop: 2, // white-on-white needs a firmer edge
};

/// Body sliders per look (1 = Alex the bun's real proportions).
pub struct Body {
    pub head: f64,
    pub ear: f64,
    pub fluff: f64,
    pub foot: f64,
    pub limb: f64,
}

pub fn body(look: &Look) -> Body {
    match look.name {
        "chunky" => Body { head: 1.38, ear: 0.86, fluff: 1.12, foot: 0.9, limb: 1.6 },
        "detailed" => Body { head: 1.0, ear: 1.0, fluff: 1.0, foot: 1.0, limb: 1.0 },
        _ => Body { head: 1.14, ear: 0.96, fluff: 1.05, foot: 1.0, limb: 1.25 },
    }
}

// ---------------------------------------------------------------- skeleton

/// All explicit points; poses are keyframes blended with `lerp`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skel {
    pub hip: P,
    pub sh: P,
    pub head: P,
    pub head_rot: f64,
    pub head_front: bool,
    pub tilt: f64,
    pub ear_n: f64,
    pub ear_f: f64,
    pub ear_sw: f64,
    pub nose: f64,
    pub eyes: f64,
    pub breath: f64,
    pub flat: f64,
    pub nf: [P; 2],
    pub ff: [P; 2],
    pub nh: [P; 2],
    pub fh: [P; 2],
    pub tail: P,
    pub toy: Option<Toy>,
}

fn mix2(a: &[P; 2], b: &[P; 2], k: f64) -> [P; 2] {
    [mix_p(a[0], b[0], k), mix_p(a[1], b[1], k)]
}

/// `lerpSkel`: numbers mix; `head_front`, `eyes` and `toy` switch at `k < 0.5`.
pub fn lerp(a: &Skel, b: &Skel, k: f64) -> Skel {
    let pick = k < 0.5;
    Skel {
        hip: mix_p(a.hip, b.hip, k),
        sh: mix_p(a.sh, b.sh, k),
        head: mix_p(a.head, b.head, k),
        head_rot: mix(a.head_rot, b.head_rot, k),
        head_front: if pick { a.head_front } else { b.head_front },
        tilt: mix(a.tilt, b.tilt, k),
        ear_n: mix(a.ear_n, b.ear_n, k),
        ear_f: mix(a.ear_f, b.ear_f, k),
        ear_sw: mix(a.ear_sw, b.ear_sw, k),
        nose: mix(a.nose, b.nose, k),
        eyes: if pick { a.eyes } else { b.eyes },
        breath: mix(a.breath, b.breath, k),
        flat: mix(a.flat, b.flat, k),
        nf: mix2(&a.nf, &b.nf, k),
        ff: mix2(&a.ff, &b.ff, k),
        nh: mix2(&a.nh, &b.nh, k),
        fh: mix2(&a.fh, &b.fh, k),
        tail: mix_p(a.tail, b.tail, k),
        toy: if pick { a.toy } else { b.toy },
    }
}

fn loaf(b: &Body) -> Skel {
    Skel {
        hip: [-0.22, 0.30],
        sh: [0.12, 0.42],
        head: [0.30, 0.70 + 0.12 * (b.head - 1.0)],
        head_rot: -0.05,
        head_front: false,
        tilt: 0.0,
        ear_n: 0.10,
        ear_f: 0.42,
        ear_sw: 0.0,
        nose: 0.0,
        eyes: 1.0,
        breath: 0.0,
        flat: 0.0,
        nf: [[0.17, 0.28], [0.25, 0.035]],
        ff: [[0.20, 0.29], [0.30, 0.04]],
        nh: [[-0.40, 0.055], [0.05, 0.045]],
        fh: [[-0.35, 0.06], [0.09, 0.05]],
        tail: [-0.49, 0.30],
        toy: None,
    }
}

/// keys: `(phase, skeleton)` sorted, `p` in [0, 1); smoothstep between keys.
fn blend(keys: &[(f64, Skel)], p: f64) -> Skel {
    for w in keys.windows(2) {
        let ((p0, a), (p1, b)) = (&w[0], &w[1]);
        if p >= *p0 && p <= *p1 {
            let d = p1 - p0;
            let u = (p - p0) / if d == 0.0 { 1.0 } else { d };
            return lerp(a, b, u * u * (3.0 - 2.0 * u));
        }
    }
    keys[keys.len() - 1].1
}

fn twitch(t: f64) -> f64 {
    if t % 2.6 < 1.1 {
        (t * 10.0).floor() % 2.0
    } else {
        0.0
    }
}

fn blink(t: f64) -> f64 {
    if t % 4.3 > 4.12 {
        0.0
    } else {
        1.0
    }
}

/// Spin the whole bun about `c` (for a binky).
fn rot_all(s: &Skel, ang: f64, c: P) -> Skel {
    let r = |q: P| add(c, rot(sub(q, c), ang));
    let r2 = |j: [P; 2]| [r(j[0]), r(j[1])];
    Skel {
        hip: r(s.hip),
        sh: r(s.sh),
        head: r(s.head),
        tail: r(s.tail),
        nf: r2(s.nf),
        ff: r2(s.ff),
        nh: r2(s.nh),
        fh: r2(s.fh),
        head_rot: s.head_rot + ang,
        ..*s
    }
}

// ---------------------------------------------------------------- anims

pub fn period(anim: Anim) -> f64 {
    match anim {
        Anim::Stand => 5.2,
        Anim::Walk => 0.62,
        Anim::Sit => 1.6,
        Anim::Tilt => 4.0,
        Anim::Hop => 1.2,
        Anim::Sleep => 9.6,
        Anim::Eat => 0.8,
        Anim::Flop => 4.0,
        Anim::Nudge => 0.7,
        Anim::Tug => 0.8,
        Anim::Stretch => 1.5,
        Anim::Watch => 6.0,
        // not rabbit anims: the poses they fall back to
        Anim::Run | Anim::Carry => 0.62,
        Anim::Wag | Anim::Lean => 5.2,
    }
}

/// Where a rope or food meets the mouth, in rig units.
pub fn mouth(s: &Skel, b: &Body) -> P {
    let hs = b.head * if s.head_front { 1.18 } else { 1.1 };
    if s.head_front {
        add(s.head, rot(mul([0.0, -0.09], hs), s.tilt))
    } else {
        add(s.head, rot(mul([0.23, -0.07], hs), s.head_rot))
    }
}

pub const WALK_SPEED: f64 = 0.72;

/// Hops forward only in the push-off part of each loop.
pub fn walk_move(p: f64) -> f64 {
    if p > 0.06 && p < 0.6 {
        1.0 / 0.54
    } else {
        0.0
    }
}

pub fn pose(anim: Anim, look: &Look, t: f64) -> Skel {
    let b = body(look);
    let l = loaf(&b);
    match anim {
        // loaf: nose going, ears turning now and then
        Anim::Stand => {
            let br = (t * TAU / 2.2).sin();
            let tm = t % 5.2;
            let ear = if tm > 3.6 && tm < 4.3 { -0.2 } else { 0.0 };
            Skel { nose: twitch(t), eyes: blink(t), breath: 0.015 * br, ear_n: l.ear_n + ear, ..l }
        }
        Anim::Walk => {
            let p = (t / 0.62) % 1.0;
            let push = Skel {
                hip: [-0.14, 0.40],
                sh: [0.26, 0.54],
                head: [0.47, 0.80 + 0.10 * (b.head - 1.0)],
                head_rot: -0.12,
                nf: [[0.32, 0.42], [0.50, 0.22]],
                ff: [[0.34, 0.44], [0.53, 0.25]],
                nh: [[-0.30, 0.30], [-0.62, 0.20]],
                fh: [[-0.26, 0.31], [-0.57, 0.22]],
                tail: [-0.40, 0.44],
                ear_n: 0.42,
                ear_f: 0.5,
                ..l
            };
            let land = Skel {
                hip: [0.0, 0.44],
                sh: [0.36, 0.33],
                head: [0.56, 0.57 + 0.10 * (b.head - 1.0)],
                head_rot: -0.25,
                nf: [[0.40, 0.22], [0.47, 0.035]],
                ff: [[0.43, 0.23], [0.52, 0.04]],
                nh: [[-0.12, 0.34], [0.12, 0.22]],
                fh: [[-0.08, 0.35], [0.16, 0.24]],
                tail: [-0.24, 0.55],
                ear_n: 0.32,
                ear_f: 0.4,
                ..l
            };
            blend(&[(0.0, l), (0.22, push), (0.42, land), (0.66, l), (1.0, l)], p)
        }
        // grooming: sits up and washes the face
        Anim::Sit => {
            let w = (t * TAU / 0.8).sin();
            Skel {
                hip: [-0.18, 0.30],
                sh: [0.04, 0.58],
                head: [0.16, 0.84 + 0.10 * (b.head - 1.0)],
                head_rot: -0.45,
                nf: [[0.10, 0.50], [0.27, 0.70 + 0.05 * w]],
                ff: [[0.12, 0.52], [0.29, 0.73 - 0.05 * w]],
                tail: [-0.44, 0.27],
                eyes: 0.0,
                ear_n: 0.45,
                ear_f: 0.55,
                ..l
            }
        }
        // stands up tall and looks at you
        Anim::Tilt => {
            let q = t % 4.0;
            let sw = if q > 1.2 && q < 1.8 {
                0.3
            } else if q > 2.8 && q < 3.3 {
                -0.3
            } else {
                0.0
            };
            Skel {
                hip: [-0.12, 0.30],
                sh: [0.0, 0.76],
                head: [0.05, 1.12 + 0.12 * (b.head - 1.0)],
                head_rot: 0.0,
                head_front: true,
                tilt: 0.06 * (t * TAU / 4.0).sin(),
                ear_sw: sw,
                nf: [[0.08, 0.66], [0.15, 0.53]],
                ff: [[0.11, 0.67], [0.18, 0.55]],
                nh: [[-0.32, 0.055], [0.10, 0.045]],
                fh: [[-0.27, 0.06], [0.14, 0.05]],
                tail: [-0.30, 0.24],
                nose: twitch(t),
                eyes: blink(t),
                ..l
            }
        }
        // binky: a happy leap with a twist
        Anim::Hop => {
            let p = (t / 1.2) % 1.0;
            let crouch = Skel { hip: [-0.22, 0.25], sh: [0.13, 0.36], head: [0.30, 0.60 + 0.10 * (b.head - 1.0)], ear_n: 0.35, ..l };
            let air = Skel {
                hip: [-0.18, 0.80],
                sh: [0.18, 0.92],
                head: [0.38, 1.12 + 0.10 * (b.head - 1.0)],
                head_rot: 0.1,
                nf: [[0.24, 0.80], [0.38, 0.66]],
                ff: [[0.27, 0.82], [0.41, 0.69]],
                nh: [[-0.30, 0.70], [-0.58, 0.82]],
                fh: [[-0.26, 0.72], [-0.52, 0.86]],
                tail: [-0.44, 0.84],
                ear_n: 0.7,
                ear_f: 0.85,
                ..l
            };
            let air = rot_all(&air, 0.32, [0.0, 0.85]);
            blend(&[(0.0, l), (0.15, crouch), (0.42, air), (0.7, crouch), (0.86, l), (1.0, l)], p)
        }
        // the sofa sploot: stretched flat, ears laid back
        Anim::Sleep => {
            let br = (t * TAU / 3.2).sin();
            let tm = t % 9.6;
            let tw = if tm > 7.6 && tm < 8.0 { 0.015 } else { 0.0 };
            Skel {
                hip: [-0.30, 0.16],
                sh: [0.16, 0.19],
                head: [0.43, 0.21 + 0.06 * (b.head - 1.0)],
                head_rot: -0.06,
                flat: 1.0,
                nf: [[0.30, 0.10], [0.62, 0.035]],
                ff: [[0.30, 0.13], [0.58, 0.06]],
                nh: [[-0.48, 0.09], [-0.86, 0.05 + tw]],
                fh: [[-0.45, 0.12], [-0.80, 0.08]],
                tail: [-0.52, 0.22],
                ear_n: 1.5,
                ear_f: 1.38,
                eyes: 0.0,
                breath: 0.03 * br,
                ..l
            }
        }
        // head down at the food, nose and jaw going
        Anim::Eat => {
            let chew = (t * TAU / 0.4).sin();
            Skel {
                sh: [0.14, 0.36],
                head: [0.42, 0.24 + 0.012 * chew],
                head_rot: -0.6,
                nose: (t * 12.0).floor() % 2.0,
                ear_n: 0.25,
                ear_f: 0.5,
                eyes: blink(t),
                ..l
            }
        }
        // a flop: rolls onto its side and lies there, eyes closed
        Anim::Flop => {
            let br = (t * TAU / 3.2).sin();
            let side = Skel {
                hip: [-0.30, 0.15],
                sh: [0.12, 0.17],
                head: [0.40, 0.20 + 0.06 * (b.head - 1.0)],
                head_rot: -0.25,
                flat: 1.0,
                nf: [[0.22, 0.12], [0.44, 0.10]],
                ff: [[0.24, 0.16], [0.46, 0.15]],
                nh: [[-0.40, 0.12], [-0.64, 0.10]],
                fh: [[-0.38, 0.16], [-0.60, 0.15]],
                tail: [-0.50, 0.16],
                ear_n: 1.6,
                ear_f: 1.5,
                eyes: 0.0,
                breath: 0.03 * br,
                ..l
            };
            let k = clamp(t / 0.6, 0.0, 1.0);
            lerp(&l, &side, k * k * (3.0 - 2.0 * k))
        }
        // pushing the toy home: short low hops, nose down behind it
        Anim::Nudge => {
            let p = (t / 0.7) % 1.0;
            let low = Skel { sh: [0.16, 0.38], head: [0.44, 0.40], head_rot: -0.35, ear_n: 0.3, ..l };
            let push = Skel {
                hip: [-0.14, 0.36],
                sh: [0.24, 0.42],
                head: [0.52, 0.42],
                head_rot: -0.35,
                nf: [[0.30, 0.32], [0.44, 0.16]],
                ff: [[0.32, 0.34], [0.47, 0.18]],
                nh: [[-0.28, 0.24], [-0.52, 0.14]],
                fh: [[-0.24, 0.25], [-0.47, 0.16]],
                tail: [-0.40, 0.40],
                ear_n: 0.38,
                ..l
            };
            let land = Skel {
                hip: [-0.04, 0.38],
                sh: [0.30, 0.30],
                head: [0.54, 0.34],
                head_rot: -0.45,
                nf: [[0.36, 0.20], [0.44, 0.035]],
                ff: [[0.39, 0.21], [0.48, 0.04]],
                nh: [[-0.14, 0.28], [0.06, 0.18]],
                fh: [[-0.10, 0.29], [0.10, 0.20]],
                tail: [-0.30, 0.46],
                ear_n: 0.3,
                ..l
            };
            blend(&[(0.0, low), (0.22, push), (0.42, land), (0.66, low), (1.0, low)], p)
        }
        // tug-of-war: front feet planted, a little hop back, head shaking
        Anim::Tug => {
            let p = (t / 0.8) % 1.0;
            let base = Skel {
                hip: [-0.24, 0.28],
                sh: [0.10, 0.38],
                head: [0.34, 0.56 + 0.10 * (b.head - 1.0)],
                nf: [[0.18, 0.26], [0.34, 0.035]],
                ff: [[0.21, 0.27], [0.38, 0.04]],
                ear_n: 0.3,
                ..l
            };
            let back = Skel {
                hip: [-0.36, 0.34],
                sh: [0.0, 0.44],
                head: [0.26, 0.62 + 0.10 * (b.head - 1.0)],
                nh: [[-0.52, 0.10], [-0.07, 0.09]],
                fh: [[-0.47, 0.11], [-0.03, 0.10]],
                tail: [-0.61, 0.34],
                ..base
            };
            let s = blend(&[(0.0, base), (0.2, base), (0.4, back), (0.7, base), (1.0, base)], p);
            Skel { head_rot: -0.15 + 0.21 * (p * TAU * 3.0).sin(), ear_sw: 0.3 * (p * TAU * 2.0).sin(), nose: twitch(t), ..s }
        }
        // waking: front paws out and chest down, back legs long, once
        Anim::Stretch => {
            let k = ramp(t.min(1.5), 0.5, 1.0, 1.5);
            let long = Skel {
                hip: [-0.30, 0.30],
                sh: [0.22, 0.24],
                head: [0.50, 0.42 + 0.10 * (b.head - 1.0)],
                head_rot: -0.2,
                nf: [[0.32, 0.15], [0.60, 0.035]],
                ff: [[0.35, 0.16], [0.64, 0.04]],
                nh: [[-0.45, 0.10], [-0.70, 0.05]],
                fh: [[-0.42, 0.11], [-0.66, 0.06]],
                tail: [-0.52, 0.34],
                ear_n: 0.6,
                ear_f: 0.7,
                ..l
            };
            lerp(&l, &long, k)
        }
        // sits up a little and looks up at the screen, nose going
        Anim::Watch => {
            let drift = (t * TAU / 6.0).sin();
            Skel {
                sh: [0.10, 0.48],
                head: [0.30 + 0.01 * drift, 0.82 + 0.12 * (b.head - 1.0)],
                head_rot: 0.30 + 0.06 * drift,
                ear_n: 0.0,
                ear_f: 0.3,
                nose: twitch(t),
                eyes: blink(t),
                breath: 0.015 * (t * TAU / 2.2).sin(),
                ..l
            }
        }
        // not rabbit anims: the poses they fall back to
        Anim::Run | Anim::Carry => pose(Anim::Walk, look, t),
        Anim::Wag | Anim::Lean => pose(Anim::Stand, look, t),
    }
}

// ---------------------------------------------------------------- shapes

pub fn shapes(s: &Skel, b: &Body, look: &Look) -> Vec<Shape> {
    let mut out: Vec<Shape> = Vec::with_capacity(24);
    let (f, lb, br, mr) = (b.fluff, b.limb, 1.0 + s.breath, look.min_r / look.r);
    let r = |x: f64| js_max(x, mr);
    let hs = b.head * if s.head_front { 1.18 } else { 1.1 };
    let h_ang = if s.head_front { s.tilt } else { s.head_rot };
    let hp = |p: P| add(s.head, rot(mul(p, hs), h_ang));
    // wide and spoon-shaped, as in the photos
    let (ear_len, ear_w) = (0.60 * b.ear, 0.098 * b.ear.sqrt());
    let ear_shape = |base: P, ang: f64, part: Part| {
        let dir = rot([0.0, 1.0], ang);
        let c = add(base, mul(dir, ear_len * 0.5 * hs));
        ell(c, ear_len * 0.5 * hs, ear_w * hs, dir[1].atan2(dir[0]), part)
    };
    let leg = |o: &mut Vec<Shape>, j: &[P; 2], far: bool, grp: i8| {
        o.push(cap(j[0], j[1], r(0.05 * lb), r(0.042 * lb), Part::RLeg).far(far).grp(grp));
        o.push(ell(add(j[1], [0.02, 0.0]), r(0.05 * lb), r(0.03 * lb), 0.0, Part::RLeg).far(far).grp(grp));
    };
    let foot = |o: &mut Vec<Shape>, j: &[P; 2], far: bool, grp: i8| {
        o.push(cap(j[0], j[1], r(0.07 * b.foot * lb), r(0.05 * b.foot * lb), Part::RFoot).far(far).grp(grp));
    };

    if !s.head_front {
        out.push(ear_shape(hp([-0.11, 0.13]), h_ang + s.ear_f, Part::RearF).far(true));
    }
    foot(&mut out, &s.fh, true, 3);
    leg(&mut out, &s.ff, true, 4);
    // body
    let mid = mul(add(s.hip, s.sh), 0.5);
    let bang = (s.sh[1] - s.hip[1]).atan2(s.sh[0] - s.hip[0]);
    let (flat, span) = (s.flat, len(sub(s.sh, s.hip)));
    out.push(ell(
        add(mid, [0.0, 0.03 * (1.0 - flat)]),
        (span * 0.5 + 0.22) * f,
        (0.33 - 0.16 * flat) * f * br,
        bang * (1.0 - flat * 0.6),
        Part::RBody,
    ));
    out.push(ell(s.sh, 0.19 * f, (0.25 - 0.10 * flat) * f * br, bang, Part::RChest));
    out.push(ell(s.hip, 0.27 * f, (0.26 - 0.10 * flat) * f, bang * 0.5, Part::RHaunch));
    out.push(ell(s.tail, r(0.085 * f), r(0.08 * f), 0.0, Part::RTail));
    out.push(ell(mix_p(s.sh, s.head, 0.55), 0.15 * f * hs, 0.15 * f * hs, 0.0, Part::RNape));
    // head
    let head_from = out.len();
    if !s.head_front {
        out.push(ell(hp([0.0, 0.0]), 0.21 * hs, 0.18 * hs, h_ang, Part::RSkull));
        out.push(ell(hp([0.05, -0.07]), 0.17 * hs, 0.135 * hs, h_ang, Part::RCheek));
        out.push(ell(hp([0.17, -0.065]), r(0.085 * hs), r(0.078 * hs), h_ang, Part::RMuzzle));
        out.push(ell(hp([0.245, -0.03 + 0.012 * s.nose]), r(0.026 * hs), r(0.022 * hs), h_ang, Part::RNose));
        out.push(ear_shape(hp([-0.06, 0.15]), h_ang + s.ear_n, Part::Rear));
    } else {
        for sg in [-1.0, 1.0] {
            let sw = if sg < 0.0 { s.ear_sw } else { 0.0 };
            out.push(ear_shape(hp([0.085 * sg, 0.14]), h_ang - 0.2 * sg + sw, Part::RfEar));
        }
        out.push(ell(hp([0.0, 0.01]), 0.20 * hs, 0.185 * hs, h_ang, Part::RfSkull));
        out.push(ell(hp([0.0, -0.08]), 0.225 * hs, 0.125 * hs, h_ang, Part::RfCheek));
        out.push(ell(hp([0.0, -0.085]), r(0.075 * hs), r(0.06 * hs), h_ang, Part::RfMuzzle));
        out.push(ell(hp([0.0, -0.055 + 0.01 * s.nose]), r(0.03 * hs), r(0.022 * hs), h_ang, Part::RNose));
    }
    for sh in &mut out[head_from..] {
        sh.grp = 6;
    }
    foot(&mut out, &s.nh, false, 1);
    leg(&mut out, &s.nf, false, 2);
    out
}

// ---------------------------------------------------------------- markings

fn near(x: f64, y: f64, cx: f64, cy: f64, rr: f64) -> bool {
    (x - cx) * (x - cx) + (y - cy) * (y - cy) < rr * rr
}

/// Read off the photos: a white face with a brown butterfly over the nose and
/// muzzle, brown rings round the eyes running up to the ear bases, white chin
/// and jowls; a white body with a chain of spots along the back, side flecks
/// and one patch on the rump. Spot edges are ragged like real fur.
pub fn paint(s: &Shape, _x: f64, _y: f64, look: &Look, nz: f64, hh: &Hit) -> i8 {
    use fam::*;
    let d = look.detail;
    let fr = nz * if d != 0 { 0.09 * d as f64 } else { 0.04 };
    let (ux, uy) = (hh.ux, hh.uy);
    let spots = |list: &[[f64; 3]]| list.iter().any(|[a, b, rr]| near(ux, uy, *a, *b, rr + fr));
    let pick = |c: bool, y: i8, n: i8| if c { y } else { n };
    match s.part {
        Part::RBody => pick(
            spots(&[[-0.66, 0.55, 0.24], [-0.30, 0.86, 0.19], [0.06, 0.93, 0.14], [0.38, 0.84, 0.10], [-0.42, 0.22, 0.12], [0.02, 0.42, 0.08], [-0.70, -0.05, 0.10]]),
            BROWN,
            WHITE,
        ),
        Part::RHaunch => pick(spots(&[[-0.30, 0.38, 0.44], [0.30, -0.25, 0.14]]), BROWN, WHITE),
        Part::RChest | Part::RTail | Part::RLeg | Part::RFoot => WHITE,
        Part::RNape => pick(spots(&[[-0.45, 0.55, 0.22]]), BROWN, WHITE),
        // profile: eye ring, ear base, bridge down to the muzzle
        Part::RSkull => {
            if uy > 0.55 && ux > -0.1 && ux < 0.5 {
                WHITE // the white forehead stripe
            } else if spots(&[[0.45, 0.22, 0.38], [-0.25, 0.86, 0.30]]) || (ux > 0.55 + fr && uy < 0.5) {
                BROWN
            } else {
                WHITE
            }
        }
        Part::RCheek => pick(ux > 0.12 + fr && uy > -0.42, BROWN, WHITE), // butterfly wing over the whisker pad
        Part::RMuzzle => pick(uy < -0.35 + fr, WHITE, BROWN),
        Part::RNose => PINK,
        Part::Rear => {
            if d >= 1 && uy.abs() < 0.42 && ux > -0.65 && ux < 0.72 {
                PINK
            } else {
                pick(ux > 0.84 || (d >= 1 && uy.abs() > 0.86 && ux > 0.55), MASK, EAR_R)
            }
        }
        Part::RearF => pick(ux > 0.84, MASK, EAR_R),
        Part::RfEar => {
            if uy.abs() < 0.5 && ux > -0.7 && ux < 0.7 {
                PINK
            } else {
                pick(ux > 0.8, MASK, EAR_R)
            }
        }
        // front: eye rings to the ear bases, butterfly bridge, white brow
        Part::RfSkull => {
            let ax = ux.abs();
            if ax < 0.26 + fr && uy > 0.18 {
                WHITE // white stripe up the forehead
            } else if near(ax, uy, 0.66, 0.10, 0.36 + fr) || (ax > 0.3 && uy > 0.55 + fr) {
                BROWN // eye rings, ear bases
            } else if ax < 0.44 + fr && uy <= 0.18 {
                BROWN // butterfly bridge
            } else {
                WHITE
            }
        }
        Part::RfCheek => pick(ux.abs() < 0.58 + fr && uy > -0.6 + 0.5 * ux.abs(), BROWN, WHITE), // butterfly wings
        Part::RfMuzzle => pick(uy < -0.55, WHITE, BROWN), // just a white chin
        _ => WHITE,
    }
}

pub fn eyes(out: &mut [u8], s: &Skel, b: &Body, look: &Look, fs: FrameSize, pal: &Palette) {
    let hs = b.head * if s.head_front { 1.18 } else { 1.1 };
    let ang = if s.head_front { s.tilt } else { s.head_rot };
    let pts: &[P] = if s.head_front { &[[-0.135, 0.03], [0.135, 0.03]] } else { &[[0.095, 0.04]] };
    for p in pts {
        let w = add(s.head, rot(mul(*p, hs), ang));
        let (ix, iy) = px_of(fs, look, w);
        if s.eyes > 0.5 {
            put(out, fs, ix, iy, pal.eye);
            if look.detail >= 1 {
                put(out, fs, ix + 1, iy, pal.eye);
                put(out, fs, ix, iy + 1, pal.eye);
                put(out, fs, ix + 1, iy + 1, pal.eye);
                put(out, fs, ix + 1, iy, pal.iris);
            }
        } else {
            put(out, fs, ix, iy + 1, pal.eye);
            put(out, fs, ix + 1, iy + 1, pal.eye);
            if look.detail >= 2 {
                put(out, fs, ix - 1, iy + 1, pal.eye);
            }
        }
    }
}
