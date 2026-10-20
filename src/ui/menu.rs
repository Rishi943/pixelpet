//! The right-click settings menu, drawn as pixel art in the pet's style.
//! Layout is in art pixels; `draw` and `hit` take the pet's zoom as `scale`.

use super::font;
use crate::settings::{FullscreenMode, PetChoice, Settings, Size};
use crate::window::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    SetPet(PetChoice),
    SetSize(Size),
    SetOpacity(u8),
    SetFullscreen(FullscreenMode),
    SetFullscreenOpacity(u8),
    ToggleAutostart,
    /// The Quit row: asks "Quit PixelPet? Yes | No" instead of quitting.
    AskQuit,
    CancelQuit,
    Quit,
}

/// One cell of the menu: a heading (no action) or a clickable option.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub label: String,
    pub action: Option<Action>,
    pub checked: bool,
}

/// Apply a choice to the settings. Quit and the autostart side effect are
/// the app's job; this only changes fields.
pub fn apply(a: &Action, s: &mut Settings) {
    match *a {
        Action::SetPet(p) => s.pet = p,
        Action::SetSize(z) => s.size = z,
        Action::SetOpacity(o) => s.opacity = o,
        Action::SetFullscreen(f) => s.fullscreen = f,
        Action::SetFullscreenOpacity(o) => s.fullscreen_opacity = o,
        Action::ToggleAutostart => s.autostart = !s.autostart,
        Action::AskQuit | Action::CancelQuit | Action::Quit => {}
    }
}

const OUTLINE: [u8; 3] = [0x24, 0x12, 0x0A];
const FILL: [u8; 3] = [0xF3, 0xD2, 0x9C];
const TEXT: [u8; 3] = [0x24, 0x12, 0x0A];
const SELECTED: [u8; 3] = [0x2F, 0x64, 0xC8];
const SELECTED_TEXT: [u8; 3] = [0xFF, 0xFF, 0xFF];

/// Panel edge to content (1 px outline + 3 px padding).
const INSET: i32 = 4;
const CELL_H: i32 = font::H as i32 + 4;
const CELL_PAD_X: i32 = 2;
const LINE_GAP: i32 = 1;
const HEADING_GAP: i32 = 4;
/// Gap between option cells; a `|` is drawn in it.
const SEP: i32 = font::W as i32 + 2;

struct Line {
    heading: Option<usize>,
    cells: Vec<usize>,
    /// Options start right after this line's own heading, not in the
    /// shared column (so a long heading doesn't shift every line).
    own_column: bool,
}

/// A laid-out cell: row index and rect `(x, y, w, h)` in art px.
#[derive(Clone, Copy)]
struct Cell {
    row: usize,
    x: i32,
    y: i32,
    w: i32,
}

pub struct Menu {
    rows: Vec<Row>,
    lines: Vec<Line>,
    cells: Vec<Cell>,
    seps: Vec<(i32, i32)>,
    w: usize,
    h: usize,
}

const STEPS: [u8; 4] = [25, 50, 75, 100];

impl Menu {
    pub fn new(s: &Settings) -> Menu {
        Menu::build(s, false)
    }

    /// The menu with its last line asking "Quit PixelPet? Yes | No".
    pub fn confirming_quit(s: &Settings) -> Menu {
        Menu::build(s, true)
    }

