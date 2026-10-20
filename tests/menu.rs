//! The right-click settings menu: copy, hit-testing, applying choices, font.

use pixelpet::settings::{FullscreenMode, PetChoice, Settings, Size};
use pixelpet::ui::font;
use pixelpet::ui::menu::{apply, Action, Menu};

const COPY: [&str; 7] = [
    "Pet: Alex | Alex the bun",
    "Size: S | M | L | XL",
    "Opacity: 25 | 50 | 75 | 100",
    "Fullscreen: Hide | Stay visible",
    "When visible: 25 | 50 | 75 | 100",
    "Launch on startup [off]",
    "Quit",
];

#[test]
fn rows_match_copy() {
    let m = Menu::new(&Settings::default());
    assert_eq!(m.lines(), COPY.map(String::from).to_vec());
    let checked: Vec<&str> = m.rows().iter().filter(|r| r.checked).map(|r| r.label.as_str()).collect();
    // defaults: Alex, M, opacity 100, hide, when-visible 50, startup off
    assert_eq!(checked, ["Alex", "M", "100", "Hide", "50"]);
    let mut s = Settings::default();
    s.autostart = true;
    let m = Menu::new(&s);
    assert_eq!(m.lines()[5], "Launch on startup [on]");
    assert!(m.rows().iter().any(|r| r.checked && r.label == "Launch on startup [on]"));
}

#[test]
fn hit_finds_cells() {
    let m = Menu::new(&Settings::default());
    for scale in [1, 3] {
        let (x, y) = m.cell_centre("L").expect("an L cell");
        assert_eq!(m.hit(x * scale as i32, y * scale as i32, scale), Some(Action::SetSize(Size::L)), "scale {scale}");
        let (x, y) = m.cell_centre("Quit").unwrap();
        assert_eq!(m.hit(x * scale as i32, y * scale as i32, scale), Some(Action::Quit));
        assert_eq!(m.hit(0, 0, scale), None, "panel border");
        let (x, y) = m.cell_centre("Pet:").unwrap();
        assert_eq!(m.hit(x * scale as i32, y * scale as i32, scale), None, "a heading is not clickable");
    }
    let img = m.draw(3);
    let (w, h) = m.size();
    assert_eq!((img.w, img.h), (w * 3, h * 3));
    // the border is the dark outline, the inside is cream
    assert_eq!(&img.px[0..4], &[0x24, 0x12, 0x0A, 255]);
    let i = ((h * 3 / 2) * img.w + 4) * 4;
    assert_eq!(&img.px[i..i + 4], &[0xF3, 0xD2, 0x9C, 255]);
}

#[test]
fn selected_cell_is_blue() {
    let m = Menu::new(&Settings::default());
    let img = m.draw(1);
    let (x, y, _, _) = m.cell_rect("Alex").unwrap();
    let i = ((y as usize + 1) * img.w + x as usize) * 4;
    assert_eq!(&img.px[i..i + 4], &[0x2F, 0x64, 0xC8, 255]);
}

#[test]
fn apply_changes_settings() {
    let mut s = Settings::default();
    apply(&Action::SetPet(PetChoice::AlexTheBun), &mut s);
    assert_eq!(s.pet, PetChoice::AlexTheBun);
    apply(&Action::SetSize(Size::XL), &mut s);
    assert_eq!(s.size, Size::XL);
    apply(&Action::SetOpacity(25), &mut s);
    assert_eq!(s.opacity, 25);
    apply(&Action::SetFullscreen(FullscreenMode::Stay), &mut s);
    assert_eq!(s.fullscreen, FullscreenMode::Stay);
    apply(&Action::SetFullscreenOpacity(75), &mut s);
    assert_eq!(s.fullscreen_opacity, 75);
    apply(&Action::ToggleAutostart, &mut s);
    assert!(s.autostart);
    apply(&Action::ToggleAutostart, &mut s);
    assert!(!s.autostart);
    let before = s.clone();
    apply(&Action::Quit, &mut s);
    assert_eq!(s, before);
}

#[test]
fn font_covers_menu_text() {
    for line in COPY.iter().chain(["Launch on startup [on]"].iter()) {
        for c in line.chars() {
            assert!(font::glyph(c).is_some(), "no glyph for {c:?}");
        }
    }
    for c in ' '..='~' {
        assert!(font::glyph(c).is_some(), "no glyph for {c:?}");
    }
    // a space is blank, everything else draws something
    assert_eq!(font::glyph(' '), Some([0; 7]));
    for c in '!'..='~' {
        assert!(font::glyph(c).unwrap().iter().any(|r| *r != 0), "{c:?} is blank");
    }
}
