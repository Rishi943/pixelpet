//! The pet maker: a small window that walks through five steps (animal,
//! breed, shape, coat, name) beside a live preview of the pet, drawn in the
//! menu's pixel style. Pure state and drawing: the app feeds it clicks,
//! drags and keys, and hands it the preview frame. Layout and copy follow
//! the approved mockup in `drafts/maker/index.html`.

use super::font;
use crate::pets::{self, clean_name, is_builtin, PetRecord, NAME_MAX};
use crate::rig::breeds::{areas, breeds, sliders, COLLARS, SWATCHES};
use crate::rig::{Anim, Design, Pet};
use crate::window::Rgba;

/// Window size in UI pixels (drawn at the menu's zoom for the Size setting).
pub const W: usize = 236;
pub const H: usize = 132;

const OUTLINE: [u8; 3] = [0x24, 0x12, 0x0A];
const FILL: [u8; 3] = [0xF3, 0xD2, 0x9C];
const SHADE: [u8; 3] = [0xE7, 0xC0, 0x80];
const TEXT: [u8; 3] = [0x24, 0x12, 0x0A];
const DIM: [u8; 3] = [0x8E, 0x6E, 0x48];
const SEL: [u8; 3] = [0x2F, 0x64, 0xC8];
const SEL_TEXT: [u8; 3] = [0xFF, 0xFF, 0xFF];
const NAME_BG: [u8; 3] = [0xFF, 0xF6, 0xE4];

// the preview box and the right-hand column
const PX: i32 = 6;
const PY: i32 = 18;
const PW: i32 = 92;
const PH: i32 = 96;
const GROUND: i32 = PY + PH - 10;
const RX: i32 = 106;
const RW: i32 = W as i32 - RX - 6;
const Y0: i32 = PY + 14;
// a slider's bar, and a swatch row's first swatch
const BAR_X: i32 = RX + 40;
const BAR_W: i32 = RW - 40;
const SW_X: i32 = RX + 40;
const SW_STEP: i32 = 9;

const STEPS: [&str; 5] = ["Animal", "Breed", "Shape", "Coat", "Name"];
const NAMES: [&str; 8] = ["Biscuit", "Pixel", "Mango", "Pepper", "Comet", "Noodle", "Toffee", "Bean"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Animal,
    Breed,
    Shape,
    Coat,
    Name,
}

const ALL_STEPS: [Step; 5] = [Step::Animal, Step::Breed, Step::Shape, Step::Coat, Step::Name];

