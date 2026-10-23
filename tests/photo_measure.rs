//! Points clicked on photos set the maker's sliders and colours. Photos here
//! are drawn in the test (flat colours on a grey wall, with a colour cast);
//! the one test on real photos is local-only.

use pixelpet::photo::measure::{fit, points, Marked, Point, Pose};
use pixelpet::photo::Photo;
use pixelpet::rig::breeds::{breeds, sliders};
use pixelpet::rig::core::fam;
use pixelpet::rig::design::Design;
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

    /// A little fixed noise over everything, as in a real photo.
    fn noisy(mut self) -> Photo {
        let mut seed = 7u32;
        for v in self.ph.rgb.iter_mut() {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = ((seed >> 24) % 5) as f64 - 2.0;
            *v = (*v as f64 + noise).round().clamp(0.0, 255.0) as u8;
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
    let span = d.head * 0.4 * L;
    let ear = (nose.0 - span * 0.8, nose.1 - span * 0.6);
    let eye = eye_from(nose, ear, d.muzzle * 0.16 * L, d.ears * 0.29 * L);
    vec![
        (Point::Nose, nose),
        (Point::Eye, eye),
        (Point::EarTip, ear),
        (Point::Shoulders, SH),
        (Point::TailBase, TB),
        (Point::TailTip, (TB.0, TB.1 + d.tail * 0.45 * L)),
        (Point::FrontPaw, (SH.0, SH.1 + d.legs * 0.9 * L)),
        // the colour clicks, on the areas `dog_on` paints
        (Point::CoatColour, (500.0, 400.0)),
        (Point::MarkColour, (500.0, 280.0)),
        (Point::LegColour, (SH.0 + 40.0, 400.0)),
        (Point::FaceColour, muzzle(nose, eye)),
    ]
}

/// Where `dog_on` paints the mask: between the nose and the eye.
fn muzzle(nose: P, eye: P) -> P {
    (nose.0 + (eye.0 - nose.0) * 0.4, nose.1 + (eye.1 - nose.1) * 0.4)
}

const TAN: u32 = 0xC47637;
const BLACK: u32 = 0x231B1B;
const CREAM: u32 = 0xE2B077;
const MASK: u32 = 0x2B2220;

/// The dog painted to match `dog_points(ONE)`, on a plain wall.
fn dog_photo() -> Photo {
    dog_on([150, 150, 150])
}

fn dog_on(wall: [u8; 3]) -> Photo {
    let mut im = Img::new(1000, 900, wall);
    im.rect(TAN, 300.0, 250.0, 700.0, 450.0); // body
    im.rect(TAN, SH.0 - 18.0, 250.0, SH.0 + 18.0, 800.0); // front leg
    im.rect(BLACK, 330.0, 250.0, 670.0, 310.0); // saddle
    im.rect(CREAM, SH.0 + 40.0 - 35.0, 365.0, SH.0 + 40.0 + 35.0, 435.0); // chest
    let pts = dog_points(ONE);
    let (nose, eye) = (pts[0].1, pts[1].1);
    im.disc(MASK, muzzle(nose, eye), 30.0);
    im.noisy()
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
    assert_eq!(f.set, ["legs", "ears", "muzzle", "tail"]);
    assert_eq!(f.shape.body, 1.0);
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
    let want = [("legs", 1.1), ("head", 1.0), ("ears", 1.15), ("muzzle", 1.1), ("tail", 0.8), ("body", 1.0)];
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
    pts.iter_mut().find(|p| p.0 == Point::FrontPaw).unwrap().1 = (SH.0, SH.1 + 1.5 * 0.9 * L);
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, pts)]);
    assert_eq!(f.shape.legs, 1.3);
}

#[test]
fn sitting_gives_the_face_and_other_gives_no_proportions() {
    let ph = Img::new(1000, 900, [150, 150, 150]).ph;
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Sitting, dog_points(Dog { legs: 0.7, tail: 0.3, ..ONE }))]);
    assert_eq!(f.set, ["ears", "muzzle"]);
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
fn colours_come_back_as_they_are_on_any_background() {
    // a grey wall, and green leaves (which a white balance once read as a
    // cast and turned a beagle blue)
    for wall in [[150, 150, 150], [40, 140, 40]] {
        let ph = dog_on(wall);
        let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, dog_points(ONE))]);
        for (family, want, name) in [(fam::TAN, TAN, "coat"), (fam::BLACK, BLACK, "saddle"), (fam::CREAM, CREAM, "legs"), (fam::MASK, MASK, "mask")] {
            let got = colour(&f, family).unwrap_or_else(|| panic!("no {name}"));
            let de = delta_e(got, want);
            eprintln!("dE {name}: {de:.2} ({got:06X} vs {want:06X})");
            assert!(de < 5.0, "{name} on {wall:?}: {got:06X} vs {want:06X}, dE {de:.1}");
        }
    }
}

#[test]
fn a_skipped_saddle_gives_a_solid_coat() {
    let ph = dog_photo();
    let at: Vec<(Point, P)> = dog_points(ONE).into_iter().filter(|p| p.0 != Point::MarkColour).collect();
    let f = fit(&Design::alex(), &[marked(&ph, Pose::Standing, at)]);
    // the coat, a shade darker
    let (coat, saddle) = (colour(&f, fam::TAN).unwrap(), colour(&f, fam::BLACK).unwrap());
    assert!(delta_e(saddle, coat) < 7.0 && lab(saddle)[0] < lab(coat)[0], "{saddle:06X} vs {coat:06X}");
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
    let mut im = Img::new(1200, 1000, [150, 150, 150]);
    im.rect(0xF3F0EB, 100.0, 560.0, 700.0, 800.0);
    im.rect(0x94693B, 250.0, 560.0, 450.0, 620.0);
    let (e_at, e_r) = ((ear.0 + (eye.0 - ear.0) * 0.35, ear.1 + (eye.1 - ear.1) * 0.35), 0.08 * 2.45 * 0.443 * l);
    im.disc(0xD6A799, e_at, e_r * 1.4);
    let ph = im.noisy();
    let at = vec![
        (Point::Nose, nose),
        (Point::Eye, eye),
        (Point::EarTip, ear),
        (Point::Shoulders, sh),
        (Point::TailBase, tb),
        (Point::CoatColour, (600.0, 750.0)),
        (Point::MarkColour, (350.0, 590.0)),
        (Point::EarColour, e_at),
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
    let ph = ph.noisy();
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
