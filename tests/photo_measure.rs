//! Points clicked on photos set the maker's sliders and colours. Photos here
//! are drawn in the test (flat colours on a grey wall, with a colour cast);
//! the one test on real photos is local-only.

use pixelpet::photo::measure::{fit, points, Marked, Point, Pose};
use pixelpet::photo::Photo;
use pixelpet::rig::breeds::{breeds, sliders};
use pixelpet::rig::core::fam;
use pixelpet::rig::design::{shades, Design};
use pixelpet::rig::Pet;

type P = (f32, f32);

// ---------------------------------------------------------------- drawing

struct Img {
    ph: Photo,
}

impl Img {
    fn new(w: u32, h: u32, bg: [u8; 3]) -> Img {
        Img { ph: Photo { w, h, rgb: bg.iter().copied().cycle().take((w * h * 3) as usize).collect() } }
    }

    fn fill(&mut self, c: u32, has: impl Fn(f32, f32) -> bool) {
        let rgb = [(c >> 16) as u8, (c >> 8) as u8, c as u8];
        for y in 0..self.ph.h {
            for x in 0..self.ph.w {
                if has(x as f32 + 0.5, y as f32 + 0.5) {
                    let i = ((y * self.ph.w + x) * 3) as usize;
                    self.ph.rgb[i..i + 3].copy_from_slice(&rgb);
                }
            }
        }
    }

    fn rect(&mut self, c: u32, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.fill(c, |x, y| x >= x0 && x < x1 && y >= y0 && y < y1);
    }

    fn disc(&mut self, c: u32, at: P, r: f32) {
        self.fill(c, |x, y| (x - at.0).powi(2) + (y - at.1).powi(2) <= r * r);
    }

    /// A light colour cast over everything, plus a little fixed noise.
    fn cast(mut self, k: [f64; 3]) -> Photo {
        let mut seed = 7u32;
        for (i, v) in self.ph.rgb.iter_mut().enumerate() {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = ((seed >> 24) % 5) as f64 - 2.0;
            *v = (*v as f64 * k[i % 3] + noise).round().clamp(0.0, 255.0) as u8;
        }
        self.ph
    }
}

fn marked(ph: &Photo, pose: Pose, at: Vec<(Point, P)>) -> Marked<'_> {
    Marked { photo: ph, pose, at }
}

/// Where the eye is, given the nose, the ear tip and the two distances.
fn eye_from(nose: P, ear: P, d_nose: f32, d_ear: f32) -> P {
    let (dx, dy) = (ear.0 - nose.0, ear.1 - nose.1);
    let d = (dx * dx + dy * dy).sqrt();
    let a = (d_nose * d_nose - d_ear * d_ear + d * d) / (2.0 * d);
    let h = (d_nose * d_nose - a * a).max(0.0).sqrt();
    // the solution on the right-hand side of nose->ear (below it, facing right)
    (nose.0 + (a * dx - h * dy) / d, nose.1 + (a * dy + h * dx) / d)
}

// ---------------------------------------------------------------- a dog

/// Shape factors on the reference ratios (1 = Alex's photo).
#[derive(Clone, Copy)]
struct Dog {
    legs: f32,
    head: f32,
    ears: f32,
    muzzle: f32,
    tail: f32,
}

const ONE: Dog = Dog { legs: 1.0, head: 1.0, ears: 1.0, muzzle: 1.0, tail: 1.0 };
const L: f32 = 400.0;
const SH: P = (700.0, 250.0);
const TB: P = (300.0, 250.0);

/// A side-on dog's points on a 1000 x 900 photo, with the proportions the
/// maker reads (Alex's reference ratios times `d`).
fn dog_points(d: Dog) -> Vec<(Point, P)> {
    let nose = (900.0, 200.0);
    let span = d.head * 0.594 * L;
    let ear = (nose.0 - span * 0.8, nose.1 - span * 0.6);
    let eye = eye_from(nose, ear, d.muzzle * 0.238 * L, d.ears * 0.434 * L);
    vec![
        (Point::Nose, nose),
        (Point::Eye, eye),
        (Point::EarTip, ear),
        (Point::Shoulders, SH),
        (Point::TailBase, TB),
        (Point::TailTip, (TB.0, TB.1 + d.tail * 0.514 * L)),
        (Point::Chest, (SH.0 + 40.0, 400.0)),
        (Point::FrontPaw, (SH.0, SH.1 + d.legs * 1.243 * L)),
        (Point::BackPaw, (TB.0, TB.1 + 480.0)),
    ]
}

