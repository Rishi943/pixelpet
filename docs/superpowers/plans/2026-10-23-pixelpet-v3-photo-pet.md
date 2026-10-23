# PixelPet v3 — a pet from photos — Implementation Plan

> **For agentic workers:** steps use checkbox (`- [ ]`) syntax. Tasks 2–5 run **in parallel**, each in its own
> git worktree branched from the commit Task 1 makes. Touch **only the files your task owns**; if you need a
> change elsewhere, stop and report it instead.

**Goal:** "From photos..." in the maker's Breed list. A Photos step takes up to 5 JPEG/PNG photos (file dialog
or drag and drop), asks the pose, then asks the person to click points on each. The points set the Shape
sliders and Coat colours, which stay editable in the usual steps.

**Spec:** `docs/superpowers/specs/2026-10-23-pixelpet-v3-photo-pet-design.md` (builds on the v1 and v2 specs).
Prototype (not in the repo): `C:\Users\rushi\AppData\Local\Temp\claude\E--AI-Localis\c753809f-ae47-421f-b698-3c0be4ad2e4f\scratchpad\v3proto\proto.py`,
with the tap coordinates it used on the `alex/` photos.

**Tech Stack:** Rust 2021, winit 0.30. New crates (approved 2026-10-23): `zune-jpeg`, `png`. Dev-only crates
are fine.

## Global Constraints

- **Alex and Alex the bun look exactly as today.** `tests/parity.rs` stays at 0 px different.
- No network. Photos are never written to disk, not even their path; they are dropped when the maker closes.
- `photo::*` and `ui::maker` know nothing about the OS or windows. Only `platform` and `app.rs` do.
- A bad photo (corrupt, truncated, 0×0, a 30000×30000 header, wrong extension) is an error message in the
  maker, never a panic or an out-of-memory.
- Copy uses only printable ASCII. Pronoun-neutral for the bun.
- §9 budget holds once the maker is closed; exe ≤ 6 MB.
- CI green on Windows, Linux, macOS. Rushi's PixelPet may be running from `target\release`: ask before a
  release build.

## Review Focus

1. **Hostile files:** pinned by `bad_files_are_errors_not_panics` (Task 2): truncated JPEG, PNG with a huge
   declared size (rejected from the header, before decoding), random bytes, empty file.
2. **Phone photos lying sideways:** EXIF orientation 6 and 8 come out upright. `exif_orientation_is_applied`
   (Task 2).
3. **Missing points:** any subset of points skipped, any pose, gives a `Fit` whose unset sliders equal the
   base design's and whose values are inside the v2 slider ranges. `any_skips_stay_in_range` (Task 4).
4. **Alex's own photos give Alex:** local-only `#[ignore]` test `alex_photos_give_about_alex` (Task 4): sliders
   within ±0.1 of 1.0 where set, colours within ΔE 15 of Alex's base shades.
5. **Clicking accuracy:** a click maps to the photo pixel under the cursor at every Size zoom, on photos of
   any aspect ratio. `photo_clicks_map_to_photo_pixels` (Task 5).

---

## Interfaces (written by Task 1, used by all)

```rust
// src/photo/mod.rs
pub mod load;
pub mod measure;
/// An upright RGB photo, long side at most `MAX_SIDE`.
pub struct Photo { pub w: u32, pub h: u32, pub rgb: Vec<u8> }
pub const MAX_SIDE: u32 = 1024;

// src/photo/load.rs
pub enum LoadError { Io(std::io::Error), NotAPhoto, TooBig }
pub fn open(path: &Path) -> Result<Photo, LoadError>;
pub fn decode(bytes: &[u8]) -> Result<Photo, LoadError>;

// src/photo/measure.rs
pub enum Pose { Standing, Sitting, Other }
pub enum Point { Nose, Eye, EarTip, Shoulders, TailBase, TailTip, Chest, FrontPaw, BackPaw }
pub fn points(pet: Pet) -> &'static [Point];          // the order the maker asks them in
pub fn prompt(pet: Pet, p: Point) -> &'static str;     // "Click the nose"
pub struct Marked<'a> { pub photo: &'a Photo, pub pose: Pose, pub at: Vec<(Point, (f32, f32))> } // photo px
pub struct Fit { pub shape: Shape, pub coat: Vec<(i8, u32)>, pub set: Vec<&'static str> }      // set: slider names
pub fn fit(base: &Design, photos: &[Marked]) -> Fit;

// src/platform/mod.rs, on trait Platform
/// The OS file dialog for JPEG/PNG, several at once. Blocks until closed.
/// None: no dialog on this system (Linux without zenity/kdialog). Some(empty): cancelled.
fn pick_photos(&self) -> Option<Vec<PathBuf>>;

// src/ui/maker.rs
// Step::Photos; Outcome::PickPhotos (the app opens the dialog);
// Maker::add_photo(&mut self, Photo), Maker::photo_error(&mut self, &str),
// Maker::click_at(&mut self, x: f32, y: f32) -> Outcome   (UI px as floats; click() delegates to it)
```

