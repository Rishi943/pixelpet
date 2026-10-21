//! macOS: a borderless, non-opaque winit window whose view's layer shows a
//! CGImage with alpha. Clicks pass through where the app says (winit's
//! `set_cursor_hittest`, which is `ignoresMouseEvents`), because macOS has
//! no per-pixel click-through for a window like this.

use super::Rgba;
use core_graphics::base::{kCGImageAlphaPremultipliedLast, kCGRenderingIntentDefault};
use core_graphics::color_space::CGColorSpace;
use core_graphics::data_provider::CGDataProvider;
use core_graphics::image::CGImage;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_foundation::NSString;
use std::sync::Arc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

/// NSWindowCollectionBehavior: on every Space, including fullscreen ones
/// (the app hides the pet there when the setting says so), not moved by
/// Exposé, skipped by Cmd-`.
const BEHAVIOUR: usize = (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8);

pub struct PetWindow {
    win: Window,
    ns_window: *mut AnyObject,
    layer: *mut AnyObject,
    /// the image the layer shows; kept alive as long as it is shown
    image: Option<CGImage>,
    /// last (x, y, w, h) sent, and the scale it was sent at: winit converts
    /// physical sizes to points with the scale of the moment
    geom: Option<(i32, i32, usize, usize, f64)>,
    opacity: Option<u8>,
    shown: bool,
}

impl PetWindow {
    pub fn new(el: &ActiveEventLoop, focusable: bool) -> Result<PetWindow, String> {
        let attrs = WindowAttributes::default()
            .with_title("PixelPet")
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_active(focusable)
            .with_visible(false);
        let win = el.create_window(attrs).map_err(|e| e.to_string())?;
        let view = match win.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as *mut AnyObject,
            _ => return Err("not an AppKit window".into()),
        };
        unsafe {
            let ns_window: *mut AnyObject = msg_send![view, window];
            if ns_window.is_null() {
                return Err("the view has no window".into());
            }
            let clear: *mut AnyObject = msg_send![class!(NSColor), clearColor];
            let _: () = msg_send![ns_window, setOpaque: Bool::NO];
            let _: () = msg_send![ns_window, setBackgroundColor: clear];
            let _: () = msg_send![ns_window, setCollectionBehavior: BEHAVIOUR];
            // a layer-HOSTING view: our own layer, set before wantsLayer, so
            // AppKit never redraws it. A layer-backed view's layer is redrawn
            // when the window gains focus, which blanked the menu (drawn once).
            let layer: *mut AnyObject = msg_send![class!(CALayer), layer];
            if layer.is_null() {
                return Err("no CALayer".into());
            }
            let _: () = msg_send![view, setLayer: layer];
            let _: () = msg_send![view, setWantsLayer: Bool::YES];
            // follow the view's size: kCALayerWidthSizable | kCALayerHeightSizable
            let _: () = msg_send![layer, setAutoresizingMask: 2u32 | 16u32];
            // hard pixels: never smooth when the layer is drawn at a scale
            let nearest = NSString::from_str("nearest");
            let _: () = msg_send![layer, setMagnificationFilter: &*nearest];
            let _: () = msg_send![layer, setMinificationFilter: &*nearest];
            let gravity = NSString::from_str("resize");
            let _: () = msg_send![layer, setContentsGravity: &*gravity];
            Ok(PetWindow { win, ns_window, layer, image: None, geom: None, opacity: None, shown: false })
        }
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

    /// Take clicks (false) or let them through to what is underneath (true).
    pub fn set_click_through(&self, through: bool) {
        let _ = self.win.set_cursor_hittest(!through);
    }

    /// Show `img` with its top-left at physical `(x, y)`. `opacity` is 0–255
    /// and applies on top of the per-pixel alpha.
    pub fn present(&mut self, img: &Rgba, x: i32, y: i32, opacity: u8) -> Result<(), String> {
        if img.w == 0 || img.h == 0 || img.px.len() != img.w * img.h * 4 {
            return Err("bad image size".into());
        }
        let scale = self.win.scale_factor();
        let resize = self.geom.map(|g| (g.2, g.3, g.4)) != Some((img.w, img.h, scale));
        if resize {
            let _ = self.win.request_inner_size(PhysicalSize::new(img.w as u32, img.h as u32));
        }
        // a resize keeps Cocoa's bottom-left corner: put the top-left back
        if resize || self.geom.map(|g| (g.0, g.1)) != Some((x, y)) {
            self.win.set_outer_position(PhysicalPosition::new(x, y));
        }
        // premultiplied RGBA, as CoreGraphics wants it
        let mut px = img.px.clone();
        for p in px.chunks_exact_mut(4) {
            let a = p[3] as u32;
            for c in &mut p[..3] {
                *c = ((*c as u32 * a + 127) / 255) as u8;
            }
        }
        let provider = CGDataProvider::from_buffer(Arc::new(px));
        let image = CGImage::new(
            img.w,
            img.h,
            8,
            32,
            img.w * 4,
            &CGColorSpace::create_device_rgb(),
            kCGImageAlphaPremultipliedLast,
            &provider,
            false,
            kCGRenderingIntentDefault,
        );
        unsafe {
            let _: () = msg_send![self.layer, setContentsScale: scale];
            // a foreign-types `&CGImageRef` is the CGImageRef pointer itself
            let contents = &*image as *const core_graphics::image::CGImageRef as *mut AnyObject;
            let _: () = msg_send![self.layer, setContents: contents];
            if self.opacity != Some(opacity) {
                let _: () = msg_send![self.ns_window, setAlphaValue: opacity as f64 / 255.0];
                self.opacity = Some(opacity);
            }
        }
        self.image = Some(image);
        if !self.shown {
            self.show();
            self.shown = true;
        }
        self.geom = Some((x, y, img.w, img.h, scale));
        Ok(())
    }
}