const TAN: u32 = 0xC47637;
const BLACK: u32 = 0x231B1B;
const CREAM: u32 = 0xE2B077;
const MASK: u32 = 0x2B2220;

/// The dog painted to match `dog_points(ONE)`, grey wall, bluish cast.
fn dog_photo() -> Photo {
    let mut im = Img::new(1000, 900, [150, 150, 150]);
    im.rect(TAN, 300.0, 250.0, 700.0, 450.0); // body
    im.rect(TAN, SH.0 - 18.0, 250.0, SH.0 + 18.0, 800.0); // front leg
    im.rect(BLACK, 330.0, 250.0, 670.0, 310.0); // saddle
    im.rect(CREAM, SH.0 + 40.0 - 35.0, 365.0, SH.0 + 40.0 + 35.0, 435.0); // chest
    let pts = dog_points(ONE);
    let (nose, eye) = (pts[0].1, pts[1].1);
    im.disc(MASK, (nose.0 + (eye.0 - nose.0) * 0.4, nose.1 + (eye.1 - nose.1) * 0.4), 30.0);
    im.cast([0.85, 1.0, 1.15])
}

fn lab(c: u32) -> [f64; 3] {
    let lin = |v: f64| if v > 0.04045 { ((v + 0.055) / 1.055).powf(2.4) } else { v / 12.92 };
    let [r, g, b] = [(c >> 16) & 255, (c >> 8) & 255, c & 255].map(|v| lin(v as f64 / 255.0));
    let (x, y, z) = ((0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047, 0.2126 * r + 0.7152 * g + 0.0722 * b, (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883);
    let f = |t: f64| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    [116.0 * f(y) - 16.0, 500.0 * (f(x) - f(y)), 200.0 * (f(y) - f(z))]
}

fn delta_e(a: u32, b: u32) -> f64 {
    let (a, b) = (lab(a), lab(b));
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

fn colour(f: &pixelpet::photo::measure::Fit, family: i8) -> Option<u32> {
    f.coat.iter().find(|(g, _)| *g == family).map(|&(_, c)| c)
}

// ---------------------------------------------------------------- tests

#[test]
fn zero_photos_return_the_base_design() {
    for pet in [Pet::Dog, Pet::Rabbit] {
        let base = Design::builtin(pet);
        let f = fit(&base, &[]);
        assert_eq!(f.shape, base.shape);
        assert_eq!(f.coat, base.coat);
        assert!(f.set.is_empty());
    }
    // a photo with no points changes nothing either
    let ph = Img::new(50, 50, [9, 9, 9]).ph;
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, vec![])]);
    assert_eq!(f.shape, Design::alex().shape);
    assert!(f.set.is_empty());
}

#[test]
fn alexs_own_proportions_give_one() {
    let ph = Img::new(1000, 900, [150, 150, 150]).ph;
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, dog_points(ONE))]);
    assert_eq!(f.set, ["legs", "body", "head", "ears", "muzzle", "tail"]);
    for name in &f.set {
        let v = f.shape.get(name).unwrap();
        assert!((v - 1.0).abs() < 0.01, "{name} = {v}");
    }
    assert_eq!(f.shape.fluff, 1.0);
}

#[test]
fn known_ratios_come_back() {
    let ph = Img::new(1000, 900, [150, 150, 150]).ph;
    let d = Dog { legs: 1.1, head: 1.2, ears: 1.15, muzzle: 1.1, tail: 0.8 };
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, dog_points(d))]);
    let want = [("legs", 1.1), ("head", 1.2), ("ears", 1.15), ("muzzle", 1.1), ("tail", 0.8), ("body", 1.0 / 1.2)];
    for (name, v) in want {
        let got = f.shape.get(name).unwrap();
        assert!((got - v).abs() < 0.01, "{name}: {got} vs {v}");
    }
}

#[test]
fn values_are_clamped_to_the_slider_range() {
    let ph = Img::new(1000, 900, [150, 150, 150]).ph;
    let d = Dog { legs: 1.0, ..ONE };
    let mut pts = dog_points(d);
    // a front paw far below: legs would be 1.5 times the reference
    pts[7].1 = (SH.0, SH.1 + 1.5 * 1.243 * L);
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, pts)]);
    assert_eq!(f.shape.legs, 1.3);
}

