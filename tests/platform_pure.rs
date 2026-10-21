//! Pure helpers for the macOS and Linux platforms, tested on any host.

use pixelpet::platform::lock::Lock;
use pixelpet::platform::pure::*;
use pixelpet::platform::Rect;
use std::path::{Path, PathBuf};

#[test]
fn mac_rect_flips_and_scales() {
    // 1440x900-point Retina screen, Dock on the left (70 pt), menu bar 25 pt
    let r = mac_rect((70.0, 0.0, 1370.0, 875.0), 900.0, 2.0);
    assert_eq!(r, Rect { x: 140, y: 50, w: 2740, h: 1750 });
    assert_eq!(r.bottom(), 1800);
}

#[test]
fn mac_point_round_trips() {
    assert_eq!(mac_point((100.0, 800.0), 900.0, 2.0), (200, 200));
    assert_eq!(cg_point(200, 200, 2.0), (100.0, 100.0));
}

#[test]
fn plist_and_desktop_round_trip() {
    let p = Path::new("/Applications/PixelPet.app/Contents/MacOS/PixelPet");
    assert_eq!(program_from_plist(&launch_agent_plist(p)).as_deref(), Some(p));
    assert!(launch_agent_plist(p).contains("<string>com.pixelpet.app</string>"));
    let odd = Path::new("/Users/a & b/<Apps>/PixelPet");
    assert_eq!(program_from_plist(&launch_agent_plist(odd)).as_deref(), Some(odd));
    let q = Path::new("/home/a b/Apps/PixelPet-x86_64.AppImage");
    let d = autostart_desktop(q);
    assert!(d.starts_with("[Desktop Entry]\n"));
    assert_eq!(exec_from_desktop(&d).as_deref(), Some(q));
    let r = Path::new("/home/u/we\"ird $dir/PixelPet");
    assert_eq!(exec_from_desktop(&autostart_desktop(r)).as_deref(), Some(r));
}

#[test]
fn appimage_autostart_uses_appimage_var() {
    let env = |k: &str| (k == "APPIMAGE").then(|| "/home/u/PixelPet-x86_64.AppImage".to_string());
    assert_eq!(
        autostart_target(Path::new("/tmp/.mount_PixelAbc/usr/bin/PixelPet"), &env).unwrap(),
        PathBuf::from("/home/u/PixelPet-x86_64.AppImage")
    );
    let empty = |_: &str| Some(String::new());
    assert_eq!(autostart_target(Path::new("/opt/PixelPet"), &empty).unwrap(), PathBuf::from("/opt/PixelPet"));
}

#[test]
fn translocated_mac_path_refuses_autostart() {
    let p = Path::new("/private/var/folders/x/AppTranslocation/1A2B/d/PixelPet.app/Contents/MacOS/PixelPet");
    assert_eq!(autostart_target(p, &|_| None), Err("move PixelPet to Applications first"));
}

fn lock_path(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("pixelpet-lock-test-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

#[test]
fn second_lock_fails_first_drop_frees_it() {
    let p = lock_path("a.lock");
    let first = Lock::claim(&p).expect("first claim");
    assert!(Lock::claim(&p).is_none(), "second claim must fail while the first is held");
    drop(first);
    assert!(Lock::claim(&p).is_some(), "claim after drop");
}

/// Run as a child process by `lock_is_released_when_holder_dies`: claims
/// the lock and exits without dropping it.
#[test]
fn lock_helper() {
    if let Ok(p) = std::env::var("PIXELPET_LOCK_HELPER") {
        let l = Lock::claim(Path::new(&p)).expect("helper claim");
        std::mem::forget(l);
        std::process::exit(0);
    }
}

#[test]
fn lock_is_released_when_holder_dies() {
    let p = lock_path("b.lock");
    let st = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["lock_helper", "--exact", "--test-threads=1"])
        .env("PIXELPET_LOCK_HELPER", &p)
        .status()
        .unwrap();
    assert!(st.success());
    assert!(p.exists(), "the helper made the lock file");
    assert!(Lock::claim(&p).is_some(), "a dead holder's lock must not block");
}
