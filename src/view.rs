//! The view: turns the brain's `Show` into the window image. Steps anim time
//! at 12 fps, blends anim changes in 3 steps, caches rendered frames, scales
//! by whole-number zoom, mirrors, and draws the sleeping z's.

use crate::brain::Show;
use crate::rig::{self, core::js_round, Anim, Frame, Look, Pet, Skel};
use crate::window::Rgba;

pub const FPS: f64 = 12.0;
const BLEND_SECS: f64 = 0.25;
const BLEND_STEPS: f64 = 3.0;

/// The z glyph. 5x5: a 4x4 Z with its outline reads as an "I" on screen.
const Z_GLYPH: [&str; 5] = ["#####", "...#.", "..#..", ".#...", "#####"];
const Z_N: i32 = Z_GLYPH.len() as i32;
const Z_OUTLINE: [u8; 3] = [0x24, 0x12, 0x0A];
/// Where a new z appears, in rig units from the ground point (forward, up).
const Z_ANCHOR: (f64, f64) = (0.45, 0.75);
const Z_EVERY: f64 = 1.3;
const Z_LIFE: f64 = 2.6;
const Z_RISE: f64 = 0.7;
const Z_DRIFT: f64 = 0.25;

/// Blend a straight-alpha colour over `dst`.
fn over(dst: &mut [u8], c: [u8; 3], a: u8) {
    if a == 255 {
        dst.copy_from_slice(&[c[0], c[1], c[2], 255]);
        return;
    }
    let sa = a as f64 / 255.0;
    let da = dst[3] as f64 / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.0 {
        return;
    }
    for k in 0..3 {
        dst[k] = ((c[k] as f64 * sa + dst[k] as f64 * da * (1.0 - sa)) / oa).round() as u8;
    }
    dst[3] = (oa * 255.0).round() as u8;
}

/// Scale `frame` by `zoom` (nearest neighbour), mirror it when `facing < 0`,
/// add `margin_top` transparent image rows above it, and draw z's. Each z is
/// `(age_seconds, x_offset_units)`; `r` is the look's px per rig unit.
pub fn compose(frame: &Frame, r: f64, zoom: u32, facing: i8, zzz: &[(f64, f64)], margin_top: usize) -> Rgba {
    let z = zoom as usize;
    let (w, h) = (frame.w * z, frame.h * z + margin_top);
    let mut img = Rgba::new(w, h);
    let flip = facing < 0;
    for fy in 0..frame.h {
        for fx in 0..frame.w {
            let si = (fy * frame.w + fx) * 4;
            if frame.rgba[si + 3] == 0 {
                continue;
            }
            let src = &frame.rgba[si..si + 4];
            let cx = if flip { frame.w - 1 - fx } else { fx };
            for dy in 0..z {
                let row = (margin_top + fy * z + dy) * w;
                for dx in 0..z {
                    let di = (row + cx * z + dx) * 4;
                    img.px[di..di + 4].copy_from_slice(src);
                }
            }
        }
    }
    // z's: art-pixel cells above the frame's ground point, in image space
    let margin_cells = margin_top as f64 / z as f64;
    for &(age, xoff) in zzz {
        if !(0.0..Z_LIFE).contains(&age) {
            continue;
        }
        let k = age / Z_LIFE;
        let zx = js_round((Z_ANCHOR.0 + xoff + Z_DRIFT * k) * r) as i32;
        let zy = js_round((Z_ANCHOR.1 + Z_RISE * k) * r) as i32;
        let col = if flip { (frame.w as i32 - frame.ox) - zx - Z_N } else { frame.ox + zx };
        let row = frame.gy - zy + margin_cells as i32;
        let alpha = if age < Z_LIFE / 2.0 { 255.0 } else { 255.0 * (Z_LIFE - age) / (Z_LIFE / 2.0) };
        let a = alpha.round().clamp(0.0, 255.0) as u8;
        let on = |gx: i32, gy: i32| (0..Z_N).contains(&gx) && (0..Z_N).contains(&gy) && Z_GLYPH[gy as usize].as_bytes()[gx as usize] == b'#';
        for gy in -1..=Z_N {
            for gx in -1..=Z_N {
                let colour = if on(gx, gy) {
                    [255, 255, 255]
                } else if (-1..=1).any(|oy| (-1..=1).any(|ox| (ox == 0) != (oy == 0) && on(gx + ox, gy + oy))) {
                    Z_OUTLINE
                } else {
                    continue;
                };
                let (cx, cy) = (col + gx, row + gy);
                if cx < 0 || cy < 0 || cx as usize * z >= w || cy as usize * z >= h {
                    continue;
                }
                for dy in 0..z {
                    for dx in 0..z {
                        let di = ((cy as usize * z + dy) * w + cx as usize * z + dx) * 4;
                        over(&mut img.px[di..di + 4], colour, a);
                    }
                }
            }
        }
    }
    img
}

