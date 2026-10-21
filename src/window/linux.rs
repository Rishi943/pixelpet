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
        let (conn, screen) = XCBConnection::connect(None).map_err(|e| e.to_string())?;
        conn.extension_information(shape::X11_EXTENSION_NAME)
            .map_err(|e| e.to_string())?
            .ok_or("the X server has no SHAPE extension")?;
        let depth = conn.get_geometry(xid).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?.depth;
        let gc = conn.generate_id().map_err(|e| e.to_string())?;
        conn.create_gc(gc, xid, &CreateGCAux::new()).map_err(|e| e.to_string())?;
        // no taskbar or pager entry; on top. Set before the first map so the
        // window manager reads it then.
        let state = atom(&conn, "_NET_WM_STATE")?;
        let wanted = [
            atom(&conn, "_NET_WM_STATE_ABOVE")?,
            atom(&conn, "_NET_WM_STATE_SKIP_TASKBAR")?,
            atom(&conn, "_NET_WM_STATE_SKIP_PAGER")?,
        ];
        conn.change_property32(PropMode::REPLACE, xid, state, AtomEnum::ATOM, &wanted).map_err(|e| e.to_string())?;
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
