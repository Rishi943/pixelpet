//! The scene: the pet and its props composed into the one image the pet
//! window shows, depth scaling, and what the cursor is over.

use pixelpet::rig::{self, Anim, Pet};
use pixelpet::scene::{self, Hit, Part, Scene, Sprite};
use pixelpet::window::Rgba;

fn solid(w: usize, h: usize, c: [u8; 4]) -> Rgba {
    let mut img = Rgba::new(w, h);
    for p in img.px.chunks_mut(4) {
        p.copy_from_slice(&c);
    }
    img
}

fn alpha(img: &Rgba, x: usize, y: usize) -> u8 {
    img.px[(y * img.w + x) * 4 + 3]
}

#[test]
fn depth_scale_halves_at_one() {
    assert_eq!(rig::depth_scale(0.0), 1.0);
    assert_eq!(rig::depth_scale(1.0), 0.5);
    assert_eq!(rig::depth_scale(-1.0), 1.0);
}

#[test]
fn project_meets_the_vanishing_point() {
    let vp = (1000.0, 600.0);
    assert_eq!(scene::project(400.0, 0.0, vp, 1200.0), (400.0, 1200.0, 1.0));
    assert_eq!(scene::project(400.0, 1.0, vp, 1200.0), (700.0, 900.0, 0.5));
    let (x, y, _) = scene::project(400.0, 1000.0, vp, 1200.0);
    assert!((x - 1000.0).abs() < 1.0 && (y - 600.0).abs() < 1.0, "({x},{y})");
}

#[test]
fn scaled_look_redraws_not_resizes() {
    let classic = rig::look("classic");
    assert!(std::ptr::eq(rig::look_scaled(classic, 1.0), classic), "depth 0 is the look itself");
    let half = rig::look_scaled(classic, 0.5);
    assert_eq!(half.r, 11.0);
    let height = |img: &Rgba| {
        let rows: Vec<usize> = (0..img.h).filter(|&y| (0..img.w).any(|x| alpha(img, x, y) > 0)).collect();
        rows.last().unwrap() - rows[0] + 1
    };
    let full = pixelpet::view::compose(&rig::render(&rig::pose(Pet::Dog, Anim::Stand, classic, 0.0, None), classic, false), classic.r, 3, 1, &[], 0);
    let small = pixelpet::view::compose(&rig::render(&rig::pose(Pet::Dog, Anim::Stand, half, 0.0, None), half, false), half.r, 3, 1, &[], 0);
    let k = height(&small) as f64 / height(&full) as f64;
    assert!((0.4..=0.6).contains(&k), "small is {k} of full height");
    // still whole art pixels: every pixel matches the top-left of its 3x3 block
    for y in 0..small.h {
        for x in 0..small.w {
            let (bx, by) = (x / 3 * 3, y / 3 * 3);
            assert_eq!(small.px[(y * small.w + x) * 4..][..4], small.px[(by * small.w + bx) * 4..][..4], "({x},{y})");
        }
    }
}

#[test]
fn compose_covers_all_sprites() {
    let s = Scene {
        sprites: vec![
            Sprite { img: solid(10, 10, [255, 0, 0, 255]), x: 100, y: 100, part: Part::Pet },
            Sprite { img: solid(10, 10, [0, 0, 255, 255]), x: 300, y: 250, part: Part::Toy },
        ],
        rope: None,
    };
    let (img, x, y) = s.compose();
    assert_eq!((img.w, img.h, x, y), (210, 160, 100, 100));
    assert_eq!(&img.px[0..4], &[255, 0, 0, 255]);
    let i = ((150 + 5) * img.w + 200 + 5) * 4;
    assert_eq!(&img.px[i..i + 4], &[0, 0, 255, 255]);
    assert_eq!(alpha(&img, 100, 50), 0, "between sprites is clear");
}

#[test]
fn rope_far_away_stays_click_through() {
    let s = Scene {
        sprites: vec![Sprite { img: solid(60, 50, [200, 120, 40, 255]), x: 100, y: 960, part: Part::Pet }],
        rope: Some(((150, 975), (1800, 200), 3, 30.0)),
    };
    let (img, x0, y0) = s.compose();
    let (px, py) = (1800 - x0, 1000 - y0);
    assert!((px as usize) < img.w && (py as usize) < img.h, "the point is inside the image");
    assert_eq!(alpha(&img, px as usize, py as usize), 0);
    assert_eq!(s.hit(1800, 1000, false), Hit::None);
    assert!(img.px.chunks(4).filter(|p| p[3] > 0).count() > 60 * 50 + 100, "the rope is drawn");
}

#[test]
fn hit_prefers_the_top_sprite() {
    let s = Scene {
        sprites: vec![
            Sprite { img: solid(40, 40, [200, 120, 40, 255]), x: 0, y: 0, part: Part::Pet },
            Sprite { img: solid(10, 10, [180, 30, 40, 255]), x: 30, y: 30, part: Part::Toy },
            Sprite { img: solid(10, 10, [255, 255, 0, 255]), x: 0, y: 0, part: Part::Food },
            Sprite { img: solid(5, 5, [255, 0, 0, 255]), x: 100, y: 100, part: Part::Effect },
        ],
        rope: None,
    };
    assert_eq!(s.hit(35, 35, false), Hit::Toy);
    assert_eq!(s.hit(15, 15, false), Hit::Pet);
    assert_eq!(s.hit(5, 5, false), Hit::Pet, "food on top never takes the hit");
    assert_eq!(s.hit(102, 102, false), Hit::None, "effects never hit");
    assert_eq!(s.hit(60, 5, false), Hit::None);
}
