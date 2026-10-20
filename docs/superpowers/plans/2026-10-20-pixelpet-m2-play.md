# PixelPet Milestone 2: Pet, play and feed — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hover over the pet and a pixel bubble offers Pet, Play (Fetch,
Tug-of-war) and Feed. Each one works as spec §5 describes, with the new
anims, for both Alex and Alex the bun, on the Windows desktop.

**Architecture:** The rig gains the new anims (written directly in Rust; the
JS drafts stay frozen as the parity reference for the look-sheet anims), a
`props` module for food, hearts, crumbs and the rope, and depth scaling. A new
`scene` module composes the pet and its props into the one image the pet
window shows, and says what the cursor is over. The brain gains a bubble
state and one controller per interaction, still pure logic driven by
`Inputs`. `ui::bubble` draws the bubble; `app` gives it its own window, like
the menu.

**Tech Stack:** Rust stable (2021); `winit` 0.30; `windows-sys` =0.52.0. No new crates.

**Spec:** `docs/superpowers/specs/2026-10-20-pixelpet-v1-design.md` (§4 hover
and bubble, §5 all of it, §7 boundaries and frame pacing, §11 tests 2 and 4).

**Starts from:** the M1 code, branch `m1-windows-desktop` (or `main` once M1 is
merged). Read `src/brain/mod.rs`, `src/view.rs` and `src/app.rs` first; this
plan extends them.

## Global Constraints

- Runtime dependencies stay `winit` and, on Windows only, `windows-sys` =0.52.0. Dev tools may use Python 3 + PIL locally; nothing in `tools/` ships.
- No network access. No input logging of any kind in the program: no key, click or cursor logs, no message spies (the user asked for this after M1).
- Never commit anything under `alex/`, `alex the bun/`, or `drafts/looks/compare-*.png`.
- Copy about Alex the bun uses no gendered pronouns ("it" / "the bun").
- Never: hunger, sickness, sadness or guilt. Food and play are treats; nothing decays, nothing asks.
- Glossary terms in code: `pet`, `species`, `coat`, `look`, `rig`, `skeleton`, `anim`, `brain`, `bubble`, `menu`, `toy`, `food`, `ground`, `depth`.
- Physical pixels in `brain`, `platform`, `window`, `scene`; rig units in `rig`.
- Anims step at 12 fps; anim changes blend over 3 steps (as M1).
- Bubble opens after **0.4 s** of hovering. Depth scale is **`1 / (1 + depth)`**.
- The loop runs at **30 Hz** during fetch and tug (and while a bubble, heart or falling food is animating); otherwise as M1 (12 Hz).
- Parity for the look-sheet anims stays at 0 px differing (`tests/parity.rs`).
- Budget (§9) still holds for sleeping and wandering: ≤ 0.5 % / ≤ 1.5 % CPU, ≤ 30 MB, file ≤ 6 MB.
- Every new anim and every new sprite gets a real-app screenshot next to the photos, and the user signs it off before the task is complete (spec §1 "Likeness", §5).
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **The pet's window covers a large area during fetch or tug** (a long rope across the screen, a far throw). Clicks beside the rope and toy must still reach the windows behind. Test in Task 4: a scene with a rope from (100,1000) to (1800,200) has transparent pixels at (1800,1000), and `hit` there is `Hit::None`.
2. **Right-click or Esc mid-throw, mid-chase or mid-tug.** Play ends at once, the toy or rope disappears, the pet goes back to wandering on the ground at depth 0, and the cursor is never moved again. Test in Task 7 (fetch) and Task 8 (tug).
3. **The user goes idle (180 s) in the middle of play, or a fullscreen app comes up.** Idle ends play first, then naps as before. Fullscreen hide hides the pet and its props together. Test in Task 5: `nap_ends_play`.
4. **A throw or a food lands where the pet can't reach** (past a screen edge, or with the pet near an edge). The target is clamped into the work area, so the pet always reaches it and comes back. Test in Task 7: `throw_target_clamped`.
5. **Tug on a platform that can't move the cursor** (`warp_cursor` returns false) and **a user who never pulls.** Without warp the win and lose rules still apply by distance, and an untouched tug ends after 20 s with no winner. Test in Task 8: `tug_without_warp` and `tug_times_out`.

---

### Task 1: Dog anims — run, carry, eat, wag, lean, tug, stretch

**Files:**
- Modify: `src/rig/mod.rs` (the `Anim` enum, `period`, `speed`, `pose` dispatch), `src/rig/dog.rs`
- Create: `tests/anims.rs`, `tools/compare.py`

**Interfaces:**
- Produces: `Anim::{Run, Carry, Eat, Wag, Lean, Tug, Stretch, Flop, Nudge}` added to `Anim` (Flop and Nudge are rabbit-only and get their poses in Task 2), with `Anim::ALL` and `name()` covering them: `run carry eat wag lean tug stretch flop nudge`.
- Produces: `rig::is_one_shot(anim: Anim) -> bool`: true for `Stretch` only. The brain plays a one-shot for `period` seconds and then moves on.
- Produces: `rig::mouth(pet: Pet, s: &Skel, look: &Look) -> (f64, f64)`: the mouth point in frame pixels (same space as `Frame.ox/gy`), for the rope and for food placement.
- Produces: `rig::with_eyes_closed(s: Skel) -> Skel`: the same skeleton with the eyes shut (the dog's `eyes = 0`, the rabbit's `eyes = 0`).

