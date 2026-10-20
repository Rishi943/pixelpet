//! Petting: counting strokes as the cursor sweeps back and forth across the pet.

/// One stroke is a change of direction after at least this many rig units
/// of travel the other way; smaller wobbles don't count.
pub const STROKE_UNITS: f64 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Strokes {
    last_x: Option<f64>,
    dir: i8,
    travel: f64,
    pub count: u32,
}

impl Strokes {
    /// The cursor is at `x` (over the pet). True when this completes a stroke.
    pub fn feed(&mut self, x: f64, unit_px: f64) -> bool {
        let Some(last) = self.last_x.replace(x) else { return false };
        let dx = x - last;
        if dx == 0.0 {
            return false;
        }
        let d = if dx > 0.0 { 1 } else { -1 };
        if self.dir == d || self.dir == 0 {
            self.dir = d;
            self.travel += dx.abs();
            return false;
        }
        let stroke = self.travel >= STROKE_UNITS * unit_px;
        self.dir = d;
        self.travel = dx.abs();
        if stroke {
            self.count += 1;
        }
        stroke
    }
}
