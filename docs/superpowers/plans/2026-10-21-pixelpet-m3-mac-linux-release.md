# PixelPet M3 — macOS, Linux and release builds — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The same PixelPet that runs on Windows runs on macOS and Linux (X11, and Wayland through XWayland), and a tagged release builds `PixelPet.exe`, `PixelPet.dmg` and `PixelPet-x86_64.AppImage` on GitHub Actions.

**Architecture:** Only `src/window/` and `src/platform/` get new per-OS files (`macos.rs`, `linux.rs`) behind the existing `PetWindow` API and `Platform` trait; `brain`, `rig`, `ui`, `scene` and `view` do not change. Everything that can be pure (coordinate flips, autostart file text, the X11 shape mask) is a pure function tested on Windows. The rest is proven on CI runners by starting the real app and screenshotting the desktop, because there is no Mac or Linux machine.

**Tech Stack:** Rust 2021, winit 0.30, `windows-sys` (Windows), `objc2`/`objc2-app-kit`/`objc2-foundation`/`core-graphics`/`core-foundation` (macOS, the versions winit already pulls in), `x11-dl` (Linux, the version winit already pulls in), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-20-pixelpet-v1-design.md` (§4, §6, §7, §8, §10, §11.6). M1/M2 plans in the same folder show the house style.

## Global Constraints

- Outside crates: `winit` everywhere; per OS, only crates winit already depends on, **pinned with `=` to the exact version in `Cargo.lock`** (as `windows-sys = "=0.52.0"` is). No new crate may enter `Cargo.lock` except dev-only crates. Check with `cargo tree -e normal --target all` before and after.
- No network access, no telemetry, **no reading of keys other than Esc**. On macOS, nothing may trigger the Input Monitoring or Accessibility permission prompt. Rushi: *"i dont want ppl to think we are logging their keys"*.
- The `brain` / `rig` / `view` boundary is unchanged: no `cfg(target_os)` outside `src/window/`, `src/platform/`, `src/main.rs` and `src/app.rs`'s event-loop builder.
- Coordinates passed through `Platform` and `PetWindow` stay **physical pixels, origin top-left of the primary monitor**, as on Windows.
- Settings paths are already per-OS in `settings::settings_path` — do not change them.
- Budget (§9) is measured on Windows only; M3 must not change the Windows numbers (sleep ≤ 0.5 % CPU, walk ≤ 1.5 %, ≤ 30 MB, exe ≤ 6 MB).
- The Windows build must stay pixel-for-pixel what Rushi signed off: `cargo test` (110 pass, 1 ignored) stays green on `x86_64-pc-windows-gnu` after every task.
- The repo is **private** (`Rishi943/pixelpet`). macOS runner minutes count 10× against the free private quota, so macOS jobs run **only on tags and `workflow_dispatch`**, never on every push.
- Pet photos and anything made from them never reach git or a CI artifact (`.gitignore` already covers them).
- Copy about Alex the bun stays pronoun-neutral.
- v1 is unsigned (§10).

## Review Focus

1. **Retina Mac (scale factor 2) with the Dock on the left.** The pet must stand on the top of the Dock-less bottom edge of `visibleFrame`, at physical pixels, not half-way up or off screen. Pinned by `mac_rect_flips_and_scales` (Task 2).
2. **Running PixelPet straight from the dmg or Downloads on macOS (App Translocation).** Turning on launch-at-startup must not write a path under `/AppTranslocation/`; it refuses with the "move PixelPet to Applications first" error. Pinned by `translocated_mac_path_refuses_autostart` (Task 2).
3. **AppImage autostart.** The `.desktop` entry must point at `$APPIMAGE`, not at the `/tmp/.mount_*` path `current_exe()` returns. Pinned by `appimage_autostart_uses_appimage_var` (Task 2).
4. **Linux desktop with no compositor (i3, openbox).** A 32-bit window shows a black box there. The pet must still show with hard edges through an XShape mask cut from alpha. Pinned by `shape_runs_cover_only_opaque_pixels` (Task 3) and the CI no-compositor screenshot (Task 4).
5. **Second launch on macOS/Linux.** A second PixelPet must exit, as on Windows, and a crashed one must not leave a stale lock that blocks the next launch. Pinned by `lock_is_released_when_holder_dies` (Task 2).

---

## File structure

| File | Responsibility |
|---|---|
| `src/platform/pure.rs` (new) | Pure helpers used by the OS files: mac coordinate flip, autostart file text and target, translocation check. |
| `src/platform/lock.rs` (new) | Single-instance lock on macOS/Linux with `std::fs::File::try_lock`. |
| `src/platform/linux.rs` (new) | `Platform` for X11. |
| `src/platform/macos.rs` (new) | `Platform` for macOS. |
| `src/window/linux.rs` (new) | `PetWindow` for X11: ARGB visual, `XPutImage`, shape fallback. |
| `src/window/shape.rs` (new) | Pure: alpha → run-length rectangles for XShape. |
| `src/window/macos.rs` (new) | `PetWindow` for macOS: non-opaque NSWindow, layer shows a CGImage. |
| `src/window/mod.rs`, `src/platform/mod.rs` | Wire the new files in place of the `unsupported`/`Stub` fallbacks. |
| `src/app.rs` | Click-through toggle each tick; X11 backend and macOS accessory policy at event-loop build. |
| `.github/workflows/ci.yml` (new) | Tests on Windows + Linux per push; macOS on tag/dispatch; screenshot smoke tests. |
| `.github/workflows/release.yml` (new) | Tagged release: exe, dmg, AppImage, as a draft GitHub release. |
| `tools/ci/` (new) | `smoke-linux.sh`, `smoke-macos.sh`, `check_shot.py`, `make-app.sh`, `make-appimage.sh`, `Info.plist`, `pixelpet.desktop`. |
| `examples/make_icon.rs` | Also writes the PNG sizes `iconutil` and AppImage need. |
| `docs/release.md` (new) | Download-page text for the unsigned first launch; the real-machine checklist still owed. |

Delegation note (from the chat on 2026-10-21): Tasks 1, 2 and 8 are self-contained and suit a Sonnet subagent. Tasks 3–7 are judged by screenshots and stay with the orchestrator.

---

### Task 1: GitHub repo, and CI that builds and tests on all three OSes

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: `src/window/mod.rs`, `src/platform/mod.rs` only if a non-Windows build fails to compile
- Test: the existing suite, on CI

**Interfaces:**
- Produces: a `ci.yml` with jobs `test-windows`, `test-linux`, `test-macos`; later tasks add steps to `test-linux` and `test-macos`.

- [ ] **Step 1: Create the remote** (Rushi chose a private repo, 2026-10-21)

Run: `gh repo create Rishi943/pixelpet --private --source . --remote origin --push`
Expected: `main` pushed; `git ls-files | grep -iE "alex/|alex the bun/|compare-"` prints nothing.

- [ ] **Step 2: Write `ci.yml`**

Triggers: `push` to any branch, `pull_request`, `workflow_dispatch`, tags `v*`. `test-windows` on `windows-latest` with toolchain `stable-x86_64-pc-windows-gnu` (same as the laptop). `test-linux` on `ubuntu-latest`: `apt-get install libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxkbcommon-x11-0 xvfb xcompmgr imagemagick`, then `cargo test`. `test-macos` on `macos-latest`, `if: startsWith(github.ref, 'refs/tags/') || github.event_name == 'workflow_dispatch'`, `cargo test`. All three cache `~/.cargo` and `target`.

- [ ] **Step 3: Push and read the run**

Run: `git push; gh run watch --exit-status`
Expected: `test-windows` and `test-linux` green, all of `tests/parity.rs` passing on Linux (the rig is the same everywhere). If Linux fails to compile, the fix is in the `cfg(not(windows))` fallbacks only.

- [ ] **Step 4: Run the macOS job once by hand**

Run: `gh workflow run ci.yml --ref main; gh run watch --exit-status`
Expected: `test-macos` green, parity tests included.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: test on Windows, Linux and (on tags) macOS"
```

