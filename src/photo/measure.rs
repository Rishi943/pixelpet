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
    Chest,
    FrontPaw,
    BackPaw,
}

/// The points the maker asks for, in order.
pub fn points(pet: Pet) -> &'static [Point] {
    use Point::*;
    match pet {
        Pet::Dog => &[Nose, Eye, EarTip, Shoulders, TailBase, TailTip, Chest, FrontPaw, BackPaw],
        Pet::Rabbit => &[Nose, Eye, EarTip, Shoulders, TailBase, Chest, FrontPaw, BackPaw],
    }
}

/// The maker's prompt line for a point.
pub fn prompt(pet: Pet, p: Point) -> &'static str {
    use Point::*;
    match (pet, p) {
        (_, Nose) => "Click the nose",
        (_, Eye) => "Click an eye",
        (_, EarTip) => "Click the tip of an ear",
        (Pet::Dog, Shoulders) => "Click the top of the shoulders",
        (Pet::Rabbit, Shoulders) => "Click the top of the back",
        (Pet::Dog, TailBase) => "Click where the tail starts",
        (Pet::Rabbit, TailBase) => "Click the tail",
        (_, TailTip) => "Click the tip of the tail",
        (_, Chest) => "Click the chest",
        (Pet::Dog, FrontPaw) => "Click a front paw",
        (Pet::Rabbit, FrontPaw) => "Click a front foot",
        (Pet::Dog, BackPaw) => "Click a back paw",
        (Pet::Rabbit, BackPaw) => "Click a back foot",
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
    chest: Option<P>,
    fp: Option<P>,
    bp: Option<P>,
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
                Point::Chest => &mut r.chest,
                Point::FrontPaw => &mut r.fp,
                Point::BackPaw => &mut r.bp,
            };
            *slot = Some(p);
        }
        r
    }

    fn all(&self) -> Vec<P> {
        [self.nose, self.eye, self.ear, self.sh, self.tb, self.tt, self.chest, self.fp, self.bp].into_iter().flatten().collect()
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

// Reference ratios: each one is what Alex's own photos give, so that his
// photos give 1.0 on every slider. Measured from the clicks of the 2026-10-23
// prototype on the standing photo (IMG-20180920-WA0002, taps in a 1000 px
// thumbnail), with eye and chest read off the same photo by eye. L is the
// shoulders-to-tail-base length.
const DOG_HEAD: f64 = 0.594; // nose to ear tip / L
const DOG_EARS: f64 = 0.434; // ear tip to eye / L
const DOG_MUZZLE: f64 = 0.238; // nose to eye / L
const DOG_LEGS: f64 = 1.243; // shoulders to front paw / L
const DOG_TAIL: f64 = 0.514; // tail base to tail tip / L
const DOG_BODY: f64 = 1.0 / DOG_HEAD; // L / nose to ear tip
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
            let span = gap(p.nose, p.ear);
            add("head", span, l, DOG_HEAD);
            add("ears", gap(p.ear, p.eye), l, DOG_EARS);
            add("muzzle", gap(p.nose, p.eye), l, DOG_MUZZLE);
            // A sitting dog's legs and tail are folded, so they say nothing.
            if pose == Pose::Standing {
                add("legs", gap(p.sh, p.fp), l, DOG_LEGS);
                add("tail", gap(p.tb, p.tt), l, DOG_TAIL);
                add("body", l, span, DOG_BODY);
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
    let f = |t: f64| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn from_lab([l, a, b]: Lab) -> u32 {
    let fy = (l + 16.0) / 116.0;
    let (fx, fz) = (fy + a / 500.0, fy - b / 200.0);
    let inv = |t: f64| if t * t * t > 0.008856 { t * t * t } else { (t - 16.0 / 116.0) / 7.787 };
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

/// An area of the photo, worked out from the points.
enum Mask {
    Disc(P, f32),
    /// Every point within `r` of the segment a-b.
    Capsule(P, P, f32),
    /// The strip along a-b from 15% to 85% of its length, `w` to each side.
    Band(P, P, f32),
}

impl Mask {
    fn bounds(&self) -> (f32, f32, f32, f32) {
        let (a, b, r) = match *self {
            Mask::Disc(c, r) => (c, c, r),
            Mask::Capsule(a, b, r) | Mask::Band(a, b, r) => (a, b, r),
        };
        (a.0.min(b.0) - r, a.1.min(b.1) - r, a.0.max(b.0) + r, a.1.max(b.1) + r)
    }

    fn has(&self, x: f32, y: f32) -> bool {
        match *self {
            Mask::Disc(c, r) => (x - c.0).powi(2) + (y - c.1).powi(2) <= r * r,
            Mask::Capsule(a, b, r) | Mask::Band(a, b, r) => {
                let (ux, uy) = (b.0 - a.0, b.1 - a.1);
                let len2 = (ux * ux + uy * uy).max(1e-6);
                let t = ((x - a.0) * ux + (y - a.1) * uy) / len2;
                if let Mask::Band(..) = self {
                    let off = ((x - a.0) * uy - (y - a.1) * ux) / len2.sqrt();
                    return t > 0.15 && t < 0.85 && off.abs() <= r;
                }
                let t = t.clamp(0.0, 1.0);
                (x - (a.0 + t * ux)).powi(2) + (y - (a.1 + t * uy)).powi(2) <= r * r
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Pick {
    Dominant,
    Darkest,
    Lightest,
}

/// One colour area and where to look for it.
struct Job {
    fam: i8,
    mask: Mask,
    pick: Pick,
}

fn lerp(a: P, b: P, t: f32) -> P {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// The areas to sample on one photo, from the points it has.
fn jobs(pet: Pet, p: &Pts) -> Vec<Job> {
    use crate::rig::core::fam;
    let mut v = Vec::new();
    // A length to size the regions by: shoulders to tail base, or failing
    // that a guess from the head.
    let unit = gap(p.sh, p.tb).or_else(|| match pet {
        Pet::Dog => gap(p.nose, p.ear).map(|h| h / DOG_HEAD),
        Pet::Rabbit => gap(p.nose, p.eye).map(|h| h / BUN_HEAD),
    });
    let Some(u) = unit.map(|u| u as f32) else {
        return v;
    };
    match pet {
        Pet::Dog => {
            if let (Some(sh), Some(fp)) = (p.sh, p.fp) {
                v.push(Job { fam: fam::TAN, mask: Mask::Capsule(lerp(fp, sh, 0.15), lerp(fp, sh, 0.7), 0.045 * u), pick: Pick::Dominant });
            }
            if let (Some(sh), Some(tb)) = (p.sh, p.tb) {
                v.push(Job { fam: fam::BLACK, mask: Mask::Band(sh, tb, 0.2 * u), pick: Pick::Darkest });
            }
            if let Some(c) = p.chest {
                v.push(Job { fam: fam::CREAM, mask: Mask::Disc(c, 0.08 * u), pick: Pick::Lightest });
            }
            if let (Some(n), Some(e)) = (p.nose, p.eye) {
                let r = 0.25 * dist(n, e) as f32;
                v.push(Job { fam: fam::MASK, mask: Mask::Disc(lerp(n, e, 0.4), r), pick: Pick::Darkest });
            }
        }
        Pet::Rabbit => {
            if let Some(c) = p.chest {
                v.push(Job { fam: fam::WHITE, mask: Mask::Disc(c, 0.12 * u), pick: Pick::Lightest });
            }
            if let (Some(sh), Some(tb)) = (p.sh, p.tb) {
                v.push(Job { fam: fam::BROWN, mask: Mask::Band(sh, tb, 0.2 * u), pick: Pick::Darkest });
            }
            if let (Some(t), Some(e)) = (p.ear, p.eye) {
                let r = 0.08 * dist(t, e) as f32;
                v.push(Job { fam: fam::PINK, mask: Mask::Disc(lerp(t, e, 0.35), r), pick: Pick::Dominant });
            }
        }
    }
    v
}

fn px(ph: &Photo, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * ph.w + x) * 3) as usize;
    [ph.rgb[i], ph.rgb[i + 1], ph.rgb[i + 2]]
}

/// The photo's background colour: the median of its border.
fn border_median(ph: &Photo) -> [f64; 3] {
    let mut ch: [Vec<f64>; 3] = Default::default();
    let t = 4.min(ph.w / 2).min(ph.h / 2).max(1);
    for y in 0..ph.h {
        for x in 0..ph.w {
            if x < t || y < t || x + t >= ph.w || y + t >= ph.h {
                let c = px(ph, x, y);
                for i in 0..3 {
                    ch[i].push(c[i] as f64);
                }
            }
        }
    }
    [median(&mut ch[0]), median(&mut ch[1]), median(&mut ch[2])]
}

/// Grey-world white balance: per-channel gains that make the average of the
/// photo outside the pet grey, keeping its brightness. The pet's own box (the
/// clicked points, padded) is left out so its colours don't skew it.
fn gains(ph: &Photo, pts: &[P]) -> [f64; 3] {
    let pad = 0.05 * ph.w.max(ph.h) as f32;
    let bx = pts.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)));
    let mut sum = [[0.0f64; 3]; 2]; // [outside, all]
    let mut n = [0u64; 2];
    for y in 0..ph.h {
        for x in 0..ph.w {
            let c = px(ph, x, y);
            let inside = !pts.is_empty()
                && (x as f32) >= bx.0 - pad
                && (x as f32) <= bx.2 + pad
                && (y as f32) >= bx.1 - pad
                && (y as f32) <= bx.3 + pad;
            for k in [1, 0] {
                if k == 1 || !inside {
                    n[k] += 1;
                    for i in 0..3 {
                        sum[k][i] += c[i] as f64;
                    }
                }
            }
        }
    }
    let k = if n[0] * 100 >= n[1] { 0 } else { 1 };
    let m = sum[k].map(|s| s / n[k].max(1) as f64);
    let lum = (m[0] + m[1] + m[2]) / 3.0;
    m.map(|c| if c > 1.0 { (lum / c).clamp(0.7, 1.4) } else { 1.0 })
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

/// The colour of one job on one photo, as Lab; None if it found no pet.
fn sample(ph: &Photo, job: &Job, bg: [f64; 3], gain: [f64; 3], strict: bool) -> Option<Lab> {
    let (x0, y0, x1, y1) = job.mask.bounds();
    let (xa, xb) = ((x0.floor().max(0.0)) as u32, (x1.ceil().max(0.0) as u32).min(ph.w.saturating_sub(1)));
    let (ya, yb) = ((y0.floor().max(0.0)) as u32, (y1.ceil().max(0.0) as u32).min(ph.h.saturating_sub(1)));
    let mut all: Vec<[u8; 3]> = Vec::new();
    for y in ya..=yb {
        for x in xa..=xb {
            if x < ph.w && y < ph.h && job.mask.has(x as f32 + 0.5, y as f32 + 0.5) {
                all.push(px(ph, x, y));
            }
        }
    }
    // Reject the background and, for dogs, highlights and tongue; but if that
    // leaves nothing the area is probably the same colour as the wall.
    let far = |c: &[u8; 3]| ((0..3).map(|i| (c[i] as f64 - bg[i]).powi(2)).sum::<f64>()).sqrt() > 28.0;
    // (Sunlit fur reads near white; the lightest pick skips that glare.)
    let glare = if matches!(job.pick, Pick::Lightest) { 85.0 } else { 101.0 };
    let tongue = |l: &Lab| (l[1] > 28.0 && l[2] < 0.6 * l[1]) || l[0] > glare;
    let balanced = |c: &[u8; 3]| [0, 1, 2].map(|i| (c[i] as f64 * gain[i]).round().clamp(0.0, 255.0) as u8);
    let mut keep: Vec<([u8; 3], Lab)> = all
        .iter()
        .filter(|c| far(c) && !(strict && c.iter().copied().min().unwrap_or(0) > 225))
        .map(|c| {
            let b = balanced(c);
            (b, to_lab(b))
        })
        .filter(|(_, l)| !(strict && tongue(l)))
        .collect();
    if keep.len() < 30 {
        keep = all.iter().map(|c| balanced(c)).map(|b| (b, to_lab(b))).collect();
    }
    if keep.len() < 30 {
        return None;
    }
    let step = keep.len().div_ceil(SAMPLES);
    let keep: Vec<_> = keep.into_iter().step_by(step).collect();
    let labs: Vec<Lab> = keep.iter().map(|k| k.1).collect();
    let of = kmeans(&labs);
    // (size, mean L) per cluster; clusters under a tenth don't get to win
    // the darkest or lightest.
    let mut st = [(0usize, 0.0f64); 3];
    for (l, &o) in labs.iter().zip(&of) {
        st[o].0 += 1;
        st[o].1 += l[0];
    }
    let n = labs.len();
    let ok = |j: usize| st[j].0 > 0 && (st[j].0 * 10 >= n || matches!(job.pick, Pick::Dominant));
    let mean = |j: usize| st[j].1 / st[j].0 as f64;
    let best = (0..3).filter(|&j| ok(j)).max_by(|&a, &b| match job.pick {
        Pick::Dominant => st[a].0.cmp(&st[b].0),
        Pick::Darkest => mean(b).total_cmp(&mean(a)),
        Pick::Lightest => mean(a).total_cmp(&mean(b)),
    })?;
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
        let todo = jobs(base.pet, &pts);
        if todo.is_empty() || m.photo.w == 0 || m.photo.h == 0 || m.photo.rgb.len() < (m.photo.w * m.photo.h * 3) as usize {
            continue;
        }
        let (bg, gain) = (border_median(m.photo), gains(m.photo, &pts.all()));
        for job in &todo {
            if let Some(lab) = sample(m.photo, job, bg, gain, base.pet == Pet::Dog) {
                match cols.iter_mut().find(|(f, _)| *f == job.fam) {
                    Some((_, v)) => v.push(lab),
                    None => cols.push((job.fam, vec![lab])),
                }
            }
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
