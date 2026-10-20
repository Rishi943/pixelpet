//! The only place with OS-specific calls, behind one trait, so `brain` tests
//! can use a fake. All coordinates are physical pixels.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// The y just below the rect: the ground line for a work area.
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

pub trait Platform {
    /// Global cursor position, if the OS tells us.
    fn cursor_pos(&self) -> Option<(i32, i32)>;
    /// Move the real cursor; false where that isn't possible.
    fn warp_cursor(&self, x: i32, y: i32) -> bool;
    /// The primary monitor's work area (screen minus taskbar).
    fn work_area(&self) -> Rect;
    /// Seconds since the last mouse or keyboard input.
    fn idle_seconds(&self) -> Option<f64>;
    /// Whether a fullscreen app is in front; None when unknown.
    fn fullscreen_app(&self) -> Option<bool>;
    /// The program the launch-on-startup entry points at, if any.
    fn autostart(&self) -> Option<PathBuf>;
    /// Point the launch-on-startup entry at `target`, or remove it with None.
    fn set_autostart(&self, target: Option<&Path>) -> std::io::Result<()>;
}

/// No OS support: no cursor, no idle, no fullscreen, a 1920x1040 work area.
pub struct Stub;

impl Platform for Stub {
    fn cursor_pos(&self) -> Option<(i32, i32)> {
        None
    }
    fn warp_cursor(&self, _x: i32, _y: i32) -> bool {
        false
    }
    fn work_area(&self) -> Rect {
        Rect { x: 0, y: 0, w: 1920, h: 1040 }
    }
    fn idle_seconds(&self) -> Option<f64> {
        None
    }
    fn fullscreen_app(&self) -> Option<bool> {
        None
    }
    fn autostart(&self) -> Option<PathBuf> {
        None
    }
    fn set_autostart(&self, _target: Option<&Path>) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "launch on startup is not supported here"))
    }
}

/// A platform for tests: each method returns its field; warps and autostart
/// changes are recorded.
pub struct Fake {
    pub cursor: RefCell<Option<(i32, i32)>>,
    pub warps: RefCell<Vec<(i32, i32)>>,
    pub can_warp: RefCell<bool>,
    pub work: RefCell<Rect>,
    pub idle: RefCell<Option<f64>>,
    pub fullscreen: RefCell<Option<bool>>,
    pub autostart: RefCell<Option<PathBuf>>,
}

impl Default for Fake {
    fn default() -> Fake {
        Fake {
            cursor: RefCell::new(None),
            warps: RefCell::new(Vec::new()),
            can_warp: RefCell::new(true),
            work: RefCell::new(Rect { x: 0, y: 0, w: 1920, h: 1040 }),
            idle: RefCell::new(Some(0.0)),
            fullscreen: RefCell::new(Some(false)),
            autostart: RefCell::new(None),
        }
    }
}

impl Platform for Fake {
    fn cursor_pos(&self) -> Option<(i32, i32)> {
        *self.cursor.borrow()
    }
    fn warp_cursor(&self, x: i32, y: i32) -> bool {
        if *self.can_warp.borrow() {
            self.warps.borrow_mut().push((x, y));
            *self.cursor.borrow_mut() = Some((x, y));
            true
        } else {
            false
        }
    }
    fn work_area(&self) -> Rect {
        *self.work.borrow()
    }
    fn idle_seconds(&self) -> Option<f64> {
        *self.idle.borrow()
    }
    fn fullscreen_app(&self) -> Option<bool> {
        *self.fullscreen.borrow()
    }
    fn autostart(&self) -> Option<PathBuf> {
        self.autostart.borrow().clone()
    }
    fn set_autostart(&self, target: Option<&Path>) -> std::io::Result<()> {
        *self.autostart.borrow_mut() = target.map(Path::to_path_buf);
        Ok(())
    }
}

#[cfg(windows)]
mod windows;

/// The real platform for this OS (`Stub` where M1 has none).
pub fn native() -> Box<dyn Platform> {
    #[cfg(windows)]
    {
        Box::new(windows::Win)
    }
    #[cfg(not(windows))]
    {
        Box::new(Stub)
    }
}
