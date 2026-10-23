//! Reading a JPEG, PNG or WebP into an upright, small `Photo`.

use std::io::Cursor;
use std::path::Path;

use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

use super::{Photo, MAX_SIDE};

/// Most pixels a picture may have before it is refused.
const MAX_PIXELS: u64 = 50_000_000;
/// Biggest file that is read at all.
const MAX_FILE: u64 = 64 << 20;

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    /// Not a JPEG, PNG or WebP, or one that can't be decoded.
    NotAPhoto,
    /// The file or the picture is too big to open.
    TooBig,
}

/// Read and decode the file at `path`.
pub fn open(path: &Path) -> Result<Photo, LoadError> {
    let len = std::fs::metadata(path).map_err(LoadError::Io)?.len();
    if len > MAX_FILE {
        return Err(LoadError::TooBig);
    }
    let bytes = std::fs::read(path).map_err(LoadError::Io)?;
    decode(&bytes)
}

/// Decode a JPEG, PNG or WebP, told apart by its first bytes.
pub fn decode(bytes: &[u8]) -> Result<Photo, LoadError> {
    let (w, h, rgb, orientation) = if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let (w, h, rgb) = decode_jpeg(bytes)?;
        (w, h, rgb, exif_orientation(bytes))
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        let (w, h, rgb) = decode_png(bytes)?;
        (w, h, rgb, 1)
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        // what browsers often save, even as ".jpg"
        let (w, h, rgb) = decode_webp(bytes)?;
        (w, h, rgb, 1)
    } else {
        return Err(LoadError::NotAPhoto);
    };
    let (w, h, rgb) = shrink(w, h, rgb);
    Ok(orient(Photo { w, h, rgb }, orientation))
}

fn check_size(w: usize, h: usize) -> Result<(), LoadError> {
    if w == 0 || h == 0 {
        return Err(LoadError::NotAPhoto);
    }
    if (w as u64) * (h as u64) > MAX_PIXELS {
        return Err(LoadError::TooBig);
    }
    Ok(())
}

fn decode_jpeg(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), LoadError> {
    // lift the decoder's own size limits so MAX_PIXELS is the one that decides
    let opts = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::RGB)
        .set_max_width(65535)
        .set_max_height(65535);
    let mut d = JpegDecoder::new_with_options(ZCursor::new(bytes), opts);
    d.decode_headers().map_err(|_| LoadError::NotAPhoto)?;
    let (w, h) = d.dimensions().ok_or(LoadError::NotAPhoto)?;
    check_size(w, h)?;
    let rgb = d.decode().map_err(|_| LoadError::NotAPhoto)?;
    if rgb.len() != w * h * 3 {
        return Err(LoadError::NotAPhoto);
    }
    Ok((w, h, rgb))
}

fn decode_png(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), LoadError> {
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    // palettes expanded, 16-bit cut to 8
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().map_err(|_| LoadError::NotAPhoto)?;
    let (w, h) = (r.info().width as usize, r.info().height as usize);
    check_size(w, h)?;
    let mut buf = vec![0; r.output_buffer_size().ok_or(LoadError::TooBig)?];
    let out = r.next_frame(&mut buf).map_err(|_| LoadError::NotAPhoto)?;
    buf.truncate(out.buffer_size());
    let n = match out.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(LoadError::NotAPhoto),
    };
    if buf.len() != w * h * n {
        return Err(LoadError::NotAPhoto);
    }
    let mut rgb = Vec::with_capacity(w * h * 3);
    for p in buf.chunks_exact(n) {
        let (c, a) = match n {
            1 => ([p[0]; 3], 255),
            2 => ([p[0]; 3], p[1]),
            3 => ([p[0], p[1], p[2]], 255),
            _ => ([p[0], p[1], p[2]], p[3]),
        };
        // alpha over white
        rgb.extend(c.map(|v| ((v as u32 * a as u32 + 255 * (255 - a as u32) + 127) / 255) as u8));
    }
    Ok((w, h, rgb))
}