## File ownership

| Task | Model | Owns |
|---|---|---|
| 1 Scaffold | Opus (lead) | `Cargo.toml`, `src/photo/mod.rs`, stubs of `load.rs`/`measure.rs`, `Platform::pick_photos` + `Stub`/`Fake`, `Step::Photos`/`Outcome::PickPhotos` stubs |
| 2 Load | Sonnet | `src/photo/load.rs`, `tests/photo_load.rs`, `tests/fixtures/photo/*` |
| 3 Dialogs | Sonnet | `src/platform/windows.rs`, `macos.rs`, `linux.rs`, `windows-sys`/`objc2-app-kit` features in `Cargo.toml` |
| 4 Measure | Sonnet | `src/photo/measure.rs`, `tests/photo_measure.rs` |
| 5 Photos step | Sonnet | `src/ui/maker.rs`, `tests/maker.rs` |
| 6 Wiring + integration | Opus (lead) | `src/app.rs`, merges, screenshots, CI |
| 7 Docs | Haiku | `docs/status.md`, `docs/release.md`, `docs/release-notes.md` |

---

### Task 1: Scaffold (lead, before the parallel tasks)

- [ ] Add `zune-jpeg` and `png` to `[dependencies]`; `cargo build`; note the exe size change.
- [ ] Write the interfaces above with stub bodies: `load` returns `Err(NotAPhoto)`; `fit` returns the base
      design's shape and coat with `set` empty; `points`/`prompt` real (they're the spec's lists).
      `pick_photos` returns `None` on every OS for now; `Fake` records calls and returns a set list.
- [ ] Maker: `Step::Photos` exists but is unreachable; `Outcome::PickPhotos` exists.
- [ ] `cargo test` passes (159 + parity). Commit `photo: scaffold for v3`.

### Task 2: Loading photos (Sonnet, parallel)

- [ ] Tests first, with tiny fixtures made by a test helper or committed (< 20 KB total, synthetic, never a
      real pet photo): a 3×2 PNG with known pixels; the same as JPEG (compare with tolerance); EXIF
      orientations 1, 3, 6, 8 (JPEGs with an APP1 orientation tag; a dev-only encoder crate may make them);
      RGBA and greyscale and 16-bit PNG; a 4000×3000 image comes out 1024×768; the hostile cases in Review
      Focus 1.
- [ ] `decode`: sniff the magic bytes (not the extension); read the header size first and return `TooBig`
      above 50 megapixels; decode; apply EXIF orientation (parse only the APP1 orientation tag by hand, no
      exif crate); convert to RGB (alpha composited on white); box-downscale to `MAX_SIDE`.
- [ ] `open` reads the file (refuse files over 64 MB as `TooBig`) and calls `decode`.
- [ ] `cargo test --test photo_load` and the full suite pass. Commit `photo: load JPEG and PNG, upright and small`.

### Task 3: File dialogs (Sonnet, parallel)

- [ ] Windows: `IFileOpenDialog` (COM, `FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST`, filter
      "Photos (*.jpg;*.jpeg;*.png)"), via `windows-sys` features added for it; CoInitialize as apartment-threaded
      on the calling thread. Owner: none (the maker window is topmost already; check it doesn't open behind).
- [ ] macOS: `NSOpenPanel` via `objc2-app-kit` (already in the lock file at 0.2.2; add only features),
      `allowsMultipleSelection`, allowed types jpeg/png, `runModal` on the main thread.
- [ ] Linux: run `zenity --file-selection --multiple --separator='\n' --file-filter='Photos | *.jpg *.jpeg *.png'`,
      else `kdialog --getopenfilename --multiple --separate-output ~ 'image/jpeg image/png'`, else `None`.
      Parsing the output is a pure function in `src/platform/pure.rs`'s style with unit tests (paths with
      spaces, trailing newline, cancel = exit code 1 → `Some(vec![])`).
- [ ] Try the Windows dialog by hand (a throwaway `examples/pick.rs` is fine, delete before commit).
      macOS/Linux: `cargo check --target` where possible, else rely on CI.
- [ ] Commit `platform: a photo file dialog on each OS`.

