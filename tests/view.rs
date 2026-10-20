//! The view: frame cache, anim blends, scaling, mirroring and z's.

use pixelpet::brain::Show;
use pixelpet::rig::{self, Anim, Frame, Pet};
use pixelpet::view::{compose, View};

fn px(img: &pixelpet::window::Rgba, x: usize, y: usize) -> [u8; 4] {
    let i = (y * img.w + x) * 4;
    img.px[i..i + 4].try_into().unwrap()
}

fn show(anim: Anim, anim_start: f64) -> Show {
    Show { anim, anim_start, x: 500.0, lift: 0.0, facing: 1, visible: true, opacity: 100, zzz: false, hover_for: 0.0 }
}

#[test]
fn compose_scales_and_mirrors() {
    let mut f = Frame { w: 3, h: 2, ox: 1, gy: 1, rgba: vec![0; 3 * 2 * 4] };
    f.rgba[0..4].copy_from_slice(&[200, 10, 20, 255]);
    let margin = 6;
    let a = compose(&f, 22.0, 3, 1, &[], margin);
    assert_eq!((a.w, a.h), (9, 6 + margin));
    for y in 0..3 {
        for x in 0..3 {
            assert_eq!(px(&a, x, margin + y), [200, 10, 20, 255]);
            assert_eq!(px(&a, 8 - x, margin + y)[3], 0);
        }
    }
    assert_eq!(px(&a, 0, 0)[3], 0, "margin is transparent");
    let b = compose(&f, 22.0, 3, -1, &[], margin);
    for y in 0..3 {
        for x in 0..3 {
            assert_eq!(px(&b, 8 - x, margin + y), [200, 10, 20, 255]);
            assert_eq!(px(&b, x, margin + y)[3], 0);
        }
    }
}

#[test]
fn compose_draws_z() {
    // classic frame geometry, empty
    let f = Frame { w: 69, h: 60, ox: 35, gy: 58, rgba: vec![0; 69 * 60 * 4] };
    let (zoom, margin) = (3, 66);
    let img = compose(&f, 22.0, zoom, 1, &[(0.0, 0.0)], margin);
    // z anchor (0.45, 0.75) units from the ground point: col 35 + round(9.9) = 45,
    // row 58 - round(16.5) = 41, so the glyph's top-left white pixel is at
    // image (45 * 3, 66 + 41 * 3)
    assert_eq!(px(&img, 135, 189), [255, 255, 255, 255]);
    assert_eq!(px(&img, 137, 191), [255, 255, 255, 255]);
    assert_eq!(px(&img, 134, 189), [0x24, 0x12, 0x0A, 255], "outline left of the glyph");
    assert_eq!(px(&img, 135, 186), [0x24, 0x12, 0x0A, 255], "outline above the glyph");
    // row 1 of the glyph is `...#.`: the first cells are not white
    assert_ne!(px(&img, 135, 192), [255, 255, 255, 255]);
    // a z late in its life fades
    let late = compose(&f, 22.0, zoom, 1, &[(2.2, 0.0)], margin);
    let max_a = late.px.chunks(4).map(|p| p[3]).max().unwrap();
    assert!(max_a > 0 && max_a < 255, "alpha {max_a}");
}

#[test]
fn cache_reuses_frames() {
    let mut v = View::new(Pet::Dog, rig::look("classic"));
    let s = show(Anim::Stand, 0.0);
    v.frame(&s, 1.00, 3, false);
    let n = v.renders();
    v.frame(&s, 1.01, 3, false); // same 1/12 s step
    assert_eq!(v.renders(), n);
    v.frame(&s, 1.10, 3, false); // next step
    assert_eq!(v.renders(), n + 1);
}

#[test]
fn blend_takes_three_steps() {
    let look = rig::look("classic");
    let mut v = View::new(Pet::Dog, look);
    v.frame(&show(Anim::Walk, -2.0), -0.05, 3, false);
    let s = show(Anim::Sleep, 0.0);
    let frames: Vec<Vec<u8>> = [0.0, 0.09, 0.17, 0.25].iter().map(|&t| v.frame(&s, t, 3, false).0.px.clone()).collect();
    assert_ne!(frames[0], frames[1]);
    assert_ne!(frames[1], frames[2]);
    assert_ne!(frames[2], frames[3]);
    let mut fresh = View::new(Pet::Dog, look);
    assert_eq!(frames[3], fresh.frame(&s, 0.25, 3, false).0.px, "from 0.25 it is the target anim's own frame");
    assert_eq!(v.frame(&s, 0.40, 3, false).0.px, fresh.frame(&s, 0.40, 3, false).0.px);
}

#[test]
fn hover_hit_is_sticky_inside_the_image() {
    use pixelpet::view::over_image;
    // a 4x4 image at (100, 50) with one opaque pixel at (1, 1)
    let mut img = pixelpet::window::Rgba::new(4, 4);
    img.px[(4 + 1) * 4 + 3] = 255;
    let at = (100, 50);
    assert!(over_image(&img, at, (101, 51), false), "opaque pixel starts a hover");
    assert!(!over_image(&img, at, (103, 53), false), "a transparent pixel does not start one");
    // once hovering, the pose changes under the cursor; the hover holds
    // anywhere inside the image so it cannot flicker
    assert!(over_image(&img, at, (103, 53), true));
    assert!(!over_image(&img, at, (104, 53), true), "outside the image ends it");
    assert!(!over_image(&img, at, (99, 51), true));
}

#[test]
fn offset_puts_feet_on_the_ground() {
    let look = rig::look("classic");
    let mut v = View::new(Pet::Dog, look);
    let (img, dx, dy) = v.frame(&show(Anim::Stand, 0.0), 0.0, 3, false);
    let (w, h) = (img.w as i32, img.h as i32);
    // the ground point sits at the frame's ox column boundary, below row gy
    assert_eq!(dx, -35 * 3);
    assert_eq!(dy, -(66 + 59 * 3));
    assert_eq!(h, 66 + 60 * 3);
    // the lowest opaque row is the last row above the ground line
    let img = img.clone();
    let lowest = (0..h).rev().find(|&y| (0..w).any(|x| px(&img, x as usize, y as usize)[3] > 0)).unwrap();
    assert_eq!(lowest + dy, -1, "feet end one pixel above the ground line");
}