fn decode_webp(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), LoadError> {
    let mut d = image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(|_| LoadError::NotAPhoto)?;
    let (w, h) = d.dimensions();
    let (w, h) = (w as usize, h as usize);
    check_size(w, h)?;
    let mut buf = vec![0; d.output_buffer_size().ok_or(LoadError::TooBig)?];
    // an animated one gives its first frame
    d.read_image(&mut buf).map_err(|_| LoadError::NotAPhoto)?;
    if !d.has_alpha() {
        return Ok((w, h, buf));
    }
    // alpha over white
    let rgb = buf
        .chunks_exact(4)
        .flat_map(|p| [0, 1, 2].map(|i| ((p[i] as u32 * p[3] as u32 + 255 * (255 - p[3] as u32) + 127) / 255) as u8))
        .collect();
    Ok((w, h, rgb))
}

/// The EXIF orientation (1..=8) of a JPEG, 1 when absent or unreadable.
/// Only the APP1 orientation tag is read.
fn exif_orientation(jpeg: &[u8]) -> u8 {
    let mut i = 2;
    while i + 4 <= jpeg.len() && jpeg[i] == 0xFF {
        let marker = jpeg[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let len = u16::from_be_bytes([jpeg[i + 2], jpeg[i + 3]]) as usize;
        let end = (i + 2 + len).min(jpeg.len());
        let body = jpeg.get(i + 4..end).unwrap_or(&[]);
        if marker == 0xE1 && body.starts_with(b"Exif\0\0") {
            return tiff_orientation(&body[6..]).unwrap_or(1);
        }
        i += 2 + len;
    }
    1
}

fn tiff_orientation(t: &[u8]) -> Option<u8> {
    let big = match t.get(..2)? {
        b"MM" => true,
        b"II" => false,
        _ => return None,
    };
    let u16_at = |o: usize| -> Option<u16> {
        let b = [*t.get(o)?, *t.get(o + 1)?];
        Some(if big { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) })
    };
    let u32_at = |o: usize| -> Option<usize> {
        let b = [*t.get(o)?, *t.get(o + 1)?, *t.get(o + 2)?, *t.get(o + 3)?];
        Some(if big { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) } as usize)
    };
    let ifd = u32_at(4)?;
    for n in 0..u16_at(ifd)? as usize {
        let e = ifd + 2 + n * 12;
        if u16_at(e)? == 0x0112 {
            let v = u16_at(e + 8)?;
            return (1..=8).contains(&v).then_some(v as u8);
        }
    }
    None
}

/// Box-average down so the long side is at most `MAX_SIDE`.
fn shrink(w: usize, h: usize, rgb: Vec<u8>) -> (u32, u32, Vec<u8>) {
    let long = w.max(h);
    if long <= MAX_SIDE as usize {
        return (w as u32, h as u32, rgb);
    }
    let scale = |n: usize| ((n * MAX_SIDE as usize + long / 2) / long).max(1);
    let (nw, nh) = (scale(w), scale(h));
    let mut out = Vec::with_capacity(nw * nh * 3);
    for y in 0..nh {
        let (y0, y1) = (y * h / nh, ((y + 1) * h / nh).max(y * h / nh + 1));
        for x in 0..nw {
            let (x0, x1) = (x * w / nw, ((x + 1) * w / nw).max(x * w / nw + 1));
            let mut sum = [0u32; 3];
            for yy in y0..y1 {
                for xx in x0..x1 {
                    let p = &rgb[(yy * w + xx) * 3..][..3];
                    for c in 0..3 {
                        sum[c] += p[c] as u32;
                    }
                }
            }
            let count = ((y1 - y0) * (x1 - x0)) as u32;
            out.extend(sum.map(|s| ((s + count / 2) / count) as u8));
        }
    }
    (nw as u32, nh as u32, out)
}

/// Turn a photo upright from its EXIF orientation (1..=8).
fn orient(p: Photo, o: u8) -> Photo {
    if o <= 1 {
        return p;
    }
    let (w, h) = (p.w as usize, p.h as usize);
    let (nw, nh) = if o >= 5 { (h, w) } else { (w, h) };
    let mut rgb = vec![0; p.rgb.len()];
    for y in 0..nh {
        for x in 0..nw {
            // where this upright pixel came from
            let (sx, sy) = match o {
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                4 => (x, h - 1 - y),
                5 => (y, x),
                6 => (y, h - 1 - x),
                7 => (w - 1 - y, h - 1 - x),
                _ => (w - 1 - y, x),
            };
            let (s, d) = ((sy * w + sx) * 3, (y * nw + x) * 3);
            rgb[d..d + 3].copy_from_slice(&p.rgb[s..s + 3]);
        }
    }
    Photo { w: nw as u32, h: nh as u32, rgb }
}