### Task 4: Points to sliders and colours (Sonnet, parallel)

- [ ] Read the prototype script and spec §4–5. Read `src/rig/design.rs`, `src/rig/breeds.rs` (slider ranges,
      which `fam` each Coat area uses) and the `ALEX`/`BUN` coats; nothing else is needed.
- [ ] Geometry: ratios against the shoulders-to-tail-base length, per spec. Which slider each ratio sets:
      Legs (shoulders→front paw, Standing only), Body (shoulders→tail base over head length), Head
      (nose→ear-tip span), Ears (ear tip above eye), Muzzle (nose→eye), Tail (tail base→tip, Standing only).
      Rabbit: Head, Ears, Feet as the points allow. Median across photos; divide by Alex's (or the bun's)
      reference ratios, stored as constants measured from the prototype's taps on `alex/` photos (comment
      where they came from); clamp to the breed's slider range. Fluff stays at base.
- [ ] Colour: grey-world white balance; Lab k-means (k = 3, fixed seed, a few iterations, on at most ~4000
      sampled pixels per region); background reject (distance to the border median), pink and highlight
      reject; saddle = darkest cluster of a two-sided band; chest = lightest; others = dominant. Pool
      across photos (median in Lab). Map to the design's areas.
- [ ] Tests with synthetic photos drawn in the test (flat-colour "dog" rectangles on a background, with a
      colour cast): known ratios come back; known colours come back within ΔE 5 after the cast; Review
      Focus 3 as a property test over every subset of skipped points and every pose; the base design is
      returned unchanged for zero photos. Plus the local-only `#[ignore]` Alex test (Review Focus 4), which
      reads `alex/` if present and skips cleanly if not.
- [ ] Commit `photo: points set the sliders and colours`.

### Task 5: The Photos step in the maker (Sonnet, parallel)

- [ ] Read `src/ui/maker.rs`, `tests/maker.rs`, the spec §3–4, and v2 spec §3. Match the existing style.
- [ ] Breed list gets "From photos..." last; picking it sets the base to the species' first breed and
      inserts `Step::Photos` after Breed (dots: 6 on this path). Back/Next/jumping work as in v2.
- [ ] The Photos step: "Open photo..." emits `Outcome::PickPhotos`; tabs "1..5" with an "x"; the pose row
      (Standing | Sitting | Other); the prompt line; the photo drawn at screen resolution in the right-hand
      area, fit and centred, with numbered markers; Skip; right-click undoes the last point; markers drag.
      The preview pet on the left shows a marker on the part being asked for (a small table of points on
      the pet's stand pose per species is fine).
- [ ] `photo_error(msg)` shows one line for a few seconds. Up to 5 photos; a 6th shows "Up to 5 photos".
- [ ] Next from Photos calls `measure::fit` and starts Shape/Coat from it; the Coat rows get the photo colour
      as an extra first swatch. Back to Photos keeps the points. Editing a saved pet never shows Photos.
- [ ] Tests in `tests/maker.rs` style: the flow and dots; Review Focus 5; markers, skip, undo; 6th photo
      refused; Next with no points keeps the base design; `all_copy()` covers the new copy.
- [ ] Commit `ui: the maker's Photos step`.

### Task 6: Wiring and integration (lead)

- [ ] Merge 2–5 into `v3` (they own disjoint files; fix any seams).
- [ ] `app.rs`: on `Outcome::PickPhotos`, call `platform.pick_photos()`; `None` → `photo_error("Drop photos
      here")`; load each with `photo::load::open` (errors → `photo_error`). `WindowEvent::DroppedFile` on the
      maker window does the same. Pass the cursor to `click_at` as UI-px floats.
- [ ] Screenshots on this laptop (`tools/screenshot.ps1`): Photos step with a photo of Alex and markers,
      then Shape and Coat as filled in. Alex built-in still pixel-identical (`PIXELPET_PIN=stand`).
- [ ] CI on all three (`gh workflow run ci.yml`); a Haiku agent reads failing logs if any.
- [ ] Budget: `tools/measure.ps1` after a photo pet is made and the maker closed; exe size.
- [ ] Commit `app: photos into the maker`.

### Task 7: Docs (Haiku)

- [ ] `docs/status.md` v3 entry; `docs/release.md` checklist (dialog on each OS, drag and drop, sideways phone
      photo, Linux without zenity); `docs/release-notes.md` one line: "Make a pet from your own photos".
- [ ] Commit `docs: v3 status and release notes`.

## Not in v3

- HEIC, video, cats, new shapes, automatic point finding.
