//! Behaviour scenarios on a fake clock. Every test steps `update` at 30 Hz.

use pixelpet::brain::{fetch, Brain, Choice, Inputs, Mouse, Show};
use pixelpet::platform::Rect;
use pixelpet::rig::{self, Anim, Pet, Toy};
use pixelpet::settings::{FullscreenMode, Settings};

const DT: f64 = 1.0 / 30.0;
const SEED: u64 = 7;
const WORK: Rect = Rect { x: 0, y: 0, w: 1920, h: 1040 };

struct Sim {
    brain: Brain,
    s: Settings,
    now: f64,
    work: Rect,
    unit_px: f64,
    cursor: Option<(i32, i32)>,
    over_pet: bool,
    idle: Option<f64>,
    fullscreen: Option<bool>,
    over_bubble: bool,
    over_toy: bool,
    /// set for one step, as the app does on right-click or Esc during play
    cancel: bool,
    /// upward flick speed at the last release, px/s (set with the Up)
    flick_up: f64,
    can_warp: bool,
    /// every cursor move the brain asked for
    warps: Vec<(i32, i32)>,
    last: Show,
}

impl Sim {
    fn new(pet: Pet) -> Sim {
        Sim::with_work(pet, WORK, 66.0)
    }

    fn with_work(pet: Pet, work: Rect, unit_px: f64) -> Sim {
        let mut brain = Brain::new(pet, SEED, 0.0, work);
        let s = Settings::default();
        let inp = Inputs {
            now: 0.0,
            work,
            unit_px,
            cursor: None,
            over_pet: false,
            mouse: Mouse::None,
            idle: Some(0.0),
            fullscreen: Some(false),
            over_bubble: false,
            over_toy: false,
            cancel: false,
            flick_up: 0.0,
            can_warp: true,
        };
        let last = brain.update(&inp, &s);
        Sim {
            brain,
            s,
            now: 0.0,
            work,
            unit_px,
            cursor: None,
            over_pet: false,
            idle: Some(0.0),
            fullscreen: Some(false),
            over_bubble: false,
            over_toy: false,
            cancel: false,
            flick_up: 0.0,
            can_warp: true,
            warps: vec![],
            last,
        }
    }

    fn step_with(&mut self, dt: f64, mouse: Mouse) -> Show {
        self.now += dt;
        let inp = Inputs {
            now: self.now,
            work: self.work,
            unit_px: self.unit_px,
            cursor: self.cursor,
            over_pet: self.over_pet,
            mouse,
            idle: self.idle,
            fullscreen: self.fullscreen,
            over_bubble: self.over_bubble,
            over_toy: self.over_toy,
            cancel: self.cancel,
            flick_up: self.flick_up,
            can_warp: self.can_warp,
        };
        self.cancel = false;
        self.last = self.brain.update(&inp, &self.s);
        // the app moves the real cursor where the brain asks
        if let Some(w) = self.last.warp {
            self.warps.push(w);
            self.cursor = Some(w);
        }
        self.last
    }

    fn step(&mut self) -> Show {
        self.step_with(DT, Mouse::None)
    }

    fn run(&mut self, secs: f64, mut each: impl FnMut(&Sim, &Show)) {
        let n = (secs / DT).round() as usize;
        for _ in 0..n {
            let sh = self.step();
            each(self, &sh);
        }
    }

    fn bounds(&self) -> (f64, f64) {
        let m = 1.2 * self.unit_px;
        (self.work.x as f64 + m, (self.work.x + self.work.w) as f64 - m)
    }

    fn in_bounds(&self, x: f64) -> bool {
        let (lo, hi) = self.bounds();
        x >= lo - 1e-9 && x <= hi + 1e-9
    }
}

/// The sequence of anim starts seen over `secs` of wandering.
fn starts(sim: &mut Sim, secs: f64) -> Vec<Anim> {
    let mut seq = vec![sim.last.anim];
    let mut cur = (sim.last.anim, sim.last.anim_start);
    sim.run(secs, |_, sh| {
        if (sh.anim, sh.anim_start) != cur {
            cur = (sh.anim, sh.anim_start);
            seq.push(sh.anim);
        }
    });
    seq
}

#[test]
fn wander_never_repeats_except_walk() {
    let mut sim = Sim::new(Pet::Dog);
    let seq = starts(&mut sim, 2.0 * 3600.0);
    assert!(seq.len() > 500, "only {} starts", seq.len());
    for w in seq.windows(2) {
        assert!(w[0] != w[1] || w[0] == Anim::Walk, "repeat: {:?}", w);
        if w[0] == Anim::Sleep {
            assert_eq!(w[1], Anim::Stand, "sleep not followed by stand");
        }
    }
    // the wander script's anims (the play anims never come from it)
    for a in [Anim::Stand, Anim::Walk, Anim::Sit, Anim::Tilt, Anim::Hop, Anim::Sleep] {
        assert!(seq.contains(&a), "{a:?} never played");
    }
}

#[test]
fn wander_weights_roughly_hold() {
    let mut sim = Sim::new(Pet::Dog);
    let seq = starts(&mut sim, 10.0 * 3600.0);
    let walk = seq.iter().filter(|a| **a == Anim::Walk).count() as f64 / seq.len() as f64;
    println!("walk share {walk:.3} of {} starts", seq.len());
    assert!((0.30..=0.45).contains(&walk), "walk share {walk}");
}

