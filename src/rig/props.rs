//! Props for play: food, hearts, crumbs and the tug rope. Food and hearts
//! are hand-drawn pixel maps in the pets' style (1 px dark outline, a few
//! tones); one char per art pixel. Frames have their ground point at the
//! bottom centre, like the pet's.

use super::{Frame, Pet};
use crate::window::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Food {
    Chicken,
    Bone,
    Kibble,
    Carrot,
    Coriander,
    Banana,
}

impl Food {
    /// The three foods a pet can be given, default first (spec §5 Feed).
    pub fn for_pet(pet: Pet) -> [Food; 3] {
        match pet {
            Pet::Dog => [Food::Chicken, Food::Bone, Food::Kibble],
            Pet::Rabbit => [Food::Carrot, Food::Coriander, Food::Banana],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Food::Chicken => "Chicken",
            Food::Bone => "Bone",
            Food::Kibble => "Kibble",
            Food::Carrot => "Carrot",
            Food::Coriander => "Coriander",
            Food::Banana => "Banana",
        }
    }
}

const OUTLINE: u32 = 0x24120A;

struct Art {
    rows: &'static [&'static str],
    /// (char, colour); `o` is always the outline.
    pal: &'static [(char, u32)],
    /// the crumb colour
    crumb: u32,
}

fn art(f: Food) -> Art {
    match f {
        // a drumstick: meat on the left, the bone end on the right
        Food::Chicken => Art {
            rows: &[
                "..ooooo......",
                ".o22333o.....",
                "o1222233o....",
                "o11222223oooo",
                "o112222oaabbo",
                ".o1112oaaoaao",
                "..ooooo.oo.oo",
            ],
            pal: &[('1', 0x8A4A1C), ('2', 0xB8692C), ('3', 0xE0974F), ('a', 0xD2C2A6), ('b', 0xFFF4E2)],
            crumb: 0xB8692C,
        },
        Food::Bone => Art {
            rows: &[
                ".oo.......oo.",
                "o22ooooooo23o",
                ".o222222223o.",
                "o12ooooooo12o",
                ".oo.......oo.",
            ],
            pal: &[('1', 0xB0A088), ('2', 0xD2C2A6), ('3', 0xFFF4E2)],
            crumb: 0xD2C2A6,
        },
        // a blue bowl heaped with kibble
        Food::Kibble => Art {
            rows: &[
                "...o2o.o1o....",
                "..o121o212oo..",
                ".o1212121212o.",
                "oaaaaaaaaaaaao",
                ".obbbbbbbbbbo.",
                "..obbbbbbbbo..",
                "...oooooooo...",
            ],
            pal: &[('1', 0x6A3518), ('2', 0x9A5226), ('a', 0x5E8FE6), ('b', 0x2F64C8)],
            crumb: 0x9A5226,
        },
        // lying down, tip to the left, leaves to the right
        Food::Carrot => Art {
            rows: &[
                "..........g.g.",
                "....ooooooggg.",
                "..oo3332221ogh",
                "oo1222222211og",
                "..oo1111111og.",
                "....oooooooo..",
            ],
            pal: &[('1', 0xC4561A), ('2', 0xE8782A), ('3', 0xF8A050), ('g', 0x3E8A2E), ('h', 0x7CC04A)],
            crumb: 0xE8782A,
        },
        // a sprig: lobed leaves on thin stems, no outline (like the carrot top)
        Food::Coriander => Art {
            rows: &[
                "...hh.hh...",
                "..hggshggh.",
                "...ggsgg...",
                ".hh.s..hh..",
                "hggh.s.hggh",
                ".gg..shgg..",
                ".....s.....",
                "....s......",
                "....s......",
            ],
            pal: &[('g', 0x2E7A2A), ('h', 0x6CB848), ('s', 0x4E8A2E)],
            crumb: 0x2E7A2A,
        },
        // a slice: pale flesh, a darker ring of seeds, a yellow rim
        Food::Banana => Art {
            rows: &[
                "..ooooo..",
                ".o33333o.",
                "o3322233o",
                "o3212123o",
                "o3221223o",
                "o3212123o",
                "o3322233o",
                ".o33333o.",
                "..ooooo..",
            ],
            pal: &[('1', 0x8A6A2A), ('2', 0xFFF3C0), ('3', 0xF0D878)],
            crumb: 0xFFF3C0,
        },
    }
}

