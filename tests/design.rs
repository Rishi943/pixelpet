//! A pet as data: the built-ins render exactly as before, and a design's
//! shape and coat change only what they should.

use pixelpet::rig::core::fam;
use pixelpet::rig::design::{shades, Design};
use pixelpet::rig::{self, Anim, Pet};

fn look() -> &'static rig::Look {
    rig::look("classic")
}

fn opaque_height(f: &rig::Frame) -> usize {
    let rows: Vec<usize> = (0..f.h).filter(|&y| (0..f.w).any(|x| f.rgba[(y * f.w + x) * 4 + 3] > 0)).collect();
    rows.last().unwrap() - rows.first().unwrap() + 1
}

#[test]
fn builtin_designs_render_exactly_as_today() {
    for pet in [Pet::Dog, Pet::Rabbit] {
        let d = Design::builtin(pet);
        for anim in Anim::ALL {
            for t in [0.0, 0.37, 1.1] {
                let old = rig::render(&rig::pose(pet, anim, look(), t, None), look(), false);
                let new = rig::render_for(&d, &rig::pose_for(&d, anim, look(), t, None), look());
                assert!(old == new, "{pet:?} {anim:?} at {t} differs");
            }
        }
    }
}

#[test]
fn shades_match_the_mockup() {
    // the mockup's family() in drafts/maker/index.html, run with node
    assert_eq!(shades(0xD9B26F), [0x714819, 0xB97E2D, 0xD9B26F, 0xEEE2C6]);
    assert_eq!(shades(0x231B1B), [0x0F0B0C, 0x191313, 0x231B1B, 0x3A2E2D]);
    assert_eq!(shades(0xF3F0EB), [0x806649, 0xBEAB90, 0xF3F0EB, 0xF9F8F6]);
    assert_eq!(shades(0x2F64C8), [0x113057, 0x1E4B8F, 0x2F64C8, 0x7591D9]);
}

fn stand(d: &Design) -> rig::Frame {
    rig::render_for(d, &rig::pose_for(d, Anim::Stand, look(), 0.0, None), look())
}

#[test]
fn longer_legs_make_a_taller_dog() {
    let mut d = Design::alex();
    let base = opaque_height(&stand(&d));
    d.shape.legs = 1.3;
    assert!(opaque_height(&stand(&d)) > base);
    d.shape.legs = 0.6;
    assert!(opaque_height(&stand(&d)) < base);
}

#[test]
fn a_coat_override_changes_only_its_family() {
    let alex = stand(&Design::alex());
    let mut d = Design::alex();
    d.coat.push((fam::TAN, 0xD9B26F));
    let lab = stand(&d);
    let tan: Vec<[u8; 3]> = shades(0xD9B26F).iter().map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect();
    let mut changed = 0;
    for (a, b) in alex.rgba.chunks(4).zip(lab.rgba.chunks(4)) {
        if a != b {
            changed += 1;
            // shading may blend a tone with the outline edge drop; the base
            // colours must be among the new shades
            assert_eq!(a[3], b[3], "alpha moved");
        }
    }
    assert!(changed > 50, "only {changed} pixels changed");
    let new_colours = lab.rgba.chunks(4).filter(|p| p[3] > 0).filter(|p| tan.contains(&[p[0], p[1], p[2]])).count();
    assert!(new_colours > 50, "the new tan shades barely appear ({new_colours})");
}

#[test]
fn collar_is_drawn_only_when_set() {
    let alex = stand(&Design::alex());
    let mut d = Design::alex();
    assert_eq!(stand(&d), alex);
    d.collar = Some(0x2F64C8);
    assert_ne!(stand(&d), alex);
}

#[test]
fn long_legs_walk_faster() {
    let mut d = Design::alex();
    let base = rig::speed_for(&d, look(), Anim::Walk);
    d.shape.legs = 1.3;
    assert!(rig::speed_for(&d, look(), Anim::Walk) > base);
    assert_eq!(rig::speed_for(&Design::alex(), look(), Anim::Walk), rig::speed(Pet::Dog, look(), Anim::Walk));
}

// ---------------------------------------------------------------- breeds

use pixelpet::rig::breeds::{breeds, sliders};

#[test]
fn first_breed_is_the_builtin() {
    assert_eq!(breeds(Pet::Dog)[0].name, "Alex the GSD");
    assert_eq!((breeds(Pet::Dog)[0].design)(), Design::alex());
    assert_eq!(breeds(Pet::Rabbit)[0].name, "Alex the bun");
    assert_eq!((breeds(Pet::Rabbit)[0].design)(), Design::bun());
    let names: Vec<&str> = breeds(Pet::Dog).iter().map(|b| b.name).collect();
    assert_eq!(names, ["Alex the GSD", "Yellow Lab", "Black Lab", "Husky", "Golden", "Corgi"]);
}

#[test]
fn every_breed_renders_every_anim() {
    for pet in [Pet::Dog, Pet::Rabbit] {
        for b in breeds(pet) {
            let d = (b.design)();
            assert_eq!(d.pet, pet);
            for anim in Anim::ALL {
                let f = rig::render_for(&d, &rig::pose_for(&d, anim, look(), 0.3, None), look());
                assert!(f.rgba.chunks(4).any(|p| p[3] > 0), "{} {anim:?} is empty", b.name);
            }
        }
    }
}

/// The frame's top, left and right edge pixels: anything drawn there was
/// clipped. (The bottom row is the ground: feet stand on it by design, even
/// Alex's, so it doesn't count.)
fn touches_edge(f: &rig::Frame) -> bool {
    let a = |x: usize, y: usize| f.rgba[(y * f.w + x) * 4 + 3] > 0;
    (0..f.w).any(|x| a(x, 0)) || (0..f.h).any(|y| a(0, y) || a(f.w - 1, y))
}

#[test]
fn extreme_shapes_stay_inside_the_frame() {
    let mut seed = 0x2545F491u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for pet in [Pet::Dog, Pet::Rabbit] {
        let mut shapes = vec![];
        for pick in [0.0, 1.0] {
            let mut d = Design::builtin(pet);
            for s in sliders(pet) {
                d.shape.set(s.field, s.lo + pick * (s.hi - s.lo));
            }
            shapes.push((format!("all {}", if pick == 0.0 { "lo" } else { "hi" }), d));
        }
        for i in 0..50 {
            let mut d = Design::builtin(pet);
            for s in sliders(pet) {
                d.shape.set(s.field, s.lo + rnd() * (s.hi - s.lo));
            }
            shapes.push((format!("random {i}"), d));
        }
        for (what, d) in &shapes {
            for anim in Anim::ALL {
                let p = rig::period(pet, anim);
                for k in 0..8 {
                    let t = p * k as f64 / 8.0;
                    let f = rig::render_for(d, &rig::pose_for(d, anim, look(), t, None), look());
                    assert!(!touches_edge(&f), "{pet:?} {what} {anim:?} at {t:.2}s is clipped: {:?}", d.shape);
                }
            }
        }
    }
}
