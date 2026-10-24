//! Updates from GitHub releases: a check on launch and once a day, and on a
//! click in the menu, download the file for this system, check its SHA-256
//! against the one GitHub publishes, swap it in place and restart.
//! The network goes through the system's own `curl` (every Windows 10+, macOS
//! and almost every Linux has one), so PixelPet carries no TLS code.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/Rishi943/pixelpet/releases/latest";
/// First check this long after launch, then this often.
pub const FIRST_CHECK: f64 = 8.0;
pub const CHECK_EVERY: f64 = 24.0 * 3600.0;
/// Set on the restarted copy, so it waits for the old one to let go.
pub const AFTER_UPDATE: &str = "PIXELPET_AFTER_UPDATE";

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Nothing to offer (not checked yet, up to date, or the check failed).
    None,
    Available(Release),
    Installing,
    /// Swapped in: restart into this program.
    Ready(PathBuf),
    /// Shown in the menu in place of the offer.
    Failed(String),
}

/// What gets replaced, per system.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// The exe itself (Windows).
    Exe(PathBuf),
    /// The AppImage file (Linux).
    AppImage(PathBuf),
    /// The whole PixelPet.app, from a zip of it (macOS).
    App(PathBuf),
}

impl Target {
    /// The release file this target is updated from.
    pub fn asset(&self) -> &'static str {
        match self {
            Target::Exe(_) => "PixelPet.exe",
            Target::AppImage(_) => "PixelPet-x86_64.AppImage",
            Target::App(_) => "PixelPet-mac.zip",
        }
    }

    /// The program to start after the swap.
    pub fn program(&self) -> PathBuf {
        match self {
            Target::Exe(p) | Target::AppImage(p) => p.clone(),
            Target::App(app) => app.join("Contents/MacOS/PixelPet"),
        }
    }
}

/// What this copy would replace, or None when it can't update itself: a dev
/// build, a Linux binary not in an AppImage, a Mac app run from the dmg or
/// from where macOS moved it to (App Translocation).
pub fn target(exe: &Path, appimage: Option<&str>) -> Result<Target, &'static str> {
    if cfg!(target_os = "linux") {
        return match appimage {
            Some(a) if !a.is_empty() => Ok(Target::AppImage(PathBuf::from(a))),
            _ => Err("Not an AppImage"),
        };
    }
    if cfg!(target_os = "macos") {
        return mac_app(exe);
    }
    Ok(Target::Exe(exe.to_path_buf()))
}

/// The `.app` holding `exe`, if it may be replaced.
pub fn mac_app(exe: &Path) -> Result<Target, &'static str> {
    let app = exe.parent().and_then(Path::parent).and_then(Path::parent).ok_or("Not in an app")?;
    let s = app.to_string_lossy();
    if !s.ends_with(".app") {
        return Err("Not in an app");
    }
    if s.contains("/AppTranslocation/") || s.starts_with("/Volumes/") {
        return Err("Move PixelPet to Applications to update");
    }
    Ok(Target::App(app.to_path_buf()))
}

/// Whether `tag` (like "v0.1.1") is a newer plain release than `current`.
/// Pre-release tags ("v0.2.0-rc1") never count.
pub fn newer(current: &str, tag: &str) -> bool {
    fn parse(v: &str) -> Option<(u32, u32, u32)> {
        let n: Vec<u32> = v.trim().trim_start_matches('v').split('.').map(|n| n.parse().ok()).collect::<Option<_>>()?;
        if n.is_empty() || n.len() > 3 {
            return None;
        }
        Some((n[0], n.get(1).copied().unwrap_or(0), n.get(2).copied().unwrap_or(0)))
    }
    match (parse(current), parse(tag)) {
        (Some(c), Some(t)) => t > c,
        _ => false,
    }
}

/// The string value of the first `"key":"..."` at or after `from`.
fn field<'a>(json: &'a str, key: &str, from: usize) -> Option<(&'a str, usize)> {
    let pat = format!("\"{key}\":");
    let at = from + json[from..].find(&pat)? + pat.len();
    let rest = json[at..].trim_start();
    let start = at + (json[at..].len() - rest.len()) + 1;
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some((&json[start..start + end], start + end))
}