#[derive(Debug)]
pub enum Outcome {
    Nothing,
    Redraw,
    /// Save this pet. An empty id means a new pet (the app gives it one).
    Done(PetRecord),
    Delete(String),
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MakerKey {
    Char(char),
    Backspace,
    Enter,
    Esc,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hit {
    Dot(usize),
    Animal(Pet),
    Breed(usize),
    Slider(usize),
    Swatch(i8, u32),
    CollarNone,
    Collar(u32),
    Random,
    Back,
    Next,
    Close,
    Delete,
    Yes,
    No,
}

struct Region {
    name: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    hit: Hit,
}

pub struct Maker {
    step: usize,
    reached: usize,
    breed: usize,
    design: Design,
    name: String,
    /// the name is still one the maker chose (so a new breed may change it)
    auto_name: bool,
    /// the pet being edited ("" for a new pet or an edited built-in)
    id: String,
    extra: Vec<(String, String)>,
    deletable: bool,
    confirm_delete: bool,
    drag: Option<usize>,
    rng: u64,
}

impl Maker {
    pub fn new(seed: u64) -> Maker {
        let mut m = Maker {
            step: 0,
            reached: 0,
            breed: 0,
            design: Design::alex(),
            name: String::new(),
            auto_name: true,
            id: String::new(),
            extra: vec![],
            deletable: false,
            confirm_delete: false,
            drag: None,
            rng: seed | 1,
        };
        m.apply_breed(Pet::Dog, 0);
        m
    }

    /// The maker on an existing pet, from the Breed step. A built-in is
    /// copied: Done saves a new pet and the built-in stays as it is.
    pub fn edit(r: &PetRecord, seed: u64) -> Maker {
        let pet = r.design.pet;
        let breed = breeds(pet).iter().position(|b| b.name == r.breed).unwrap_or(0);
        let builtin = is_builtin(&r.id);
        Maker {
            step: 1,
            reached: STEPS.len() - 1,
            breed,
            design: r.design.clone(),
            name: r.name.clone(),
            auto_name: false,
            id: if builtin { String::new() } else { r.id.clone() },
            extra: r.extra.clone(),
            deletable: !builtin,
            confirm_delete: false,
            drag: None,
            rng: seed | 1,
        }
    }

    pub fn step(&self) -> Step {
        ALL_STEPS[self.step]
    }

    pub fn design(&self) -> &Design {
        &self.design
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// What the preview pet does on this step, to show off what it changes.
    pub fn preview_anim(&self) -> Anim {
        match self.step() {
            Step::Animal | Step::Breed => Anim::Stand,
            Step::Shape => Anim::Walk,
            Step::Coat => Anim::Sit,
            Step::Name => Anim::Tilt,
        }
    }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[((self.rand() * from.len() as f64) as usize).min(from.len() - 1)]
    }

    fn apply_breed(&mut self, pet: Pet, i: usize) {
        self.breed = i;
        self.design = (breeds(pet)[i].design)();
        if self.auto_name {
            self.name = if i == 0 {
                pets::builtins().into_iter().find(|b| b.design.pet == pet).map(|b| b.name).unwrap_or_default()
            } else {
                self.pick(&NAMES).to_string()
            };
        }
    }

    fn go(&mut self, i: usize) {
        self.step = i.min(STEPS.len() - 1);
        self.reached = self.reached.max(self.step);
        self.confirm_delete = false;
    }

    fn randomise(&mut self) {
        let pet = self.design.pet;
        for s in sliders(pet) {
            let v = s.lo + self.rand() * (s.hi - s.lo);
            self.design.shape.set(s.field, v);
        }
        for &(f, _) in areas(pet) {
            let c = self.pick(&SWATCHES);
            self.set_coat(f, c);
        }
        if pet != Pet::Rabbit {
            self.design.collar = Some(self.pick(&COLLARS));
        }
        if self.auto_name || self.step() != Step::Name {
            self.name = self.pick(&NAMES).to_string();
            self.auto_name = true;
        }
    }

    fn set_coat(&mut self, f: i8, c: u32) {
        match self.design.coat.iter_mut().find(|(cf, _)| *cf == f) {
            Some(slot) => slot.1 = c,
            None => self.design.coat.push((f, c)),
        }
    }

    /// An area's colour now: the design's, else the species coat's base.
    fn coat_of(&self, f: i8) -> u32 {
        if let Some(&(_, c)) = self.design.coat.iter().find(|(cf, _)| *cf == f) {
            return c;
        }
        self.design.species_coat().fams.iter().find(|(cf, _)| *cf == f).map_or(0, |(_, v)| v[2])
    }

    fn set_slider(&mut self, i: usize, x: i32) {
        let s = &sliders(self.design.pet)[i];
        let v = (x - BAR_X) as f64 / BAR_W as f64;
        let val = if v <= 0.0 {
            s.lo
        } else if v >= 1.0 {
            s.hi
        } else {
            s.lo + v * (s.hi - s.lo)
        };
        self.design.shape.set(s.field, val);
    }

    fn done(&self) -> Outcome {
        let breed = &breeds(self.design.pet)[self.breed];
        let name = Some(clean_name(&self.name)).filter(|n| !n.is_empty()).unwrap_or_else(|| breed.name.to_string());
        Outcome::Done(PetRecord {
            id: self.id.clone(),
            name,
            breed: breed.name.to_string(),
            design: self.design.clone(),
            extra: self.extra.clone(),
        })
    }

    /// A click at UI pixel `(x, y)`.
    pub fn click(&mut self, x: i32, y: i32) -> Outcome {
        self.drag = None;
        let (_, regions) = self.layout();
        let Some(hit) = regions.iter().find(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h).map(|r| r.hit) else {
            return Outcome::Nothing;
        };
        let pet = self.design.pet;
        match hit {
            Hit::Dot(i) => self.go(i),
            Hit::Animal(p) => {
                self.apply_breed(p, 0);
                self.go(1);
            }
            Hit::Breed(i) => self.apply_breed(pet, i),
            Hit::Slider(i) => {
                self.drag = Some(i);
                self.set_slider(i, x);
            }
            Hit::Swatch(f, c) => self.set_coat(f, c),
            Hit::CollarNone => self.design.collar = None,
            Hit::Collar(c) => self.design.collar = Some(c),
            Hit::Random => self.randomise(),
            Hit::Back => self.go(self.step.saturating_sub(1)),
            Hit::Next if self.step == STEPS.len() - 1 => return self.done(),
            Hit::Next => self.go(self.step + 1),
            Hit::Close => return Outcome::Close,
            Hit::Delete => self.confirm_delete = true,
            Hit::No => self.confirm_delete = false,
            Hit::Yes => return Outcome::Delete(self.id.clone()),
        }
        Outcome::Redraw
    }

    /// The cursor moved to `(x, y)` with the button held: drags a slider
    /// grabbed by the last click.
    pub fn drag(&mut self, x: i32, _y: i32) -> Outcome {
        match self.drag {
            Some(i) => {
                self.set_slider(i, x);
                Outcome::Redraw
            }
            None => Outcome::Nothing,
        }
    }

    pub fn key(&mut self, k: MakerKey) -> Outcome {
        match k {
            MakerKey::Esc if self.confirm_delete => self.confirm_delete = false,
            MakerKey::Esc => return Outcome::Close,
            MakerKey::Enter if self.step() == Step::Name => return self.done(),
            MakerKey::Enter => self.go(self.step + 1),
            MakerKey::Char(c) if self.step() == Step::Name => {
                if !(' '..='~').contains(&c) || self.name.chars().count() >= NAME_MAX {
                    return Outcome::Nothing;
                }
                self.name.push(c);
                self.auto_name = false;
            }
            MakerKey::Backspace if self.step() == Step::Name => {
                self.name.pop();
                self.auto_name = false;
            }
            _ => return Outcome::Nothing,
        }
        Outcome::Redraw
    }

    /// Where `what` is, in UI pixels: a button's label ("Next >", "Husky",
    /// "x"), `dot:N`, `slider:<label>`, `swatch:<area>:<i>` or
    /// `swatch:Collar:none`.
    pub fn target(&self, what: &str) -> Option<(i32, i32, i32, i32)> {
        self.layout().1.into_iter().find(|r| r.name == what).map(|r| (r.x, r.y, r.w, r.h))
    }

    /// The window at `scale`, with `preview` (the pet's frame, already
    /// zoomed) drawn so that its pixel `anchor` stands on the preview's
    /// ground line, in the middle, clipped to the box.
    pub fn draw(&self, scale: u32, preview: &Rgba, anchor: (i32, i32)) -> Rgba {
        let (art, _) = self.layout();
        let z = scale.max(1) as usize;
        let mut img = Rgba::new(W * z, H * z);
        for y in 0..H * z {
            for x in 0..W * z {
                let c = art[(y / z) * W + x / z];
                img.px[(y * W * z + x) * 4..][..4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        let s = z as i32;
        let (bx, by) = ((PX * 2 + PW) * s / 2 - anchor.0, (GROUND + 1) * s - anchor.1);
        let (x0, y0, x1, y1) = ((PX + 1) * s, (PY + 12) * s, (PX + PW - 1) * s, (PY + PH - 1) * s);
        for py in 0..preview.h {
            for px in 0..preview.w {
                let (sx, sy) = (bx + px as i32, by + py as i32);
                if sx < x0 || sy < y0 || sx >= x1 || sy >= y1 {
                    continue;
                }
                let p = &preview.px[(py * preview.w + px) * 4..][..4];
                if p[3] == 0 {
                    continue;
                }
                let d = &mut img.px[(sy as usize * W * z + sx as usize) * 4..][..4];
                let a = p[3] as u32;
                for k in 0..3 {
                    d[k] = ((p[k] as u32 * a + d[k] as u32 * (255 - a) + 127) / 255) as u8;
                }
            }
        }
        img
    }

    /// One layout pass: the window's pixels at scale 1, and what is where.
    fn layout(&self) -> (Vec<[u8; 3]>, Vec<Region>) {
        let mut c = Canvas { art: vec![FILL; W * H], regions: vec![] };
        c.rect(0, 0, W as i32, H as i32, OUTLINE);
        c.rect(1, 1, W as i32 - 2, H as i32 - 2, FILL);
        c.text("Make a pet", 6, 5, TEXT);
        c.button("x", W as i32 - 12, 3, false, Hit::Close);
        for i in 0..STEPS.len() {
            let x = W as i32 - 16 - (STEPS.len() - i) as i32 * 7;
            if i == self.step {
                c.rect(x, 6, 5, 5, SEL);
            } else if i <= self.reached {
                c.rect(x, 6, 5, 5, TEXT);
            } else {
                c.boxed(x, 6, 5, 5, FILL, DIM);
            }
            if i <= self.reached {
                c.region(format!("dot:{i}"), x - 1, 5, 7, 7, Hit::Dot(i));
            }
        }
        c.rect(1, 15, W as i32 - 2, 1, OUTLINE);
        // the preview box (the pet is drawn into it by `draw`)
        c.boxed(PX, PY, PW, PH, SHADE, OUTLINE);
        c.rect(PX + 4, GROUND, PW - 8, 1, DIM);
        let shown: String = self.name.chars().take(14).collect();
        c.text(&shown, PX + (PW - font::text_width(&shown) as i32) / 2, PY + 4, TEXT);
        // the step
        let count = format!("{}/{}", self.step + 1, STEPS.len());
        c.text(STEPS[self.step], RX, PY + 1, TEXT);
        c.text(&count, W as i32 - 6 - font::text_width(&count) as i32, PY + 1, DIM);
        let pet = self.design.pet;
        match self.step() {
            Step::Animal => {
                c.wide_button("Dog", Y0, pet == Pet::Dog, Hit::Animal(Pet::Dog));
                c.wide_button("Rabbit", Y0 + 12, pet == Pet::Rabbit, Hit::Animal(Pet::Rabbit));
                c.wide_button("Cat", Y0 + 24, pet == Pet::Cat, Hit::Animal(Pet::Cat));
            }
            Step::Breed => {
                for (i, b) in breeds(pet).iter().enumerate() {
                    c.wide_button(b.name, Y0 + 12 * i as i32, i == self.breed, Hit::Breed(i));
                }
            }
            Step::Shape => {
                for (i, s) in sliders(pet).iter().enumerate() {
                    let y = Y0 + 12 * i as i32;
                    c.text(s.label, RX, y + 2, TEXT);
                    let v = ((self.design.shape.get(s.field).unwrap_or(1.0) - s.lo) / (s.hi - s.lo)).clamp(0.0, 1.0);
                    c.rect(BAR_X, y + 5, BAR_W, 1, DIM);
                    c.rect(BAR_X, y + 5, (BAR_W as f64 * v).round() as i32, 1, SEL);
                    c.boxed(BAR_X + ((BAR_W - 3) as f64 * v).round() as i32, y + 2, 3, 7, FILL, OUTLINE);
                    c.region(format!("slider:{}", s.label), BAR_X - 2, y, BAR_W + 4, 11, Hit::Slider(i));
                }
            }
            Step::Coat => {
                let mut y = Y0;
                for &(f, label) in areas(pet) {
                    c.text(label, RX, y + 2, TEXT);
                    let cur = self.coat_of(f);
                    for (i, &col) in SWATCHES.iter().enumerate() {
                        c.swatch(format!("swatch:{label}:{i}"), SW_X + SW_STEP * i as i32, y, Some(col), col == cur, Hit::Swatch(f, col));
                    }
                    y += 14;
                }
                if pet != Pet::Rabbit {
                    c.text("Collar", RX, y + 2, TEXT);
                    c.swatch("swatch:Collar:none".into(), SW_X, y, None, self.design.collar.is_none(), Hit::CollarNone);
                    for (i, &col) in COLLARS.iter().enumerate() {
                        let x = SW_X + SW_STEP * (i as i32 + 1);
                        c.swatch(format!("swatch:Collar:{i}"), x, y, Some(col), self.design.collar == Some(col), Hit::Collar(col));
                    }
                }
            }
            Step::Name => {
                c.text("Type a name", RX, Y0 + 1, DIM);
                c.boxed(RX, Y0 + 12, RW, 13, NAME_BG, OUTLINE);
                let shown: String = {
                    let n = self.name.chars().count();
                    self.name.chars().skip(n.saturating_sub(19)).collect()
                };
                let end = RX + 3 + font::text_width(&shown) as i32 + 1;
                c.text(&shown, RX + 3, Y0 + 15, TEXT);
                c.rect(end, Y0 + 14, 1, 9, TEXT);
                if self.confirm_delete {
                    let q: String = format!("Delete {}?", self.name).chars().take(20).collect();
                    c.text(&q, RX, Y0 + 32, TEXT);
                    let x = c.button("Yes", RX, Y0 + 42, false, Hit::Yes);
                    c.button("No", x + 4, Y0 + 42, false, Hit::No);
                } else {
                    c.text("Then they hop onto", RX, Y0 + 32, DIM);
                    c.text("your taskbar.", RX, Y0 + 41, DIM);
                    if self.deletable {
                        c.button("Delete", RX, Y0 + 54, false, Hit::Delete);
                    }
                }
            }
        }
        // the bottom row
        c.rect(1, H as i32 - 16, W as i32 - 2, 1, OUTLINE);
        let by = H as i32 - 13;
        c.button("Random", PX + 2, by, false, Hit::Random);
        if self.step > 0 {
            c.button("< Back", RX, by, false, Hit::Back);
        }
        let next = if self.step == STEPS.len() - 1 { "Done" } else { "Next >" };
        c.button(next, W as i32 - 6 - (font::text_width(next) as i32 + 6), by, true, Hit::Next);
        (c.art, c.regions)
    }
}

/// Every fixed string the maker shows (for the font check).
pub fn all_copy() -> Vec<&'static str> {
    let mut v = vec!["Make a pet", "x", "Dog", "Rabbit", "Cat", "Collar", "Type a name", "Then they hop onto", "your taskbar."];
    v.extend(["Delete", "Delete ?", "Yes", "No", "Random", "< Back", "Next >", "Done", "0123456789/"]);
    v.extend(STEPS);
    v.extend(NAMES);
    v.extend(areas(Pet::Dog).iter().map(|a| a.1));
    v.extend(areas(Pet::Rabbit).iter().map(|a| a.1));
    v.extend(areas(Pet::Cat).iter().map(|a| a.1));
    v
}

struct Canvas {
    art: Vec<[u8; 3]>,
    regions: Vec<Region>,
}

impl Canvas {
    fn put(&mut self, x: i32, y: i32, c: [u8; 3]) {
        if x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
            self.art[y as usize * W + x as usize] = c;
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, c);
            }
        }
    }

    fn boxed(&mut self, x: i32, y: i32, w: i32, h: i32, fill: [u8; 3], line: [u8; 3]) {
        self.rect(x, y, w, h, line);
        self.rect(x + 1, y + 1, w - 2, h - 2, fill);
    }

    fn text(&mut self, s: &str, x: i32, y: i32, c: [u8; 3]) {
        for (i, ch) in s.chars().enumerate() {
            let g = font::glyph(ch).unwrap_or([0; 7]);
            let gx = x + i as i32 * (font::W as i32 + 1);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..font::W {
                    if bits >> (font::W - 1 - col) & 1 == 1 {
                        self.put(gx + col as i32, y + row as i32, c);
                    }
                }
            }
        }
    }

