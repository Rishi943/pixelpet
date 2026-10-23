//! From points clicked on photos to the maker's sliders and coat colours.

use super::Photo;
use crate::rig::{Design, Pet, Shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Standing,
    Sitting,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    Nose,
    Eye,
    EarTip,
    Shoulders,
    TailBase,
    TailTip,
    FrontPaw,
    /// Clicks on the fur whose colour sets a Coat area: the main coat; the
    /// saddle or patches (skipped: a solid coat); a dog's legs and muzzle; a
    /// rabbit's inner ear.
    CoatColour,
    MarkColour,
    LegColour,
    FaceColour,
    EarColour,
}

/// The points the maker asks for, in order: the body, then the colours.
pub fn points(pet: Pet) -> &'static [Point] {
    use Point::*;
    match pet {
        Pet::Dog => &[Nose, Eye, EarTip, Shoulders, TailBase, TailTip, FrontPaw, CoatColour, MarkColour, LegColour, FaceColour],
        Pet::Rabbit => &[Nose, Eye, EarTip, Shoulders, TailBase, FrontPaw, CoatColour, MarkColour, EarColour],
    }
}

/// The colour clicks, each with the colour family it sets.
fn colour_fam(pet: Pet, p: Point) -> Option<i8> {
    use crate::rig::core::fam;
    Some(match (pet, p) {
        (Pet::Dog, Point::CoatColour) => fam::TAN,
        (Pet::Dog, Point::MarkColour) => fam::BLACK,
        (Pet::Dog, Point::LegColour) => fam::CREAM,
        (Pet::Dog, Point::FaceColour) => fam::MASK,
        (Pet::Rabbit, Point::CoatColour) => fam::WHITE,
        (Pet::Rabbit, Point::MarkColour) => fam::BROWN,
        (Pet::Rabbit, Point::EarColour) => fam::PINK,
        _ => return None,
    })
}

/// The maker's prompt line for a point.
pub fn prompt(pet: Pet, p: Point) -> &'static str {
    use Point::*;
    match (pet, p) {
        (_, Nose) => "Click the nose",
        (_, Eye) => "Click an eye",
        (_, EarTip) => "Click the tip of an ear",
        (Pet::Dog, Shoulders) => "Click the shoulders",
        (Pet::Rabbit, Shoulders) => "Click the top of the back",
        (Pet::Dog, TailBase) => "Click the base of the tail",
        (Pet::Rabbit, TailBase) => "Click the tail",
        (_, TailTip) => "Click the tip of the tail",
        (Pet::Dog, FrontPaw) => "Click a front paw",
        (Pet::Rabbit, FrontPaw) => "Click a front foot",
        (_, CoatColour) => "Click the main coat colour",
        (Pet::Dog, MarkColour) => "Saddle colour (or Skip)",
        (Pet::Rabbit, MarkColour) => "Patch colour (or Skip)",
        (_, LegColour) => "Click the leg colour",
        (_, FaceColour) => "Click the muzzle colour",
        (_, EarColour) => "Click inside an ear",
    }
}

/// One photo with its pose and the points clicked on it, in photo pixels.
/// Skipped points are simply missing.
pub struct Marked<'a> {
    pub photo: &'a Photo,
    pub pose: Pose,
    pub at: Vec<(Point, (f32, f32))>,
}

/// What the photos gave: a whole shape and coat for the design, plus the
/// names of the sliders the photos actually set (the rest are the base's).
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub shape: Shape,
    pub coat: Vec<(i8, u32)>,
    pub set: Vec<&'static str>,
}


// ---------------------------------------------------------------- geometry

type P = (f32, f32);

/// The clicked points of one photo, looked up by kind.
#[derive(Default)]
struct Pts {
    nose: Option<P>,
    eye: Option<P>,
    ear: Option<P>,
    sh: Option<P>,
    tb: Option<P>,
    tt: Option<P>,
    fp: Option<P>,
}

impl Pts {
    fn of(m: &Marked) -> Pts {
        let mut r = Pts::default();
        for &(k, p) in &m.at {
            if !(p.0.is_finite() && p.1.is_finite()) {
                continue;
            }
            let slot = match k {
                Point::Nose => &mut r.nose,
                Point::Eye => &mut r.eye,
                Point::EarTip => &mut r.ear,
                Point::Shoulders => &mut r.sh,
                Point::TailBase => &mut r.tb,
                Point::TailTip => &mut r.tt,
                Point::FrontPaw => &mut r.fp,
                _ => continue,
            };
            *slot = Some(p);
        }
        r
    }
}

fn dist(a: P, b: P) -> f64 {
    (((a.0 - b.0) as f64).powi(2) + ((a.1 - b.1) as f64).powi(2)).sqrt()
}

/// A distance between two points, if both are there and far enough apart to
/// mean anything (under a pixel is a double click, not a body part).
fn gap(a: Option<P>, b: Option<P>) -> Option<f64> {
    let d = dist(a?, b?);
    (d >= 1.0).then_some(d)
}

