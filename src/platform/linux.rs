//! Linux, on X11 (or Wayland through XWayland), over one x11rb connection.
//! Every query answers None/false rather than failing if the server lacks
//! what it needs.

use super::pure::{autostart_desktop, autostart_target, exec_from_desktop, wayland_session};
use super::{Platform, Rect};
use std::path::{Path, PathBuf};
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::screensaver::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window};
use x11rb::xcb_ffi::XCBConnection;

const XK_ESCAPE: u32 = 0xff1b;
/// Taken off the bottom of the monitor when the desktop gives no work area.
const NO_WORKAREA_MARGIN: i32 = 48;

pub struct X11 {
    conn: XCBConnection,
    root: Window,
    esc: Option<u8>,
    a_workarea: u32,
    a_active: u32,
    a_state: u32,
    a_fullscreen: u32,
    /// `~/.config/autostart/pixelpet.desktop`, read from the environment once
    autostart_file: Option<PathBuf>,
    /// a Wayland session seen through XWayland: no idle, warp or fullscreen
    wayland: bool,
}

impl X11 {
    pub fn open() -> Option<X11> {
        X11::connect().map_err(|e| eprintln!("PixelPet: no X server: {e}")).ok()
    }

    pub fn connect() -> Result<X11, String> {
        let (conn, screen) = XCBConnection::connect(None).map_err(|e| e.to_string())?;
        let root = conn.setup().roots.get(screen).ok_or("no such screen")?.root;
        let atom = |n: &str| -> Result<u32, String> {
            Ok(conn.intern_atom(false, n.as_bytes()).map_err(|e| e.to_string())?.reply().map_err(|e| format!("{n}: {e}"))?.atom)
        };
        let (a_workarea, a_active, a_state, a_fullscreen) = (
            atom("_NET_WORKAREA")?,
            atom("_NET_ACTIVE_WINDOW")?,
            atom("_NET_WM_STATE")?,
            atom("_NET_WM_STATE_FULLSCREEN")?,
        );
        let esc = escape_keycode(&conn);
        Ok(X11 { conn, root, esc, a_workarea, a_active, a_state, a_fullscreen, autostart_file: autostart_file(), wayland: wayland_session(&|k| std::env::var(k).ok()) })
    }

    fn cardinals(&self, win: Window, prop: u32, kind: AtomEnum) -> Option<Vec<u32>> {
        let r = self.conn.get_property(false, win, prop, kind, 0, 1024).ok()?.reply().ok()?;
        let v: Vec<u32> = r.value32()?.collect();
        Some(v)
    }

    fn primary_monitor(&self) -> Option<Rect> {
        let r = self.conn.randr_get_monitors(self.root, true).ok()?.reply().ok()?;
        let m = r.monitors.iter().find(|m| m.primary).or_else(|| r.monitors.first())?;
        Some(Rect { x: m.x as i32, y: m.y as i32, w: m.width as i32, h: m.height as i32 })
    }
}

fn escape_keycode(conn: &XCBConnection) -> Option<u8> {
    let (min, max) = (conn.setup().min_keycode, conn.setup().max_keycode);
    let map = conn.get_keyboard_mapping(min, max - min + 1).ok()?.reply().ok()?;
    let per = map.keysyms_per_keycode.max(1) as usize;
    let i = map.keysyms.chunks(per).position(|syms| syms.contains(&XK_ESCAPE))?;
    Some(min + i as u8)
}

fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = ((a.x + a.w).min(b.x + b.w), (a.y + a.h).min(b.y + b.h));
    (x1 > x0 && y1 > y0).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