#[test]
fn walking_turns_at_edges() {
    let mut sim = Sim::new(Pet::Dog);
    let mut flips = 0;
    let mut facing = sim.last.facing;
    sim.run(120.0, |sim, sh| {
        assert!(sim.in_bounds(sh.x), "x {} out of {:?}", sh.x, sim.bounds());
        if sh.facing != facing {
            flips += 1;
            facing = sh.facing;
        }
    });
    assert!(flips >= 1);
    // a long walk reaches an edge and turns there
    let mut sim = Sim::new(Pet::Dog);
    sim.brain.pin(Some(Anim::Walk));
    let mut facing = sim.step().facing;
    let mut edge_flips = 0;
    sim.run(300.0, |sim, sh| {
        assert!(sim.in_bounds(sh.x), "x {} out of {:?}", sh.x, sim.bounds());
        if sh.facing != facing {
            let (lo, hi) = sim.bounds();
            assert!(sh.x - lo < 2.0 || hi - sh.x < 2.0, "turned at {} away from an edge", sh.x);
            edge_flips += 1;
            facing = sh.facing;
        }
    });
    assert!(edge_flips >= 2, "{edge_flips} edge turns");
}

#[test]
fn rabbit_moves_only_in_bursts() {
    let mut sim = Sim::new(Pet::Rabbit);
    sim.brain.pin(Some(Anim::Walk));
    let mut prev = sim.step().x;
    let (mut still, mut moving) = (0, 0);
    sim.run(20.0, |sim, sh| {
        assert_eq!(sh.anim, Anim::Walk);
        let phase = ((sim.now - sh.anim_start) / rig::period(Pet::Rabbit, Anim::Walk)) % 1.0;
        if rig::move_profile(Pet::Rabbit, Anim::Walk, phase) == 0.0 {
            assert_eq!(sh.x, prev, "moved at phase {phase}");
            still += 1;
        } else {
            moving += 1;
        }
        prev = sh.x;
    });
    assert!(still > 50 && moving > 50, "still {still} moving {moving}");
}

#[test]
fn nap_after_three_idle_minutes() {
    let mut sim = Sim::new(Pet::Dog);
    sim.cursor = Some((1500, 1000));
    let mut t_idle = 0.0;
    while t_idle < 179.0 {
        t_idle += DT;
        sim.idle = Some(t_idle);
        sim.step();
    }
    assert_ne!(sim.last.anim, Anim::Sleep, "asleep before 180 s idle (seed picked sleep?)");
    while t_idle < 181.0 {
        t_idle += DT;
        sim.idle = Some(t_idle);
        sim.step();
    }
    assert_eq!(sim.last.anim, Anim::Sleep);
    assert!(sim.last.zzz);
    // stays asleep however long the idle lasts
    sim.run(60.0, |sim, sh| {
        let _ = sim;
        assert_eq!(sh.anim, Anim::Sleep);
    });
    // input: wakes and stretches at once
    sim.idle = Some(0.0);
    let sh = sim.step();
    assert_eq!(sh.anim, Anim::Stretch);
    assert!(!sh.zzz);
    // after 1.5 s it walks towards the cursor
    sim.run(1.6, |_, _| {});
    assert_eq!(sim.last.anim, Anim::Walk);
    let towards = if 1500.0 > sim.last.x { 1 } else { -1 };
    assert_eq!(sim.last.facing, towards);
    // and arrives, then wanders on
    let mut arrived = false;
    sim.run(60.0, |sim, sh| {
        if (sh.x - 1500.0).abs() <= 1.5 * sim.unit_px + 1.0 {
            arrived = true;
        }
    });
    assert!(arrived, "never reached the cursor, x {}", sim.last.x);
}

#[test]
fn wakes_on_any_input_between_idle_polls() {
    // the app polls idle every 2 s, so a single nudge can show up as
    // idle = 1.5 s: still input since the last poll, so it must wake
    let mut sim = Sim::new(Pet::Dog);
    sim.idle = Some(181.0);
    sim.run(1.0, |_, _| {});
    assert_eq!(sim.last.anim, Anim::Sleep);
    sim.idle = Some(183.0);
    sim.run(2.0, |_, _| {});
    assert_eq!(sim.last.anim, Anim::Sleep, "the same stale idle value keeps it asleep");
    sim.idle = Some(1.5);
    assert_eq!(sim.step().anim, Anim::Stretch, "idle went down: there was input");
}

#[test]
fn hover_looks_and_counts() {
    let mut sim = Sim::new(Pet::Dog);
    sim.run(5.0, |_, _| {});
    sim.over_pet = true;
    let x0 = sim.last.x;
    sim.run(1.0, |_, _| {});
    assert_eq!(sim.last.anim, Anim::Tilt);
    assert!((sim.last.hover_for - 1.0).abs() <= DT + 1e-9, "hover_for {}", sim.last.hover_for);
    assert_eq!(sim.last.x, x0, "moved while hovered");
    sim.over_pet = false;
    let sh = sim.step();
    assert_eq!(sh.hover_for, 0.0);
    assert!((sh.anim_start - sim.now).abs() < 1e-9 || sh.anim == Anim::Tilt, "resumed anim not restarted");
}

