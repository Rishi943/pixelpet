//! Saved pets: one small `key = value` file per pet in a `pets/` folder next
//! to the settings file. Alex, Alex the bun, Mellow and Oreo are built in:
//! always there, first, and never changed or deleted. Anything missing or broken in a
//! file falls back to the pet's breed; unknown keys are kept.

use crate::rig::breeds::{areas, breeds, sliders};
use crate::rig::{Design, Pet};
use std::io;
use std::path::{Path, PathBuf};

pub const ALEX_ID: &str = "alex";
pub const BUN_ID: &str = "alex-the-bun";
pub const MELLOW_ID: &str = "mellow";
pub const OREO_ID: &str = "oreo";
/// Longest name the maker takes.
pub const NAME_MAX: usize = 24;

#[derive(Clone, Debug, PartialEq)]
pub struct PetRecord {
    pub id: String,
    pub name: String,
    pub breed: String,
    pub design: Design,
    /// Keys this version doesn't know, kept as they were.
    pub extra: Vec<(String, String)>,
}

pub fn builtins() -> [PetRecord; 4] {
    [
        PetRecord { id: ALEX_ID.into(), name: "Alex".into(), breed: "Alex the GSD".into(), design: Design::alex(), extra: vec![] },
        PetRecord { id: BUN_ID.into(), name: "Alex the bun".into(), breed: "Alex the bun".into(), design: Design::bun(), extra: vec![] },
        PetRecord { id: MELLOW_ID.into(), name: "Mellow".into(), breed: "Mellow".into(), design: Design::mellow(), extra: vec![] },
        PetRecord { id: OREO_ID.into(), name: "Oreo".into(), breed: "Oreo".into(), design: Design::oreo(), extra: vec![] },
    ]
}

pub fn is_builtin(id: &str) -> bool {
    [ALEX_ID, BUN_ID, MELLOW_ID, OREO_ID].contains(&id)
}

/// A name as the maker allows it: printable ASCII, at most NAME_MAX.
pub fn clean_name(s: &str) -> String {
    s.chars().filter(|c| (' '..='~').contains(c)).take(NAME_MAX).collect::<String>().trim().to_string()
}

fn species_key(p: Pet) -> &'static str {
    match p {
        Pet::Dog => "dog",
        Pet::Rabbit => "rabbit",
        Pet::Cat => "cat",
    }
}

pub fn to_text(r: &PetRecord) -> String {
    let pet = r.design.pet;
    let mut t = String::from("# a PixelPet pet\n");
    t += &format!("name = {}\n", r.name);
    t += &format!("species = {}\n", species_key(pet));
    t += &format!("breed = {}\n", r.breed);
    for s in sliders(pet) {
        t += &format!("shape.{} = {}\n", s.field, r.design.shape.get(s.field).unwrap_or(1.0));
    }
    for &(f, c) in &r.design.coat {
        if let Some((_, label)) = areas(pet).iter().find(|(af, _)| *af == f) {
            t += &format!("coat.{} = #{:06X}\n", label.to_ascii_lowercase(), c);
        }
    }
    t += &match r.design.collar {
        Some(c) => format!("collar = #{c:06X}\n"),
        None => "collar = none\n".into(),
    };
    for (k, v) in &r.extra {
        t += &format!("{k} = {v}\n");
    }
    t
}

fn colour(v: &str) -> Option<u32> {
    let h = v.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    u32::from_str_radix(h, 16).ok()
}

/// None only when the pet can't be shown at all (no or unknown species).
pub fn from_text(id: &str, text: &str) -> Option<PetRecord> {
    let mut kv: Vec<(String, String)> = vec![];
    for line in text.strip_prefix('\u{feff}').unwrap_or(text).lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            kv.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let get = |k: &str| kv.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
    let pet = match get("species")? {
        "dog" => Pet::Dog,
        "rabbit" => Pet::Rabbit,
        "cat" => Pet::Cat,
        _ => return None,
    };
    let breed = breeds(pet).iter().find(|b| Some(b.name) == get("breed")).unwrap_or(&breeds(pet)[0]);
    let mut design = (breed.design)();
    let mut extra = vec![];
    for (k, v) in &kv {
        if let Some(field) = k.strip_prefix("shape.") {
            match (sliders(pet).iter().find(|s| s.field == field), v.parse::<f64>()) {
                (Some(s), Ok(x)) if x.is_finite() => {
                    design.shape.set(field, x.clamp(s.lo, s.hi));
                }
                (Some(_), _) => {}
                (None, _) => extra.push((k.clone(), v.clone())),
            }
        } else if let Some(label) = k.strip_prefix("coat.") {
            match (areas(pet).iter().find(|(_, l)| l.eq_ignore_ascii_case(label)), colour(v)) {
                (Some(&(f, _)), Some(c)) => match design.coat.iter_mut().find(|(cf, _)| *cf == f) {
                    Some(slot) => slot.1 = c,
                    None => design.coat.push((f, c)),
                },
                (Some(_), None) => {}
                (None, _) => extra.push((k.clone(), v.clone())),
            }
        } else if k == "collar" {
            design.collar = if pet == Pet::Rabbit { None } else { colour(v) };
        } else if !matches!(k.as_str(), "name" | "species" | "breed") {
            extra.push((k.clone(), v.clone()));
        }
    }
    let name = get("name").map(clean_name).filter(|n| !n.is_empty()).unwrap_or_else(|| breed.name.to_string());
    Some(PetRecord { id: id.to_string(), name, breed: breed.name.to_string(), design, extra })
}

/// `<folder of settings.txt>/pets`.
pub fn pets_dir(settings_path: &Path) -> PathBuf {
    settings_path.parent().map(Path::to_path_buf).unwrap_or_default().join("pets")
}

/// The built-ins, then saved pets in the order they were made (ids grow
/// with time). Files that can't be read or shown are skipped.
pub fn load_all(dir: &Path) -> Vec<PetRecord> {
    let mut all = builtins().to_vec();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "txt")).collect())
        .unwrap_or_default();
    files.sort();
    for p in files {
        let Some(id) = p.file_stem().and_then(|s| s.to_str()) else { continue };
        if is_builtin(id) {
            continue;
        }
        if let Some(r) = std::fs::read_to_string(&p).ok().and_then(|t| from_text(id, &t)) {
            all.push(r);
        }
    }
    all
}

pub fn save(dir: &Path, r: &PetRecord) -> io::Result<()> {
    if is_builtin(&r.id) || r.id.is_empty() {
        return Err(io::Error::other("the built-in pets can't be changed"));
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(format!("{}.txt", r.id)), to_text(r))
}

pub fn delete(dir: &Path, id: &str) -> io::Result<()> {
    if is_builtin(id) {
        return Err(io::Error::other("the built-in pets can't be deleted"));
    }
    std::fs::remove_file(dir.join(format!("{id}.txt")))
}

/// A fresh id from the time, unique in `dir`.
pub fn new_id(dir: &Path, now_ms: u128) -> String {
    let base = format!("p{now_ms}");
    let mut id = base.clone();
    let mut n = 1;
    while dir.join(format!("{id}.txt")).exists() {
        n += 1;
        id = format!("{base}-{n}");
    }
    id
}

/// The pet with `id`, or Alex when it's gone.
pub fn find<'a>(all: &'a [PetRecord], id: &str) -> &'a PetRecord {
    all.iter().find(|r| r.id == id).or_else(|| all.iter().find(|r| r.id == ALEX_ID)).unwrap_or(&all[0])
}
