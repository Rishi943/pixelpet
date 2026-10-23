//! Loading photos: JPEG and PNG, upright, small, and never a panic on a bad file.
//! Fixtures are made here, in memory, from synthetic pixels.

use pixelpet::photo::load::{decode, open, LoadError};
use pixelpet::photo::MAX_SIDE;

/// Six block colours, a 3 x 2 grid: A B C over D E F.
const GRID: [[u8; 3]; 6] = [
    [250, 20, 20],
    [20, 250, 20],
    [20, 20, 250],
    [250, 250, 20],
    [20, 250, 250],
    [120, 120, 120],
];

fn png_bytes(w: u32, h: u32, color: png::ColorType, depth: png::BitDepth, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(color);
    enc.set_depth(depth);
    enc.write_header().unwrap().write_image_data(data).unwrap();
    out
}

fn rgb_png(w: u32, h: u32, data: &[u8]) -> Vec<u8> {
    png_bytes(w, h, png::ColorType::Rgb, png::BitDepth::Eight, data)
}

/// 3 x 2 grid of 8 px blocks (24 x 16), as RGB.
fn blocks() -> Vec<u8> {
    let mut v = Vec::new();
    for y in 0..16 {
        for x in 0..24 {
            v.extend(GRID[(y / 8) * 3 + x / 8]);
        }
    }
    v
}

fn jpeg_bytes(w: u16, h: u16, rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 95).encode(rgb, w, h, jpeg_encoder::ColorType::Rgb).unwrap();
    out
}

/// The same JPEG with an EXIF APP1 segment (orientation `o`) after the SOI.
fn with_orientation(jpeg: &[u8], o: u16, big: bool) -> Vec<u8> {
    let short = |v: u16| if big { v.to_be_bytes() } else { v.to_le_bytes() };
    let long = |v: u32| if big { v.to_be_bytes() } else { v.to_le_bytes() };
    let mut tiff = Vec::new();
    tiff.extend(if big { b"MM" } else { b"II" });
    tiff.extend(short(42));
    tiff.extend(long(8));
    tiff.extend(short(1));
    tiff.extend(short(0x0112));
    tiff.extend(short(3));
    tiff.extend(long(1));
    tiff.extend(short(o));
    tiff.extend([0, 0]);
    tiff.extend(long(0));
    let mut seg = b"Exif\0\0".to_vec();
    seg.extend(tiff);
    let mut out = jpeg[..2].to_vec();
    out.extend([0xFF, 0xE1]);
    out.extend((seg.len() as u16 + 2).to_be_bytes());
    out.extend(seg);
    out.extend(&jpeg[2..]);
    out
}

fn px(p: &pixelpet::photo::Photo, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * p.w + x) * 3) as usize;
    [p.rgb[i], p.rgb[i + 1], p.rgb[i + 2]]
}

fn near(a: [u8; 3], b: [u8; 3], tol: i32) -> bool {
    (0..3).all(|i| (a[i] as i32 - b[i] as i32).abs() <= tol)
}

#[test]
fn png_pixels_come_out_exact() {
    let data: Vec<u8> = (0..18).map(|i| i * 13).collect();
    let p = decode(&rgb_png(3, 2, &data)).unwrap();
    assert_eq!((p.w, p.h), (3, 2));
    assert_eq!(p.rgb, data);
}

#[test]
fn jpeg_matches_the_same_picture() {
    let p = decode(&jpeg_bytes(24, 16, &blocks())).unwrap();
    assert_eq!((p.w, p.h), (24, 16));
    for (i, c) in GRID.iter().enumerate() {
        let (x, y) = ((i % 3) as u32 * 8 + 4, (i / 3) as u32 * 8 + 4);
        assert!(near(px(&p, x, y), *c, 12), "block {i}: {:?} vs {c:?}", px(&p, x, y));
    }
}

#[test]
fn exif_orientation_is_applied() {
    let (a, b, c, d, e, f) = (GRID[0], GRID[1], GRID[2], GRID[3], GRID[4], GRID[5]);
    // (orientation, upright size, upright blocks row by row)
    let cases = [
        (1, (24, 16), vec![a, b, c, d, e, f]),
        (3, (24, 16), vec![f, e, d, c, b, a]),
        (6, (16, 24), vec![d, a, e, b, f, c]),
        (8, (16, 24), vec![c, f, b, e, a, d]),
        (2, (24, 16), vec![c, b, a, f, e, d]),
    ];
    let src = jpeg_bytes(24, 16, &blocks());
    for big in [false, true] {
        for (o, (w, h), want) in &cases {
            let p = decode(&with_orientation(&src, *o, big)).unwrap();
            assert_eq!((p.w, p.h), (*w, *h), "orientation {o}");
            let cols = w / 8;
            for (i, c) in want.iter().enumerate() {
                let (x, y) = (i as u32 % cols * 8 + 4, i as u32 / cols * 8 + 4);
                assert!(near(px(&p, x, y), *c, 12), "orientation {o}, block {i}");
            }
        }
    }
}

#[test]
fn odd_orientation_tags_are_ignored() {
    let src = jpeg_bytes(24, 16, &blocks());
    for o in [0, 9, 300] {
        let p = decode(&with_orientation(&src, o, false)).unwrap();
        assert_eq!((p.w, p.h), (24, 16));
    }
}

