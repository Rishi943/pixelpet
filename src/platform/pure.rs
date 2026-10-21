//! Pure helpers the macOS and Linux platforms use, kept here so they are
//! tested on any host: coordinate flips, the launch-on-startup files, and
//! where launch-on-startup should point.

use super::Rect;
use std::path::{Path, PathBuf};

/// A Cocoa rect `(x, y, w, h)` in points, origin at the bottom-left of the
/// primary screen, as physical pixels with the origin top-left.
pub fn mac_rect(frame: (f64, f64, f64, f64), primary_h: f64, scale: f64) -> Rect {
    let (x, y, w, h) = frame;
    Rect {
        x: (x * scale).round() as i32,
        y: ((primary_h - y - h) * scale).round() as i32,
        w: (w * scale).round() as i32,
        h: (h * scale).round() as i32,
    }
}

/// A Cocoa point (bottom-left origin, points) as physical top-left pixels.
pub fn mac_point(p: (f64, f64), primary_h: f64, scale: f64) -> (i32, i32) {
    ((p.0 * scale).round() as i32, ((primary_h - p.1) * scale).round() as i32)
}

/// Physical top-left pixels as CoreGraphics global points (top-left origin).
pub fn cg_point(x: i32, y: i32, scale: f64) -> (f64, f64) {
    (x as f64 / scale, y as f64 / scale)
}

const LABEL: &str = "com.pixelpet.app";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// `~/Library/LaunchAgents/com.pixelpet.app.plist` that starts `exe` at login.
pub fn launch_agent_plist(exe: &Path) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         \t<key>Label</key>\n\t<string>{LABEL}</string>\n\
         \t<key>ProgramArguments</key>\n\t<array>\n\t\t<string>{}</string>\n\t</array>\n\
         \t<key>RunAtLoad</key>\n\t<true/>\n\
         </dict>\n</plist>\n",
        xml_escape(&exe.to_string_lossy())
    )
}

/// The program a plist written by `launch_agent_plist` starts.
pub fn program_from_plist(text: &str) -> Option<PathBuf> {
    let after = &text[text.find("<key>ProgramArguments</key>")?..];
    let start = after.find("<string>")? + "<string>".len();
    let end = after[start..].find("</string>")? + start;
    Some(PathBuf::from(xml_unescape(&after[start..end])))
}

/// `~/.config/autostart/pixelpet.desktop` that starts `exe` at login.
/// Exec quoting and string escaping follow the desktop-entry spec.
pub fn autostart_desktop(exe: &Path) -> String {
    let mut arg = String::from("\"");
    for c in exe.to_string_lossy().chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                arg.push('\\');
                arg.push(c);
            }
            '%' => arg.push_str("%%"),
            _ => arg.push(c),
        }
    }
    arg.push('"');
    // string-level escaping: backslashes are doubled once more
    let exec = arg.replace('\\', "\\\\");
    format!(
        "[Desktop Entry]\nType=Application\nName=PixelPet\nExec={exec}\nX-GNOME-Autostart-enabled=true\n"
    )
}

/// The program a `.desktop` file written by `autostart_desktop` starts.
pub fn exec_from_desktop(text: &str) -> Option<PathBuf> {
    let line = text.lines().find_map(|l| l.strip_prefix("Exec="))?;
    let arg = line.replace("\\\\", "\\");
    let inner = arg.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            '"' => return Some(PathBuf::from(out.replace("%%", "%"))),
            _ => out.push(c),
        }
    }
    None
}

/// Where launch-on-startup should point: the AppImage itself when running
/// from one (`current_exe` is inside its temporary mount), never a macOS
/// App Translocation copy (a random path that vanishes).
pub fn autostart_target(exe: &Path, env: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, &'static str> {
    if let Some(a) = env("APPIMAGE").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(a));
    }
    if exe.to_string_lossy().contains("/AppTranslocation/") {
        return Err("move PixelPet to Applications first");
    }
    Ok(exe.to_path_buf())
}

/// Whether this is a Wayland session (PixelPet then runs through XWayland,
/// which answers idle, warp and fullscreen queries for X clients only, so
/// the answers look real and are wrong: spec §8's Wayland column says none).
pub fn wayland_session(env: &dyn Fn(&str) -> Option<String>) -> bool {
    env("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) || env("XDG_SESSION_TYPE").is_some_and(|v| v == "wayland")
}
