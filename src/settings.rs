//! The settings file: plain `key = value` lines. Unknown keys are kept when
//! the file is rewritten; a missing or broken file means defaults.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    S,
    M,
    L,
    XL,
}

impl Size {
    /// Whole-number pixel zoom for look B.
    pub fn zoom(&self) -> u32 {
        match self {
            Size::S => 2,
            Size::M => 3,
            Size::L => 4,
            Size::XL => 5,
        }
    }

    fn key(&self) -> &'static str {
        match self {
            Size::S => "S",
            Size::M => "M",
            Size::L => "L",
            Size::XL => "XL",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FullscreenMode {
    Hide,
    Stay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The id of the pet on screen (`pets::ALEX_ID`, `pets::BUN_ID`, or a
    /// saved pet's id). v1's `alex` / `alex-the-bun` are the same ids.
    pub pet: String,
    pub size: Size,
    pub opacity: u8,
    pub fullscreen: FullscreenMode,
    pub fullscreen_opacity: u8,
    pub autostart: bool,
    extra: Vec<(String, String)>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            pet: crate::pets::ALEX_ID.to_string(),
            size: Size::M,
            opacity: 100,
            fullscreen: FullscreenMode::Hide,
            fullscreen_opacity: 50,
            autostart: false,
            extra: Vec::new(),
        }
    }
}

/// 25-100 in steps of 25, nearest step.
fn parse_opacity(v: &str) -> Option<u8> {
    let n: f64 = v.trim_end_matches('%').trim().parse().ok()?;
    if !n.is_finite() {
        return None;
    }
    Some(((n / 25.0).round() * 25.0).clamp(25.0, 100.0) as u8)
}

impl Settings {
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let lv = v.to_ascii_lowercase();
            match k.to_ascii_lowercase().as_str() {
                "pet" if !lv.is_empty() => s.pet = lv,
                "size" => match lv.as_str() {
                    "s" => s.size = Size::S,
                    "m" => s.size = Size::M,
                    "l" => s.size = Size::L,
                    "xl" => s.size = Size::XL,
                    _ => {}
                },
                "opacity" => s.opacity = parse_opacity(v).unwrap_or(s.opacity),
                "fullscreen" => match lv.as_str() {
                    "hide" => s.fullscreen = FullscreenMode::Hide,
                    "stay" => s.fullscreen = FullscreenMode::Stay,
                    _ => {}
                },
                "fullscreen_opacity" => s.fullscreen_opacity = parse_opacity(v).unwrap_or(s.fullscreen_opacity),
                "autostart" => match lv.as_str() {
                    "on" => s.autostart = true,
                    "off" => s.autostart = false,
                    _ => {}
                },
                _ => match s.extra.iter_mut().find(|(ek, _)| ek == k) {
                    Some(e) => e.1 = v.to_string(),
                    None => s.extra.push((k.to_string(), v.to_string())),
                },
            }
        }
        s
    }

    /// Known keys in spec order, then unknown keys as they were read.
    pub fn to_text(&self) -> String {
        let mut t = String::from("# PixelPet settings\n");
        t += &format!("pet = {}\n", self.pet);
        t += &format!("size = {}\n", self.size.key());
        t += &format!("opacity = {}\n", self.opacity);
        t += &format!("fullscreen = {}\n", if self.fullscreen == FullscreenMode::Hide { "hide" } else { "stay" });
        t += &format!("fullscreen_opacity = {}\n", self.fullscreen_opacity);
        t += &format!("autostart = {}\n", if self.autostart { "on" } else { "off" });
        for (k, v) in &self.extra {
            t += &format!("{k} = {v}\n");
        }
        t
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
}

pub fn current_os() -> Os {
    if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else {
        Os::Linux
    }
}

/// Where the settings file lives. A `settings.txt` next to the program
/// (portable mode) wins; otherwise the per-OS config folder. The path is
/// built as text with the target OS's separator, so it is the same on any host.
pub fn settings_path(os: Os, exe: &Path, env: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    let exe_dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let portable = exe_dir.join("settings.txt");
    if portable.is_file() {
        return portable;
    }
    let var = |k: &str| env(k).filter(|v| !v.is_empty());
    let found = match os {
        Os::Windows => var("APPDATA")
            .or_else(|| var("USERPROFILE").map(|h| format!("{h}\\AppData\\Roaming")))
            .map(|d| format!("{}\\PixelPet\\settings.txt", d.trim_end_matches(['\\', '/']))),
        Os::MacOs => var("HOME").map(|h| format!("{}/Library/Application Support/PixelPet/settings.txt", h.trim_end_matches('/'))),
        Os::Linux => var("XDG_CONFIG_HOME")
            .or_else(|| var("HOME").map(|h| format!("{}/.config", h.trim_end_matches('/'))))
            .map(|d| format!("{}/pixelpet/settings.txt", d.trim_end_matches('/'))),
    };
    found.map(PathBuf::from).unwrap_or(portable)
}

/// No settings file yet: PixelPet has never been run (or set up) here.
pub fn first_launch(settings_path: &Path) -> bool {
    !settings_path.exists()
}

/// Missing or unreadable file means defaults.
pub fn load(path: &Path) -> Settings {
    match std::fs::read(path) {
        Ok(bytes) => Settings::parse(&String::from_utf8_lossy(&bytes)),
        Err(_) => Settings::default(),
    }
}

/// Write `settings.txt.tmp`, then rename it over the real file.
pub fn save(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, s.to_text())?;
    std::fs::rename(&tmp, path)
}

