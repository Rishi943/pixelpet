//! The pet maker: a small window that walks through five steps (animal,
//! breed, shape, coat, name) beside a live preview of the pet, drawn in the
//! menu's pixel style. Pure state and drawing: the app feeds it clicks,
//! drags and keys, and hands it the preview frame. Layout and copy follow
//! the approved mockup in `drafts/maker/index.html`. "From photos..." in the
//! Breed list adds a sixth step, Photos, where the person's photos are shown
//! and clicked on (see `photo::measure` for what the points become).

use super::font;
use crate::pets::{self, clean_name, is_builtin, PetRecord, NAME_MAX};
use crate::photo::measure::{self, Marked, Point, Pose};
use crate::photo::Photo;
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
const RX: i32 = 106;
const RW: i32 = W as i32 - RX - 6;
const Y0: i32 = PY + 14;
// a slider's bar, and a swatch row's first swatch
const BAR_X: i32 = RX + 40;
const BAR_W: i32 = RW - 40;
const SW_X: i32 = RX + 40;
const SW_STEP: i32 = 9;

/// The Photos step's bigger window, so points can be placed with care.
pub const PHOTOS_W: usize = 320;
pub const PHOTOS_H: usize = 200;
// On it the preview box is a reference pet with every point numbered, and
// the photo's box sits under the controls in the right-hand column.
const QW: i32 = 100;
const QH: i32 = PHOTOS_H as i32 - 20 - PY;
const QX: i32 = PX + QW + 8;
const QR: i32 = PHOTOS_W as i32 - QX - 6;
const AY: i32 = 50;
const AH: i32 = PY + QH - AY;
const MARK: [u8; 3] = [0xFF, 0xD0, 0x30];
const AIM: [u8; 3] = [0xE0, 0x30, 0x30];
const ERR: [u8; 3] = [0xB0, 0x20, 0x20];
/// Most photos at once, and how long a message stays up (seconds).
pub const MAX_PHOTOS: usize = 5;
const ERROR_SECS: f64 = 4.0;
/// How close (UI px) a click must be to a marker to grab it.
const GRAB: f32 = 4.0;
/// Letters in the Photos step's one prompt line (it fits the wide column).
const LINE_MAX: usize = 26;

const NAMES: [&str; 8] = ["Biscuit", "Pixel", "Mango", "Pepper", "Comet", "Noodle", "Toffee", "Bean"];
// the Photos step's prompt lines when there is no point to ask for
const HELP_NONE: &str = "Open a side-on photo";
const HELP_POSE: &str = "First, pick the pose";
const HELP_DONE: &str = "All done. Press Next.";

fn step_name(s: Step) -> &'static str {
    match s {
        Step::Animal => "Animal",
        Step::Breed => "Breed",
        Step::Photos => "Photos",
        Step::Shape => "Shape",
        Step::Coat => "Coat",
        Step::Name => "Name",
    }
}

