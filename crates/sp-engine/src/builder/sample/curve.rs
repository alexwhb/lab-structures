//! The sample's stress–strain curve, built from a piece type's own builder properties
//! ([`TypeSpec`]: area, E, k, tension and compression strength, ductile, tension-only) and its
//! capacity in the builder at a temperature.
//!
//! - **Pull**: a straight line at E (× the fire's stiffness factor) up to the strength the builder
//!   gives the piece (its tension capacity ÷ area). Brittle types snap there. Ductile types
//!   (Steel, Cable) then flow on a yield plateau, harden and neck to fracture — the shape of a
//!   representative mild-steel / cold-drawn-wire curve, its stresses in proportion to the
//!   builder's strength ([`DUCTILE_STEEL`], [`DUCTILE_WIRE`]).
//! - **Push** (not Rope or Cable): a sample of the builder's section and length (1 m unless
//!   chosen) is squeezed. It gives way at the builder's compression capacity: the smaller of
//!   crushing (A·fC) and Euler buckling (π²·E·k·A²/L²), each with its fire factor. Then it
//!   bows sideways (buckled), squashes (steel), or cracks and crushes (concrete, Road, Floor,
//!   wood).
//!
//! Capacities come from the builder's own piece model: a `sim::Constraint` built as `Sim::new`
//! builds one, heated by [`crate::builder::fire::apply`], read with `cap_t_now()` /
//! `cap_c_now()`. So "a piece this size breaks at N t" is the builder's number.
//!
//! Units: strain in **percent** (shortening for Push, as a positive number), stress in **MPa**,
//! forces in N.

use crate::builder::data::{PieceType, TypeSpec};
use crate::builder::fire::{self, AMBIENT_C, Factors, FireMaterial};
use crate::builder::sim::{self, Constraint};
use crate::truss::hypot;

/// The builder's tonne: forces are shown as N / 9810 (the inspector's `N / 9810`).
pub const N_PER_TONNE: f64 = 9810.0;
/// The default sample length for Push, m (SPEC LAB-1: "a 1 m sample of the builder's own
/// section").
pub const DEFAULT_LENGTH_M: f64 = 1.0;
/// Shortest sample, m.
pub const MIN_LENGTH_M: f64 = 1.0;
/// Largest necking drawn at fracture (fraction of the gauge half-width lost), as v1.
pub const NECK_MAX: f64 = 0.55;

/// Pull or push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Test {
    /// A tensile test: pulled until it breaks.
    #[default]
    Pull,
    /// A compression test: pushed until it crushes or buckles.
    Push,
}

impl Test {
    /// "Pull" / "Push".
    pub fn name(self) -> &'static str {
        match self {
            Test::Pull => "Pull",
            Test::Push => "Push",
        }
    }
}

/// How the sample fails at the end of its curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Failure {
    /// Ductile, pulled: yielded, hardened, necked and tore ("Fractured").
    Fractured,
    /// Brittle, pulled: snapped at its strength (Wood, Rope).
    Snapped,
    /// Concrete, Road or Floor pulled: cracked at its (small) tension strength.
    Cracked,
    /// Steel pushed: squashed for good (the builder's "crushed").
    Squashed,
    /// Concrete, Road, Floor or Wood pushed past its peak until it crushed.
    Crushed,
    /// Pushed, bowed sideways at the Euler load.
    Buckled,
    /// Nothing left to test: wood or rope burned through in the heat.
    BurnedThrough,
}

/// A ductile curve's shape at 20 °C: `[strain %, stress ÷ yield]` corners after the yield point
/// (the first is the end of the yield plateau). At other temperatures the plastic strains keep
/// their offsets beyond the (heat-shifted) yield strain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuctileShape {
    /// Yield strain at 20 °C, % (where the room-temperature line ends; the offsets are from it).
    pub yield_strain: f64,
    /// Corners after yield: `[strain %, stress ÷ yield]` at 20 °C.
    pub corners: &'static [[f64; 2]],
    /// Strain at the peak stress (necking starts after it), % at 20 °C.
    pub ultimate_strain: f64,
}

/// Mild structural steel: v1's representative S275 curve (`materials::CURVES[Steel]`: plateau to
/// 1.5 %, peak 1.56 × yield at 15 %, fracture at 22 %) with its stresses in proportion.
pub const DUCTILE_STEEL: DuctileShape = DuctileShape {
    yield_strain: 0.125,
    corners: &[
        [1.5, 1.0],
        [5.0, 360.0 / 275.0],
        [10.0, 415.0 / 275.0],
        [15.0, 430.0 / 275.0],
        [19.0, 405.0 / 275.0],
        [22.0, 360.0 / 275.0],
    ],
    ultimate_strain: 15.0,
};