fn autostart_file() -> Option<PathBuf> {
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let dir = var("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| var("HOME").map(|h| Path::new(&h).join(".config")))?;
    Some(dir.join("autostart").join("pixelpet.desktop"))
}

impl Platform for X11 {
    fn cursor_pos(&self) -> Option<(i32, i32)> {
        let r = self.conn.query_pointer(self.root).ok()?.reply().ok()?;
        r.same_screen.then_some((r.root_x as i32, r.root_y as i32))
    }

    fn warp_cursor(&self, x: i32, y: i32) -> bool {
        if self.wayland {
            return false; // XWayland moves only its own idea of the pointer
        }
        let ok = self.conn.warp_pointer(x11rb::NONE, self.root, 0, 0, 0, 0, x as i16, y as i16).is_ok();
        ok && self.conn.flush().is_ok()
    }

    fn work_area(&self) -> Rect {
        let screen = &self.conn.setup().roots[0];
        let whole = Rect { x: 0, y: 0, w: screen.width_in_pixels as i32, h: screen.height_in_pixels as i32 };
        let mon = self.primary_monitor().unwrap_or(whole);
        // desktop 0's work area, clipped to the primary monitor
        let wa = self.cardinals(self.root, self.a_workarea, AtomEnum::CARDINAL).filter(|v| v.len() >= 4);
        match wa.and_then(|v| intersect(mon, Rect { x: v[0] as i32, y: v[1] as i32, w: v[2] as i32, h: v[3] as i32 })) {
            Some(r) => r,
            None => Rect { h: mon.h - NO_WORKAREA_MARGIN, ..mon },
        }
    }

    fn idle_seconds(&self) -> Option<f64> {
        if self.wayland {
            return None; // XWayland sees input to X clients only
        }
        let r = self.conn.screensaver_query_info(self.root).ok()?.reply().ok()?;
        Some(r.ms_since_user_input as f64 / 1000.0)
    }

    fn fullscreen_app(&self) -> Option<bool> {
        if self.wayland {
            return None; // _NET_ACTIVE_WINDOW knows X windows only
        }
        let active = *self.cardinals(self.root, self.a_active, AtomEnum::WINDOW)?.first()?;
        if active == 0 {
            return None;
        }
        let state = self.cardinals(active, self.a_state, AtomEnum::ATOM).unwrap_or_default();
        Some(state.contains(&self.a_fullscreen))
    }

    fn autostart(&self) -> Option<PathBuf> {
        exec_from_desktop(&std::fs::read_to_string(self.autostart_file.as_ref()?).ok()?)
    }

    fn set_autostart(&self, target: Option<&Path>) -> std::io::Result<()> {
        let file = self.autostart_file.clone().ok_or_else(|| std::io::Error::other("no home folder"))?;
        match target {
            Some(t) => {
                let t = autostart_target(t, &|k| std::env::var(k).ok()).map_err(std::io::Error::other)?;
                std::fs::create_dir_all(file.parent().unwrap())?;
                std::fs::write(&file, autostart_desktop(&t))
            }
            None => match std::fs::remove_file(&file) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                r => r,
            },
        }
    }

    fn esc_down(&self) -> bool {
        let (Some(kc), Some(r)) = (self.esc, self.conn.query_keymap().ok().and_then(|c| c.reply().ok())) else {
            return false;
        };
        r.keys[kc as usize / 8] & (1 << (kc % 8)) != 0
    }
}

#[cfg(test)]
mod tests {
    //! Run under a real X server (CI: xvfb-run).
    use super::*;

    /// The app opens X from one thread; tests open it from many at once,
    /// which fails now and then while libxcb is first loaded. One at a time.
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn x() -> X11 {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        X11::connect().expect("an X server (run under xvfb-run)")
    }

    #[test]
    fn work_area_is_inside_the_root() {
        let p = x();
        let s = &p.conn.setup().roots[0];
        let w = p.work_area();
        assert!(w.w > 0 && w.h > 0 && w.x >= 0 && w.y >= 0, "{w:?}");
        assert!(w.x + w.w <= s.width_in_pixels as i32 && w.bottom() <= s.height_in_pixels as i32, "{w:?}");
    }

    #[test]
    fn cursor_is_known_and_warp_moves_it() {
        let p = x();
        assert!(p.cursor_pos().is_some());
        assert!(p.warp_cursor(123, 45));
        assert_eq!(p.cursor_pos(), Some((123, 45)));
    }

    #[test]
    fn idle_is_known() {
        assert!(x().idle_seconds().is_some_and(|s| s >= 0.0));
    }

    #[test]
    fn esc_has_a_keycode_and_is_up() {
        let p = x();
        assert!(p.esc.is_some());
        assert!(!p.esc_down());
    }

    #[test]
    fn autostart_round_trip_in_a_temp_home() {
        // never set_var here: other test threads are inside libxcb's getenv
        let dir = std::env::temp_dir().join(format!("pixelpet-xdg-{}", std::process::id()));
        let mut p = x();
        p.autostart_file = Some(dir.join("autostart/pixelpet.desktop"));
        let exe = Path::new("/opt/pixel pet/PixelPet");
        p.set_autostart(Some(exe)).unwrap();
        assert_eq!(p.autostart().as_deref(), Some(exe));
        assert!(dir.join("autostart/pixelpet.desktop").is_file());
        p.set_autostart(None).unwrap();
        assert_eq!(p.autostart(), None);
        p.set_autostart(None).unwrap(); // removing twice is fine
        let _ = std::fs::remove_dir_all(dir);
    }
}
