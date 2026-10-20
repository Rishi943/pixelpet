//! The dog species and Alex's coat, ported from `drafts/looks/rig.js`.

use super::core::*;
use super::{Anim, Toy};

// ---------------------------------------------------------------- coat

/// Alex: red-and-black GSD. Sampled from his photos.
pub static ALEX: Coat = Coat {
    fams: &[
        (fam::TAN, [0x6A3518, 0x9A5226, 0xC47637, 0xE09A57]),
        (fam::CREAM, [0x9C6A3E, 0xC68F57, 0xE2B077, 0xF3D29C]),
        (fam::BLACK, [0x0D0A0A, 0x171212, 0x231B1B, 0x3B3132]),
        (fam::MASK, [0x100C0B, 0x1C1514, 0x2B2220, 0x4A3B34]),
        (fam::EAR_IN, [0x3A2218, 0x5A3424, 0x7E4A34, 0xA36B50]),
        (fam::NOSE, [0x080707, 0x100E0E, 0x191616, 0x3E3838]),
        (fam::TONGUE, [0x8E3442, 0xB44E5E, 0xD96C7A, 0xF09AA4]),
        (fam::MOUTH, [0x22080B, 0x341015, 0x46141B, 0x5E1E26]),
        (fam::COLLAR, [0x132C66, 0x1E4494, 0x2F64C8, 0x5E8FE6]),
        (fam::BALL, [0x560910, 0x86121E, 0xB81F2D, 0xE2525C]),
        (fam::SEAM, [0xB0A088, 0xD2C2A6, 0xEADBC0, 0xFFF4E2]),
        (fam::COCO, [0x382010, 0x583418, 0x7A4C25, 0xA26E3E]),
    ],
    eye: 0x140A06,
    iris: 0xA0601F,
    outline: 0x24120A,
    edge_drop: 1,
};

// ---------------------------------------------------------------- body

/// Body sliders. 1 = Alex's real proportions.
pub struct Body {
    pub leg_len: f64,
    pub body_len: f64,
    pub head: f64,
    pub ear: f64,
    pub muzzle: f64,
    pub tail: f64,
    pub fluff: f64,
    pub limb: f64,
}

pub fn body(look: &Look) -> Body {
    match look.name {
        "chunky" => Body { leg_len: 0.72, body_len: 0.84, head: 1.5, ear: 1.15, muzzle: 0.78, tail: 0.9, fluff: 1.12, limb: 1.7 },
        "classic" => Body { leg_len: 0.88, body_len: 0.95, head: 1.2, ear: 1.08, muzzle: 0.94, tail: 1.0, fluff: 1.05, limb: 1.25 },
        _ => Body { leg_len: 1.0, body_len: 1.0, head: 1.0, ear: 1.0, muzzle: 1.0, tail: 1.0, fluff: 1.0, limb: 1.0 },
    }
}

