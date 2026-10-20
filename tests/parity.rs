//! The Rust rig against frames rendered by the approved JS drafts.
mod common;

#[test]
fn dog_frames_match_js() {
    common::check_all(|c| c.pet == "dog");
}

#[test]
fn toys_match_js() {
    common::check_all(|c| c.pet == "toy");
}

#[test]
fn rabbit_frames_match_js() {
    common::check_all(|c| c.pet == "rabbit");
}

#[test]
fn rabbit_walk_moves_in_bursts() {
    use pixelpet::rig::{self, Anim, Pet};
    let look = rig::look("classic");
    assert_eq!(rig::speed(Pet::Rabbit, look, Anim::Walk), 0.72);
    for (p, want) in [(0.0, 0.0), (0.06, 0.0), (0.07, 1.0 / 0.54), (0.3, 1.0 / 0.54), (0.59, 1.0 / 0.54), (0.6, 0.0), (0.9, 0.0)] {
        assert_eq!(rig::move_profile(Pet::Rabbit, Anim::Walk, p), want, "phase {p}");
    }
    assert_eq!(rig::move_profile(Pet::Dog, Anim::Walk, 0.3), 1.0);
}