/// The release's version, and the download URL and SHA-256 of `asset`, from
/// the GitHub API's JSON for one release.
pub fn parse_release(json: &str, asset: &str) -> Option<Release> {
    let (tag, _) = field(json, "tag_name", 0)?;
    let assets = json.find("\"assets\":")?;
    // each asset ends with its browser_download_url; its own name and digest
    // are the last ones before that
    let mut from = assets;
    while let Some((url, end)) = field(json, "browser_download_url", from) {
        let chunk = &json[from..end];
        let name = chunk.rmatch_indices("\"name\":").next().and_then(|(i, _)| field(chunk, "name", i)).map(|(n, _)| n);
        if name == Some(asset) {
            let digest = chunk.rmatch_indices("\"digest\":").next().and_then(|(i, _)| field(chunk, "digest", i))?.0;
            let sha = digest.strip_prefix("sha256:")?;
            if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            return Some(Release { version: tag.trim_start_matches('v').to_string(), url: url.to_string(), sha256: sha.to_ascii_lowercase() });
        }
        from = end;
    }
    None
}

/// `curl`, without a console window on Windows.
fn curl() -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let mut c = Command::new(Path::new(&root).join(r"System32\curl.exe"));
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        c
    }
    #[cfg(not(windows))]
    {
        Command::new(if Path::new("/usr/bin/curl").exists() { "/usr/bin/curl" } else { "curl" })
    }
}

fn get(url: &str, out: Option<&Path>, max_secs: u32) -> Result<Vec<u8>, String> {
    let mut c = curl();
    c.args(["-fsSL", "--max-time", &max_secs.to_string(), "-H", "User-Agent: PixelPet", "-H", "Accept: application/vnd.github+json"]);
    if let Some(p) = out {
        c.arg("-o").arg(p);
    }
    let r = c.arg(url).output().map_err(|e| format!("curl: {e}"))?;
    if !r.status.success() {
        return Err(format!("download failed ({})", r.status));
    }
    Ok(r.stdout)
}

/// Ask GitHub for the latest release; Some when it's newer than this build.
pub fn check(target: &Target) -> Option<Release> {
    latest(target).filter(|r| newer(env!("CARGO_PKG_VERSION"), &r.version))
}

/// The latest release's file for `target`, whatever its version.
pub fn latest(target: &Target) -> Option<Release> {
    let json = String::from_utf8(get(LATEST, None, 20).ok()?).ok()?;
    parse_release(&json, target.asset())
}

/// Download `r`, check it, and swap it in. Returns the program to restart.
pub fn install(target: &Target, r: &Release) -> Result<PathBuf, String> {
    let dest = match target {
        Target::Exe(p) | Target::AppImage(p) => p.clone(),
        Target::App(app) => app.clone(),
    };
    let dir = dest.parent().ok_or("no folder")?;
    let tmp = dir.join(format!(".{}.download", target.asset()));
    let _ = std::fs::remove_file(&tmp);
    get(&r.url, Some(&tmp), 600)?;
    let data = std::fs::read(&tmp).map_err(|e| e.to_string())?;
    if hex(&sha256(&data)) != r.sha256 {
        let _ = std::fs::remove_file(&tmp);
        return Err("download was damaged".into());
    }
    let done = swap(target, &tmp, dir);
    let _ = std::fs::remove_file(&tmp);
    done.map(|_| target.program())
}

fn swap(target: &Target, tmp: &Path, dir: &Path) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    match target {
        Target::Exe(exe) => {
            // a running exe can't be overwritten, but it can be renamed
            let old = old_exe(exe);
            let _ = std::fs::remove_file(&old);
            std::fs::rename(exe, &old).map_err(io)?;
            if let Err(e) = std::fs::rename(tmp, exe) {
                let _ = std::fs::rename(&old, exe);
                return Err(e.to_string());
            }
            Ok(())
        }
        Target::AppImage(img) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(tmp, std::fs::Permissions::from_mode(0o755)).map_err(io)?;
            }
            std::fs::rename(tmp, img).map_err(io)
        }
        Target::App(app) => {
            let unpack = dir.join(".PixelPet-update");
            let _ = std::fs::remove_dir_all(&unpack);
            let ok = Command::new("/usr/bin/ditto").arg("-x").arg("-k").arg(tmp).arg(&unpack).status().map_err(io)?;
            let new_app = unpack.join("PixelPet.app");
            if !ok.success() || !new_app.join("Contents/MacOS/PixelPet").is_file() {
                let _ = std::fs::remove_dir_all(&unpack);
                return Err("could not unpack the update".into());
            }
            let old = old_app(app);
            let _ = std::fs::remove_dir_all(&old);
            std::fs::rename(app, &old).map_err(io)?;
            if let Err(e) = std::fs::rename(&new_app, app) {
                let _ = std::fs::rename(&old, app);
                let _ = std::fs::remove_dir_all(&unpack);
                return Err(e.to_string());
            }
            let _ = std::fs::remove_dir_all(&unpack);
            Ok(())
        }
    }
}