#[test]
fn click_is_not_drag() {
    let mut sim = Sim::new(Pet::Dog);
    sim.brain.pin(Some(Anim::Stand));
    sim.step();
    let x0 = sim.last.x;
    let (px, py) = (x0 as i32, WORK.bottom() - 20);
    sim.over_pet = true;
    sim.cursor = Some((px, py));
    sim.step_with(DT, Mouse::Down(px, py));
    sim.cursor = Some((px + 3, py));
    sim.step_with(DT, Mouse::Move(px + 3, py));
    sim.step_with(DT, Mouse::Up(px + 3, py));
    sim.run(1.0, |_, _| {});
    assert_eq!(sim.last.x, x0);
    assert_eq!(sim.last.lift, 0.0);
}

#[test]
fn drag_lifts_then_falls() {
    let mut sim = Sim::new(Pet::Dog);
    sim.step();
    let x0 = sim.last.x as i32;
    let ground = WORK.bottom();
    sim.over_pet = true;
    sim.cursor = Some((x0, ground - 20));
    sim.step_with(DT, Mouse::Down(x0, ground - 20));
    sim.cursor = Some((x0 + 100, ground - 200));
    let sh = sim.step_with(DT, Mouse::Move(x0 + 100, ground - 200));
    let want = 200.0 - 0.3 * sim.unit_px;
    assert!((sh.lift - want).abs() < 1e-6, "lift {} want {want}", sh.lift);
    assert_eq!(sh.x, (x0 + 100) as f64);
    assert_eq!(sh.anim, Anim::Stand);
    sim.over_pet = false;
    let t_up = sim.now;
    sim.step_with(DT, Mouse::Up(x0 + 100, ground - 200));
    let limit = (2.0 * want / 2400.0).sqrt() + 0.05;
    while sim.last.lift > 0.0 {
        assert!(sim.now - t_up <= limit, "still falling after {} s", sim.now - t_up);
        sim.step();
    }
    assert_eq!(sim.last.anim, Anim::Stand);
    assert_eq!(sim.last.x, (x0 + 100) as f64);
}

/// XWayland: the pointer goes over a native Wayland window mid-drag, the
/// cursor reads None and the release may never come. The pet must land.
#[test]
fn cursor_vanishing_mid_drag_drops_the_pet_on_the_ground() {
    let mut sim = Sim::new(Pet::Dog);
    sim.step();
    let x0 = sim.last.x as i32;
    let ground = WORK.bottom();
    sim.over_pet = true;
    sim.cursor = Some((x0, ground - 20));
    sim.step_with(DT, Mouse::Down(x0, ground - 20));
    sim.cursor = Some((x0 + 100, ground - 200));
    assert!(sim.step_with(DT, Mouse::Move(x0 + 100, ground - 200)).lift > 0.0);
    sim.over_pet = false;
    sim.cursor = None;
    sim.run(1.0, |_, _| {});
    assert_eq!(sim.last.lift, 0.0, "still held up with no cursor");
    assert_eq!(sim.last.x, (x0 + 100) as f64, "lands where it was let go");
    // and a later click elsewhere is not a drag of the pet
    sim.cursor = Some((10, 10));
    sim.step_with(DT, Mouse::Down(10, 10));
    let sh = sim.step_with(DT, Mouse::Move(300, 300));
    assert_eq!(sh.lift, 0.0);
}

#[test]
fn fullscreen_hide_and_stay() {
    let mut sim = Sim::new(Pet::Dog);
    sim.fullscreen = Some(true);
    let sh = sim.step();
    assert!(!sh.visible);
    sim.s.fullscreen = FullscreenMode::Stay;
    sim.s.fullscreen_opacity = 50;
    let sh = sim.step();
    assert!(sh.visible);
    assert_eq!(sh.opacity, 50);
    sim.fullscreen = Some(false);
    sim.s.opacity = 75;
    let sh = sim.step();
    assert!(sh.visible);
    assert_eq!(sh.opacity, 75);
    sim.fullscreen = None; // unknown: the pet simply stays
    sim.s.fullscreen = FullscreenMode::Hide;
    assert!(sim.step().visible);
}

#[test]
fn ground_is_work_area_bottom() {
    for work in [Rect { x: 0, y: 0, w: 1920, h: 1040 }, Rect { x: 0, y: 40, w: 1920, h: 1040 }] {
        for unit_px in [66.0, 99.0] {
            let mut sim = Sim::with_work(Pet::Dog, work, unit_px);
            sim.run(30.0, |sim, sh| {
                assert_eq!(sh.lift, 0.0);
                assert!(sim.in_bounds(sh.x));
            });
        }
    }
}

#[test]
fn clamps_on_work_area_change() {
    let mut sim = Sim::with_work(Pet::Dog, Rect { x: 0, y: 0, w: 2560, h: 1400 }, 66.0);
    sim.step();
    assert!(sim.last.x > 1280.0 - 1.2 * 66.0, "test needs the pet right of the new edge, x {}", sim.last.x);
    sim.work = Rect { x: 0, y: 0, w: 1280, h: 680 };
    let sh = sim.step();
    assert!(sim.in_bounds(sh.x), "x {}", sh.x);
}

