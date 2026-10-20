//! The anims added for play (spec §5 "New anims"). The look-sheet anims are
//! pinned by `tests/parity.rs`; these are written in Rust and checked here by
//! what each one must do, and by eye against the photos.

use pixelpet::rig::{self, dog, Anim, Pet, Skel, Toy};

const DOG_NEW: [Anim; 7] = [Anim::Run, Anim::Carry, Anim::Eat, Anim::Wag, Anim::Lean, Anim::Tug, Anim::Stretch];

fn look() -> &'static rig::Look {
    rig::look("classic")
}

fn dog_at(a: Anim, t: f64) -> dog::Skel {
    match rig::pose(Pet::Dog, a, look(), t, None) {
        Skel::Dog(s) => s,
        _ => unreachable!(),
    }
}

fn times(period: f64, n: usize) -> impl Iterator<Item = f64> {
    (0..n).map(move |i| period * i as f64 / n as f64)
}

fn skel_has_nan(s: &dog::Skel) -> bool {
    let mut v = vec![s.hip, s.sh, s.head, s.neck_a, s.neck_b];
    v.extend(s.tail);
    v.extend(s.legs.nf);
    v.extend(s.legs.ff);
    v.extend(s.legs.nh);
    v.extend(s.legs.fh);
    v.iter().any(|p| p[0].is_nan() || p[1].is_nan())
        || [s.head_rot, s.tilt, s.mouth, s.tongue, s.eyes, s.ear_back, s.breath].iter().any(|x| x.is_nan())
}

/// At least `min` opaque pixels, and none on the left, right or top frame
/// edge (a pose that pokes out of the frame gets clipped there). The bottom
/// row is below the ground line; the approved walk and hop touch it too.
fn renders_inside(f: &rig::Frame, min: usize) -> Result<(), String> {
    let mut n = 0;
    for y in 0..f.h {
        for x in 0..f.w {
            if f.rgba[(y * f.w + x) * 4 + 3] > 0 {
                n += 1;
                if x == 0 || y == 0 || x == f.w - 1 {
                    return Err(format!("opaque pixel on the frame edge at ({x},{y})"));
                }
            }
        }
    }
    if n < min {
        return Err(format!("only {n} opaque px"));
    }
    Ok(())
}

#[test]
fn new_dog_anims_render() {
    for a in DOG_NEW {
        for i in 0..=20 {
            let t = i as f64 * 0.1;
            let s = rig::pose(Pet::Dog, a, look(), t, None);
            if let Skel::Dog(d) = &s {
                assert!(!skel_has_nan(d), "{a:?} t={t}: NaN in the skeleton");
            }
            let f = rig::render(&s, look(), false);
            if let Err(e) = renders_inside(&f, 300) {
                panic!("{a:?} t={t}: {e}");
            }
        }
    }
}

#[test]
fn run_is_faster_than_walk() {
    let walk = rig::speed(Pet::Dog, look(), Anim::Walk);
    assert!(rig::speed(Pet::Dog, look(), Anim::Run) >= 2.0 * walk);
    assert!(rig::speed(Pet::Dog, look(), Anim::Carry) >= 1.5 * walk);
    assert!(rig::period(Pet::Dog, Anim::Run) < rig::period(Pet::Dog, Anim::Walk));
}

#[test]
fn eat_puts_the_head_down() {
    for t in times(rig::period(Pet::Dog, Anim::Eat), 12) {
        let (e, s) = (dog_at(Anim::Eat, t), dog_at(Anim::Stand, t));
        assert!(e.head[1] <= s.head[1] - 0.25, "t={t}: head {} vs stand {}", e.head[1], s.head[1]);
        assert!(e.head[1] < e.sh[1], "t={t}: head above the shoulder");
    }
}

#[test]
fn tug_pulls_back_and_shakes() {
    let p = rig::period(Pet::Dog, Anim::Tug);
    let mut signs = vec![];
    for t in times(p, 36) {
        let (g, s) = (dog_at(Anim::Tug, t), dog_at(Anim::Stand, t));
        assert!(g.sh[0] - g.hip[0] > s.sh[0] - s.hip[0], "t={t}: hips not back");
        signs.push(g.head_rot - dog_at(Anim::Tug, 0.0).head_rot + 1e-9 >= 0.0);
    }
    let changes = signs.windows(2).filter(|w| w[0] != w[1]).count();
    assert!(changes >= 3, "head shook {changes} times");
}

#[test]
fn lean_leans() {
    for t in times(rig::period(Pet::Dog, Anim::Lean), 12) {
        let (l, s) = (dog_at(Anim::Lean, t), dog_at(Anim::Stand, t));
        assert!(((l.sh[0] - l.hip[0]) - (s.sh[0] - s.hip[0])).abs() >= 0.1, "t={t}");
        assert_eq!(l.eyes, 0.0, "t={t}: eyes open");
    }
}

#[test]
fn wag_wags() {
    let xs: Vec<f64> = times(rig::period(Pet::Dog, Anim::Wag), 48).map(|t| dog_at(Anim::Wag, t).tail[4][0]).collect();
    let span = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
    assert!(span >= 0.3, "tail tip x spans {span}");
}

