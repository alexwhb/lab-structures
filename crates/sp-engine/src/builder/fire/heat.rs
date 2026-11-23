//! The fire's heat (FIRE-1): where it burns, how far it reaches, and how hot each piece gets.
//!
//! Illustrative, deterministic and cheap (one pass over the pieces per substep):
//!
//! - The fire lights [`FIRE_START_S`] into a run (or [`LIGHT_DELAY_S`] after "Light it now", or
//!   [`CHANGE_DELAY_S`] after Fire is switched on mid-run) and grows over [`GROW_S`]: its
//!   temperature rises from 20 °C to the level's and its reach from 0 to the level's
//!   ([`Blaze`]).
//! - A piece's exposure comes from its distance to the fire, measured as a plume: only what is at
//!   or above the fire's height heats, and height above it counts [`PLUME`] × (flames and hot gas
//!   rise). Inside half the reach a piece sees the full fire temperature; it falls off linearly
//!   to nothing at the reach.
//! - A piece's temperature follows its exposure with a first-order lag (time constant per
//!   material, [`lag_s`]: thin steel heats fastest, thick concrete slowest).
//! - Strength and stiffness follow the [`super::factors`] tables: steel and cable at their
//!   temperature now (they recover as they cool), the others at the hottest they have been
//!   (charred wood, melted rope and cracked concrete stay damaged).

use super::factors::{AMBIENT_C, FireMaterial};
use crate::builder::data::Scene;

/// Seconds into a run when a fire set before the run lights (an earthquake starts at 3 s too).
pub const FIRE_START_S: f64 = 3.0;
/// Seconds after Fire is switched on during a run before it lights (as a quake change, 0.5 s).
pub const CHANGE_DELAY_S: f64 = 0.5;
/// Seconds after "Light it now" (as "Shake now", 0.2 s).
pub const LIGHT_DELAY_S: f64 = 0.2;
/// Seconds the fire takes to reach its full temperature and reach.
pub const GROW_S: f64 = 5.0;
/// Height above the fire counts this much of a horizontal metre (the plume rises).
pub const PLUME: f64 = 0.5;
/// Inside this fraction of the reach a piece sees the full fire temperature.
pub const CORE: f64 = 0.5;
/// River scenes: the fire burns on the water under the middle of the span, world m.
pub const RIVER_SOURCE: [f64; 2] = [28.0, -8.0];
/// Open ground with nothing on a foundation: the default tower's centre line, world m.
pub const FLAT_DEFAULT_X: f64 = 100.0;
/// Below this many degrees above ambient a piece counts as cold.
pub const COLD_MARGIN_C: f64 = 0.05;

/// The heating time constant of a material, s (illustrative: section thickness and conductivity).
pub fn lag_s(m: FireMaterial) -> f64 {
    match m {
        FireMaterial::Steel | FireMaterial::Cable => 3.0,
        FireMaterial::Concrete => 8.0,
        FireMaterial::Wood => 4.0,
        FireMaterial::Rope => 2.0,
    }
}

/// Where the fire burns: under the middle of the span on the river; on open ground at the base
/// of the tower — the mean x of the joints on foundations (`supports`: pass only those a piece
/// stands on), or of `joints` at ground level when nothing is founded, else [`FLAT_DEFAULT_X`].
pub fn source_for(scene: Scene, supports: &[[f64; 2]], joints: &[[f64; 2]]) -> [f64; 2] {
    match scene {
        Scene::River => RIVER_SOURCE,
        Scene::Flat => {
            let mean = |ps: &mut dyn Iterator<Item = f64>| {
                let (mut s, mut n) = (0.0, 0usize);
                for x in ps {
                    if x.is_finite() {
                        s += x;
                        n += 1;
                    }
                }
                (n > 0).then(|| s / n as f64)
            };
            let x = mean(&mut supports.iter().map(|p| p[0]))
                .or_else(|| mean(&mut joints.iter().filter(|p| p[1].abs() < 0.5).map(|p| p[0])))
                .unwrap_or(FLAT_DEFAULT_X);
            [x, 0.0]
        }
    }
}

/// The plume distance from the fire at `s` to the segment `a`–`b`, m: only the part of the piece
/// at or above the fire's height counts (flames and hot gas rise; nothing below the fire heats),
/// and height above the fire counts [`PLUME`] ×. A piece wholly below the fire, or non-finite
/// input, gives `f64::INFINITY` (out of reach).
pub fn plume_distance(s: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    if !(s.iter().chain(&a).chain(&b)).all(|v| v.is_finite()) {
        return f64::INFINITY;
    }
    let (da, db) = (a[1] - s[1], b[1] - s[1]);
    let (a, b) = match (da >= 0.0, db >= 0.0) {
        (true, true) => (a, b),
        (false, false) => return f64::INFINITY,
        _ => {
            // clip at the fire's height, keeping the upper end
            let f = da / (da - db);
            let m = [a[0] + (b[0] - a[0]) * f, s[1]];
            if da >= 0.0 { (a, m) } else { (m, b) }
        }
    };
    let map = |p: [f64; 2]| [p[0] - s[0], (p[1] - s[1]).max(0.0) * PLUME];
    point_segment(map(a), map(b))
}