/// Is the cursor on the pet? `img` sits with its top-left at `at`. A hover
/// starts only on an opaque pixel; once `sticky` (already hovering) it holds
/// anywhere inside the image, because the look-at-you pose has a different
/// outline and would otherwise end the hover and start it again each frame.
pub fn over_image(img: &Rgba, at: (i32, i32), cursor: (i32, i32), sticky: bool) -> bool {
    let (ix, iy) = (cursor.0 - at.0, cursor.1 - at.1);
    if ix < 0 || iy < 0 || ix as usize >= img.w || iy as usize >= img.h {
        return false;
    }
    sticky || img.px[(iy as usize * img.w + ix as usize) * 4 + 3] > 0
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct FrameKey {
    anim: Anim,
    tick: i64,
    blend_step: u8, // 3 = no blend
    blend_id: u64,
    collar: bool,
    /// the look's `r` at this depth
    r: u32,
    eyes_closed: bool,
    /// a toy in the mouth (fetch)
    toy: Option<rig::Toy>,
}

#[derive(Clone, PartialEq, Debug)]
struct ImageKey {
    frame: FrameKey,
    zoom: u32,
    facing: i8,
    zzz: Vec<(i64, i64)>,
}

struct Blend {
    start: f64,
    from: Skel,
    id: u64,
}

pub struct View {
    design: rig::Design,
    /// the design's palette, compiled once (None: a built-in's own)
    palette: Option<rig::core::Palette>,
    look: &'static Look,
    frame_key: Option<FrameKey>,
    frame: Option<Frame>,
    image_key: Option<ImageKey>,
    image: Rgba,
    last_anim: Option<Anim>,
    last_skel: Option<Skel>,
    blend: Option<Blend>,
    blends: u64,
    renders: usize,
    zzz_since: Option<f64>,
    image_gen: u64,
}

impl View {
    pub fn new(pet: Pet, look: &'static Look) -> View {
        View::with_design(rig::Design::builtin(pet), look)
    }

    /// A view of a made pet.
    pub fn with_design(design: rig::Design, look: &'static Look) -> View {
        let plain = design.coat.is_empty() && design.collar.is_none();
        View {
            palette: (!plain).then(|| design.palette()),
            design,
            look,
            frame_key: None,
            frame: None,
            image_key: None,
            image: Rgba::new(1, 1),
            last_anim: None,
            last_skel: None,
            blend: None,
            blends: 0,
            renders: 0,
            zzz_since: None,
            image_gen: 0,
        }
    }

    /// The image last returned by `frame`.
    pub fn image(&self) -> &Rgba {
        &self.image
    }

    /// How many times the rig has rendered (a test hook for the cache).
    pub fn renders(&self) -> usize {
        self.renders
    }

    /// The image to show, plus the offset from the pet's ground point
    /// (`x`, ground y) to the image's top-left, in physical px. At `depth`
    /// the pet is redrawn smaller (`rig::look_scaled`), still in whole art
    /// pixels of `zoom`.
    pub fn frame(&mut self, show: &Show, now: f64, zoom: u32, collar: bool, depth: f64) -> (&Rgba, i32, i32) {
        let look = rig::look_scaled(self.look, rig::depth_scale(depth));
        // an anim change blends from whatever was last on screen
        if self.last_anim.is_some_and(|a| a != show.anim) {
            if let Some(from) = self.last_skel {
                self.blends += 1;
                self.blend = Some(Blend { start: now, from, id: self.blends });
            }
        }
        self.last_anim = Some(show.anim);
        let step = match &self.blend {
            Some(b) => {
                let k = ((now - b.start) / BLEND_SECS * BLEND_STEPS).floor();
                if k >= BLEND_STEPS {
                    self.blend = None;
                    3
                } else {
                    k.max(0.0) as u8
                }
            }
            None => 3,
        };
        let tick = ((now - show.anim_start) * FPS).floor() as i64;
        let key = FrameKey {
            anim: show.anim,
            tick,
            blend_step: step,
            blend_id: self.blend.as_ref().map_or(0, |b| b.id),
            collar,
            r: look.r as u32,
            eyes_closed: show.eyes_closed,
            toy: show.props.toy.filter(|t| t.in_mouth).map(|t| t.toy),
        };
        if self.frame_key != Some(key) {
            let t = tick as f64 / FPS;
            let mut s = rig::pose_for(&self.design, show.anim, look, t, key.toy);
            if let (Some(b), true) = (&self.blend, step < 3) {
                s = rig::lerp(&b.from, &s, step as f64 / BLEND_STEPS);
            }
            if show.eyes_closed {
                s = rig::with_eyes_closed(s);
            }
            self.frame = Some(match &self.palette {
                Some(pal) => rig::render_with(&self.design, &s, look, pal),
                None if self.design.shape == rig::Shape::default() => rig::render(&s, look, collar),
                None => rig::render_for(&self.design, &s, look),
            });
            self.frame_key = Some(key);
            self.last_skel = Some(s);
            self.renders += 1;
        }
        // z's, stepped at 12 fps
        let zzz: Vec<(f64, f64)> = if show.zzz {
            let since = *self.zzz_since.get_or_insert(now);
            let mut v = vec![];
            let mut n = 0u32;
            loop {
                let born = since + n as f64 * Z_EVERY;
                if born > now {
                    break;
                }
                let age = ((now - born) * FPS).floor() / FPS;
                if age < Z_LIFE {
                    v.push((age, if n % 2 == 1 { 0.12 } else { 0.0 }));
                }
                n += 1;
            }
            v
        } else {
            self.zzz_since = None;
            vec![]
        };
        let margin = js_round(look.r) as usize * zoom as usize;
        let ikey = ImageKey {
            frame: key,
            zoom,
            facing: show.facing,
            zzz: zzz.iter().map(|(a, x)| ((a * FPS).round() as i64, (x * 100.0).round() as i64)).collect(),
        };
        let frame = self.frame.as_ref().unwrap();
        if self.image_key.as_ref() != Some(&ikey) {
            self.image = compose(frame, look.r, zoom, show.facing, &zzz, margin);
            self.image_key = Some(ikey);
            self.image_gen += 1;
        }
        let z = zoom as i32;
        let ox = if show.facing < 0 { frame.w as i32 - frame.ox } else { frame.ox };
        let dx = -ox * z;
        let dy = -((frame.gy + 1) * z + margin as i32);
        (&self.image, dx, dy)
    }

    /// Goes up by one each time the composed image is rebuilt, so the app
    /// presents only when something changed.
    pub fn image_generation(&self) -> u64 {
        self.image_gen
    }
}
