//! Render anim frames straight from the rig onto a grey sheet, enlarged, for
//! looking at poses closely: one row per anim, one column per time.
//! `cargo run --release --example anim_sheet -- <dog|rabbit> <out.ppm> <zoom> <anim,anim,...> <t,t,...>`
//! `... -- props <out.ppm> <zoom>`: every food at 0-3 bites, and the heart.
//! Writes a binary PPM (convert with any image tool).

use pixelpet::rig::{self, Anim, Pet};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() >= 4 && a[1] == "props" {
        return props_sheet(&a[2], a[3].parse().expect("zoom"));
    }
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

fn props_sheet(out: &str, zoom: usize) {
    use pixelpet::rig::{props, Food, Frame};
    let foods = [Food::Chicken, Food::Bone, Food::Kibble, Food::Carrot, Food::Coriander, Food::Banana];
    let mut rows: Vec<Vec<Frame>> = foods.iter().map(|&f| (0..4).map(|b| props::food(f, b)).collect()).collect();
    rows.push(vec![props::heart()]);
    let cell = 18 * zoom;
    let (w, h) = (cell * 4, cell * rows.len());
    let mut px = vec![150u8; w * h * 3];
    for (r, row) in rows.iter().enumerate() {
        for (c, f) in row.iter().enumerate() {
            for y in 0..f.h * zoom {
                for x in 0..f.w * zoom {
                    let s = ((y / zoom) * f.w + x / zoom) * 4;
                    if f.rgba[s + 3] == 0 {
                        continue;
                    }
                    let d = ((r * cell + zoom + y) * w + c * cell + zoom + x) * 3;
                    px[d..d + 3].copy_from_slice(&f.rgba[s..s + 3]);
                }
            }
        }
    }
    let mut bytes = format!("P6
{w} {h}
255
").into_bytes();
    bytes.extend_from_slice(&px);
    std::fs::write(out, bytes).expect("write");
    println!("wrote {out} ({w}x{h})");
}
