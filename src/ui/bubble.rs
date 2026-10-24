//! The hover bubble: Pet / Play / Feed, drawn like the menu (same font,
//! colours and cells), with a tail pointing down at the pet. Play and Feed
//! open a second page of choices with a Back cell. Layout is in art px.

use super::font;
use super::menu::{FILL, OUTLINE, TEXT};
use crate::brain::Choice;
use crate::rig::{Food, Pet, Toy};
use crate::window::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Top,
    Play,
    Feed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Go(Page),
    Pick(Choice),
}

const INSET: i32 = 4;
const CELL_H: i32 = font::H as i32 + 4;
const CELL_PAD_X: i32 = 2;
const SEP: i32 = font::W as i32 + 2;
const TAIL: i32 = 4;

pub struct Bubble {
    pet: Pet,
    page: Page,
    /// label, what it does, x, w
    cells: Vec<(String, Cell, i32, i32)>,
    w: usize,
}

impl Bubble {
    pub fn new(pet: Pet) -> Bubble {
        let mut b = Bubble { pet, page: Page::Top, cells: vec![], w: 0 };
        b.layout();
        b
    }

    pub fn page(&self) -> Page {
        self.page
    }

    /// Back to the first page (after a choice, or when it closes).
    pub fn reset(&mut self) {
        self.page = Page::Top;
        self.layout();
    }

    fn layout(&mut self) {
        let opts: Vec<(&str, Cell)> = match (self.page, self.pet) {
            (Page::Top, _) => vec![("Pet", Cell::Pick(Choice::Pet)), ("Play", Cell::Go(Page::Play)), ("Feed", Cell::Go(Page::Feed))],
            (Page::Play, Pet::Dog) => vec![
                ("Ball", Cell::Pick(Choice::Fetch(Toy::Ball))),
                ("Coconut", Cell::Pick(Choice::Fetch(Toy::Coconut))),
                ("Back", Cell::Go(Page::Top)),
            ],
            (Page::Play, Pet::Cat) => vec![("Ball", Cell::Pick(Choice::Fetch(Toy::Ball))), ("Back", Cell::Go(Page::Top))],
            (Page::Play, Pet::Rabbit) => vec![
                ("Football", Cell::Pick(Choice::Fetch(Toy::Football))),
                ("Back", Cell::Go(Page::Top)),
            ],
            (Page::Feed, pet) => {
                let mut v: Vec<(&str, Cell)> = Food::for_pet(pet).iter().map(|&f| (f.label(), Cell::Pick(Choice::Feed(f)))).collect();
                v.push(("Back", Cell::Go(Page::Top)));
                v
            }
        };
        self.cells.clear();
        let mut x = INSET - CELL_PAD_X;
        for (k, (label, cell)) in opts.into_iter().enumerate() {
            if k > 0 {
                x += SEP;
            }
            let w = font::text_width(label) as i32 + 2 * CELL_PAD_X;
            self.cells.push((label.to_string(), cell, x, w));
            x += w;
        }
        self.w = (x + INSET) as usize;
    }

    /// The current page as text: `opt | opt | ...`.
    pub fn lines(&self) -> String {
        self.cells.iter().map(|c| c.0.as_str()).collect::<Vec<_>>().join(" | ")
    }

    /// Size in art px, tail included.
    pub fn size(&self) -> (usize, usize) {
        (self.w, (2 * INSET + CELL_H + TAIL) as usize)
    }

    pub fn cell_centre(&self, label: &str) -> Option<(i32, i32)> {
        self.cells.iter().find(|c| c.0 == label).map(|c| (c.2 + c.3 / 2, INSET + CELL_H / 2))
    }

    /// A click at image px `(x, y)` of `draw(scale)`: changes page, or
    /// returns the choice made.
    pub fn click(&mut self, x: i32, y: i32, scale: u32) -> Option<Choice> {
        let s = scale.max(1) as i32;
        let (ax, ay) = (x.div_euclid(s), y.div_euclid(s));
        if !(INSET..INSET + CELL_H).contains(&ay) {
            return None;
        }
        let cell = self.cells.iter().find(|c| ax >= c.2 && ax < c.2 + c.3)?.1;
        match cell {
            Cell::Go(p) => {
                self.page = p;
                self.layout();
                None
            }
            Cell::Pick(c) => Some(c),
        }
    }

    pub fn draw(&self, scale: u32) -> Rgba {
        let (w, h) = self.size();
        let panel_h = h as i32 - TAIL;
        let mut art = vec![None::<[u8; 3]>; w * h];
        let mut put = |x: i32, y: i32, c: [u8; 3]| {
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                art[y as usize * w + x as usize] = Some(c);
            }
        };
        for y in 0..panel_h {
            for x in 0..w as i32 {
                let edge = x == 0 || y == 0 || x == w as i32 - 1 || y == panel_h - 1;
                put(x, y, if edge { OUTLINE } else { FILL });
            }
        }
        // the tail, pointing down at the pet, open into the panel
        let c = w as i32 / 2;
        for k in 0..TAIL {
            let half = TAIL - 1 - k;
            for x in c - half..=c + half {
                put(x, panel_h + k, if (x - c).abs() == half { OUTLINE } else { FILL });
            }
        }
        for x in c - (TAIL - 2)..=c + (TAIL - 2) {
            put(x, panel_h - 1, FILL);
        }
        let mut text = |x: i32, y: i32, s: &str| {
            for (i, ch) in s.chars().enumerate() {
                let g = font::glyph(ch).unwrap_or([0; 7]);
                let gx = x + i as i32 * (font::W as i32 + 1);
                for (row, bits) in g.iter().enumerate() {
                    for col in 0..font::W {
                        if bits >> (font::W - 1 - col) & 1 == 1 {
                            put(gx + col as i32, y + row as i32, TEXT);
                        }
                    }
                }
            }
        };
        for (k, (label, _, x, _)) in self.cells.iter().enumerate() {
            text(x + CELL_PAD_X, INSET + 2, label);
            if k > 0 {
                text(x - SEP + 1, INSET + 2, "|");
            }
        }
        let z = scale.max(1) as usize;
        let mut img = Rgba::new(w * z, h * z);
        for y in 0..h * z {
            for x in 0..w * z {
                if let Some(c) = art[(y / z) * w + x / z] {
                    let i = (y * w * z + x) * 4;
                    img.px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
                }
            }
        }
        img
    }
}
