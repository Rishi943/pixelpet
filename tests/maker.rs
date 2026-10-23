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

// ---- the Photos step

use pixelpet::photo::measure::Point;
use pixelpet::photo::Photo;

/// A photo whose pixel (x, y) is the colour (x, y, 0): a pixel's colour says
/// where it is. Sides must be under 256.
fn photo(w: u32, h: u32) -> Photo {
    let rgb = (0..w * h).flat_map(|i| [(i % w) as u8, (i / w) as u8, 0]).collect();
    Photo { w, h, rgb }
}

/// A maker on the Photos step (dog) with one photo of `w` x `h` and its pose
/// answered.
fn on_photos(w: u32, h: u32) -> Maker {
    let mut m = Maker::new(1);
    click(&mut m, "Dog");
    click(&mut m, "From photos...");
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Photos);
    m.add_photo(photo(w, h));
    click(&mut m, "Standing");
    m
}

fn rect_px(m: &Maker, z: usize) -> (usize, usize, usize, usize) {
    let (x, y, w, h) = m.photo_rect().expect("a photo is shown");
    let z = z as f32;
    ((x * z).round() as usize, (y * z).round() as usize, ((x + w) * z).round() as usize, ((y + h) * z).round() as usize)
}

#[test]
fn from_photos_adds_a_sixth_step() {
    let mut m = Maker::new(1);
    click(&mut m, "Dog");
    assert!(!m.wants_photos());
    click(&mut m, "From photos...");
    assert!(m.wants_photos());
    assert_eq!(m.design(), &Design::alex(), "the base is the first breed");
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Photos);
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Shape);
    click(&mut m, "< Back");
    assert_eq!(m.step(), Step::Photos);
    click(&mut m, "< Back");
    assert_eq!(m.step(), Step::Breed);
    click(&mut m, "dot:2");
    assert_eq!(m.step(), Step::Photos, "a reached step can be jumped to");
    let mut seen = vec![];
    while m.step() != Step::Name {
        seen.push(m.step());
        click(&mut m, "Next >");
    }
    assert_eq!(seen, [Step::Photos, Step::Shape, Step::Coat]);
    assert!(m.target("dot:5").is_some() && m.target("dot:6").is_none(), "six dots on this path");
    // a plain breed takes the step away again
    click(&mut m, "dot:1");
    click(&mut m, "Husky");
    assert!(!m.wants_photos());
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Shape);
    assert!(m.target("dot:5").is_none(), "five dots again");
}

#[test]
fn editing_a_saved_pet_never_shows_photos() {
    let mut m = Maker::edit(&builtins()[0], 1);
    assert!(m.target("From photos...").is_none());
    assert!(m.target("Husky").is_some());
    let mut seen = vec![];
    while m.step() != Step::Name {
        seen.push(m.step());
        click(&mut m, "Next >");
    }
    assert!(!seen.contains(&Step::Photos));
    m.add_photo(photo(10, 10));
    assert_eq!(m.photo_count(), 0, "photos aren't taken when they aren't wanted");
}

#[test]
fn open_photo_asks_the_app_for_the_dialog() {
    let mut m = on_photos(30, 20);
    assert!(matches!(click(&mut m, "Open photo..."), Outcome::PickPhotos));
}

#[test]
fn photo_clicks_map_to_photo_pixels() {
    // photos of any shape, at every Size zoom: the click lands on the photo
    // pixel that is drawn under the cursor, and the marker is drawn there
    for (w, h) in [(30, 20), (20, 30), (60, 10), (8, 8), (31, 17)] {
        for z in 1..=4usize {
            let mut m = on_photos(w, h);
            let first = pixelpet::photo::measure::points(Pet::Dog)[0];
            let img = m.draw(z as u32, &Rgba::new(1, 1), (0, 0));
            let (x0, y0, x1, y1) = rect_px(&m, z);
            let mut tried = 0;
            for sy in (y0 + 1..y1 - 1).step_by((y1 - y0) / 3) {
                for sx in (x0 + 1..x1 - 1).step_by((x1 - x0) / 4) {
                    let o = (sy * img.w + sx) * 4;
                    let (px, py) = (img.px[o] as f32, img.px[o + 1] as f32);
                    m.click_at((sx as f32 + 0.5) / z as f32, (sy as f32 + 0.5) / z as f32);
                    let marks = m.marks();
                    let (pt, (mx, my)) = *marks.last().unwrap_or_else(|| panic!("{w}x{h} z{z}: no marker for {sx},{sy}"));
                    assert_eq!(pt, first);
                    assert_eq!((mx.floor(), my.floor()), (px, py), "{w}x{h} at zoom {z}, screen {sx},{sy}");
                    // the marker is drawn on that spot (its dot is a different colour)
                    let drawn = m.draw(z as u32, &Rgba::new(1, 1), (0, 0));
                    assert_eq!(&drawn.px[(sy * drawn.w + sx) * 4..][..3], &[0xFF, 0xD0, 0x30], "{w}x{h} z{z}: marker at {sx},{sy}");
                    m.right_click(0.0, 0.0);
                    tried += 1;
                }
            }
            assert!(tried >= 4, "{w}x{h} z{z}: only {tried} spots tried");
        }
    }
}

