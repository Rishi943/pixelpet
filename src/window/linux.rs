//! Linux (X11, or Wayland through XWayland): a winit window on a 32-bit
//! visual, fed with `PutImage` over our own x11rb connection. Clicks pass
//! through transparent pixels because the window's input shape is cut from
//! the image's alpha, as the Windows layered window does by itself. With no
//! compositor running, the visible (bounding) shape is cut too, so the pet
//! shows with hard edges instead of on a black box.

use super::shape::shape_runs;
use super::Rgba;
use std::borrow::Cow;
use std::time::{Duration, Instant};
use winit::event_loop::ActiveEventLoop;
use winit::platform::x11::{WindowAttributesExtX11, WindowType};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::wrapper::ConnectionExt as _;
use x11rb::image::{BitsPerPixel, Image, ImageOrder, ScanlinePad};
use x11rb::protocol::shape::{self, ConnectionExt as _, SK, SO};
use x11rb::protocol::xproto::{
    AtomEnum, ChangeWindowAttributesAux, ClipOrdering, ConfigureWindowAux, ConnectionExt as _, CreateGCAux, Gcontext,
    Pixmap, PropMode, Rectangle,
};
use x11rb::xcb_ffi::XCBConnection;

/// How often to look again for a compositor starting or stopping.
const COMPOSITOR_RECHECK: Duration = Duration::from_secs(2);

pub struct PetWindow {
    win: Window,
    conn: XCBConnection,
    xid: u32,
    gc: Gcontext,
    depth: u8,
    cm_atom: u32,
    composited: bool,
    checked: Instant,
    buf: Vec<u8>,
    /// the window's background pixmap, so the server repaints it on expose
    pixmap: Option<(Pixmap, usize, usize)>,
    /// last (x, y, w, h) and opacity sent, to skip unchanged requests
    geom: Option<(i32, i32, usize, usize)>,
    opacity: Option<u8>,
    shown: bool,
    state: WmState,
}

/// `_NET_WM_STATE`: on top, no taskbar or pager entry. Window managers drop
/// it when a window is unmapped (EWMH), so it is written before every map.
pub(crate) struct WmState {
    prop: u32,
    atoms: [u32; 3],
}

impl WmState {
    pub(crate) fn intern(conn: &XCBConnection) -> Result<WmState, String> {
        Ok(WmState {
            prop: atom(conn, "_NET_WM_STATE")?,
            atoms: [
                atom(conn, "_NET_WM_STATE_ABOVE")?,
                atom(conn, "_NET_WM_STATE_SKIP_TASKBAR")?,
                atom(conn, "_NET_WM_STATE_SKIP_PAGER")?,
            ],
        })
    }

    pub(crate) fn write(&self, conn: &XCBConnection, xid: u32) -> Result<(), x11rb::errors::ConnectionError> {
        conn.change_property32(PropMode::REPLACE, xid, self.prop, AtomEnum::ATOM, &self.atoms)?;
        conn.flush()
    }
}

fn atom(conn: &XCBConnection, name: &str) -> Result<u32, String> {
    Ok(conn.intern_atom(false, name.as_bytes()).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?.atom)
}