#[test]
fn sitting_gives_the_head_and_other_gives_no_proportions() {
    let ph = Img::new(1000, 900, [150, 150, 150]).ph;
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Sitting, dog_points(Dog { legs: 0.7, tail: 0.3, ..ONE }))]);
    assert_eq!(f.set, ["head", "ears", "muzzle"]);
    assert_eq!((f.shape.legs, f.shape.tail, f.shape.body), (1.0, 1.0, 1.0));
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Other, dog_points(ONE))]);
    assert!(f.set.is_empty());
}

#[test]
fn several_photos_use_the_median() {
    let ph = Img::new(1000, 900, [150, 150, 150]).ph;
    let one = |legs| marked(&ph, Pose::Standing, dog_points(Dog { legs, ..ONE }));
    // 0.8, 1.0 and a wild 1.25: the wild one doesn't drag the answer
    let f = fit(&Design::alex(), &[one(0.8), one(1.25), one(1.0)]);
    assert!((f.shape.legs - 1.0).abs() < 0.01, "{}", f.shape.legs);
}

#[test]
fn colours_come_back_through_a_colour_cast() {
    let ph = dog_photo();
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, dog_points(ONE))]);
    for (family, want, name) in [(fam::TAN, TAN, "coat"), (fam::BLACK, BLACK, "saddle"), (fam::CREAM, CREAM, "legs"), (fam::MASK, MASK, "mask")] {
        let got = colour(&f, family).unwrap_or_else(|| panic!("no {name}"));
        let de = delta_e(got, want);
        eprintln!("dE {name}: {de:.2} ({got:06X} vs {want:06X})");
        assert!(de < 5.0, "{name}: {got:06X} vs {want:06X}, dE {de:.1}");
    }
}

#[test]
fn colours_pool_across_photos_and_replace_the_base_ones() {
    let ph = dog_photo();
    let two = [marked(&ph, Pose::Standing, dog_points(ONE)), marked(&ph, Pose::Other, dog_points(ONE))];
    let f = fit(&(breeds(Pet::Dog)[1].design)(), &two);
    // the Yellow Lab's four areas are all still there, once each, and the photo's tan replaced its coat
    assert_eq!(f.coat.len(), 4);
    assert!(delta_e(colour(&f, fam::TAN).unwrap(), TAN) < 5.0);
}

#[test]
fn a_rabbit_gets_its_head_ears_and_colours() {
    // Alex the bun's reference ratios on a 1200 x 1000 photo
    let (sh, tb) = ((500.0, 600.0), (100.0, 630.0));
    let d = |a: P, b: P| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    let l = d(sh, tb);
    let (eye, nose) = ((800.0, 500.0), (960.0, 420.0));
    let ne = d(nose, eye);
    let k = 0.443 * l / ne; // scale the head to the bun's ratio
    let nose = (eye.0 + (nose.0 - eye.0) * k, eye.1 + (nose.1 - eye.1) * k);
    let ear = (eye.0 - 0.3 * 2.45 * 0.443 * l, eye.1 - 0.954 * 2.45 * 0.443 * l);
    let chest = (560.0, 700.0);
    let mut im = Img::new(1200, 1000, [150, 150, 150]);
    im.rect(0xF3F0EB, 100.0, 560.0, 700.0, 800.0);
    im.rect(0x94693B, 250.0, 560.0, 450.0, 620.0);
    let (e_at, e_r) = ((ear.0 + (eye.0 - ear.0) * 0.35, ear.1 + (eye.1 - ear.1) * 0.35), 0.08 * 2.45 * 0.443 * l);
    im.disc(0xD6A799, e_at, e_r * 1.4);
    let ph = im.cast([0.95, 1.02, 1.03]); // a cool cast that doesn't clip the white
    let at = vec![
        (Point::Nose, nose),
        (Point::Eye, eye),
        (Point::EarTip, ear),
        (Point::Shoulders, sh),
        (Point::TailBase, tb),
        (Point::Chest, chest),
    ];
    let f = fit(&Design::bun(), &[marked(&ph, Pose::Standing, at)]);
    assert_eq!(f.set, ["head", "ears"]);
    assert!((f.shape.head - 1.0).abs() < 0.01 && (f.shape.ears - 1.0).abs() < 0.01, "{:?}", f.shape);
    assert_eq!(f.shape.feet, 1.0);
    for (family, want, name) in [(fam::WHITE, 0xF3F0EB, "white"), (fam::BROWN, 0x94693B, "brown"), (fam::PINK, 0xD6A799, "pink")] {
        let got = colour(&f, family).unwrap_or_else(|| panic!("no {name}"));
        let de = delta_e(got, want);
        eprintln!("dE bun {name}: {de:.2}");
        assert!(de < 5.0, "{name}: {got:06X} vs {want:06X}, dE {de:.1}");
    }
}

