//! Shared rig code, ported from `drafts/looks/rig.js`: vector maths, shapes,
//! hit tests, shading, rasterising, despeckle, edge separation and outline.
//!
//! Exactness rules (so frames match the JS pixel for pixel):
//! - `hash` uses wrapping i32 maths and unsigned shifts like `Math.imul`/`>>>`.
//! - every `Math.round` is `js_round`, every `Math.hypot` is `js_hypot`.
//! - float expressions keep the JS evaluation order.

use std::sync::OnceLock;

pub type P = [f64; 2];

pub const TAU: f64 = std::f64::consts::PI * 2.0;
pub const PI: f64 = std::f64::consts::PI;
pub const PAW_Y: f64 = 0.035;

pub fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1]]
}
pub fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
pub fn mul(a: P, s: f64) -> P {
    [a[0] * s, a[1] * s]
}
pub fn len(a: P) -> f64 {
    js_hypot(&[a[0], a[1]])
}
pub fn rot(a: P, t: f64) -> P {
    let (c, s) = (t.cos(), t.sin());
    [a[0] * c - a[1] * s, a[0] * s + a[1] * c]
}
/// `Math.max(a, Math.min(b, x))`
pub fn clamp(x: f64, a: f64, b: f64) -> f64 {
    js_max(a, js_min(b, x))
}
/// 0 to 1 by `a`, held, back to 0 from `b` to `c` (smoothstep edges).
pub fn ramp(t: f64, a: f64, b: f64, c: f64) -> f64 {
    let sm = |x: f64| {
        let x = clamp(x, 0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    if t < a {
        sm(t / a)
    } else if t < b {
        1.0
    } else {
        1.0 - sm((t - b) / (c - b))
    }
}

pub fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
pub fn mix_p(a: P, b: P, t: f64) -> P {
    [mix(a[0], b[0], t), mix(a[1], b[1], t)]
}
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.round`: halves round up.
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `Math.hypot` as V8 computes it: normalise by the largest value, then a
/// Kahan-compensated sum of squares.
pub fn js_hypot(v: &[f64]) -> f64 {
    let mut max = 0.0f64;
    for x in v {
        if x.is_nan() {
            return f64::NAN;
        }
        if x.abs() > max {
            max = x.abs();
        }
    }
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut comp) = (0.0f64, 0.0f64);
    for x in v {
        let n = x.abs() / max;
        let summand = n * n - comp;
        let prelim = sum + summand;
        comp = (prelim - sum) - summand;
        sum = prelim;
    }
    sum.sqrt() * max
}

/// Per-pixel fur noise in [-1, 1].
pub fn hash(x: i32, y: i32) -> f64 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ ((h as u32) >> 13) as i32).wrapping_mul(1274126177);
    ((h ^ ((h as u32) >> 16) as i32) & 1023) as f64 / 511.5 - 1.0
}

// ---------------------------------------------------------------- looks

/// A drawing style.
#[derive(Debug, PartialEq)]
pub struct Look {
    pub name: &'static str,
    pub r: f64,
    pub zoom: u32,
    pub tones: u8,
    pub soft_outline: bool,
    pub min_r: f64,
    pub noise: f64,
    pub dither: f64,
    pub detail: u8,
}

pub static CHUNKY: Look = Look { name: "chunky", r: 12.0, zoom: 5, tones: 2, soft_outline: false, min_r: 0.9, noise: 0.0, dither: 0.0, detail: 0 };
pub static CLASSIC: Look = Look { name: "classic", r: 22.0, zoom: 3, tones: 3, soft_outline: false, min_r: 0.8, noise: 0.025, dither: 0.0, detail: 1 };
pub static DETAILED: Look = Look { name: "detailed", r: 36.0, zoom: 2, tones: 4, soft_outline: true, min_r: 0.7, noise: 0.11, dither: 0.08, detail: 2 };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSize {
    pub w: usize,
    pub h: usize,
    pub ox: i32,
    pub gy: i32,
}

pub fn frame_size(look: &Look) -> FrameSize {
    let r = look.r;
    let w = (3.1 * r).ceil() as usize;
    let h = (2.7 * r).ceil() as usize;
    FrameSize { w, h, ox: js_round(1.6 * r) as i32, gy: h as i32 - 2 }
}

// ---------------------------------------------------------------- palette

/// Colour families, in the JS `FAMS` order.
pub mod fam {
    pub const TAN: i8 = 0;
    pub const CREAM: i8 = 1;
    pub const BLACK: i8 = 2;
    pub const MASK: i8 = 3;
    pub const EAR_IN: i8 = 4;
    pub const NOSE: i8 = 5;
    pub const TONGUE: i8 = 6;
    pub const MOUTH: i8 = 7;
    pub const COLLAR: i8 = 8;
    pub const BALL: i8 = 9;
    pub const SEAM: i8 = 10;
    pub const COCO: i8 = 11;
    pub const WHITE: i8 = 12;
    pub const BROWN: i8 = 13;
    pub const EAR_R: i8 = 14;
    pub const PINK: i8 = 15;
    pub const FWHITE: i8 = 16;
    pub const FBLACK: i8 = 17;
    pub const COUNT: usize = 18;
}

/// A coat as written in the drafts: families `[deep, shadow, base, light]`.
pub struct Coat {
    pub fams: &'static [(i8, [u32; 4])],
    pub eye: u32,
    pub iris: u32,
    pub outline: u32,
    pub edge_drop: i8,
}

/// Toys look the same whoever carries them.
pub const TOYS: &[(i8, [u32; 4])] = &[
    (fam::BALL, [0x560910, 0x86121E, 0xB81F2D, 0xE2525C]),
    (fam::SEAM, [0xB0A088, 0xD2C2A6, 0xEADBC0, 0xFFF4E2]),
    (fam::COCO, [0x382010, 0x583418, 0x7A4C25, 0xA26E3E]),
    (fam::FWHITE, [0x9A9A9E, 0xCACACF, 0xEEEEF0, 0xFFFFFF]),
    (fam::FBLACK, [0x0C0C0E, 0x151518, 0x202024, 0x3A3A40]),
];

pub type Rgb = [u8; 3];

pub struct Palette {
    pub fam: [[Rgb; 4]; fam::COUNT],
    pub eye: Rgb,
    pub iris: Rgb,
    pub outline: Rgb,
    pub edge_drop: i8,
}

fn rgb(h: u32) -> Rgb {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

impl Palette {
    /// `compilePalette`: the coat's family, else the toy family, else the coat's tan.
    pub fn compile(c: &Coat) -> Palette {
        let find = |list: &[(i8, [u32; 4])], f: i8| list.iter().find(|(k, _)| *k == f).map(|(_, v)| *v);
        let tan = find(c.fams, fam::TAN).expect("coat has tan");
        let mut out = [[[0u8; 3]; 4]; fam::COUNT];
        for (f, slot) in out.iter_mut().enumerate() {
            let v = find(c.fams, f as i8).or_else(|| find(TOYS, f as i8)).unwrap_or(tan);
            *slot = [rgb(v[0]), rgb(v[1]), rgb(v[2]), rgb(v[3])];
        }
        Palette { fam: out, eye: rgb(c.eye), iris: rgb(c.iris), outline: rgb(c.outline), edge_drop: c.edge_drop }
    }
}

// ---------------------------------------------------------------- shapes

/// Every body part name used by the drafts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    // dog
    EarF, EarN, EarFront, Tail, Rump, Torso, Chest, Neck, Brisket, Mouth, Jaw, Skull, Tongue,
    Muzzle, Nose, FSkull, FCheek, FMuzzle, LegU, Leg, Thigh, Paw,
    /// the back of the head (seen from behind, running into the screen)
    BackHead,
    // toys
    Ball, Coco, FBall,
    // rabbit
    RearF, Rear, RfEar, RLeg, RFoot, RBody, RChest, RHaunch, RTail, RNape, RSkull, RCheek,
    RMuzzle, RNose, RfSkull, RfCheek, RfMuzzle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Cap { a: P, b: P, ra: f64, rb: f64 },
    Ell { c: P, rx: f64, ry: f64, ang: f64 },
    Tri { p: [P; 3] },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub kind: Kind,
    pub part: Part,
    pub grp: i8,
    pub far: bool,
    pub flip: bool,
    pub idx: u8,
    pub no_edge: bool,
    pub collar: bool,
}

impl Shape {
    fn of(kind: Kind, part: Part) -> Shape {
        Shape { kind, part, grp: 0, far: false, flip: false, idx: 0, no_edge: false, collar: false }
    }
    pub fn far(mut self, far: bool) -> Shape {
        self.far = far;
        self
    }
    pub fn grp(mut self, g: i8) -> Shape {
        self.grp = g;
        self
    }
    pub fn flip(mut self) -> Shape {
        self.flip = true;
        self
    }
    pub fn idx(mut self, i: u8) -> Shape {
        self.idx = i;
        self
    }
    pub fn no_edge(mut self) -> Shape {
        self.no_edge = true;
        self
    }
}

pub fn cap(a: P, b: P, ra: f64, rb: f64, part: Part) -> Shape {
    Shape::of(Kind::Cap { a, b, ra, rb }, part)
}
pub fn ell(c: P, rx: f64, ry: f64, ang: f64, part: Part) -> Shape {
    Shape::of(Kind::Ell { c, rx, ry, ang }, part)
}
pub fn tri(p0: P, p1: P, p2: P, part: Part) -> Shape {
    Shape::of(Kind::Tri { p: [p0, p1, p2] }, part)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToyKind {
    Ball,
    Coconut,
    Football,
}

pub fn toy_shape(t: ToyKind, at: P) -> Shape {
    match t {
        ToyKind::Football => ell(at, 0.085, 0.085, 0.2, Part::FBall).grp(9),
        ToyKind::Coconut => ell(at, 0.115, 0.10, 0.3, Part::Coco).grp(9),
        ToyKind::Ball => ell(at, 0.072, 0.072, 0.5, Part::Ball).grp(9),
    }
}

// ---------------------------------------------------------------- hit

fn light() -> &'static [f64; 3] {
    static L: OnceLock<[f64; 3]> = OnceLock::new();
    L.get_or_init(|| {
        let v = [0.35, 0.75, 0.56];
        let n = js_hypot(&v);
        [v[0] / n, v[1] / n, v[2] / n]
    })
}

fn lit_from(nx: f64, ny: f64) -> f64 {
    let l = light();
    let nz = js_max(0.0, 1.0 - nx * nx - ny * ny).sqrt();
    nx * l[0] + ny * l[1] + nz * l[2]
}

fn or_tiny(x: f64) -> f64 {
    // JS `x || 1e-9`
    if x == 0.0 || x.is_nan() {
        1e-9
    } else {
        x
    }
}

fn seg_dist(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = clamp(((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / or_tiny(dx * dx + dy * dy), 0.0, 1.0);
    js_hypot(&[p[0] - a[0] - dx * t, p[1] - a[1] - dy * t])
}

/// What the last hit test found (the JS module-level `H`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    pub lit: f64,
    pub t: f64,
    pub v: f64,
    pub ux: f64,
    pub uy: f64,
    pub e0: f64,
    pub e1: f64,
    pub h: f64,
}

pub fn hit(s: &Shape, x: f64, y: f64, hh: &mut Hit) -> bool {
    match s.kind {
        Kind::Cap { a, b, ra, rb } => {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let l2 = or_tiny(dx * dx + dy * dy);
            let t = clamp(((x - a[0]) * dx + (y - a[1]) * dy) / l2, 0.0, 1.0);
            let r = mix(ra, rb, t);
            let (ex, ey) = (x - a[0] - dx * t, y - a[1] - dy * t);
            if ex * ex + ey * ey > r * r {
                return false;
            }
            let ln = l2.sqrt();
            hh.t = t;
            hh.v = (ex * -dy / ln + ey * dx / ln) / r * if s.flip { -1.0 } else { 1.0 };
            hh.lit = lit_from(ex / r, ey / r);
            true
        }
        Kind::Ell { c: cc, rx, ry, ang } => {
            let (c, sn) = (ang.cos(), ang.sin());
            let (lx, ly) = (x - cc[0], y - cc[1]);
            let ux = (lx * c + ly * sn) / rx;
            let uy = (-lx * sn + ly * c) / ry;
            if ux * ux + uy * uy > 1.0 {
                return false;
            }
            hh.ux = ux;
            hh.uy = uy;
            hh.v = uy;
            hh.t = 0.0;
            hh.lit = lit_from(ux * c - uy * sn, ux * sn + uy * c);
            true
        }
        Kind::Tri { p: [a, b, c] } => {
            let d1 = (x - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (y - b[1]);
            let d2 = (x - c[0]) * (b[1] - c[1]) - (b[0] - c[0]) * (y - c[1]);
            let d3 = (x - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (y - a[1]);
            if (d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0) {
                return false;
            }
            let p = [x, y];
            hh.e0 = seg_dist(p, a, c);
            hh.e1 = seg_dist(p, b, c);
            let m = mul(add(a, b), 0.5);
            let hl = len(sub(c, m));
            hh.h = clamp(((x - m[0]) * (c[0] - m[0]) + (y - m[1]) * (c[1] - m[1])) / (hl * hl), 0.0, 1.0);
            hh.lit = 0.5 + 0.25 * hh.h;
            true
        }
    }
}

/// Toy markings (the toy cases of the dog `paint`).
pub fn paint_toy(s: &Shape, hh: &Hit) -> i8 {
    match s.part {
        Part::Ball => {
            if (hh.ux - 0.15).abs() < 0.13 {
                fam::SEAM
            } else {
                fam::BALL
            }
        }
        Part::Coco => fam::COCO,
        Part::FBall => {
            const SPOTS: [P; 5] = [[0.0, 0.05], [0.8, 0.35], [-0.8, 0.35], [0.5, -0.75], [-0.5, -0.75]];
            if SPOTS.iter().any(|[a, b]| js_hypot(&[hh.ux - a, hh.uy - b]) < 0.3) {
                fam::FBLACK
            } else {
                fam::FWHITE
            }
        }
        _ => fam::TAN,
    }
}

// ---------------------------------------------------------------- raster

const BAYER: [i32; 4] = [0, 2, 3, 1];

fn tone_band(l: f64, look: &Look) -> i32 {
    match look.tones {
        2 => {
            if l < 0.32 {
                1
            } else {
                2
            }
        }
        3 => {
            if l < 0.30 {
                1
            } else if l > 0.84 {
                3
            } else {
                2
            }
        }
        _ => {
            if l < 0.10 {
                0
            } else if l < 0.36 {
                1
            } else if l > 0.80 {
                3
            } else {
                2
            }
        }
    }
}

fn tone_of(lit: f64, look: &Look, nz: f64, ix: i32, iy: i32) -> i32 {
    let t0 = tone_band(lit, look);
    let b = BAYER[((ix & 1) + 2 * (iy & 1)) as usize] as f64;
    let t1 = tone_band(lit + look.noise * nz + look.dither * (b / 4.0 - 0.375), look);
    // fur noise moves a pixel one shade at most
    t1.min(t0 + 1).max(t0 - 1)
}

fn bbox(s: &Shape) -> [f64; 4] {
    match s.kind {
        Kind::Cap { a, b, ra, rb } => {
            let m = js_max(ra, rb);
            [js_min(a[0], b[0]) - m, js_min(a[1], b[1]) - m, js_max(a[0], b[0]) + m, js_max(a[1], b[1]) + m]
        }
        Kind::Ell { c, rx, ry, .. } => {
            let m = js_max(rx, ry);
            [c[0] - m, c[1] - m, c[0] + m, c[1] + m]
        }
        Kind::Tri { p } => {
            let xs = [p[0][0], p[1][0], p[2][0]];
            let ys = [p[0][1], p[1][1], p[2][1]];
            let mn = |v: [f64; 3]| js_min(js_min(v[0], v[1]), v[2]);
            let mx = |v: [f64; 3]| js_max(js_max(v[0], v[1]), v[2]);
            [mn(xs), mn(ys), mx(xs), mx(ys)]
        }
    }
}

/// Per-pixel family / tone / group / edge flag.
pub struct Raster {
    pub fam: Vec<i8>,
    pub tone: Vec<i8>,
    pub grp: Vec<i8>,
    pub edg: Vec<i8>,
}

pub type PaintFn = fn(s: &Shape, x: f64, y: f64, look: &Look, nz: f64, hh: &Hit) -> i8;

pub fn raster(list: &[Shape], look: &Look, fs: FrameSize, paint: PaintFn) -> Raster {
    let (w, h) = (fs.w as i32, fs.h as i32);
    let (ox, gy, r) = (fs.ox as f64, fs.gy as f64, look.r);
    let n = fs.w * fs.h;
    let mut rs = Raster { fam: vec![-1; n], tone: vec![0; n], grp: vec![-1; n], edg: vec![0; n] };
    let mut hh = Hit::default();
    for s in list {
        let b = bbox(s);
        let x0 = clamp((ox + b[0] * r).floor(), 0.0, (w - 1) as f64) as i32;
        let x1 = clamp((ox + b[2] * r).ceil(), 0.0, (w - 1) as f64) as i32;
        let y0 = clamp((gy - b[3] * r).floor(), 0.0, (h - 1) as f64) as i32;
        let y1 = clamp((gy - b[1] * r).ceil(), 0.0, (h - 1) as f64) as i32;
        let anc = match s.kind {
            Kind::Cap { a, .. } => a,
            Kind::Ell { c, .. } => c,
            Kind::Tri { p } => p[0],
        };
        let ax = js_round(anc[0] * r);
        let ay = js_round(anc[1] * r);
        for iy in y0..=y1 {
            for ix in x0..=x1 {
                let x = (ix as f64 + 0.5 - ox) / r;
                let y = (gy - iy as f64 - 0.5) / r;
                if !hit(s, x, y, &mut hh) {
                    continue;
                }
                let nz = hash((ix as f64 - ox - ax) as i32, (iy as f64 + ay) as i32);
                let i = (iy * w + ix) as usize;
                let fm = if s.grp == 9 { paint_toy(s, &hh) } else { paint(s, x, y, look, nz, &hh) };
                let mut tn = tone_of(hh.lit, look, if s.part == Part::Coco { nz * 2.5 } else { nz }, ix, iy);
                if s.far {
                    tn = (tn - 1).max(0);
                }
                rs.fam[i] = fm;
                rs.tone[i] = tn as i8;
                rs.grp[i] = s.grp;
                rs.edg[i] = if s.no_edge { 0 } else { 1 };
            }
        }
    }
    rs
}

pub fn to_rgba(rs: &mut Raster, look: &Look, fs: FrameSize, pal: &Palette) -> Vec<u8> {
    let (w, h) = (fs.w, fs.h);
    let n = w * h;
    let mut out = vec![0u8; n * 4];
    let fam_ = &mut rs.fam;
    // lone pixels of a different colour read as dirt: absorb them
    for iy in 1..h.saturating_sub(1) {
        for ix in 1..w.saturating_sub(1) {
            let i = iy * w + ix;
            let a = fam_[i - 1];
            if a < 0 || fam_[i] == a || fam_[i + 1] != a || fam_[i - w] != a || fam_[i + w] != a {
                continue;
            }
            if fam_[i] == fam::NOSE || fam_[i] == fam::TONGUE {
                continue;
            }
            fam_[i] = a;
        }
    }
    // separate near legs from the body behind them
    if look.detail >= 1 {
        let mut t2 = rs.tone.clone();
        let drop = pal.edge_drop;
        for iy in 0..h {
            for ix in 1..w.saturating_sub(1) {
                let i = iy * w + ix;
                let g = rs.grp[i];
                if (g != 1 && g != 2 && g != 6) || rs.edg[i] == 0 || iy < 1 || iy >= h - 1 {
                    continue;
                }
                for j in [i - 1, i + 1, i - w, i + w] {
                    if rs.grp[j] >= 0 && rs.grp[j] != g && rs.grp[j] != 9 {
                        t2[i] = (rs.tone[i] - drop).max(0);
                        break;
                    }
                }
            }
        }
        rs.tone = t2;
    }
    let fam_ = &rs.fam;
    for i in 0..n {
        if fam_[i] < 0 {
            continue;
        }
        let c = pal.fam[fam_[i] as usize][rs.tone[i] as usize];
        out[i * 4..i * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    }
    // outline
    let o = pal.outline;
    for iy in 0..h {
        for ix in 0..w {
            let i = iy * w + ix;
            if fam_[i] >= 0 {
                continue;
            }
            let nb = if ix > 0 && fam_[i - 1] >= 0 {
                i - 1
            } else if ix < w - 1 && fam_[i + 1] >= 0 {
                i + 1
            } else if iy > 0 && fam_[i - w] >= 0 {
                i - w
            } else if iy < h - 1 && fam_[i + w] >= 0 {
                i + w
            } else {
                continue;
            };
            let mut c = o;
            if look.soft_outline {
                let d = pal.fam[fam_[nb] as usize][0];
                c = [((d[0] as u16 + o[0] as u16) >> 1) as u8, ((d[1] as u16 + o[1] as u16) >> 1) as u8, ((d[2] as u16 + o[2] as u16) >> 1) as u8];
            }
            out[i * 4..i * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    out
}

pub fn put(out: &mut [u8], fs: FrameSize, ix: i32, iy: i32, c: Rgb) {
    if ix < 0 || iy < 0 || ix >= fs.w as i32 || iy >= fs.h as i32 {
        return;
    }
    let i = (iy as usize * fs.w + ix as usize) * 4;
    out[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
}

/// Pixel coordinates of a rig point, as the eye code computes them.
pub fn px_of(fs: FrameSize, look: &Look, w: P) -> (i32, i32) {
    ((fs.ox as f64 + w[0] * look.r).floor() as i32, (fs.gy as f64 - w[1] * look.r).floor() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_js() {
        assert_eq!(hash(0, 0), -1.0);
        assert_eq!(hash(1, 0), -0.0811339198435973);
        assert_eq!(hash(-5, 17), 0.47409579667644186);
        assert_eq!(hash(123, -456), 0.11827956989247301);
        assert_eq!(hash(40000, -70000), -0.33333333333333337);
    }

    #[test]
    fn js_round_is_half_up() {
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(2.5), 3.0);
    }

    #[test]
    fn frame_sizes() {
        let fs = |n: &str| {
            let f = frame_size(crate::rig::look(n));
            (f.w, f.h, f.ox, f.gy)
        };
        assert_eq!(fs("chunky"), (38, 33, 19, 31));
        assert_eq!(fs("classic"), (69, 60, 35, 58));
        assert_eq!(fs("detailed"), (112, 98, 58, 96));
    }
}
