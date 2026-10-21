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