/// Cold-drawn high-strength wire (cable): a short plateau, peak 1.13 × yield (the 0.1 % proof
/// stress is about 0.88 of the tensile strength, EN 10138) at 3.5 % (the minimum elongation at
/// maximum force), fracture at 4.5 %.
pub const DUCTILE_WIRE: DuctileShape = DuctileShape {
    yield_strain: 0.9375,
    corners: &[[1.2, 1.0], [2.5, 1.1], [3.5, 1.0 / 0.88], [4.5, 1.08]],
    ultimate_strain: 3.5,
};

/// Post-peak corners for brittle pushing, `[strain offset beyond the peak %, stress ÷ peak]`:
/// concrete to EN 1992-1-1's crushing strain 0.35 % at 20 °C (`ε_cu1`).
pub const CONCRETE_PUSH_TAIL: [[f64; 2]; 2] =
    [[0.25 - 40.0 / 300.0, 0.85], [0.35 - 40.0 / 300.0, 0.7]];
/// Wood pushed along the grain: the fibres fold and it keeps most of its strength a while.
pub const WOOD_PUSH_TAIL: [[f64; 2]; 2] = [[0.55, 0.9], [0.95, 0.75]];
/// Steel pushed: the plateau runs this far beyond the yield strain before the test stops, %.
pub const STEEL_PUSH_PLATEAU: f64 = 1.875;
/// A buckled sample bows on at a falling load: `[strain ÷ buckling strain, stress ÷ σ_cr]`.
pub const BUCKLE_TAIL: [[f64; 2]; 1] = [[2.0, 0.85]];

/// The builder's piece of `kind`, `len` metres long, at `temp_c` °C: a `Constraint` with the
/// core's own capacities and stiffness (`sim::capacity`, what `Sim::new` uses), heated by
/// [`fire::apply`]. Lengths are made safe (non-finite or ≤ 0 → 1 m).
pub fn piece_at(kind: PieceType, len: f64, temp_c: f64) -> Constraint {
    let t = kind.spec();
    let lm = hypot(safe_len(len), 0.0);
    let cap = sim::capacity(kind, lm);
    let mut c = Constraint {
        id: "S1".into(),
        a: 0,
        b: 1,
        kind,
        len: lm,
        kk: cap.kk,
        cap_t: cap.cap_t,
        cap_c: cap.cap_c,
        crush: cap.crush,
        pcr: cap.pcr,
        buckles: cap.buckles,
        t_only: t.t_only,
        deck: t.deck,
        road: kind == PieceType::Road,
        n: 0.0,
        avg: 0.0,
        broken: false,
        over: false,
        slack: false,
        exposed: false,
        strength: 1.0,
        stiffness: 1.0,
        temp: AMBIENT_C,
        heat_mode: None,
    };
    c.temp = safe_temp(temp_c);
    fire::apply(&mut c);
    c
}

/// A length made safe: non-finite or ≤ 0 → [`DEFAULT_LENGTH_M`].
pub fn safe_len(len: f64) -> f64 {
    if len.is_finite() && len > 0.0 {
        len
    } else {
        DEFAULT_LENGTH_M
    }
}

/// A temperature made safe: NaN → 20 °C, then clamped to −273…1200 °C.
pub fn safe_temp(t_c: f64) -> f64 {
    if t_c.is_nan() {
        AMBIENT_C
    } else {
        t_c.clamp(-273.0, 1200.0)
    }
}

/// The builder's capacity for a piece of `kind`, `len` m long, at `temp_c` °C, in the direction
/// of `test`, N: Pull → `cap_t_now()`; Push → `cap_c_now()` (`None` for Rope and Cable, which
/// only pull).
pub fn capacity(kind: PieceType, test: Test, len: f64, temp_c: f64) -> Option<f64> {
    let c = piece_at(kind, len, temp_c);
    match test {
        Test::Pull => Some(c.cap_t_now()),
        Test::Push if c.t_only => None,
        Test::Push => Some(c.cap_c_now()),
    }
}

/// The stress in the sample at a strain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stress {
    /// Stress, MPa (0 once broken).
    pub mpa: f64,
    /// Past the end of the curve.
    pub broken: bool,
}

