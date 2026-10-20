//! Props for play: food (eaten in three bites), hearts, crumbs.

use pixelpet::rig::props;
use pixelpet::rig::{Food, Frame, Pet};

const FOODS: [Food; 6] = [Food::Chicken, Food::Bone, Food::Kibble, Food::Carrot, Food::Coriander, Food::Banana];

fn opaque(f: &Frame) -> usize {
    f.rgba.chunks(4).filter(|p| p[3] > 0).count()
}

#[test]
fn foods_have_three_bites() {
    for f in FOODS {
        let whole = props::food(f, 0);
        assert!(whole.w <= 16 && whole.h <= 12, "{f:?} is {}x{}", whole.w, whole.h);
        assert!(opaque(&whole) >= 30, "{f:?}: {} px", opaque(&whole));
        let (one, two, gone) = (props::food(f, 1), props::food(f, 2), props::food(f, 3));
        assert!(opaque(&one) < opaque(&whole) && opaque(&two) < opaque(&one), "{f:?} bites don't shrink it");
        assert!(opaque(&two) > 0, "{f:?} gone after two bites");
        assert_eq!(opaque(&gone), 0, "{f:?} left after three bites");
        // the ground point is the bottom centre
        assert_eq!((whole.ox, whole.gy), ((whole.w / 2) as i32, whole.h as i32 - 1));
    }
}

#[test]
fn food_choices_per_pet() {
    assert_eq!(Food::for_pet(Pet::Dog), [Food::Chicken, Food::Bone, Food::Kibble]);
    assert_eq!(Food::for_pet(Pet::Rabbit), [Food::Carrot, Food::Coriander, Food::Banana]);
    let labels: Vec<&str> = FOODS.iter().map(|f| f.label()).collect();
    assert_eq!(labels, ["Chicken", "Bone", "Kibble", "Carrot", "Coriander", "Banana"]);
}

#[test]
fn crumbs_match_the_food() {
    for f in FOODS {
        let c = props::crumb(f);
        assert_eq!(c[3], 255);
        let whole = props::food(f, 0);
        assert!(whole.rgba.chunks(4).any(|p| p == c), "{f:?}: crumb colour not in the food");
    }
}

#[test]
fn heart_is_red_and_small() {
    let h = props::heart();
    assert!(h.w <= 9 && h.h <= 8, "{}x{}", h.w, h.h);
    assert!(opaque(&h) >= 20);
    let mut counts: std::collections::HashMap<[u8; 4], usize> = Default::default();
    for p in h.rgba.chunks(4).filter(|p| p[3] > 0) {
        *counts.entry([p[0], p[1], p[2], p[3]]).or_default() += 1;
    }
    let top = counts.into_iter().max_by_key(|(_, n)| *n).unwrap().0;
    assert!(top[0] > 150 && top[1] < 90, "most common colour {top:?}");
}
