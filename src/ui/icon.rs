//! The app icon: Alex sitting, cropped to a square and scaled up
//! nearest-neighbour, written as a Windows `.ico` (32-bit BMP images).

use crate::rig::{self, Anim, Pet};
use crate::window::Rgba;

/// Image sizes in the `.ico`, smallest first.
pub const SIZES: [u32; 4] = [16, 32, 48, 256];

/// The icon as a `size` x `size` straight-alpha image.
pub fn image(size: u32) -> Rgba {
    let look = rig::look("classic");
    let f = rig::render(&rig::pose(Pet::Dog, Anim::Sit, look, 0.0, None), look, false);
    let alpha = |x: usize, y: usize| f.rgba[(y * f.w + x) * 4 + 3];
    // the opaque bounds, padded to a square around their centre
    let (mut x0, mut y0, mut x1, mut y1) = (f.w, f.h, 0, 0);
    for y in 0..f.h {
        for x in 0..f.w {
            if alpha(x, y) > 0 {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    let side = (x1 - x0).max(y1 - y0) as i32 + 2;
    let (cx, cy) = ((x0 + x1) as i32, (y0 + y1) as i32);
    let (left, top) = ((cx - side) / 2, (cy - side) / 2);
    let size = size as usize;
    let mut img = Rgba::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let sx = left + (x as i32 * side) / size as i32;
            let sy = top + (y as i32 * side) / size as i32;
            if sx >= 0 && sy >= 0 && (sx as usize) < f.w && (sy as usize) < f.h {
                let s = (sy as usize * f.w + sx as usize) * 4;
                let d = (y * size + x) * 4;
                img.px[d..d + 4].copy_from_slice(&f.rgba[s..s + 4]);
            }
        }
    }
    img
}

/// The `.ico` file bytes.
pub fn ico() -> Vec<u8> {
    let images: Vec<Vec<u8>> = SIZES.iter().map(|&s| bmp(&image(s))).collect();
    let mut out = vec![];
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(SIZES.len() as u16).to_le_bytes());
    let mut off = 6 + 16 * SIZES.len();
    for (&s, b) in SIZES.iter().zip(&images) {
        let d = (s % 256) as u8; // 0 means 256
        out.extend_from_slice(&[d, d, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(b.len() as u32).to_le_bytes());
        out.extend_from_slice(&(off as u32).to_le_bytes());
        off += b.len();
    }
    for b in images {
        out.extend_from_slice(&b);
    }
    out
}

/// One icon image: BITMAPINFOHEADER, BGRA rows bottom-up, then the 1-bit
/// AND mask (set where transparent).
fn bmp(img: &Rgba) -> Vec<u8> {
    let (w, h) = (img.w, img.h);
    let mut b = vec![];
    for v in [40u32, w as u32, 2 * h as u32] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    for _ in 0..6 {
        b.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB, sizes, colours
    }
    for y in (0..h).rev() {
        for x in 0..w {
            let p = &img.px[(y * w + x) * 4..][..4];
            b.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    let row = w.div_ceil(32) * 4;
    for y in (0..h).rev() {
        let mut bits = vec![0u8; row];
        for x in 0..w {
            if img.px[(y * w + x) * 4 + 3] == 0 {
                bits[x / 8] |= 0x80 >> (x % 8);
            }
        }
        b.extend_from_slice(&bits);
    }
    b
}