/// Where the preview pet's part is in its stand pose, in units of the look's
/// radius right of and below the spot the pet stands on (read off the
/// rendered frames of Alex and Alex the bun, both facing right).
fn pet_spot(pet: Pet, p: Point) -> (f32, f32) {
    use Point::*;
    match (pet, p) {
        (Pet::Dog, Nose) => (0.95, -1.2),
        (Pet::Dog, Eye) => (0.6, -1.3),
        (Pet::Dog, EarTip) => (0.42, -1.78),
        (Pet::Dog, Shoulders) => (0.1, -1.0),
        (Pet::Dog, TailBase) => (-0.62, -0.86),
        (Pet::Dog, TailTip) => (-0.88, -0.12),
        (Pet::Dog, Chest) => (0.35, -0.65),
        (Pet::Dog, FrontPaw) => (0.45, -0.05),
        (Pet::Dog, BackPaw) => (-0.35, -0.05),
        (Pet::Rabbit, Nose) => (0.6, -0.82),
        (Pet::Rabbit, Eye) => (0.45, -0.9),
        (Pet::Rabbit, EarTip) => (0.2, -1.78),
        (Pet::Rabbit, Shoulders) => (-0.2, -0.72),
        (Pet::Rabbit, TailBase | TailTip) => (-0.55, -0.42),
        (Pet::Rabbit, Chest) => (0.15, -0.4),
        (Pet::Rabbit, FrontPaw) => (0.25, -0.05),
        (Pet::Rabbit, BackPaw) => (-0.3, -0.05),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Animal,
    Breed,
    /// Only on the "From photos..." path, between Breed and Shape.
    Photos,
    Shape,
    Coat,
    Name,
}

const PLAIN_STEPS: [Step; 5] = [Step::Animal, Step::Breed, Step::Shape, Step::Coat, Step::Name];
const PHOTO_STEPS: [Step; 6] = [Step::Animal, Step::Breed, Step::Photos, Step::Shape, Step::Coat, Step::Name];

#[derive(Debug)]
pub enum Outcome {
    Nothing,
    Redraw,
    /// Save this pet. An empty id means a new pet (the app gives it one).
    Done(PetRecord),
    Delete(String),
    Close,
    /// Open the system's photo dialog and hand the photos to `add_photo`.
    PickPhotos,
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
    FromPhotos,
    Open,
    Tab(usize),
    Remove,
    Pose(Pose),
    Skip,
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

/// One photo in the Photos step, with what has been clicked on it.
struct Shot {
    photo: Photo,
    pose: Option<Pose>,
    /// the points answered, in order: where (photo pixels), or None if skipped
    done: Vec<(Point, Option<(f32, f32)>)>,
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
    /// a saved pet is being edited, so no photos (they are long gone)
    editing: bool,
    /// "From photos..." is picked, so Photos is a step
    photos_path: bool,
    shots: Vec<Shot>,
    /// the photo shown
    cur: usize,
    /// the marker (index into the shown photo's `done`) being dragged
    grab: Option<usize>,
    /// the photos changed since they last set the design
    photos_dirty: bool,
    /// the coat the photos gave, offered as a first swatch in each Coat row
    photo_coat: Vec<(i8, u32)>,
    /// a message for the Photos step, and the `tick` time it first showed
    error: Option<(String, Option<f64>)>,
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
            editing: false,
            photos_path: false,
            shots: vec![],
            cur: 0,
            grab: None,
            photos_dirty: false,
            photo_coat: vec![],
            error: None,
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
            reached: PLAIN_STEPS.len() - 1,
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
            editing: true,
            photos_path: false,
            shots: vec![],
            cur: 0,
            grab: None,
            photos_dirty: false,
            photo_coat: vec![],
            error: None,
        }
    }

    fn steps(&self) -> &'static [Step] {
        if self.photos_path {
            &PHOTO_STEPS
        } else {
            &PLAIN_STEPS
        }
    }

    pub fn step(&self) -> Step {
        self.steps()[self.step]
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
            Step::Animal | Step::Breed | Step::Photos => Anim::Stand,
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
        self.photos_dirty = true;
        if self.auto_name {
            self.name = if i == 0 {
                pets::builtins().into_iter().find(|b| b.design.pet == pet).map(|b| b.name).unwrap_or_default()
            } else {
                self.pick(&NAMES).to_string()
            };
        }
    }

    fn go(&mut self, i: usize) {
        let from = (self.step(), self.step);
        self.step = i.min(self.steps().len() - 1);
        self.reached = self.reached.max(self.step);
        self.confirm_delete = false;
        self.grab = None;
        if from.0 == Step::Photos && self.step > from.1 && self.photos_dirty {
            self.apply_photos();
        }
    }

    /// Picking a breed (or "From photos...", which is the first breed with
    /// the Photos step added). Dots past this step no longer mean anything.
    fn set_path(&mut self, photos: bool) {
        if photos != self.photos_path {
            self.photos_path = photos;
            self.reached = self.step;
        }
    }

    /// The marked photos set the shape and coat: Alex's (or the bun's) own
    /// design is the base, and photos with no points count for nothing.
    fn apply_photos(&mut self) {
        self.photos_dirty = false;
        let pet = self.design.pet;
        let known = measure::points(pet);
        let marked: Vec<Marked> = self
            .shots
            .iter()
            .filter_map(|s| {
                let at: Vec<_> = s.done.iter().filter_map(|&(p, at)| at.filter(|_| known.contains(&p)).map(|a| (p, a))).collect();
                (!at.is_empty()).then_some(Marked { photo: &s.photo, pose: s.pose.unwrap_or(Pose::Other), at })
            })
            .collect();
        if marked.is_empty() {
            self.photo_coat.clear();
            return;
        }
        let fit = measure::fit(&(breeds(pet)[0].design)(), &marked);
        self.design.shape = fit.shape;
        self.design.coat = fit.coat.clone();
        self.photo_coat = fit.coat;
    }

    fn next(&mut self) -> Outcome {
        if self.step == self.steps().len() - 1 {
            return self.done();
        }
        self.go(self.step + 1);
        Outcome::Redraw
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
        if pet == Pet::Dog {
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
        let coat = match self.design.pet {
            Pet::Dog => &crate::rig::dog::ALEX,
            Pet::Rabbit => &crate::rig::rabbit::BUN,
        };
        coat.fams.iter().find(|(cf, _)| *cf == f).map_or(0, |(_, v)| v[2])
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
        self.click_at(x as f32, y as f32)
    }

    /// A click at `(x, y)` in UI pixels as floats (the cursor's position in
    /// window pixels over the zoom), so a point on a photo lands on the
    /// photo pixel under the cursor at any zoom.
    pub fn click_at(&mut self, x: f32, y: f32) -> Outcome {
        self.drag = None;
        self.grab = None;
        let (ix, iy) = (x.floor() as i32, y.floor() as i32);
        let (_, regions) = self.layout();
        let Some(hit) = regions.iter().find(|r| ix >= r.x && ix < r.x + r.w && iy >= r.y && iy < r.y + r.h).map(|r| r.hit) else { return if self.step() == Step::Photos { self.click_photo(x, y) } else { Outcome::Nothing } };
        let pet = self.design.pet;
        match hit {
            Hit::Dot(i) => self.go(i),
            Hit::Animal(p) => {
                self.set_path(false);
                self.apply_breed(p, 0);
                self.go(1);
            }
            Hit::Breed(i) => {
                self.set_path(false);
                self.apply_breed(pet, i);
            }
            Hit::FromPhotos => {
                self.set_path(true);
                self.apply_breed(pet, 0);
            }
            Hit::Open => return Outcome::PickPhotos,
            Hit::Tab(i) => self.cur = i.min(self.shots.len().saturating_sub(1)),
            Hit::Remove => {
                if self.cur < self.shots.len() {
                    self.shots.remove(self.cur);
                    self.cur = self.cur.min(self.shots.len().saturating_sub(1));
                    self.photos_dirty = true;
                }
            }
            Hit::Pose(p) => {
                // the chosen pose's button takes it back, to choose again
                if let Some(s) = self.shots.get_mut(self.cur) {
                    s.pose = if s.pose == Some(p) { None } else { Some(p) };
                    self.photos_dirty = true;
                }
            }
            Hit::Skip => {
                if let Some(p) = self.next_point() {
                    self.shots[self.cur].done.push((p, None));
                    self.photos_dirty = true;
                }
            }
            Hit::Slider(i) => {
                self.drag = Some(i);
                self.set_slider(i, ix);
            }
            Hit::Swatch(f, c) => self.set_coat(f, c),
            Hit::CollarNone => self.design.collar = None,
            Hit::Collar(c) => self.design.collar = Some(c),
            Hit::Random => self.randomise(),
            Hit::Back => self.go(self.step.saturating_sub(1)),
            Hit::Next => return self.next(),
            Hit::Close => return Outcome::Close,
            Hit::Delete => self.confirm_delete = true,
            Hit::No => self.confirm_delete = false,
            Hit::Yes => return Outcome::Delete(self.id.clone()),
        }
        Outcome::Redraw
    }

    /// The cursor moved to `(x, y)` with the button held: drags a slider
    /// grabbed by the last click.
    pub fn drag(&mut self, x: i32, y: i32) -> Outcome {
        self.drag_at(x as f32, y as f32)
    }

    /// `drag` with the cursor in float UI pixels: also drags a marker
    /// grabbed by the last click.
    pub fn drag_at(&mut self, x: f32, y: f32) -> Outcome {
        if let (Some(i), Some((rx, ry, rw, _))) = (self.grab, self.photo_rect()) {
            let p = &self.shots[self.cur].photo;
            let k = rw / p.w as f32;
            let at = (((x - rx) / k).clamp(0.0, p.w as f32), ((y - ry) / k).clamp(0.0, p.h as f32));
            if let Some(d) = self.shots[self.cur].done.get_mut(i) {
                d.1 = Some(at);
                self.photos_dirty = true;
            }
            return Outcome::Redraw;
        }
        match self.drag {
            Some(i) => {
                self.set_slider(i, x.floor() as i32);
                Outcome::Redraw
            }
            None => Outcome::Nothing,
        }
    }

    /// The button came up: lets go of a slider or marker.
    pub fn release(&mut self) {
        self.drag = None;
        self.grab = None;
    }

    /// A right click at `(x, y)`: on the Photos step, takes back the last
    /// point (marker or skip) of the shown photo.
    pub fn right_click(&mut self, _x: f32, _y: f32) -> Outcome {
        self.grab = None;
        let on_photos = self.step() == Step::Photos;
        let popped = on_photos && self.shots.get_mut(self.cur).is_some_and(|s| s.done.pop().is_some());
        if popped {
            self.photos_dirty = true;
            return Outcome::Redraw;
        }
        Outcome::Nothing
    }

    /// A photo was opened or dropped: add it as the shown photo. Ignored
    /// unless "From photos..." is picked; a sixth is refused with a message.
    pub fn add_photo(&mut self, photo: Photo) {
        if !self.photos_path {
            return;
        }
        if self.shots.len() >= MAX_PHOTOS {
            self.photo_error("Up to 5 photos");
            return;
        }
        self.shots.push(Shot { photo, pose: None, done: vec![] });
        self.cur = self.shots.len() - 1;
        self.photos_dirty = true;
    }

    /// Whether photos are wanted now (so the app can skip decoding them).
    pub fn wants_photos(&self) -> bool {
        self.photos_path
    }

    /// Show one line on the Photos step until `tick` has seen it for a few
    /// seconds.
    pub fn photo_error(&mut self, msg: &str) {
        self.error = Some((msg.chars().map(|c| if (' '..='~').contains(&c) { c } else { '?' }).take(40).collect(), None));
    }

    /// The message showing, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_ref().map(|e| e.0.as_str())
    }

    /// The app's clock, in seconds, now and then (the maker has none): takes
    /// a message down once it has been up long enough. Redraw if it did.
    pub fn tick(&mut self, now: f64) -> Outcome {
        match &mut self.error {
            Some((_, at @ None)) => *at = Some(now),
            Some((_, Some(t))) if now - *t >= ERROR_SECS => {
                self.error = None;
                return Outcome::Redraw;
            }
            _ => {}
        }
        Outcome::Nothing
    }

    /// How many photos there are, and which one is shown.
    pub fn photo_count(&self) -> usize {
        self.shots.len()
    }

    pub fn photo_shown(&self) -> usize {
        self.cur
    }

    /// The shown photo's points so far, in photo pixels (skips left out).
    pub fn marks(&self) -> Vec<(Point, (f32, f32))> {
        self.shots.get(self.cur).map_or(vec![], |s| s.done.iter().filter_map(|&(p, at)| at.map(|a| (p, a))).collect())
    }

    /// The point the shown photo is asking for next.
    pub fn next_point(&self) -> Option<Point> {
        let s = self.shots.get(self.cur)?;
        s.pose?;
        measure::points(self.design.pet).iter().copied().find(|p| !s.done.iter().any(|d| d.0 == *p))
    }

    /// Where the shown photo is drawn, `(x, y, w, h)` in UI pixels: as big
    /// as fits its box, whatever its shape, in the middle.
    pub fn photo_rect(&self) -> Option<(f32, f32, f32, f32)> {
        if self.step() != Step::Photos {
            return None;
        }
        let p = &self.shots.get(self.cur)?.photo;
        let (bx, by, bw, bh) = ((QX + 1) as f32, (AY + 1) as f32, (QR - 2) as f32, (AH - 2) as f32);
        let k = (bw / p.w as f32).min(bh / p.h as f32);
        let (w, h) = (p.w as f32 * k, p.h as f32 * k);
        Some((bx + (bw - w) / 2.0, by + (bh - h) / 2.0, w, h))
    }

    /// A click that no button took, on the Photos step: grabs a marker, or
    /// drops the next one on the photo.
    fn click_photo(&mut self, x: f32, y: f32) -> Outcome {
        let Some((rx, ry, rw, rh)) = self.photo_rect() else { return Outcome::Nothing };
        let k = rw / self.shots[self.cur].photo.w as f32;
        let near = |d: &(Point, Option<(f32, f32)>)| {
            d.1.map(|(px, py)| ((rx + px * k - x).powi(2) + (ry + py * k - y).powi(2)).sqrt()).filter(|&dist| dist <= GRAB)
        };
        let nearest = self.shots[self.cur].done.iter().enumerate().filter_map(|(i, d)| near(d).map(|dist| (i, dist)));
        if let Some((i, _)) = nearest.min_by(|a, b| a.1.total_cmp(&b.1)) {
            self.grab = Some(i);
            return Outcome::Redraw;
        }
        let Some(p) = self.next_point() else { return Outcome::Nothing };
        if x < rx || y < ry || x >= rx + rw || y >= ry + rh {
            return Outcome::Nothing;
        }
        self.shots[self.cur].done.push((p, Some(((x - rx) / k, (y - ry) / k))));
        self.photos_dirty = true;
        Outcome::Redraw
    }

    pub fn key(&mut self, k: MakerKey) -> Outcome {
        match k {
            MakerKey::Esc if self.confirm_delete => self.confirm_delete = false,
            MakerKey::Esc => return Outcome::Close,
            MakerKey::Enter if self.step() == Step::Name => return self.done(),
            MakerKey::Enter => return self.next(),
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
        let (w, h) = self.size();
        let z = scale.max(1) as usize;
        let mut img = Rgba::new(w * z, h * z);
        for y in 0..h * z {
            for x in 0..w * z {
                let c = art[(y / z) * w + x / z];
                img.px[(y * w * z + x) * 4..][..4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        self.draw_photo(&mut img, z);
        let s = z as i32;
        let (pw, ph) = self.preview_box();
        let ground = PY + ph - 10;
        let (bx, by) = ((PX * 2 + pw) * s / 2 - anchor.0, (ground + 1) * s - anchor.1);
        let (x0, y0, x1, y1) = ((PX + 1) * s, (PY + 12) * s, (PX + pw - 1) * s, (PY + ph - 1) * s);
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
                let d = &mut img.px[(sy as usize * img.w + sx as usize) * 4..][..4];
                let a = p[3] as u32;
                for k in 0..3 {
                    d[k] = ((p[k] as u32 * a + d[k] as u32 * (255 - a) + 127) / 255) as u8;
                }
            }
        }
        // on the Photos step the pet is a reference: every point numbered as
        // on the photo, the one being asked for in red
        if self.step() == Step::Photos {
            let pet = self.design.pet;
            let next = self.next_point();
            let unit = preview.w as f32 / 3.1;
            let clip = (x0, y0, x1, y1);
            // the one being asked for last, so it is on top
            let mut order: Vec<(usize, Point)> = measure::points(pet).iter().copied().enumerate().collect();
            order.sort_by_key(|&(_, pt)| Some(pt) == next);
            for (i, pt) in order {
                let (fx, fy) = pet_spot(pet, pt);
                let (cx, cy) = ((PX * 2 + pw) as f32 * s as f32 / 2.0 + fx * unit, (ground + 1) as f32 * s as f32 + fy * unit);
                let (r, col) = if Some(pt) == next { (2.0, AIM) } else { (1.3, MARK) };
                disc(&mut img, cx, cy, (r + 0.8) * s as f32, OUTLINE, clip);
                disc(&mut img, cx, cy, r * s as f32, col, clip);
                if Some(pt) == next {
                    let digit = char::from_digit(i as u32 + 1, 10).unwrap_or('?');
                    let (lx, ly) = ((cx + 3.0 * s as f32).round() as i32, (cy - 9.0 * s as f32).round() as i32);
                    for (dx, dy, c) in [(-1, 0, OUTLINE), (1, 0, OUTLINE), (0, -1, OUTLINE), (0, 1, OUTLINE), (0, 0, SEL_TEXT)] {
                        glyph_px(&mut img, digit, lx + dx * s, ly + dy * s, s, c, clip);
                    }
                }
            }
        }
        img
    }

    /// The window's size in UI pixels: bigger on the Photos step.
    pub fn size(&self) -> (usize, usize) {
        if self.step() == Step::Photos {
            (PHOTOS_W, PHOTOS_H)
        } else {
            (W, H)
        }
    }

    /// The preview box's width and height for this step.
    fn preview_box(&self) -> (i32, i32) {
        if self.step() == Step::Photos {
            (QW, QH)
        } else {
            (PW, PH)
        }
    }

    /// One layout pass: the window's pixels at scale 1, and what is where.
    fn layout(&self) -> (Vec<[u8; 3]>, Vec<Region>) {
        let (cw, ch) = self.size();
        let mut c = Canvas { art: vec![FILL; cw * ch], w: cw, h: ch, regions: vec![] };
        let (w, h) = (cw as i32, ch as i32);
        c.rect(0, 0, w, h, OUTLINE);
        c.rect(1, 1, w - 2, h - 2, FILL);
        c.text("Make a pet", 6, 5, TEXT);
        c.button("x", w - 12, 3, false, Hit::Close);
        let n = self.steps().len();
        for i in 0..n {
            let x = w - 16 - (n - i) as i32 * 7;
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
        c.rect(1, 15, w - 2, 1, OUTLINE);
        // the preview box (the pet is drawn into it by `draw`)
        let (pw, ph) = self.preview_box();
        let rx = if self.step() == Step::Photos { QX } else { RX };
        c.boxed(PX, PY, pw, ph, SHADE, OUTLINE);
        c.rect(PX + 4, PY + ph - 10, pw - 8, 1, DIM);
        let shown: String = self.name.chars().take(14).collect();
        c.text(&shown, PX + (pw - font::text_width(&shown) as i32) / 2, PY + 4, TEXT);
        // the step
        let count = format!("{}/{}", self.step + 1, n);
        c.text(step_name(self.step()), rx, PY + 1, TEXT);
        c.text(&count, w - 6 - font::text_width(&count) as i32, PY + 1, DIM);
        let pet = self.design.pet;
        match self.step() {
            Step::Animal => {
                c.wide_button("Dog", Y0, pet == Pet::Dog, Hit::Animal(Pet::Dog));
                c.wide_button("Rabbit", Y0 + 12, pet == Pet::Rabbit, Hit::Animal(Pet::Rabbit));
                c.text("Cat (later)", RX + 3, Y0 + 26, DIM);
            }
            Step::Photos => self.photos_layout(&mut c),
            Step::Breed => {
                let list = breeds(pet);
                for (i, b) in list.iter().enumerate() {
                    c.wide_button(b.name, Y0 + 12 * i as i32, i == self.breed && !self.photos_path, Hit::Breed(i));
                }
                if !self.editing {
                    c.wide_button("From photos...", Y0 + 12 * list.len() as i32, self.photos_path, Hit::FromPhotos);
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
                    // the photo's colour, first, so it can be gone back to
                    let from_photo = self.photo_coat.iter().find(|(pf, _)| *pf == f).map(|c| c.1);
                    if let Some(col) = from_photo {
                        c.swatch(format!("swatch:{label}:photo"), SW_X, y, Some(col), col == cur, Hit::Swatch(f, col));
                    }
                    let off = from_photo.is_some() as i32;
                    for (i, &col) in SWATCHES.iter().enumerate() {
                        c.swatch(format!("swatch:{label}:{i}"), SW_X + SW_STEP * (i as i32 + off), y, Some(col), col == cur, Hit::Swatch(f, col));
                    }
                    y += 14;
                }
                if pet == Pet::Dog {
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
        c.rect(1, h - 16, w - 2, 1, OUTLINE);
        let by = h - 13;
        c.button("Random", PX + 2, by, false, Hit::Random);
        if self.step > 0 {
            c.button("< Back", rx, by, false, Hit::Back);
        }
        let next = if self.step == n - 1 { "Done" } else { "Next >" };
        c.button(next, w - 6 - (font::text_width(next) as i32 + 6), by, true, Hit::Next);
        (c.art, c.regions)
    }

    /// The Photos step's controls, in the wide column: tabs in the title row;
    /// then the three poses until one is chosen, after that Open, the chosen
    /// pose (click to choose again) and Skip; one line of prompt; the photo's
    /// box (drawn into by `draw`, which has the zoom).
    fn photos_layout(&self, c: &mut Canvas) {
        let tab_x = |i: usize| QX + 40 + 9 * i as i32;
        for i in 0..self.shots.len() {
            c.tab((i + 1).to_string(), &(i + 1).to_string(), tab_x(i), i == self.cur, Hit::Tab(i));
        }
        if !self.shots.is_empty() {
            c.tab("remove".into(), "x", tab_x(MAX_PHOTOS), false, Hit::Remove);
        }
        const POSES: [(Pose, &str); 3] = [(Pose::Standing, "Standing"), (Pose::Sitting, "Sitting"), (Pose::Other, "Other")];
        let shot = self.shots.get(self.cur);
        match shot.map(|s| s.pose) {
            Some(None) => {
                let mut x = QX;
                for (p, label) in POSES {
                    x = c.pose_button(label, x, 29, false, Hit::Pose(p)) + 4;
                }
            }
            chosen => {
                c.button("Open photo...", QX, 29, false, Hit::Open);
                if let Some(Some(p)) = chosen {
                    let label = POSES.iter().find(|q| q.0 == p).map_or("", |q| q.1);
                    c.pose_button(label, QX + font::text_width("Open photo...") as i32 + 7, 29, true, Hit::Pose(p));
                }
                if self.next_point().is_some() && shot.is_some() {
                    let w = font::text_width("Skip") as i32 + 6;
                    c.button("Skip", QX + QR - w, 29, false, Hit::Skip);
                }
            }
        }
        let (text, col) = match (&self.error, shot) {
            (Some((e, _)), _) => (e.as_str(), ERR),
            (None, None) => (HELP_NONE, DIM),
            (None, Some(s)) if s.pose.is_none() => (HELP_POSE, TEXT),
            _ => match self.next_point() {
                Some(p) => (measure::prompt(self.design.pet, p), TEXT),
                None => (HELP_DONE, DIM),
            },
        };
        let line: String = text.chars().take(LINE_MAX).collect();
        c.text(&line, QX, 40, col);
        c.boxed(QX, AY, QR, AH, SHADE, OUTLINE);
    }

    /// The shown photo at screen resolution in its box, with its markers.
    fn draw_photo(&self, img: &mut Rgba, z: usize) {
        let Some((rx, ry, rw, rh)) = self.photo_rect() else { return };
        let shot = &self.shots[self.cur];
        let p = &shot.photo;
        let zf = z as f32;
        let k = rw / p.w as f32;
        // a few samples a screen pixel when the photo is shrunk
        let n = (1.0 / (k * zf)).ceil().clamp(1.0, 4.0) as usize;
        let (x0, y0, x1, y1) = ((rx * zf).round() as i32, (ry * zf).round() as i32, ((rx + rw) * zf).round() as i32, ((ry + rh) * zf).round() as i32);
        for sy in y0.max(0)..y1.min(img.h as i32) {
            for sx in x0.max(0)..x1.min(img.w as i32) {
                let mut sum = [0u32; 3];
                for j in 0..n {
                    for i in 0..n {
                        let u = ((sx as f32 + (i as f32 + 0.5) / n as f32) / zf - rx) / k;
                        let v = ((sy as f32 + (j as f32 + 0.5) / n as f32) / zf - ry) / k;
                        let (px, py) = ((u as usize).min(p.w as usize - 1), (v as usize).min(p.h as usize - 1));
                        let o = (py * p.w as usize + px) * 3;
                        for (s, &b) in sum.iter_mut().zip(&p.rgb[o..o + 3]) {
                            *s += b as u32;
                        }
                    }
                }
                let d = &mut img.px[(sy as usize * img.w + sx as usize) * 4..][..4];
                for (dk, s) in d.iter_mut().zip(sum) {
                    *dk = ((s + (n * n) as u32 / 2) / (n * n) as u32) as u8;
                }
                d[3] = 255;
            }
        }
        let clip = (QX * z as i32, AY * z as i32, (QX + QR) * z as i32, (AY + AH) * z as i32);
        let order = measure::points(self.design.pet);
        for (pt, (px, py)) in shot.done.iter().filter_map(|&(pt, at)| at.map(|a| (pt, a))) {
            let (cx, cy) = ((rx + px * k) * zf, (ry + py * k) * zf);
            disc(img, cx, cy, 2.5 * zf, OUTLINE, clip);
            disc(img, cx, cy, 1.5 * zf, MARK, clip);
            let digit = char::from_digit(order.iter().position(|&o| o == pt).map_or(0, |i| i as u32 + 1), 10).unwrap_or('?');
            // beside the marker, on whichever side has room
            let lx = if cx + 3.0 * zf + 5.0 * zf > clip.2 as f32 { cx - 8.0 * zf } else { cx + 3.0 * zf };
            let ly = if cy - 9.0 * zf < clip.1 as f32 { cy + 3.0 * zf } else { cy - 9.0 * zf };
            for (dx, dy, col) in [(-1, 0, OUTLINE), (1, 0, OUTLINE), (0, -1, OUTLINE), (0, 1, OUTLINE), (0, 0, SEL_TEXT)] {
                glyph_px(img, digit, lx.round() as i32 + dx * z as i32, ly.round() as i32 + dy * z as i32, z as i32, col, clip);
            }
        }
    }
}


/// A filled round dot at `(cx, cy)`, radius `r` (screen pixels), kept inside
/// `clip` (x0, y0, x1, y1).
fn disc(img: &mut Rgba, cx: f32, cy: f32, r: f32, col: [u8; 3], clip: (i32, i32, i32, i32)) {
    for y in ((cy - r).floor() as i32).max(clip.1)..((cy + r).ceil() as i32 + 1).min(clip.3).min(img.h as i32) {
        for x in ((cx - r).floor() as i32).max(clip.0)..((cx + r).ceil() as i32 + 1).min(clip.2).min(img.w as i32) {
            if (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) <= r * r {
                img.px[(y as usize * img.w + x as usize) * 4..][..3].copy_from_slice(&col);
            }
        }
    }
}

/// One font glyph with its top left at screen pixel `(x, y)`, each of its
/// pixels `z` screen pixels square, kept inside `clip`.
fn glyph_px(img: &mut Rgba, ch: char, x: i32, y: i32, z: i32, col: [u8; 3], clip: (i32, i32, i32, i32)) {
    let g = font::glyph(ch).unwrap_or([0; 7]);
    for (row, bits) in g.iter().enumerate() {
        for c in 0..font::W {
            if bits >> (font::W - 1 - c) & 1 == 1 {
                let (gx, gy) = (x + c as i32 * z, y + row as i32 * z);
                for yy in gy.max(clip.1)..(gy + z).min(clip.3).min(img.h as i32) {
                    for xx in gx.max(clip.0)..(gx + z).min(clip.2).min(img.w as i32) {
                        img.px[(yy as usize * img.w + xx as usize) * 4..][..3].copy_from_slice(&col);
                    }
                }
            }
        }
    }
}

/// Every fixed string the maker shows (for the font check).
pub fn all_copy() -> Vec<&'static str> {
    let mut v = vec!["Make a pet", "x", "Dog", "Rabbit", "Cat (later)", "Collar", "Type a name", "Then they hop onto", "your taskbar."];
    v.extend(["Delete", "Delete ?", "Yes", "No", "Random", "< Back", "Next >", "Done", "0123456789/"]);
    v.extend(["From photos...", "Open photo...", "Skip", "Standing", "Sitting", "Other", "Up to 5 photos", "Drop photos here", "That photo is too big", "Not a JPEG, PNG or WebP"]);
    v.extend([HELP_NONE, HELP_POSE, HELP_DONE]);
    for pet in [Pet::Dog, Pet::Rabbit] {
        v.extend(measure::points(pet).iter().map(|&p| measure::prompt(pet, p)));
    }
    v.extend(PLAIN_STEPS.iter().chain(&PHOTO_STEPS).map(|&s| step_name(s)));
    v.extend(NAMES);
    v.extend(areas(Pet::Dog).iter().map(|a| a.1));
    v.extend(areas(Pet::Rabbit).iter().map(|a| a.1));
    v
}

struct Canvas {
    art: Vec<[u8; 3]>,
    w: usize,
    h: usize,
    regions: Vec<Region>,
}

impl Canvas {
    fn put(&mut self, x: i32, y: i32, c: [u8; 3]) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.art[y as usize * self.w + x as usize] = c;
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

    /// A small tab in the title row (the photo's number, or the "x" that
    /// removes the shown one); `name` is what `target` finds it by.
    fn tab(&mut self, name: String, label: &str, x: i32, on: bool, hit: Hit) {
        if on {
            self.rect(x, 17, 8, 10, SEL);
        } else {
            self.boxed(x, 17, 8, 10, FILL, DIM);
        }
        self.text(label, x + 2, 19, if on { SEL_TEXT } else { TEXT });
        self.region(name, x, 17, 8, 10, hit);
    }

    /// A button with one pixel of room round its label, to fit three in a
    /// row; returns its right edge.
    fn pose_button(&mut self, label: &str, x: i32, y: i32, on: bool, hit: Hit) -> i32 {
        let w = font::text_width(label) as i32 + 2;
        if on {
            self.rect(x, y, w, 11, SEL);
        }
        self.text(label, x + 1, y + 2, if on { SEL_TEXT } else { TEXT });
        self.region(label.to_string(), x, y, w, 11, hit);
        x + w
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
