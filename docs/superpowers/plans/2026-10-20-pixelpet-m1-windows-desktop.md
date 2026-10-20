# PixelPet Milestone 1: Alex on the Windows desktop — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Alex and Alex the bun, drawn by a Rust port of the approved rig,
living on the Windows desktop. They wander, nap when you're idle, look at you
on hover, can be dragged, respect fullscreen apps, and are set up from a
right-click pixel menu.

**Architecture:** A pure-Rust `rig` renders a skeleton to RGBA pixels and is
proven identical to the approved JS drafts by golden-frame tests. A pure
`brain` decides what the pet does from timestamped inputs. `platform` wraps
the few OS calls. `window` shows frames in a per-pixel-alpha layered window,
so clicks pass through transparent pixels natively. `app` wires them on a
winit event loop.

**Tech Stack:** Rust stable (2021 edition); `winit` 0.30; `windows-sys`
(Windows only, same version winit resolves to). No other runtime crates.

**Spec:** `docs/superpowers/specs/2026-10-20-pixelpet-v1-design.md`. Read it
first. The JS drafts in `drafts/looks/` (`rig.js`, `rabbit.js`) are the
source of truth for every rig number.

**Roadmap (separate plans, written when this one is done):**
- M2: bubble menu, Pet / Fetch / Tug / Feed, new anims.
- M3: macOS and Linux windows and platform code, CI release builds, packaging.

## Global Constraints

- Runtime dependencies: `winit` and, on Windows only, `windows-sys`. Nothing else ships. Dev-dependencies are allowed.
- No network access anywhere in the program.
- Never commit anything under `alex/`, `alex the bun/`, or `drafts/looks/compare-*.png` (personal photos). `.gitignore` already covers them.
- Copy about Alex the bun uses no gendered pronouns.
- No hunger, sickness, sadness or guilt mechanics.
- One name per thing: use the spec §3 glossary terms in code (`pet`, `species`, `coat`, `look`, `rig`, `skeleton`, `anim`, `brain`, `bubble`, `menu`, `toy`, `ground`, `depth`).
- Coordinates in `brain`, `platform` and `window` are **physical pixels**. Rig coordinates are rig units (withers or head height = 1, y up).
- Anim time steps at 12 fps (`FPS = 12`); a change of anim blends over 0.25 s in 3 steps (`floor(k*3)/3`), as on the look sheet.
- Settings defaults: `pet=alex size=M opacity=100 fullscreen=hide fullscreen_opacity=50 autostart=off`. Size zoom for look B: S=2, M=3, L=4, XL=5.
- Idle nap after 180 s without input.
- Local toolchain: `stable-x86_64-pc-windows-gnu`, installed at `~/.cargo/bin`.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Display scaling at 125 %/150 %.** The ground line and pet size must stay right. Test in Task 6: a fake work area in physical px with `unit_px` scaled; the pet's feet land on `work.y + work.h`.
2. **Taskbar not at the bottom, or auto-hidden.** The ground is still the work-area bottom, with no gap and no overlap. Test in Task 6: work areas `{0,0,1920,1040}` and `{0,40,1920,1040}` both put feet at the rect bottom.
3. **A hand-edited settings file** (BOM, CRLF, blank lines, `opacity = 60`, unknown keys). It must still load with sensible values and keep unknown keys. Test in Task 4.
4. **The program moved after enabling launch on startup.** The entry follows the program. Test in Task 4 (`autostart_needs_update`).
5. **Resolution change, monitor unplug, or resume from sleep while running.** No teleporting or walking off-screen. Test in Task 6: work area shrinks, so x is clamped; a 600 s time jump moves the pet at most one 0.1 s step.

---

### Task 1: Spike — a see-through pet window on Windows

Proves the one risky assumption before anything else is built: a winit window
switched to a layered window, fed by `UpdateLayeredWindow`, shows per-pixel
alpha, lets clicks through transparent pixels, stays on top, and has no taskbar entry.

**Files:**
- Create: `Cargo.toml`, `src/lib.rs`, `src/main.rs` (prints "PixelPet" and exits for now)
- Create: `src/window/mod.rs`, `src/window/windows.rs`
- Create: `examples/spike_window.rs`
- Create: `tools/screenshot.ps1`