    fn region(&mut self, name: String, x: i32, y: i32, w: i32, h: i32, hit: Hit) {
        self.regions.push(Region { name, x, y, w, h, hit });
    }

    /// A text button; returns its right edge.
    fn button(&mut self, label: &str, x: i32, y: i32, on: bool, hit: Hit) -> i32 {
        let w = font::text_width(label) as i32 + 6;
        self.button_w(label, x, y, w, on, hit);
        x + w
    }

    fn button_w(&mut self, label: &str, x: i32, y: i32, w: i32, on: bool, hit: Hit) {
        if on {
            self.rect(x, y, w, 11, SEL);
        }
        self.text(label, x + 3, y + 2, if on { SEL_TEXT } else { TEXT });
        self.region(label.to_string(), x, y, w, 11, hit);
    }

    /// A choice as wide as the right-hand column.
    fn wide_button(&mut self, label: &str, y: i32, on: bool, hit: Hit) {
        self.button_w(label, RX, y, RW, on, hit);
    }

    /// A colour swatch (None: the crossed-out "no collar").
    fn swatch(&mut self, name: String, x: i32, y: i32, colour: Option<u32>, on: bool, hit: Hit) {
        if on {
            self.rect(x - 1, y, SW_STEP, 11, SEL);
        }
        let rgb = |c: u32| [(c >> 16) as u8, (c >> 8) as u8, c as u8];
        self.boxed(x, y + 1, 7, 9, colour.map_or(FILL, rgb), OUTLINE);
        if colour.is_none() {
            for k in 0..5 {
                self.put(x + 1 + k, y + 2 + k + k / 2, DIM);
            }
        }
        self.region(name, x - 1, y, SW_STEP, 11, hit);
    }
}
