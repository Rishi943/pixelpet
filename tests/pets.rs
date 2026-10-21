//! Saved pets: small key = value files, built-ins that never change, and
//! fallbacks for anything missing or broken.

use pixelpet::pets::{self, PetRecord, ALEX_ID, BUN_ID};
use pixelpet::rig::breeds::breeds;
use pixelpet::rig::core::fam;
use pixelpet::rig::Pet;
use pixelpet::settings::Settings;
use std::path::PathBuf;

fn dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("pixelpet-pets-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn husky() -> PetRecord {
    let mut design = (breeds(Pet::Dog)[3].design)();
    design.shape.legs = 1.137;
    design.shape.tail = 0.55;
    design.collar = Some(0xC8384A);
    PetRecord { id: "p1".into(), name: "Biscuit".into(), breed: "Husky".into(), design, extra: vec![] }
}

#[test]
fn a_record_round_trips() {
    let r = husky();
    let back = pets::from_text("p1", &pets::to_text(&r)).expect("parses");
    assert_eq!(back, r);
}

#[test]
fn bad_values_fall_back_and_unknown_keys_survive() {
    let text = "species = dog\nbreed = Husky\nshape.legs = 99\ncoat.saddle = blue\ncollar = #12\nfuture.key = x\n";
    let r = pets::from_text("p2", text).expect("parses");
    let husky = (breeds(Pet::Dog)[3].design)();
    assert_eq!(r.design.shape.legs, 1.3, "clamped to the slider's top");
    let saddle = |d: &pixelpet::rig::Design| d.coat.iter().find(|(f, _)| *f == fam::BLACK).map(|c| c.1);
    assert_eq!(saddle(&r.design), saddle(&husky), "a bad colour keeps the breed's");
    assert_eq!(r.design.collar, None);
    assert_eq!(r.name, "Husky", "no name: the breed's");
    assert!(pets::to_text(&r).contains("future.key = x"));
    // an unknown species can't be shown
    assert!(pets::from_text("p3", "species = cat\nname = Tom\n").is_none());
}

#[test]
fn builtins_come_first_and_cannot_be_saved_or_deleted() {
    let d = dir("builtins");
    pets::save(&d, &husky()).unwrap();
    let all = pets::load_all(&d);
    assert_eq!(all.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), [ALEX_ID, BUN_ID, "p1"]);
    assert_eq!(all[0].name, "Alex");
    assert_eq!(all[1].name, "Alex the bun");
    assert!(pets::is_builtin(ALEX_ID) && pets::is_builtin(BUN_ID) && !pets::is_builtin("p1"));
    let mut fake = all[0].clone();
    fake.name = "Not Alex".into();
    assert!(pets::save(&d, &fake).is_err());
    assert!(pets::delete(&d, ALEX_ID).is_err());
    pets::delete(&d, "p1").unwrap();
    assert_eq!(pets::load_all(&d).len(), 2);
}

#[test]
fn broken_files_are_skipped() {
    let d = dir("broken");
    std::fs::write(d.join("p5.txt"), "species = cat\n").unwrap();
    std::fs::write(d.join("p6.txt"), [0xFFu8, 0xFE, 0x00]).unwrap();
    pets::save(&d, &husky()).unwrap();
    assert_eq!(pets::load_all(&d).len(), 3);
}

#[test]
fn missing_pet_falls_back_to_alex() {
    let all = pets::load_all(&dir("missing"));
    assert_eq!(pets::find(&all, "p123").id, ALEX_ID);
    assert_eq!(pets::find(&all, BUN_ID).id, BUN_ID);
}

#[test]
fn ids_are_unique() {
    let d = dir("ids");
    let a = pets::new_id(&d, 1000);
    std::fs::write(d.join(format!("{a}.txt")), "species = dog\n").unwrap();
    let b = pets::new_id(&d, 1000);
    assert_ne!(a, b);
    assert!(a.starts_with("p1000"));
}

#[test]
fn v1_settings_still_choose_the_bun() {
    assert_eq!(Settings::parse("pet = alex-the-bun\n").pet, BUN_ID);
    assert_eq!(Settings::parse("").pet, ALEX_ID);
    assert!(Settings::parse("pet = p42\n").to_text().contains("pet = p42"));
}
