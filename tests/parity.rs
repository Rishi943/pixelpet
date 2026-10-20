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
