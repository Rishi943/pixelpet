//! Reading a JPEG or PNG into an upright, small `Photo`.

use std::path::Path;

use super::Photo;

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    /// Not a JPEG or PNG, or one that can't be decoded.
    NotAPhoto,
    /// The file or the picture is too big to open.
    TooBig,
}

/// Read and decode the file at `path`.
pub fn open(path: &Path) -> Result<Photo, LoadError> {
    let bytes = std::fs::read(path).map_err(LoadError::Io)?;
    decode(&bytes)
}

/// Decode a JPEG or PNG, told apart by its first bytes.
pub fn decode(_bytes: &[u8]) -> Result<Photo, LoadError> {
    Err(LoadError::NotAPhoto)
}
