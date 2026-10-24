//! A pet file as a PNG sheet, to look at a design before shipping it:
//! `cargo run --release --example pet_preview -- <pet.txt> <out.png> [zoom]`.
//! Top row: stand, walk, sit, run, eat, sleep, side-on. Bottom row: the
//! head-on tilt, plus the pet on a dark and a light desktop for contrast.

use pixelpet::pets;
use pixelpet::rig::{self, Anim, Frame};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("usage: pet_preview <pet.txt> <out.png> [zoom]");
        std::process::exit(2);
    }
    let text = std::fs::read_to_string(&a[1]).expect("read the pet file");
    let rec = pets::from_text("preview", &text).expect("not a pet file (species must be dog, rabbit or cat)");
    let zoom: usize = a.get(3).and_then(|z| z.parse().ok()).unwrap_or(4);
    let look = rig::look("classic");
    let d = &rec.design;
    let frame = |anim: Anim, t: f64| rig::render_for(d, &rig::pose_for(d, anim, look, t, None), look);
    let top = [(Anim::Stand, 0.3), (Anim::Walk, 0.2), (Anim::Sit, 0.5), (Anim::Run, 0.1), (Anim::Eat, 0.3), (Anim::Sleep, 1.0)];
    let bottom = [(Anim::Tilt, 1.5), (Anim::Stand, 0.3), (Anim::Stand, 0.3)];
    let f0 = frame(Anim::Stand, 0.0);
    let (cw, ch) = (f0.w * zoom, f0.h * zoom);
    let (w, h) = (cw * top.len(), ch * 2);
    let sky = [142, 201, 240];
    let mut px: Vec<u8> = sky.iter().copied().cycle().take(w * h * 3).collect();
    let mut paste = |f: &Frame, col: usize, row: usize, bg: [u8; 3]| {
        for y in 0..ch {
            for x in 0..cw {
                let (sx, sy) = (x / zoom, y / zoom);
                let ground = sy as i32 > f.gy;
                let mut out = if ground { [60, 90, 60] } else { bg };
                if sx < f.w && sy < f.h {
                    let s = (sy * f.w + sx) * 4;
                    let al = f.rgba[s + 3] as u32;
                    for k in 0..3 {
                        out[k] = ((f.rgba[s + k] as u32 * al + out[k] as u32 * (255 - al)) / 255) as u8;
                    }
                }
                let o = ((row * ch + y) * w + col * cw + x) * 3;
                px[o..o + 3].copy_from_slice(&out);
            }
        }
    };
    for (i, &(anim, t)) in top.iter().enumerate() {
        paste(&frame(anim, t), i, 0, sky);
    }
    for (i, &(anim, t)) in bottom.iter().enumerate() {
        let bg = [sky, [35, 42, 54], [238, 238, 232]][i];
        paste(&frame(anim, t), i, 1, bg);
    }
    std::fs::write(&a[2], png(w, h, &px)).expect("write png");
    println!("{} ({}, {:?}): wrote {} ({w}x{h})", rec.name, rec.breed, d.pet, a[2]);
}

/// An RGB PNG, deflate "stored" (uncompressed) blocks: no crates needed.
fn png(w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for row in rgb.chunks(w * 3) {
        raw.push(0); // no filter
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    while let Some(b) = blocks.next() {
        z.push(if blocks.peek().is_none() { 1 } else { 0 });
        z.extend_from_slice(&(b.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut s1, mut s2) = (1u32, 0u32);
    for &x in &raw {
        s1 = (s1 + x as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    z.extend_from_slice(&((s2 << 16) | s1).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    for (kind, data) in [(&b"IHDR"[..], ihdr), (b"IDAT", z), (b"IEND", vec![])] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = kind.to_vec();
        c.extend_from_slice(&data);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc(&c).to_be_bytes());
    }
    out
}

fn crc(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}