/// A sample's curve for one test at one temperature and length.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleCurve {
    /// The piece type.
    pub kind: PieceType,
    /// Pull or push.
    pub test: Test,
    /// Temperature, °C.
    pub temp_c: f64,
    /// Sample length, m (matters for Push: buckling).
    pub length_m: f64,
    /// `[strain %, stress MPa]` corners from `[0, 0]`; the sample fails past the last.
    pub points: Vec<[f64; 2]>,
    /// The builder's capacity this way, N ("a piece this size breaks at …").
    pub capacity_n: f64,
    /// That capacity as a stress on the section, MPa (the end of the straight line).
    pub limit_mpa: f64,
    /// Strain at the end of the straight line, %.
    pub elastic_strain: f64,
    /// Strain where necking starts (ductile Pull), %.
    pub ultimate_strain: Option<f64>,
    /// How it fails.
    pub failure: Failure,
    /// Push: it buckles before it crushes.
    pub buckles: bool,
    /// Crushing capacity this way (Push), N — what it would carry if it did not buckle.
    pub crush_n: f64,
    /// The fire's factors at this temperature (strength as the builder applies it).
    pub strength: f64,
    /// … and stiffness.
    pub stiffness: f64,
    /// The section, m².
    pub area: f64,
    /// Young's modulus at this temperature, Pa.
    pub e: f64,
}

impl SampleCurve {
    /// The curve for `kind` under `test` at `temp_c` °C, `len` m long; `None` for pushing Rope or
    /// Cable (they only pull).
    pub fn new(kind: PieceType, test: Test, temp_c: f64, len: f64) -> Option<SampleCurve> {
        let t: &TypeSpec = kind.spec();
        if test == Test::Push && t.t_only {
            return None;
        }
        let temp_c = safe_temp(temp_c);
        let length_m = safe_len(len);
        let c = piece_at(kind, length_m, temp_c);
        let e = t.e * c.stiffness;
        let pct = |mpa: f64| if e > 0.0 { mpa * 1e6 / e * 100.0 } else { 0.0 };
        let mat = FireMaterial::of(kind);
        let mut curve = SampleCurve {
            kind,
            test,
            temp_c,
            length_m,
            points: vec![[0.0, 0.0]],
            capacity_n: 0.0,
            limit_mpa: 0.0,
            elastic_strain: 0.0,
            ultimate_strain: None,
            failure: Failure::BurnedThrough,
            buckles: false,
            crush_n: 0.0,
            strength: c.strength,
            stiffness: c.stiffness,
            area: t.area,
            e,
        };
        match test {
            Test::Pull => {
                let cap = c.cap_t_now();
                curve.capacity_n = cap;
                if !positive(cap) || !positive(e) {
                    return Some(curve);
                }
                let sy = cap / t.area / 1e6;
                let ey = pct(sy);
                curve.limit_mpa = sy;
                curve.elastic_strain = ey;
                curve.points.push([ey, sy]);
                if t.ductile {
                    let shape = if kind == PieceType::Cable {
                        &DUCTILE_WIRE
                    } else {
                        &DUCTILE_STEEL
                    };
                    let shift = ey - shape.yield_strain;
                    for q in shape.corners {
                        curve.points.push([q[0] + shift, q[1] * sy]);
                    }
                    curve.ultimate_strain = Some(shape.ultimate_strain + shift);
                    curve.failure = Failure::Fractured;
                } else {
                    curve.failure = match mat {
                        FireMaterial::Concrete => Failure::Cracked,
                        _ => Failure::Snapped,
                    };
                }
            }
            Test::Push => {
                let cap = c.cap_c_now();
                let crush = c.crush * c.strength;
                curve.capacity_n = cap;
                curve.crush_n = crush;
                if !positive(cap) || !positive(e) || !cap.is_finite() {
                    curve.capacity_n = cap.max(0.0);
                    return Some(curve);
                }
                let s = cap / t.area / 1e6;
                let es = pct(s);
                curve.limit_mpa = s;
                curve.elastic_strain = es;
                curve.points.push([es, s]);
                curve.buckles = c.pcr * c.stiffness < crush;
                if curve.buckles {
                    for q in BUCKLE_TAIL {
                        curve.points.push([es * q[0], s * q[1]]);
                    }
                    curve.failure = Failure::Buckled;
                } else if t.ductile {
                    curve.points.push([es + STEEL_PUSH_PLATEAU, s]);
                    curve.failure = Failure::Squashed;
                } else {
                    let tail = if mat == FireMaterial::Wood {
                        &WOOD_PUSH_TAIL
                    } else {
                        &CONCRETE_PUSH_TAIL
                    };
                    for q in tail {
                        curve.points.push([es + q[0], s * q[1]]);
                    }
                    curve.failure = Failure::Crushed;
                }
            }
        }
        Some(curve)
    }