// ---------------------------------------------------------------- skeleton

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Legs {
    pub nf: [P; 4],
    pub ff: [P; 4],
    pub nh: [P; 4],
    pub fh: [P; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skel {
    pub hip: P,
    pub sh: P,
    pub head: P,
    pub head_rot: f64,
    pub head_front: bool,
    pub tilt: f64,
    pub mouth: f64,
    pub tongue: f64,
    pub eyes: f64,
    pub ear_back: f64,
    pub breath: f64,
    pub toy: Option<Toy>,
    pub legs: Legs,
    pub tail: [P; 5],
    pub neck_a: P,
    pub neck_b: P,
}

fn mix_pts<const N: usize>(a: &[P; N], b: &[P; N], k: f64) -> [P; N] {
    let mut o = *a;
    for i in 0..N {
        o[i] = mix_p(a[i], b[i], k);
    }
    o
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
        mouth: mix(a.mouth, b.mouth, k),
        tongue: mix(a.tongue, b.tongue, k),
        eyes: if pick { a.eyes } else { b.eyes },
        ear_back: mix(a.ear_back, b.ear_back, k),
        breath: mix(a.breath, b.breath, k),
        toy: if pick { a.toy } else { b.toy },
        legs: Legs {
            nf: mix_pts(&a.legs.nf, &b.legs.nf, k),
            ff: mix_pts(&a.legs.ff, &b.legs.ff, k),
            nh: mix_pts(&a.legs.nh, &b.legs.nh, k),
            fh: mix_pts(&a.legs.fh, &b.legs.fh, k),
        },
        tail: mix_pts(&a.tail, &b.tail, k),
        neck_a: mix_p(a.neck_a, b.neck_a, k),
        neck_b: mix_p(a.neck_b, b.neck_b, k),
    }
}

fn ik(a: P, c: P, l1: f64, l2: f64, bend: f64) -> P {
    let d = sub(c, a);
    let dist = clamp(len(d), (l1 - l2).abs() + 1e-4, l1 + l2 - 1e-4);
    let an = clamp((l1 * l1 + dist * dist - l2 * l2) / (2.0 * l1 * dist), -1.0, 1.0).acos();
    let ang = d[1].atan2(d[0]) + bend * an;
    [a[0] + l1 * ang.cos(), a[1] + l1 * ang.sin()]
}

struct Geom {
    hip: P,
    sh: P,
    f_paw: P,
    h_paw: P,
    f1: f64,
    f2: f64,
    h1: f64,
    h2: f64,
    rest_ang: f64,
}

fn geom(b: &Body) -> Geom {
    let (lk, bl) = (b.leg_len, b.body_len);
    let by = 0.24 + 0.46 * lk;
    let hip = [-0.38 * bl, by];
    let sh = [0.30 * bl, by + 0.02];
    let f_top = add(sh, [0.0, -0.10]);
    let f_paw = [sh[0] + 0.03, PAW_Y];
    let f_w = add(f_paw, [-0.02, 0.10 * lk]);
    let h_top = add(hip, [0.03, -0.06]);
    let h_paw = [hip[0] - 0.10, PAW_Y];
    let h_h = add(h_paw, [-0.05, 0.17 * lk]);
    let fd = len(sub(f_w, f_top));
    let hd = len(sub(h_h, h_top));
    Geom {
        hip,
        sh,
        f_paw,
        h_paw,
        f1: fd * 0.53,
        f2: fd * 0.53,
        h1: hd * 0.6,
        h2: hd * 0.6,
        rest_ang: (sh[1] - hip[1]).atan2(sh[0] - hip[0]),
    }
}

const HANG: [f64; 4] = [-2.45, -1.95, -1.72, -2.05];

fn tail_chain(base: P, angs: [f64; 4], seg: f64) -> [P; 5] {
    let mut pts = [base; 5];
    let mut p = base;
    for (i, a) in angs.iter().enumerate() {
        p = add(p, [a.cos() * seg, a.sin() * seg]);
        pts[i + 1] = p;
    }
    pts
}

fn front_leg(top: P, paw: P, flex: f64, b: &Body, g: &Geom) -> [P; 4] {
    let wrist = add(paw, [-0.02 + 0.09 * flex, (0.10 - 0.035 * flex) * b.leg_len]);
    [top, ik(top, wrist, g.f1, g.f2, -1.0), wrist, paw]
}

fn hind_leg(top: P, paw: P, flex: f64, b: &Body, g: &Geom) -> [P; 4] {
    let hock = add(paw, [-0.05 - 0.06 * flex, (0.17 - 0.05 * flex) * b.leg_len]);
    [top, ik(top, hock, g.h1, g.h2, 1.0), hock, paw]
}

/// Paw targets, in the JS key order nf, ff, nh, fh.
#[derive(Clone, Copy)]
struct Paws {
    nf: P,
    ff: P,
    nh: P,
    fh: P,
}

/// The drivers `build` fills a skeleton from. Defaults match the JS `||`s.
struct Drive {
    hip: P,
    sh: P,
    head: P,
    head_rot: f64,
    head_front: bool,
    tilt: f64,
    mouth: f64,
    tongue: f64,
    eyes: f64,
    ear_back: f64,
    breath: f64,
    toy: Option<Toy>,
    paws: Paws,
    flex: [f64; 4], // nf, ff, nh, fh
    legs: [Option<[P; 4]>; 4],
    tail_base: Option<P>,
    tail_angs: Option<[f64; 4]>,
    tail_len: Option<f64>,
}

impl Drive {
    fn new(hip: P, sh: P, head: P, paws: Paws) -> Drive {
        Drive {
            hip,
            sh,
            head,
            head_rot: 0.0,
            head_front: false,
            tilt: 0.0,
            mouth: 0.0,
            tongue: 0.0,
            eyes: 1.0,
            ear_back: 0.0,
            breath: 0.0,
            toy: None,
            paws,
            flex: [0.0; 4],
            legs: [None; 4],
            tail_base: None,
            tail_angs: None,
            tail_len: None,
        }
    }
}

fn build(b: &Body, g: &Geom, c: Drive) -> Skel {
    let ang = (c.sh[1] - c.hip[1]).atan2(c.sh[0] - c.hip[0]) - g.rest_ang;
    let f_top = add(c.sh, rot([0.0, -0.10], ang));
    let h_top = add(c.hip, rot([0.03, -0.06], ang));
    let legs = Legs {
        nf: c.legs[0].unwrap_or_else(|| front_leg(f_top, c.paws.nf, c.flex[0], b, g)),
        ff: c.legs[1].unwrap_or_else(|| front_leg(add(f_top, [0.03, 0.01]), c.paws.ff, c.flex[1], b, g)),
        nh: c.legs[2].unwrap_or_else(|| hind_leg(h_top, c.paws.nh, c.flex[2], b, g)),
        fh: c.legs[3].unwrap_or_else(|| hind_leg(add(h_top, [0.03, 0.01]), c.paws.fh, c.flex[3], b, g)),
    };
    let tb = add(c.hip, rot(c.tail_base.unwrap_or([-0.20, 0.06]), ang));
    let tail = tail_chain(tb, c.tail_angs.unwrap_or(HANG), 0.19 * b.tail * c.tail_len.unwrap_or(1.0));
    let neck_a = add(c.sh, rot([0.07, 0.14], ang));
    let neck_b = add(c.head, rot([-0.07 * b.head, -0.07 * b.head], if c.head_front { 0.0 } else { c.head_rot }));
    Skel {
        hip: c.hip,
        sh: c.sh,
        head: c.head,
        head_rot: c.head_rot,
        head_front: c.head_front,
        tilt: c.tilt,
        mouth: c.mouth,
        tongue: c.tongue,
        eyes: c.eyes,
        ear_back: c.ear_back,
        breath: c.breath,
        toy: c.toy,
        legs,
        tail,
        neck_a,
        neck_b,
    }
}

fn rest_paws(g: &Geom) -> Paws {
    Paws { nf: g.f_paw, ff: add(g.f_paw, [0.06, 0.0]), nh: g.h_paw, fh: add(g.h_paw, [0.06, 0.0]) }
}

fn head_rest(b: &Body, g: &Geom) -> P {
    add(g.sh, [0.29, 0.52 + 0.07 * (b.head - 1.0)])
}

fn hang_map(f: impl Fn(f64, f64) -> f64) -> [f64; 4] {
    let mut o = HANG;
    for (i, a) in HANG.iter().enumerate() {
        o[i] = f(*a, i as f64);
    }
    o
}

// ---------------------------------------------------------------- anims

pub fn period(anim: Anim) -> f64 {
    match anim {
        Anim::Stand => 2.0,
        Anim::Walk => 0.9,
        Anim::Sit => 2.0,
        Anim::Tilt => 4.0,
        Anim::Hop => 1.0,
        Anim::Sleep => 9.6,
        Anim::Run => RUN_PERIOD,
        Anim::Carry => CARRY_PERIOD,
        Anim::Eat => 1.0,
        Anim::Wag => 2.4,
        Anim::Lean => 3.0,
        Anim::Tug => 0.9,
        Anim::Stretch => 1.5,
        // not dog anims: the poses they fall back to
        Anim::Flop => 9.6,
        Anim::Nudge => 0.9,
    }
}

const RUN_PERIOD: f64 = 0.42;
const RUN_DUTY: f64 = 0.38;
const CARRY_PERIOD: f64 = 0.5;
const CARRY_DUTY: f64 = 0.5;
/// Run and carry speeds as multiples of the walk; their strides are sized
/// from these so the paws don't slide.
pub const RUN_SPEEDUP: f64 = 4.0;
pub const CARRY_SPEEDUP: f64 = 2.5;

/// Units per second while walking, with no foot slide.
pub fn walk_speed(b: &Body) -> f64 {
    (0.30 * b.leg_len) / (0.62 * 0.9)
}

pub fn pose(anim: Anim, look: &Look, t: f64, toy: Option<Toy>) -> Skel {
    let b = body(look);
    match anim {
        Anim::Stand => stand(&b, t, toy),
        Anim::Walk => walk(&b, t, toy),
        Anim::Sit => sit_skel(&b, t, toy, false, 0.0),
        Anim::Tilt => sit_skel(&b, t, toy, true, 0.0),
        Anim::Hop => hop(&b, t, toy),
        Anim::Sleep | Anim::Flop => sleep(&b, t),
        Anim::Run => run(&b, t, toy),
        Anim::Carry => carry(&b, t, toy),
        Anim::Eat => eat(&b, t),
        Anim::Wag => wag(&b, t),
        Anim::Lean => lean(&b, t),
        Anim::Tug => tug(&b, t),
        Anim::Stretch => stretch(&b, t),
        Anim::Nudge => walk(&b, t, toy),
    }
}

/// Where a rope or food meets the mouth, in rig units.
pub fn mouth(s: &Skel, b: &Body) -> P {
    let hs = b.head * if s.head_front { 1.15 } else { 1.0 };
    if s.head_front {
        add(s.head, rot(mul([0.0, -0.16], hs), s.tilt))
    } else {
        add(s.head, rot(mul([0.06 + 0.18 * b.muzzle, -0.10], hs), s.head_rot))
    }
}

/// Paw targets and flexes for a gait: each leg's phase offset (nf, ff, nh,
/// fh), the share of the loop it is on the ground, stride and lift.
fn gait(rest: Paws, p: f64, offs: [f64; 4], duty: f64, stride: f64, lift: f64) -> (Paws, [f64; 4]) {
    let rs = [rest.nf, rest.ff, rest.nh, rest.fh];
    let mut paws = [[0.0; 2]; 4];
    let mut flex = [0.0; 4];
    for k in 0..4 {
        let ph = (p + offs[k]).rem_euclid(1.0);
        let (dx, dy, f) = if ph < duty {
            (stride * (0.5 - ph / duty), 0.0, 0.0)
        } else {
            let s = (ph - duty) / (1.0 - duty);
            (stride * (-0.5 + s), lift * (PI * s).sin(), (PI * s).sin())
        };
        paws[k] = add(rs[k], [dx, dy]);
        flex[k] = f;
    }
    (Paws { nf: paws[0], ff: paws[1], nh: paws[2], fh: paws[3] }, flex)
}

/// Fetch: a bounding gallop, front pair then hind pair, ears back, tail out.
fn run(b: &Body, t: f64, toy: Option<Toy>) -> Skel {
    let g = geom(b);
    let p = (t / RUN_PERIOD) % 1.0;
    let stride = RUN_SPEEDUP * walk_speed(b) * RUN_DUTY * RUN_PERIOD;
    let (paws, flex) = gait(rest_paws(&g), p, [0.0, 0.08, 0.5, 0.58], RUN_DUTY, stride, 0.16 * b.leg_len);
    // the spine gathers and stretches once a stride
    let w = (p * TAU).sin();
    let bob = 0.03 * (p * TAU * 2.0).cos();
    let mut c = Drive::new(
        add(g.hip, [-0.03 * w, bob + 0.02 * w]),
        add(g.sh, [0.03 * w, bob - 0.02 * w]),
        add(head_rest(b, &g), [0.10 + 0.03 * w, -0.10 + bob]),
        paws,
    );
    c.flex = flex;
    c.head_rot = -0.10;
    c.mouth = if toy.is_some() { 0.45 } else { 0.5 };
    c.tongue = if toy.is_some() { 0.0 } else { 0.9 };
    c.ear_back = 0.6;
    c.tail_angs = Some(hang_map(|_, i| -2.85 - 0.06 * i + 0.10 * (p * TAU + i).sin()));
    c.toy = toy;
    build(b, &g, c)
}

/// Bringing the toy back: a quick trot, head up, mouth closed on the toy.
fn carry(b: &Body, t: f64, toy: Option<Toy>) -> Skel {
    let g = geom(b);
    let p = (t / CARRY_PERIOD) % 1.0;
    let stride = CARRY_SPEEDUP * walk_speed(b) * CARRY_DUTY * CARRY_PERIOD;
    let (paws, flex) = gait(rest_paws(&g), p, [0.0, 0.5, 0.5, 0.0], CARRY_DUTY, stride, 0.12 * b.leg_len);
    let bob = 0.02 * (p * TAU * 2.0).cos();
    let mut c = Drive::new(add(g.hip, [0.0, bob]), add(g.sh, [0.0, bob]), add(head_rest(b, &g), [0.03, 0.04 + bob]), paws);
    c.flex = flex;
    c.head_rot = 0.12;
    c.mouth = if toy.is_some() { 0.45 } else { 0.1 };
    c.ear_back = 0.2;
    c.tail_angs = Some(hang_map(|a, i| a + 0.35 + (p * TAU).sin() * 0.12 * (i + 1.0) * 0.5));
    c.toy = toy;
    build(b, &g, c)
}

/// Head down at the bowl, chewing.
fn eat(b: &Body, t: f64) -> Skel {
    let g = geom(b);
    let chew = (t * TAU / 0.5).sin();
    // muzzle forward and down to the ground, in front of the front paws
    let mut c = Drive::new(g.hip, add(g.sh, [0.02, -0.10]), add(g.sh, [0.52, -0.40 + 0.015 * chew]), rest_paws(&g));
    c.head_rot = -0.75;
    c.mouth = 0.6 * (PI * (t % 1.0)).sin();
    c.ear_back = 0.1;
    c.tail_angs = Some(hang_map(|a, i| a + 0.15 + (t * TAU).sin() * 0.06 * (i + 1.0) * 0.5));
    build(b, &g, c)
}

/// Happy: the tail sweeps, the body wiggles, now and then a lip lick.
fn wag(b: &Body, t: f64) -> Skel {
    let g = geom(b);
    let w = (t * TAU * 2.5).sin();
    let k = t % 2.4;
    let lick = if (1.2..1.6).contains(&k) { (PI * (k - 1.2) / 0.4).sin() } else { 0.0 };
    let mut c = Drive::new(add(g.hip, [0.02 * w, 0.0]), g.sh, add(head_rest(b, &g), [0.0, 0.01 * w]), rest_paws(&g));
    c.head_rot = 0.12;
    c.mouth = 0.08 + 0.3 * lick;
    c.tongue = lick;
    c.ear_back = 0.25;
    // tail up and swinging 35 degrees each way
    c.tail_angs = Some(hang_map(|_, i| 2.75 - 0.18 * i + 0.61 * w * (0.7 + 0.15 * i)));
    build(b, &g, c)
}

/// Petted: sits and leans in, eyes closed.
fn lean(b: &Body, t: f64) -> Skel {
    let mut s = sit_skel(b, t, None, false, 0.21);
    s.eyes = 0.0;
    s.mouth = 0.05;
    s.tongue = 0.0;
    s
}

/// Tug-of-war: braced, hips back and low, shaking the rope.
fn tug(b: &Body, t: f64) -> Skel {
    let g = geom(b);
    let shake = (t * TAU * 3.0 / 0.9).sin();
    let rest = rest_paws(&g);
    let paws = Paws { nf: add(rest.nf, [0.14, 0.0]), ff: add(rest.ff, [0.12, 0.0]), nh: rest.nh, fh: rest.fh };
    let mut c = Drive::new(add(g.hip, [-0.10, -0.22]), add(g.sh, [0.0, -0.08]), add(head_rest(b, &g), [0.10, -0.24]), paws);
    c.flex = [0.0, 0.0, 0.9, 0.9];
    c.head_rot = -0.10 + 0.314 * shake;
    c.mouth = 0.15;
    c.ear_back = 0.35;
    c.tail_angs = Some(hang_map(|a, i| a + 0.25 + shake * 0.05 * (i + 1.0)));
    build(b, &g, c)
}

/// Waking up: a play bow and back, once.
fn stretch(b: &Body, t: f64) -> Skel {
    let g = geom(b);
    let k = ramp(t.min(1.5), 0.5, 1.0, 1.5);
    let rest = rest_paws(&g);
    let paws = Paws { nf: add(rest.nf, [0.16 * k, 0.0]), ff: add(rest.ff, [0.16 * k, 0.0]), nh: rest.nh, fh: rest.fh };
    let mut c = Drive::new(add(g.hip, [0.0, 0.05 * k]), add(g.sh, [0.04 * k, -0.27 * k]), add(head_rest(b, &g), [0.12 * k, -0.34 * k]), paws);
    c.head_rot = 0.08 - 0.25 * k;
    c.mouth = 0.5 * k;
    c.tongue = 0.4 * k;
    c.ear_back = 0.3 * k;
    c.tail_angs = Some(hang_map(|a, i| a + 1.1 * k * (0.6 + 0.2 * i)));
    build(b, &g, c)
}

fn stand(b: &Body, t: f64, toy: Option<Toy>) -> Skel {
    let g = geom(b);
    let pant = (t * TAU * 2.5).sin();
    let mut c = Drive::new(g.hip, add(g.sh, [0.0, 0.004 * pant]), add(head_rest(b, &g), [0.0, 0.006 * pant]), rest_paws(&g));
    c.head_rot = 0.08;
    c.mouth = if toy.is_some() { 0.45 } else { 0.42 + 0.12 * pant };
    c.tongue = if toy.is_some() { 0.0 } else { 1.0 };
    c.breath = 0.012 * pant;
    c.tail_angs = Some(hang_map(|a, i| a + (t * TAU * 1.5).sin() * 0.10 * (i + 1.0) * 0.5));
    c.toy = toy;
    build(b, &g, c)
}

fn walk(b: &Body, t: f64, toy: Option<Toy>) -> Skel {
    let g = geom(b);
    let p = (t / 0.9) % 1.0;
    let (stride, duty, lift) = (0.30 * b.leg_len, 0.62, 0.10 * b.leg_len);
    let rest = rest_paws(&g);
    // nf, ff, nh, fh with their phase offsets
    let legs = [(rest.nf, 0.25), (rest.ff, 0.75), (rest.nh, 0.0), (rest.fh, 0.5)];
    let mut paws = [[0.0; 2]; 4];
    let mut flex = [0.0; 4];
    for (k, (r, off)) in legs.iter().enumerate() {
        let ph = (p + off) % 1.0;
        let (mut dy, mut f) = (0.0, 0.0);
        let dx = if ph < duty {
            stride * (0.5 - ph / duty)
        } else {
            let s = (ph - duty) / (1.0 - duty);
            dy = lift * (PI * s).sin();
            f = (PI * s).sin();
            stride * (-0.5 + s)
        };
        paws[k] = add(*r, [dx, dy]);
        flex[k] = f;
    }
    let bob = 0.014 * (p * TAU * 2.0).cos();
    let mut c = Drive::new(
        add(g.hip, [0.0, bob]),
        add(g.sh, [0.0, bob * 0.7]),
        add(head_rest(b, &g), [0.01, -0.02 + 0.012 * (p * TAU * 2.0 + 0.8).cos()]),
        Paws { nf: paws[0], ff: paws[1], nh: paws[2], fh: paws[3] },
    );
    c.flex = flex;
    c.head_rot = 0.0;
    c.mouth = if toy.is_some() { 0.45 } else { 0.35 };
    c.tongue = if toy.is_some() { 0.0 } else { 0.7 };
    c.tail_angs = Some(hang_map(|a, i| a + 0.05 + (p * TAU).sin() * 0.08 * (i + 1.0) * 0.5));
    c.toy = toy;
    build(b, &g, c)
}

fn hop(b: &Body, t: f64, toy: Option<Toy>) -> Skel {
    let g = geom(b);
    let p = (t / 1.0) % 1.0;
    let (mut lift, mut crouch, mut air, mut pitch) = (0.0, 0.0, 0.0, 0.0);
    if p < 0.22 {
        crouch = 0.09 * (PI / 2.0 * p / 0.22).sin();
    } else if p < 0.78 {
        let s = (p - 0.22) / 0.56;
        lift = 0.5 * b.leg_len * (PI * s).sin();
        air = (PI * s).sin();
        pitch = 0.06 * (PI * s).cos();
    } else {
        crouch = 0.09 * (PI / 2.0 * (p - 0.78) / 0.22).cos();
    }
    let dy = lift - crouch;
    let rest = rest_paws(&g);
    let paws = if air > 0.0 {
        Paws {
            nf: add(rest.nf, [0.10 * air, lift + 0.10 * air]),
            ff: add(rest.ff, [0.12 * air, lift + 0.12 * air]),
            nh: add(rest.nh, [-0.10 * air, lift + 0.04 * air]),
            fh: add(rest.fh, [-0.08 * air, lift + 0.05 * air]),
        }
    } else {
        rest
    };
    let mut c = Drive::new(
        add(g.hip, [0.0, dy - crouch * 0.4 - pitch]),
        add(g.sh, [0.0, dy + pitch]),
        add(head_rest(b, &g), [0.02 * air, dy + pitch * 1.4]),
        paws,
    );
    c.head_rot = -0.12 + pitch;
    c.flex = [0.9 * air, 0.9 * air, 0.5 * air, 0.5 * air];
    c.mouth = if toy.is_some() { 0.45 } else { 0.6 };
    c.tongue = if toy.is_some() { 0.0 } else { 0.6 };
    c.ear_back = 0.35 * air;
    c.tail_angs = Some(hang_map(|a, i| a + air * 0.5 * (i + 1.0) * 0.35));
    c.toy = toy;
    build(b, &g, c)
}

fn sleep(b: &Body, t: f64) -> Skel {
    let g = geom(b);
    let (bl, f) = (b.body_len, b.fluff);
    let br = (t * TAU / 3.2).sin();
    // lying flat on his side, legs stretched forward (from Alex's sleeping photo)
    let hip = [-0.40 * bl, 0.27 * f];
    let sh = [0.18 * bl, 0.31 * f];
    let tm = t % 9.6;
    let tw = if tm > 7.4 && tm < 8.2 { (tm * 30.0).sin() * 0.018 } else { 0.0 }; // dream twitch
    let (fx, hx) = (sh[0], hip[0]);
    let nf = [[fx + 0.10, 0.16], [fx + 0.28, 0.07], [fx + 0.62, 0.055 + tw], [fx + 0.74, 0.05 + tw]];
    let ff = nf.map(|q| add(q, [-0.02, 0.07]));
    let nh = [[hx + 0.12, 0.18], [hx + 0.36, 0.10], [hx + 0.24, 0.05], [hx + 0.44, 0.05 + tw * 0.5]];
    let fh = nh.map(|q| add(q, [0.06, 0.05]));
    let mut c = Drive::new(hip, sh, [fx + 0.36, 0.29 + 0.08 * (b.head - 1.0)], rest_paws(&g));
    c.head_rot = -0.12;
    c.eyes = 0.0;
    c.ear_back = 0.12;
    c.breath = 0.035 * br;
    c.legs = [Some(nf), Some(ff), Some(nh), Some(fh)];
    c.tail_base = Some([-0.20, -0.04]);
    c.tail_angs = Some([-2.2, -2.75, -3.1, 2.75]);
    c.tail_len = Some(0.9);
    build(b, &g, c)
}

/// `lean`: radians the upper body tips towards the facing side, about the hip.
fn sit_skel(b: &Body, t: f64, toy: Option<Toy>, tilt: bool, lean: f64) -> Skel {
    // measured off Alex's side-on sitting photo, in withers units, from the front paws
    let g = geom(b);
    let (lk, bl, f) = (b.leg_len, b.body_len, b.fluff);
    let pp = 0.20 * bl;
    let sh = [pp - 0.15 * bl, 0.75 * lk];
    let hip = [pp - 0.52 * bl, 0.26 * f];
    let pant = (t * TAU * 2.5).sin();
    let h_top = add(hip, [0.06, 0.02]);
    let nh = [h_top, [hip[0] + 0.28, 0.20], [hip[0] - 0.02, 0.07], [hip[0] + 0.26, PAW_Y]];
    let fh = nh.map(|q| add(q, [0.05, 0.01]));
    let mut tilt_a = 0.0;
    let mut mouth = if toy.is_some() { 0.45 } else { 0.42 + 0.12 * pant };
    let mut tongue = if toy.is_some() { 0.0 } else { 1.0 };
    if tilt {
        let k = t % 4.0;
        let ramp = if k < 0.5 {
            0.0
        } else if k < 0.9 {
            (k - 0.5) / 0.4
        } else if k < 2.2 {
            1.0
        } else if k < 2.6 {
            1.0 - (k - 2.2) / 0.4
        } else {
            0.0
        };
        tilt_a = 0.34 * ramp * if (t / 4.0).floor() % 2.0 != 0.0 { -1.0 } else { 1.0 };
        mouth = if toy.is_some() { 0.45 } else { 0.0 };
        tongue = 0.0;
    }
    let head = add(
        sh,
        [
            if tilt { 0.17 } else { 0.20 },
            (if tilt { 0.50 } else { 0.57 }) * lk + 0.06 * (b.head - 1.0) + (if tilt { 0.0 } else { 0.004 * pant }),
        ],
    );
    let (sh, head) = if lean != 0.0 {
        (add(hip, rot(sub(sh, hip), -lean)), add(hip, rot(sub(head, hip), -lean)))
    } else {
        (sh, head)
    };
    let rest = rest_paws(&g);
    let mut c = Drive::new(hip, sh, head, Paws { nf: [pp, PAW_Y], ff: [pp + 0.06, PAW_Y], nh: rest.nh, fh: rest.fh });
    c.head_rot = 0.22;
    c.head_front = tilt;
    c.tilt = tilt_a;
    c.legs = [None, None, Some(nh), Some(fh)];
    c.mouth = mouth * 0.75;
    c.tongue = tongue;
    c.breath = 0.01 * pant;
    c.tail_base = Some([-0.20, -0.06]);
    c.tail_angs = Some([-2.45 + 0.1 * (t * TAU * 1.5).sin(), -2.95, 2.55, 1.95]);
    c.tail_len = Some(0.8);
    c.toy = toy;
    build(b, &g, c)
}

// ---------------------------------------------------------------- shapes

pub fn shapes(s: &Skel, b: &Body, look: &Look) -> Vec<Shape> {
    let mut out: Vec<Shape> = Vec::with_capacity(40);
    let (f, lb, br, mr) = (b.fluff, b.limb, 1.0 + s.breath, look.min_r / look.r);
    let r = |x: f64| js_max(x, mr);
    let hs = b.head * if s.head_front { 1.15 } else { 1.0 };
    let h_ang = if s.head_front { s.tilt } else { s.head_rot };
    let hp = |p: P| add(s.head, rot(mul(p, hs), h_ang));
    let torso_ang = (s.sh[1] - s.hip[1]).atan2(s.sh[0] - s.hip[0]);

    // ears behind the head
    let ear_tip = |b0: P, b1: P, len_e: f64, lean: f64| add(mul(add(b0, b1), 0.5), rot([0.0, len_e], lean));
    if !s.head_front {
        let (b0, b1) = ([-0.18, 0.08], [-0.05, 0.11]);
        out.push(tri(hp(b0), hp(b1), hp(ear_tip(b0, b1, 0.25 * b.ear, 0.30 + 1.2 * s.ear_back)), Part::EarF).far(true));
    }
    let leg_shapes = |o: &mut Vec<Shape>, j: &[P; 4], front: bool, far: bool, grp: i8| {
        let [a, bb, c, d] = *j;
        if front {
            o.push(cap(a, bb, r(0.085 * lb * f), r(0.06 * lb), Part::LegU).far(far).grp(grp).no_edge());
            o.push(cap(bb, c, r(0.05 * lb), r(0.042 * lb), Part::Leg).far(far).grp(grp));
            o.push(cap(c, d, r(0.04 * lb), r(0.038 * lb), Part::Leg).far(far).grp(grp));
        } else {
            o.push(cap(a, bb, r(0.15 * f * lb.sqrt()), r(0.075 * lb), Part::Thigh).far(far).grp(grp).no_edge());
            o.push(cap(bb, c, r(0.062 * lb), r(0.042 * lb), Part::Leg).far(far).grp(grp));
            o.push(cap(c, d, r(0.04 * lb), r(0.036 * lb), Part::Leg).far(far).grp(grp));
        }
        o.push(ell(add(d, [0.028, 0.004]), r(0.055 * lb), r(0.03 * lb), 0.0, Part::Paw).far(far).grp(grp));
    };
    // far legs
    leg_shapes(&mut out, &s.legs.fh, false, true, 3);
    leg_shapes(&mut out, &s.legs.ff, true, true, 4);
    // tail
    let tr = [0.06, 0.085, 0.09, 0.075, 0.035];
    for i in 0..4 {
        out.push(cap(s.tail[i], s.tail[i + 1], r(tr[i] * f), r(tr[i + 1] * f), Part::Tail).idx(i as u8).flip().grp(5));
    }
    // body
    out.push(ell(s.hip, 0.23 * f, 0.23 * f * br, torso_ang, Part::Rump));
    out.push(cap(s.hip, s.sh, 0.21 * f * br, 0.26 * f * br, Part::Torso));
    out.push(ell(add(s.sh, rot([0.04, -0.03], torso_ang)), 0.25 * f, 0.28 * f * br, torso_ang, Part::Chest));
    out.push(cap(s.neck_a, s.neck_b, 0.19 * f, 0.14 * f * b.head, Part::Neck));
    out.push(ell(add(s.sh, [0.10, -0.08]), 0.17 * f, 0.20 * f * br, 0.0, Part::Brisket));
    // head
    let head_from = out.len();
    if !s.head_front {
        let (m, mz) = (s.mouth, b.muzzle);
        let jaw_a = [0.06, -0.095];
        let jaw_b = add(jaw_a, rot([0.20 * mz, 0.0], -0.08 - 0.32 * m));
        let up_tip = [0.09 + 0.22 * mz, -0.075];
        if m > 0.08 {
            out.push(tri(hp([0.05, -0.08]), hp(add(up_tip, [0.0, -0.03])), hp(jaw_b), Part::Mouth));
        }
        out.push(cap(hp(jaw_a), hp(jaw_b), 0.06 * hs, r(0.04 * hs), Part::Jaw));
        out.push(ell(hp([0.0, 0.0]), 0.19 * hs, 0.155 * hs, h_ang, Part::Skull));
        if s.tongue > 0.05 {
            let t_a = [0.11 + 0.06 * mz, -0.11];
            let t_b = add(t_a, [0.05, -0.15 * s.tongue - 0.02]);
            out.push(cap(hp(t_a), hp(t_b), r(0.032 * hs), r(0.04 * hs), Part::Tongue));
        }
        out.push(cap(hp([0.09, -0.035]), hp(up_tip), 0.095 * hs, 0.058 * hs, Part::Muzzle));
        out.push(ell(hp(add(up_tip, [0.012, 0.008])), r(0.042 * hs), r(0.032 * hs), h_ang, Part::Nose));
        let (b0, b1) = ([-0.11, 0.09], [0.04, 0.12]);
        out.push(tri(hp(b0), hp(b1), hp(ear_tip(b0, b1, 0.27 * b.ear, 0.24 + 1.2 * s.ear_back)), Part::EarN));
    } else {
        for sgn in [-1.0, 1.0] {
            let (b0, b1) = ([0.18 * sgn, 0.05], [0.05 * sgn, 0.13]);
            out.push(tri(hp(b0), hp(b1), hp([0.19 * sgn, 0.08 + 0.29 * b.ear]), Part::EarFront));
        }
        out.push(ell(hp([0.0, 0.02]), 0.19 * hs, 0.16 * hs, h_ang, Part::FSkull));
        out.push(ell(hp([0.0, -0.07]), 0.16 * hs, 0.095 * hs, h_ang, Part::FCheek));
        if s.mouth > 0.08 {
            out.push(ell(hp([0.0, -0.16]), 0.055 * hs, 0.03 * hs * s.mouth + 0.01, h_ang, Part::Mouth));
        }
        out.push(ell(hp([0.0, -0.10]), 0.07 * hs, 0.065 * hs, h_ang, Part::FMuzzle));
        out.push(cap(hp([0.0, -0.01]), hp([0.0, -0.09]), r(0.035 * hs), r(0.045 * hs), Part::FMuzzle));
        if s.tongue > 0.05 {
            out.push(cap(hp([0.0, -0.155]), hp([0.0, -0.165 - 0.1 * s.tongue]), r(0.035 * hs), r(0.04 * hs), Part::Tongue));
        }
        out.push(ell(hp([0.0, -0.075]), r(0.045 * hs), r(0.03 * hs), h_ang, Part::Nose));
    }
    for sh in &mut out[head_from..] {
        sh.grp = 6;
    }
    // near legs
    leg_shapes(&mut out, &s.legs.nh, false, false, 1);
    leg_shapes(&mut out, &s.legs.nf, true, false, 2);
    // toy in mouth
    if let Some(toy) = s.toy {
        let big = toy == Toy::Coconut;
        let at = if s.head_front {
            hp([0.0, if big { -0.22 } else { -0.19 }])
        } else {
            hp([0.20 * b.muzzle + if big { 0.06 } else { 0.07 }, if big { -0.15 } else { -0.12 }])
        };
        out.push(toy_shape(toy.kind(), at));
    }
    out
}

// ---------------------------------------------------------------- markings

/// Coat markings: which colour family a point on a body part gets.
pub fn paint(s: &Shape, _x: f64, _y: f64, look: &Look, nz: f64, hh: &Hit) -> i8 {
    use fam::*;
    let d = look.detail;
    let fr = if d != 0 { nz * 0.10 * d as f64 } else { 0.0 };
    let (ux, uy, v, t) = (hh.ux, hh.uy, hh.v, hh.t);
    match s.part {
        Part::Torso => {
            let thr = -0.22 + 1.5 * js_max(0.0, 0.2 - t) + 1.8 * js_max(0.0, t - 0.7);
            if v > thr + fr {
                BLACK
            } else if v < -0.62 {
                CREAM
            } else {
                TAN
            }
        }
        Part::Rump => {
            if uy > -0.02 + fr && ux > -0.45 + 0.3 * uy {
                BLACK
            } else if ux < -0.45 && uy < 0.1 {
                CREAM
            } else {
                TAN
            }
        }
        Part::Brisket => {
            if uy < -0.55 {
                CREAM
            } else {
                TAN
            }
        }
        Part::Chest => {
            if uy > 0.5 + fr && ux < 0.0 {
                BLACK
            } else if uy < -0.6 {
                CREAM
            } else {
                TAN
            }
        }
        Part::Neck => {
            if s.collar && t > 0.5 && t < 0.68 {
                COLLAR
            } else if v > 0.45 + fr * 2.0 && t < 0.4 {
                BLACK
            } else {
                TAN
            }
        }
        Part::Skull => {
            let (ex, ey) = (ux - 0.5, uy - 0.26);
            if d >= 1 && js_hypot(&[ux - 0.40, uy - 0.62]) < 0.15 {
                CREAM // eyebrow spot
            } else if d >= 2 && ex > -0.2 && ex < 0.55 && (ey + 0.35 * ex).abs() < 0.1 {
                MASK // eye line to muzzle
            } else if ux > 0.55 + fr && uy < 0.15 {
                MASK
            } else if d >= 1 && uy > 0.72 + fr && ux < -0.1 {
                BLACK // dark crown
            } else {
                TAN
            }
        }
        Part::Muzzle => {
            if t < 0.32 && v < -0.15 {
                TAN
            } else {
                MASK
            }
        }
        Part::Jaw => {
            if t > 0.42 {
                MASK
            } else {
                TAN
            }
        }
        Part::Nose => NOSE,
        Part::Tongue => TONGUE,
        Part::Mouth => MOUTH,
        Part::EarN => {
            if d == 0 || hh.e0 < 0.035 || hh.e1 < 0.022 || hh.h > 0.8 {
                BLACK
            } else {
                EAR_IN
            }
        }
        Part::EarF => BLACK,
        Part::EarFront => {
            if d == 0 || hh.e0 < 0.03 || hh.e1 < 0.03 || hh.h > 0.8 {
                BLACK
            } else {
                EAR_IN
            }
        }
        Part::FSkull => {
            if d >= 1 && ux.abs() < 0.07 && uy > 0.2 {
                MASK // forehead line
            } else if d >= 1 && js_hypot(&[ux.abs() - 0.40, uy - 0.48]) < 0.13 {
                CREAM // eyebrows
            } else if d >= 2 && ux.abs() < 0.62 && uy < 0.12 - 0.4 * ux.abs() + fr {
                MASK
            } else {
                TAN
            }
        }
        Part::FCheek => {
            if ux.abs() < 0.36 + fr {
                MASK
            } else {
                TAN
            }
        }
        Part::FMuzzle => MASK,
        Part::Tail => {
            if (s.idx == 3 && t > 0.35) || v > -0.2 + fr {
                BLACK
            } else if v < -0.6 {
                CREAM
            } else {
                TAN
            }
        }
        Part::Thigh => {
            if t < 0.32 && v > 0.25 + fr {
                BLACK
            } else if v < -0.35 {
                CREAM
            } else {
                TAN
            }
        }
        Part::LegU => {
            if v < -0.4 {
                CREAM
            } else {
                TAN
            }
        }
        Part::Leg => {
            if v < 0.15 {
                CREAM
            } else {
                TAN
            }
        }
        Part::Paw => CREAM,
        Part::Ball | Part::Coco | Part::FBall => paint_toy(s, hh),
        _ => TAN,
    }
}

pub fn eyes(out: &mut [u8], s: &Skel, b: &Body, look: &Look, fs: FrameSize, pal: &Palette) {
    let hs = b.head * if s.head_front { 1.15 } else { 1.0 };
    let ang = if s.head_front { s.tilt } else { s.head_rot };
    let pts: &[P] = if s.head_front { &[[-0.08, 0.04], [0.08, 0.04]] } else { &[[0.095, 0.04]] };
    for p in pts {
        let w = add(s.head, rot(mul(*p, hs), ang));
        let (ix, iy) = px_of(fs, look, w);
        if s.eyes > 0.5 {
            put(out, fs, ix, iy, pal.eye);
            if look.detail >= 2 {
                if s.head_front {
                    put(out, fs, ix, iy - 1, pal.iris);
                } else {
                    put(out, fs, ix - 1, iy, pal.iris);
                }
            }
            if look.detail >= 2 && s.head_front {
                put(out, fs, ix, iy + 1, pal.eye);
            }
        } else {
            put(out, fs, ix, iy, pal.eye);
            put(out, fs, ix - 1, iy, pal.eye);
            if look.detail >= 2 {
                put(out, fs, ix + 1, iy, pal.eye);
            }
        }
    }
}
