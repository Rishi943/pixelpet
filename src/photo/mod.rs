//! Photos of a person's own pet, for the maker's Photos step: loading them
//! (`load`) and turning clicked points into sliders and colours (`measure`).
//! Nothing here touches a window, and no photo is ever written anywhere.

pub mod load;
pub mod measure;

/// The longest side a loaded photo is cut down to.
pub const MAX_SIDE: u32 = 1024;

/// An upright RGB photo (3 bytes a pixel, rows top to bottom), long side at
/// most `MAX_SIDE`.
#[derive(Clone, Debug, PartialEq)]
pub struct Photo {
    pub w: u32,
    pub h: u32,
    pub rgb: Vec<u8>,
}