/// Whether the launch-on-startup entry must be written or removed so that it
/// points at this program (or is gone when not wanted).
pub fn autostart_needs_update(current: Option<&Path>, exe: &Path, wanted: bool) -> bool {
    if wanted {
        current != Some(exe)
    } else {
        current.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn defaults_when_empty() {
        let s = Settings::parse("");
        assert_eq!(s, Settings::default());
        assert_eq!(s.pet, PetChoice::Alex);
        assert_eq!(s.size, Size::M);
        assert_eq!(s.opacity, 100);
        assert_eq!(s.fullscreen, FullscreenMode::Hide);
        assert_eq!(s.fullscreen_opacity, 50);
        assert!(!s.autostart);
        assert_eq!(s.size.zoom(), 3);
        assert_eq!(s.pet.pet(), Pet::Dog);
    }

    #[test]
    fn round_trip() {
        let s = Settings {
            pet: PetChoice::AlexTheBun,
            size: Size::XL,
            opacity: 75,
            fullscreen: FullscreenMode::Stay,
            fullscreen_opacity: 25,
            autostart: true,
            extra: vec![],
        };
        assert_eq!(Settings::parse(&s.to_text()), s);
        assert_eq!(s.pet.pet(), Pet::Rabbit);
        assert_eq!([Size::S, Size::M, Size::L, Size::XL].map(|z| z.zoom()), [2, 3, 4, 5]);
    }

    #[test]
    fn keeps_unknown_keys() {
        let s = Settings::parse("colour = red\npet = alex-the-bun\n");
        assert_eq!(s.pet, PetChoice::AlexTheBun);
        let text = s.to_text();
        assert!(text.contains("colour = red"), "{text}");
        assert!(text.starts_with("# PixelPet settings\n"), "{text}");
        assert_eq!(Settings::parse(&text), s);
    }

    #[test]
    fn tolerates_bom_crlf_blanks_spaces() {
        let s = Settings::parse("\u{feff}pet=alex-the-bun\r\n\r\n  size =  L \r\n# a comment\r\n");
        assert_eq!(s.pet, PetChoice::AlexTheBun);
        assert_eq!(s.size, Size::L);
    }

    #[test]
    fn bad_values_fall_back_per_key() {
        let s = Settings::parse("size = huge\npet = alex-the-bun\nopacity = lots\nnot a line\n");
        assert_eq!(s.size, Size::M);
        assert_eq!(s.pet, PetChoice::AlexTheBun);
        assert_eq!(s.opacity, 100);
    }

    #[test]
    fn opacity_snaps_to_steps() {
        for (given, want) in [(60, 50), (0, 25), (130, 100), (75, 75)] {
            let s = Settings::parse(&format!("opacity = {given}\nfullscreen_opacity = {given}"));
            assert_eq!(s.opacity, want, "opacity {given}");
            assert_eq!(s.fullscreen_opacity, want, "fullscreen_opacity {given}");
        }
    }

    #[test]
    fn paths() {
        let exe = Path::new("Z:/nowhere/PixelPet.exe");
        assert_eq!(
            settings_path(Os::Windows, exe, &env_of(&[("APPDATA", "C:\\U\\AppData\\Roaming")])),
            PathBuf::from("C:\\U\\AppData\\Roaming\\PixelPet\\settings.txt")
        );
        assert_eq!(
            settings_path(Os::MacOs, exe, &env_of(&[("HOME", "/Users/u")])),
            PathBuf::from("/Users/u/Library/Application Support/PixelPet/settings.txt")
        );
        assert_eq!(
            settings_path(Os::Linux, exe, &env_of(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/home/u")])),
            PathBuf::from("/x/pixelpet/settings.txt")
        );
        assert_eq!(
            settings_path(Os::Linux, exe, &env_of(&[("HOME", "/home/u")])),
            PathBuf::from("/home/u/.config/pixelpet/settings.txt")
        );
    }

    /// A fresh, empty directory of our own under the system temp dir.
    fn scratch_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pixelpet-test-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn portable_mode() {
        let dir = scratch_dir("portable");
        let exe = dir.join("PixelPet.exe");
        assert_eq!(
            settings_path(Os::Windows, &exe, &env_of(&[("APPDATA", "C:\\U")])),
            PathBuf::from("C:\\U\\PixelPet\\settings.txt")
        );
        std::fs::write(dir.join("settings.txt"), "size = L\n").unwrap();
        assert_eq!(settings_path(Os::Windows, &exe, &env_of(&[("APPDATA", "C:\\U")])), dir.join("settings.txt"));
    }

    #[test]
    fn save_then_load() {
        let dir = scratch_dir("save");
        let path = dir.join("sub").join("settings.txt");
        assert_eq!(load(&path), Settings::default());
        let s = Settings::parse("pet = alex-the-bun\ncolour = red\n");
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);
        assert!(!dir.join("sub").join("settings.txt.tmp").exists());
    }

    #[test]
    fn autostart_needs_update_cases() {
        let exe = Path::new("C:/a/PixelPet.exe");
        let old = Path::new("C:/old/PixelPet.exe");
        assert!(autostart_needs_update(None, exe, true));
        assert!(!autostart_needs_update(Some(exe), exe, true));
        assert!(autostart_needs_update(Some(old), exe, true));
        assert!(autostart_needs_update(Some(exe), exe, false));
        assert!(!autostart_needs_update(None, exe, false));
    }
}
