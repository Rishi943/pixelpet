//! The pet's on-screen window: a borderless, always-on-top, see-through
//! window that shows one RGBA image at a physical-pixel position.

/// Straight-alpha RGBA, row-major, `w * h * 4` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Rgba {
    pub fn new(w: usize, h: usize) -> Rgba {
        Rgba { w, h, px: vec![0; w * h * 4] }
    }
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::PetWindow;

#[cfg(not(windows))]
mod unsupported {
    use super::Rgba;
    use winit::event_loop::ActiveEventLoop;
    use winit::window::WindowId;

    pub struct PetWindow {
        _never: (),
    }

    impl PetWindow {
        pub fn new(_el: &ActiveEventLoop, _focusable: bool) -> Result<PetWindow, String> {
            Err("unsupported in M1".into())
        }
        pub fn present(&mut self, _img: &Rgba, _x: i32, _y: i32, _opacity: u8) -> Result<(), String> {
            Err("unsupported in M1".into())
        }
        pub fn id(&self) -> WindowId {
            unreachable!("PetWindow cannot be built on this OS in M1")
        }
        pub fn hide(&self) {}
        pub fn show(&self) {}
    }
}
#[cfg(not(windows))]
pub use self::unsupported::PetWindow;