- [ ] **Step 1: Write the failing tests in `tests/anims.rs`.**
  - `new_dog_anims_render` — for each of the seven dog anims, at t ∈ {0, 0.1, …, 2.0}, `rig::render(&rig::pose(Pet::Dog, a, look, t, None), look, false)` has at least 300 opaque pixels, all opaque pixels lie inside the frame, and no skeleton field is NaN.
  - `run_is_faster_than_walk` — `speed(Dog, look, Run) >= 2.0 * speed(Dog, look, Walk)`, `speed(Dog, look, Carry) >= 1.5 * speed(Dog, look, Walk)`, and `period(Dog, Run) < period(Dog, Walk)`.
  - `eat_puts_the_head_down` — at every t, the eat skeleton's `head.1` is lower than the stand skeleton's `head.1` by at least 0.25 rig units, and lower than `sh.1` (the shoulder).
  - `tug_pulls_back_and_shakes` — the tug skeleton's `hip.0` is behind `sh.0` (towards the tail) by more than the stand skeleton's; `head_rot` takes at least 3 distinct signs-changes in one period (the shake).
  - `lean_leans` — `sh.0 - hip.0` differs from stand's by at least 0.1, and `eyes == 0` at every t (he leans in with his eyes closed).
  - `wag_wags` — the tail tip `tail[4]` swings: across one period its x spans at least 0.3 units.
  - `carry_holds_the_toy` — `pose(Dog, Carry, look, t, Some(Toy::Ball))` keeps `toy == Some(Ball)`; `Run` without a toy has `toy == None`.
  - `stretch_is_one_shot` — `is_one_shot(Stretch)`, `!is_one_shot(Walk)`, `period(Dog, Stretch) == 1.5`; the stretch skeleton at t = 0.75 has the shoulder lower than the hip (play bow).
  - `mouth_is_at_the_head` — `mouth(Dog, stand, look)` is within `0.6 * look.r` px of the head joint projected to frame px, and on the facing side of it.
  - `old_anims_unchanged` — run `tests/parity.rs`; it still reports 0 px differing (this is a verification step, not a new test).
- [ ] **Step 2: Run `cargo test --test anims`.** Expected: FAIL to compile (`no variant named Run`).
- [ ] **Step 3: Add the variants and poses in `src/rig/dog.rs`.** Build each pose from the existing ones the way the drafts do (shared `stand`/`walk`/`sit_skel` helpers, `mix_p`, `rot`), with these timing values:
  - `Run`: a bounding gallop (both front legs then both hind legs), period 0.42 s, `speed = 2.4 × walk_speed`, mouth open with tongue out.
  - `Carry`: the run gait at period 0.5 s, `speed = 1.8 × walk_speed`, head held high, mouth closed on the toy (`toy` from the argument, drawn as now).
  - `Eat`: standing, neck down so the muzzle reaches the ground line, head bobbing on a 0.5 s chew, mouth opening 0 → 0.6 → 0, period 1.0 s.
  - `Wag`: stand with the tail sweeping ±35° at 3 Hz and the body wiggling 0.02 units; a lip lick (tongue 0 → 1 → 0 over 0.4 s) at 1.2 s in a 2.4 s period.
  - `Lean`: sitting, the whole upper body tilted 12° towards the facing side, eyes 0, slow tail sway, period 3.0 s.
  - `Tug`: front legs braced forward, hips back and low, head shaking ±18° in 3 shakes per 0.9 s, mouth closed, period 0.9 s.
  - `Stretch`: a 1.5 s play bow — chest down and hips up by 0.5 s, held to 1.0 s, back to stand by 1.5 s, tail up. `Flop`/`Nudge` on the dog fall back to `Sleep`/`Walk` poses so nothing panics.
  - `mouth` projects the head joint plus the muzzle offset the shapes already use, through the same frame mapping as `render`.
- [ ] **Step 4: Run `cargo test --test anims` and `cargo test --test parity`.** Expected: all pass; parity 0 px differing.
- [ ] **Step 5: Write `tools/compare.py`.** `python tools/compare.py --out drafts/looks/compare-m2-<name>.png --photos <photo>... --shots <png>...` tiles the photos (scaled to 360 px tall) above the screenshots (nearest-neighbour, same height) on a dark background with each file's name under it. Output under `drafts/looks/compare-*.png` only (git-ignored).
- [ ] **Step 6: Screenshot each new dog anim in the real app.** For each anim: `powershell -File tools/shoot.ps1 -Exe target/release/PixelPet.exe -Pin <anim> -Out <scratch>/<anim>-<n>.png -Wait <w>` at three waits spread over its period (crop the region around the pet). Build `drafts/looks/compare-m2-dog.png` with two photos from `alex/` that show the matching pose where one exists. Look at it yourself first: proportions, coat markings, ears and tail must match the look sheet.
- [ ] **Step 7: STOP — user sign-off.** Show the user the compare image and ask for a yes or for changes per anim. Apply changes and repeat Steps 4–7 until signed off. Ledger the sign-off.
- [ ] **Step 8: Commit** `src/rig/mod.rs src/rig/dog.rs tests/anims.rs tools/compare.py` — "rig: dog anims for play (run, carry, eat, wag, lean, tug, stretch)".