#[test]
fn other_png_kinds_become_rgb() {
    let g = png_bytes(2, 1, png::ColorType::Grayscale, png::BitDepth::Eight, &[10, 200]);
    assert_eq!(decode(&g).unwrap().rgb, [10, 10, 10, 200, 200, 200]);

    // opaque, half see-through and clear, over white
    let rgba = [255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 128];
    let a = decode(&png_bytes(3, 1, png::ColorType::Rgba, png::BitDepth::Eight, &rgba)).unwrap();
    assert_eq!(px(&a, 0, 0), [255, 0, 0]);
    assert_eq!(px(&a, 1, 0), [255, 255, 255]);
    assert!(near(px(&a, 2, 0), [127, 127, 127], 2));

    // 16-bit, big-endian samples: 0x8000 -> 128
    let d16 = [0x80, 0, 0x40, 0, 0xFF, 0xFF];
    let w16 = decode(&png_bytes(1, 1, png::ColorType::Rgb, png::BitDepth::Sixteen, &d16)).unwrap();
    assert!(near(px(&w16, 0, 0), [128, 64, 255], 1));
}

#[test]
fn big_photos_shrink_to_the_max_side() {
    let (w, h) = (4000u32, 3000u32);
    let data: Vec<u8> = [30u8, 120, 220].iter().copied().cycle().take((w * h * 3) as usize).collect();
    let p = decode(&rgb_png(w, h, &data)).unwrap();
    assert_eq!((p.w, p.h), (MAX_SIDE, 768));
    assert_eq!(p.rgb.len(), (p.w * p.h * 3) as usize);
    assert_eq!(px(&p, 500, 400), [30, 120, 220]);

    // tall ones too, and small ones are left alone
    let tall = decode(&rgb_png(30, 2000, &vec![9; 30 * 2000 * 3])).unwrap();
    assert_eq!((tall.w, tall.h), (15, 1024));
}

/// Set the frame size inside a JPEG's SOF0 header.
fn patch_jpeg_size(jpeg: &[u8], w: u16, h: u16) -> Vec<u8> {
    let mut v = jpeg.to_vec();
    let i = v.windows(2).position(|m| m == [0xFF, 0xC0]).expect("baseline SOF");
    v[i + 5..i + 7].copy_from_slice(&h.to_be_bytes());
    v[i + 7..i + 9].copy_from_slice(&w.to_be_bytes());
    v
}

/// Set the size in a PNG's IHDR, fixing its checksum.
fn patch_png_size(png: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut v = png.to_vec();
    v[16..20].copy_from_slice(&w.to_be_bytes());
    v[20..24].copy_from_slice(&h.to_be_bytes());
    let mut crc = !0u32;
    for &b in &v[12..29] {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 };
        }
    }
    v[29..33].copy_from_slice(&(!crc).to_be_bytes());
    v
}

#[test]
fn bad_files_are_errors_not_panics() {
    let jpeg = jpeg_bytes(24, 16, &blocks());
    let png = rgb_png(24, 16, &blocks());

    // truncated
    assert!(decode(&jpeg[..jpeg.len() / 2]).is_err());
    assert!(decode(&jpeg[..10]).is_err());
    assert!(decode(&png[..png.len() / 2]).is_err());
    assert!(decode(&png[..20]).is_err());

    // empty, junk, wrong kind
    assert!(matches!(decode(&[]), Err(LoadError::NotAPhoto)));
    assert!(matches!(decode(b"GIF89a not a photo"), Err(LoadError::NotAPhoto)));
    let mut seed = 12345u32;
    let junk: Vec<u8> = (0..4000)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect();
    assert!(decode(&junk).is_err());
    for head in [&[0xFF, 0xD8, 0xFF][..], &[0x89, b'P', b'N', b'G'][..]] {
        let mut v = head.to_vec();
        v.extend(&junk);
        assert!(decode(&v).is_err());
    }

    // declared huge: refused from the header alone
    let huge_png = patch_png_size(&png, 30000, 30000);
    assert!(matches!(decode(&huge_png), Err(LoadError::TooBig)));
    assert!(matches!(decode(&patch_jpeg_size(&jpeg, 30000, 30000)), Err(LoadError::TooBig)));

    // 0 x 0
    assert!(decode(&patch_jpeg_size(&jpeg, 0, 0)).is_err());
    assert!(decode(&patch_png_size(&png, 0, 0)).is_err());
}

#[test]
fn files_are_judged_by_content_not_extension() {
    let dir = std::env::temp_dir().join(format!("pixelpet-photo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let png_as_jpg = dir.join("a.jpg");
    std::fs::write(&png_as_jpg, rgb_png(3, 2, &[7; 18])).unwrap();
    assert_eq!(open(&png_as_jpg).unwrap().w, 3);
    let text_as_png = dir.join("b.png");
    std::fs::write(&text_as_png, "hello").unwrap();
    assert!(matches!(open(&text_as_png), Err(LoadError::NotAPhoto)));
    assert!(matches!(open(&dir.join("missing.png")), Err(LoadError::Io(_))));
    let _ = std::fs::remove_dir_all(&dir);
}