---

### Task 2: Pure helpers — mac coordinates, autostart files, single-instance lock

**Files:**
- Create: `src/platform/pure.rs`, `src/platform/lock.rs`
- Modify: `src/platform/mod.rs` (`pub mod pure;`, `claim_single_instance` uses `lock` off Windows)
- Test: `tests/platform_pure.rs` (new)

**Interfaces:**
- Produces, in `pixelpet::platform::pure`:
  - `fn mac_rect(frame: (f64, f64, f64, f64), primary_h: f64, scale: f64) -> Rect` — a Cocoa rect `(x, y, w, h)` in points, origin bottom-left of the primary screen, to physical top-left `Rect`.
  - `fn mac_point(p: (f64, f64), primary_h: f64, scale: f64) -> (i32, i32)`, and its inverse `fn cg_point(x: i32, y: i32, scale: f64) -> (f64, f64)` (CoreGraphics global coords are top-left points).
  - `fn launch_agent_plist(exe: &Path) -> String` — label `com.pixelpet.app`, `ProgramArguments` = `[exe]`, `RunAtLoad` true. Paths XML-escaped.
  - `fn autostart_desktop(exe: &Path) -> String` — `[Desktop Entry]`, `Type=Application`, `Name=PixelPet`, `Exec="<exe>"` (quoted per the desktop-entry spec), `X-GNOME-Autostart-enabled=true`.
  - `fn exec_from_desktop(text: &str) -> Option<PathBuf>` and `fn program_from_plist(text: &str) -> Option<PathBuf>` — read back what the two writers wrote (for `Platform::autostart`).
  - `fn autostart_target(exe: &Path, env: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, &'static str>` — `$APPIMAGE` when set and non-empty; `Err("move PixelPet to Applications first")` when the path contains `/AppTranslocation/`; else `exe`.
