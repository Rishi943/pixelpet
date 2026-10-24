//! Write the app icon.
//!   `cargo run --example make_icon -- <out.ico>`  the Windows icon (default
//!     `target/release/PixelPet.ico`, next to the exe, where
//!     `tools/start-menu.ps1` looks for it)
//!   `cargo run --example make_icon -- --pngs <dir>`  `<dir>/PixelPet.iconset/`
//!     (the sizes `iconutil` wants for the macOS .icns) and `<dir>/pixelpet.png`
//!     (256 px, for the Linux AppImage)

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--pngs") {
        let dir = std::path::PathBuf::from(args.get(1).expect("--pngs <dir>"));
        let set = dir.join("PixelPet.iconset");
        std::fs::create_dir_all(&set).expect("make the iconset folder");
        for base in [16, 32, 128, 256, 512] {
            for (scale, suffix) in [(1, ""), (2, "@2x")] {
                let name = format!("icon_{base}x{base}{suffix}.png");
                std::fs::write(set.join(name), pixelpet::ui::icon::png(base * scale)).expect("write a png");
            }
        }
        std::fs::write(dir.join("pixelpet.png"), pixelpet::ui::icon::png(256)).expect("write pixelpet.png");
        println!("wrote {} and {}", set.display(), dir.join("pixelpet.png").display());
        return;
    }
    let out = args.first().cloned().unwrap_or_else(|| "target/release/PixelPet.ico".into());
    std::fs::write(&out, pixelpet::ui::icon::ico()).expect("write the icon");
    println!("wrote {out}");
}