### Task 2: Rabbit anims — eat, flop, nudge, tug, stretch

**Files:**
- Modify: `src/rig/rabbit.rs`, `src/rig/mod.rs`
- Test: `tests/anims.rs`

**Interfaces:**
- Consumes: the `Anim` variants, `is_one_shot`, `mouth`, `with_eyes_closed` from Task 1.
- Produces: rabbit poses for `Eat, Flop, Nudge, Tug, Stretch`. Rabbit `Run` and `Carry` fall back to the `Walk` pose (the bun chases by hopping); `Wag` and `Lean` fall back to `Stand`.
- Produces: `speed(Rabbit, look, Nudge) = 0.6 × rabbit::WALK_SPEED`, with `move_profile` for Nudge equal to the walk's burst profile.

- [ ] **Step 1: Write the failing tests in `tests/anims.rs`.**
  - `new_rabbit_anims_render` — as `new_dog_anims_render`, for the five rabbit anims.
  - `flop_lies_on_its_side` — the flop skeleton by t = 1.0 has `flat >= 0.9` and `eyes == 0`, and its highest joint is lower than the stand skeleton's `head.1 - 0.3`.
  - `nudge_keeps_the_nose_down` — the nudge skeleton's `head.1` is at most stand's `head.1 - 0.2` at every t.
  - `bun_tug_hops_back` — across one tug period, `hip.0` moves backwards (against facing) and returns; the ears swing (`ear_sw` changes sign).
  - `bun_eat_chews` — `nose` twitches at least twice per period and `head.1` is below stand's.
  - `rabbit_fallbacks_do_not_panic` — `pose(Rabbit, a, …)` for every `Anim::ALL` value renders.
- [ ] **Step 2: Run `cargo test --test anims`.** Expected: the new tests FAIL.
- [ ] **Step 3: Add the poses in `src/rig/rabbit.rs`.** From the loaf/stand/walk helpers:
  - `Eat`: loaf with the head down to the ground, fast nose and jaw twitch, period 0.8 s.
  - `Flop`: from loaf, roll onto the side over 0.6 s (`flat` 0 → 1, legs out), then lie still with slow breathing, eyes closed; period 4.0 s, the roll only at the start (hold after 0.6 s).
  - `Nudge`: the walk's hop, shorter and lower, with the head down and forward, period 0.7 s.
  - `Tug`: front feet planted, a small backward hop every 0.8 s, head shaking ±12°, ears swinging, period 0.8 s.
  - `Stretch`: 1.5 s: front paws forward and chest low, back legs extended, then back to loaf.
- [ ] **Step 4: Run `cargo test --test anims` and `cargo test --test parity`.** Expected: all pass, parity 0 px.
- [ ] **Step 5: Screenshot each new rabbit anim** as Task 1 Step 6, with photos from `alex the bun/`, into `drafts/looks/compare-m2-bun.png`.
- [ ] **Step 6: STOP — user sign-off,** as Task 1 Step 7.
- [ ] **Step 7: Commit** — "rig: rabbit anims for play (eat, flop, nudge, tug, stretch)".

### Task 3: Props — food, hearts, crumbs, rope

**Files:**
- Create: `src/rig/props.rs`, `tests/props.rs`
- Modify: `src/rig/mod.rs` (`pub mod props;`)

**Interfaces:**
- Produces: `rig::Food { Chicken, Bone, Kibble, Carrot, Coriander, Banana }` with `Food::for_pet(pet: Pet) -> [Food; 3]` (default first: Alex `[Chicken, Bone, Kibble]`, the bun `[Carrot, Coriander, Banana]`) and `Food::label(&self) -> &'static str`: `"Chicken" "Bone" "Kibble" "Carrot" "Coriander" "Banana"`.
- Produces: `props::food(food: Food, bites: u8) -> Frame` (bites 0–3; 3 = gone, an empty frame), `props::heart() -> Frame`, `props::crumb(food: Food) -> [u8; 4]` (one art-pixel colour), all in art pixels with `ox`/`gy` at the bottom centre, drawn as hand-authored pixel maps in the pet palettes' style (1 px dark outline `0x24120A`, 3 tones).
- Produces: `props::rope(img: &mut Rgba, from: (i32, i32), to: (i32, i32), zoom: u32, sag: f64)` — draws a 2-tone rope 1 art pixel thick (`zoom` px), sagging by `sag` px at the middle, snapped to the art grid.

