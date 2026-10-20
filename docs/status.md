# PixelPet status

Newest first. The project's own session record; the shared `E:\AI` tracker only points here.

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