#[test]
fn carry_holds_the_toy() {
    for t in times(rig::period(Pet::Dog, Anim::Carry), 6) {
        match rig::pose(Pet::Dog, Anim::Carry, look(), t, Some(Toy::Ball)) {
            Skel::Dog(s) => assert_eq!(s.toy, Some(Toy::Ball)),
            _ => unreachable!(),
        }
    }
    assert_eq!(dog_at(Anim::Run, 0.1).toy, None);
}

#[test]
fn stretch_is_one_shot() {
    assert!(rig::is_one_shot(Anim::Stretch));
    assert!(!rig::is_one_shot(Anim::Walk));
    assert_eq!(rig::period(Pet::Dog, Anim::Stretch), 1.5);
    let s = dog_at(Anim::Stretch, 0.75);
    assert!(s.sh[1] < s.hip[1], "play bow: chest down, hips up");
}

#[test]
fn mouth_is_at_the_head() {
    let look = look();
    let s = rig::pose(Pet::Dog, Anim::Stand, look, 0.0, None);
    let d = dog_at(Anim::Stand, 0.0);
    let fs = rig::core::frame_size(look);
    let head = (fs.ox as f64 + d.head[0] * look.r, fs.gy as f64 - d.head[1] * look.r);
    let (mx, my) = rig::mouth(Pet::Dog, &s, look);
    let dist = ((mx - head.0).powi(2) + (my - head.1).powi(2)).sqrt();
    assert!(dist <= 0.6 * look.r, "mouth {dist:.1} px from the head");
    assert!(mx > head.0, "mouth on the facing side");
}

#[test]
fn eyes_close_on_request() {
    let s = rig::with_eyes_closed(rig::pose(Pet::Dog, Anim::Stand, look(), 0.0, None));
    match s {
        Skel::Dog(d) => assert_eq!(d.eyes, 0.0),
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------- rabbit

const BUN_NEW: [Anim; 5] = [Anim::Eat, Anim::Flop, Anim::Nudge, Anim::Tug, Anim::Stretch];

fn bun_at(a: Anim, t: f64) -> rig::rabbit::Skel {
    match rig::pose(Pet::Rabbit, a, look(), t, None) {
        Skel::Rabbit(s) => s,
        _ => unreachable!(),
    }
}

#[test]
fn new_rabbit_anims_render() {
    for a in BUN_NEW {
        for i in 0..=20 {
            let t = i as f64 * 0.1;
            let f = rig::render(&rig::pose(Pet::Rabbit, a, look(), t, None), look(), false);
            if let Err(e) = renders_inside(&f, 300) {
                panic!("{a:?} t={t}: {e}");
            }
        }
    }
}

#[test]
fn flop_lies_on_its_side() {
    let stand = bun_at(Anim::Stand, 0.0);
    for t in [1.0, 2.0, 3.9] {
        let s = bun_at(Anim::Flop, t);
        assert!(s.flat >= 0.9, "t={t}: flat {}", s.flat);
        assert_eq!(s.eyes, 0.0, "t={t}");
        let top = [s.hip, s.sh, s.head, s.tail, s.nf[0], s.nf[1], s.ff[0], s.ff[1], s.nh[0], s.nh[1], s.fh[0], s.fh[1]]
            .iter()
            .map(|p| p[1])
            .fold(f64::MIN, f64::max);
        assert!(top < stand.head[1] - 0.3, "t={t}: highest joint {top}");
    }
}

#[test]
fn nudge_keeps_the_nose_down() {
    let stand = bun_at(Anim::Stand, 0.0);
    for t in times(rig::period(Pet::Rabbit, Anim::Nudge), 14) {
        assert!(bun_at(Anim::Nudge, t).head[1] <= stand.head[1] - 0.2, "t={t}");
    }
}

#[test]
fn bun_tug_hops_back() {
    let p = rig::period(Pet::Rabbit, Anim::Tug);
    let xs: Vec<f64> = times(p, 32).map(|t| bun_at(Anim::Tug, t).hip[0]).collect();
    let back = xs.iter().cloned().fold(f64::MAX, f64::min);
    assert!(back < xs[0] - 0.05, "hips go back: {back} vs {}", xs[0]);
    assert!((xs[0] - bun_at(Anim::Tug, p - 1e-6).hip[0]).abs() < 0.02, "and return");
    let sw: Vec<f64> = times(p, 32).map(|t| bun_at(Anim::Tug, t).ear_sw).collect();
    assert!(sw.iter().any(|&x| x > 0.05) && sw.iter().any(|&x| x < -0.05), "ears swing");
}

#[test]
fn bun_eat_chews() {
    let p = rig::period(Pet::Rabbit, Anim::Eat);
    let stand = bun_at(Anim::Stand, 0.0);
    let noses: Vec<f64> = times(p, 40).map(|t| bun_at(Anim::Eat, t).nose).collect();
    let rises = noses.windows(2).filter(|w| w[1] > w[0]).count();
    assert!(rises >= 2, "nose twitched {rises} times");
    for t in times(p, 10) {
        assert!(bun_at(Anim::Eat, t).head[1] < stand.head[1], "t={t}");
    }
}

#[test]
fn rabbit_fallbacks_do_not_panic() {
    for a in Anim::ALL {
        let f = rig::render(&rig::pose(Pet::Rabbit, a, look(), 0.3, None), look(), false);
        assert!(f.rgba.chunks(4).any(|p| p[3] > 0), "{a:?}");
    }
}
