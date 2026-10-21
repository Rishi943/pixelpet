//! The pet maker: steps, choices, sliders, swatches, the name box, Done and
//! Delete, and its size and font. Clicks find their targets by name.

use pixelpet::pets::{builtins, PetRecord};
use pixelpet::rig::breeds::{breeds, sliders};
use pixelpet::rig::core::fam;
use pixelpet::rig::{Design, Pet};
use pixelpet::ui::font;
use pixelpet::ui::maker::{self, Maker, MakerKey, Outcome, Step};
use pixelpet::window::Rgba;

fn click(m: &mut Maker, what: &str) -> Outcome {
    let (x, y, w, h) = m.target(what).unwrap_or_else(|| panic!("no {what:?} on step {:?}", m.step()));
    m.click(x + w / 2, y + h / 2)
}

fn to_shape(m: &mut Maker) {
    click(m, "Dog");
    click(m, "Next >");
    assert_eq!(m.step(), Step::Shape);
}

#[test]
fn steps_go_forward_and_back() {
    let mut m = Maker::new(1);
    assert_eq!(m.step(), Step::Animal);
    assert!(m.target("< Back").is_none(), "no Back on the first step");
    assert!(m.target("dot:3").is_none(), "steps not reached yet can't be clicked");
    click(&mut m, "Next >");
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Shape);
    click(&mut m, "< Back");
    assert_eq!(m.step(), Step::Breed);
    click(&mut m, "dot:2");
    assert_eq!(m.step(), Step::Shape, "a reached step can be jumped to");
    click(&mut m, "dot:0");
    assert_eq!(m.step(), Step::Animal);
}

#[test]
fn choosing_an_animal_goes_to_breed() {
    let mut m = Maker::new(1);
    click(&mut m, "Rabbit");
    assert_eq!(m.step(), Step::Breed);
    assert_eq!(m.design(), &Design::bun());
    assert_eq!(m.name(), "Alex the bun");
    assert!(m.target("Cat (later)").is_none(), "the cat isn't clickable");
}

#[test]
fn choosing_a_breed_sets_its_design() {
    let mut m = Maker::new(1);
    click(&mut m, "Dog");
    click(&mut m, "Husky");
    assert_eq!(m.design(), &(breeds(Pet::Dog)[3].design)());
    assert_ne!(m.name(), "Alex");
    click(&mut m, "Alex the GSD");
    assert_eq!(m.design(), &Design::alex());
    assert_eq!(m.name(), "Alex");
}

#[test]
fn a_slider_drag_sets_its_value_in_range() {
    let mut m = Maker::new(1);
    to_shape(&mut m);
    let (x, y, w, h) = m.target("slider:Legs").unwrap();
    m.click(x + 1, y + h / 2);
    m.drag(x + w + 40, y + h / 2);
    let legs = &sliders(Pet::Dog)[0];
    assert_eq!(m.design().shape.legs, legs.hi, "dragged past the end: the top");
    m.drag(x - 40, y + h / 2);
    assert_eq!(m.design().shape.legs, legs.lo);
    // a click elsewhere lets go of it
    click(&mut m, "Random");
    let before = m.design().shape.legs;
    m.drag(x + w / 2, y);
    assert_eq!(m.design().shape.legs, before);
}

#[test]
fn a_swatch_sets_its_area_and_none_removes_the_collar() {
    let mut m = Maker::new(1);
    to_shape(&mut m);
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Coat);
    click(&mut m, "swatch:Saddle:4");
    let saddle = m.design().coat.iter().find(|(f, _)| *f == fam::BLACK).map(|c| c.1);
    assert_eq!(saddle, Some(pixelpet::rig::breeds::SWATCHES[4]));
    click(&mut m, "swatch:Collar:2");
    assert_eq!(m.design().collar, Some(pixelpet::rig::breeds::COLLARS[2]));
    click(&mut m, "swatch:Collar:none");
    assert_eq!(m.design().collar, None);
}

fn to_name(m: &mut Maker) {
    while m.step() != Step::Name {
        click(m, "Next >");
    }
}

#[test]
fn name_takes_only_printable_ascii_up_to_24() {
    let mut m = Maker::new(1);
    to_name(&mut m);
    for _ in 0..30 {
        m.key(MakerKey::Backspace);
    }
    assert_eq!(m.name(), "");
    m.key(MakerKey::Backspace);
    for c in ['B', 'é', '🐶', 'e', 'a', 'n'] {
        m.key(MakerKey::Char(c));
    }
    assert_eq!(m.name(), "Bean");
    for _ in 0..30 {
        m.key(MakerKey::Char('x'));
    }
    assert_eq!(m.name().len(), 24);
}