- Produces, in `platform::lock`: `struct Lock(File)`, `fn Lock::claim(path: &Path) -> Option<Lock>` using `File::try_lock` (stable since Rust 1.89; the laptop has 1.99). The OS drops the lock when the process dies.
- `claim_single_instance` on macOS/Linux locks `<dir of settings file>/pixelpet.lock` (create the dir if missing).

- [ ] **Step 1: Write the failing tests** in `tests/platform_pure.rs`

```rust
#[test] fn mac_rect_flips_and_scales() {
    // 1440x900-point Retina screen, Dock on the left (70 pt), menu bar 25 pt
    let r = mac_rect((70.0, 0.0, 1370.0, 875.0), 900.0, 2.0);
    assert_eq!(r, Rect { x: 140, y: 50, w: 2740, h: 1750 });
    assert_eq!(r.bottom(), 1800);
}
#[test] fn mac_point_round_trips() {
    assert_eq!(mac_point((100.0, 800.0), 900.0, 2.0), (200, 200));
    assert_eq!(cg_point(200, 200, 2.0), (100.0, 100.0));
}
#[test] fn plist_and_desktop_round_trip() {
    let p = Path::new("/Applications/PixelPet.app/Contents/MacOS/PixelPet");
    assert_eq!(program_from_plist(&launch_agent_plist(p)).as_deref(), Some(p));
    assert!(launch_agent_plist(p).contains("<string>com.pixelpet.app</string>"));
    let q = Path::new("/home/a b/Apps/PixelPet-x86_64.AppImage");
    assert_eq!(exec_from_desktop(&autostart_desktop(q)).as_deref(), Some(q));
}
#[test] fn appimage_autostart_uses_appimage_var() {
    let env = |k: &str| (k == "APPIMAGE").then(|| "/home/u/PixelPet-x86_64.AppImage".to_string());
    assert_eq!(autostart_target(Path::new("/tmp/.mount_PixelAbc/usr/bin/PixelPet"), &env).unwrap(),
               PathBuf::from("/home/u/PixelPet-x86_64.AppImage"));
}
#[test] fn translocated_mac_path_refuses_autostart() {
    let p = Path::new("/private/var/folders/x/AppTranslocation/1A2B/d/PixelPet.app/Contents/MacOS/PixelPet");
    assert_eq!(autostart_target(p, &|_| None), Err("move PixelPet to Applications first"));
}
#[test] fn second_lock_fails_first_drop_frees_it() { /* claim, claim again -> None, drop, claim -> Some */ }
#[test] fn lock_is_released_when_holder_dies() {
    // spawn `cargo`-built helper or `std::process::Command::new(current_exe)` with an env flag that
    // claims the lock and exits without dropping it; then Lock::claim(path) must be Some.
}
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test --test platform_pure`
Expected: FAIL, unresolved `pixelpet::platform::pure`.