#[test]
fn clicks_off_the_photo_or_before_the_pose_drop_no_marker() {
    let mut m = Maker::new(1);
    click(&mut m, "Dog");
    click(&mut m, "From photos...");
    click(&mut m, "Next >");
    m.add_photo(photo(30, 20));
    let (x, y, w, h) = m.photo_rect().unwrap();
    m.click_at(x + w / 2.0, y + h / 2.0);
    assert!(m.marks().is_empty(), "no points before the pose is answered");
    click(&mut m, "Sitting");
    m.click_at(x - 0.5, y + h / 2.0);
    m.click_at(x + w + 0.5, y + h / 2.0);
    m.click_at(x + w / 2.0, y - 0.5);
    assert!(m.marks().is_empty(), "just off the photo");
    m.click_at(x + 0.01, y + 0.01);
    assert_eq!(m.marks().len(), 1);
}

#[test]
fn markers_skip_undo_and_drag() {
    let mut m = on_photos(30, 20);
    let pts = pixelpet::photo::measure::points(Pet::Dog);
    assert_eq!(m.next_point(), Some(pts[0]));
    let (x, y, w, h) = m.photo_rect().unwrap();
    m.click_at(x + w * 0.9, y + h * 0.5);
    assert_eq!(m.next_point(), Some(pts[1]));
    click(&mut m, "Skip");
    assert_eq!(m.next_point(), Some(pts[2]), "a skipped point is passed over");
    m.click_at(x + w * 0.5, y + h * 0.2);
    let marks = m.marks();
    assert_eq!(marks.iter().map(|p| p.0).collect::<Vec<_>>(), [pts[0], pts[2]]);
    // right click takes back the last point, then the skip
    assert!(matches!(m.right_click(0.0, 0.0), Outcome::Redraw));
    assert_eq!(m.next_point(), Some(pts[2]));
    m.right_click(0.0, 0.0);
    assert_eq!(m.next_point(), Some(pts[1]));
    assert_eq!(m.marks().len(), 1);
    // a click on a marker grabs it instead of dropping a new one
    m.click_at(x + w * 0.9 + 1.0, y + h * 0.5);
    assert_eq!(m.marks().len(), 1);
    m.drag_at(x + w * 0.4, y + h * 0.3);
    let (mx, my) = m.marks()[0].1;
    let k = w / 30.0;
    assert!((mx - w * 0.4 / k).abs() < 1e-3 && (my - h * 0.3 / k).abs() < 1e-3, "{mx},{my}");
    m.drag_at(x - 50.0, y + h * 2.0);
    assert_eq!(m.marks()[0].1, (0.0, 20.0), "dragged off the photo: its edge");
    m.release();
    m.drag_at(x + w * 0.5, y + h * 0.5);
    assert_eq!(m.marks()[0].1, (0.0, 20.0), "let go");
    // every point answered: no more asked, and Skip goes
    for i in 0..pts.len() {
        m.click_at(x + w * (0.1 + 0.08 * i as f32), y + h * 0.9);
    }
    assert!(m.next_point().is_none());
    assert!(m.target("Skip").is_none());
    let n = m.marks().len();
    m.click_at(x + w * 0.99, y + h * 0.01);
    assert_eq!(m.marks().len(), n);
}

#[test]
fn tabs_switch_and_x_removes() {
    let mut m = on_photos(30, 20);
    m.add_photo(photo(20, 30));
    m.add_photo(photo(10, 10));
    assert_eq!((m.photo_count(), m.photo_shown()), (3, 2), "the newest is shown");
    assert!(m.target("4").is_none());
    click(&mut m, "1");
    assert_eq!(m.photo_shown(), 0);
    let (x, y, w, h) = m.photo_rect().unwrap();
    m.click_at(x + w / 2.0, y + h / 2.0);
    assert_eq!(m.marks().len(), 1);
    click(&mut m, "2");
    m.click_at(x + w / 2.0, y + h / 2.0);
    assert!(m.marks().is_empty(), "this one's pose isn't answered yet");
    click(&mut m, "Other");
    m.release();
    click(&mut m, "3");
    assert!(m.marks().is_empty());
    click(&mut m, "2");
    assert!(m.marks().is_empty(), "each photo has its own points");
    click(&mut m, "remove");
    assert_eq!((m.photo_count(), m.photo_shown()), (2, 1));
    click(&mut m, "1");
    assert_eq!(m.marks().len(), 1, "the other photo kept its point");
    click(&mut m, "remove");
    click(&mut m, "remove");
    assert_eq!(m.photo_count(), 0);
    assert!(m.photo_rect().is_none() && m.target("remove").is_none());
    assert!(matches!(m.right_click(0.0, 0.0), Outcome::Nothing));
}