#[test]
fn time_jump_is_one_step() {
    let mut sim = Sim::new(Pet::Dog);
    sim.brain.pin(Some(Anim::Walk));
    sim.run(2.0, |_, _| {});
    let x0 = sim.last.x;
    let sh = sim.step_with(600.0, Mouse::None);
    let max = rig::speed(Pet::Dog, rig::look("classic"), Anim::Walk) * 0.1 * sim.unit_px + 1.0;
    assert!((sh.x - x0).abs() <= max, "jumped {} px (max {max})", (sh.x - x0).abs());
}

#[test]
fn watching_the_screen_is_frequent_and_faces_it() {
    let mut sim = Sim::new(Pet::Dog);
    let centre = (sim.work.x + sim.work.w / 2) as f64;
    let mut cur = (sim.last.anim, sim.last.anim_start);
    let mut seq = vec![];
    sim.run(10.0 * 3600.0, |_, sh| {
        if (sh.anim, sh.anim_start) != cur {
            cur = (sh.anim, sh.anim_start);
            seq.push(sh.anim);
            if sh.anim == Anim::Watch {
                assert!((centre - sh.x) * sh.facing as f64 >= 0.0, "watching away from the screen at x={}", sh.x);
            }
        }
    });
    let n = |a: Anim| seq.iter().filter(|x| **x == a).count();
    assert!(n(Anim::Watch) > n(Anim::Sit), "watch {} vs sit {}", n(Anim::Watch), n(Anim::Sit));
}

// ---------------------------------------------------------------- M2: bubble, petting

/// Wander a little, then put the cursor on the pet.
fn on_the_pet(pet: Pet) -> Sim {
    let mut sim = Sim::new(pet);
    sim.run(2.0, |_, _| {});
    sim.cursor = Some((sim.last.x as i32, 1000));
    sim.over_pet = true;
    sim
}

#[test]
fn bubble_after_hover() {
    let mut sim = on_the_pet(Pet::Dog);
    sim.run(0.36, |_, _| {});
    assert!(!sim.last.bubble, "open after {} s", sim.last.hover_for);
    sim.run(0.1, |_, _| {});
    assert!(sim.last.bubble, "closed after {} s", sim.last.hover_for);
    // onto the bubble: stays
    sim.over_pet = false;
    sim.over_bubble = true;
    sim.run(2.0, |_, sh| assert!(sh.bubble, "closed while over the bubble"));
    // over neither: lingers 0.6 s
    sim.over_bubble = false;
    sim.run(0.5, |_, sh| assert!(sh.bubble, "closed too soon"));
    sim.run(0.2, |_, _| {});
    assert!(!sim.last.bubble, "still open");
}

#[test]
fn no_bubble_while_asleep_or_dragged() {
    let mut sim = Sim::new(Pet::Dog);
    sim.idle = Some(181.0);
    sim.step();
    sim.over_pet = true;
    sim.run(2.0, |_, sh| assert!(!sh.bubble, "bubble while asleep"));
    let mut sim = on_the_pet(Pet::Dog);
    let (x, y) = sim.cursor.unwrap();
    sim.step_with(DT, Mouse::Down(x, y));
    sim.step_with(DT, Mouse::Move(x, y - 40));
    for _ in 0..60 {
        let sh = sim.step_with(DT, Mouse::Move(x, y - 40));
        assert!(!sh.bubble, "bubble while dragged");
    }
}

/// Sweep the cursor across the pet: `n` strokes of `px` each way.
fn stroke(sim: &mut Sim, n: usize, px: i32, mut each: impl FnMut(&Show)) {
    let steps = 12;
    for k in 0..n {
        let dir = if k % 2 == 0 { 1 } else { -1 };
        for i in 0..steps {
            // spread px over the steps exactly (integer division would lose it)
            let d = (i + 1) as i32 * px / steps as i32 - i as i32 * px / steps as i32;
            let (x, y) = sim.cursor.unwrap();
            sim.cursor = Some((x + dir * d, y));
            let sh = sim.step();
            each(&sh);
        }
    }
}

fn hearts_seen(sim: &mut Sim, n: usize, px: i32, eyes: &mut Vec<bool>, anims: &mut Vec<Anim>) -> usize {
    let mut seen: Vec<f64> = vec![];
    stroke(sim, n, px, |sh| {
        for h in sh.props.hearts.iter().flatten() {
            if !seen.iter().any(|s| (s - h).abs() < 1e-9) {
                seen.push(*h);
            }
        }
        eyes.push(sh.eyes_closed);
        anims.push(sh.anim);
    });
    seen.len()
}

#[test]
fn petting_alex() {
    let mut sim = on_the_pet(Pet::Dog);
    sim.brain.choose(Choice::Pet, sim.now);
    assert!(sim.step().playing);
    let (mut eyes, mut anims) = (vec![], vec![]);
    // 7 sweeps of 60 px (~0.9 units) = 6 direction changes
    let hearts = hearts_seen(&mut sim, 7, 60, &mut eyes, &mut anims);
    assert_eq!(hearts, 6, "one heart per stroke");
    assert!(eyes.iter().skip(13).all(|e| *e), "eyes closed from the first stroke");
    assert!(anims.iter().skip(13).take(40).any(|a| *a == Anim::Wag), "wags while petted");
    assert_eq!(sim.last.anim, Anim::Lean, "leans in after 6 strokes");
    // leaving the pet ends it
    sim.over_pet = false;
    sim.run(0.6, |_, _| {});
    assert!(!sim.last.playing, "still petting after the cursor left");
    assert!(!sim.last.eyes_closed);
}