/// Every pose and every subset of skipped points (clicks scattered over a
/// small noisy photo) still gives values inside the slider ranges, with the
/// sliders it didn't set at the base's.
#[test]
fn any_skips_stay_in_range() {
    let mut ph = Img::new(120, 90, [120, 130, 140]);
    ph.rect(0x553322, 20.0, 30.0, 90.0, 60.0);
    let ph = ph.cast([1.0, 1.0, 1.0]);
    for pet in [Pet::Dog, Pet::Rabbit] {
        let all = points(pet);
        let bases: Vec<Design> = breeds(pet).iter().map(|b| (b.design)()).collect();
        for (n, base) in bases.iter().enumerate() {
            // every breed for the subsets of a small stride, all subsets for the first
            let step = if n == 0 { 1 } else { 37 };
            for mask in (0u32..1 << all.len()).step_by(step) {
                for pose in [Pose::Standing, Pose::Sitting, Pose::Other] {
                    let at: Vec<(Point, P)> = all
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| mask & (1 << i) != 0)
                        // scattered, with a repeat and a far-off click now and then
                        .map(|(i, &p)| (p, (((i * 53 + mask as usize * 7) % 130) as f32 - 5.0, ((i * 29 + mask as usize * 3) % 100) as f32 - 5.0)))
                        .collect();
                    let f = fit(base, &[marked(&ph, pose, at)]);
                    let known: Vec<&str> = sliders(pet).iter().map(|s| s.field).collect();
                    for name in ["legs", "body", "head", "ears", "muzzle", "tail", "fluff", "feet"] {
                        let (v, b) = (f.shape.get(name).unwrap(), base.shape.get(name).unwrap());
                        if f.set.contains(&name) {
                            let s = sliders(pet).iter().find(|s| s.field == name).expect("set slider is one of the species'");
                            assert!(v >= s.lo && v <= s.hi, "{pet:?} {name} = {v}");
                        } else {
                            assert_eq!(v, b, "{pet:?} {name} moved without being set");
                        }
                    }
                    assert!(f.set.iter().all(|n| known.contains(n)));
                    assert!(f.shape.fluff == base.shape.fluff && f.shape.feet == base.shape.feet);
                    for (family, _) in &f.coat {
                        assert!(pixelpet::rig::breeds::areas(pet).iter().any(|(a, _)| a == family));
                    }
                }
            }
        }
    }
}

#[test]
fn nan_and_empty_inputs_do_not_panic() {
    let ph = Img::new(40, 30, [100, 100, 100]).ph;
    let nan = vec![(Point::Nose, (f32::NAN, 1.0)), (Point::Eye, (f32::INFINITY, 2.0)), (Point::Shoulders, (5.0, 5.0)), (Point::TailBase, (5.0, 5.0))];
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, nan)]);
    assert!(f.set.is_empty());
    let empty = Photo { w: 0, h: 0, rgb: vec![] };
    let at = dog_points(ONE);
    let _ = fit(&Design::alex(), &[marked(&empty, Pose::Standing, at)]);
}

// ---------------------------------------------------------------- real photos (local only)

