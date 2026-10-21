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

/// The ground with an auto-hide taskbar in view. Windows leaves an
/// auto-hide taskbar inside the work area, so when it slides up it would
/// cover the pet; the pet rides on its top edge instead. A hidden one (a few
/// px showing), a normal one (already outside the work area), or one docked
/// at the top or side changes nothing.
pub fn ground_with_taskbar(work: Rect, bar: Option<Rect>) -> Rect {
    let Some(bar) = bar else { return work };
    let spans = bar.x < work.x + work.w && bar.x + bar.w > work.x && bar.w * 2 >= work.w;
    let at_bottom = bar.y > work.y + work.h / 2;
    let showing = work.bottom() - bar.y;
    if spans && at_bottom && showing > 4 && bar.y < work.bottom() {
        Rect { h: bar.y - work.y, ..work }
    } else {
        work
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
    /// Whether Esc is held down right now. The app asks only while play is
    /// going on (Esc ends it), and reads no other key.
    fn esc_down(&self) -> bool;
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
    fn esc_down(&self) -> bool {
        false
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
    pub esc: RefCell<bool>,
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
            esc: RefCell::new(false),
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
    fn esc_down(&self) -> bool {
        *self.esc.borrow()
    }
}

pub mod lock;
pub mod pure;

#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

/// Held for as long as this is the one running PixelPet.
pub struct InstanceGuard {
    #[cfg(windows)]
    _inner: windows::Mutex,
    #[cfg(not(windows))]
    _inner: lock::Lock,
}

/// Claim the single-instance lock `name`. None means another PixelPet
/// already holds it. On Windows it is a named mutex; elsewhere a file lock
/// next to the settings file.
pub fn claim_single_instance(name: &str) -> Option<InstanceGuard> {
    #[cfg(windows)]
    {
        windows::Mutex::claim(name).map(|m| InstanceGuard { _inner: m })
    }
    #[cfg(not(windows))]
    {
        let exe = std::env::current_exe().unwrap_or_default();
        let settings = crate::settings::settings_path(crate::settings::current_os(), &exe, &|k| std::env::var(k).ok());
        let path = settings.with_file_name(format!("{}.lock", name.to_lowercase()));
        lock::Lock::claim(&path).map(|l| InstanceGuard { _inner: l })
    }
}

/// The real platform for this OS (`Stub` where there is none, or no X server).
pub fn native() -> Box<dyn Platform> {
    #[cfg(windows)]
    {
        Box::new(windows::Win)
    }
    #[cfg(target_os = "linux")]
    {
        match linux::X11::open() {
            Some(x) => Box::new(x),
            None => Box::new(Stub),
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Box::new(Stub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect { x: 0, y: 0, w: 2560, h: 1440 };

    #[test]
    fn shown_autohide_taskbar_lifts_the_ground() {
        let bar = Rect { x: 0, y: 1380, w: 2560, h: 60 };
        assert_eq!(ground_with_taskbar(WORK, Some(bar)).bottom(), 1380);
    }

    #[test]
    fn hidden_autohide_taskbar_leaves_the_ground() {
        // hidden: only a 2 px sliver is on screen
        let bar = Rect { x: 0, y: 1438, w: 2560, h: 60 };
        assert_eq!(ground_with_taskbar(WORK, Some(bar)), WORK);
        // half-way through sliding up: the ground follows it
        let bar = Rect { x: 0, y: 1410, w: 2560, h: 60 };
        assert_eq!(ground_with_taskbar(WORK, Some(bar)).bottom(), 1410);
    }

    #[test]
    fn other_taskbars_leave_the_ground() {
        // a normal taskbar is already outside the work area
        let work = Rect { x: 0, y: 0, w: 2560, h: 1380 };
        assert_eq!(ground_with_taskbar(work, Some(Rect { x: 0, y: 1380, w: 2560, h: 60 })), work);
        // docked at the top or the side
        assert_eq!(ground_with_taskbar(WORK, Some(Rect { x: 0, y: 0, w: 2560, h: 60 })), WORK);
        assert_eq!(ground_with_taskbar(WORK, Some(Rect { x: 0, y: 0, w: 60, h: 1440 })), WORK);
        // none found, or on another monitor
        assert_eq!(ground_with_taskbar(WORK, None), WORK);
        assert_eq!(ground_with_taskbar(WORK, Some(Rect { x: 2560, y: 1380, w: 1920, h: 60 })), WORK);
    }
}
