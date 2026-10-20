//! Render anim frames straight from the rig onto a grey sheet, enlarged, for
//! looking at poses closely: one row per anim, one column per time.
//! `cargo run --release --example anim_sheet -- <dog|rabbit> <out.ppm> <zoom> <anim,anim,...> <t,t,...>`
//! Writes a binary PPM (convert with any image tool).

use pixelpet::rig::{self, Anim, Pet};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 6 {
        eprintln!("usage: anim_sheet <dog|rabbit> <out.ppm> <zoom> <anims> <times>");
        std::process::exit(2);
    }
    let pet = if a[1] == "rabbit" { Pet::Rabbit } else { Pet::Dog };
    let zoom: usize = a[3].parse().expect("zoom");
    let anims: Vec<Anim> = a[4].split(',').map(|n| Anim::from_name(n).unwrap_or_else(|| panic!("no anim {n}"))).collect();
    let times: Vec<f64> = a[5].split(',').map(|t| t.parse().expect("time")).collect();
    let look = rig::look("classic");
    let fs = rig::core::frame_size(look);
    let (cw, ch) = (fs.w * zoom, fs.h * zoom);
    let (w, h) = (cw * times.len(), ch * anims.len());
    let mut px = vec![150u8; w * h * 3];
    for (r, &anim) in anims.iter().enumerate() {
        for (c, &t) in times.iter().enumerate() {
            let f = rig::render(&rig::pose(pet, anim, look, t, None), look, false);
            for y in 0..f.h * zoom {
                for x in 0..f.w * zoom {
                    let s = ((y / zoom) * f.w + x / zoom) * 4;
                    let al = f.rgba[s + 3] as u32;
                    let d = ((r * ch + y) * w + c * cw + x) * 3;
                    for k in 0..3 {
                        let bg = if (x / zoom + y / zoom) % 2 == 0 { 150 } else { 140 };
                        px[d + k] = ((f.rgba[s + k] as u32 * al + bg * (255 - al)) / 255) as u8;
                    }
                }
            }
            // ground line
            let gy = r * ch + (f.gy as usize + 1) * zoom;
            for x in c * cw..(c + 1) * cw {
                let d = (gy.min(h - 1) * w + x) * 3;
                px[d..d + 3].copy_from_slice(&[60, 90, 60]);
            }
        }
    }
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    out.extend_from_slice(&px);
    std::fs::write(&a[2], out).expect("write");
    println!("wrote {} ({w}x{h})", a[2]);
}