#[test]
fn petting_the_bun() {
    let mut sim = on_the_pet(Pet::Rabbit);
    sim.brain.choose(Choice::Pet, sim.now);
    sim.step();
    let (mut eyes, mut anims) = (vec![], vec![]);
    let hearts = hearts_seen(&mut sim, 7, 60, &mut eyes, &mut anims);
    assert_eq!(hearts, 6);
    assert!(eyes.iter().skip(13).all(|e| *e));
    assert!(anims.iter().skip(13).take(40).all(|a| *a == Anim::Stand), "the bun closes its eyes, standing");
    assert_eq!(sim.last.anim, Anim::Flop, "flops after 6 strokes");
}

#[test]
fn jitter_is_not_a_stroke() {
    let mut sim = on_the_pet(Pet::Dog);
    sim.brain.choose(Choice::Pet, sim.now);
    sim.step();
    let (mut eyes, mut anims) = (vec![], vec![]);
    // 10 px each way is under 0.6 units (40 px)
    assert_eq!(hearts_seen(&mut sim, 12, 10, &mut eyes, &mut anims), 0);
}

#[test]
fn nap_ends_play() {
    let mut sim = on_the_pet(Pet::Dog);
    sim.brain.choose(Choice::Pet, sim.now);
    sim.step();
    let (mut eyes, mut anims) = (vec![], vec![]);
    hearts_seen(&mut sim, 3, 60, &mut eyes, &mut anims);
    sim.idle = Some(181.0);
    let sh = sim.step();
    assert!(!sh.playing);
    assert_eq!(sh.anim, Anim::Sleep);
    assert!(sh.props.hearts.iter().all(|h| h.is_none()) && sh.props.toy.is_none() && sh.props.food.is_none());
    assert!(!sh.eyes_closed || sh.anim == Anim::Sleep);
}

#[test]
fn cancel_ends_petting() {
    let mut sim = on_the_pet(Pet::Dog);
    sim.brain.choose(Choice::Pet, sim.now);
    sim.step();
    sim.cancel = true;
    let sh = sim.step();
    assert!(!sh.playing);
    assert!(!sh.eyes_closed);
}

#[test]
fn fullscreen_hides_play_too() {
    let mut sim = on_the_pet(Pet::Dog);
    sim.brain.choose(Choice::Pet, sim.now);
    sim.step();
    sim.fullscreen = Some(true);
    sim.s.fullscreen = FullscreenMode::Hide;
    assert!(!sim.step().visible);
}

#[test]
fn choosing_closes_the_bubble() {
    let mut sim = on_the_pet(Pet::Dog);
    sim.run(0.5, |_, _| {});
    assert!(sim.last.bubble);
    sim.brain.choose(Choice::Pet, sim.now);
    assert!(!sim.step().bubble);
}

// ---------------------------------------------------------------- M2: fetch

/// Choose fetch with the cursor at (x, 700), grab the toy and flick it up
/// at `speed` px/s.
fn throw(sim: &mut Sim, toy: Toy, x: i32, speed: f64) {
    sim.over_pet = false;
    sim.cursor = Some((x, 700));
    sim.brain.choose(Choice::Fetch(toy), sim.now);
    let sh = sim.step();
    let t = sh.props.toy.expect("the toy appears");
    let (lo, hi) = sim.bounds();
    let want = (x as f64).clamp(lo, hi);
    assert!((t.x - want).abs() < 1.0 && t.lift > 0.0, "the toy appears under the cursor (kept on screen): {t:?}");
    sim.over_toy = true;
    sim.step_with(DT, Mouse::Down(x, 700));
    for k in 1..=3 {
        sim.cursor = Some((x, 700 - 100 * k));
        sim.step_with(DT, Mouse::Move(x, 700 - 100 * k));
    }
    sim.over_toy = false;
    sim.flick_up = speed;
    sim.step_with(DT, Mouse::Up(x, 400));
    sim.flick_up = 0.0;
}

/// Run until play settles (Ready) or `secs` pass; the anims seen, in order
/// of first appearance, and the deepest the pet went.
fn fetch_run(sim: &mut Sim, secs: f64) -> (Vec<Anim>, f64, Vec<bool>) {
    let (mut seq, mut deepest, mut mouth) = (vec![], 0.0f64, vec![]);
    let n = (secs / DT) as usize;
    for _ in 0..n {
        let sh = sim.step();
        if seq.last() != Some(&sh.anim) {
            seq.push(sh.anim);
        }
        deepest = deepest.max(sh.depth);
        mouth.push(sh.props.toy.is_some_and(|t| t.in_mouth));
        if sh.anim == Anim::Sit && sh.depth == 0.0 {
            break;
        }
    }
    (seq, deepest, mouth)
}