// Reference ratios: what a typical dog's side-on photo gives, so that it
// gets 1.0 (Alex's proportions) on every slider. Measured on 2026-10-23 from
// clicks on side-on photos of a Lab, Husky, Golden, Border Collie and beagle:
// legs and muzzle are their median, ears the prick-eared Husky's (a Lab then
// gives 0.79, as the Yellow Lab breed has), tail the median. Alex's one
// standing photo turns towards the camera, which skewed every ratio. L is the
// shoulders-to-tail-base length.
const DOG_EARS: f64 = 0.29; // ear tip to eye / L
const DOG_MUZZLE: f64 = 0.16; // nose to eye / L
const DOG_LEGS: f64 = 0.9; // shoulders to front paw / L
const DOG_TAIL: f64 = 0.45; // tail base to tail tip / L
// Alex the bun, from the lying photo (WhatsApp ... (10)) and the sitting-up
// one (... (12)): nose to eye over the top-of-back-to-tail length, and the
// eye-to-ear-tip length over nose to eye. Rougher than the dog's.
const BUN_HEAD: f64 = 0.443;
const BUN_EARS: f64 = 2.45;

/// Each slider's value from one photo's points: (name, ratio over its
/// reference). Sliders the photo can't speak to are left out.
fn ratios(pet: Pet, pose: Pose, p: &Pts) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    if pose == Pose::Other {
        return out;
    }
    let l = gap(p.sh, p.tb);
    let mut add = |name, num: Option<f64>, den: Option<f64>, reference: f64| {
        if let (Some(n), Some(d)) = (num, den) {
            out.push((name, n / d / reference));
        }
    };
    match pet {
        Pet::Dog => {
            // No Head: nose to ear tip mixes in how tall the ears are, so
            // every floppy-eared dog came out at the smallest head.
            add("ears", gap(p.ear, p.eye), l, DOG_EARS);
            add("muzzle", gap(p.nose, p.eye), l, DOG_MUZZLE);
            // A sitting dog's legs and tail are folded, so they say nothing.
            if pose == Pose::Standing {
                add("legs", gap(p.sh, p.fp), l, DOG_LEGS);
                add("tail", gap(p.tb, p.tt), l, DOG_TAIL);
                // No Body: the only length to measure it against is the head,
                // and Head already uses that same ratio.
            }
        }
        Pet::Rabbit => {
            let eye = gap(p.nose, p.eye);
            add("head", eye, l, BUN_HEAD);
            add("ears", gap(p.eye, p.ear), eye, BUN_EARS);
        }
    }
    out
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

// ---------------------------------------------------------------- colour

const SAMPLES: usize = 4000;

fn lin(c: f64) -> f64 {
    if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    }
}

fn unlin(c: f64) -> f64 {
    if c > 0.0031308 {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * c
    }
}

type Lab = [f64; 3];

fn to_lab(rgb: [u8; 3]) -> Lab {
    let [r, g, b] = rgb.map(|c| lin(c as f64 / 255.0));
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f64| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn from_lab([l, a, b]: Lab) -> u32 {
    let fy = (l + 16.0) / 116.0;
    let (fx, fz) = (fy + a / 500.0, fy - b / 200.0);
    let inv = |t: f64| {
        if t * t * t > 0.008856 {
            t * t * t
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };
    let (x, y, z) = (inv(fx) * 0.95047, inv(fy), inv(fz) * 1.08883);
    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let bl = 0.0557 * x - 0.2040 * y + 1.0570 * z;
    let ch = |v: f64| (unlin(v.clamp(0.0, 1.0)) * 255.0).round() as u32;
    (ch(r) << 16) | (ch(g) << 8) | ch(bl)
}

fn sq(a: &Lab, b: &Lab) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum()
}

fn px(ph: &Photo, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * ph.w + x) * 3) as usize;
    [ph.rgb[i], ph.rgb[i + 1], ph.rgb[i + 2]]
}

/// Three-colour k-means in Lab with a fixed seed. Returns each sample's
/// cluster.
fn kmeans(lab: &[Lab]) -> Vec<usize> {
    let k = 3.min(lab.len());
    let mut seed = 0x2545_F491u32;
    let mut c: Vec<Lab> = Vec::new();
    // Distinct starting colours where there are any: a few tries each, then
    // take what comes (a flat patch has only one colour to offer).
    while c.len() < k {
        let mut cand = lab[0];
        for _ in 0..8 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            cand = lab[(seed >> 8) as usize % lab.len()];
            if !c.contains(&cand) {
                break;
            }
        }
        c.push(cand);
    }
    let mut of = vec![0usize; lab.len()];
    for _ in 0..10 {
        for (i, l) in lab.iter().enumerate() {
            of[i] = (0..k).min_by(|&a, &b| sq(l, &c[a]).total_cmp(&sq(l, &c[b]))).unwrap_or(0);
        }
        for j in 0..k {
            let (mut s, mut n) = ([0.0; 3], 0.0);
            for (l, _) in lab.iter().zip(&of).filter(|(_, &o)| o == j) {
                for i in 0..3 {
                    s[i] += l[i];
                }
                n += 1.0;
            }
            if n > 0.0 {
                c[j] = s.map(|v| v / n);
            }
        }
    }
    of
}

