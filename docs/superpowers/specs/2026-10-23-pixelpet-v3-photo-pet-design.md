# PixelPet v3 — a pet from photos (design)

Date: 2026-10-23 · Status: draft, written up for Rushi's review

Builds on the v1 spec (`2026-10-20-pixelpet-v1-design.md`), the v2 spec (`2026-10-21-pixelpet-v2-pet-maker-design.md`)
and the shipped v2 code. Everything they say still holds unless this document changes it.

## 1. What it is, and why

v2 lets people invent a pet. v3 is the other half: getting close to a **real** pet. A person gives the maker one
or more photos of their pet, clicks a few points on each, and the maker sets the v2 **shape sliders and coat
colours** from them. The person then finishes by hand in the usual Shape, Coat and Name steps.

Rushi's calls (2026-10-23): photos set **colours and proportions**; the person **clicks guided points** (no
model, nothing automatic that could guess wrong silently); file picker, drag and drop, and several photos.

## 2. Goals, non-goals, anti-goals

**Goals**

- A "From photos..." path in the maker that ends in the normal Shape step with the sliders already set, and
  the Coat step with the colours already set.
- Works on any photo a person has: busy backgrounds, indoor light, a pet that isn't centred.
- Several photos (up to 5) give steadier results than one; one is enough.
- A point that isn't visible in a photo (tail behind the pet, a paw cut off) can be skipped.

**Non-goals for v3**

- Cats and new shapes (floppy ears, spots, merle): still need new art, as in v2.
- Video. Photos only.
- HEIC (the iPhone default). JPEG and PNG only; the picker says so.
- Telling the species from the photo: the person picks Dog or Rabbit first, as today.

**Anti-goals: these would make v3 a failure even if everything technically works**

- **Alex or Alex the bun look any different from today.** Parity stays at 0 px different.
- **Any network access**, or any photo leaving the computer. v1's "no network of any kind" holds.
- **Keeping the photos.** The maker reads them, measures them and forgets them. Nothing about a photo (not
  its path, not a thumbnail) is written to disk; the pet file holds only the slider values and colours, as
  in v2.
- A results screen that hides what happened. Every value a photo set is a normal slider or swatch the person
  can see and change.

## 3. The flow

The v2 maker gains one entry and one step.

1. **Animal:** unchanged.
2. **Breed:** the list gets a last entry, **"From photos..."**. Picking it starts from the species' first
   breed (Alex the GSD / Alex the bun) as the base and inserts a **Photos** step after this one. The step
   dots grow from five to six on this path.
3. **Photos** (new): add photos, click the points on each. See §4.
4. **Shape:** unchanged, but the sliders start at the values the photos gave.
5. **Coat:** unchanged, but each area starts at the photo's colour, shown as an extra first swatch in its
   row (so the person can go back to it after trying others).
6. **Name:** unchanged.

Back from Shape returns to Photos with its points kept. Editing a saved pet never reopens Photos (the photos
are gone); it goes through the v2 steps.

## 4. The Photos step

- **Adding photos:** an "Open photo..." button opens the system's file dialog (JPEG/PNG, several at once).
  Dropping image files on the maker window adds them too. Up to 5; extra ones are ignored with a short note.
  Photos are shown as numbered tabs ("1 2 3") above the photo, with a small "x" to remove one.
- **The photo** fills the step's area at screen resolution (not pixelated, so points can be placed
  accurately), scaled to fit, turned the right way up (JPEG orientation tag).
- **Points:** one prompt line at a time, e.g. **"Click the nose"**, with the preview pet on the left showing
  a marker on that same part. Each click drops a numbered marker and moves to the next point.
  - "Skip" for a point that isn't visible; right-click removes the last marker.
  - Markers can be dragged to fix them.
  - Asks for a side-on photo, standing or walking. A photo where the pet faces the camera can still give
    colours; its points are optional.
- **The point set** (to be settled by the prototype, see §9):
  - **Dog:** nose tip, ear tip, top of shoulders, base of tail, tail tip, front paw, back paw.
  - **Rabbit:** nose tip, ear tip, top of back, tail, front foot, back foot.
- **"Next"** is enabled once one photo has at least the points needed for one value; values with no points in
  any photo stay at the base breed's.

## 5. From points to sliders and colours

- **Proportions:** each slider comes from a ratio of distances between points, e.g. leg length (shoulders to
  front paw) over body length (shoulders to tail base). Ratios don't depend on the photo's size or distance.
  Each ratio is divided by the same ratio measured on the base breed's own photos, so the built-in pet's
  photos give 1.0 on every slider. With several photos, the **median** is used. Results are clamped to the
  v2 slider ranges.
- **Colours:** the points mark out regions (the saddle between shoulders and tail base, the legs above the
  paws, the muzzle behind the nose, the ear below its tip). Each region's colour is a robust pick over its
  pixels (most common cluster, ignoring the background at the region's edges), pooled across photos. The
  colour sets the v2 area (Dog: Coat, Saddle, Legs, Mask; Rabbit: Coat, Patches, Ears); its four shades come
  from v2's `shades()` as for any swatch.
- **Fluff, Head and Feet** have no reliable points in v3: they stay at the base breed's values.

## 6. Constraints

- Everything in v1 §1 and §7 and v2 §6 holds, except one change Rushi has to approve:
  - **New crates for reading photos.** Nothing in `Cargo.lock` decodes JPEG or PNG. Proposed: the small
    pure-Rust `zune-jpeg` and `png` crates, no C code, same on every OS. The alternative is each OS's own
    decoder (WIC, ImageIO), but Linux has no standard one, so a crate is needed anyway.
- **File dialogs** use each OS's own: the Windows common dialog, the macOS open panel (through `objc2`,
  already in the lock file), and on Linux `zenity` or `kdialog` if present. If neither is, the button says
  "Drop a photo here" and drag and drop is the way in.
- **Memory:** photos are cut down to at most 1024 px on the long side when loaded, and dropped when the maker
  closes. The §9 budget holds once the maker is closed; the exe stays ≤ 6 MB.
- The photos in `alex/` and `alex the bun/` are private and never committed. Tests in CI use small synthetic
  images; checking against the real photos is a local-only test.

## 7. Decided in chat (Rushi's calls)

- Photos set colours and proportions (2026-10-23).
- Guided clicks, not an automatic cut-out or a bundled model.
- File picker, drag and drop, and several photos; the details are Claude's.
- v2 merged to `main` before v3 starts.

## 8. My calls (Claude's, for Rushi to check while reading)

- "From photos..." sits at the end of the Breed list, not as a new first step.
- Up to 5 photos.
- Photos are never kept, not even as a thumbnail on the pet.
- JPEG and PNG only; no HEIC.
- The photo path starts from Alex the GSD / Alex the bun as the base, so a photo of Alex gives Alex's
  proportions.

## 9. Left to the builder

- The final point set and region shapes, from the prototype on the `alex/` photos.
- The exact ratio for each slider, and the reference values measured on the built-in pets.
- How the photo is drawn inside the pixel-styled maker window on each OS.
- The order of work and the test plan.
