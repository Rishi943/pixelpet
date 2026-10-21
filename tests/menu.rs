//! The right-click settings menu: copy, hit-testing, applying choices, font.

use pixelpet::settings::{FullscreenMode, Settings, Size};
use pixelpet::ui::font;
use pixelpet::ui::menu::{apply, step_pet, Action, Menu};

const COPY: [&str; 8] = [
    "Pet: < Alex >",
    "Size: S | M | L | XL",
    "Opacity: 25 | 50 | 75 | 100",
    "Fullscreen: Hide | Stay visible",
    "When visible: 25 | 50 | 75 | 100",
    "Launch on startup [off]",
    "Make a pet...",
    "Quit",
];

#[test]
fn rows_match_copy() {
    let m = Menu::new(&Settings::default(), "Alex");
    assert_eq!(m.lines(), COPY.map(String::from).to_vec());
    let checked: Vec<&str> = m.rows().iter().filter(|r| r.checked).map(|r| r.label.as_str()).collect();
    // defaults: Alex, M, opacity 100, hide, when-visible 50, startup off
    assert_eq!(checked, ["Alex", "M", "100", "Hide", "50"]);
    let mut s = Settings::default();
    s.autostart = true;
    let m = Menu::new(&s, "Alex");
    assert_eq!(m.lines()[5], "Launch on startup [on]");
    assert!(m.rows().iter().any(|r| r.checked && r.label == "Launch on startup [on]"));
}

#[test]
fn hit_finds_cells() {
    let m = Menu::new(&Settings::default(), "Alex");
    for scale in [1, 3] {
        let (x, y) = m.cell_centre("L").expect("an L cell");
        assert_eq!(m.hit(x * scale as i32, y * scale as i32, scale), Some(Action::SetSize(Size::L)), "scale {scale}");
        let (x, y) = m.cell_centre("Quit").unwrap();
        assert_eq!(m.hit(x * scale as i32, y * scale as i32, scale), Some(Action::AskQuit));
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
    let m = Menu::new(&Settings::default(), "Alex");
    let img = m.draw(1);
    let (x, y, _, _) = m.cell_rect("Alex").unwrap();
    let i = ((y as usize + 1) * img.w + x as usize) * 4;
    assert_eq!(&img.px[i..i + 4], &[0x2F, 0x64, 0xC8, 255]);
}

#[test]
fn apply_changes_settings() {
    let mut s = Settings::default();
    // the pet actions are the app's (they need the saved pets)
    for a in [Action::PrevPet, Action::NextPet, Action::EditPet, Action::MakePet] {
        apply(&a, &mut s);
        assert_eq!(s, Settings::default());
    }
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

#[test]
fn quit_asks_first() {
    let s = Settings::default();
    let m = Menu::new(&s, "Alex");
    let (x, y) = m.cell_centre("Quit").unwrap();
    assert_eq!(m.hit(x, y, 1), Some(Action::AskQuit), "the first click only asks");
    let m = Menu::confirming_quit(&s, "Alex");
    assert_eq!(m.lines()[7], "Quit PixelPet? Yes | No");
    let (x, y) = m.cell_centre("Yes").unwrap();
    assert_eq!(m.hit(x, y, 1), Some(Action::Quit));
    let (x, y) = m.cell_centre("No").unwrap();
    assert_eq!(m.hit(x, y, 1), Some(Action::CancelQuit));
    // asking changes no setting
    let mut t = s.clone();
    apply(&Action::AskQuit, &mut t);
    apply(&Action::CancelQuit, &mut t);
    assert_eq!(t, s);
}

/// When launch on startup is refused (macOS: run from the dmg or Downloads),
/// the menu says why instead of the toggle silently flipping back.
#[test]
fn a_refused_autostart_is_explained_in_the_menu() {
    let s = Settings::default();
    let m = Menu::with_note(&s, "Alex", "Move PixelPet to Applications first");
    assert_eq!(m.lines()[5], "Launch on startup [off]");
    assert_eq!(m.lines()[6], "Move PixelPet to Applications first");
    assert_eq!(m.lines()[7], "Make a pet...");
    assert_eq!(m.lines()[8], "Quit");
    // the note is not clickable, and the menu keeps its other widths
    assert!(m.rows().iter().all(|r| r.label != "Move PixelPet to Applications first" || r.action.is_none()));
    for c in "Move PixelPet to Applications first".chars() {
        assert!(font::glyph(c).is_some(), "font lacks {c:?}");
    }
}

#[test]
fn the_pet_line_is_arrows_round_the_name() {
    let m = Menu::new(&Settings::default(), "Biscuit");
    assert_eq!(m.lines()[0], "Pet: < Biscuit >");
    for (label, action) in [("<", Action::PrevPet), ("Biscuit", Action::EditPet), (">", Action::NextPet), ("Make a pet...", Action::MakePet)] {
        let (x, y) = m.cell_centre(label).unwrap();
        assert_eq!(m.hit(x, y, 1), Some(action), "{label}");
    }
    assert!(m.rows().iter().any(|r| r.checked && r.label == "Biscuit"), "the pet's name is shown selected");
}

#[test]
fn step_pet_wraps() {
    let dir = std::env::temp_dir().join(format!("pixelpet-menu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut r = pixelpet::pets::builtins()[0].clone();
    r.id = "p9".into();
    r.name = "Bean".into();
    pixelpet::pets::save(&dir, &r).unwrap();
    let all = pixelpet::pets::load_all(&dir);
    assert_eq!(step_pet(&all, "alex", 1), "alex-the-bun");
    assert_eq!(step_pet(&all, "p9", 1), "alex", "past the last: the first");
    assert_eq!(step_pet(&all, "alex", -1), "p9", "before the first: the last");
    assert_eq!(step_pet(&all, "gone", 1), "alex-the-bun", "an unknown pet counts as the first");
}

#[test]
fn deleting_the_current_pet_falls_back_to_alex() {
    let all = pixelpet::pets::load_all(&std::env::temp_dir().join("pixelpet-menu-none"));
    let current = pixelpet::pets::find(&all, "p-deleted");
    assert_eq!(current.id, "alex");
    assert_eq!(Menu::new(&Settings::default(), &current.name).lines()[0], "Pet: < Alex >");
}