fn old_exe(exe: &Path) -> PathBuf {
    let mut s = exe.as_os_str().to_owned();
    s.push(".old");
    PathBuf::from(s)
}

fn old_app(app: &Path) -> PathBuf {
    app.with_file_name(".PixelPet-old.app")
}

/// Remove what the last update left behind (the renamed old program).
pub fn clean_up(target: &Target) {
    match target {
        Target::Exe(exe) => {
            let _ = std::fs::remove_file(old_exe(exe));
        }
        Target::App(app) => {
            let _ = std::fs::remove_dir_all(old_app(app));
        }
        Target::AppImage(_) => {}
    }
}

/// Start the updated program; the caller then quits.
pub fn restart(program: &Path) -> std::io::Result<()> {
    Command::new(program).env(AFTER_UPDATE, "1").spawn().map(|_| ())
}

/// Runs checks and installs off the UI thread; the app reads `state` each tick.
pub struct Updater {
    pub target: Option<Target>,
    pub state: Arc<Mutex<State>>,
    busy: Arc<Mutex<bool>>,
    pub last_check: Option<f64>,
}

impl Updater {
    pub fn new(target: Option<Target>) -> Updater {
        if let Some(t) = &target {
            clean_up(t);
        }
        Updater { target, state: Arc::new(Mutex::new(State::None)), busy: Arc::new(Mutex::new(false)), last_check: None }
    }

    pub fn state(&self) -> State {
        self.state.lock().map(|s| s.clone()).unwrap_or(State::None)
    }

    /// Check now if it's due (and nothing else is running).
    pub fn tick(&mut self, now: f64) {
        let Some(t) = self.target.clone() else { return };
        let due = match self.last_check {
            None => now >= FIRST_CHECK,
            Some(at) => now - at >= CHECK_EVERY,
        };
        if !due || !matches!(self.state(), State::None) {
            return;
        }
        self.last_check = Some(now);
        self.spawn(move |state| {
            if let Some(r) = check(&t) {
                *state.lock().unwrap() = State::Available(r);
            }
        });
    }

    /// Download and swap in the release on offer.
    pub fn install(&mut self) {
        let (Some(t), State::Available(r)) = (self.target.clone(), self.state()) else { return };
        if let Ok(mut s) = self.state.lock() {
            *s = State::Installing;
        }
        self.spawn(move |state| {
            let next = match install(&t, &r) {
                Ok(p) => State::Ready(p),
                Err(e) => State::Failed(format!("Update failed: {e}")),
            };
            *state.lock().unwrap() = next;
        });
    }

    fn spawn(&self, job: impl FnOnce(&Mutex<State>) + Send + 'static) {
        {
            let Ok(mut b) = self.busy.lock() else { return };
            if *b {
                return;
            }
            *b = true;
        }
        let (state, busy) = (self.state.clone(), self.busy.clone());
        std::thread::spawn(move || {
            job(&state);
            *busy.lock().unwrap() = false;
        });
    }
}

