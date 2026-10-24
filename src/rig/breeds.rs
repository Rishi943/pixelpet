//! The pet maker's starting points and limits: breeds, slider ranges, the
//! colour areas and swatches. First guesses from the approved mockup
//! (drafts/maker), to be tuned by eye in the real app.

use super::core::fam;
use super::design::{Design, Shape};
use super::Pet;

pub struct Breed {
    pub name: &'static str,
    pub design: fn() -> Design,
}

pub struct Slider {
    /// A `Shape` field name (`Shape::get`/`set`).
    pub field: &'static str,
    pub label: &'static str,
    pub lo: f64,
    pub hi: f64,
}

fn dog(shape: Shape, coat: [u32; 4]) -> Design {
    let [tan, black, cream, mask] = coat;
    Design {
        pet: Pet::Dog,
        shape,
        coat: vec![(fam::TAN, tan), (fam::BLACK, black), (fam::CREAM, cream), (fam::MASK, mask)],
        collar: None,
    }
}

fn rabbit(coat: [u32; 3]) -> Design {
    let [white, brown, pink] = coat;
    Design {
        pet: Pet::Rabbit,
        shape: Shape::default(),
        coat: vec![(fam::WHITE, white), (fam::BROWN, brown), (fam::PINK, pink)],
        collar: None,
    }
}

const LAB: Shape = Shape { ears: 0.78, fluff: 0.96, muzzle: 1.05, ..ONE };
const ONE: Shape = Shape { legs: 1.0, body: 1.0, head: 1.0, ears: 1.0, muzzle: 1.0, tail: 1.0, fluff: 1.0, feet: 1.0 };

static DOGS: [Breed; 6] = [
    Breed { name: "Alex the GSD", design: Design::alex },
    Breed { name: "Yellow Lab", design: || dog(LAB, [0xD9B26F, 0xC9A060, 0xE8CC92, 0xC9A060]) },
    Breed { name: "Black Lab", design: || dog(LAB, [0x2A2424, 0x231E1E, 0x2F2828, 0x231E1E]) },
    Breed {
        name: "Husky",
        design: || dog(Shape { ears: 0.95, fluff: 1.2, tail: 1.1, ..ONE }, [0xE8E6E2, 0x5A6068, 0xF3F1EC, 0x6A7078]),
    },
    Breed {
        name: "Golden",
        design: || dog(Shape { ears: 0.75, fluff: 1.3, tail: 1.25, ..ONE }, [0xC98A3A, 0xB77A32, 0xDDA65A, 0xB77A32]),
    },
    Breed {
        name: "Corgi",
        design: || dog(Shape { legs: 0.62, body: 1.15, ears: 1.35, tail: 0.45, ..ONE }, [0xC8703A, 0xB9622F, 0xF2E6D6, 0xC8703A]),
    },
];

static RABBITS: [Breed; 3] = [
    Breed { name: "Alex the bun", design: Design::bun },
    Breed { name: "Grey", design: || rabbit([0x9AA0A8, 0x6A7078, 0xC9A7A0]) },
    Breed { name: "Black and white", design: || rabbit([0xF3F0EB, 0x2A2424, 0xD6A799]) },
];

fn cat(coat: [u32; 2]) -> Design {
    let [main, chest] = coat;
    Design { pet: Pet::Cat, shape: Shape::default(), coat: vec![(fam::TAN, main), (fam::CREAM, chest)], collar: None }
}

static CATS: [Breed; 4] = [
    Breed { name: "Mellow", design: Design::mellow },
    Breed { name: "Oreo", design: Design::oreo },
    Breed { name: "Ginger", design: || cat([0xD2783A, 0xF0C89A]) },
    Breed { name: "Grey", design: || cat([0x8A9098, 0xD8DCE0]) },
];

pub fn breeds(pet: Pet) -> &'static [Breed] {
    match pet {
        Pet::Dog => &DOGS,
        Pet::Rabbit => &RABBITS,
        Pet::Cat => &CATS,
    }
}

static DOG_SLIDERS: [Slider; 7] = [
    Slider { field: "legs", label: "Legs", lo: 0.6, hi: 1.3 },
    Slider { field: "body", label: "Body", lo: 0.75, hi: 1.25 },
    Slider { field: "head", label: "Head", lo: 0.9, hi: 1.5 },
    Slider { field: "ears", label: "Ears", lo: 0.6, hi: 1.5 },
    Slider { field: "muzzle", label: "Muzzle", lo: 0.7, hi: 1.3 },
    Slider { field: "tail", label: "Tail", lo: 0.4, hi: 1.5 },
    Slider { field: "fluff", label: "Fluff", lo: 0.9, hi: 1.35 },
];

static RABBIT_SLIDERS: [Slider; 4] = [
    Slider { field: "head", label: "Head", lo: 0.85, hi: 1.3 },
    Slider { field: "ears", label: "Ears", lo: 0.7, hi: 1.4 },
    Slider { field: "fluff", label: "Fluff", lo: 0.9, hi: 1.3 },
    Slider { field: "feet", label: "Feet", lo: 0.8, hi: 1.3 },
];

static CAT_SLIDERS: [Slider; 6] = [
    Slider { field: "legs", label: "Legs", lo: 0.75, hi: 1.25 },
    Slider { field: "body", label: "Body", lo: 0.8, hi: 1.25 },
    Slider { field: "head", label: "Head", lo: 0.9, hi: 1.3 },
    Slider { field: "ears", label: "Ears", lo: 0.7, hi: 1.4 },
    Slider { field: "tail", label: "Tail", lo: 0.6, hi: 1.3 },
    Slider { field: "fluff", label: "Fluff", lo: 0.85, hi: 1.3 },
];

pub fn sliders(pet: Pet) -> &'static [Slider] {
    match pet {
        Pet::Dog => &DOG_SLIDERS,
        Pet::Rabbit => &RABBIT_SLIDERS,
        Pet::Cat => &CAT_SLIDERS,
    }
}

/// The colour areas a person can set, as (family, label). The collar is
/// its own row (dogs and cats).
pub fn areas(pet: Pet) -> &'static [(i8, &'static str)] {
    match pet {
        Pet::Dog => &[(fam::TAN, "Coat"), (fam::BLACK, "Saddle"), (fam::CREAM, "Legs"), (fam::MASK, "Mask")],
        Pet::Rabbit => &[(fam::WHITE, "Coat"), (fam::BROWN, "Patches"), (fam::PINK, "Ears")],
        Pet::Cat => &[(fam::TAN, "Coat"), (fam::CREAM, "Chest")],
    }
}

pub const SWATCHES: [u32; 8] = [0xC47637, 0xE2B077, 0xD9C27A, 0xF3F0EB, 0x9AA0A8, 0x5A6068, 0x231B1B, 0x6E3B22];
pub const COLLARS: [u32; 8] = [0x2F64C8, 0xC8384A, 0x3E9A57, 0xE0A030, 0x7A4CC0, 0x231B1B, 0xF3F0EB, 0xD86AA0];
