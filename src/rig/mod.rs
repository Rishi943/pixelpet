//! The rig: turns a skeleton into pixels. Knows nothing about windows or time.
//! A Rust port of the approved drafts in `drafts/looks/`, proven identical to
//! them by `tests/parity.rs`.

pub mod core;
pub mod dog;
pub mod props;
pub mod rabbit;

pub use self::core::Look;
pub use self::props::Food;
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
    // play (spec §5); dog: run carry eat wag lean tug stretch,
    // rabbit: eat flop nudge tug stretch. Each species falls back to a
    // near pose for the others.
    Run,
    Carry,
    Eat,
    Wag,
    Lean,
    Tug,
    Stretch,
    Flop,
    Nudge,
    /// Looking up at the screen, as if watching it with you (both species).
    Watch,
    /// Dog, fetch: running into the screen (seen from behind), back out of
    /// it (seen from the front), and back out with the toy in his mouth.
    RunAway,
    RunBack,
    CarryBack,
}

impl Anim {
    pub const ALL: [Anim; 19] = [
        Anim::Stand,
        Anim::Walk,
        Anim::Sit,
        Anim::Tilt,
        Anim::Hop,
        Anim::Sleep,
        Anim::Run,
        Anim::Carry,
        Anim::Eat,
        Anim::Wag,
        Anim::Lean,
        Anim::Tug,
        Anim::Stretch,
        Anim::Flop,
        Anim::Nudge,
        Anim::Watch,
        Anim::RunAway,
        Anim::RunBack,
        Anim::CarryBack,
    ];

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
            Anim::Run => "run",
            Anim::Carry => "carry",
            Anim::Eat => "eat",
            Anim::Wag => "wag",
            Anim::Lean => "lean",
            Anim::Tug => "tug",
            Anim::Stretch => "stretch",
            Anim::Flop => "flop",
            Anim::Nudge => "nudge",
            Anim::Watch => "watch",
            Anim::RunAway => "run-away",
            Anim::RunBack => "run-back",
            Anim::CarryBack => "carry-back",
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

/// How much smaller something is at `depth` (0 = on the ground line).
pub fn depth_scale(depth: f64) -> f64 {
    1.0 / (1.0 + depth.max(0.0))
}

/// `look` redrawn at `scale`: the same style with `r` scaled (at least 6),
/// so a pet far away is drawn with fewer art pixels and stays crisp, rather
/// than an image shrunk. Built once per `r` and kept. Scale 1 is `look`.
pub fn look_scaled(look: &'static Look, scale: f64) -> &'static Look {
    use std::collections::HashMap;
    use std::sync::Mutex;
    let r = (look.r * scale).round().max(6.0);
    if r == look.r {
        return look;
    }
    static CACHE: OnceLock<Mutex<HashMap<(&'static str, u32), &'static Look>>> = OnceLock::new();
    let mut c = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    *c.entry((look.name, r as u32)).or_insert_with(|| {
        Box::leak(Box::new(Look {
            name: look.name,
            r,
            zoom: look.zoom,
            tones: look.tones,
            soft_outline: look.soft_outline,
            min_r: look.min_r,
            noise: look.noise,
            dither: look.dither,
            detail: look.detail,
        }))
    })
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
        Pet::Rabbit => Skel::Rabbit(rabbit::Skel { toy, ..rabbit::pose(anim, look, t) }),
    }
}

/// Blend two skeletons of the same species (panics if they differ).
pub fn lerp(a: &Skel, b: &Skel, k: f64) -> Skel {
    match (a, b) {
        (Skel::Dog(a), Skel::Dog(b)) => Skel::Dog(dog::lerp(a, b, k)),
        (Skel::Rabbit(a), Skel::Rabbit(b)) => Skel::Rabbit(rabbit::lerp(a, b, k)),
        _ => panic!("rig::lerp: skeletons of different species"),
    }
}

fn alex_palette() -> &'static Palette {
    static P: OnceLock<Palette> = OnceLock::new();
    P.get_or_init(|| Palette::compile(&dog::ALEX))
}

fn bun_palette() -> &'static Palette {
    static P: OnceLock<Palette> = OnceLock::new();
    P.get_or_init(|| Palette::compile(&rabbit::BUN))
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
        Skel::Rabbit(s) => {
            let b = rabbit::body(look);
            let list = rabbit::shapes(s, &b, look);
            let pal = bun_palette();
            let mut rs = raster(&list, look, fs, rabbit::paint);
            let mut out = to_rgba(&mut rs, look, fs, pal);
            rabbit::eyes(&mut out, s, &b, look, fs, pal);
            Frame { w: fs.w, h: fs.h, ox: fs.ox, gy: fs.gy, rgba: out }
        }
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

/// Plays once (for `period` seconds) rather than looping.
pub fn is_one_shot(anim: Anim) -> bool {
    anim == Anim::Stretch
}

/// The same skeleton with the eyes shut (being petted).
pub fn with_eyes_closed(s: Skel) -> Skel {
    match s {
        Skel::Dog(d) => Skel::Dog(dog::Skel { eyes: 0.0, ..d }),
        Skel::Rabbit(r) => Skel::Rabbit(rabbit::Skel { eyes: 0.0, ..r }),
    }
}

/// The mouth in frame pixels (the space of `Frame.ox`/`gy`, before zoom
/// and mirroring), where a rope or food meets the pet.
pub fn mouth(pet: Pet, s: &Skel, look: &Look) -> (f64, f64) {
    let fs = frame_size(look);
    let p = match (pet, s) {
        (Pet::Dog, Skel::Dog(d)) => dog::mouth(d, &dog::body(look)),
        (Pet::Rabbit, Skel::Rabbit(r)) => rabbit::mouth(r, &rabbit::body(look)),
        _ => panic!("skeleton of the other species"),
    };
    (fs.ox as f64 + p[0] * look.r, fs.gy as f64 - p[1] * look.r)
}

/// Loop length of an anim, in seconds.
pub fn period(pet: Pet, anim: Anim) -> f64 {
    match pet {
        Pet::Dog => dog::period(anim),
        Pet::Rabbit => rabbit::period(anim),
    }
}

/// Rig units per second while this anim plays; 0 when it doesn't travel.
pub fn speed(pet: Pet, look: &Look, anim: Anim) -> f64 {
    match (pet, anim) {
        (Pet::Dog, Anim::Walk) => dog::walk_speed(&dog::body(look)),
        (Pet::Dog, Anim::Run | Anim::RunAway | Anim::RunBack) => dog::RUN_SPEEDUP * dog::walk_speed(&dog::body(look)),
        (Pet::Dog, Anim::Carry | Anim::CarryBack) => dog::CARRY_SPEEDUP * dog::walk_speed(&dog::body(look)),
        (Pet::Rabbit, Anim::Walk | Anim::Run | Anim::Carry | Anim::RunAway | Anim::RunBack | Anim::CarryBack) => rabbit::WALK_SPEED,
        (Pet::Rabbit, Anim::Nudge) => 0.6 * rabbit::WALK_SPEED,
        _ => 0.0,
    }
}

/// Shapes the speed over one loop (`phase` in [0, 1)); 1.0 for constant speed.
pub fn move_profile(pet: Pet, anim: Anim, phase: f64) -> f64 {
    match (pet, anim) {
        (Pet::Rabbit, Anim::Walk | Anim::Run | Anim::Carry | Anim::Nudge | Anim::RunAway | Anim::RunBack | Anim::CarryBack) => rabbit::walk_move(phase),
        _ => 1.0,
    }
}