#[test]
fn fetch_round_trip_alex() {
    let mut sim = Sim::new(Pet::Dog);
    sim.run(1.0, |_, _| {});
    throw(&mut sim, Toy::Ball, 900, 3000.0);
    // in flight: the toy goes into the screen
    let d0 = sim.last.props.toy.unwrap().depth;
    sim.run(0.3, |_, _| {});
    let d1 = sim.last.props.toy.unwrap().depth;
    assert!(d1 > d0, "toy depth {d0} -> {d1}");
    let target = fetch::throw_depth(3000.0, sim.unit_px);
    let (seq, deepest, mouth) = fetch_run(&mut sim, 40.0);
    // side-on, or seen from behind/front when the run is mostly into the screen
    let at = |a: &[Anim]| seq.iter().position(|x| a.contains(x)).unwrap_or_else(|| panic!("{a:?} missing from {seq:?}"));
    let (run, carry) = (at(&[Anim::Run, Anim::RunAway]), at(&[Anim::Carry, Anim::CarryBack]));
    assert!(run < at(&[Anim::Eat]) && at(&[Anim::Eat]) < carry && carry < at(&[Anim::Sit]), "{seq:?}");
    assert!((deepest - target).abs() < 0.05, "ran to depth {deepest}, the toy was at {target}");
    assert!(mouth.iter().any(|m| *m), "never carried it");
    let sh = sim.last;
    assert!(sh.playing && sh.anim == Anim::Sit && sh.depth == 0.0);
    assert!((sh.x - 900.0).abs() <= sim.unit_px, "came back to {} not the cursor 900", sh.x);
    let t = sh.props.toy.expect("the toy is by the pet");
    assert!(!t.in_mouth && t.lift == 0.0 && t.depth == 0.0);
    assert!((t.x - sh.x) * sh.facing as f64 > 0.0, "the toy is in front of the pet");
}

#[test]
fn fetch_round_trip_bun() {
    let mut sim = Sim::new(Pet::Rabbit);
    sim.run(1.0, |_, _| {});
    throw(&mut sim, Toy::Football, 900, 3000.0);
    let (seq, deepest, mouth) = fetch_run(&mut sim, 60.0);
    assert!(seq.contains(&Anim::Walk) && seq.contains(&Anim::Nudge), "{seq:?}");
    assert!(!seq.contains(&Anim::Eat), "the bun doesn't pick it up");
    assert!(deepest > 0.2);
    assert!(mouth.iter().all(|m| !*m), "the bun never carries it in its mouth");
    assert_eq!(sim.last.anim, Anim::Sit);
}

#[test]
fn bun_noses_the_toy_ahead() {
    let mut sim = Sim::new(Pet::Rabbit);
    sim.run(1.0, |_, _| {});
    throw(&mut sim, Toy::Football, 900, 3000.0);
    let mut checked = 0;
    for _ in 0..(60.0 / DT) as usize {
        let sh = sim.step();
        if sh.anim == Anim::Nudge {
            let t = sh.props.toy.unwrap();
            assert!((t.x - sh.x) * sh.facing as f64 > 0.0 && t.lift == 0.0, "toy not ahead of the nose: {t:?} pet x {}", sh.x);
            checked += 1;
        }
        if sh.anim == Anim::Sit {
            break;
        }
    }
    assert!(checked > 10);
}

#[test]
fn weak_flick_just_drops() {
    let mut sim = Sim::new(Pet::Dog);
    sim.run(1.0, |_, _| {});
    throw(&mut sim, Toy::Ball, 700, 2.0 * 66.0);
    let (seq, deepest, _) = fetch_run(&mut sim, 40.0);
    assert_eq!(deepest, 0.0, "a weak flick doesn't throw it in");
    assert!(seq.contains(&Anim::Carry) && seq.last() == Some(&Anim::Sit), "{seq:?}");
}

#[test]
fn throw_depth_formula() {
    assert_eq!(fetch::throw_depth(0.0, 66.0), 0.25);
    assert_eq!(fetch::throw_depth(1650.0, 66.0), 1.0);
    assert_eq!(fetch::throw_depth(1e9, 66.0), 3.0);
}

#[test]
fn throw_target_clamped() {
    let mut sim = Sim::new(Pet::Dog);
    sim.run(1.0, |_, _| {});
    throw(&mut sim, Toy::Ball, 5, 3000.0);
    let (lo, hi) = sim.bounds();
    let (seq, _, _) = fetch_run(&mut sim, 60.0);
    assert!(seq.contains(&Anim::Eat), "never reached the toy: {seq:?}");
    let t = sim.last.props.toy.unwrap();
    assert!(t.x >= lo - 1e-6 && t.x <= hi + 1e-6, "toy at {} outside {lo}..{hi}", t.x);
    assert!(sim.in_bounds(sim.last.x));
}

#[test]
fn cancel_mid_flight() {
    for when in [0.2, 1.5] {
        let mut sim = Sim::new(Pet::Dog);
        sim.run(1.0, |_, _| {});
        throw(&mut sim, Toy::Coconut, 900, 3000.0);
        sim.run(when, |_, _| {});
        sim.cancel = true;
        let sh = sim.step();
        assert!(!sh.playing, "still playing after cancel at {when}");
        assert!(sh.props.toy.is_none() && sh.depth == 0.0 && sh.lift == 0.0);
    }
}