#[test]
fn a_sixth_photo_is_refused_and_the_message_goes_away() {
    let mut m = on_photos(10, 10);
    for _ in 1..5 {
        m.add_photo(photo(10, 10));
    }
    assert_eq!(m.photo_count(), 5);
    assert_eq!(m.error(), None);
    m.add_photo(photo(10, 10));
    assert_eq!(m.photo_count(), 5);
    assert_eq!(m.error(), Some("Up to 5 photos"));
    assert!(matches!(m.tick(100.0), Outcome::Nothing), "the clock starts at the first tick");
    assert!(matches!(m.tick(103.0), Outcome::Nothing));
    assert_eq!(m.error(), Some("Up to 5 photos"));
    assert!(matches!(m.tick(104.5), Outcome::Redraw));
    assert_eq!(m.error(), None);
    assert!(matches!(m.tick(110.0), Outcome::Nothing));
    // the app's messages: any text, ASCII only
    m.photo_error("Not a photo: caf\u{e9}");
    assert_eq!(m.error(), Some("Not a photo: caf?"));
    let img = m.draw(2, &Rgba::new(1, 1), (0, 0));
    assert_eq!(img.w, maker::W * 2);
}

#[test]
fn next_with_no_points_keeps_the_base_design() {
    let mut m = on_photos(30, 20);
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Shape);
    assert_eq!(m.design(), &Design::alex());
    click(&mut m, "Next >");
    assert!(m.target("swatch:Saddle:photo").is_none(), "no photo colour without points");
    assert!(m.target("swatch:Saddle:0").is_some());
}

#[test]
fn points_start_shape_and_coat_and_back_keeps_them() {
    let mut m = on_photos(30, 20);
    let (x, y, w, h) = m.photo_rect().unwrap();
    m.click_at(x + w * 0.5, y + h * 0.5);
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Shape);
    // hand edit, then Back and forward with nothing changed: the edit stays
    let (sx, sy, _, sh) = m.target("slider:Legs").unwrap();
    m.click(sx + 1, sy + sh / 2);
    m.release();
    let edited = m.design().clone();
    click(&mut m, "< Back");
    assert_eq!(m.step(), Step::Photos);
    assert_eq!(m.marks().len(), 1, "the points are kept");
    click(&mut m, "Next >");
    assert_eq!(m.design(), &edited);
    click(&mut m, "Next >");
    assert_eq!(m.step(), Step::Coat);
    // a colour the photos gave is a swatch, the first in its row
    for (label, f) in [("Coat", fam::TAN), ("Saddle", fam::BLACK), ("Legs", fam::CREAM), ("Mask", fam::MASK)] {
        let given = m.design().coat.iter().any(|c| c.0 == f);
        let photo_at = m.target(&format!("swatch:{label}:photo"));
        assert_eq!(photo_at.is_some(), given, "{label}");
        if let Some((px, _, _, _)) = photo_at {
            assert!(px < m.target(&format!("swatch:{label}:0")).unwrap().0);
        }
    }
    // jumping ahead with a dot works the same as Next
    let mut m = on_photos(30, 20);
    let (x, y, w, h) = m.photo_rect().unwrap();
    m.click_at(x + w * 0.5, y + h * 0.5);
    click(&mut m, "Next >");
    click(&mut m, "dot:2");
    assert_eq!(m.step(), Step::Photos);
    m.click_at(x + w * 0.4, y + h * 0.4);
    click(&mut m, "dot:3");
    assert_eq!(m.step(), Step::Shape);
}

#[test]
fn the_photos_step_draws_inside_the_window_with_the_pet_marked() {
    for z in 1..=3u32 {
        let mut m = on_photos(30, 20);
        let blank = Rgba::new(1, 1);
        let img = m.draw(z, &blank, (0, 0));
        assert_eq!((img.w, img.h), (maker::W * z as usize, maker::H * z as usize));
        // the part being asked for is marked on the preview's box
        let aim = [0xE0, 0x30, 0x30];
        let count = |img: &Rgba| img.px.chunks(4).filter(|p| p[..3] == aim).count();
        assert!(count(&img) > 0, "the preview shows where the nose is");
        let (x, y, w, h) = m.photo_rect().unwrap();
        m.click_at(x + w / 2.0, y + h / 2.0);
        assert!(count(&m.draw(z, &blank, (0, 0))) > 0, "and the next part");
    }
}

#[test]
fn photos_copy_is_known_to_the_font_check() {
    let copy = maker::all_copy();
    for s in ["From photos...", "Open photo...", "Skip", "Standing", "Sitting", "Other", "Up to 5 photos", "Photos", "Click the nose", "Click a front foot"] {
        assert!(copy.contains(&s), "{s:?} missing from all_copy");
    }
    let _ = Point::Nose;
}