/// Wait up to `secs` for `claim` to succeed: the old copy is still quitting.
pub fn wait_for<T>(secs: f64, mut claim: impl FnMut() -> Option<T>) -> Option<T> {
    let tries = (secs / 0.1).ceil() as u32;
    for _ in 0..tries {
        if let Some(g) = claim() {
            return Some(g);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    claim()
}

/// SHA-256 (FIPS 180-4).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([block[4 * i], block[4 * i + 1], block[4 * i + 2], block[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_values() {
        assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let long = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(hex(&sha256(long)), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    }

    #[test]
    fn newer_versions() {
        assert!(newer("0.1.0", "v0.1.1"));
        assert!(newer("0.1.9", "v0.2.0"));
        assert!(newer("0.9.0", "1.0"));
        assert!(!newer("0.1.1", "v0.1.1"));
        assert!(!newer("0.2.0", "v0.1.9"));
        assert!(!newer("0.1.0", "v0.2.0-rc1"));
        assert!(!newer("0.1.0", "nonsense"));
    }

    const JSON: &str = r#"{"url":"https://api.github.com/repos/x/y/releases/1","author":{"login":"a","id":1},"tag_name":"v0.1.1","name":"PixelPet v0.1.1","draft":false,"assets":[{"url":"u1","id":1,"name":"PixelPet-x86_64.AppImage","label":"","uploader":{"login":"a","id":1},"content_type":"application/vnd.appimage","size":2,"digest":"sha256:fa6849c3426c6bb49c0cbfc02d5685fb87fc6ad069b3ace30373c7178400c0a1","browser_download_url":"https://github.com/x/y/releases/download/v0.1.1/PixelPet-x86_64.AppImage"},{"url":"u2","id":2,"name":"PixelPet.exe","label":"","uploader":{"login":"a","id":1},"size":3,"digest": "sha256:0DC819B2C198B1EB57E8F09A3A43370263A5ADC98EDD972F735F6BC41C2BD5AB","browser_download_url":"https://github.com/x/y/releases/download/v0.1.1/PixelPet.exe"},{"url":"u3","id":3,"name":"PixelPet-mac.zip","label":"","digest":null,"browser_download_url":"https://github.com/x/y/releases/download/v0.1.1/PixelPet-mac.zip"}],"body":"notes"}"#;

    #[test]
    fn finds_the_asset_and_its_digest() {
        let r = parse_release(JSON, "PixelPet.exe").unwrap();
        assert_eq!(r.version, "0.1.1");
        assert_eq!(r.url, "https://github.com/x/y/releases/download/v0.1.1/PixelPet.exe");
        assert_eq!(r.sha256, "0dc819b2c198b1eb57e8f09a3a43370263a5adc98edd972f735f6bc41c2bd5ab");
        let r = parse_release(JSON, "PixelPet-x86_64.AppImage").unwrap();
        assert!(r.url.ends_with("/PixelPet-x86_64.AppImage"));
        assert_eq!(&r.sha256[..8], "fa6849c3");
    }

    #[test]
    fn no_digest_or_no_asset_means_no_update() {
        assert_eq!(parse_release(JSON, "PixelPet-mac.zip"), None);
        assert_eq!(parse_release(JSON, "PixelPet.dmg"), None);
        assert_eq!(parse_release("{}", "PixelPet.exe"), None);
    }

    #[test]
    fn mac_targets() {
        let t = mac_app(Path::new("/Applications/PixelPet.app/Contents/MacOS/PixelPet")).unwrap();
        assert_eq!(t, Target::App(PathBuf::from("/Applications/PixelPet.app")));
        assert_eq!(t.program(), PathBuf::from("/Applications/PixelPet.app/Contents/MacOS/PixelPet"));
        assert!(mac_app(Path::new("/private/var/folders/x/AppTranslocation/ABC/d/PixelPet.app/Contents/MacOS/PixelPet")).is_err());
        assert!(mac_app(Path::new("/Volumes/PixelPet/PixelPet.app/Contents/MacOS/PixelPet")).is_err());
        assert!(mac_app(Path::new("/Users/r/dev/target/release/PixelPet")).is_err());
    }

    #[test]
    fn swaps_an_exe_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("pixelpet-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("PixelPet.exe");
        std::fs::write(&exe, b"old").unwrap();
        let tmp = dir.join("new");
        std::fs::write(&tmp, b"new").unwrap();
        let t = Target::Exe(exe.clone());
        swap(&t, &tmp, &dir).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"new");
        assert_eq!(std::fs::read(old_exe(&exe)).unwrap(), b"old");
        clean_up(&t);
        assert!(!old_exe(&exe).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
