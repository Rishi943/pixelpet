//! The scene: the pet and its props (toy, food, hearts, crumbs)
//! composed into the one image the pet window shows, plus what the cursor is
//! over. Screen coordinates, physical px. Knows nothing about the OS.

use crate::rig;
use crate::window::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Pet,
    Toy,
    Food,
    Effect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    None,
    Pet,
    Toy,
}

/// An image with its top-left at screen `(x, y)`.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub img: Rgba,
    pub x: i32,
    pub y: i32,
    pub part: Part,
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    /// Drawn in order: later sprites cover earlier ones.
    pub sprites: Vec<Sprite>,
}

/// Where a ground point `x` at `depth` lands on screen, and its scale:
/// everything shrinks towards the vanishing point `vp` by `1 / (1 + depth)`.
pub fn project(x: f64, depth: f64, vp: (f64, f64), ground: f64) -> (f64, f64, f64) {
    let s = rig::depth_scale(depth);
    (vp.0 + (x - vp.0) * s, vp.1 + (ground - vp.1) * s, s)
}

fn over(dst: &mut [u8], src: &[u8]) {
    let a = src[3] as u32;
    if a == 255 {
        dst.copy_from_slice(src);
    } else if a > 0 {
        let da = dst[3] as u32;
        let out_a = a + da * (255 - a) / 255;
        for k in 0..3 {
            let c = (src[k] as u32 * a + dst[k] as u32 * da * (255 - a) / 255) / out_a.max(1);
            dst[k] = c.min(255) as u8;
        }
        dst[3] = out_a as u8;
    }
}

impl Scene {
    fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut b: Option<(i32, i32, i32, i32)> = None;
        let mut add = |x0: i32, y0: i32, x1: i32, y1: i32| {
            b = Some(match b {
                None => (x0, y0, x1, y1),
                Some((a, c, d, e)) => (a.min(x0), c.min(y0), d.max(x1), e.max(y1)),
            });
        };
        for s in &self.sprites {
            add(s.x, s.y, s.x + s.img.w as i32, s.y + s.img.h as i32);
        }
        b
    }

    /// One image covering every sprite, and its screen top-left.
    pub fn compose(&self) -> (Rgba, i32, i32) {
        let Some((x0, y0, x1, y1)) = self.bounds() else { return (Rgba::new(1, 1), 0, 0) };
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut img = Rgba::new(w.max(1), h.max(1));
        for s in &self.sprites {
            for sy in 0..s.img.h {
                let row = (s.y - y0) as usize + sy;
                let si = sy * s.img.w * 4;
                let di = (row * w + (s.x - x0) as usize) * 4;
                for k in 0..s.img.w {
                    over(&mut img.px[di + k * 4..di + k * 4 + 4], &s.img.px[si + k * 4..si + k * 4 + 4]);
                }
            }
        }
        (img, x0, y0)
    }

    /// What the cursor at screen `(x, y)` is on: the topmost pet or toy with
    /// an opaque pixel there (the pet holds a hover anywhere inside its image
    /// once `sticky_pet`, as `view::over_image`). Food and effects never
    /// take a click.
    pub fn hit(&self, x: i32, y: i32, sticky_pet: bool) -> Hit {
        for s in self.sprites.iter().rev() {
            let on = |sticky: bool| crate::view::over_image(&s.img, (s.x, s.y), (x, y), sticky);
            match s.part {
                Part::Toy if on(false) => return Hit::Toy,
                Part::Pet if on(sticky_pet) => return Hit::Pet,
                _ => {}
            }
        }
        Hit::None
    }
}