fn rgba(c: u32) -> [u8; 4] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]
}

fn draw(rows: &[&str], pal: &[(char, u32)], keep_w: usize) -> Frame {
    let h = rows.len();
    let w = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut out = vec![0u8; w * h * 4];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if x >= keep_w {
                continue;
            }
            let col = match ch {
                'o' => Some(OUTLINE),
                _ => pal.iter().find(|(c, _)| *c == ch).map(|(_, v)| *v),
            };
            if let Some(c) = col {
                // a bitten edge gets the outline, so the cut reads
                let c = if keep_w < w && x + 1 == keep_w { OUTLINE } else { c };
                out[(y * w + x) * 4..][..4].copy_from_slice(&rgba(c));
            }
        }
    }
    Frame { w, h, ox: (w / 2) as i32, gy: h as i32 - 1, rgba: out }
}

/// `food` after `bites` bites (0 whole … 3 gone): each bite takes a third
/// off the right-hand side.
pub fn food(food: Food, bites: u8) -> Frame {
    let a = art(food);
    let w = a.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let keep = match bites {
        0 => w,
        1 => w * 2 / 3,
        2 => w / 3,
        _ => 0,
    };
    draw(a.rows, a.pal, keep)
}

/// One crumb: a single art pixel of the food's colour.
pub fn crumb(food: Food) -> [u8; 4] {
    rgba(art(food).crumb)
}

/// A small heart, floated up for each stroke when the pet is petted.
pub fn heart() -> Frame {
    const ROWS: &[&str] = &[
        ".oo...oo.",
        "o223o223o",
        "o2222223o",
        "o1222222o",
        ".o11222o.",
        "..o112o..",
        "...o1o...",
        "....o....",
    ];
    draw(ROWS, &[('1', 0x9E1E34), ('2', 0xD9364A), ('3', 0xF59AA6)], usize::MAX)
}

const ROPE: [u32; 2] = [0xC8A060, 0x9C7A40];

/// Draw the tug rope from `from` to `to` (image px), one art pixel thick
/// (`zoom` px blocks on the image's art grid), two tones twisting along it,
/// its middle sagging `sag` px below the straight line.
pub fn rope(img: &mut Rgba, from: (i32, i32), to: (i32, i32), zoom: u32, sag: f64) {
    let z = zoom.max(1) as i32;
    let (x0, y0) = (from.0 as f64, from.1 as f64);
    let (x2, y2) = (to.0 as f64, to.1 as f64);
    // quadratic curve whose middle is `sag` below the chord's middle
    let (cx, cy) = ((x0 + x2) / 2.0, (y0 + y2) / 2.0 + 2.0 * sag);
    let steps = (((x2 - x0).abs() + (y2 - y0).abs()) / z as f64 * 2.0).ceil().max(1.0) as usize;
    let mut last = None;
    let mut n = 0usize;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let u = 1.0 - t;
        let x = u * u * x0 + 2.0 * u * t * cx + t * t * x2;
        let y = u * u * y0 + 2.0 * u * t * cy + t * t * y2;
        let b = ((x as i32).div_euclid(z), (y as i32).div_euclid(z));
        if last == Some(b) {
            continue;
        }
        last = Some(b);
        n += 1;
        let c = rgba(ROPE[(n / 2) % 2]);
        let (bx, by) = (b.0 * z, b.1 * z);
        if bx < 0 || by < 0 || bx + z > img.w as i32 || by + z > img.h as i32 {
            continue;
        }
        for yy in by..by + z {
            for xx in bx..bx + z {
                let d = (yy as usize * img.w + xx as usize) * 4;
                img.px[d..d + 4].copy_from_slice(&c);
            }
        }
    }
}