#[test]
fn waiting_toy_drops() {
    let mut sim = Sim::new(Pet::Dog);
    sim.run(1.0, |_, _| {});
    sim.cursor = Some((800, 700));
    sim.brain.choose(Choice::Fetch(Toy::Ball), sim.now);
    sim.step();
    sim.cursor = Some((300, 300)); // the user wanders off; the toy stays put
    sim.run(1.9, |_, sh| assert!(sh.props.toy.unwrap().lift > 0.0, "dropped too soon"));
    sim.run(3.0, |_, _| {});
    let t = sim.last.props.toy.unwrap();
    assert!(t.lift == 0.0 && (t.x - 800.0).abs() < 1.0, "{t:?}");
    let (seq, _, _) = fetch_run(&mut sim, 30.0);
    assert_eq!(seq.last(), Some(&Anim::Sit), "the pet sits by it: {seq:?}");
    assert!(sim.last.playing);
}

// ---------------------------------------------------------------- M2: feed

// Fed from the hand (the user: "would love to feed it directly instead of
// just dropping it ... eating and slowly pulling the cursor towards it as it
// eats through a carrot the cursor is holding").

/// Choose `food` with the cursor `units` to the pet's right at mouth height.
fn feed_start(pet: Pet, food: rig::Food, units: f64) -> Sim {
    let mut sim = Sim::new(pet);
    sim.run(1.0, |_, _| {});
    let (_, my) = sim.brain.eat_mouth(sim.unit_px, sim.work);
    sim.cursor = Some(((sim.last.x + units * sim.unit_px) as i32, my as i32 - (0.6 * sim.unit_px) as i32));
    sim.brain.choose(Choice::Feed(food), sim.now);
    sim.step();
    sim
}

fn food_at_cursor(sim: &Sim) -> bool {
    let (Some(f), Some((cx, cy))) = (sim.last.props.food, sim.cursor) else { return false };
    let ground = sim.work.bottom() as f64;
    (f.x - cx as f64).abs() < 1.0 && (f.lift - (ground - cy as f64)).abs() < 1.0
}

/// Run until play ends; anims in order, bites in order, distances from the
/// food to the eating mouth while eating.
fn feed_run(sim: &mut Sim) -> (Vec<Anim>, Vec<u8>, Vec<f64>) {
    let (mut anims, mut bites, mut dists) = (vec![], vec![], vec![]);
    for _ in 0..(30.0 / DT) as usize {
        let sh = sim.step();
        if anims.last() != Some(&sh.anim) {
            anims.push(sh.anim);
        }
        if let Some(f) = sh.props.food {
            assert!(food_at_cursor(sim), "the food left the hand: {f:?} cursor {:?}", sim.cursor);
            if bites.last() != Some(&f.bites) {
                bites.push(f.bites);
            }
            if sh.anim == Anim::Eat {
                let (mx, my) = sim.brain.eat_mouth(sim.unit_px, sim.work);
                let (cx, cy) = sim.cursor.unwrap();
                dists.push(((cx as f64 - mx).powi(2) + (cy as f64 - my).powi(2)).sqrt());
            }
        }
        if !sh.playing {
            break;
        }
    }
    (anims, bites, dists)
}

#[test]
fn the_bun_eats_from_the_hand() {
    let mut sim = feed_start(Pet::Rabbit, rig::Food::Carrot, 4.0);
    assert!(sim.last.playing && food_at_cursor(&sim), "the carrot is in the hand");
    let (anims, bites, dists) = feed_run(&mut sim);
    let eat = anims.iter().position(|a| *a == Anim::Eat).expect("never ate");
    assert!(anims[..eat].contains(&Anim::Walk), "came over first: {anims:?}");
    assert_eq!(bites, [0, 1, 2, 3], "three bites");
    // it pulls the cursor (and the carrot) in to its mouth as it eats
    assert!(dists.first().unwrap() > dists.last().unwrap(), "not pulled in: {dists:?}");
    assert!(*dists.last().unwrap() <= 0.4 * sim.unit_px, "never reached the mouth");
    assert!(!sim.warps.is_empty(), "the cursor was never moved");
    assert_eq!(anims.get(eat + 1), Some(&Anim::Hop), "a binky after: {anims:?}");
    assert!(!sim.last.playing && sim.last.props.food.is_none());
}

#[test]
fn alex_eats_from_the_hand_then_wags() {
    let mut sim = feed_start(Pet::Dog, rig::Food::Chicken, 3.0);
    let (mut eat_t, mut wag_t, mut crumbs) = (0.0, 0.0, false);
    for _ in 0..(30.0 / DT) as usize {
        let sh = sim.step();
        match sh.anim {
            Anim::Eat => eat_t += DT,
            Anim::Wag => wag_t += DT,
            _ => {}
        }
        crumbs |= sh.props.crumbs.is_some();
        if !sh.playing {
            break;
        }
    }
    assert!(eat_t >= 2.4 - 0.05 && eat_t < 4.0, "ate for {eat_t} s");
    assert!((wag_t - 1.6).abs() < 0.1, "wagged for {wag_t} s");
    assert!(crumbs, "no crumbs");
}

#[test]
fn pulling_the_food_away_stops_the_meal() {
    let mut sim = feed_start(Pet::Rabbit, rig::Food::Banana, 3.0);
    for _ in 0..(10.0 / DT) as usize {
        if sim.step().anim == Anim::Eat {
            break;
        }
    }
    assert_eq!(sim.last.anim, Anim::Eat);
    sim.run(0.7, |_, _| {});
    let bites = sim.last.props.food.unwrap().bites;
    // snatch it away and hold it there
    let far = ((sim.last.x + 6.0 * sim.unit_px) as i32, 700);
    let mut walked = false;
    for _ in 0..20 {
        sim.cursor = Some(far);
        let sh = sim.step();
        walked |= sh.anim == Anim::Walk;
        assert_eq!(sh.props.food.unwrap().bites, bites, "kept eating it from afar");
    }
    assert!(walked, "didn't follow the food");
    assert!(sim.last.playing);
}

