//! From points clicked on photos to the maker's sliders and coat colours.

use super::Photo;
use crate::rig::{Design, Pet, Shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Standing,
    Sitting,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    Nose,
    Eye,
    EarTip,
    Shoulders,
    TailBase,
    TailTip,
    Chest,
    FrontPaw,
    BackPaw,
}

/// The points the maker asks for, in order.
pub fn points(pet: Pet) -> &'static [Point] {
    use Point::*;
    match pet {
        Pet::Dog => &[Nose, Eye, EarTip, Shoulders, TailBase, TailTip, Chest, FrontPaw, BackPaw],
        Pet::Rabbit => &[Nose, Eye, EarTip, Shoulders, TailBase, Chest, FrontPaw, BackPaw],
    }
}

/// The maker's prompt line for a point.
pub fn prompt(pet: Pet, p: Point) -> &'static str {
    use Point::*;
    match (pet, p) {
        (_, Nose) => "Click the nose",
        (_, Eye) => "Click an eye",
        (_, EarTip) => "Click the tip of an ear",
        (Pet::Dog, Shoulders) => "Click the top of the shoulders",
        (Pet::Rabbit, Shoulders) => "Click the top of the back",
        (Pet::Dog, TailBase) => "Click where the tail starts",
        (Pet::Rabbit, TailBase) => "Click the tail",
        (_, TailTip) => "Click the tip of the tail",
        (_, Chest) => "Click the chest",
        (Pet::Dog, FrontPaw) => "Click a front paw",
        (Pet::Rabbit, FrontPaw) => "Click a front foot",
        (Pet::Dog, BackPaw) => "Click a back paw",
        (Pet::Rabbit, BackPaw) => "Click a back foot",
    }
}

/// One photo with its pose and the points clicked on it, in photo pixels.
/// Skipped points are simply missing.
pub struct Marked<'a> {
    pub photo: &'a Photo,
    pub pose: Pose,
    pub at: Vec<(Point, (f32, f32))>,
}

/// What the photos gave: a whole shape and coat for the design, plus the
/// names of the sliders the photos actually set (the rest are the base's).
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub shape: Shape,
    pub coat: Vec<(i8, u32)>,
    pub set: Vec<&'static str>,
}

/// Work out the sliders and colours from the marked photos, starting from
/// `base`.
pub fn fit(base: &Design, _photos: &[Marked]) -> Fit {
    Fit { shape: base.shape, coat: base.coat.clone(), set: Vec::new() }
}