- [ ] **Step 3: Implement `pure.rs` and `lock.rs`** with the signatures above. Point maths: `x_px = round(x_pt * scale)`, `y_px = round((primary_h - y_pt - h_pt) * scale)` for rects, `(primary_h - y_pt)` for points.

- [ ] **Step 4: Run the tests and the whole suite**

Run: `cargo test`
Expected: all pass, 1 ignored.

- [ ] **Step 5: Commit**

```bash
git add src/platform/pure.rs src/platform/lock.rs src/platform/mod.rs tests/platform_pure.rs
git commit -m "platform: pure mac coords, autostart files, single-instance lock"
```

---

### Task 3: Linux window — X11 ARGB window with click-through and a no-compositor fallback

**Files:**
- Create: `src/window/linux.rs`, `src/window/shape.rs`
- Modify: `src/window/mod.rs` (Linux uses `linux::PetWindow`), `src/app.rs` (X11 backend; click-through toggle), `Cargo.toml` (`[target.'cfg(target_os = "linux")'.dependencies] x11-dl = "=2.21.0"`)
- Test: `tests/shape.rs` (new, runs everywhere)

**Interfaces:**
- Consumes: `Rgba` (straight alpha, RGBA, row-major).
- Produces, on every OS, one new method on `PetWindow`: `fn set_click_through(&self, through: bool)`. Windows: no-op (the layered window does it per pixel). Linux/macOS: winit `set_cursor_hittest(!through)`.
- Produces in `window::shape`: `fn shape_runs(img: &Rgba, min_alpha: u8) -> Vec<(i32, i32, i32, i32)>` — one `(x, y, w, 1)` per horizontal run of pixels with alpha ≥ `min_alpha` (use 128).
- `PetWindow::new`/`present`/`id`/`hide`/`show`/`focus` keep their exact Windows signatures.