- [ ] **Step 1: Write the failing tests in `tests/props.rs`.**
  - `foods_have_three_bites` — for every food, `food(f, 0)` has ≥ 30 opaque px, `food(f, 1)` and `food(f, 2)` have strictly fewer, `food(f, 3)` has 0; each frame is at most 16×12 art px.
  - `food_choices_per_pet` — `Food::for_pet(Dog)[0] == Chicken`, `Food::for_pet(Rabbit)[0] == Carrot`, labels as above.
  - `heart_is_red_and_small` — `heart()` is at most 9×8, has ≥ 20 opaque px, and its most common opaque colour has red > 150, green < 90.
  - `rope_is_on_the_grid` — on a 200×100 image, `rope(…, (10,10), (190,60), 3, 12.0)` paints pixels only in whole 3×3 blocks, the block nearest each end is painted, and the middle is lower than the straight line by 9–15 px.
- [ ] **Step 2: Run `cargo test --test props`.** Expected: FAIL (`props` not found).
- [ ] **Step 3: Implement `src/rig/props.rs`.** Pixel maps as `&[&str]` rows with one char per art pixel (`.` clear, `o` outline, `1 2 3` dark/mid/light of the food's colour family). Bites remove columns from the side facing away from the bite (right side first). Kibble is a small bowl with brown pieces; coriander a sprig of green leaves; banana a pale slice with a darker ring.
- [ ] **Step 4: Run `cargo test --test props`.** Expected: PASS.
- [ ] **Step 5: Commit** — "rig: props (food, hearts, crumbs, rope)". (The food art is signed off with the feed screenshots in Task 9.)

### Task 4: Scene — one composed image, depth, and what the cursor is over

**Files:**
- Create: `src/scene.rs`, `tests/scene.rs`
- Modify: `src/lib.rs`, `src/rig/mod.rs` (scaled looks), `src/view.rs` (render at a depth), `src/app.rs` (present the scene instead of the pet image)

**Interfaces:**
- Consumes: `view::View::frame`, `view::over_image`, `props::*`.
- Produces: `rig::depth_scale(depth: f64) -> f64` = `1.0 / (1.0 + depth.max(0.0))`.
- Produces: `rig::look_scaled(look: &'static Look, scale: f64) -> &'static Look` — the same look with `r = max(6, round(look.r × scale))`, cached per integer `r` (built once, kept in a `OnceLock` table). Scale 1 returns `look` itself, so depth 0 is pixel-identical to M1.
- Produces: `scene::project(x: f64, depth: f64, vp: (f64, f64), ground: f64) -> (f64, f64, f64)` — screen x, screen ground y and scale for a ground point at `depth`: `s = depth_scale(depth)`, `sx = vp.0 + (x - vp.0) * s`, `sy = vp.1 + (ground - vp.1) * s`. The vanishing point `vp` is `(throw x, ground - 0.3 × work.h)`.
- Produces: `scene::Sprite { img: Rgba, x: i32, y: i32, part: Part }` with `scene::Part { Pet, Toy, Food, Effect }`, and `scene::Scene { sprites: Vec<Sprite>, rope: Option<((i32, i32), (i32, i32), u32)> }`.
- Produces: `Scene::compose(&self) -> (Rgba, i32, i32)` — one image covering the bounding box of every sprite and the rope, with its top-left in screen px. Later sprites draw over earlier ones.
- Produces: `Scene::hit(&self, x: i32, y: i32, sticky_pet: bool) -> Hit` with `Hit { None, Pet, Toy }`: the topmost opaque sprite of part Pet or Toy under the point (Pet uses `over_image`'s stickiness rule; Effect and Food never hit; the rope never hits).

- [ ] **Step 1: Write the failing tests in `tests/scene.rs`.**
  - `depth_scale_halves_at_one` — `depth_scale(0.0) == 1.0`, `depth_scale(1.0) == 0.5`, `depth_scale(-1.0) == 1.0`.
  - `project_meets_the_vanishing_point` — with `vp = (1000, 600)`, ground 1200: depth 0 maps `x = 400` to `(400, 1200, 1.0)`; depth 1 to `(700, 900, 0.5)`; depth 1000 to within 1 px of the vp.
  - `scaled_look_redraws_not_resizes` — a Stand frame rendered with `look_scaled(CLASSIC, 0.5)` and composed at zoom 3 is made only of 3×3 blocks and is 0.4–0.6 of the depth-0 frame's height; `look_scaled(CLASSIC, 1.0)` is `CLASSIC` (pointer-equal).
  - `compose_covers_all_sprites` — two 10×10 sprites at (100,100) and (300,250) compose to a 210×160 image at (100,100), each sprite's pixels in place.
  - `rope_far_away_stays_click_through` — (Review Focus 1) a pet sprite at (100,960) and a rope to (1800,200): the image's pixel at screen (1800,1000) is fully transparent and `hit(1800,1000)` is `Hit::None`.
  - `hit_prefers_the_top_sprite` — a Toy sprite over a Pet sprite returns `Hit::Toy` on the overlap, `Hit::Pet` on the pet-only part, `Hit::None` on transparent pixels; a Food or Effect sprite returns `Hit::None`.
- [ ] **Step 2: Run `cargo test --test scene`.** Expected: FAIL (`scene` not found).
- [ ] **Step 3: Implement.** `View::frame` gains a `depth: f64` argument (part of its frame-cache key) and renders with `look_scaled(look, depth_scale(depth))`. Its returned offsets keep the feet on the projected ground. `app` builds a `Scene` every tick from the brain's `Show` (pet sprite always; the others once Tasks 5–9 add them) and presents `compose()` at its top-left, skipping the present when the key (image generation, top-left, opacity) is unchanged, as now. `app::over_pet` becomes `scene.hit(...) == Hit::Pet`.
- [ ] **Step 4: Run `cargo test`.** Expected: all pass, including the M1 view tests with `depth = 0.0`.
- [ ] **Step 5: Measure.** `powershell -File tools/measure.ps1 -Exe target/release/PixelPet.exe` with no other PixelPet running. Expected: PASS for sleep and walk (the scene must not cost the budget).
- [ ] **Step 6: Commit** — "scene: compose the pet and props into one image; depth scale".

### Task 5: Brain — the bubble, Pet, stretch on waking

**Files:**
- Modify: `src/brain/mod.rs`
- Create: `src/brain/pet.rs` (the stroke counter)
- Test: `tests/brain.rs`

**Interfaces:**
- Consumes: `Anim::{Stretch, Wag, Lean, Flop}`, `rig::is_one_shot`.
- Produces: `Inputs` gains `over_bubble: bool`, `over_toy: bool`, `cancel: bool` (right-click on the pet or Esc during play, set by the app for one tick).
- Produces: `brain::Choice { Pet, Fetch(Toy), Tug, Feed(Food) }` and `Brain::choose(&mut self, c: Choice, now: f64)`.
- Produces: `Show` gains `bubble: bool`, `depth: f64`, `eyes_closed: bool`, `playing: bool`, and `props: Props`, where `Props` is `Copy` with `toy: Option<ToyShow>`, `food: Option<FoodShow>`, `rope_to: Option<(i32, i32)>`, `hearts: [Option<f64>; 6]` (start times), `crumbs: Option<f64>` (start time). `ToyShow { toy: Toy, x: f64, lift: f64, depth: f64, in_mouth: bool }`, `FoodShow { food: Food, x: f64, lift: f64, bites: u8 }`. `Props::default()` is all empty.
- Produces: constants `BUBBLE_AFTER = 0.4`, `BUBBLE_LINGER = 0.6`, `STROKES_TO_SETTLE = 6`, `PLAY_IDLE_END = 60.0`.

- [ ] **Step 1: Write the failing tests in `tests/brain.rs`** (extend the existing `Sim` helper with the new `Inputs` fields).
  - `bubble_after_hover` — hovering 0.39 s: `bubble == false`; at 0.41 s: `true`; cursor moves from pet to bubble (`over_pet = false, over_bubble = true`): stays `true`; over neither for 0.59 s: `true`; at 0.61 s: `false`.
  - `no_bubble_while_asleep_or_dragged` — in Nap, Drag and Fall, hovering 2 s never sets `bubble`.
  - `wake_stretches` — after a nap and input, the anim is `Stretch` for 1.5 s, then `Walk` towards the cursor (replaces M1's Stand-then-seek; update `nap_after_three_idle_minutes` and `wakes_on_any_input_between_idle_polls` to expect `Stretch`).
  - `petting_alex` — `choose(Pet)`, then the cursor sweeps across the pet (over_pet) left-right-left: each direction change after ≥ 0.6 units of travel adds one heart (`hearts` gets a start time); `eyes_closed` and anim `Wag` from the first stroke; after 6 strokes the anim is `Lean`; the cursor off the pet for 0.5 s ends it (`playing == false`, back to wander, eyes open).
  - `petting_the_bun` — the same with `Pet::Rabbit`: eyes closed on `Stand`, then `Flop` after 6 strokes.
  - `jitter_is_not_a_stroke` — reversals with < 0.6 units of travel add no hearts.
  - `nap_ends_play` — (Review Focus 3) during Pet, idle reaches 180 s: `playing == false`, props empty, anim `Sleep`.
  - `fullscreen_hides_props_too` — during Pet with fullscreen + hide: `visible == false` (props are drawn only when visible; the app test is the scene).
- [ ] **Step 2: Run `cargo test --test brain`.** Expected: the new tests FAIL.
- [ ] **Step 3: Implement.** A new `Mode::Play(Play)` where `Play` is an enum the later tasks extend (`Pet { strokes, last_dir, travel, off_since }` here). Bubble: open when `hover_for >= BUBBLE_AFTER` in Wander or Seek; stays open while `over_pet || over_bubble`; closes `BUBBLE_LINGER` after both are false, on any `choose`, and on entering Nap/Drag/Fall/Play. Hearts: a ring of 6 start times, each shown for 1.2 s. Waking plays `Stretch` for `rig::period(pet, Stretch)` then seeks as before. `cancel` or `PLAY_IDLE_END` seconds with no mouse input ends any play. Leaving play resets depth to 0 and props to empty.
- [ ] **Step 4: Run `cargo test --test brain`.** Expected: all pass (M1 tests included, as updated).
- [ ] **Step 5: Commit** — "brain: bubble, petting, stretch on waking".

### Task 6: The bubble on screen, and the app wiring for play

**Files:**
- Create: `src/ui/bubble.rs`, `tests/bubble.rs`
- Modify: `src/ui/mod.rs`, `src/app.rs`, `src/view.rs` (hearts on the scene), `src/platform/mod.rs`, `src/platform/windows.rs`

**Interfaces:**
- Consumes: `Show.bubble`, `Show.playing`, `Show.props.hearts`, `Brain::choose`, `Inputs.over_bubble/cancel`, `Food::for_pet`, `Food::label`.
- Produces: `ui::bubble::Bubble::new(pet: Pet) -> Bubble`, `Bubble::page(&self) -> Page` with `Page { Top, Play, Feed }`, `Bubble::lines(&self) -> String` (the current page as text), `Bubble::size(&self) -> (usize, usize)` (art px, including the tail), `Bubble::draw(&self, scale: u32) -> Rgba`, `Bubble::click(&mut self, x: i32, y: i32, scale: u32) -> Option<Choice>` (navigates pages itself; returns a choice when one is made).
- Produces: `Platform::esc_down(&self) -> bool` — Windows: `GetAsyncKeyState(VK_ESCAPE)` high bit; `Stub`: false; `Fake`: a `RefCell<bool>` field. The app calls it **only while `Show.playing` is true**, and reads no other key.

- [ ] **Step 1: Write the failing tests in `tests/bubble.rs`.**
  - `bubble_copy` — Alex: Top is `"Pet | Play | Feed"`; after clicking Play, `"Ball | Coconut | Tug-of-war | Back"`; after Back then Feed, `"Chicken | Bone | Kibble | Back"`. The bun: Play is `"Football | Tug-of-war | Back"`, Feed `"Carrot | Coriander | Banana | Back"`.
  - `bubble_choices` — at scale 3, clicking the centres of: Top/Pet → `Some(Choice::Pet)`; Play/Coconut → `Some(Fetch(Toy::Coconut))`; Play/Tug-of-war → `Some(Tug)`; Feed/Chicken → `Some(Feed(Food::Chicken))`; Play and Back → `None` (page change only); the tail and border → `None`.
  - `bubble_has_a_tail` — `draw(1)` has opaque outline pixels in the bottom 4 rows only near the horizontal centre (the tail pointing down at the pet), and the panel colours match the menu's (`0x24120A` outline, `0xF3D29C` fill).
- [ ] **Step 2: Run `cargo test --test bubble`.** Expected: FAIL.
- [ ] **Step 3: Implement `ui::bubble`.** Reuse the menu's font, colours, cell layout and `|` separators (factor the shared cell-layout code out of `menu.rs` into a private helper both use, keeping `tests/menu.rs` green). A 4-row tail under the panel's centre.
- [ ] **Step 4: Wire the app.** When `Show.bubble` turns true, open a no-activate `PetWindow` for the bubble centred above the pet's head (kept inside the work area; below the menu if both are open, never overlapping the pet); close it when false. A left click in it goes to `Bubble::click`; a choice goes to `Brain::choose` and resets the bubble to Top. `over_bubble` is the cursor inside the bubble window's rect. Right-click on the pet while `playing` sets `cancel` instead of opening the menu; while `playing`, `esc_down()` rising edge sets `cancel`. Hearts are scene sprites (Part::Effect) rising 1.5 units over 1.2 s above the pet's head and fading out over the last 0.4 s. The 30 Hz rule from Global Constraints goes into the tick rate.
- [ ] **Step 5: Run `cargo test`.** Expected: all pass.
- [ ] **Step 6: Real run.** Run the release exe; hover Alex, open each page, choose Pet and stroke him. Screenshot the bubble on each page and the hearts. Check: the bubble never covers the pet; clicks next to the bubble go through. Ledger what you saw.
- [ ] **Step 7: Commit** — "ui: hover bubble; app: play wiring, hearts, Esc during play".

### Task 7: Fetch

**Files:**
- Create: `src/brain/fetch.rs`
- Modify: `src/brain/mod.rs`, `src/app.rs` (toy sprite, flick velocity)
- Test: `tests/brain.rs`

**Interfaces:**
- Consumes: `Choice::Fetch(Toy)`, `Inputs.over_toy`, `Show.props.toy`, `Show.depth`, `scene::project`, `rig::render_toy`, `Anim::{Run, Carry, Eat, Nudge, Sit}`.
- Produces: `brain::fetch` states, in order: `Waiting` (toy at the cursor, 2 s, then it drops to the ground under the cursor), `Held` (mouse down on the toy: it follows the cursor), `Flying { from, depth, t }`, `Chase`, `Pickup` (dog: 0.4 s of `Eat` at the toy), `Return` (dog `Carry` with `in_mouth`; bun `Nudge`, toy just ahead of the nose on the ground), `Ready` (toy on the ground 0.6 units in front of the pet at the cursor's x; pet `Sit`; grab it again to throw).
- Produces: `fetch::throw_depth(up_px_per_s: f64, unit_px: f64) -> f64` = `clamp(up_px_per_s / unit_px / 25.0, 0.25, 3.0)`; a throw needs an upward flick speed of at least `4 × unit_px` px/s, otherwise the toy just drops where it was let go.
- Produces: flight time `0.6 + 0.3 × depth` s; the toy's screen path is `project` of its ground point at a depth easing linearly from 0 to the target, plus an arc lift of `4 × depth × p × (1 − p) × 3 units` (p = flight progress), drawn at `depth_scale` size.

- [ ] **Step 1: Write the failing tests in `tests/brain.rs`.**
  - `fetch_round_trip_alex` — `choose(Fetch(Ball))`; toy at the cursor; mouse down on it (`over_toy`), move up 300 px in 0.1 s, release: state Flying, `props.toy.depth` rises; after the flight the pet's anim is `Run` and `Show.depth` rises towards the toy's; at the toy: `Eat` for 0.4 s, then `Carry` with `in_mouth`; back at depth 0 near the cursor x: `Sit`, toy on the ground in front, `playing` still true.
  - `fetch_round_trip_bun` — the same with the rabbit and `Football`: chase anim `Walk`, return anim `Nudge` with the toy on the ground ahead of the nose, never `in_mouth`.
  - `weak_flick_just_drops` — releasing with an upward speed under `4 × unit_px` px/s: no flight, the toy falls to the ground at that x, the pet comes to it and brings it back (depth stays 0).
  - `throw_depth_formula` — `throw_depth(0.0, 66.0) == 0.25`, `throw_depth(1650.0, 66.0) == 1.0`, `throw_depth(1e9, 66.0) == 3.0`.
  - `throw_target_clamped` — (Review Focus 4) a throw from x = 5 px leftwards: the toy's ground x stays inside `[work.x + 1.2u, work.right − 1.2u]` and the pet reaches it.
  - `cancel_mid_flight` — (Review Focus 2) `cancel` during Flying or Chase: `playing == false`, `props.toy == None`, `depth == 0`, the pet is on the ground (lift 0), anim from the wander script.
  - `waiting_toy_drops` — no grab for 2 s: the toy falls to the ground under where it was and stays (state Ready-like: the pet sits by it).
- [ ] **Step 2: Run `cargo test --test brain`.** Expected: the new tests FAIL.
- [ ] **Step 3: Implement `brain/fetch.rs`** as a `Play::Fetch(FetchState)` arm. The chase moves the pet's (x, depth) together so both arrive at once, at `speed(pet, look, Run)` units/s measured as `|dx| / unit_px + 6 × |d depth|` (rabbit: Walk speed with its burst profile). The return comes back to (cursor x clamped, depth 0). The app computes the flick speed from the last 100 ms of `Mouse::Move` samples while Held and passes it on `Mouse::Up` (add `Inputs.flick_up: f64`, px/s, positive upwards).
- [ ] **Step 4: Run `cargo test`.** Expected: all pass.
- [ ] **Step 5: Real run.** Fetch with both pets and all three toys: a weak, medium and hard flick each. Screenshot the toy in flight, the pet small at depth, the pickup and the return. Check the pet stays crisp at depth (whole art pixels) and the window shrinks back afterwards. Ledger it.
- [ ] **Step 6: Commit** — "brain: fetch".

### Task 8: Tug-of-war

**Files:**
- Create: `src/brain/tug.rs`
- Modify: `src/brain/mod.rs`, `src/app.rs` (the rope)
- Test: `tests/brain.rs`

**Interfaces:**
- Consumes: `Choice::Tug`, `Platform::warp_cursor` (via a new `Inputs.can_warp: bool` and an output `Show.warp: Option<(i32, i32)>` the app applies), `rig::mouth`, `props::rope`, `Anim::{Tug, Hop, Wag}`.
- Produces: tug rules — the rope runs from the mouth to the cursor (`props.rope_to`). Bursts start every 0.9–1.6 s (random, seeded) and last 0.35 s; during a burst the brain asks to move the cursor `0.6 × unit_px` px in total towards the mouth, spread over the burst's ticks. The user wins after the cursor stays more than `5 × unit_px` px from the mouth for 2 s in a row: the pet lets go (rope gone) and plays `Hop` once. The pet wins when the cursor is over the pet (`over_pet`): `Tug` with a fast shake for 1.2 s (the victory shake), rope gone. Either way play ends after that. No pull from the user for 20 s (cursor within 0.5 units of where it was) ends it with no winner.
- Produces: without warp (`can_warp == false`), bursts lean the pet back further (the `Tug` anim's lean doubled, shown via `Show.anim` staying `Tug` and the rope sagging less) and no `warp` is ever asked for; the same win rules apply by distance.

- [ ] **Step 1: Write the failing tests in `tests/brain.rs`.**
  - `tug_pulls_the_cursor` — `choose(Tug)` with the cursor 3 units from the mouth: within 1.6 s a burst asks for warps that, summed, move it `0.6 × unit_px` (±1 px) towards the mouth; the anim is `Tug`; `rope_to` follows the cursor.
  - `tug_user_wins` — keep the cursor 6 units away for 2.0 s: `Hop`, `rope_to == None`, then `playing == false` after the hop.
  - `tug_pet_wins` — move the cursor onto the pet: victory shake (`Tug`) for 1.2 s, rope gone, then play ends.
  - `tug_without_warp` — (Review Focus 5) `can_warp = false`: no `warp` is ever returned; the user still wins at 6 units for 2 s; the pet still wins when the cursor reaches it.
  - `tug_times_out` — (Review Focus 5) the cursor never moves for 20 s: play ends, no `Hop`.
  - `cancel_mid_tug` — (Review Focus 2) `cancel` mid-burst: play ends at once and no `warp` is returned on that or any later tick.
- [ ] **Step 2: Run `cargo test --test brain`.** Expected: the new tests FAIL.
- [ ] **Step 3: Implement `brain/tug.rs`** as `Play::Tug(TugState)`; the app applies `Show.warp` with `Platform::warp_cursor` and draws the rope into the scene (`Scene.rope`) from the mouth point (`rig::mouth` mapped through the pet sprite's placement and facing) to the cursor, sag `0.4 × unit_px` px (half that while bursting).
- [ ] **Step 4: Run `cargo test`.** Expected: all pass.
- [ ] **Step 5: Real run.** Ask the user before this step: tug moves their real cursor. Then tug with both pets: win once, lose once. Screenshot the rope and the pull. Ledger it.
- [ ] **Step 6: Commit** — "brain: tug-of-war".

### Task 9: Feed

**Files:**
- Create: `src/brain/feed.rs`
- Modify: `src/brain/mod.rs`, `src/app.rs` (food sprite, crumbs)
- Test: `tests/brain.rs`

**Interfaces:**
- Consumes: `Choice::Feed(Food)`, `props::food`, `props::crumb`, `Anim::{Walk, Eat, Wag, Hop}`; the bubble's screen position, passed when choosing (`Brain::choose_at(c, now, from: (i32, i32))`, which `choose` calls with the pet's head position).
- Produces: feed sequence — the food drops from `from` with the same gravity as a dropped pet (`GRAVITY = 2400` px/s²) to the ground, at x clamped 1.5 units away from the pet on the side the bubble was; the pet walks to it (facing it); `Eat` for 2.4 s with `bites` 0 → 1 → 2 → 3 at 0.6 s steps and crumbs from the first bite; then the reaction: dog `Wag` 1.6 s (its lip lick included), bun `Hop` once (the binky); then play ends.

- [ ] **Step 1: Write the failing tests in `tests/brain.rs`.**
  - `feed_alex_chicken` — `choose_at(Feed(Chicken), now, bubble_pos)`: `props.food` falls (lift decreasing to 0); the pet walks to it; `Eat` with bites stepping 0..3 at 0.6 s; then `Wag` for 1.6 s; then `playing == false`, `props.food == None`.
  - `feed_bun_carrot` — the same with the rabbit: reaction `Hop`.
  - `food_lands_on_the_ground_in_reach` — (Review Focus 4) a bubble at the screen's right edge: the food's x stays inside the work area and the pet reaches it.
  - `cancel_mid_meal` — `cancel` while eating: food gone, play ends.
- [ ] **Step 2: Run `cargo test --test brain`.** Expected: the new tests FAIL.
- [ ] **Step 3: Implement `brain/feed.rs`** as `Play::Feed(FeedState)`; the app draws the food sprite (Part::Food, at the pet's zoom) and 6 crumbs (1 art px each, falling 0.5 units with small random x spread, gone after 0.8 s) in front of the mouth.
- [ ] **Step 4: Run `cargo test`.** Expected: all pass.
- [ ] **Step 5: Real run and food sign-off.** Feed all six foods (three per pet). Screenshot each food on the ground and mid-meal; build `drafts/looks/compare-m2-food.png`. **STOP — user sign-off** on the food art and the eat/reaction anims in motion.
- [ ] **Step 6: Commit** — "brain: feed".

### Task 10: Whole-milestone check

**Files:**
- Modify: none expected (fixes only, each with its own failing test first).

- [ ] **Step 1: Run `cargo test`** and `cargo build --release`. Expected: all pass, no warnings.
- [ ] **Step 2: Measure** with `tools/measure.ps1` (nothing else running): sleep and walk PASS; exe ≤ 6 MB. Also note (not gated) CPU during 60 s of fetch, by pinning nothing and playing by hand while `Get-Process PixelPet` samples CPU; report it.
- [ ] **Step 3: Real-run pass with the user** over spec §11 test 4: every new anim and interaction for both pets, screenshots next to the photos. The user signs off the milestone.
- [ ] **Step 4: Commit** any fixes; ledger the measurements.
