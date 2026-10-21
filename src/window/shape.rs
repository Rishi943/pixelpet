//! An image's opaque pixels as rectangles, for X11 window shapes.

use super::Rgba;

/// One `(x, y, w, 1)` per horizontal run of pixels with alpha >= `min_alpha`,
/// top to bottom, left to right.
pub fn shape_runs(img: &Rgba, min_alpha: u8) -> Vec<(i32, i32, i32, i32)> {
    let mut runs = Vec::new();
    for y in 0..img.h {
        let mut start = None;
        for x in 0..=img.w {
            let solid = x < img.w && img.px[(y * img.w + x) * 4 + 3] >= min_alpha;
            match (solid, start) {
                (true, None) => start = Some(x),
                (false, Some(s)) => {
                    runs.push((s as i32, y as i32, (x - s) as i32, 1));
                    start = None;
                }
                _ => {}
            }
        }
    }
    runs
}