Decisions:
- Linux always uses the X11 backend: build the event loop with `winit::platform::x11::EventLoopBuilderExtX11::with_x11()` in `app::run`. On Wayland that runs through XWayland (§7). With no `DISPLAY`, exit with the message `PixelPet needs X11 or XWayland`.
- Window: `with_transparent(true)` (winit picks the 32-bit visual), `with_decorations(false)`, `AlwaysOnTop`, `with_x11_window_type(vec![XWindowType::Utility])`, then add `_NET_WM_STATE_SKIP_TASKBAR` and `_NET_WM_STATE_SKIP_PAGER` with a `_NET_WM_STATE` client message.
- Present: premultiply into BGRA (the X ARGB visual's byte order on little-endian) in a kept buffer, `XPutImage` through a GC made once; move and resize with winit `set_outer_position`/`request_inner_size` (physical), only when they change. Opacity: `_NET_WM_WINDOW_OPACITY` (`0xffffffff * opacity / 255`).
- No compositor (no owner of `_NET_WM_CM_S0`): set the XShape bounding region from `shape_runs` on each present whose image changed; opacity is then ignored. Re-check the owner on the 2 s poll.
- App: each tick, after `inp` is built, `win.set_click_through(!(inp.over_pet || inp.over_toy || self.button_down))`; the bubble window is hittable while `over_bubble`; the menu window is always hittable.

- [ ] **Step 1: Write the failing test** `tests/shape.rs`

```rust
#[test] fn shape_runs_cover_only_opaque_pixels() {
    // 4x2: row 0 = [opaque, opaque, clear, opaque]; row 1 = [clear, half(127), half(128), clear]
    assert_eq!(shape_runs(&img, 128), vec![(0,0,2,1), (3,0,1,1), (2,1,1,1)]);
}
#[test] fn empty_image_has_no_runs() { assert!(shape_runs(&Rgba::new(3, 3), 128).is_empty()); }
```

- [ ] **Step 2: Run to see it fail** — `cargo test --test shape` → FAIL, no `shape_runs`.
- [ ] **Step 3: Implement `shape.rs`, then `linux.rs` and the app changes** as decided above.
- [ ] **Step 4: Run the suite on Windows and on CI**

Run: `cargo test` locally, then `git push; gh run watch --exit-status`
Expected: all green; `test-linux` compiles the real Linux window.

- [ ] **Step 5: Commit**

```bash
git add src/window/linux.rs src/window/shape.rs src/window/mod.rs src/app.rs Cargo.toml Cargo.lock tests/shape.rs
git commit -m "window: Linux X11 pet window with click-through and a shape fallback"
```

---

### Task 4: Linux platform, and the screenshot smoke test

**Files:**
- Create: `src/platform/linux.rs`, `tools/ci/smoke-linux.sh`, `tools/ci/check_shot.py`
- Modify: `src/platform/mod.rs` (`native()` returns `linux::X11` on Linux), `.github/workflows/ci.yml` (`test-linux` runs the smoke)
- Test: `#[cfg(target_os = "linux")]` tests in `src/platform/linux.rs`, run on CI under Xvfb

**Interfaces:**
- Consumes: `pure::autostart_desktop`, `pure::exec_from_desktop`, `pure::autostart_target` (Task 2).
- Produces: `pub struct X11` implementing `Platform`, opened once with `XOpenDisplay`; every method returns `None`/`false` instead of panicking if the display or an extension is missing.

Per §8: `cursor_pos` = `XQueryPointer` on the root; `warp_cursor` = `XWarpPointer`; `work_area` = `_NET_WORKAREA` for desktop 0 clipped to the primary monitor from `XRRGetMonitors` (fallback: whole root minus 48 px); `idle_seconds` = XScreenSaver `XScreenSaverQueryInfo` (`x11_dl::xss`, loaded at run time; missing → `None`); `fullscreen_app` = `_NET_ACTIVE_WINDOW`'s `_NET_WM_STATE` contains `_NET_WM_STATE_FULLSCREEN` (`None` if no active window); `esc_down` = `XQueryKeymap` bit for `XKeysymToKeycode(XK_Escape)` only; `autostart`/`set_autostart` read/write `$XDG_CONFIG_HOME/autostart/pixelpet.desktop` (fallback `~/.config/autostart/`) with `autostart_target`, and `set_autostart(None)` deletes it.

Smoke test (`smoke-linux.sh MODE`, MODE = `composited` or `bare`): start `Xvfb :99 -screen 0 1920x1080x24`, a solid `xsetroot -solid "#3a6ea5"` background, `xcompmgr` only in `composited` mode; run `PIXELPET_PIN=stand PIXELPET_EXIT_AFTER=8 ./target/debug/PixelPet &`; at 5 s `import -window root shot-$MODE.png`; wait for exit code 0. `check_shot.py shot.png` (stdlib + `zlib` PNG reader, no Pillow) passes when: some pixels in the bottom 300 rows differ from `#3a6ea5` (the pet is there), the top-left 100×100 is all `#3a6ea5` (nothing else drew), and in `composited` mode the pixels 2 px outside the pet's bounding box are background (transparency works, no black box). Upload both PNGs as the `linux-shots` artifact.

- [ ] **Step 1: Write the Linux-only tests** in `src/platform/linux.rs`: `work_area_is_inside_the_root`, `cursor_is_known`, `warp_moves_the_cursor`, `autostart_round_trip_in_a_temp_home` (sets `XDG_CONFIG_HOME` to a temp dir).
- [ ] **Step 2: Push, see them fail on CI** — `test-linux` red (no `X11` yet).
- [ ] **Step 3: Implement `X11`, the two scripts, and the CI steps** (tests run under `xvfb-run -a cargo test`).
- [ ] **Step 4: Push and look at the pictures**

Run: `git push; gh run watch --exit-status; gh run download --name linux-shots -D C:/Users/rushi/AppData/Local/Temp/claude/pixelpet-shots`
Expected: green, and Read both PNGs: Alex standing on the ground, no black rectangle in `composited`, hard-edged but whole in `bare`. A pass that shows no pet is a fail.

- [ ] **Step 5: Commit**

```bash
git add src/platform/linux.rs src/platform/mod.rs tools/ci/smoke-linux.sh tools/ci/check_shot.py .github/workflows/ci.yml
git commit -m "platform: Linux X11, with a screenshot smoke test on CI"
```

---

### Task 5: macOS window — non-opaque NSWindow showing a CGImage

**Files:**
- Create: `src/window/macos.rs`, `tools/ci/smoke-macos.sh`
- Modify: `src/window/mod.rs`, `src/app.rs` (accessory policy), `Cargo.toml` (`[target.'cfg(target_os = "macos")'.dependencies]` `objc2`, `objc2-foundation`, `objc2-app-kit`, `core-graphics`, `core-foundation`, each `=` the `Cargo.lock` version, with only the features needed), `.github/workflows/ci.yml` (`test-macos` runs the smoke)
- Test: the CI screenshot

**Interfaces:**
- Consumes: `PetWindow` API and `set_click_through` from Task 3.
- Produces: `macos::PetWindow` with that same API.

Decisions:
- Event loop: `EventLoopBuilderExtMacOS::with_activation_policy(ActivationPolicy::Accessory)` (no Dock icon; the bundle also sets `LSUIElement`, Task 8).
- Window: `with_transparent(true)`, no decorations, `AlwaysOnTop` (floating level), `with_has_shadow(false)`; on the `NSWindow`: `setOpaque(false)`, clear background, `setCollectionBehavior(CanJoinAllSpaces | Stationary | IgnoresCycle)`. Fullscreen "stay" adds `FullScreenAuxiliary` (§8), set from `Settings.fullscreen` when the window is made and when the setting changes.
- Present: build a `CGImage` (8-bit RGBA, premultiplied-last, sRGB) from a kept premultiplied buffer; `view.setWantsLayer(true)`, `layer.setContents(image)`, `layer.setMagnificationFilter(nearest)` so the pixels stay hard; move/resize with winit physical position/size only when they change; opacity with `NSWindow setAlphaValue(opacity / 255)`.
- Focus: the menu window must become key so Esc and focus-lost close it (winit's window class allows key status when borderless; verify in the smoke via the `menu-focus` script action).

Smoke (`smoke-macos.sh`): `PIXELPET_PIN=stand PIXELPET_EXIT_AFTER=8 ./target/debug/PixelPet &`, `screencapture -x shot-mac.png` at 5 s, then `PIXELPET_SCRIPT="2:menu-focus" PIXELPET_EXIT_AFTER=6` with a shot at 4 s; exit codes 0. `check_shot.py --no-bg-colour` (the runner's wallpaper is not ours) checks only that the bottom 300 rows changed between a shot with the pet and one taken after it exits. Upload as `mac-shots`. If the runner's `screencapture` returns a blank image (it may lack Screen Recording permission), record that in `docs/gotchas.md`, keep the exit-code check, and leave the visual check to the real-Mac checklist.

- [ ] **Step 1: Add the smoke to CI first, so it fails** — `gh workflow run ci.yml`; `test-macos` red (`PetWindow` unsupported).
- [ ] **Step 2: Implement `macos.rs` and the app/Cargo changes**; locally `rustup target add aarch64-apple-darwin; cargo check --target aarch64-apple-darwin` (if it cannot check without the SDK, rely on CI).
- [ ] **Step 3: Run it on CI and look**

Run: `gh workflow run ci.yml --ref <branch>; gh run watch --exit-status; gh run download --name mac-shots -D C:/Users/rushi/AppData/Local/Temp/claude/pixelpet-shots`
Expected: green; Read the PNGs: Alex at the bottom with no box around him, and the pixel menu open.

- [ ] **Step 4: Confirm the Windows suite is untouched** — `cargo test` locally, all pass.
- [ ] **Step 5: Commit**

```bash
git add src/window/macos.rs src/window/mod.rs src/app.rs Cargo.toml Cargo.lock tools/ci/smoke-macos.sh .github/workflows/ci.yml
git commit -m "window: macOS pet window on a layer, with a screenshot smoke test"
```

---

### Task 6: macOS platform

**Files:**
- Create: `src/platform/macos.rs`
- Modify: `src/platform/mod.rs` (`native()` returns `macos::Mac`)
- Test: `#[cfg(target_os = "macos")]` tests in `src/platform/macos.rs`, run on the macOS job

**Interfaces:**
- Consumes: `pure::mac_rect`, `pure::mac_point`, `pure::cg_point`, `pure::launch_agent_plist`, `pure::program_from_plist`, `pure::autostart_target` (Task 2).
- Produces: `pub struct Mac` implementing `Platform`.

Per §8, with no permission prompts: `cursor_pos` = `NSEvent::mouseLocation` → `mac_point` with the primary screen (`NSScreen::screens()[0]`) height and `backingScaleFactor`; `warp_cursor` = `CGWarpMouseCursorPosition(cg_point(..))`, returning false on a non-zero error; `work_area` = primary `visibleFrame` → `mac_rect`; `idle_seconds` = `CGEventSourceSecondsSinceLastEventType(CombinedSessionState, kCGAnyInputEventType)` (no permission needed); `fullscreen_app` = `CGWindowListCopyWindowInfo(OnScreenOnly | ExcludeDesktopElements)`: any layer-0 window not owned by our pid whose `kCGWindowBounds` equals the primary screen's full frame → `Some(true)`, a readable list without one → `Some(false)`, failure → `None`; `esc_down` = **always `false`** (reading keys needs Input Monitoring, which is the prompt Rushi does not want; right-click still ends play); `autostart`/`set_autostart` read/write `~/Library/LaunchAgents/com.pixelpet.app.plist` via `autostart_target`, deleting the file for `None`. A refused target (`Err`) returns `io::ErrorKind::Other` with the message, which the menu already shows on failure.

- [ ] **Step 1: Write the macOS-only tests** in `src/platform/macos.rs`: `work_area_is_inside_the_main_screen`, `cursor_is_known`, `idle_is_small_while_testing`, `fullscreen_is_known`, `autostart_round_trip_in_a_temp_home` (sets `HOME`).
- [ ] **Step 2: Run on CI, see them fail** — `gh workflow run ci.yml`; red.
- [ ] **Step 3: Implement `Mac`.**
- [ ] **Step 4: Run on CI** — green, and the Task 5 shots still show the pet on the ground line (not floating above the Dock gap or below the screen).
- [ ] **Step 5: Commit**

```bash
git add src/platform/macos.rs src/platform/mod.rs
git commit -m "platform: macOS cursor, work area, idle, fullscreen, autostart"
```

---

### Task 7: Esc and dev switches across platforms — check the seams

**Files:**
- Modify: `src/app.rs` only if a seam is found
- Test: `tests/brain.rs` (add one case)

**Interfaces:**
- Consumes: `Fake` platform, `Inputs`, `Brain` (existing).

The brain already handles a platform that can't warp (`can_warp`) and one with no idle seconds. M3 adds the first platform where Esc never reads true and where `cursor_pos` can be `None` mid-session (XWayland when the pointer is over a native Wayland window).

- [ ] **Step 1: Write the failing tests** in `tests/brain.rs`

```rust
#[test] fn play_ends_by_right_click_when_esc_is_never_seen() { /* Fake.esc stays false; a tick with Inputs.cancel = true (what right-click sets) ends fetch; show.playing == false */ }
#[test] fn cursor_vanishing_mid_drag_drops_the_pet_on_the_ground() { /* Down over pet, Move, then cursor None for 1 s: pet lands, not stuck lifted */ }
```

- [ ] **Step 2: Run** — `cargo test --test brain`; note which fail (either may already pass; a pass is kept as a pin).
- [ ] **Step 3: Fix in the brain/app only what failed.**
- [ ] **Step 4: Run the suite** — `cargo test`, all pass.
- [ ] **Step 5: Commit**

```bash
git add tests/brain.rs src/app.rs src/brain/mod.rs
git commit -m "brain: no-Esc and lost-cursor platforms"
```

---

### Task 8: Release builds — exe, dmg, AppImage

**Files:**
- Create: `.github/workflows/release.yml`, `tools/ci/make-app.sh`, `tools/ci/make-appimage.sh`, `tools/ci/Info.plist`, `tools/ci/pixelpet.desktop`, `docs/release.md`
- Modify: `examples/make_icon.rs` (also write `icon_16…1024.png`)
- Test: a tag on a throwaway version, `v0.1.0-rc1`

**Interfaces:**
- Consumes: `examples/make_icon.rs`, the smoke scripts from Tasks 4 and 5.

On tag `v*`, three jobs, then one job that makes a **draft** release with the three files:
- **windows** (`windows-latest`): target `x86_64-pc-windows-msvc`, `RUSTFLAGS=-C target-feature=+crt-static`, `cargo build --release`, upload `PixelPet.exe`; fail if > 6 MB; check `dumpbin /dependents` lists no `VCRUNTIME140.dll`.
- **macos** (`macos-latest`): build `x86_64-apple-darwin` and `aarch64-apple-darwin`, `lipo -create`; `make-app.sh` lays out `PixelPet.app/Contents/{MacOS/PixelPet, Info.plist, Resources/PixelPet.icns}` (`iconutil` from the PNGs); `Info.plist` sets `CFBundleIdentifier com.pixelpet.app`, `LSUIElement true`, `CFBundleShortVersionString` from the tag; `hdiutil create -volname PixelPet -srcfolder` the .app plus an `Applications` symlink → `PixelPet.dmg`. Check `lipo -archs` prints both archs, and run the Task 5 smoke against the bundled binary.
- **linux** (`ubuntu-latest`, `container: debian:bullseye` for an old glibc): build release, `make-appimage.sh` makes `AppDir/usr/bin/PixelPet`, `pixelpet.desktop`, `pixelpet.png`, `AppRun`, then `appimagetool --appimage-extract-and-run` → `PixelPet-x86_64.AppImage`. Bundle no X11 libraries (winit loads the system's at run time). Run the Task 4 smoke against the AppImage (`--appimage-extract-and-run`).

`docs/release.md`: what each file is, and the one-time first launch for an unsigned app — Windows: *More info → Run anyway*; macOS: right-click → *Open* → *Open* (and drag to Applications first, or launch-at-startup refuses); Linux: `chmod +x`, then run. Plus **the real-machine checklist still owed** before a release is called done (§11.6): on a real Mac and a real Linux desktop — the pet stands on the ground, clicks pass through around it, drag, the bubble, the menu opens and closes with Esc, pet/fetch/feed, fullscreen hide, launch at startup survives a reboot.

- [ ] **Step 1: Write `release.yml`, the scripts and `docs/release.md`.**
- [ ] **Step 2: Tag a release candidate**

Run: `git tag v0.1.0-rc1; git push origin v0.1.0-rc1; gh run watch --exit-status`
Expected: all jobs green; `gh release view v0.1.0-rc1` lists `PixelPet.exe`, `PixelPet.dmg`, `PixelPet-x86_64.AppImage`, as a draft.

- [ ] **Step 3: Run the release exe on this laptop** (Rushi quits the running PixelPet first; the Start menu exe is locked). Download it with `gh release download`, run it, screenshot it next to the photos as in M1. Expected: the same pet as the GNU build.
- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml tools/ci examples/make_icon.rs docs/release.md
git commit -m "release: exe, dmg and AppImage from one tag"
```

---

## Not in M3

- Running on a real Mac or a real Linux desktop: **owed, not done**. Rushi has neither (2026-10-21), so M3 is "built and smoke-tested on CI", not "released". The checklist is in `docs/release.md`.
- Native Wayland (no XWayland), code signing and notarisation, multiple monitors: later, per the spec.
- The deferred M2 minors (dev switches in release, unused tug anims, CPU during play, click mid-feed): untouched. The smoke tests need `PIXELPET_PIN`/`EXIT_AFTER`/`SCRIPT`, so they stay for now.
