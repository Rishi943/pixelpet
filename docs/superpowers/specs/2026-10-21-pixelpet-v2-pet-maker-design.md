# PixelPet v2 — the pet maker (design)

Date: 2026-10-21 · Status: agreed in chat with Rushi, written up for review

Builds on the v1 spec (`2026-10-20-pixelpet-v1-design.md`) and the shipped M1–M3 code. Everything v1 says
still holds unless this document changes it.

## 1. What it is, and why

People make their own pet, the way you make a character in an RPG (Rushi's references: the Cyberpunk 2077 and
Skyrim character creators). You pick an animal, start from a breed, shape it, colour it, name it, and see the
real pet change as you go. Rushi: *"set it up, customise as they want and see a preview right there, like in an
rpg game."*

People will use it both to **invent** a pet and to get close to a **real** one. v2 is the inventing half.
**v3** (later, not this document) fills the same controls in automatically from uploaded photos, so v2's
controls must be the ones a photo could set: proportions and colours on the existing skeleton.

The approved look of the maker is the mockup in **`drafts/maker/index.html`**: open it in a browser (it runs
the real JS rig from `drafts/looks/`). Rushi signed it off: *"perfect"*. It is the design reference for layout,
copy and behaviour. The Rust app draws the same thing.

## 2. Goals, non-goals, anti-goals

**Goals**

- A small pet-maker window inside PixelPet, in the app's own pixel style, with a live preview of the real pet.
- A five-step flow, not tabs: Rushi asked for *"a flow so people go through all the options"*.
- **First launch opens the maker by itself**, because no pet has been picked yet.
- Pets people make are **saved** and listed in the right-click menu to switch between (Rushi: *"yes we can
  have them saved"*).
- "Random" rolls a whole new pet at any step.

**Non-goals for v2**

- **Cats.** They're shown as "Cat (later)" and need new art.
- **New shapes:** floppy ears, curly tails, spots, merle and so on. These need new drawing work in the rig.
  The breeds in v2 are made from proportions and colours on the existing skeleton.
- Building a pet from photos (that's v3).
- More than one pet on screen at a time (still parked, as in v1).

**Anti-goals: these would make v2 a failure even if everything technically works**

- **Alex or Alex the bun look any different from today.** These are memorial pets. Picking "Alex the GSD"
  with no changes, or "Alex the bun", must give exactly today's pixels. The parity tests in
  `tests/parity.rs` must stay at 0 px different.
- A maker that opens in a web browser, or anything that listens on a port. v1's "no network access of any
  kind" still holds. Rushi agreed the maker is a window in the app.
- A big, full-screen editor. Rushi: *"make it small and min… smaller than a full size window if we dont need
  the space"*.
- Any chore, hunger or guilt mechanic (v1 §2, unchanged).

## 3. The window

A separate PixelPet window, drawn entirely by the app in the same pixel font, colours and outline as the
right-click menu (`src/ui/menu.rs`: fill `#F3D29C`, outline/text `#24120A`, selected `#2F64C8` with white
text). Bigger than the menu, much smaller than a normal app window.

- **Size:** 236 × 132 UI pixels, drawn at the menu's zoom for the current Size setting (M = ×3, about
  708 × 396 screen px). It is centred on the primary monitor's work area.
- **It is a normal, focusable window** while open, because it takes typing. It has no taskbar entry, like
  the pet. It closes with Esc, or with a small "x" at the top right.
- **Layout** (as in the mockup):
  - The title "Make a pet", with five step dots to its right. The current step is blue, done steps are dark,
    and future ones are hollow. Clicking a done step jumps back to it.
  - On the left, a **preview box** showing the real pet at its actual on-screen size (the current Size
    setting), standing on a ground line, with its name above.
  - On the right, the step's title and "N/5", then the step's controls.
  - Along the bottom, "Random" on the left, "< Back" in the middle (from step 2) and "Next >" on the right.
    The last step shows "Done".
- **The preview acts out what the step changes:** stand on steps 1–2, walk while shaping, sit while
  colouring, head tilt (the bun: its stand-up-tall equivalent) while naming, and a happy hop on Done. It
  updates on every change, with no lag a person would notice.

## 4. The five steps

1. **Animal:** Dog, Rabbit, and "Cat (later)" greyed out. Choosing one moves straight on to step 2.
2. **Breed:** a list. Choosing one sets the shape and colours and shows in the preview at once.
   - **Dog:** **Alex the GSD** first and selected by default (exactly Alex, Rushi 2026-10-21), then Yellow
     Lab, Black Lab, Husky, Golden, Corgi.
   - **Rabbit:** the first entry is exactly Alex the bun, plus Grey and Black & white.
   - The mockup's breed values (in `drafts/maker/index.html`) are first guesses, to be tuned by eye in
     screenshots of the real app.
3. **Shape:** sliders.
   - **Dog:** Legs, Body, Head, Ears, Muzzle, Tail, Fluff.
   - **Rabbit:** Head, Ears, Fluff, Feet.
   - Each slider is a multiplier on that part's breed value (the mockup's ranges, e.g. Legs 0.6–1.3).
     Dragging and clicking both work.
4. **Coat:** one row per area, each a row of colour swatches.
   - **Dog areas:** Coat, Saddle, Legs, Mask, Collar.
   - **Rabbit areas:** Coat, Patches, Ears.
   - Picking a colour sets that area. Its four shades (dark, shadow, base, light) are worked out from the one
     colour, as the mockup's `family()` does.
   - The breed's own colours stay exact until a swatch is clicked. That is how Alex keeps his hand-tuned coat.