**Interfaces:**
- Produces: `window::Rgba { w: usize, h: usize, px: Vec<u8> }` (straight-alpha RGBA, row-major).
- Produces: `window::PetWindow::new(el: &winit::event_loop::ActiveEventLoop, focusable: bool) -> Result<PetWindow, String>`.
- Produces: `PetWindow::present(&mut self, img: &Rgba, x: i32, y: i32, opacity: u8) -> Result<(), String>`. Places the top-left at physical `(x, y)`.
- Produces: `PetWindow::id(&self) -> winit::window::WindowId`, `PetWindow::hide(&self)`, `PetWindow::show(&self)`.
- Produces: `PetWindow::hwnd(&self) -> isize` (Windows only, used by the spike's self-test).

- [ ] **Step 1: Set up the crate.** Package `pixelpet`, edition 2021, a lib target, and a bin `[[bin]] name = "PixelPet" path = "src/main.rs"`. Deps: `winit = "0.30"`. Under `[target.'cfg(windows)'.dependencies]`, `windows-sys` with features `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Shell`, `Win32_System_Registry`, `Win32_System_SystemInformation`, `Win32_UI_Input_KeyboardAndMouse`. Pin the windows-sys version to the one in `cargo tree -i windows-sys`.
- [ ] **Step 2: Implement `PetWindow` in `src/window/windows.rs`.**
  - Create via winit with: no decorations, not resizable, `WindowLevel::AlwaysOnTop`, `with_skip_taskbar(true)`, `with_active(false)`, not transparent (we do transparency ourselves).
  - Then `SetWindowLongPtrW(GWL_EXSTYLE, ex | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST)`, plus `WS_EX_NOACTIVATE` when `focusable == false`.
  - `present` converts RGBA to premultiplied BGRA in a 32-bit top-down DIB section (`CreateDIBSection`, kept between calls and recreated only on size change). It calls `UpdateLayeredWindow` with `ULW_ALPHA`, `BLENDFUNCTION { AC_SRC_OVER, 0, opacity, AC_SRC_ALPHA }`, `pptDst = (x, y)` and `psize = (w, h)`.
  - `src/window/mod.rs` re-exports it under `#[cfg(windows)]`. On other OSes it exposes a `PetWindow` whose methods return `Err("unsupported in M1")`.
- [ ] **Step 3: Write `examples/spike_window.rs`.** It shows a 96×96 image: an opaque red disc of radius 40 with a 1 px dark outline, fully transparent outside, bobbing ±8 px at 1 Hz near the bottom-centre of the primary screen for 10 s. After the first present it self-tests with `WindowFromPoint`: the disc centre must return our hwnd and a transparent corner pixel must not. It prints `SPIKE PASS` or `SPIKE FAIL: <which>` and the window's ex-style bits.
- [ ] **Step 4: Write `tools/screenshot.ps1`** with params `-Out <png> [-X -Y -W -H]`. It captures the screen (or a region) with `System.Drawing.Graphics.CopyFromScreen` and saves a PNG. It needs no modules beyond .NET.
- [ ] **Step 5: Run and verify.**
  - Run `cargo run --example spike_window`. Expected output: `SPIKE PASS`.
  - While it runs, `powershell -File tools/screenshot.ps1 -Out spike.png`. Look at the image: only the disc is visible over the desktop, with no black or white box around it.
  - Check by hand that there's no taskbar button and that the disc stays above a maximised Notepad.
  - If anything fails, stop and report. The window approach in the spec §7 must be revisited before Task 6.
- [ ] **Step 6: Commit** — `git add Cargo.toml Cargo.lock src examples tools/screenshot.ps1 && git commit -m "spike: per-pixel-alpha layered pet window on Windows"`.

---

### Task 2: Rig core and the dog, proven against the JS drafts

**Files:**
- Create: `tools/golden.js` (node, no npm packages)
- Create: `tests/golden/cases.txt`, `tests/golden/*.ppg` (generated, committed)
- Create: `src/rig/mod.rs`, `src/rig/core.rs`, `src/rig/dog.rs`
- Create: `tests/parity.rs`, `tests/common/mod.rs` (golden loader + diff writer)
- Modify: `src/lib.rs` (`pub mod rig;`)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces (in `rig`):
  - `pub enum Pet { Dog, Rabbit }`
  - `pub enum Anim { Stand, Walk, Sit, Tilt, Hop, Sleep }` plus `Anim::from_name(&str) -> Option<Anim>` (`"stand"`, …).
  - `pub enum Toy { Ball, Coconut, Football }` plus `Toy::from_name(&str)` (`"ball"`, `"coconut"`, `"football"`).
  - `pub struct Look { pub name: &'static str, pub r: f64, pub zoom: u32, pub tones: u8, pub soft_outline: bool, pub min_r: f64, pub noise: f64, pub dither: f64, pub detail: u8 }` and `pub fn look(name: &str) -> &'static Look` for `"chunky" | "classic" | "detailed"`.
  - `pub struct Frame { pub w: usize, pub h: usize, pub ox: i32, pub gy: i32, pub rgba: Vec<u8> }`
  - `pub enum Skel { Dog(dog::Skel), Rabbit(rabbit::Skel) }` (the rabbit variant is filled in Task 3).
  - `pub fn pose(pet: Pet, anim: Anim, look: &Look, t: f64, toy: Option<Toy>) -> Skel`
  - `pub fn lerp(a: &Skel, b: &Skel, k: f64) -> Skel` (panics if species differ)
  - `pub fn render(s: &Skel, look: &Look, collar: bool) -> Frame`
  - `pub fn render_toy(toy: Toy, look: &Look) -> Frame`
  - `pub fn period(pet: Pet, anim: Anim) -> f64`
  - `pub fn speed(pet: Pet, look: &Look, anim: Anim) -> f64` (rig units/s; 0 when the anim doesn't travel)
  - `pub fn move_profile(pet: Pet, anim: Anim, phase: f64) -> f64` (1.0 for constant speed)

- [ ] **Step 1: Write `tools/golden.js`.**
  - It `require`s `../drafts/looks/rig.js` and `../drafts/looks/rabbit.js` and renders every case in a `CASES` array.
  - It writes `tests/golden/<name>.ppg` files: 4-byte magic `PPG1`, then little-endian `u32 w, u32 h, i32 ox, i32 gy`, then `w*h*4` RGBA bytes.
  - It writes `tests/golden/cases.txt` with one line per case: `name|pet|look|anim|t|toy|collar|lerp`. Here `toy` is `-` or a toy name, `collar` is `0`/`1`, and `lerp` is `-` or `anim2:t2:k`, meaning `lerpSkel(pose(anim,t), pose(anim2,t2), k)`.
  - Cases for `dog` in looks `classic` and `detailed`:
    - stand 0, 0.7
    - walk 0, 0.225, 0.45, 0.675
    - sit 0.3
    - tilt 0.2, 1.5, 5.5
    - hop 0.15, 0.5, 0.85
    - sleep 1, 7.8
    - walk 0.1 + ball; stand 0.3 + coconut; stand 0 with collar
    - walk 0.3 lerp `sleep:1:0.3333333333333333`
  - Toy cases: `render_toy` ball, coconut, football in classic, written as names `toy-<name>-classic`.
  - Rabbit cases are added to this same file in Task 3.
- [ ] **Step 2: Generate the goldens** with `node tools/golden.js`. Expected: `wrote N cases` and the files present (about 40 dog and toy files).
- [ ] **Step 3: Write the failing tests.**
  - `tests/parity.rs::dog_frames_match_js`: for every `pet=dog` line in `cases.txt`, render with the Rust rig and compare to the golden. Assert `w, h, ox, gy` are equal and that differing pixels are ≤ 0.5 % of `w*h`. On any difference it prints `name: N px differ` and writes `target/parity/<name>-diff.ppm` (golden | rust | diff in red).
  - `toys_match_js`: the same for the toy cases.
  - Unit tests in `src/rig/core.rs`:
    - `hash_matches_js`: `hash(0,0) == -1.0`, `hash(1,0) == -0.0811339198435973`, `hash(-5,17) == 0.47409579667644186`, `hash(123,-456) == 0.11827956989247301`, `hash(40000,-70000) == -0.33333333333333337`.
    - `js_round_is_half_up`: `js_round(-2.5) == -2.0`, `js_round(2.5) == 3.0`.
    - `frame_sizes`: chunky `(38,33,19,31)`, classic `(69,60,35,58)`, detailed `(112,98,58,96)` as `(W,H,ox,gy)`.
- [ ] **Step 4: Run `cargo test` and confirm it fails** (rig functions not defined).
- [ ] **Step 5: Port `rig.js` to `src/rig/core.rs` (shared) and `src/rig/dog.rs` (dog).**
  - Port function by function, keeping the JS names in snake_case: `ik`, `geom`, `build`, `tailChain`, the six `ANIMS`, `sitSkel`, `shapes`, `hit`, `paint`, `toneBand`/`toneOf`, `bbox`, `raster`, `toRGBA` (despeckle, edge separation, outline), `drawEyes`, `frameSize`, `renderToy`.
  - Palettes and family order are copied exactly (`FAMS` order, `TOYS`, `ALEX`).
  - Shapes are `struct Shape { kind: Kind, part: Part, grp: i8, far: bool, flip: bool, idx: u8, no_edge: bool, collar: bool }` with `enum Kind { Cap{a,b,ra,rb}, Ell{c,rx,ry,ang}, Tri{p:[P;3]} }`. `Part` is an enum of every part name used in the JS.
  - Four exactness rules:
    - `hash` uses `i32::wrapping_mul` and `wrapping_add`, and for `>>>` it uses `((h as u32) >> n) as i32`.
    - Every JS `Math.round` becomes `js_round(x) = (x + 0.5).floor()`.
    - Integer pixel maths uses `i32` with `floor`/`ceil` exactly as the JS does.
    - `lerp` follows `lerpSkel`: numbers mix; `head_front`, `eyes` and `toy` switch at `k < 0.5`.
- [ ] **Step 6: Run `cargo test`.** Expected: all pass. Any case that differs by more than 0 px but within 0.5 % is listed in the output; report the list.
- [ ] **Step 7: Commit** — `git add tools/golden.js tests src && git commit -m "rig: Rust port of the dog rig, matched to the approved JS frames"`.

---

### Task 3: The rabbit and toys, proven against the JS drafts

**Files:**
- Create: `src/rig/rabbit.rs`
- Modify: `tools/golden.js` (rabbit cases), `src/rig/mod.rs` (dispatch to rabbit), `tests/parity.rs`

**Interfaces:**
- Consumes: everything Task 2 produced; `core` helpers (`ell`, `cap`, `put`, `lerp` mixing, `Hit`).
- Produces: `Skel::Rabbit(rabbit::Skel)` working through every `rig` function from Task 2. `speed(Pet::Rabbit, _, Anim::Walk) == 0.72`, and `move_profile(Pet::Rabbit, Anim::Walk, p)` returns `1/0.54` when `0.06 < p < 0.6`, else `0`.

- [ ] **Step 1: Add rabbit cases to `tools/golden.js`** for looks `classic` and `detailed`:
  - stand 0, 1.05, 4.0
  - walk 0.14, 0.26, 0.4
  - sit 0.2, 0.6
  - tilt 0.5, 1.5
  - hop 0.2, 0.5
  - sleep 1, 7.7
  - walk 0.3 lerp `sleep:1:0.6666666666666666`

  Re-run `node tools/golden.js`.
- [ ] **Step 2: Write the failing test** `tests/parity.rs::rabbit_frames_match_js` (same rules as the dog test), plus `rabbit_walk_moves_in_bursts`, which asserts the speed and profile values above.
- [ ] **Step 3: Run `cargo test rabbit` and confirm it fails.**
- [ ] **Step 4: Port `rabbit.js`** to `src/rig/rabbit.rs`:
  - `BUN` palette with `edgeDrop = 2`
  - `BODIES`
  - `loaf`, `blend` (smoothstep), `twitch`, `blink`, `rotAll`
  - the six anims, `shapes`, `paint`, `eyes`

  Same exactness rules as Task 2.
- [ ] **Step 5: Run `cargo test`.** Expected: all parity tests pass.
- [ ] **Step 6: Commit** — `git commit -am "rig: rabbit port (Alex the bun), matched to the approved JS frames"` (add the new files first).

---

### Task 4: Settings file

**Files:**
- Create: `src/settings.rs`, with tests in the same file
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `pub enum PetChoice { Alex, AlexTheBun }` with `fn pet(&self) -> rig::Pet`
  - `pub enum Size { S, M, L, XL }` with `fn zoom(&self) -> u32` (2/3/4/5)
  - `pub enum FullscreenMode { Hide, Stay }`
  - `pub struct Settings { pub pet: PetChoice, pub size: Size, pub opacity: u8, pub fullscreen: FullscreenMode, pub fullscreen_opacity: u8, pub autostart: bool, extra: Vec<(String, String)> }` with `Default`
  - `Settings::parse(text: &str) -> Settings`, `Settings::to_text(&self) -> String`
  - `pub enum Os { Windows, MacOs, Linux }`, `pub fn current_os() -> Os`
  - `pub fn settings_path(os: Os, exe: &Path, env: &dyn Fn(&str) -> Option<String>) -> PathBuf`
  - `pub fn load(path: &Path) -> Settings`, `pub fn save(path: &Path, s: &Settings) -> std::io::Result<()>` (write `settings.txt.tmp`, then rename)
  - `pub fn autostart_needs_update(current: Option<&Path>, exe: &Path, wanted: bool) -> bool`

- [ ] **Step 1: Write the failing tests.**
  - `defaults_when_empty`: `parse("")` equals `Settings::default()` with the Global Constraints defaults.
  - `round_trip`: a non-default value for every field gives `parse(to_text(s)) == s`.
  - `keeps_unknown_keys`: `"colour = red\npet = alex-the-bun\n"` gives `to_text` containing `colour = red`.
  - `tolerates_bom_crlf_blanks_spaces`: `"\u{feff}pet=alex-the-bun\r\n\r\n  size =  L \r\n"` gives `AlexTheBun`, `L`.
  - `bad_values_fall_back_per_key`: `size = huge` gives `M` while `pet = alex-the-bun` still applies.
  - `opacity_snaps_to_steps`: `60` → `50`, `0` → `25`, `130` → `100`, `75` → `75`.
  - `paths`:
    - Windows with `APPDATA=C:\\U\\AppData\\Roaming` → `C:\U\AppData\Roaming\PixelPet\settings.txt`
    - macOS with `HOME=/Users/u` → `/Users/u/Library/Application Support/PixelPet/settings.txt`
    - Linux with `XDG_CONFIG_HOME=/x` → `/x/pixelpet/settings.txt`
    - Linux with only `HOME=/home/u` → `/home/u/.config/pixelpet/settings.txt`
  - `portable_mode`: when `<exe dir>/settings.txt` exists (tempdir), `settings_path` returns it.
  - `autostart_needs_update`: `(None, exe, true)` → true; `(Some(exe), exe, true)` → false; `(Some(old), exe, true)` → true; `(Some(exe), exe, false)` → true; `(None, exe, false)` → false.
- [ ] **Step 2: Run `cargo test settings` and confirm it fails.**
- [ ] **Step 3: Implement.** Format: `key = value` per line; `#` starts a comment line; keys and values are trimmed. Written files start with `# PixelPet settings` and list known keys in spec order, then unknown keys.
- [ ] **Step 4: Run `cargo test settings` and confirm it passes.**
- [ ] **Step 5: Commit** — `git commit -m "settings: key = value file with portable mode"`.

---

### Task 5: Platform layer (Windows) and a fake for tests

**Files:**
- Create: `src/platform/mod.rs` (trait, `Rect`, `Fake`, `Stub`), `src/platform/windows.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }` with `fn bottom(&self) -> i32`
  - `pub trait Platform` with:
    - `cursor_pos(&self) -> Option<(i32,i32)>`
    - `warp_cursor(&self, x: i32, y: i32) -> bool`
    - `work_area(&self) -> Rect`
    - `idle_seconds(&self) -> Option<f64>`
    - `fullscreen_app(&self) -> Option<bool>`
    - `autostart(&self) -> Option<PathBuf>`
    - `set_autostart(&self, target: Option<&Path>) -> std::io::Result<()>`
  - `pub fn native() -> Box<dyn Platform>`: the Windows impl on Windows, otherwise `Stub` (no cursor, no warp, a work area of `{0,0,1920,1040}`, `None` idle and fullscreen, autostart unsupported → `Err`).
  - `pub struct Fake` with public `RefCell` fields mirroring each method's return, for tests.

- [ ] **Step 1: Write the failing tests** in `src/platform/windows.rs` (`#[cfg(all(test, windows))]`):
  - `work_area_is_inside_primary_monitor`: w and h > 0, and the rect lies inside `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`.
  - `idle_is_small_while_testing`: `idle_seconds()` is `Some(s)` with `0 ≤ s < 3600`.
  - `fullscreen_is_known`: `fullscreen_app()` is `Some(false)` while tests run in a normal console.
  - `autostart_round_trip`: using the value name `PixelPet-test` (via the inner fn `set_autostart_named`), set it to `C:\tmp\a b\PixelPet.exe`, read it back equal, delete it, read back `None`.
- [ ] **Step 2: Run `cargo test platform` and confirm it fails.**
- [ ] **Step 3: Implement the Windows calls.**
  - Cursor: `GetCursorPos`, `SetCursorPos`.
  - Work area: `MonitorFromPoint((0,0), MONITOR_DEFAULTTOPRIMARY)` + `GetMonitorInfoW` → `rcWork`.
  - Idle: `GetLastInputInfo` against `GetTickCount`, giving seconds.
  - Fullscreen: `SHQueryUserNotificationState`, where states `QUNS_BUSY(2)`, `QUNS_RUNNING_D3D_FULL_SCREEN(3)` and `QUNS_PRESENTATION_MODE(4)` count as fullscreen. Otherwise, true if the foreground window's rect equals its monitor's full rect and its class isn't `Progman` or `WorkerW`.
  - Autostart: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `PixelPet`, data = the quoted exe path, via `RegSetKeyValueW`, `RegGetValueW` and `RegDeleteKeyValueW`.
- [ ] **Step 4: Run `cargo test platform` and confirm it passes.**
- [ ] **Step 5: Commit** — `git commit -m "platform: Windows cursor, work area, idle, fullscreen, autostart"`.

---

### Task 6: Brain — what the pet does

Pure logic, no OS and no pixels. All scenarios run on a fake clock.

**Files:**
- Create: `src/brain/mod.rs` (`Brain`, `Inputs`, `Show`, `Mouse`), `src/brain/wander.rs` (script + xorshift RNG)
- Create: `tests/brain.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `rig::{Pet, Anim, period, speed, move_profile, look}`, `platform::Rect`, `settings::{Settings, FullscreenMode}`.
- Produces:
  - `pub enum Mouse { None, Down(i32,i32), Move(i32,i32), Up(i32,i32) }`
  - `pub struct Inputs { pub now: f64, pub work: Rect, pub unit_px: f64, pub cursor: Option<(i32,i32)>, pub over_pet: bool, pub mouse: Mouse, pub idle: Option<f64>, pub fullscreen: Option<bool> }`. `unit_px` = rig `R` × zoom, i.e. physical px per rig unit.
  - `pub struct Show { pub anim: Anim, pub anim_start: f64, pub x: f64, pub lift: f64, pub facing: i8, pub visible: bool, pub opacity: u8, pub zzz: bool, pub hover_for: f64 }`. `x` is the pet's ground x and `lift` is px above the ground, both physical. `opacity` is final, after fullscreen rules. `hover_for` is in seconds (M2 opens the bubble at ≥ 0.4).
  - `pub struct Brain`, with `Brain::new(pet: Pet, seed: u64, now: f64, work: Rect) -> Brain`, `Brain::update(&mut self, inp: &Inputs, s: &Settings) -> Show`, and `Brain::pin(&mut self, anim: Option<Anim>)` (holds one anim; used by the measure tool).

**Rules (decisions the tests pin):**
- **Wander:** weights walk 4, stand 1.4, sit 1.4, tilt 1.1, hop 1, sleep 0.8. The same anim never comes twice in a row except walk; sleep is always followed by stand. Durations in seconds: walk 3–6, stand 2.5–4, sit 3–5, tilt 4, hop 2.4, sleep 9.6–14.4. When walk starts, the pet turns around with probability 0.35.
- **Moving:** while `speed > 0`, `x += facing * speed * move_profile(phase) * unit_px * dt`, where `phase = ((now - anim_start) / period) % 1`. The pet turns at `1.2 * unit_px` from either work-area edge. `dt` is clamped to 0.1 s.
- **Idle nap:** when `idle ≥ 180`, the anim becomes sleep and stays until `idle < 1`. Then it's stand for 1.5 s, then walk towards `cursor.x` (facing set towards it) until within `1.5 * unit_px` of it, then wander resumes.
- **Hover:** while `over_pet` (and not dragging), the anim is tilt with `anim_start` = hover start, and `hover_for` grows. On leaving, the previous wander anim resumes with a fresh start time.
- **Drag:** `Down` on the pet then `Move` more than 4 px from the down point starts a drag. `x` follows `cursor.x`, `lift = max(0, work.bottom() - cursor.y - 0.3*unit_px)`, and the anim is stand. On `Up` it falls at 2400 px/s² until `lift = 0`, then stands for 1 s. A `Down` + `Up` within 4 px is not a drag.
- **Fullscreen:** with `fullscreen == Some(true)`, Hide → `visible=false`; Stay → `opacity = s.fullscreen_opacity`. Otherwise `opacity = s.opacity`.
- **Work area changes:** clamp `x` into `[1.2*unit_px, work.w - 1.2*unit_px] + work.x` every update.
- **`zzz`** is true while the anim is sleep.

- [ ] **Step 1: Write the failing tests** in `tests/brain.rs`. Each uses the same seed and steps `update` every 1/30 s.
  - `wander_never_repeats_except_walk`: over 2 h of simulated time, no two consecutive anims are equal unless both are walk, and every sleep is followed by stand.
  - `wander_weights_roughly_hold`: over 10 h, the walk share of anim starts is between 0.30 and 0.45.
  - `walking_turns_at_edges`: x always stays within the bounds above, and facing flips at least once in 120 s.
  - `rabbit_moves_only_in_bursts`: with `Pet::Rabbit` pinned to walk, x is constant whenever `move_profile` is 0.
  - `nap_after_three_idle_minutes`: idle 179 s → not sleep; 181 s → sleep; then idle 0 → stand within 0.1 s, then walk towards the cursor.
  - `hover_looks_and_counts`: `over_pet` true for 1 s → anim tilt, `hover_for ≈ 1.0` (±1/30).
  - `click_is_not_drag`: Down then Up within 3 px leaves x unchanged.
  - `drag_lifts_then_falls`: drag to 200 px above the ground → `lift ≈ 200 - 0.3*unit_px`. After Up, lift reaches 0 within `sqrt(2*lift/2400) + 0.05` s.
  - `fullscreen_hide_and_stay`: with Hide, visible is false; with Stay and `fullscreen_opacity=50`, visible is true and opacity is 50.
  - `ground_is_work_area_bottom` (Review Focus 1, 2): for work areas `{0,0,1920,1040}` and `{0,40,1920,1040}` at `unit_px` 66 and 99, `lift == 0` when not dragging. The app places feet at `work.bottom()`.
  - `clamps_on_work_area_change` (Review Focus 5): shrinking the work area from 2560 to 1280 wide puts x back in bounds on the next update.
  - `time_jump_is_one_step` (Review Focus 5): after a 600 s gap, |Δx| ≤ `speed * 0.1 * unit_px` (+1 px).
- [ ] **Step 2: Run `cargo test --test brain` and confirm it fails.**
- [ ] **Step 3: Implement** `Brain` and the wander script. The RNG is xorshift64* (own code; no `rand` crate). `Brain` owns the current anim, its start time, x, facing, lift, the drag state, the idle/wake state and the pinned anim.
- [ ] **Step 4: Run `cargo test --test brain` and confirm it passes.**
- [ ] **Step 5: Commit** — `git commit -m "brain: wander, idle nap, hover, drag, fullscreen rules"`.

---

### Task 7: The app — Alex on the real desktop

**Files:**
- Create: `src/app.rs` (winit `ApplicationHandler`), `src/view.rs` (frame cache, blend, compose, z's)
- Modify: `src/main.rs` (run the app), `src/lib.rs`
- Create: `tests/view.rs`

**Interfaces:**
- Consumes: `rig::*`, `brain::*`, `platform::native`, `settings::*`, `window::{PetWindow, Rgba}`.
- Produces:
  - `pub struct View`, with `View::new(pet: Pet, look: &'static Look)`, and `View::frame(&mut self, show: &Show, now: f64, zoom: u32, collar: bool) -> (&Rgba, i32, i32)`. It returns the composed image plus the offset from the pet's ground point (`x`, ground y) to the image's top-left.
  - `pub fn compose(frame: &rig::Frame, zoom: u32, facing: i8, zzz: &[(f64, f64)], margin_top: usize) -> Rgba`, where each z is `(age_seconds, x_offset_units)`.
  - The binary `PixelPet.exe` runs the pet. The env var `PIXELPET_PIN=<anim>` pins an anim (for measuring and screenshots).

**Rules:**
- Anim time `t = floor((now - anim_start) * 12) / 12`.
- On an anim change, blend from the last skeleton over 0.25 s in 3 steps with `rig::lerp`.
- Cache the rendered `rig::Frame` by key `(anim, t, blend step, toy, collar)` and re-render only when the key changes.
- Scale by whole-number zoom, nearest-neighbour; mirror when `facing < 0`.
- A z glyph is 4×4 (`####`, `..#.`, `.#..`, `####`) drawn white with a 1-art-px dark outline (`#24120A`). A new one spawns every 1.3 s while `zzz`; each lives 2.6 s, rises 0.7 units and drifts 0.25 units forward, fading in its last half.
- The window image has `margin_top` = 1.0 unit, so z's and hops fit.
- The window top-left = `(round(x) - ox*zoom, work.bottom() - gy*zoom - round(lift))`, snapped to whole art pixels in x.
- Present only when the image or the position changed. Pass `opacity` to `present` as `round(show.opacity * 255 / 100)`; `Show.opacity` is a percent.
- **Frame pacing:** request redraws at 12 Hz normally and 30 Hz while dragging or falling. Use `ControlFlow::WaitUntil`.
- **Inputs:** `over_pet` = a winit `CursorMoved` arrived within the last 150 ms and no `CursorLeft` since (the layered window only gets mouse events over opaque pixels). Mouse buttons map from `MouseInput`. Idle and fullscreen are polled every 2 s, the cursor every frame.
- **Startup:** load settings (Task 4 path). If `autostart_needs_update(...)`, apply it via the platform.

- [ ] **Step 1: Write the failing tests** in `tests/view.rs`:
  - `compose_scales_and_mirrors`: a 3×2 frame with a known pixel at (0,0) and zoom 3 gives a 9×(6+margin) image, with the pixel block at the top-left for facing 1 and the top-right for facing −1.
  - `compose_draws_z`: one z at age 0 lands its glyph's top-left white pixel at the expected spot above the head.
  - `cache_reuses_frames`: two `View::frame` calls with the same `Show` and `now` within the same 1/12 s step render once (count via a test hook).
  - `blend_takes_three_steps`: an anim change at `now=0` gives distinct frames at `now` = 0.0, 0.09, 0.17, then the target anim's own frame from 0.25.
- [ ] **Step 2: Run `cargo test --test view` and confirm it fails.**
- [ ] **Step 3: Implement** `view.rs`, then `app.rs` and `main.rs`.
- [ ] **Step 4: Run `cargo test` and confirm everything passes.**
- [ ] **Step 5: Run it for real and check it by eye.**
  - Run `cargo run --release`, then `PIXELPET_PIN=walk`, `sit`, `tilt`, `hop` and `sleep` one at a time, for both pets (switch with `pet=` in the settings file).
  - For each, take a region capture around the pet with `tools/screenshot.ps1`.
  - Compare side by side with the look sheet frames and the photos, as was done for the look sheet. Feet on the taskbar line, no box around the pet, z's above the head while sleeping.
  - Hover makes it look at you; dragging lifts it and it falls back; clicks on empty space next to the pet reach the app below.
  - Fix anything off, then re-shoot.
- [ ] **Step 6: Commit** — `git commit -m "app: the pet lives on the Windows desktop"`.

---

### Task 8: Right-click menu

**Files:**
- Create: `src/ui/mod.rs`, `src/ui/font.rs` (5×7 bitmap font for printable ASCII, as const data), `src/ui/menu.rs`
- Create: `tests/menu.rs`
- Modify: `src/app.rs` (open on right-click; second `PetWindow` with `focusable = true`)

**Interfaces:**
- Consumes: `settings::*`, `platform::Platform`, `window::{PetWindow, Rgba}`.
- Produces:
  - `pub enum Action { SetPet(PetChoice), SetSize(Size), SetOpacity(u8), SetFullscreen(FullscreenMode), SetFullscreenOpacity(u8), ToggleAutostart, Quit }`
  - `pub struct Menu`, with `Menu::new(s: &Settings) -> Menu`, `Menu::draw(&self, scale: u32) -> Rgba`, `Menu::hit(&self, x: i32, y: i32, scale: u32) -> Option<Action>` (x and y relative to the image), and `Menu::rows(&self) -> &[Row]`.
  - `pub struct Row { pub label: String, pub action: Option<Action>, pub checked: bool }`
  - `pub fn apply(a: &Action, s: &mut Settings)` (does not touch the platform; Quit and ToggleAutostart side effects are handled in `app`).

**Rows, in order** (exact copy; `•` marks the current choice):

```
Pet:    Alex | Alex the bun
Size:   S | M | L | XL
Opacity:  25 | 50 | 75 | 100
Fullscreen: Hide | Stay visible
When visible:  25 | 50 | 75 | 100
Launch on startup  [on/off]
Quit
```

Each option is its own clickable cell. The panel is pixel-drawn: a 1-art-px dark outline (`#24120A`), a cream fill (`#F3D29C`), text in `#24120A` and the selected cell in `#2F64C8` with white text. Drawn at the pet's zoom.

- [ ] **Step 1: Write the failing tests** in `tests/menu.rs`:
  - `rows_match_copy`: the labels equal the block above, with the current values checked for default settings.
  - `hit_finds_cells`: the centre of the `L` cell returns `SetSize(L)`; the panel border returns `None`.
  - `apply_changes_settings`: each `Action` variant changes the matching field.
  - `font_covers_menu_text`: every character in the copy has a glyph.
- [ ] **Step 2: Run `cargo test --test menu` and confirm it fails.**
- [ ] **Step 3: Implement** `font.rs` (any public-domain 5×7 font, transcribed as `[u8; 7]` row bitmasks), `menu.rs`, and the `app` wiring:
  - Right-click on the pet opens the menu window beside the pet, kept inside the work area.
  - A click applies the action, saves settings and redraws the menu.
  - Focus lost or Esc closes it.
  - `ToggleAutostart` calls `platform.set_autostart(Some(exe))` / `None`.
  - Changing pet or size takes effect at once.
  - Quit exits cleanly.
- [ ] **Step 4: Run `cargo test` and confirm everything passes.** Then by hand: open the menu, change every setting, restart the app, and check that the settings persisted. Toggle launch on startup and check the `Run` key in `regedit`. Screenshot the open menu and look at it.
- [ ] **Step 5: Commit** — `git commit -m "ui: right-click settings menu"`.

---

### Task 9: Measure it

**Files:**
- Create: `tools/measure.ps1`

**Interfaces:**
- Consumes: the `PIXELPET_PIN` env var (Task 7) and the release binary.

- [ ] **Step 1: Write `tools/measure.ps1`** with params `-Exe <path> -Seconds 60 -Runs 3`.
  - For each of `sleep` and `walk` it starts the exe with `PIXELPET_PIN` set.
  - It samples `Get-Process` CPU time and working set every second, then stops it, three runs each.
  - It prints a table of the median CPU % (CPU-seconds delta ÷ wall seconds ÷ logical cores × 100) and the median working set in MB, plus the exe size in MB.
  - It prints `PASS`/`FAIL` against the budget: sleeping ≤ 0.5 %, wandering ≤ 1.5 %, memory ≤ 30 MB, file ≤ 6 MB.
- [ ] **Step 2: Build and run.** `cargo build --release`, then `powershell -File tools/measure.ps1 -Exe target/release/PixelPet.exe`. Expected: a filled table. Report the numbers as measured, including any FAIL, and name the biggest cost if over budget.
- [ ] **Step 3: Commit** — `git commit -m "tools: measure CPU, memory and size against the budget"`.
