//! CI only: a full-screen window of one colour, as a known desktop for the
//! Linux smoke screenshot (Xvfb's root stays black whatever xsetroot says).
//!     ci_backdrop RRGGBB      runs until killed

#[cfg(target_os = "linux")]
fn main() {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, WindowClass};
    use x11rb::xcb_ffi::XCBConnection;

    let rgb = u32::from_str_radix(&std::env::args().nth(1).expect("colour RRGGBB"), 16).expect("hex colour");
    let (conn, screen) = XCBConnection::connect(None).expect("X server");
    let s = &conn.setup().roots[screen];
    let w = conn.generate_id().unwrap();
    let aux = CreateWindowAux::new().background_pixel(rgb).override_redirect(1);
    conn.create_window(
        s.root_depth,
        w,
        s.root,
        0,
        0,
        s.width_in_pixels,
        s.height_in_pixels,
        0,
        WindowClass::INPUT_OUTPUT,
        s.root_visual,
        &aux,
    )
    .unwrap();
    conn.map_window(w).unwrap();
    conn.flush().unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
