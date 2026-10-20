//! The wander script: which anim plays next and for how long, from the look
//! sheet's weights. Own xorshift64* RNG, so a seed replays exactly.

use crate::rig::Anim;

/// xorshift64*.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // splitmix the seed so small seeds still start well mixed; never 0
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(if z == 0 { 1 } else { z })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }
}

const WEIGHTS: [(Anim, f64); 7] = [
    (Anim::Walk, 4.0),
    // the user asked for this often: looking up at the screen with you
    (Anim::Watch, 2.0),
    (Anim::Stand, 1.4),
    (Anim::Sit, 1.4),
    (Anim::Tilt, 1.1),
    (Anim::Hop, 1.0),
    (Anim::Sleep, 0.8),
];

/// One step of the script.
pub struct Next {
    pub anim: Anim,
    pub secs: f64,
    /// Turn around before starting (only ever true for walk).
    pub turn: bool,
}

pub struct Wander {
    rng: Rng,
    last: Option<Anim>,
}

impl Wander {
    pub fn new(seed: u64) -> Wander {
        Wander { rng: Rng::new(seed), last: None }
    }

    /// The same anim never comes twice in a row except walk; sleep is always
    /// followed by stand.
    pub fn next(&mut self) -> Next {
        let anim = if self.last == Some(Anim::Sleep) {
            Anim::Stand
        } else {
            let ok = |a: Anim| a == Anim::Walk || Some(a) != self.last;
            let total: f64 = WEIGHTS.iter().filter(|(a, _)| ok(*a)).map(|(_, w)| w).sum();
            let mut r = self.rng.f64() * total;
            let mut pick = Anim::Walk;
            for (a, w) in WEIGHTS.iter().filter(|(a, _)| ok(*a)) {
                pick = *a;
                if r < *w {
                    break;
                }
                r -= w;
            }
            pick
        };
        self.last = Some(anim);
        let secs = match anim {
            Anim::Walk => self.rng.range(3.0, 6.0),
            Anim::Stand => self.rng.range(2.5, 4.0),
            Anim::Sit => self.rng.range(3.0, 5.0),
            Anim::Tilt => 4.0,
            Anim::Hop => 2.4,
            Anim::Sleep => self.rng.range(9.6, 14.4),
            Anim::Watch => self.rng.range(6.0, 12.0),
            _ => 2.0, // not in the script
        };
        let turn = anim == Anim::Walk && self.rng.f64() < 0.35;
        Next { anim, secs, turn }
    }

    /// Something outside the script played (a nap): the script continues as
    /// if it had.
    pub fn note(&mut self, anim: Anim) {
        self.last = Some(anim);
    }
}
