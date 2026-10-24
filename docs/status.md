# PixelPet status

Newest first. The project's own session record; the shared `E:\AI` tracker only points here.

## 2026-10-24 — PixelPet v3 (pet from photos) parked; v2 goes to release

**Decision (Rushi):** no photo input ships. *"I want alex or alex the bun level of quality"*; clicking points
was *"too much work"* and the result still didn't look like the photo. v2 (default pets + make your own by
hand) is what releases.

**Where v3 is:** branch `v3`, not merged. It has: JPEG/PNG/WebP loading, OS photo dialogs, drag and drop,
a Photos step in the maker (bigger window, reference pet), body and colour clicks, proportions calibrated on
five breeds. Spec `docs/superpowers/specs/2026-10-23-pixelpet-v3-photo-pet-design.md`, plan
`docs/superpowers/plans/2026-10-23-pixelpet-v3-photo-pet.md` (on `v3`).

**Why it can't reach Alex's level:** photos only move sliders and colours on the GSD skeleton and its fixed
markings (a beagle stays a GSD in beagle colours). Cut-out-and-pixelate gives a still picture that can't
walk, nap or fetch, and an offline vision model is ~5–40 MB against the 6 MB budget.

**Better route to likeness, if wanted later:** new rig art (floppy ears, curly tails, marking layouts, cats)
as more hand-made breeds; or a real pet drawn by hand into a built-in breed, the way Alex was.

## 2026-10-23 — PixelPet v2: the pet maker, merged to main

**Repo:** branch `v2` (which already held `m3`) fast-forwarded into `main`. Spec
`docs/superpowers/specs/2026-10-21-pixelpet-v2-pet-maker-design.md`, plan
`docs/superpowers/plans/2026-10-21-pixelpet-v2-pet-maker.md`. Suite: 159 pass, 1 ignored. CI green on `v2`.

**What landed:** a pet is data (`Design`: breed, proportions, colours), with Alex and the bun unchanged (parity
at 0 px). A small pixel-style maker window in five steps with a live preview, opened on first launch and from
"Make a pet..." in the menu; saved pets are small text files, switched with arrows in the menu. X11 connects
with a short retry; macOS swaps the pet's image without a cross-fade.

**Not done / owed:** the real-machine checklist in `docs/release.md` (now includes the maker) is still unrun on
a Mac or Linux desktop.

**Next:** v3, building a pet from photos. Rushi chose: photos set **colours and proportions**; input by file
picker, drag and drop and several photos (details left to Claude). Still fully offline. Spec first.

## 2026-10-21 (later) — PixelPet M3: macOS, Linux and release builds, on branch `m3`

**Repo:** now on GitHub, **private**: `Rishi943/pixelpet` (remote `origin`). Branch `m3`, not merged. Plan
`docs/superpowers/plans/2026-10-21-pixelpet-m3-mac-linux-release.md`. Suite: 124 pass, 1 ignored.

**What landed:** macOS window + platform (`src/window/macos.rs`, `src/platform/macos.rs`), Linux X11 window +
platform (`src/window/linux.rs`, `src/platform/linux.rs`, Wayland via XWayland), pure helpers tested anywhere
(`src/platform/pure.rs`, `lock.rs`, `src/window/shape.rs`). CI on all three OSes with screenshot smoke tests
(`.github/workflows/ci.yml`, `tools/ci/`); macOS jobs only on tags/manual runs (private repo, 10x minutes).
`release.yml`: a tag `v*` makes a DRAFT release with `PixelPet.exe` (MSVC, static CRT, 0.84 MB),
`PixelPet.dmg` (universal, ad-hoc signed, no Dock icon) and `PixelPet-x86_64.AppImage` (built on Ubuntu 20.04).
Drafts `v0.1.0-rc1` (stale) and `v0.1.0-rc2` (current) exist; nothing is published.