    fn build(s: &Settings, confirm_quit: bool) -> Menu {
        let mut rows: Vec<Row> = vec![];
        let mut lines: Vec<Line> = vec![];
        let mut line = |rows: &mut Vec<Row>, heading: Option<&str>, opts: Vec<(String, Action, bool)>| {
            let h = heading.map(|t| {
                rows.push(Row { label: t.to_string(), action: None, checked: false });
                rows.len() - 1
            });
            let mut cells = vec![];
            for (label, action, checked) in opts {
                rows.push(Row { label, action: Some(action), checked });
                cells.push(rows.len() - 1);
            }
            lines.push(Line { heading: h, cells, own_column: false });
        };
        line(
            &mut rows,
            Some("Pet:"),
            vec![
                ("Alex".into(), Action::SetPet(PetChoice::Alex), s.pet == PetChoice::Alex),
                ("Alex the bun".into(), Action::SetPet(PetChoice::AlexTheBun), s.pet == PetChoice::AlexTheBun),
            ],
        );
        line(
            &mut rows,
            Some("Size:"),
            [(Size::S, "S"), (Size::M, "M"), (Size::L, "L"), (Size::XL, "XL")]
                .into_iter()
                .map(|(z, t)| (t.to_string(), Action::SetSize(z), s.size == z))
                .collect(),
        );
        line(
            &mut rows,
            Some("Opacity:"),
            STEPS.iter().map(|&o| (o.to_string(), Action::SetOpacity(o), s.opacity == o)).collect(),
        );
        line(
            &mut rows,
            Some("Fullscreen:"),
            vec![
                ("Hide".into(), Action::SetFullscreen(FullscreenMode::Hide), s.fullscreen == FullscreenMode::Hide),
                ("Stay visible".into(), Action::SetFullscreen(FullscreenMode::Stay), s.fullscreen == FullscreenMode::Stay),
            ],
        );
        line(
            &mut rows,
            Some("When visible:"),
            STEPS.iter().map(|&o| (o.to_string(), Action::SetFullscreenOpacity(o), s.fullscreen_opacity == o)).collect(),
        );
        let startup = format!("Launch on startup [{}]", if s.autostart { "on" } else { "off" });
        line(&mut rows, None, vec![(startup, Action::ToggleAutostart, s.autostart)]);
        if confirm_quit {
            line(
                &mut rows,
                Some("Quit PixelPet?"),
                vec![("Yes".into(), Action::Quit, false), ("No".into(), Action::CancelQuit, false)],
            );
            lines.last_mut().unwrap().own_column = true;
        } else {
            line(&mut rows, None, vec![("Quit".into(), Action::AskQuit, false)]);
        }

        // layout: headings in one column, options after the widest heading
        let tw = |r: usize| font::text_width(&rows[r].label) as i32;
        let head_w = lines.iter().filter(|l| !l.own_column).filter_map(|l| l.heading).map(tw).max().unwrap_or(0);
        let mut cells = vec![];
        let mut seps = vec![];
        let mut right = 0;
        for (i, l) in lines.iter().enumerate() {
            let y = INSET + i as i32 * (CELL_H + LINE_GAP);
            if let Some(h) = l.heading {
                cells.push(Cell { row: h, x: INSET, y, w: tw(h) });
            }
            let mut x = match l.heading {
                Some(h) if l.own_column => INSET + tw(h) + HEADING_GAP,
                Some(_) => INSET + head_w + HEADING_GAP,
                None => INSET - CELL_PAD_X,
            };
            for (k, &r) in l.cells.iter().enumerate() {
                if k > 0 {
                    seps.push((x, y));
                    x += SEP;
                }
                let w = tw(r) + 2 * CELL_PAD_X;
                cells.push(Cell { row: r, x, y, w });
                x += w;
            }
            right = right.max(x);
        }
        let n = lines.len() as i32;
        let w = (right + INSET) as usize;
        let h = (2 * INSET + n * CELL_H + (n - 1) * LINE_GAP) as usize;
        Menu { rows, lines, cells, seps, w, h }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Each line as text: `heading opt | opt`, as written in the copy.
    pub fn lines(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|l| {
                let opts: Vec<&str> = l.cells.iter().map(|&r| self.rows[r].label.as_str()).collect();
                match l.heading {
                    Some(h) => format!("{} {}", self.rows[h].label, opts.join(" | ")),
                    None => opts.join(" | "),
                }
            })
            .collect()
    }

    /// Panel size in art px.
    pub fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    /// Rect `(x, y, w, h)` in art px of the first cell with this label.
    pub fn cell_rect(&self, label: &str) -> Option<(i32, i32, i32, i32)> {
        self.cells.iter().find(|c| self.rows[c.row].label == label).map(|c| (c.x, c.y, c.w, CELL_H))
    }

    pub fn cell_centre(&self, label: &str) -> Option<(i32, i32)> {
        self.cell_rect(label).map(|(x, y, w, h)| (x + w / 2, y + h / 2))
    }

    /// The action under image pixel `(x, y)` of `draw(scale)`.
    pub fn hit(&self, x: i32, y: i32, scale: u32) -> Option<Action> {
        let s = scale.max(1) as i32;
        let (ax, ay) = (x.div_euclid(s), y.div_euclid(s));
        self.cells
            .iter()
            .find(|c| ax >= c.x && ax < c.x + c.w && ay >= c.y && ay < c.y + CELL_H)
            .and_then(|c| self.rows[c.row].action)
    }

    pub fn draw(&self, scale: u32) -> Rgba {
        let (w, h) = (self.w, self.h);
        let mut art = vec![[0u8; 3]; w * h];
        let mut put = |x: i32, y: i32, c: [u8; 3]| {
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                art[y as usize * w + x as usize] = c;
            }
        };
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let edge = x == 0 || y == 0 || x == w as i32 - 1 || y == h as i32 - 1;
                put(x, y, if edge { OUTLINE } else { FILL });
            }
        }
        let text = |put: &mut dyn FnMut(i32, i32, [u8; 3]), x: i32, y: i32, s: &str, c: [u8; 3]| {
            for (i, ch) in s.chars().enumerate() {
                let g = font::glyph(ch).unwrap_or([0; 7]);
                let gx = x + i as i32 * (font::W as i32 + 1);
                for (row, bits) in g.iter().enumerate() {
                    for col in 0..font::W {
                        if bits >> (font::W - 1 - col) & 1 == 1 {
                            put(gx + col as i32, y + row as i32, c);
                        }
                    }
                }
            }
        };
        for c in &self.cells {
            let row = &self.rows[c.row];
            let ty = c.y + 2;
            if row.action.is_none() {
                text(&mut put, c.x, ty, &row.label, TEXT);
                continue;
            }
            let colour = if row.checked {
                for y in c.y..c.y + CELL_H {
                    for x in c.x..c.x + c.w {
                        put(x, y, SELECTED);
                    }
                }
                SELECTED_TEXT
            } else {
                TEXT
            };
            text(&mut put, c.x + CELL_PAD_X, ty, &row.label, colour);
        }
        for &(x, y) in &self.seps {
            text(&mut put, x + 1, y + 2, "|", TEXT);
        }
        // scale up, nearest neighbour; the panel is fully opaque
        let z = scale.max(1) as usize;
        let mut img = Rgba::new(w * z, h * z);
        for y in 0..h * z {
            for x in 0..w * z {
                let c = art[(y / z) * w + x / z];
                let i = (y * w * z + x) * 4;
                img.px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        img
    }
}