impl PetWindow {
    pub fn new(el: &ActiveEventLoop, focusable: bool) -> Result<PetWindow, String> {
        let attrs = WindowAttributes::default()
            .with_title("PixelPet")
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_x11_window_type(vec![WindowType::Utility])
            .with_active(focusable)
            .with_visible(false);
        let win = el.create_window(attrs).map_err(|e| e.to_string())?;
        let xid = match win.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::Xlib(h) => h.window as u32,
            RawWindowHandle::Xcb(h) => h.window.get(),
            _ => return Err("not an X11 window".into()),
        };
        let (conn, screen) = crate::platform::connect_x()?;
        conn.extension_information(shape::X11_EXTENSION_NAME)
            .map_err(|e| e.to_string())?
            .ok_or("the X server has no SHAPE extension")?;
        let depth = conn.get_geometry(xid).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?.depth;
        let gc = conn.generate_id().map_err(|e| e.to_string())?;
        conn.create_gc(gc, xid, &CreateGCAux::new()).map_err(|e| e.to_string())?;
        let state = WmState::intern(&conn)?;
        state.write(&conn, xid).map_err(|e| e.to_string())?;
        if !focusable {
            // WM_HINTS input = False: a click-to-focus window manager leaves
            // the user's window focused (Windows: WS_EX_NOACTIVATE)
            conn.change_property32(PropMode::REPLACE, xid, AtomEnum::WM_HINTS, AtomEnum::WM_HINTS, &[1, 0, 0, 0, 0, 0, 0, 0, 0])
                .map_err(|e| e.to_string())?;
        }
        let cm_atom = atom(&conn, &format!("_NET_WM_CM_S{screen}"))?;
        conn.flush().map_err(|e| e.to_string())?;
        let mut w = PetWindow {
            win,
            conn,
            xid,
            gc,
            depth,
            cm_atom,
            composited: false,
            checked: Instant::now(),
            buf: Vec::new(),
            pixmap: None,
            geom: None,
            opacity: None,
            shown: false,
            state,
        };
        w.composited = w.compositor_running();
        Ok(w)
    }

    fn compositor_running(&self) -> bool {
        // a 24-bit window has no alpha to composite, whatever is running
        self.depth == 32
            && self.conn.get_selection_owner(self.cm_atom).ok().and_then(|c| c.reply().ok()).is_some_and(|r| r.owner != 0)
    }

    pub fn id(&self) -> WindowId {
        self.win.id()
    }

    pub fn hide(&self) {
        self.win.set_visible(false);
    }

    pub fn show(&self) {
        let _ = self.state.write(&self.conn, self.xid);
        self.win.set_visible(true);
    }

    pub fn focus(&self) {
        self.win.focus_window();
    }

    /// Clicks already pass through transparent pixels (the input shape).
    pub fn set_click_through(&self, _through: bool) {}

    /// Show `img` with its top-left at physical `(x, y)`. `opacity` is 0–255
    /// and applies on top of the per-pixel alpha (ignored with no compositor).
    pub fn present(&mut self, img: &Rgba, x: i32, y: i32, opacity: u8) -> Result<(), String> {
        if img.w == 0 || img.h == 0 || img.px.len() != img.w * img.h * 4 || img.w > 0x7fff || img.h > 0x7fff {
            return Err("bad image size".into());
        }
        let e = |e: x11rb::errors::ConnectionError| e.to_string();
        if self.checked.elapsed() >= COMPOSITOR_RECHECK {
            self.checked = Instant::now();
            let now = self.compositor_running();
            if now != self.composited {
                self.composited = now;
                self.geom = None; // re-cut the shapes
            }
        }
        if self.geom.map(|g| (g.2, g.3)) != Some((img.w, img.h)) || self.geom.map(|g| (g.0, g.1)) != Some((x, y)) {
            let aux = ConfigureWindowAux::new().x(x).y(y).width(img.w as u32).height(img.h as u32);
            self.conn.configure_window(self.xid, &aux).map_err(e)?;
        }
        // premultiplied BGRA: the byte order of a little-endian ARGB visual
        self.buf.resize(img.px.len(), 0);
        for (d, s) in self.buf.chunks_exact_mut(4).zip(img.px.chunks_exact(4)) {
            let a = s[3] as u32;
            let pm = |c: u8| ((c as u32 * a + 127) / 255) as u8;
            d[0] = pm(s[2]);
            d[1] = pm(s[1]);
            d[2] = pm(s[0]);
            d[3] = s[3];
        }
        let rects = |min: u8| -> Vec<Rectangle> {
            shape_runs(img, min)
                .into_iter()
                .map(|(x, y, w, h)| Rectangle { x: x as i16, y: y as i16, width: w as u16, height: h as u16 })
                .collect()
        };
        self.conn
            .shape_rectangles(SO::SET, SK::INPUT, ClipOrdering::YX_BANDED, self.xid, 0, 0, &rects(1))
            .map_err(e)?;
        if self.composited {
            if self.geom.is_none() {
                self.conn.shape_mask(SO::SET, SK::BOUNDING, self.xid, 0, 0, x11rb::NONE).map_err(e)?;
            }
        } else {
            self.conn
                .shape_rectangles(SO::SET, SK::BOUNDING, ClipOrdering::YX_BANDED, self.xid, 0, 0, &rects(128))
                .map_err(e)?;
        }
        if self.opacity != Some(opacity) {
            let a = atom(&self.conn, "_NET_WM_WINDOW_OPACITY")?;
            let v = (0xffff_ffffu64 * opacity as u64 / 255) as u32;
            self.conn.change_property32(PropMode::REPLACE, self.xid, a, AtomEnum::CARDINAL, &[v]).map_err(e)?;
            self.opacity = Some(opacity);
        }
        if self.pixmap.map(|p| (p.1, p.2)) != Some((img.w, img.h)) {
            if let Some((old, ..)) = self.pixmap.take() {
                self.conn.free_pixmap(old).map_err(e)?;
            }
            let pm = self.conn.generate_id().map_err(|e| e.to_string())?;
            self.conn.create_pixmap(self.depth, pm, self.xid, img.w as u16, img.h as u16).map_err(e)?;
            self.pixmap = Some((pm, img.w, img.h));
        }
        let pm = self.pixmap.unwrap().0;
        let image = Image::new(
            img.w as u16,
            img.h as u16,
            ScanlinePad::Pad32,
            self.depth,
            BitsPerPixel::B32,
            ImageOrder::LsbFirst,
            Cow::Borrowed(&self.buf),
        )
        .map_err(|e| e.to_string())?;
        image.put(&self.conn, pm, self.gc, 0, 0).map_err(e)?;
        self.conn
            .change_window_attributes(self.xid, &ChangeWindowAttributesAux::new().background_pixmap(pm))
            .map_err(e)?;
        self.conn.clear_area(false, self.xid, 0, 0, 0, 0).map_err(e)?;
        self.conn.flush().map_err(e)?;
        if !self.shown {
            self.show();
            self.shown = true;
        }
        self.geom = Some((x, y, img.w, img.h));
        Ok(())
    }
}