**Seen on CI screenshots:** Alex on a Linux desktop with and without a compositor (no box), on the macOS Dock
with the pixel menu open. Same pixels as Windows (parity tests pass on all three).

**Decisions:** macOS never reads Esc (it needs the Input Monitoring prompt, which looks like a key logger;
right-click ends play). In a Wayland session idle, cursor warp and fullscreen are off (spec §8).

**Not done / owed:** nobody has run it on a real Mac or Linux desktop — checklist in `docs/release.md`. Known
open: on macOS a click on the pet probably takes focus from the user's app (needs a non-activating panel).
The release exe has not been run on the laptop yet (Rushi's PixelPet was running; quit it first).
Deferred minors are listed in the M3 closing message.

**Next:** Rushi decides on merging `m3` to `main`; a real-machine pass whenever a Mac/Linux box is to hand;
then v2 (sliders pet maker).

## 2026-10-21 — PixelPet: M1 (Windows desktop) and M2 (pet, play, feed) built and merged to main

**Repo:** `E:\AI\pixelpet` (git, no remote), `main` at `03e018a`. Spec `docs/superpowers/specs/2026-10-20-pixelpet-v1-design.md`,
plans `docs/superpowers/plans/2026-10-20-pixelpet-m1-windows-desktop.md` and `-m2-play.md`. Plan ledgers deleted (git
history is the record). Suite: 110 tests pass, 1 ignored (`autostart_round_trip`, touches the real registry).

**What landed:**
1. M1: Alex and Alex the bun on the Windows desktop — a Rust port of the JS rig (71 golden frames, 0 px different),
   wander/nap/hover/drag, fullscreen hide/stay, right-click pixel menu (quit asks first), single instance, Start menu
   shortcut with an icon drawn from the rig (`tools/start-menu.ps1`, `examples/make_icon.rs`). Budget measured at the
   M2 build: sleep **0.060 %** CPU, walk **0.071 %**, **15.1 MB**, exe **2.13 MB**, all PASS.
2. Two Windows bugs found by Rushi and fixed with tests (`tests/window_hit.rs`): clicks dropped after a drag
   (default hit-test answered HTNOWHERE; now HTCLIENT) and L/XL cut off (a clipping region stuck at the old size;
   cleared on every resize). Auto-hide taskbar: the pet rides on it when it shows.
3. M2: hover bubble (Pet | Play | Feed), petting (hearts, eyes closed, Alex wags then leans, the bun flops), fetch
   with depth (toy flies into the screen; Alex runs in seen from BEHIND and back seen from the FRONT, carries it in his
   mouth; the bun noses it back), hand-feeding, a frequent "watch the screen with you" anim, stretch on waking.
   12 new anims + 3 run views, all signed off. Play ends on right-click/Esc, a nap, or 30 s without playing.

**Decisions in Rushi's words:** no input logging in shipped code (*"i dont want ppl to think we are logging their
keys"* — the message spy and PIXELPET_LOG were removed); tug-of-war **removed** (*"doesn't feel like playing"*);
feed from the hand (*"slowly pulling the cursor towards it as it eats through a carrot the cursor is holding"*);
wag *"side to side"*; *"no need to do the cpu or mem measure"* for the end of M2.

**Sign-offs:** the anims (*"love the rest"*), food art (*"looks great"*), fetch run views (*"love it!"*), feeding
(*"love it"*).

**Open (deferred minors):** dev switches `PIXELPET_PIN/EXIT_AFTER/SCRIPT/OPEN_MENU` still ship in release; the rig's
tug anims are unused; CPU during play (30 Hz composes) unmeasured; clicking the pet mid-feed starts a drag.

**Next:** M3 — macOS and Linux windows/platform code, CI release builds (exe, dmg, AppImage), real-machine checks.
Write its plan from the spec first (writing-plans), in a fresh session. After that: v2 (sliders pet maker), v3 (coat
from photos). Rebuilding `target\release` needs Rushi to quit PixelPet first (the Start menu exe is locked).
