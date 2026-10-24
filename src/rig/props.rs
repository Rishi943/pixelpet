//! Props for play: food, hearts and crumbs. Food and hearts
//! are hand-drawn pixel maps in the pets' style (1 px dark outline, a few
//! tones); one char per art pixel. Frames have their ground point at the
//! bottom centre, like the pet's.

use super::{Frame, Pet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Food {
    Chicken,
    Bone,
    Kibble,
    Carrot,
    Coriander,
    Banana,
    Fish,
}

impl Food {
    /// The three foods a pet can be given, default first (spec §5 Feed).
    pub fn for_pet(pet: Pet) -> [Food; 3] {
        match pet {
            Pet::Dog => [Food::Chicken, Food::Bone, Food::Kibble],
            Pet::Rabbit => [Food::Carrot, Food::Coriander, Food::Banana],
            Pet::Cat => [Food::Fish, Food::Chicken, Food::Kibble],
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
            Food::Fish => "Fish",
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
        // lying on its side, head to the left, tail fin to the right
        Food::Fish => Art {
            rows: &[
                "..ooooooo..oo.",
                ".o2223333o.o2o",
                "oe22222233oo2o",
                "o1222222223o2o",
                ".o1111112o.o1o",
                "..ooooooo..oo.",
            ],
            pal: &[('1', 0x4A6A8A), ('2', 0x7FA6C8), ('3', 0xC8DCEC), ('e', 0x101014)],
            crumb: 0x7FA6C8,
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