    /// Strain at failure, % (the last corner; 0 when it has burned through).
    pub fn break_strain(&self) -> f64 {
        self.points.last().map_or(0.0, |p| p[0])
    }

    /// Nothing to test: the heat took all its strength.
    pub fn burned_through(&self) -> bool {
        self.failure == Failure::BurnedThrough
    }

    /// The highest stress on the curve, MPa.
    pub fn peak_mpa(&self) -> f64 {
        self.points.iter().map(|p| p[1]).fold(0.0, f64::max)
    }

    /// The stress at `strain` % (NaN / negative → 0): linear between the corners, 0 and broken
    /// past the last. A burned-through sample is broken from the start.
    pub fn stress_at(&self, strain: f64) -> Stress {
        let e = if strain.is_nan() || strain < 0.0 {
            0.0
        } else {
            strain
        };
        if self.burned_through() {
            return Stress {
                mpa: 0.0,
                broken: true,
            };
        }
        let p = &self.points;
        if e > self.break_strain() {
            return Stress {
                mpa: 0.0,
                broken: true,
            };
        }
        for w in p.windows(2) {
            let (a, b) = (w[0], w[1]);
            if e <= b[0] {
                let f = if b[0] > a[0] {
                    (e - a[0]) / (b[0] - a[0])
                } else {
                    1.0
                };
                return Stress {
                    mpa: a[1] + (b[1] - a[1]) * f,
                    broken: false,
                };
            }
        }
        Stress {
            mpa: p.last().map_or(0.0, |q| q[1]),
            broken: false,
        }
    }

    /// The force on the builder's section at `strain` %, N.
    pub fn force_at(&self, strain: f64) -> f64 {
        self.stress_at(strain).mpa * 1e6 * self.area
    }

    /// Necking at `strain` % (ductile Pull): 0 up to the ultimate strain, rising to
    /// [`NECK_MAX`] at fracture.
    pub fn necking(&self, strain: f64) -> f64 {
        let Some(u) = self.ultimate_strain else {
            return 0.0;
        };
        let brk = self.break_strain();
        let e = if strain.is_nan() { 0.0 } else { strain };
        if e > u && brk > u {
            NECK_MAX.min((e.min(brk) - u) / (brk - u) * NECK_MAX)
        } else {
            0.0
        }
    }

    /// The fire factors as one value pair (strength, stiffness).
    pub fn factors(&self) -> Factors {
        Factors {
            strength_t: self.strength,
            strength_c: self.strength,
            stiffness: self.stiffness,
        }
    }
}

/// `x > 0` (false for NaN).
fn positive(x: f64) -> bool {
    x > 0.0
}

/// A force in the builder's tonnes (N / 9810).
pub fn tonnes(n: f64) -> f64 {
    n / N_PER_TONNE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steel_pull_matches_v1_shape_at_room_temperature() {
        let c = SampleCurve::new(PieceType::Steel, Test::Pull, 20.0, 1.0).unwrap();
        assert_eq!(c.points[1], [0.125, 250.0]);
        assert!((c.points[2][0] - 1.5).abs() < 1e-12);
        assert!((c.break_strain() - 22.0).abs() < 1e-12);
        assert_eq!(c.failure, Failure::Fractured);
    }

    #[test]
    fn rope_and_cable_cannot_be_pushed() {
        assert!(SampleCurve::new(PieceType::Rope, Test::Push, 20.0, 1.0).is_none());
        assert!(SampleCurve::new(PieceType::Cable, Test::Push, 20.0, 1.0).is_none());
        assert!(capacity(PieceType::Cable, Test::Push, 1.0, 20.0).is_none());
    }

    #[test]
    fn concrete_push_tail_ends_at_0_35_percent() {
        let c = SampleCurve::new(PieceType::Concrete, Test::Push, 20.0, 1.0).unwrap();
        assert!((c.break_strain() - 0.35).abs() < 1e-12, "{:?}", c.points);
    }
}