/// The fur's colour round a colour click: the most common of three colours
/// in a small disc (so a stray whisker or bit of shadow doesn't win), as Lab.
/// None if the click is off the photo.
fn colour_at(ph: &Photo, at: P) -> Option<Lab> {
    let r = (0.015 * ph.w.max(ph.h) as f32).max(2.0);
    let (x0, x1) = ((at.0 - r).floor().max(0.0) as u32, ((at.0 + r).ceil().max(0.0) as u32).min(ph.w.saturating_sub(1)));
    let (y0, y1) = ((at.1 - r).floor().max(0.0) as u32, ((at.1 + r).ceil().max(0.0) as u32).min(ph.h.saturating_sub(1)));
    let mut labs = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if x < ph.w && y < ph.h && (x as f32 + 0.5 - at.0).powi(2) + (y as f32 + 0.5 - at.1).powi(2) <= r * r {
                labs.push(to_lab(px(ph, x, y)));
            }
        }
    }
    if labs.is_empty() {
        return None;
    }
    let step = labs.len().div_ceil(SAMPLES);
    let labs: Vec<Lab> = labs.into_iter().step_by(step).collect();
    let of = kmeans(&labs);
    let best = (0..3).max_by_key(|&j| of.iter().filter(|&&o| o == j).count())?;
    let mut ch: [Vec<f64>; 3] = Default::default();
    for (l, _) in labs.iter().zip(&of).filter(|(_, &o)| o == best) {
        for i in 0..3 {
            ch[i].push(l[i]);
        }
    }
    Some([median(&mut ch[0]), median(&mut ch[1]), median(&mut ch[2])])
}

// ---------------------------------------------------------------- fit

/// Work out the sliders and colours from the marked photos, starting from
/// `base`.
pub fn fit(base: &Design, photos: &[Marked]) -> Fit {
    use crate::rig::core::fam;
    let mut shape = base.shape;
    let mut coat = base.coat.clone();
    let mut set = Vec::new();

    // Proportions: the median over photos, clamped to the slider's range.
    let mut per: Vec<(&'static str, Vec<f64>)> = Vec::new();
    // Colours: every photo's pick per area, pooled below.
    let mut cols: Vec<(i8, Vec<Lab>)> = Vec::new();
    for m in photos {
        let pts = Pts::of(m);
        for (name, v) in ratios(base.pet, m.pose, &pts) {
            if !v.is_finite() {
                continue;
            }
            match per.iter_mut().find(|(n, _)| *n == name) {
                Some((_, vs)) => vs.push(v),
                None => per.push((name, vec![v])),
            }
        }
        if m.photo.w == 0 || m.photo.h == 0 || m.photo.rgb.len() < (m.photo.w * m.photo.h * 3) as usize {
            continue;
        }
        for &(pt, at) in &m.at {
            let (Some(f), true) = (colour_fam(base.pet, pt), at.0.is_finite() && at.1.is_finite()) else { continue };
            if let Some(lab) = colour_at(m.photo, at) {
                match cols.iter_mut().find(|(g, _)| *g == f) {
                    Some((_, v)) => v.push(lab),
                    None => cols.push((f, vec![lab])),
                }
            }
        }
    }
    // The saddle or patches skipped everywhere but the coat clicked: one
    // solid coat (a dog's saddle a shade darker, as the Yellow Lab has).
    let (main, mark) = match base.pet {
        Pet::Dog => (fam::TAN, fam::BLACK),
        Pet::Rabbit => (fam::WHITE, fam::BROWN),
    };
    if !cols.iter().any(|c| c.0 == mark) {
        if let Some(c) = cols.iter().find(|c| c.0 == main).map(|c| c.1.clone()) {
            let darker = if base.pet == Pet::Dog { 5.0 } else { 0.0 };
            cols.push((mark, c.iter().map(|l| [l[0] - darker, l[1], l[2]]).collect()));
        }
    }
    for s in crate::rig::breeds::sliders(base.pet) {
        if let Some((_, vs)) = per.iter_mut().find(|(n, _)| *n == s.field) {
            shape.set(s.field, median(vs).clamp(s.lo, s.hi));
            set.push(s.field);
        }
    }
    for (f, labs) in &mut cols {
        let mid: Vec<f64> = (0..3).map(|i| median(&mut labs.iter().map(|l| l[i]).collect::<Vec<_>>())).collect();
        let c = from_lab([mid[0], mid[1], mid[2]]);
        match coat.iter_mut().find(|(g, _)| g == f) {
            Some(e) => e.1 = c,
            None => coat.push((*f, c)),
        }
    }
    Fit { shape, coat, set }
}