5. **Name:** a text box with a cursor. Typing, Backspace and Enter (= Done) work, up to 24 characters of the
   app font's printable ASCII. The default is the breed's name, or "Alex" / "Alex the bun" for the first
   breeds. Random picks a name from a short list.

"Done" saves the pet, makes it the pet on screen, and closes the window. The pet then hops onto the taskbar
(its normal entrance, as after a drag).

## 5. Saved pets and the menu

- A **pet** is now: species, breed, body values, coat colours, collar colour, and name.
- Pets are saved as files next to `settings.txt`, in a `pets/` folder, one small file per pet. They use the
  same plain `key = value` style as the settings file and keep unknown keys, so a v3 file still loads in v2.
- **Alex and Alex the bun always exist.** They are built in, can't be deleted, and are shown first. Existing
  users keep their current pet: `pet = alex` / `alex-the-bun` in `settings.txt` still works.
- **Right-click menu:** the "Pet:" line stays one row: **"Pet: < Alex >"**. The arrows step through every
  saved pet (Alex, Alex the bun, then the user's, in the order they were made), and the pet on screen
  changes as you step (Rushi: *"arrows work"*). A new line, **"Make a pet…"**, opens the maker. Clicking
  the current pet's name opens the maker on it, to edit it.
- **First launch** means no `settings.txt` exists yet. The maker opens by itself, and Alex stands on the
  taskbar behind it. Closing the maker without finishing leaves Alex as the pet, and the maker doesn't reopen
  next time.

## 6. Constraints

- Everything in v1 §1 and §7 still holds:
  - One portable file per OS, no installer, no network.
  - No new crates beyond what's in `Cargo.lock` (dev-only crates are fine).
  - The rig, brain and platform boundaries stay as they are.
  - It runs on Windows, macOS and Linux, and CI's screenshot smoke tests still pass.
- **The rig takes a pet's body and coat as values.** Today `dog::Body` and `rabbit::Body` are chosen by look
  name, and `Coat` is a `'static` per species. Both must become per-pet data passed into posing and
  rendering. Alex's and the bun's values stay exactly today's numbers.
- **Every anim and interaction still works on every made pet:** wander, nap, hover, drag, pet, fetch, feed,
  watch. This is why v2 only changes proportions and colours on the existing skeleton.
- The §9 performance budget still holds for a made pet (sleep ≤ 0.5 % CPU, walk ≤ 1.5 %, ≤ 30 MB,
  exe ≤ 6 MB, measured on Windows). The maker window may use more while it's open.
- Likeness check, as for every art change: screenshots of the real app, with Alex and the bun compared to
  today's pixels.

## 7. Decided in chat (Rushi's calls)

- It's a window inside PixelPet, not a browser page (agreed 2026-10-21).
- It's a step-by-step flow, not tabs.
- In the menu, the "Pet:" line becomes "< name >" arrows.
- Alex the GSD is the default, and is named as such in the breed list.
- The look is minimal and pixel-styled. The window is small, bigger than the menu, smaller than a full
  window.
- The first launch opens the maker by itself.
- Made pets are saved and can be switched between.
- Inventing a pet is v2; building from photos is v3.

## 8. My calls (Claude's, for Rushi to check while reading)

- **Breed names, slider ranges, swatch colours and default names** are the mockup's, pending tuning by eye.
- **Where pets are stored:** a `pets/` folder next to `settings.txt`, one file per pet. In portable mode that
  means next to the program.
- **Editing and deleting:**
  - Picking the current pet in the menu opens the maker to edit it.
  - Deleting is a "Delete" button on the maker's Name step for user-made pets only, with a "Delete <name>?
    Yes | No" confirmation, like Quit.
- **Closing on first launch** leaves Alex.
- **A small "x" close button** at the top right, plus Esc. The mockup has neither.
- **Collar:** today the desktop never draws one, so the Collar row gets a "None" swatch first. The built-in
  Alex has no collar, and a fresh "Alex the GSD" starts with none, so both match today's pixels. Any other
  swatch puts a collar on the pet, on the desktop too.
- **Alex and Alex the bun are never changed.** Opening either in the maker and pressing Done saves a new pet,
  and the built-in stays as it is. Pets are told apart by a hidden id, not their name, so two pets can share
  a name. Making an unchanged "Alex the GSD" from "Make a pet…" still saves a new pet, named Alex.
- **First launch** = no `settings.txt`. v1 users who never changed a setting have none either, so they see
  the maker once on upgrade, with Alex behind it; closing it keeps Alex. Opening the maker writes
  `settings.txt`, so it doesn't reopen.
- **The rabbit's sliders** (Head, Ears, Fluff, Feet) come from the rabbit rig's existing proportions. The
  mockup didn't expose them.
- The maker's window is focusable and takes key input only while open. It reads typed characters for the
  name and nothing else, and only from its own window.

## 9. Left to the builder

- How to hand per-pet body and coat through the rig without breaking the golden parity tests.
- The maker's internal structure (state, hit-testing, drawing), following `src/ui/menu.rs` and
  `src/ui/bubble.rs`.
- The pet file's exact keys.
- How the first-launch check works on each OS.
- The order of work and the test plan: the brain tests, UI unit tests in the style of `tests/menu.rs`, and
  screenshots of each step in the real app.