/// Distance from the origin to the segment `p`–`q`.
fn point_segment(p: [f64; 2], q: [f64; 2]) -> f64 {
    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
    let l2 = dx * dx + dy * dy;
    let f = if l2 > 0.0 {
        (-(p[0] * dx + p[1] * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (x, y) = (p[0] + dx * f, p[1] + dy * f);
    x.hypot(y)
}

/// How much of the fire's heat reaches a piece at plume distance `d` from a fire of reach `r`:
/// 1 inside [`CORE`] × r, falling linearly to 0 at r.
pub fn exposure(d: f64, r: f64) -> f64 {
    if r.is_nan() || d.is_nan() || r <= 0.0 || d >= r {
        return 0.0;
    }
    let core = CORE * r;
    if d <= core {
        1.0
    } else {
        (1.0 - (d - core) / (r - core)).clamp(0.0, 1.0)
    }
}

/// 0 before the fire lights at `fire_start`, rising to 1 over [`GROW_S`] (always 0 while
/// Fire is off).
pub fn growth(t: f64, fire_start: Option<f64>, on: bool) -> f64 {
    match fire_start {
        Some(l) if on && t.is_finite() && t >= l => ((t - l) / GROW_S).clamp(0.0, 1.0),
        _ => 0.0,
    }
}

/// The fully grown reach of a fire at `fire_c` °C, m of plume distance: the levels' 8 / 12 /
/// 16 m at 400 / 700 / 900 °C, interpolated in between (0 at 20 °C).
pub fn full_reach(fire_c: f64) -> f64 {
    super::factors::interp(
        &[AMBIENT_C, 400.0, 700.0, 900.0],
        &[0.0, 8.0, 12.0, 16.0],
        fire_c,
    )
}

/// One step of a piece's temperature toward `target` over `dt` seconds (exact first-order lag).
pub fn approach(temp: f64, target: f64, dt: f64, m: FireMaterial) -> f64 {
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let temp = if temp.is_finite() { temp } else { AMBIENT_C };
    temp + (target - temp) * (1.0 - (-dt / lag_s(m)).exp())
}

/// The fire at one moment: what the stage draws and the pieces feel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blaze {
    /// Where it burns, world m.
    pub source: [f64; 2],
    /// 0 → 1 while it grows.
    pub growth: f64,
    /// Its temperature now, °C.
    pub temp_c: f64,
    /// Its reach now, m of plume distance.
    pub reach: f64,
}

impl Blaze {
    /// A fire of `level_c` °C (0 = off) lit at `fire_start`, at sim time `t`, burning at `source`.
    pub fn at(level_c: f64, fire_start: Option<f64>, t: f64, source: [f64; 2]) -> Blaze {
        let level_c = sanitize_fire_c(level_c);
        let growth = growth(t, fire_start, level_c > 0.0);
        let peak = level_c.max(AMBIENT_C);
        Blaze {
            source,
            growth,
            temp_c: AMBIENT_C + (peak - AMBIENT_C) * growth,
            reach: full_reach(peak) * growth,
        }
    }

    /// True once it has lit.
    pub fn burning(&self) -> bool {
        self.growth > 0.0
    }

    /// The temperature a piece from `a` to `b` heads for, °C.
    pub fn target(&self, a: [f64; 2], b: [f64; 2]) -> f64 {
        if !self.burning() {
            return AMBIENT_C;
        }
        let x = exposure(plume_distance(self.source, a, b), self.reach);
        AMBIENT_C + (self.temp_c - AMBIENT_C) * x
    }
}

/// A Fire setting made safe: NaN, negative and 0 are off; at most 1200 °C (the tables' end).
pub fn sanitize_fire_c(c: f64) -> f64 {
    if c.is_nan() || c <= 0.0 {
        0.0
    } else {
        c.min(1200.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plume_distance_counts_height_above_half() {
        let s = [0.0, 0.0];
        assert_eq!(plume_distance(s, [-1.0, 8.0], [1.0, 8.0]), 4.0);
        assert_eq!(plume_distance(s, [-1.0, -8.0], [1.0, -8.0]), f64::INFINITY);
        assert_eq!(plume_distance(s, [3.0, -5.0], [3.0, 5.0]), 3.0);
        assert_eq!(
            plume_distance(s, [f64::NAN, 0.0], [1.0, 1.0]),
            f64::INFINITY
        );
        // A zero-length piece is a point.
        assert_eq!(plume_distance(s, [3.0, 0.0], [3.0, 0.0]), 3.0);
    }

    #[test]
    fn exposure_core_and_edge() {
        assert_eq!(exposure(2.0, 8.0), 1.0);
        assert_eq!(exposure(4.0, 8.0), 1.0);
        assert_eq!(exposure(6.0, 8.0), 0.5);
        assert_eq!(exposure(8.0, 8.0), 0.0);
        assert_eq!(exposure(1.0, 0.0), 0.0);
        assert_eq!(exposure(f64::NAN, 8.0), 0.0);
    }
}
