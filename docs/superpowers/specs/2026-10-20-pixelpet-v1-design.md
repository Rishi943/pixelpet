# PixelPet v1 — design

Date: 2026-10-20 · Status: approved in chat, awaiting review of this written version

## 1. What it is

A small pixel-art pet that lives on the desktop, like Clippy. It wanders along
the bottom of the screen, naps, reacts to the cursor, and can be petted,
played with and fed. It runs on Windows, macOS and Linux.

v1 ships two default pets, both memorial pets drawn from photos:

- **Alex**, a red-and-black German Shepherd (the user's dog). Photos: `alex/`.
- **Alex the bun**, a white rabbit with chestnut patches (a friend's rabbit).
  Photos: `alex the bun/`. Pronouns not stated: keep all copy neutral.

The approved look is **B (Classic)**. The look sheet that was signed off is
`drafts/looks/` (published at https://claude.ai/artifact/GWvR6xFBJH7irBbEhU8uK8).

Two hard requirements from the user:

1. **Lightweight, few dependencies.** Anyone can download one file and run it.
   No installer, no runtime to install, no internet access.
2. **Likeness.** These are memorial pets. Any art change is checked against
   the photos with screenshots of the real app, not only test renders.

## 2. Scope

**In v1**

- One pet on screen at a time; switch between Alex and Alex the bun from the menu.
- Wander, sit, look at you, jump, sleep (the default idle), plus the new
  interaction animations in §5.
- Hover menu: Pet, Play (Fetch, Tug-of-war), Feed.
- Right-click menu: pet, size, opacity, fullscreen behaviour, launch on startup, quit.
- Settings saved to a small text file.
- Release builds for Windows, macOS and Linux, from one codebase.

**Later, not v1**

- v2: make your own pet (dogs, cats, rabbits) with sliders and a randomise button.
- v3: build a pet's coat from uploaded photos or video.
- Ideas parked: both pets at once, walking on top of windows, more than one
  monitor, sounds, a tray icon.

**Never:** hunger, sickness, sadness or any guilt mechanic. Food and play are
treats, not chores.

## 3. Glossary (one name per thing)

| Term | Meaning |
|---|---|
| **pet** | The animal on screen. One at a time. |
| **species** | `dog` or `rabbit` in v1. Owns its poses, shapes, markings and eyes. |
| **coat** | A species' colour palette and markings. v1 coats: Alex, Alex the bun. |
| **look** | A drawing style: A Chunky, **B Classic**, C Detailed. v1 ships B. |
| **rig** | The code that turns a skeleton into pixels. |
| **skeleton** | Joint positions plus a few numbers (mouth, eyes, ears) for one frame. |
| **anim** | A named, looping or one-shot motion that produces skeletons over time (`walk`, `sit`, `tilt`, `hop`, `sleep`, `stand`, …). |
| **brain** | The behaviour logic. It decides which anim plays, and where the pet is. |
| **bubble** | The hover menu (Pet / Play / Feed). |
| **menu** | The right-click settings menu. |
| **toy** | Cricket ball, coconut, football, tug rope. |
| **food** | Chicken, bone, kibble; carrot, coriander, banana. |
| **ground** | The line the pet stands on: the top of the taskbar, which is the bottom of the screen's work area. |
| **depth** | How far "into the screen" something is during fetch. 0 = on the ground line. |

## 4. Living on the desktop

- The pet walks along the ground of the **primary monitor**, turning at the edges.
- The **wander script** picks the next anim with weights (walk 4, stand 1.4,
  sit 1.4, look 1.1, jump 1, sleep 0.8) and random durations. These are the
  look sheet's numbers.
- **Idle nap.** After 3 minutes with no mouse or keyboard input the pet lies
  down and sleeps. On input it wakes, stretches, and walks towards the cursor.
- **Point at the pet** and it stops and looks at you: Alex does the head
  tilt, the bun stands up tall. The bubble appears after 0.4 s of hovering.
- **Drag** the pet with the left button to pick it up and drop it elsewhere
  on the ground (it falls back down if dropped in the air).
- **Fullscreen apps:** per the setting, the pet either hides or stays
  visible at the "fullscreen opacity". When detection isn't possible (see §8)
  the pet simply stays.

## 5. Interactions

### Pet
Choose Pet, then stroke the pet by moving the cursor back and forth across it.
Each stroke floats a heart. Alex closes his eyes and wags; after enough
strokes he leans into it. The bun's eyes close and, after enough strokes, it
flops onto its side. Leaving the pet ends it.

### Play → Fetch
1. The toy appears under the cursor: cricket ball (default) or coconut for
   Alex, the football for the bun.
2. Drag it and flick it upward. Flick speed sets how far it goes in **depth**.
3. The toy flies towards a vanishing point above the ground and shrinks as
   it goes. Depth scale is `1 / (1 + depth)`.
4. The pet runs after it and shrinks with depth. Shrinking re-draws the pet
   at a smaller pixel size rather than scaling the image, so it stays crisp.
5. Alex picks the toy up in his mouth and runs back. The bun noses it back
   along the ground.
6. The toy is dropped near the cursor, ready for another throw. Right-click
   or Esc ends play.

During fetch the pet's window grows to cover the area the throw needs, then
shrinks back.

### Play → Tug-of-war
A rope runs from the pet's mouth to the cursor. The pet pulls in bursts with
head shakes, and each burst moves the **real cursor** a little towards the
pet. The user pulls back by moving the mouse away.

- Pull far enough for 2 s and the user wins: the pet lets go and does a happy jump.
- Let the cursor get dragged onto the pet and the pet wins: a victory shake.
- Either way it ends there; right-click or Esc ends it early.
- The bun plays too: it grips the rope in its teeth and hops backwards.
- Where the cursor can't be moved (Linux on Wayland), the rope stretches and
  the pet leans back instead, and the same win rules apply by distance.

### Feed
Choose Feed, then a food. It drops from the bubble to the ground. The pet
walks over, eats (head down, chewing, crumbs), then reacts: Alex wags and licks
his lips, the bun does a binky.

- Alex: **chicken** (default), bone, bowl of kibble.
- Alex the bun: **carrot** (default), coriander, banana slice. The user may
  add the bun's real favourite later.

### New anims needed beyond the look sheet

| Species | Anims |
|---|---|
| dog | `run` (fetch), `carry` (run with toy), `eat`, `wag` (happy), `lean` (petted), `tug` (pull back + head shake), `stretch` (waking) |
| rabbit | `eat` (chewing), `flop` (on its side), `nudge` (push toy with nose), `tug` (grip + hop back), `stretch` |

Each new anim is checked by screenshot against the photos before sign-off,
the same way as the look sheet.

## 6. Settings

Opened from the right-click **menu**, drawn as pixel art in the same style as
the pet, with no separate settings window.

| Key | Values | Default |
|---|---|---|
| `pet` | `alex`, `alex-the-bun` | `alex` |
| `size` | `S`, `M`, `L`, `XL` (whole-number pixel zoom 2/3/4/5 for look B) | `M` |
| `opacity` | 25–100 in steps of 25 | 100 |
| `fullscreen` | `hide`, `stay` | `hide` |
| `fullscreen_opacity` | 25–100 in steps of 25 | 50 |
| `autostart` | `on`, `off` | `off` |

**File:** plain `key = value` lines, one per setting. Unknown keys are kept
when the file is rewritten, and a missing or broken file means defaults.

- Windows: `%APPDATA%\PixelPet\settings.txt`
- macOS: `~/Library/Application Support/PixelPet/settings.txt`
- Linux: `$XDG_CONFIG_HOME/pixelpet/settings.txt` (normally `~/.config/pixelpet/`)
- **Portable mode:** if `settings.txt` sits next to the program, that file
  is used instead.

**Launch on startup** points at wherever the program currently is. On each
launch, if autostart is on and the program has moved, the entry is rewritten.

- Windows: a value in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- macOS: `~/Library/LaunchAgents/com.pixelpet.app.plist`.
- Linux: `~/.config/autostart/pixelpet.desktop`, which for an AppImage
  points at `$APPIMAGE`.

## 7. Architecture

One Rust program. Outside crates:

- `winit` for the window and input.
- `softbuffer` to put a pixel buffer on screen.
- `windows-sys` on Windows only, with the same version winit already uses,
  for the few Win32 calls in §8.

macOS and Linux calls go through crates winit already pulls in (`objc2*`,
`x11-dl`), or small `extern "C"` declarations. Nothing else ships. Dev-only
crates for tests (for example `png`) are fine.

```
src/
  main.rs            event loop, frame pacing
  app.rs             wires brain + rig + ui + window together
  brain/             behaviour: wander script, idle nap, hover, drag,
                     pet / fetch / tug / feed controllers   (pure logic)
  rig/               core.rs   shapes, hit tests, shading, outline (from rig.js)
                     dog.rs    dog anims, shapes, Alex coat
                     rabbit.rs rabbit anims, shapes, Alex-the-bun coat (from rabbit.js)
                     props.rs  toys and food sprites
  ui/                bubble, menu, tiny bitmap pixel font, icons   (drawn by code)
  window.rs          transparent always-on-top window, present, click-through
  platform/          mod.rs (trait), windows.rs, macos.rs, linux.rs
  settings.rs        read/write the key = value file, paths, portable mode
tests/
  golden/            reference PNGs rendered from the JS drafts
  parity.rs          Rust rig vs golden, pixel by pixel
  brain.rs           behaviour with a fake clock and fake platform
tools/
  golden.js          renders the reference frames from drafts/looks/*.js
```

**Boundaries**

- `rig` knows nothing about windows or time. It takes a skeleton, a look, a
  coat and a size, and returns RGBA pixels.
- `brain` knows nothing about pixels or the OS. It takes inputs (time,
  cursor, idle seconds, fullscreen flag, clicks, menu choices) and returns
  what to show: anim, time within it, position, facing, depth, props and bubble state.
- `platform` is the only place with OS-specific code, behind one trait, so
  `brain` tests use a fake.
- `app` runs: tick → read platform → brain update → rig render (cached per
  frame key) → ui overlay → present.

**Frame pacing.** Anims step at 12 frames a second; movement updates at
display rate but snaps to the art-pixel grid. Nothing is redrawn when nothing
changed: a sleeping pet redraws only for breathing frames and z's. During
fetch and tug the loop runs at 30 Hz.

**Window.** Borderless, transparent, always on top, with no taskbar or dock
entry, sized to the pet plus a margin and moved as the pet walks. Clicks
pass through except over opaque pet pixels or an open bubble or menu. To do
that, the app polls the global cursor about 30 times a second and switches
the window's hit-testing on and off. The window is moved to the pet, not
the pet drawn inside a big window, which keeps the pixel buffer small and
CPU low.

## 8. Per-OS behaviour

| Capability | Windows | macOS | Linux X11 | Linux Wayland |
|---|---|---|---|---|
| Transparent, always on top | yes | yes | yes | transparent yes; staying on top is up to the desktop |
| Hide from taskbar/dock | skip-taskbar | accessory app, no dock icon | window type hint | compositor-dependent |
| Global cursor position | `GetCursorPos` | `NSEvent mouseLocation` | `XQueryPointer` | **no**: only over our window |
| Move the cursor (tug) | yes | yes (cursor warp) | yes | **no**: rope-stretch fallback |
| Click-through toggle | yes | yes | yes | all-or-nothing, so transparent corners of the small window take clicks |
| Work area (ground line) | `GetMonitorInfo` rcWork | `NSScreen visibleFrame` | `_NET_WORKAREA` | monitor bottom minus a margin |
| Idle seconds | `GetLastInputInfo` | `CGEventSourceSecondsSinceLastEventType` | XScreenSaver extension | none: wander script only |
| Fullscreen app detection | `SHQueryUserNotificationState` + foreground window covers monitor | frontmost window covers screen; "stay" also joins fullscreen Spaces | `_NET_WM_STATE_FULLSCREEN` on the active window | none: pet stays |
| Launch on startup | registry Run key | LaunchAgent plist | autostart `.desktop` | autostart `.desktop` |

## 9. Performance budget

Measured on the user's Windows laptop as the median of 3 runs, 60 s each,
reporting the process's CPU % and working set:

- Sleeping: ≤ 0.5 % CPU.
- Wandering: ≤ 1.5 % CPU.
- Memory: ≤ 30 MB.
- Program file: ≤ 6 MB.

The script that measures this lives in `tools/` and is committed.

## 10. Distribution

Built by one GitHub Actions workflow on each tagged release:

- **Windows:** `PixelPet.exe`, a single portable file. MSVC target with a
  statically linked C runtime, so no "VCRUNTIME140.dll missing".
- **macOS:** `PixelPet.app` as a universal binary (Intel + Apple Silicon),
  with `LSUIElement` set so there's no dock icon, inside `PixelPet.dmg`.
- **Linux:** `PixelPet-x86_64.AppImage`, built inside an older distro image
  so it runs on most distros. winit loads X11/Wayland libraries at run time.

v1 is unsigned. Windows SmartScreen and macOS Gatekeeper show a warning on
first launch, and the download page explains the one-time "Run anyway" /
right-click → Open step. Signing can come later.

Local development on this laptop uses the `stable-x86_64-pc-windows-gnu` Rust
toolchain, which avoids the Visual Studio Build Tools download. Release builds
for Windows use MSVC on CI.

## 11. Testing

1. **Rig parity.** `tools/golden.js` renders reference frames from the
   approved JS drafts: both species, every look-sheet anim at several times,
   looks B and C. `tests/parity.rs` renders the same frames in Rust and
   compares pixels. The target is identical. Up to 0.5 % of pixels may differ
   (rounding at shape edges between JS and Rust maths), and the test prints
   the count and writes a diff image.
2. **Brain.** Scripted scenarios with a fake clock and fake platform: idle
   → nap → wake; hover → look → bubble; drag/drop; fetch start → flight →
   chase → return; tug win and lose; tug on a platform that can't move the
   cursor; fullscreen hide/stay.
3. **Settings.** Round-trip, unknown keys kept, broken file means defaults,
   portable mode chosen when the file sits next to the program.
4. **Real runs, with screenshots.** On the Windows laptop, every anim and
   interaction, compared side by side with the photos (as for the look sheet).
5. **Resources.** The §9 measurements, reported with the numbers.
6. **macOS and Linux.** CI builds and starts the app headless (smoke test
   only). A real check on a Mac and a Linux desktop by a person is required
   before calling a release done.

## 12. Privacy

- No network access of any kind, and no telemetry.
- The pet photos (`alex/`, `alex the bun/`) and the draft comparison images
  made from them stay **out of git** (`.gitignore`). Only the hand-tuned
  palettes and markings, which are code, go in the repo.

## 13. Risks and open points

- **macOS transparency and click-through** behave differently from Windows.
  Spike this first on CI and with a real Mac.
- **Wayland** limits are accepted, with the fallbacks in §8.
- **Fullscreen detection** has edge cases (borderless "fullscreen" games,
  video players). A wrong guess only changes visibility.
- **Rust toolchain** is not yet installed on the laptop (needs approval,
  about 1 GB).
- The bun's favourite food is still to come from the user.
