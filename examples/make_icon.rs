//! Write the app icon: `cargo run --example make_icon -- <out.ico>`
//! (default `target/release/PixelPet.ico`, next to the exe, where
//! `tools/start-menu.ps1` looks for it).

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "target/release/PixelPet.ico".into());
    std::fs::write(&out, pixelpet::ui::icon::ico()).expect("write the icon");
    println!("wrote {out}");
}
