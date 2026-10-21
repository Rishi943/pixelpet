//! Writes the right-click menu, as drawn for default settings at zoom 3, to
//! `target/menu-preview.ppm` so it can be looked at without running the app.
//! An argument is shown as the note under "Launch on startup".

use pixelpet::settings::Settings;
use pixelpet::ui::menu::Menu;

fn main() {
    let s = Settings::default();
    let menu = match std::env::args().nth(1) {
        Some(note) => Menu::with_note(&s, &note),
        None => Menu::new(&s),
    };
    let img = menu.draw(3);
    let mut out = format!("P6\n{} {}\n255\n", img.w, img.h).into_bytes();
    for p in img.px.chunks(4) {
        out.extend_from_slice(&p[..3]);
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("menu-preview.ppm");
    std::fs::write(&path, out).expect("write preview");
    println!("wrote {} ({}x{})", path.display(), img.w, img.h);
}
