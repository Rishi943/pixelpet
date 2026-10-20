//! The hover bubble: Pet / Play / Feed, with a page per choice.

use pixelpet::brain::Choice;
use pixelpet::rig::{Food, Pet, Toy};
use pixelpet::ui::bubble::{Bubble, Page};

/// Click the centre of the cell labelled `label` at `scale`.
fn click(b: &mut Bubble, label: &str, scale: u32) -> Option<Choice> {
    let (x, y) = b.cell_centre(label).unwrap_or_else(|| panic!("no {label} on {:?}: {}", b.page(), b.lines()));
    b.click(x * scale as i32, y * scale as i32, scale)
}

#[test]
fn bubble_copy() {
    let mut b = Bubble::new(Pet::Dog);
    assert_eq!(b.lines(), "Pet | Play | Feed");
    assert_eq!(click(&mut b, "Play", 1), None);
    assert_eq!(b.page(), Page::Play);
    assert_eq!(b.lines(), "Ball | Coconut | Tug-of-war | Back");
    assert_eq!(click(&mut b, "Back", 1), None);
    assert_eq!(click(&mut b, "Feed", 1), None);
    assert_eq!(b.lines(), "Chicken | Bone | Kibble | Back");
    let mut b = Bubble::new(Pet::Rabbit);
    click(&mut b, "Play", 1);
    assert_eq!(b.lines(), "Football | Tug-of-war | Back");
    click(&mut b, "Back", 1);
    click(&mut b, "Feed", 1);
    assert_eq!(b.lines(), "Carrot | Coriander | Banana | Back");
}

#[test]
fn bubble_choices() {
    let s = 3;
    let mut b = Bubble::new(Pet::Dog);
    assert_eq!(click(&mut b, "Pet", s), Some(Choice::Pet));
    let mut b = Bubble::new(Pet::Dog);
    click(&mut b, "Play", s);
    assert_eq!(click(&mut b, "Coconut", s), Some(Choice::Fetch(Toy::Coconut)));
    let mut b = Bubble::new(Pet::Dog);
    click(&mut b, "Play", s);
    assert_eq!(click(&mut b, "Ball", s), Some(Choice::Fetch(Toy::Ball)));
    let mut b = Bubble::new(Pet::Dog);
    click(&mut b, "Play", s);
    assert_eq!(click(&mut b, "Tug-of-war", s), Some(Choice::Tug));
    let mut b = Bubble::new(Pet::Dog);
    click(&mut b, "Feed", s);
    assert_eq!(click(&mut b, "Chicken", s), Some(Choice::Feed(Food::Chicken)));
    let mut b = Bubble::new(Pet::Rabbit);
    click(&mut b, "Play", s);
    assert_eq!(click(&mut b, "Football", s), Some(Choice::Fetch(Toy::Football)));
    // the border and the tail are not buttons
    let mut b = Bubble::new(Pet::Dog);
    let (w, h) = b.size();
    assert_eq!(b.click(0, 0, s), None);
    assert_eq!(b.click((w as i32 / 2) * s as i32, (h as i32 - 2) * s as i32, s), None);
    assert_eq!(b.page(), Page::Top, "a miss changes nothing");
}

#[test]
fn bubble_has_a_tail() {
    let b = Bubble::new(Pet::Dog);
    let img = b.draw(1);
    let (w, h) = b.size();
    assert_eq!((img.w, img.h), (w, h));
    let centre = w as i32 / 2;
    let mut tail = 0;
    for y in h - 4..h {
        for x in 0..w {
            if img.px[(y * w + x) * 4 + 3] > 0 {
                tail += 1;
                assert!((x as i32 - centre).abs() <= 5, "tail pixel at ({x},{y}), centre {centre}");
            }
        }
    }
    assert!(tail >= 6, "no tail");
    // same colours as the menu: dark outline, cream fill
    assert_eq!(&img.px[(0 * w + w / 2) * 4..][..4], &[0x24, 0x12, 0x0A, 255]);
    assert_eq!(&img.px[((h / 2 - 2) * w + 2) * 4..][..4], &[0xF3, 0xD2, 0x9C, 255]);
}
