//! A pet as data: its species, shape, coat colours and collar. The two
//! built-in designs are Alex and Alex the bun exactly as drawn in v1.

use super::core::{fam, Coat, Palette};
use super::Pet;

/// Multipliers on the look's body (1 = the built-in pet's proportions). The
/// dog uses legs..fluff, the rabbit head, ears, fluff and feet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub legs: f64,
    pub body: f64,
    pub head: f64,
    pub ears: f64,
    pub muzzle: f64,
    pub tail: f64,
    pub fluff: f64,
    pub feet: f64,
}

impl Default for Shape {
    fn default() -> Shape {
        Shape { legs: 1.0, body: 1.0, head: 1.0, ears: 1.0, muzzle: 1.0, tail: 1.0, fluff: 1.0, feet: 1.0 }
    }
}

impl Shape {
    /// A field by its name (`legs`, `body`, ...); None for an unknown name.
    pub fn get(&self, field: &str) -> Option<f64> {
        Some(match field {
            "legs" => self.legs,
            "body" => self.body,
            "head" => self.head,
            "ears" => self.ears,
            "muzzle" => self.muzzle,
            "tail" => self.tail,
            "fluff" => self.fluff,
            "feet" => self.feet,
            _ => return None,
        })
    }

    /// Set a field by its name; false for an unknown name.
    pub fn set(&mut self, field: &str, v: f64) -> bool {
        let slot = match field {
            "legs" => &mut self.legs,
            "body" => &mut self.body,
            "head" => &mut self.head,
            "ears" => &mut self.ears,
            "muzzle" => &mut self.muzzle,
            "tail" => &mut self.tail,
            "fluff" => &mut self.fluff,
            "feet" => &mut self.feet,
            _ => return false,
        };
        *slot = v;
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    pub pet: Pet,
    pub shape: Shape,
    /// (colour family from `core::fam`, base colour 0xRRGGBB) over the
    /// species coat; its other shades are worked out by `shades`.
    pub coat: Vec<(i8, u32)>,
    /// None: no collar drawn.
    pub collar: Option<u32>,
}

impl Design {
    pub fn alex() -> Design {
        Design { pet: Pet::Dog, shape: Shape::default(), coat: vec![], collar: None }
    }

    pub fn bun() -> Design {
        Design { pet: Pet::Rabbit, shape: Shape::default(), coat: vec![], collar: None }
    }

    pub fn builtin(pet: Pet) -> Design {
        match pet {
            Pet::Dog => Design::alex(),
            Pet::Rabbit => Design::bun(),
        }
    }

    fn species_coat(&self) -> &'static Coat {
        match self.pet {
            Pet::Dog => &super::dog::ALEX,
            Pet::Rabbit => &super::rabbit::BUN,
        }
    }

    /// The species coat with this design's colours on top.
    pub fn palette(&self) -> Palette {
        let mut p = Palette::compile(self.species_coat());
        let set = |p: &mut Palette, f: i8, base: u32| {
            p.fam[f as usize] = shades(base).map(|c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]);
        };
        for &(f, base) in &self.coat {
            set(&mut p, f, base);
        }
        if let Some(c) = self.collar {
            set(&mut p, fam::COLLAR, c);
        }
        p
    }
}

/// One picked colour as the four shades of a family `[deep, shadow, base,
/// light]`: darker drifts warmer and richer, lighter cooler and paler, like
/// the hand-tuned coats. The mockup's `family()` (drafts/maker), exactly.
pub fn shades(base: u32) -> [u32; 4] {
    let rgb = [((base >> 16) & 255) as f64 / 255.0, ((base >> 8) & 255) as f64 / 255.0, (base & 255) as f64 / 255.0];
    let (h, s, l) = rgb_to_hsl(rgb);
    let tones = [
        (h - 6.0, (s * 1.1).min(1.0), l * 0.42),
        (h - 3.0, (s * 1.05).min(1.0), l * 0.7),
        (h, s, l),
        (h + 4.0, s * 0.92, (l * 1.25 + 0.05).min(0.97)),
    ];
    tones.map(|(h, s, l)| to_u32(hsl_to_rgb(((h + 360.0) % 360.0, s, l))))
}

fn rgb_to_hsl([r, g, b]: [f64; 3]) -> (f64, f64, f64) {
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let (l, d) = ((mx + mn) / 2.0, mx - mn);
    if d == 0.0 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    // `%` keeps the sign, as in JavaScript
    let h = if mx == r {
        ((g - b) / d) % 6.0
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    ((h * 60.0 + 360.0) % 360.0, s, l)
}

fn hsl_to_rgb((h, s, l): (f64, f64, f64)) -> [f64; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    [r + m, g + m, b + m]
}

fn to_u32(c: [f64; 3]) -> u32 {
    // JavaScript's Math.round: halves go up
    let ch = |v: f64| ((v.clamp(0.0, 1.0) * 255.0) + 0.5).floor() as u32;
    (ch(c[0]) << 16) | (ch(c[1]) << 8) | ch(c[2])
}
