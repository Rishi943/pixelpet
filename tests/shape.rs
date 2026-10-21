//! The X11 shape mask: rows of pixels at or above an alpha threshold.

use pixelpet::window::shape::shape_runs;
use pixelpet::window::Rgba;

fn img(rows: &[&[u8]]) -> Rgba {
    let (w, h) = (rows[0].len(), rows.len());
    let mut i = Rgba::new(w, h);
    for (y, r) in rows.iter().enumerate() {
        for (x, a) in r.iter().enumerate() {
            i.px[(y * w + x) * 4..][..4].copy_from_slice(&[200, 100, 50, *a]);
        }
    }
    i
}

#[test]
fn shape_runs_cover_only_opaque_pixels() {
    let i = img(&[&[255, 255, 0, 255], &[0, 127, 128, 0]]);
    assert_eq!(shape_runs(&i, 128), vec![(0, 0, 2, 1), (3, 0, 1, 1), (2, 1, 1, 1)]);
    // threshold 1: everything not fully clear, as the Windows layered window does
    assert_eq!(shape_runs(&i, 1), vec![(0, 0, 2, 1), (3, 0, 1, 1), (1, 1, 2, 1)]);
}

#[test]
fn a_run_reaching_the_right_edge_is_closed() {
    assert_eq!(shape_runs(&img(&[&[0, 255, 255]]), 1), vec![(1, 0, 2, 1)]);
}

#[test]
fn empty_image_has_no_runs() {
    assert!(shape_runs(&Rgba::new(3, 3), 128).is_empty());
}
