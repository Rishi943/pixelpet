// Release builds have no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(e) = pixelpet::app::run() {
        eprintln!("PixelPet: {e}");
        std::process::exit(1);
    }
}