/// Alex's own photos give about Alex: sliders within 0.1 of 1.0 where set,
/// colours within dE 15 of one of his base shades. Reads the folder named by
/// PIXELPET_ALEX_PHOTOS (the private `alex/` photos); skips if it isn't set.
/// The taps are the prototype's, in a 1000 px thumbnail of each photo
/// (photos 0 and 2 are sitting, 4 is standing; eye and chest on 4 read off a zoomed crop).
/// TODO: decode with `photo::load::open` once it exists; until then a JPEG
/// is read here with zune-jpeg and cut down to 1000 px without EXIF turning,
/// as the prototype did.
#[test]
#[ignore]
fn alex_photos_give_about_alex() {
    let Some(dir) = std::env::var_os("PIXELPET_ALEX_PHOTOS").map(std::path::PathBuf::from).filter(|d| d.is_dir()) else {
        eprintln!("PIXELPET_ALEX_PHOTOS not set to a folder: skipped");
        return;
    };
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.to_lowercase().ends_with(".jpg") || n.to_lowercase().ends_with(".jpeg"))
        .collect();
    names.sort();
    assert!(names.len() >= 5, "expected Alex's six photos in {dir:?}");
    let photos: Vec<Photo> = [0, 2, 4].map(|i| load_jpeg(&dir.join(&names[i]))).into();
    let p = |x: f32, y: f32| (x, y);
    let sitting = |nose, ear, sh, tt, tb, fp, bp| {
        vec![(Point::Nose, nose), (Point::EarTip, ear), (Point::Shoulders, sh), (Point::TailTip, tt), (Point::TailBase, tb), (Point::FrontPaw, fp), (Point::BackPaw, bp)]
    };
    let marks = [
        marked(&photos[0], Pose::Sitting, sitting(p(635., 315.), p(432., 168.), p(335., 430.), p(68., 858.), p(118., 850.), p(432., 940.), p(270., 918.))),
        marked(&photos[1], Pose::Sitting, sitting(p(138., 315.), p(343., 178.), p(425., 445.), p(690., 845.), p(625., 815.), p(335., 925.), p(500., 905.))),
        marked(
            &photos[2],
            Pose::Standing,
            vec![
                (Point::Nose, p(505., 342.)),
                (Point::Eye, p(545., 300.)),
                (Point::EarTip, p(518., 198.)),
                (Point::Shoulders, p(440., 272.)),
                (Point::TailBase, p(205., 335.)),
                (Point::TailTip, p(145., 445.)),
                (Point::Chest, p(455., 425.)),
                (Point::FrontPaw, p(478., 572.)),
                (Point::BackPaw, p(248., 502.)),
            ],
        ),
    ];
    let f = fit(&Design::alex(), &marks);
    eprintln!("set {:?}, shape {:?}", f.set, f.shape);
    for name in &f.set {
        let v = f.shape.get(name).unwrap();
        assert!((v - 1.0).abs() <= 0.1, "{name} = {v}");
    }
    let alex = [(fam::TAN, "coat"), (fam::BLACK, "saddle"), (fam::CREAM, "legs"), (fam::MASK, "mask")];
    let mut worst = 0.0f64;
    for (family, name) in alex {
        let Some(got) = colour(&f, family) else {
            eprintln!("{name}: not found");
            continue;
        };
        let de = shades(match family {
            x if x == fam::TAN => TAN,
            x if x == fam::BLACK => BLACK,
            x if x == fam::CREAM => CREAM,
            _ => MASK,
        })
        .into_iter()
        .map(|s| delta_e(got, s))
        .fold(f64::MAX, f64::min);
        eprintln!("{name}: {got:06X}, dE to nearest shade {de:.1}");
        worst = worst.max(de);
    }
    assert!(worst <= 15.0, "worst colour is dE {worst:.1} from Alex's shades");
}

/// Decode a JPEG and box-average it down to 1000 px on the long side.
fn load_jpeg(path: &std::path::Path) -> Photo {
    use zune_jpeg::zune_core::bytestream::ZCursor;
    let bytes = std::fs::read(path).unwrap();
    let mut dec = zune_jpeg::JpegDecoder::new(ZCursor::new(&bytes));
    let rgb = dec.decode().unwrap();
    let (w, h) = dec.dimensions().unwrap();
    let s = (w.max(h) as f32 / 1000.0).max(1.0);
    let (nw, nh) = ((w as f32 / s) as usize, (h as f32 / s) as usize);
    let mut out = Vec::with_capacity(nw * nh * 3);
    for y in 0..nh {
        for x in 0..nw {
            let (x0, x1) = ((x as f32 * s) as usize, (((x + 1) as f32 * s) as usize).clamp(1, w).max((x as f32 * s) as usize + 1));
            let (y0, y1) = ((y as f32 * s) as usize, (((y + 1) as f32 * s) as usize).clamp(1, h).max((y as f32 * s) as usize + 1));
            let mut sum = [0u32; 3];
            for yy in y0..y1.min(h) {
                for xx in x0..x1.min(w) {
                    for c in 0..3 {
                        sum[c] += rgb[(yy * w + xx) * 3 + c] as u32;
                    }
                }
            }
            let n = ((y1.min(h) - y0) * (x1.min(w) - x0)).max(1) as u32;
            out.extend(sum.map(|v| (v / n) as u8));
        }
    }
    Photo { w: nw as u32, h: nh as u32, rgb: out }
}