impl Drop for PetWindow {
    fn drop(&mut self) {
        if let Some((pm, ..)) = self.pixmap.take() {
            let _ = self.conn.free_pixmap(pm);
        }
        let _ = self.conn.free_gc(self.gc);
        let _ = self.conn.flush();
    }
}

#[cfg(test)]
mod tests {
    //! Run under a real X server (CI: xvfb-run). No window manager there, so
    //! the test removes the state itself, as a WM does on unmap.
    use super::*;
    use x11rb::protocol::xproto::{CreateWindowAux, WindowClass};

    #[test]
    fn state_is_written_again_after_the_wm_drops_it() {
        let (conn, screen) = crate::platform::connect_x().expect("an X server (run under xvfb-run)");
        let s = &conn.setup().roots[screen];
        let w = conn.generate_id().unwrap();
        conn.create_window(0, w, s.root, 0, 0, 10, 10, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new()).unwrap();
        let st = WmState::intern(&conn).unwrap();
        let read = || -> Vec<u32> {
            let r = conn.get_property(false, w, st.prop, AtomEnum::ATOM, 0, 16).unwrap().reply().unwrap();
            r.value32().map(|v| v.collect()).unwrap_or_default()
        };
        st.write(&conn, w).unwrap();
        assert_eq!(read(), st.atoms);
        conn.delete_property(w, st.prop).unwrap();
        assert!(read().is_empty());
        st.write(&conn, w).unwrap();
        assert_eq!(read(), st.atoms, "show() must restore on-top and skip-taskbar");
    }
}
