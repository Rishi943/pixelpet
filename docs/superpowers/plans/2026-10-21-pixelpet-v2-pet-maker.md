# PixelPet v2 — the pet maker — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** People make their own pet in a small five-step pixel window inside PixelPet (animal, breed, shape, coat, name) with a live preview. Made pets are saved and switched between from the right-click menu, and the maker opens by itself on first launch.

**Architecture:**
- A pet becomes data. A `rig::Design` holds the species, shape multipliers, coat colours and collar. The rig gains `*_for(&Design, …)` versions of `pose`/`render`/`speed`/`mouth`, and the old functions become wrappers over the two built-in designs, so Alex's and the bun's pixels cannot move.
- `src/pets.rs` saves designs as small `key = value` files.
- `src/ui/maker.rs` is a pure state machine plus pixel drawing, in the style of `src/ui/menu.rs`.
- `app.rs` owns the maker window and puts the live preview into it.

**Tech Stack:** Rust 2021, winit 0.30, the existing rig/ui/platform modules. No new crates.

**Spec:** `docs/superpowers/specs/2026-10-21-pixelpet-v2-pet-maker-design.md` (it builds on `2026-10-20-pixelpet-v1-design.md`). Approved mockup: `drafts/maker/index.html`, whose source holds the breed table, slider ranges, swatches and the `family()` shading function this plan ports.

## Global Constraints

- **Alex and Alex the bun look exactly as today.** `tests/parity.rs` stays at 0 px different. Built-in designs render byte-identical to `rig::render(rig::pose(..))` today. The built-ins are never changed or deleted.
- No new crates in `Cargo.lock`. No network, no port, no browser. No reading keys outside the maker window, and only its own typed characters there.
- `rig` knows nothing about files or windows; `pets.rs` knows nothing about windows; `ui::maker` knows nothing about the OS.
- Every anim and interaction works on every made pet: wander, nap, hover, drag, pet, fetch, feed, watch.
- Copy uses only printable ASCII (the app font). Names are 1–24 characters. Alex the bun stays pronoun-neutral.
- Maker window: 236 × 132 UI px at the Size zoom (M = ×3), centred on the work area, focusable, with no taskbar entry. Esc or the "x" closes it.
- Menu colours: fill `#F3D29C`, outline/text `#24120A`, selected `#2F64C8` with white text, dim `#8E6E48`, preview box `#E7C080`.
- Spec §9 budget: sleep ≤ 0.5 % CPU, walk ≤ 1.5 %, ≤ 30 MB, exe ≤ 6 MB on Windows. Measure it for a made pet at the end.
- CI stays green on Windows, Linux and macOS, including the screenshot smoke tests.
- Rushi's PixelPet may be running from `target\release`. Ask him to quit it before any release build.

## Review Focus