#[test]
fn fed_by_hand_without_warp() {
    let mut sim = feed_start(Pet::Rabbit, rig::Food::Coriander, 3.0);
    sim.can_warp = false;
    // the user holds it right at the mouth
    let mut ate = 0.0;
    for _ in 0..(20.0 / DT) as usize {
        let (mx, my) = sim.brain.eat_mouth(sim.unit_px, sim.work);
        sim.cursor = Some((mx as i32, my as i32));
        let sh = sim.step();
        if sh.anim == Anim::Eat {
            ate += DT;
        }
        if !sh.playing {
            break;
        }
    }
    assert!(sim.warps.is_empty(), "moved a cursor that can't be moved");
    assert!(ate >= 2.3, "ate for {ate}");
    assert!(!sim.last.playing);
}

#[test]
fn food_out_of_reach_stays_in_the_hand() {
    // held high above the pet: it never gets eaten, and nothing panics
    let mut sim = feed_start(Pet::Dog, rig::Food::Bone, 0.0);
    let x = sim.last.x as i32;
    for _ in 0..(5.0 / DT) as usize {
        sim.cursor = Some((x, 100));
        let sh = sim.step();
        assert_ne!(sh.anim, Anim::Eat, "ate it from the top of the screen");
        assert!(sim.in_bounds(sh.x));
    }
    assert!(sim.last.playing && food_at_cursor(&sim));
}

#[test]
fn cancel_mid_meal() {
    let mut sim = feed_start(Pet::Dog, rig::Food::Chicken, 3.0);
    for _ in 0..(10.0 / DT) as usize {
        if sim.step().anim == Anim::Eat {
            break;
        }
    }
    assert_eq!(sim.last.anim, Anim::Eat);
    sim.cancel = true;
    let sh = sim.step();
    assert!(!sh.playing && sh.props.food.is_none() && sh.props.crumbs.is_none() && sh.warp.is_none());
}

#[test]
fn alex_runs_into_the_screen_and_back() {
    let mut sim = Sim::new(Pet::Dog);
    sim.run(1.0, |_, _| {});
    // thrown straight in from right where he stands
    let x = sim.last.x as i32;
    throw(&mut sim, Toy::Ball, x, 3000.0);
    let (seq, _, _) = fetch_run(&mut sim, 40.0);
    assert!(seq.contains(&Anim::RunAway), "ran in side-on: {seq:?}");
    assert!(seq.contains(&Anim::CarryBack), "came back side-on: {seq:?}");
    // the bun keeps its side-on hop
    let mut sim = Sim::new(Pet::Rabbit);
    sim.run(1.0, |_, _| {});
    let x = sim.last.x as i32;
    throw(&mut sim, Toy::Football, x, 3000.0);
    let (seq, _, _) = fetch_run(&mut sim, 60.0);
    assert!(!seq.contains(&Anim::RunAway) && !seq.contains(&Anim::CarryBack), "{seq:?}");
}

#[test]
fn play_put_away_when_the_user_goes_back_to_work() {
    // moving the mouse for other things is not playing: a toy left lying,
    // or food left in the hand, is put away after 30 s
    let mut sim = Sim::new(Pet::Dog);
    sim.run(1.0, |_, _| {});
    throw(&mut sim, Toy::Ball, 900, 3000.0);
    fetch_run(&mut sim, 40.0);
    assert!(sim.last.playing && sim.last.anim == Anim::Sit, "ready for another throw");
    let mut k = 0;
    sim.run(25.0, |_, _| {});
    for _ in 0..(10.0 / DT) as usize {
        k += 1;
        sim.cursor = Some((200 + (k % 50) * 10, 300)); // busy elsewhere
        sim.step();
    }
    assert!(!sim.last.playing && sim.last.props.toy.is_none(), "the toy is still out after 35 s of not playing");

    let mut sim = feed_start(Pet::Rabbit, rig::Food::Carrot, 0.0);
    for i in 0..(32.0 / DT) as usize {
        sim.cursor = Some((100 + (i as i32 % 40) * 5, 150)); // held up high, out of reach
        sim.step();
    }
    assert!(!sim.last.playing && sim.last.props.food.is_none(), "the food is still in the hand");
}

#[test]
fn long_legged_dog_walks_further() {
    let walked = |legs: f64| {
        let mut d = pixelpet::rig::Design::alex();
        d.shape.legs = legs;
        let mut sim = Sim::new(Pet::Dog);
        sim.brain = Brain::with_design(d, SEED, 0.0, WORK);
        sim.brain.pin(Some(Anim::Walk));
        sim.step();
        let x0 = sim.last.x;
        sim.run(2.0, |_, _| {});
        (sim.last.x - x0).abs()
    };
    let (short, long) = (walked(1.0), walked(1.3));
    assert!(long > short * 1.2, "legs 1.3 walked {long:.0} px, legs 1.0 {short:.0} px");
}
