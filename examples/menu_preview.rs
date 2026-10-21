//! Writes the right-click menu, as drawn for default settings at zoom 3, to
//! `target/menu-preview.ppm` so it can be looked at without running the app.
//! An argument is shown as the note under "Launch on startup".
//! `maker N` writes the pet maker on step N (1-5) to
//! `target/maker-preview-N.ppm` instead, with Alex in the preview.

use pixelpet::rig;
use pixelpet::settings::Settings;
use pixelpet::ui::maker::{Maker, Outcome};
use pixelpet::ui::menu::Menu;
use pixelpet::window::Rgba;

fn write(img: &Rgba, name: &str) {
    let mut out = format!("P6\n{} {}\n255\n", img.w, img.h).into_bytes();
    for p in img.px.chunks(4) {
        out.extend_from_slice(&p[..3]);
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(name);
    std::fs::write(&path, out).expect("write preview");
    println!("wrote {} ({}x{})", path.display(), img.w, img.h);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("maker") {
        let step: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).clamp(1, 5);
        let mut m = Maker::new(7);
        for _ in 1..step {
            let (x, y, w, h) = m.target("Next >").expect("a Next button");
            assert!(matches!(m.click(x + w / 2, y + h / 2), Outcome::Redraw));
        }
        let z = 3usize;
        let look = rig::look("classic");
        let d = m.design();
        let f = rig::render_for(d, &rig::pose_for(d, m.preview_anim(), look, 0.0, None), look);
        let mut pet = Rgba::new(f.w * z, f.h * z);
        for y in 0..f.h * z {
            for x in 0..f.w * z {
                let s = ((y / z) * f.w + x / z) * 4;
                pet.px[(y * f.w * z + x) * 4..][..4].copy_from_slice(&f.rgba[s..s + 4]);
            }
        }
        let anchor = (f.ox * z as i32, (f.gy + 1) * z as i32);
        write(&m.draw(z as u32, &pet, anchor), &format!("maker-preview-{step}.ppm"));
        return;
    }
    let s = Settings::default();
    let menu = match args.first() {
        Some(note) => Menu::with_note(&s, "Alex", note),
        None => Menu::new(&s, "Alex"),
    };
    write(&menu.draw(3), "menu-preview.ppm");
}