#[test]
fn done_returns_the_pet() {
    let mut m = Maker::new(1);
    click(&mut m, "Dog");
    click(&mut m, "Corgi");
    to_name(&mut m);
    for _ in 0..30 {
        m.key(MakerKey::Backspace);
    }
    for c in "Bean".chars() {
        m.key(MakerKey::Char(c));
    }
    match click(&mut m, "Done") {
        Outcome::Done(r) => {
            assert_eq!((r.id.as_str(), r.name.as_str(), r.breed.as_str()), ("", "Bean", "Corgi"));
            assert_eq!(r.design, (breeds(Pet::Dog)[5].design)());
        }
        o => panic!("{o:?}"),
    }
    // an empty name is the breed's name; Enter is Done too
    let mut m = Maker::new(1);
    to_name(&mut m);
    for _ in 0..30 {
        m.key(MakerKey::Backspace);
    }
    match m.key(MakerKey::Enter) {
        Outcome::Done(r) => assert_eq!(r.name, "Alex the GSD"),
        o => panic!("{o:?}"),
    }
}

#[test]
fn editing_alex_never_touches_alex() {
    let alex = &builtins()[0];
    let mut m = Maker::edit(alex, 1);
    assert_eq!(m.step(), Step::Breed);
    click(&mut m, "Husky");
    to_name(&mut m);
    match click(&mut m, "Done") {
        Outcome::Done(r) => assert_eq!(r.id, "", "a new pet, not Alex"),
        o => panic!("{o:?}"),
    }
}

#[test]
fn delete_is_offered_only_for_made_pets_and_asks_first() {
    let mut m = Maker::edit(&builtins()[0], 1);
    to_name(&mut m);
    assert!(m.target("Delete").is_none(), "Alex can't be deleted");
    let mine = PetRecord { id: "p7".into(), name: "Bean".into(), breed: "Corgi".into(), design: (breeds(Pet::Dog)[5].design)(), extra: vec![] };
    let mut m = Maker::edit(&mine, 1);
    to_name(&mut m);
    assert!(matches!(click(&mut m, "Delete"), Outcome::Redraw));
    assert!(m.target("Yes").is_some() && m.target("No").is_some());
    click(&mut m, "No");
    assert!(m.target("Yes").is_none());
    click(&mut m, "Delete");
    match click(&mut m, "Yes") {
        Outcome::Delete(id) => assert_eq!(id, "p7"),
        o => panic!("{o:?}"),
    }
    // editing keeps the id
    let mut m = Maker::edit(&mine, 1);
    to_name(&mut m);
    match click(&mut m, "Done") {
        Outcome::Done(r) => assert_eq!(r.id, "p7"),
        o => panic!("{o:?}"),
    }
}

#[test]
fn close_and_esc_close() {
    let mut m = Maker::new(1);
    assert!(matches!(click(&mut m, "x"), Outcome::Close));
    assert!(matches!(m.key(MakerKey::Esc), Outcome::Close));
}

#[test]
fn random_is_repeatable_with_a_seed() {
    let roll = |seed| {
        let mut m = Maker::new(seed);
        click(&mut m, "Random");
        (m.design().clone(), m.name().to_string())
    };
    assert_eq!(roll(5), roll(5));
    assert_ne!(roll(5), roll(6));
    let (d, _) = roll(5);
    for s in sliders(d.pet) {
        let v = d.shape.get(s.field).unwrap();
        assert!(v >= s.lo && v <= s.hi, "{} = {v}", s.field);
    }
}

#[test]
fn the_preview_acts_out_the_step() {
    use pixelpet::rig::Anim;
    let mut m = Maker::new(1);
    let mut seen = vec![m.preview_anim()];
    while m.step() != Step::Name {
        click(&mut m, "Next >");
        seen.push(m.preview_anim());
    }
    assert_eq!(seen, [Anim::Stand, Anim::Stand, Anim::Walk, Anim::Sit, Anim::Tilt]);
}

#[test]
fn draw_is_the_window_size() {
    let m = Maker::new(1);
    let img = m.draw(3, &Rgba::new(1, 1), (0, 0));
    assert_eq!((img.w, img.h), (maker::W * 3, maker::H * 3));
    assert_eq!(&img.px[0..4], &[0x24, 0x12, 0x0A, 255], "outline in the corner");
}

#[test]
fn font_covers_every_maker_string() {
    let mut words: Vec<String> = maker::all_copy().into_iter().map(String::from).collect();
    for pet in [Pet::Dog, Pet::Rabbit] {
        words.extend(breeds(pet).iter().map(|b| b.name.to_string()));
        words.extend(sliders(pet).iter().map(|s| s.label.to_string()));
    }
    for w in &words {
        for c in w.chars() {
            assert!(font::glyph(c).is_some(), "{w:?} needs {c:?}");
        }
    }
}
