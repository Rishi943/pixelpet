//! Behaviour scenarios on a fake clock. Every test steps `update` at 30 Hz.

use pixelpet::brain::{Brain, Inputs, Mouse, Show};
use pixelpet::platform::Rect;
use pixelpet::rig::{self, Anim, Pet};
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
    last: Show,
}

impl Sim {
    fn new(pet: Pet) -> Sim {
        Sim::with_work(pet, WORK, 66.0)
    }

    fn with_work(pet: Pet, work: Rect, unit_px: f64) -> Sim {
        let mut brain = Brain::new(pet, SEED, 0.0, work);
        let s = Settings::default();
        let inp = Inputs { now: 0.0, work, unit_px, cursor: None, over_pet: false, mouse: Mouse::None, idle: Some(0.0), fullscreen: Some(false) };
        let last = brain.update(&inp, &s);
        Sim { brain, s, now: 0.0, work, unit_px, cursor: None, over_pet: false, idle: Some(0.0), fullscreen: Some(false), last }
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
        };
        self.last = self.brain.update(&inp, &self.s);
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
    // input: wakes to stand at once
    sim.idle = Some(0.0);
    let sh = sim.step();
    assert_eq!(sh.anim, Anim::Stand);
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
    assert_eq!(sim.step().anim, Anim::Stand, "idle went down: there was input");
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
