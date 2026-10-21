//! macOS, through AppKit (by message) and CoreGraphics. Nothing here may
//! need the Accessibility or Input Monitoring permission, so Esc is never
//! read (right-click still ends play).

use super::pure::{autostart_target, cg_point, launch_agent_plist, mac_point, mac_rect, program_from_plist};
use super::{Platform, Rect};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::display::CGDisplay;
use core_graphics::geometry::CGPoint;
use core_graphics::window::{copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{NSPoint, NSRect};
use std::path::{Path, PathBuf};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
}
const COMBINED_SESSION_STATE: i32 = 0;
const ANY_INPUT_EVENT: u32 = !0;

pub struct Mac {
    /// `~/Library/LaunchAgents/com.pixelpet.app.plist`, from `HOME` once
    agent: Option<PathBuf>,
}

impl Mac {
    pub fn new() -> Mac {
        Mac { agent: launch_agent() }
    }
}

/// The primary screen (the one with the menu bar): its frame, visible frame
/// (minus menu bar and Dock), and backing scale.
fn primary() -> Option<(NSRect, NSRect, f64)> {
    unsafe {
        let screens: *mut AnyObject = msg_send![class!(NSScreen), screens];
        if screens.is_null() {
            return None;
        }
        let n: usize = msg_send![screens, count];
        if n == 0 {
            return None;
        }
        let s: *mut AnyObject = msg_send![screens, objectAtIndex: 0usize];
        let frame: NSRect = msg_send![s, frame];
        let visible: NSRect = msg_send![s, visibleFrame];
        let scale: f64 = msg_send![s, backingScaleFactor];
        Some((frame, visible, scale))
    }
}

fn tuple(r: NSRect) -> (f64, f64, f64, f64) {
    (r.origin.x, r.origin.y, r.size.width, r.size.height)
}

fn number(d: &CFDictionary<CFString, CFType>, key: &'static str) -> Option<f64> {
    d.find(CFString::from_static_string(key))?.downcast::<CFNumber>()?.to_f64()
}

fn launch_agent() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok().filter(|h| !h.is_empty())?;
    Some(Path::new(&home).join("Library/LaunchAgents/com.pixelpet.app.plist"))
}

impl Platform for Mac {
    fn cursor_pos(&self) -> Option<(i32, i32)> {
        let (frame, _, scale) = primary()?;
        let p: NSPoint = unsafe { msg_send![class!(NSEvent), mouseLocation] };
        Some(mac_point((p.x, p.y), frame.size.height, scale))
    }

    fn warp_cursor(&self, x: i32, y: i32) -> bool {
        let Some((_, _, scale)) = primary() else { return false };
        let (px, py) = cg_point(x, y, scale);
        CGDisplay::warp_mouse_cursor_position(CGPoint::new(px, py)).is_ok()
    }

    fn work_area(&self) -> Rect {
        match primary() {
            Some((frame, visible, scale)) => mac_rect(tuple(visible), frame.size.height, scale),
            None => Rect { x: 0, y: 0, w: 1440, h: 860 },
        }
    }

    fn idle_seconds(&self) -> Option<f64> {
        let s = unsafe { CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, ANY_INPUT_EVENT) };
        (s.is_finite() && s >= 0.0).then_some(s)
    }

    /// A normal-layer window of another app covering the whole primary screen.
    fn fullscreen_app(&self) -> Option<bool> {
        let (frame, _, _) = primary()?;
        let list: CFArray = copy_window_info(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID)?;
        let me = std::process::id() as f64;
        let covers = list.get_all_values().into_iter().any(|p| {
            let d: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(p as _) };
            let Some(b) = d.find(CFString::from_static_string("kCGWindowBounds")).and_then(|b| b.downcast::<CFDictionary>()) else {
                return false;
            };
            let b: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(b.as_concrete_TypeRef()) };
            number(&d, "kCGWindowLayer") == Some(0.0)
                && number(&d, "kCGWindowOwnerPID") != Some(me)
                && number(&b, "X") == Some(0.0)
                && number(&b, "Y") == Some(0.0)
                && number(&b, "Width") == Some(frame.size.width)
                && number(&b, "Height") == Some(frame.size.height)
        });
        Some(covers)
    }

    fn autostart(&self) -> Option<PathBuf> {
        program_from_plist(&std::fs::read_to_string(self.agent.as_ref()?).ok()?)
    }

    fn set_autostart(&self, target: Option<&Path>) -> std::io::Result<()> {
        let file = self.agent.clone().ok_or_else(|| std::io::Error::other("no home folder"))?;
        match target {
            Some(t) => {
                let t = autostart_target(t, &|k| std::env::var(k).ok()).map_err(std::io::Error::other)?;
                std::fs::create_dir_all(file.parent().unwrap())?;
                std::fs::write(&file, launch_agent_plist(&t))
            }
            None => match std::fs::remove_file(&file) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                r => r,
            },
        }
    }

    /// Never: reading keys needs Input Monitoring, the prompt that looks
    /// like a key logger. Right-click ends play instead.
    fn esc_down(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    //! Run on a Mac with a logged-in session (CI: macos-latest).
    use super::*;

    #[test]
    fn work_area_is_inside_the_main_screen() {
        let (frame, _, scale) = primary().expect("a screen");
        let w = Mac::new().work_area();
        let (sw, sh) = ((frame.size.width * scale) as i32, (frame.size.height * scale) as i32);
        assert!(w.w > 0 && w.h > 0 && w.x >= 0 && w.y >= 0, "{w:?}");
        assert!(w.x + w.w <= sw && w.bottom() <= sh, "{w:?} on {sw}x{sh}");
        assert!(w.y > 0, "the menu bar is above the work area: {w:?}");
    }

    #[test]
    fn cursor_is_known_and_inside_the_screen() {
        let (x, y) = Mac::new().cursor_pos().expect("cursor");
        let (frame, _, scale) = primary().unwrap();
        assert!(x >= 0 && y >= 0 && x <= (frame.size.width * scale) as i32 && y <= (frame.size.height * scale) as i32);
    }

    #[test]
    fn idle_is_known() {
        assert!(Mac::new().idle_seconds().is_some());
    }

    #[test]
    fn fullscreen_is_known() {
        assert!(Mac::new().fullscreen_app().is_some());
    }

    #[test]
    fn esc_is_never_read() {
        assert!(!Mac::new().esc_down());
    }

    #[test]
    fn autostart_round_trip_in_a_temp_home() {
        // never set_var here: other test threads read the environment
        let home = std::env::temp_dir().join(format!("pixelpet-home-{}", std::process::id()));
        let mac = Mac { agent: Some(home.join("Library/LaunchAgents/com.pixelpet.app.plist")) };
        let exe = Path::new("/Applications/PixelPet.app/Contents/MacOS/PixelPet");
        mac.set_autostart(Some(exe)).unwrap();
        assert_eq!(mac.autostart().as_deref(), Some(exe));
        assert!(home.join("Library/LaunchAgents/com.pixelpet.app.plist").is_file());
        mac.set_autostart(None).unwrap();
        assert_eq!(mac.autostart(), None);
        let moved = Path::new("/private/var/folders/x/AppTranslocation/A/d/PixelPet.app/Contents/MacOS/PixelPet");
        assert!(mac.set_autostart(Some(moved)).is_err());
        let _ = std::fs::remove_dir_all(home);
    }
}