1. **A pet file edited by hand or from a newer version** (a missing key, a colour that isn't `#RRGGBB`, a slider of 99, an unknown species): it loads with each bad value replaced by the breed default and clamped to its slider range, never panics, and keeps the unknown keys when saved. Pinned by `bad_values_fall_back_and_unknown_keys_survive` (Task 3).
2. **Extreme sliders** (every slider at its minimum or maximum, and "Random" rolls): no anim of either species may draw past the frame's edge, which would clip ears or a tail on the desktop. Pinned by `extreme_shapes_stay_inside_the_frame` (Task 2). Slider ranges shrink if it fails.
3. **`settings.txt` names a pet whose file was deleted**: the app falls back to Alex and doesn't crash or open an empty pet. Pinned by `missing_pet_falls_back_to_alex` (Task 3).
4. **Typing in the name box**: non-ASCII characters (é, emoji, IME input) are ignored rather than drawn as tofu, Backspace on empty does nothing, and the 25th character is refused. Pinned by `name_takes_only_printable_ascii_up_to_24` (Task 5).
5. **The maker open while the pet on screen changes** (arrows used in the menu, or the current pet deleted): the pet on screen is always a valid saved pet, and Done on a deleted pet saves it again as new. Pinned by `deleting_the_current_pet_falls_back_to_alex` (Task 6).

---

## File structure

| File | Responsibility |
|---|---|
| `src/rig/design.rs` (new) | `Design`, `Shape`, `Coat` overrides, `shades()`; the two built-ins. |
| `src/rig/mod.rs`, `dog.rs`, `rabbit.rs` | `*_for` API; species code takes a `Body` instead of reading one from the look. |
| `src/rig/breeds.rs` (new) | The breed table (dog 6, rabbit 3) and slider ranges. |
| `src/pets.rs` (new) | `PetRecord`, the file format, the store folder, built-ins, ids. |
| `src/settings.rs` | `pet` becomes a pet id string; `first_launch()`. |
| `src/brain/*.rs`, `src/view.rs` | Run on a `Design`. |
| `src/ui/maker.rs` (new) | The maker: steps, actions, hit-testing, keys, drawing. |
| `src/ui/menu.rs` | `Pet: < name >` arrows, "Make a pet...". |
| `src/app.rs` | The maker window, first launch, switching, saving. |
| `tests/design.rs`, `tests/pets.rs`, `tests/maker.rs` (new); `tests/menu.rs`, `tests/brain.rs` | Tests. |

---

### Task 1: A pet as data — `Design`, shades, and the `*_for` rig API

**Files:**
- Create: `src/rig/design.rs`, `tests/design.rs`
- Modify: `src/rig/mod.rs`, `src/rig/dog.rs`, `src/rig/rabbit.rs`, `src/rig/core.rs` (an owned `Palette` from a design)

**Interfaces:**
- Produces (in `pixelpet::rig::design`, re-exported from `rig`):
  - `pub struct Shape { pub legs, body, head, ears, muzzle, tail, fluff, feet: f64 }`. These are multipliers on the look's body, with `Default` = all 1.0.
    - The dog uses legs → `leg_len`, body → `body_len`, head, ears → `ear`, muzzle, tail, fluff.
    - The rabbit uses head, ears → `ear`, fluff, feet → `foot`.
    - Unused fields are ignored.
  - `pub struct Design { pub pet: Pet, pub shape: Shape, pub coat: Vec<(i8, u32)>, pub collar: Option<u32> }`.
    - `coat` is a list of (family id from `core::fam`, base colour `0xRRGGBB`), overriding the species coat.
    - Collar `None` means no collar is drawn.
    - Derives `Clone, Debug, PartialEq`.
  - `Design::alex() -> Design` (Dog, default shape, no overrides, no collar) and `Design::bun() -> Design` (Rabbit, the same). `Design::builtin(pet: Pet) -> Design`.
  - `pub fn shades(base: u32) -> [u32; 4]`: the mockup's `family()` ported exactly (HSL; deep = l×0.42 with h−6 and s×1.1, shadow = l×0.7 with h−3 and s×1.05, base, light = min(0.97, l×1.25+0.05) with h+4 and s×0.92; s capped at 1, hue wrapped).
  - `Design::palette(&self) -> core::Palette`: the species coat compiled, then each override family replaced by `shades(base)`, and the `COLLAR` family set from `shades(collar)` when there is one.
- Produces in `rig`:
  - `pub fn pose_for(d: &Design, anim: Anim, look: &Look, t: f64, toy: Option<Toy>) -> Skel`
  - `pub fn render_for(d: &Design, s: &Skel, look: &Look) -> Frame` (collar drawn iff `d.collar.is_some()`)
  - `pub fn speed_for(d: &Design, look: &Look, anim: Anim) -> f64`
  - `pub fn mouth_for(d: &Design, s: &Skel, look: &Look) -> (f64, f64)`
  - `dog::body_for(look, &Shape) -> dog::Body` and `rabbit::body_for(look, &Shape) -> rabbit::Body`: the look's body with the multipliers applied.
  - `dog::pose`, `rabbit::pose`, `dog::walk_speed` and the mouths take a `&Body` (for example `dog::pose_with(anim, &Body, t, toy)`). The existing `dog::pose(anim, look, t, toy)` stays as `pose_with(anim, &body(look), t, toy)`.
- The existing `rig::pose/render/speed/mouth` keep their signatures. They call the `_for` versions with `Design::builtin(pet)`; `render(s, look, collar)` keeps its collar flag, for icon and tests. A built-in's palette must be the same compiled `ALEX`/`BUN` palette (no overrides, so no shading drift).

- [ ] **Step 1: Write the failing tests** in `tests/design.rs`:
  - `builtin_designs_render_exactly_as_today`: for both species, every `Anim::ALL` at t = 0.0, 0.37 and 1.1, `render_for(&Design::builtin(pet), &pose_for(..), look)` equals `render(&pose(..), look, false)` byte for byte.
  - `shades_match_the_mockup`: `shades(0xD9B26F)` and `shades(0x231B1B)` equal the values the mockup's `family()` gives. Compute them once with node from `drafts/maker/index.html`'s function and paste them into the test.
  - `longer_legs_make_a_taller_dog`: the opaque height of the stand frame with `shape.legs = 1.3` is greater than with 1.0, and with 0.6 it is less.
  - `a_coat_override_changes_only_its_family`: overriding `fam::TAN` changes some pixels, and every pixel that differs from Alex's frame is one of the new tan shades.
  - `collar_is_drawn_only_when_set`: `collar: None` gives Alex's frame; `Some(0x2F64C8)` changes the neck pixels.
  - `long_legs_walk_faster`: `speed_for` with legs 1.3 is greater than with 1.0, for `Anim::Walk`.
- [ ] **Step 2: Run them; they fail.** Run `cargo test --test design`. Expected: they don't compile (no `rig::design`).
- [ ] **Step 3: Implement.** Thread `&Body` through dog and rabbit (signature changes only; no numbers change), add `design.rs`, and add the `_for` functions. Cache `Design::palette` in callers, not in the rig.
- [ ] **Step 4: Run the whole suite.** Run `cargo test`. Expected: all pass, and `tests/parity.rs` shows 0 px different on all 4 cases.
- [ ] **Step 5: Commit.** `git add src/rig tests/design.rs` → `git commit -m "rig: a pet as data (Design), with Alex and the bun unchanged"`

---

### Task 2: Breeds and slider ranges

**Files:**
- Create: `src/rig/breeds.rs`
- Modify: `src/rig/mod.rs` (`pub mod breeds;`)
- Test: `tests/design.rs`

**Interfaces:**
- Consumes `Design`, `Shape`, `fam` (Task 1).
- Produces:
  - `pub struct Breed { pub name: &'static str, pub design: fn() -> Design }`.
  - `pub fn breeds(pet: Pet) -> &'static [Breed]`.
    - **Dog**, in this order: "Alex the GSD" (`Design::alex()`), "Yellow Lab", "Black Lab", "Husky", "Golden", "Corgi".
    - **Rabbit**: "Alex the bun" (`Design::bun()`), "Grey", "Black and white".
    - Values come from the mockup's `BREEDS`: body multipliers to `Shape`, and the coat keys `tan/black/cream/mask` (dog) or `white/brown/pink` (rabbit) to `fam` ids.
  - `pub struct Slider { pub field: &'static str, pub label: &'static str, pub lo: f64, pub hi: f64 }`.
  - `pub fn sliders(pet: Pet) -> &'static [Slider]`.
    - **Dog**: Legs 0.6–1.3, Body 0.75–1.25, Head 0.9–1.5, Ears 0.6–1.5, Muzzle 0.7–1.3, Tail 0.4–1.5, Fluff 0.9–1.35.
    - **Rabbit**: Head 0.85–1.3, Ears 0.7–1.4, Fluff 0.9–1.3, Feet 0.8–1.3.
    - `Shape::get(field)` and `Shape::set(field, v)` access a field by name.
  - `pub fn areas(pet: Pet) -> &'static [(i8, &'static str)]`.
    - **Dog**: TAN "Coat", BLACK "Saddle", CREAM "Legs", MASK "Mask".
    - **Rabbit**: WHITE "Coat", BROWN "Patches", PINK "Ears".
    - The collar is its own row, dog only.
  - `pub const SWATCHES: [u32; 8]` (the mockup's coat swatches) and `pub const COLLARS: [u32; 8]` (the mockup's collar colours).

- [ ] **Step 1: Failing tests.**
  - `first_breed_is_the_builtin`: `breeds(Dog)[0]` is "Alex the GSD" and equals `Design::alex()`; `breeds(Rabbit)[0]` is "Alex the bun" and equals `Design::bun()`.
  - `every_breed_renders_every_anim`: none panic, and each frame has opaque pixels.
  - `extreme_shapes_stay_inside_the_frame`: for each species, with every slider at lo, then at hi, then in 50 seeded random mixes, every `Anim::ALL` at 8 times through its period leaves the frame's outermost 1-px border fully transparent.
- [ ] **Step 2: Run them; they fail.** `cargo test --test design`
- [ ] **Step 3: Implement the table.** If `extreme_shapes_stay_inside_the_frame` fails, **narrow that slider's range** (the spec leaves the ranges to tuning). Don't change `frame_size` for the built-ins: it would move Alex's pixels.
- [ ] **Step 4: Run the whole suite.** `cargo test` → pass.
- [ ] **Step 5: Commit.** `git commit -m "rig: breeds and slider ranges"`

---

### Task 3: Saved pets and the pet setting

**Files:**
- Create: `src/pets.rs`, `tests/pets.rs`
- Modify: `src/lib.rs`, `src/settings.rs`, `src/ui/menu.rs` (compile fixes only), `tests/menu.rs` (compile fixes only)

**Interfaces:**
- Consumes `Design`, `breeds`, `sliders` (Tasks 1–2).
- Produces in `pixelpet::pets`:
  - `pub struct PetRecord { pub id: String, pub name: String, pub breed: String, pub design: Design, pub extra: Vec<(String, String)> }`.
  - `pub const ALEX_ID: &str = "alex"` and `pub const BUN_ID: &str = "alex-the-bun"`, the same strings as v1's `pet =` values.
  - `pub fn builtins() -> [PetRecord; 2]`: Alex (name "Alex", breed "Alex the GSD") and the bun (name "Alex the bun").
  - `pub fn is_builtin(id: &str) -> bool`.
  - `pub fn to_text(r: &PetRecord) -> String` and `pub fn from_text(id: &str, text: &str) -> Option<PetRecord>`.
    - Keys: `name`, `species` (`dog` | `rabbit`), `breed`, `shape.<field>`, `coat.<area label in lower case>`, `collar` (`none` | `#RRGGBB`).
    - Unknown keys are kept in `extra`.
    - A value that won't parse, or a slider out of range, falls back to that breed's value (or the first breed's) and is clamped to the slider range.
    - Unknown species → `None`.
  - `pub fn pets_dir(settings_path: &Path) -> PathBuf` (`<settings dir>/pets`).
  - `pub fn load_all(dir: &Path) -> Vec<PetRecord>`: built-ins first, then `*.txt` files in creation order (sorted by id, since ids are time-based). Broken files are skipped.
  - `pub fn save(dir: &Path, r: &PetRecord) -> io::Result<()>` writes `<id>.txt` and refuses built-in ids.
  - `pub fn delete(dir: &Path, id: &str) -> io::Result<()>` refuses built-ins.
  - `pub fn new_id(now_ms: u128) -> String` → `"p<now_ms>"`, adding a suffix if it is taken.
  - `pub fn find<'a>(all: &'a [PetRecord], id: &str) -> &'a PetRecord`: falls back to Alex.
- `settings::Settings.pet` becomes a `String` holding a pet id (default `"alex"`). The `pet = ` line is read and written as-is. `PetChoice` is removed; `Settings` loses `Copy` (fix the compile errors with `.clone()` or borrows).
  - `pub fn first_launch(settings_path: &Path) -> bool` is `!settings_path.exists()`.
  - The menu temporarily shows the two built-ins exactly as now (Task 6 replaces it).

- [ ] **Step 1: Failing tests** in `tests/pets.rs`:
  - `a_record_round_trips`: a made Husky with odd slider values, a collar and the name "Biscuit".
  - `bad_values_fall_back_and_unknown_keys_survive`: `shape.legs = 99` clamps to 1.3, `coat.saddle = blue` uses the breed's colour, `collar = #12` means none, `future.key = x` is kept in `extra` and written back, and a missing `name` gives the breed name.
  - `builtins_come_first_and_cannot_be_saved_or_deleted`: `save`/`delete` on "alex" give an `Err`.
  - `missing_pet_falls_back_to_alex`: `find(&all, "p123")` with no such pet is Alex.
  - `ids_are_unique`: two `new_id` calls in the same ms in one folder.
  - `v1_settings_still_choose_the_bun`: a `settings.txt` containing `pet = alex-the-bun` loads `pet == "alex-the-bun"`.
- [ ] **Step 2: Run them; they fail.** `cargo test --test pets`
- [ ] **Step 3: Implement.** In tests, use a temp folder under `std::env::temp_dir()` named by pid and test name. Never `set_var` (the M3 lesson).
- [ ] **Step 4: Run the whole suite.** `cargo test` → pass.
- [ ] **Step 5: Commit.** `git commit -m "pets: saved pets as small text files; the pet setting is an id"`

---

### Task 4: Brain and view run on a design

**Files:**
- Modify: `src/brain/mod.rs`, `src/brain/fetch.rs`, `src/brain/feed.rs` (only where `rig::speed`/`mouth` are called), `src/view.rs`
- Test: `tests/brain.rs`, `tests/view.rs`

**Interfaces:**
- Consumes `Design`, `speed_for`, `pose_for`, `render_for`, `mouth_for` (Task 1).
- Produces:
  - `Brain::with_design(d: Design, seed: u64, now: f64, work: Rect) -> Brain`. `Brain::new(pet, …)` stays as `with_design(Design::builtin(pet), …)`.
  - `View::with_design(d: Design, look) -> View`, with `View::new(pet, look)` kept the same way. The palette is compiled once per view.
  - `View::frame`'s `collar` argument is removed: the design decides. Update the one caller in `app.rs`.

- [ ] **Step 1: Failing tests.**
  - `long_legged_dog_walks_further` (`tests/brain.rs`): two brains, same seed; one uses a design with legs 1.3. Over 10 s of `Anim::Walk` (pinned), the longer one moves further.
  - `view_draws_the_design` (`tests/view.rs`): a Black Lab view's first frame differs from Alex's, and `View::new(Dog)` equals `View::with_design(Design::alex())`.
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the whole suite.** `cargo test` → all pass, existing brain tests untouched.
- [ ] **Step 5: Commit.** `git commit -m "brain, view: run on a pet's design"`

---

### Task 5: The maker — state, input and drawing

**Files:**
- Create: `src/ui/maker.rs`, `tests/maker.rs`
- Modify: `src/ui/mod.rs`, `examples/menu_preview.rs` (add `maker N` to write step N to `target/maker-preview-N.ppm`)

**Interfaces:**
- Consumes `breeds`, `sliders`, `areas`, `SWATCHES`, `COLLARS` (Task 2), `PetRecord`, `is_builtin` (Task 3), `ui::font`, and the menu colours.
- Produces:
  - `pub const W: usize = 236; pub const H: usize = 132;` (UI px).
  - `pub enum Step { Animal, Breed, Shape, Coat, Name }`.
  - `pub struct Maker`:
    - `Maker::new(seed: u64) -> Maker` starts on Animal with Alex.
    - `Maker::edit(r: &PetRecord, seed: u64) -> Maker` starts on Breed, holding that pet. Editing a built-in keeps its design but gets a new id on Done.
  - `pub enum Outcome { Nothing, Redraw, Done(PetRecord), Delete(String), Close }`.
  - `Maker::click(x: i32, y: i32) -> Outcome` and `Maker::drag(x: i32, y: i32) -> Outcome` take coordinates in UI px; drag moves a slider grabbed by the last click.
  - `Maker::key(k: MakerKey) -> Outcome`, with `pub enum MakerKey { Char(char), Backspace, Enter, Esc }`.
  - `Maker::design() -> &Design`, `Maker::name() -> &str`, `Maker::step() -> Step`.
  - `Maker::preview_anim() -> Anim`: Stand, Stand, Walk, Sit, Tilt by step, and `Hop` for 1 s after Done is pressed. The rabbit gets its own equivalents where a dog anim has none.
  - `Maker::draw(scale: u32, preview: &Rgba) -> Rgba` returns `W*scale × H*scale`. `preview` is the pet frame already zoomed, placed in the preview box on its ground line and clipped to the box.
  - `pub fn preview_box() -> (i32, i32, i32, i32)` and `pub fn preview_ground() -> i32`, for the app.

Behaviour follows the mockup (`drafts/maker/index.html`, `draw()` and its handlers):
- The layout and copy: "Make a pet", the step dots, the "N/5" count, and the step titles Animal/Breed/Shape/Coat/Name.
- The bottom row: "Random", "< Back" from step 2, and "Next >" or "Done".
- **Two additions over the mockup**:
  - An "x" at the top right closes the window (Outcome::Close).
  - On the Name step, a user-made pet (not built-in) shows "Delete". It asks "Delete <name>? Yes | No"; Yes gives `Delete(id)`.
- **The Coat step** gets the collar row "None" first (a crossed-out swatch), selected for Alex.
- **Choosing an animal** jumps to Breed. **Choosing a breed** resets shape, coat and collar to that breed, and the name to the breed's default: "Alex" / "Alex the bun" for the first breeds, otherwise a name from `NAMES = ["Biscuit","Pixel","Mango","Pepper","Comet","Noodle","Toffee","Bean"]` picked by the seed.
- **Random** rolls every slider uniformly in its range, each area's colour from `SWATCHES`, the collar from `COLLARS`, and a name from `NAMES`, all from the seed.
- **Done**, or Enter on the Name step, gives `Done(record)`:
  - Editing a user pet keeps its id.
  - A new pet, or a built-in being edited, gets `id: String::new()`; the app assigns it.

- [ ] **Step 1: Failing tests** in `tests/maker.rs`, using `cell_centre`-style helpers so that clicks are found, not hard-coded:
  - `steps_go_forward_and_back`: Next, Next, Back; dots can only be clicked for steps already reached.
  - `choosing_an_animal_goes_to_breed`.
  - `choosing_a_breed_sets_its_design`: Husky's design equals `breeds(Dog)[3].design()`.
  - `a_slider_drag_sets_its_value_in_range`: dragging past the right end gives `hi`.
  - `a_swatch_sets_its_area_and_none_removes_the_collar`.
  - `name_takes_only_printable_ascii_up_to_24`: `Char('é')` and `Char('🐶')` are ignored, Backspace on empty is fine, and the 25th character is refused.
  - `done_returns_the_pet`: a made Corgi named "Bean" gives `Done` with breed "Corgi", that design, and name "Bean".
  - `editing_alex_never_touches_alex`: `Maker::edit(&builtins()[0])` then Done returns an empty id.
  - `delete_is_offered_only_for_made_pets_and_asks_first`.
  - `random_is_repeatable_with_a_seed`.
  - `draw_is_the_window_size`: `draw(3, …)` is 708 × 396, and the corner pixels are the outline colour.
  - `font_covers_every_maker_string`: every breed name, label and button text.
- [ ] **Step 2: Run them; they fail.** `cargo test --test maker`
- [ ] **Step 3: Implement.** Keep the drawing and the hit regions from one layout pass, as `menu.rs` does.
- [ ] **Step 4: Run the whole suite, then look.** Run `cargo test`, then `cargo run --example menu_preview -- maker 1` … `maker 5`. Convert to PNG and look at each one next to the matching mockup screen. They should match the mockup's layout.
- [ ] **Step 5: Commit.** `git commit -m "ui: the pet maker (steps, input, drawing)"`

---

### Task 6: The menu — `Pet: < name >` and "Make a pet..."

**Files:**
- Modify: `src/ui/menu.rs`, `tests/menu.rs`

**Interfaces:**
- Consumes `PetRecord` (Task 3).
- Produces:
  - `Menu::new(s: &Settings, pet_name: &str)`, with `confirming_quit` and `with_note` the same.
  - `Action::PrevPet`, `Action::NextPet`, `Action::EditPet`, `Action::MakePet` replace `SetPet`. `menu::apply` leaves them to the app.
  - `pub fn step_pet(all: &[PetRecord], current: &str, dir: i32) -> String` wraps around both ends.
- The copy:
  - Line 1 is `Pet: < Alex >`. The name cell is `EditPet` and is shown selected (blue). `<` and `>` are their own cells.
  - A line `Make a pet...` sits after "Launch on startup" and before Quit. The font has no ellipsis character, so it's three dots.

- [ ] **Step 1: Update and add tests.**
  - `rows_match_copy`: line 0 is `"Pet: < | Alex | >"` in `lines()` form (cells joined by ` | `), and there is a new line `"Make a pet..."`.
  - `step_pet_wraps`: with 3 pets, `>` from the last gives the first and `<` from the first gives the last.
  - `deleting_the_current_pet_falls_back_to_alex`: this is a pets + menu helper test. `pets::find` on a removed id gives Alex, and `Menu::new` is built with "Alex".
  - Drop the `SetPet` assertions.
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the whole suite.** `cargo test` → pass. Run `cargo run --example menu_preview` and look at the menu.
- [ ] **Step 5: Commit.** `git commit -m "menu: pets behind arrows, and Make a pet"`

---

### Task 7: The app — the maker window, first launch, switching and saving

**Files:**
- Modify: `src/app.rs`, `tools/ci/smoke-linux.sh`, `tools/ci/smoke-macos.sh`
- Test: CI smoke tests, real-app screenshots on Windows

**Interfaces:**
- Consumes everything above: `Maker`, `Outcome`, `MakerKey`, `pets::*`, `Brain::with_design`, `View::with_design`, `Menu::new(s, name)`, `step_pet`.
- App behaviour:
  - **At start:** `pets::load_all(pets_dir)`, and the current pet is `pets::find(settings.pet)`, built with `with_design`.
  - **First launch:** if `settings::first_launch(&settings_path)`, open the maker (`Maker::new(seed)`) once the pet window exists, and write `settings.txt` straight away so it doesn't reopen.
  - **The maker window** is a focusable `PetWindow` (`PetWindow::new(el, true)`), centred on the work area at the Size zoom, and redrawn at 12 Hz while open.
    - The preview frame is rendered with `rig::render_for(maker.design(), pose_for(.., maker.preview_anim(), t))` at the Size zoom and handed to `Maker::draw`.
    - The window takes `MouseInput` (click), `CursorMoved` while the button is held (drag), and `KeyboardInput`: `event.text` for `Char`, plus `Backspace`, `Enter` and `Escape`.
    - `Focused(false)` does **not** close it, unlike the menu, because people click away to look at the desktop.
  - **Outcomes:**
    - `Done(r)`: give a new id if `r.id` is empty, `pets::save`, reload the list, set `settings.pet = id`, save the settings, rebuild the brain and view with the design, and close the maker. The pet does its hop.
    - `Delete(id)`: `pets::delete`; if it was the current pet, switch to Alex; then close.
    - `Close`: just close.
  - **Menu actions:**
    - `PrevPet`/`NextPet`: `settings.pet = step_pet(..)`, save, rebuild brain and view, redraw the menu.
    - `EditPet`: close the menu, open `Maker::edit(current)`.
    - `MakePet`: close the menu, open `Maker::new`.
  - **Smoke tests:** both scripts write `pet = alex` to a `settings.txt` in their temp config folder before launch, so the pet shots stay as they are. Each also takes one extra shot, `maker-<os>.png`, on a fresh config folder (first launch) for a person to look at.

- [ ] **Step 1: Write the smoke-test changes first** and push. The new `maker-*.png` shot should show only the pet (the maker isn't built yet).
- [ ] **Step 2: Implement the app wiring.**
- [ ] **Step 3: Test on this laptop.** Ask Rushi to quit PixelPet. Run the debug build with a fresh portable `settings.txt`-less folder (copy the exe into a temp folder) and screenshot with `tools/screenshot.ps1`:
  - The maker on first launch, steps 1–5.
  - A made Husky on the taskbar.
  - The menu with `< Husky >`.
  - Alex after switching back. **Alex's crop must be pixel-identical to a crop of today's build in the same pose** (`PIXELPET_PIN=stand`).
- [ ] **Step 4: CI.** `git push`, then `gh workflow run ci.yml`; all three green. Look at `maker-linux.png` and `maker-mac.png`.
- [ ] **Step 5: Measure the budget.** `tools/measure.ps1`, with a made pet as the current pet: sleep, walk, memory and exe size. Report them against §9.
- [ ] **Step 6: Commit.** `git commit -m "app: the pet maker window, first launch, saved pets"`

---

### Task 8: Docs

**Files:** `docs/status.md`, `docs/release.md` (the real-machine checklist gets: maker opens on first launch; Done, edit and delete; arrows; typing a name on macOS/Linux), `docs/release-notes.md` (one line: "Make your own pet from the right-click menu").

- [ ] **Step 1:** Write them.
- [ ] **Step 2:** Commit: `git commit -m "docs: v2 status and release notes"`

## Not in v2

- Cats, floppy ears, curly tails and spots (new art).
- Building a pet from photos (v3).
- More than one pet on screen at once.
