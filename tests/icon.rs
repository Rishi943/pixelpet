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
