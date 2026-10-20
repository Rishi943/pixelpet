//! The rig: turns a skeleton into pixels. Knows nothing about windows or time.
//! A Rust port of the approved drafts in `drafts/looks/`, proven identical to
//! them by `tests/parity.rs`.

pub mod core;
pub mod dog;
pub mod rabbit;

pub use self::core::Look;
use self::core::{frame_size, raster, to_rgba, toy_shape, Palette, ToyKind};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pet {
    Dog,
    Rabbit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Anim {
    Stand,
    Walk,
    Sit,
    Tilt,
    Hop,
    Sleep,
}

impl Anim {
    pub const ALL: [Anim; 6] = [Anim::Stand, Anim::Walk, Anim::Sit, Anim::Tilt, Anim::Hop, Anim::Sleep];

    pub fn from_name(s: &str) -> Option<Anim> {
        Anim::ALL.into_iter().find(|a| a.name() == s)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Anim::Stand => "stand",
            Anim::Walk => "walk",
            Anim::Sit => "sit",
            Anim::Tilt => "tilt",
            Anim::Hop => "hop",
            Anim::Sleep => "sleep",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Toy {
    Ball,
    Coconut,
    Football,
}

impl Toy {
    pub fn from_name(s: &str) -> Option<Toy> {
        match s {
            "ball" => Some(Toy::Ball),
            "coconut" => Some(Toy::Coconut),
            "football" => Some(Toy::Football),
            _ => None,
        }
    }

    pub(crate) fn kind(&self) -> ToyKind {
        match self {
            Toy::Ball => ToyKind::Ball,
            Toy::Coconut => ToyKind::Coconut,
            Toy::Football => ToyKind::Football,
        }
    }
}

/// `"chunky" | "classic" | "detailed"`; anything else gives classic.
pub fn look(name: &str) -> &'static Look {
    match name {
        "chunky" => &self::core::CHUNKY,
        "detailed" => &self::core::DETAILED,
        _ => &self::core::CLASSIC,
    }
}

/// One rendered frame: straight-alpha RGBA, with the ground point at
/// `(ox, gy)` in frame pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub ox: i32,
    pub gy: i32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Skel {
    Dog(dog::Skel),
    Rabbit(rabbit::Skel),
}

pub fn pose(pet: Pet, anim: Anim, look: &Look, t: f64, toy: Option<Toy>) -> Skel {
    match pet {
        Pet::Dog => Skel::Dog(dog::pose(anim, look, t, toy)),
        Pet::Rabbit => unimplemented!("rabbit rig lands in M1 Task 3"),
    }
}

/// Blend two skeletons of the same species (panics if they differ).
pub fn lerp(a: &Skel, b: &Skel, k: f64) -> Skel {
    match (a, b) {
        (Skel::Dog(a), Skel::Dog(b)) => Skel::Dog(dog::lerp(a, b, k)),
        _ => panic!("rig::lerp: skeletons of different species"),
    }
}

fn alex_palette() -> &'static Palette {
    static P: OnceLock<Palette> = OnceLock::new();
    P.get_or_init(|| Palette::compile(&dog::ALEX))
}

pub fn render(s: &Skel, look: &Look, collar: bool) -> Frame {
    let fs = frame_size(look);
    match s {
        Skel::Dog(s) => {
            let b = dog::body(look);
            let mut list = dog::shapes(s, &b, look);
            if collar {
                for sh in &mut list {
                    if sh.part == self::core::Part::Neck {
                        sh.collar = true;
                    }
                }
            }
            let pal = alex_palette();
            let mut rs = raster(&list, look, fs, dog::paint);
            let mut out = to_rgba(&mut rs, look, fs, pal);
            dog::eyes(&mut out, s, &b, look, fs, pal);
            Frame { w: fs.w, h: fs.h, ox: fs.ox, gy: fs.gy, rgba: out }
        }
        Skel::Rabbit(_) => unimplemented!("rabbit rig lands in M1 Task 3"),
    }
}

pub fn render_toy(toy: Toy, look: &Look) -> Frame {
    let w = (0.3 * look.r).ceil() as usize + 4;
    let h = (0.25 * look.r).ceil() as usize + 4;
    let fs = self::core::FrameSize { w, h, ox: (w / 2) as i32, gy: h as i32 - 2 };
    let rad = match toy {
        Toy::Coconut => 0.10,
        Toy::Football => 0.085,
        Toy::Ball => 0.072,
    };
    let s = toy_shape(toy.kind(), [0.0, self::core::js_max(rad, look.min_r / look.r) * 0.95]);
    let pal = alex_palette();
    let mut rs = raster(&[s], look, fs, dog::paint);
    let out = to_rgba(&mut rs, look, fs, pal);
    Frame { w, h, ox: fs.ox, gy: fs.gy, rgba: out }
}

/// Loop length of an anim, in seconds.
pub fn period(pet: Pet, anim: Anim) -> f64 {
    match pet {
        Pet::Dog => dog::period(anim),
        Pet::Rabbit => unimplemented!("rabbit rig lands in M1 Task 3"),
    }
}

/// Rig units per second while this anim plays; 0 when it doesn't travel.
pub fn speed(pet: Pet, look: &Look, anim: Anim) -> f64 {
    match (pet, anim) {
        (Pet::Dog, Anim::Walk) => dog::walk_speed(&dog::body(look)),
        _ => 0.0,
    }
}

/// Shapes the speed over one loop (`phase` in [0, 1)); 1.0 for constant speed.
pub fn move_profile(_pet: Pet, _anim: Anim, _phase: f64) -> f64 {
    1.0
}
