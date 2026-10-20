//! Golden-frame loader and diff writer shared by the parity tests.
#![allow(dead_code)]

use pixelpet::rig::{self, Anim, Frame, Pet, Toy};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Case {
    pub name: String,
    pub pet: String,
    pub look: String,
    pub anim: String,
    pub t: f64,
    pub toy: Option<Toy>,
    pub collar: bool,
    pub lerp: Option<(Anim, f64, f64)>,
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden")
}

pub fn cases() -> Vec<Case> {
    let text = fs::read_to_string(golden_dir().join("cases.txt")).expect("cases.txt");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('|').collect();
            assert_eq!(f.len(), 8, "bad case line: {l}");
            let lerp = if f[7] == "-" {
                None
            } else {
                let p: Vec<&str> = f[7].split(':').collect();
                Some((Anim::from_name(p[0]).unwrap(), p[1].parse().unwrap(), p[2].parse().unwrap()))
            };
            Case {
                name: f[0].into(),
                pet: f[1].into(),
                look: f[2].into(),
                anim: f[3].into(),
                t: f[4].parse().unwrap(),
                toy: if f[5] == "-" { None } else { Some(Toy::from_name(f[5]).unwrap()) },
                collar: f[6] == "1",
                lerp,
            }
        })
        .collect()
}

pub fn load(name: &str) -> Frame {
    let b = fs::read(golden_dir().join(format!("{name}.ppg"))).expect("golden file");
    assert_eq!(&b[0..4], b"PPG1");
    let u = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    let w = u(4) as usize;
    let h = u(8) as usize;
    let ox = u(12) as i32;
    let gy = u(16) as i32;
    let rgba = b[20..].to_vec();
    assert_eq!(rgba.len(), w * h * 4);
    Frame { w, h, ox, gy, rgba }
}

/// Render a case with the Rust rig.
pub fn render(c: &Case) -> Frame {
    let look = rig::look(&c.look);
    if c.pet == "toy" {
        return rig::render_toy(c.toy.unwrap(), look);
    }
    let pet = match c.pet.as_str() {
        "dog" => Pet::Dog,
        "rabbit" => Pet::Rabbit,
        p => panic!("unknown pet {p}"),
    };
    let anim = Anim::from_name(&c.anim).unwrap();
    let mut s = rig::pose(pet, anim, look, c.t, c.toy);
    if let Some((a2, t2, k)) = c.lerp {
        s = rig::lerp(&s, &rig::pose(pet, a2, look, t2, c.toy), k);
    }
    rig::render(&s, look, c.collar)
}

/// Compare one case; returns the number of differing pixels (or panics on a
/// size mismatch). Writes `target/parity/<name>-diff.ppm` when any differ.
pub fn compare(c: &Case) -> usize {
    let g = load(&c.name);
    let r = render(c);
    assert_eq!((r.w, r.h, r.ox, r.gy), (g.w, g.h, g.ox, g.gy), "{}: frame geometry", c.name);
    let n = g.w * g.h;
    let diff: Vec<bool> = (0..n).map(|i| g.rgba[i * 4..i * 4 + 4] != r.rgba[i * 4..i * 4 + 4]).collect();
    let count = diff.iter().filter(|d| **d).count();
    if count > 0 {
        write_diff(&c.name, &g, &r, &diff);
    }
    count
}

fn write_diff(name: &str, g: &Frame, r: &Frame, diff: &[bool]) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("parity");
    fs::create_dir_all(&dir).unwrap();
    let (w, h) = (g.w, g.h);
    let mut out = format!("P6\n{} {}\n255\n", w * 3, h).into_bytes();
    let bg = [214u8, 220, 228];
    let px = |f: &Frame, i: usize| -> [u8; 3] {
        if f.rgba[i * 4 + 3] == 0 { bg } else { [f.rgba[i * 4], f.rgba[i * 4 + 1], f.rgba[i * 4 + 2]] }
    };
    for y in 0..h {
        for x in 0..w {
            out.extend_from_slice(&px(g, y * w + x));
        }
        for x in 0..w {
            out.extend_from_slice(&px(r, y * w + x));
        }
        for x in 0..w {
            out.extend_from_slice(&if diff[y * w + x] { [255, 0, 0] } else { [40, 40, 40] });
        }
    }
    fs::write(dir.join(format!("{name}-diff.ppm")), out).unwrap();
}

/// Run every case matching `pick`, print the differing ones, and assert each
/// is within 0.5 % of its pixels.
pub fn check_all(pick: impl Fn(&Case) -> bool) {
    let cs: Vec<Case> = cases().into_iter().filter(|c| pick(c)).collect();
    assert!(!cs.is_empty(), "no cases selected");
    let mut bad = vec![];
    for c in &cs {
        let n = compare(c);
        let g = load(&c.name);
        if n > 0 {
            println!("{}: {} px differ", c.name, n);
        }
        if n as f64 > 0.005 * (g.w * g.h) as f64 {
            bad.push(format!("{} ({} px)", c.name, n));
        }
    }
    println!("{} cases checked", cs.len());
    assert!(bad.is_empty(), "over 0.5 % differ: {}", bad.join(", "));
}
