//! The app icon: Alex sitting, as a Windows .ico with 16-256 px images.

use pixelpet::ui::icon;

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

#[test]
fn ico_holds_four_square_sizes() {
    let b = icon::ico();
    assert_eq!((u16_at(&b, 0), u16_at(&b, 2), u16_at(&b, 4)), (0, 1, 4), "reserved, type icon, 4 images");
    for (k, size) in icon::SIZES.iter().enumerate() {
        let e = 6 + k * 16;
        // 0 in the directory means 256
        assert_eq!(b[e] as u32, size % 256);
        assert_eq!(b[e + 1] as u32, size % 256);
        assert_eq!(u16_at(&b, e + 6), 32, "32 bits per pixel");
        let (len, off) = (u32_at(&b, e + 8) as usize, u32_at(&b, e + 12) as usize);
        assert!(off + len <= b.len());
        // the image is a BMP info header: width, then double height (XOR + AND masks)
        assert_eq!(u32_at(&b, off), 40);
        assert_eq!(u32_at(&b, off + 4), *size);
        assert_eq!(u32_at(&b, off + 8), size * 2);
        let mask_row = (size.div_ceil(32) * 4) as usize;
        assert_eq!(len, 40 + (size * size * 4) as usize + mask_row * *size as usize);
    }
}

#[test]
fn icon_shows_the_pet() {
    let img = icon::image(32);
    assert_eq!((img.w, img.h), (32, 32));
    let opaque = img.px.chunks(4).filter(|p| p[3] == 255).count();
    let clear = img.px.chunks(4).filter(|p| p[3] == 0).count();
    // a pet on a see-through square, filling a fair share of it
    assert!(opaque > 32 * 32 / 4, "{opaque} opaque px");
    assert!(clear > 32 * 32 / 10, "{clear} clear px");
}

/// Read back a PNG written with stored (uncompressed) deflate blocks.
fn decode_stored_png(b: &[u8]) -> (u32, u32, Vec<u8>) {
    assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n");
    let (mut pos, mut idat, mut wh) = (8, Vec::new(), (0, 0));
    while pos < b.len() {
        let n = u32::from_be_bytes(b[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &b[pos + 4..pos + 8];
        let body = &b[pos + 8..pos + 8 + n];
        let crc = u32::from_be_bytes(b[pos + 8 + n..pos + 12 + n].try_into().unwrap());
        assert_eq!(crc, crc32(&b[pos + 4..pos + 8 + n]), "chunk CRC");
        if kind == b"IHDR" {
            wh = (u32::from_be_bytes(body[0..4].try_into().unwrap()), u32::from_be_bytes(body[4..8].try_into().unwrap()));
            assert_eq!(&body[8..13], &[8, 6, 0, 0, 0], "8-bit RGBA, no interlace");
        } else if kind == b"IDAT" {
            idat.extend_from_slice(body);
        }
        pos += 12 + n;
    }
    // zlib: 2-byte header, stored blocks, adler32
    let (mut i, mut raw) = (2, Vec::new());
    loop {
        let last = idat[i] & 1;
        assert_eq!(idat[i] & 6, 0, "stored block");
        let len = u16::from_le_bytes([idat[i + 1], idat[i + 2]]) as usize;
        raw.extend_from_slice(&idat[i + 5..i + 5 + len]);
        i += 5 + len;
        if last == 1 {
            break;
        }
    }
    let adler = u32::from_be_bytes(idat[i..i + 4].try_into().unwrap());
    let (mut a, mut s) = (1u32, 0u32);
    for &c in &raw {
        a = (a + c as u32) % 65521;
        s = (s + a) % 65521;
    }
    assert_eq!(adler, (s << 16) | a, "adler32");
    // drop the per-row filter byte (all 0)
    let stride = wh.0 as usize * 4 + 1;
    assert!(raw.chunks(stride).all(|r| r[0] == 0));
    (wh.0, wh.1, raw.chunks(stride).flat_map(|r| r[1..].to_vec()).collect())
}

fn crc32(b: &[u8]) -> u32 {
    let mut c = !0u32;
    for &x in b {
        c ^= x as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

#[test]
fn png_round_trips_the_icon_image() {
    for size in [16, 256, 1024] {
        let (w, h, px) = decode_stored_png(&icon::png(size));
        assert_eq!((w, h), (size, size));
        assert_eq!(px, icon::image(size).px, "size {size}");
    }
}
