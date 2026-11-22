//! The Tower world (SCR-3, `docs/design/mockup/Tower.dc.html`): a skyscraper in a storm as a
//! **simplified cantilever** (PROV-1: "Wind + sway: simplified cantilever").
//!
//! [`calc`] is the mockup's `calc(n, sys, shape, v10, damper)` operation for operation, so the
//! screen shows exactly the mockup's numbers (golden test `tests/tower_goldens.rs`, generated
//! from the mockup by `tests/tower_ground_goldens.mjs`):
//!
//! - 4 m floors on a 40 m square plan; power-law wind `V(z) = V10·(max(z, 10)/10)^0.2`;
//! - floor forces `F = 0.613·V²·Cd·B·w(z)·4` at mid-floor (`w(z)` tapers to 45 % for the
//!   tapered shape), base shear `ΣF`, overturning `ΣF·z`;
//! - cantilever deflection `δ = ΣF·z²(3H − z)/(6EI)` with `EI = 4.3e13 × k(system)`;
//! - first frequency `f1 = 0.5596·√(EI/(μH⁴))`, `μ = 4e5 kg/m × mass(shape)`;
//! - damping ζ 1.5 %, 5 % with the tuned mass damper; peak acceleration
//!   `a = (2πf1)²·0.25·δ·√(0.015/ζ)`, boosted near vortex lock-in;
//! - vortex shedding `fs = 0.12·V_H/B`; lock-in nearness `max(0, 1 − |fs/f1 − 1|/0.25)`;
//! - game cost `n × cost(system) × cost(shape) (+ $6M damper)`.
//!
//! Around it: the comfort bands and sentences, the status pill, the "What's happening"
//! sentences, the readout tiles, "Same storm, four shapes", the "Challenge · 100 floors" goals,
//! the stage helpers (mode shape, taper / twist, wind arrows) and the Play gusts model
//! ([`sway_params`] → [`crate::dynamics::SwaySim`]).
//!
//! Everything is total: inputs are clamped to the screen's ranges ([`TowerInput::sanitized`]),
//! nothing panics.

pub mod fmt;

use crate::dynamics::SwayParams;
use fmt::{fixed, round_str};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Storey height, m.
pub const FLOOR_H: f64 = 4.0;
/// Plan width (square), m.
pub const PLAN_B: f64 = 40.0;
/// Power-law wind profile exponent.
pub const ALPHA: f64 = 0.2;
/// Bending stiffness of the core-only tower, N·m² (× the system's `k`).
pub const EI_CORE: f64 = 4.3e13;
/// Mass per metre of height, kg/m (× the shape's mass factor).
pub const MU: f64 = 4e5;
/// Structural damping ratio.
pub const ZETA: f64 = 0.015;
/// Damping ratio with the tuned mass damper.
pub const ZETA_DAMPER: f64 = 0.05;
/// Strouhal number of a square section.
pub const STROUHAL: f64 = 0.12;
/// Half-width of the lock-in band of `fs / f1` around 1 (so 0.75–1.25).
pub const LOCK_BAND: f64 = 0.25;
/// Game cost of the damper, $M.
pub const DAMPER_COST: f64 = 6.0;
/// Air pressure coefficient `½ρ`, kg/m³.
pub const HALF_RHO: f64 = 0.613;
/// Gravity used for milli-g, m/s².
pub const G: f64 = 9.81;
/// m/s → mph (the mockup's factor).
pub const MPH: f64 = 2.237;

/// Floors range (both the − / + buttons and the slider; SPEC MB-3).
pub const FLOORS_MIN: u32 = 5;
/// See [`FLOORS_MIN`].
pub const FLOORS_MAX: u32 = 120;
/// Storm slider range, m/s at 10 m.
pub const V10_MIN: f64 = 5.0;
/// See [`V10_MIN`].
pub const V10_MAX: f64 = 50.0;
/// "Show sway" exaggeration range.
pub const EX_MIN: f64 = 1.0;
/// See [`EX_MIN`].
pub const EX_MAX: f64 = 150.0;
/// Default exaggeration (`sway drawn ×40`).
pub const EX_DEFAULT: f64 = 40.0;

/// Drift aim: `H/500` or better is "OK".
pub const DRIFT_AIM: f64 = 500.0;
/// Drift worse than `H/200` is "Dangerously flexible".
pub const DRIFT_DANGER: f64 = 200.0;
/// Comfort bands, milli-g: noticed, uneasy, seasick (illustrative).
pub const COMFORT_BANDS: [f64; 3] = [5.0, 10.0, 20.0];
/// The comfort bar's full scale ("30+ milli-g").
pub const COMFORT_MAX: f64 = 30.0;
/// The challenge's budget, $M.
pub const BUDGET: f64 = 160.0;
/// The challenge's floors.
pub const CHALLENGE_FLOORS: u32 = 100;
/// The challenge's storm, m/s.
pub const CHALLENGE_V10: f64 = 30.0;
/// "Challenge · 100 floors" is rung 7 of the series' 12-rung challenge ladder.
pub const CHALLENGE_RUNG: (u32, u32) = (7, 12);

/// Generalised mass of the first mode per `μH`, for the mode shape [`mode_shape`]:
/// `∫₀¹ φ² dx = 33/140`.
pub const MODAL_MASS_FACTOR: f64 = 33.0 / 140.0;

/// A colour role. The screen maps it to `sp_theme`; [`Tone::css`] names the mockup colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tone {
    /// Primary text `#EAEAEA`.
    Plain,
    /// Fine / comfortable / done `#5FBF8A`.
    Ok,
    /// Marginal (the sand accent) `#D6C7A1`.
    Marginal,
    /// Uneasy / leaning / heavy `#E7A47C`.
    Caution,
    /// Danger text on dark (lock-in, over budget, seasick) `#E8A39E`.
    Alert,
    /// Failure (dangerously flexible, bearing failure) `#E5534B`.
    Danger,
}

impl Tone {
    /// The mockup's CSS colour for this role.
    pub fn css(self) -> &'static str {
        match self {
            Tone::Plain => "#EAEAEA",
            Tone::Ok => "#5FBF8A",
            Tone::Marginal => "#D6C7A1",
            Tone::Caution => "#E7A47C",
            Tone::Alert => "#E8A39E",
            Tone::Danger => "#E5534B",
        }
    }
}

// ============================================================================================
// Systems and shapes (the mockup's `defs()`)

/// Structural system (the left column's first tile group).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum System {
    /// A concrete core alone.
    #[default]
    Core,
    /// A framed tube (stiffness at the perimeter).
    Tube,
    /// Core tied to the outer columns by outrigger trusses.
    Outriggers,
    /// A tube with diagonal bracing.
    Braced,
}

/// A system's data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemDef {
    /// The mockup's key (`core`, `tube`, `outrigger`, `braced`).
    pub key: &'static str,
    /// The tile label.
    pub name: &'static str,
    /// Stiffness factor on [`EI_CORE`].
    pub k: f64,
    /// Cost factor.
    pub cost: f64,
    /// The tile icon: an SVG path in a 40 × 40 box (stroke `#D9D9D9`, width 1.5).
    pub icon: &'static str,
}

const SYSTEMS: [SystemDef; 4] = [
    SystemDef {
        key: "core",
        name: "Core",
        k: 1.0,
        cost: 1.0,
        icon: "M10 36V4H30V36M16 36V10H24V36",
    },
    SystemDef {
        key: "tube",
        name: "Tube",
        k: 2.2,
        cost: 1.2,
        icon: "M10 36V4H30V36M14 36V4M18 36V4M22 36V4M26 36V4",
    },
    SystemDef {
        key: "outrigger",
        name: "Outriggers",
        k: 3.0,
        cost: 1.3,
        icon: "M10 36V4H30V36M17 36V4H23V36M10 14H30M10 26H30",
    },
    SystemDef {
        key: "braced",
        name: "Braced tube",
        k: 4.2,
        cost: 1.45,
        icon: "M10 36V4H30V36M10 4L30 20L10 36M30 4L10 20L30 36",
    },
];

impl System {
    /// All systems in tile order.
    pub const ALL: [System; 4] = [
        System::Core,
        System::Tube,
        System::Outriggers,
        System::Braced,
    ];

    /// This system's data.
    pub fn def(self) -> &'static SystemDef {
        &SYSTEMS[self as usize]
    }

    /// The system with the mockup key `key`.
    pub fn from_key(key: &str) -> Option<System> {
        System::ALL.into_iter().find(|s| s.def().key == key)
    }
}

/// Tower shape (the left column's second tile group).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Shape {
    /// A plain prism.
    #[default]
    Straight,
    /// Narrows to 45 % at the top.
    Tapered,
    /// Rotates a quarter turn over its height.
    Twisted,
    /// Sky slots through the floors every 20 storeys.
    Hollow,
}

/// A shape's data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeDef {
    /// The mockup's key (`straight`, `tapered`, `twisted`, `hollow`).
    pub key: &'static str,
    /// The tile label.
    pub name: &'static str,
    /// Drag coefficient.
    pub cd: f64,
    /// Cross-wind (vortex) response factor.
    pub cross: f64,
    /// Mass factor on [`MU`].
    pub mass: f64,
    /// Cost factor.
    pub cost: f64,
    /// The tile icon: an SVG path in a 40 × 40 box.
    pub icon: &'static str,
}

const SHAPES: [ShapeDef; 4] = [
    ShapeDef {
        key: "straight",
        name: "Straight",
        cd: 1.4,
        cross: 1.0,
        mass: 1.0,
        cost: 1.0,
        icon: "M12 36V4H28V36Z",
    },
    ShapeDef {
        key: "tapered",
        name: "Tapered",
        cd: 1.25,
        cross: 0.6,
        mass: 0.75,
        cost: 0.95,
        icon: "M9 36L16 4H24L31 36Z",
    },
    ShapeDef {
        key: "twisted",
        name: "Twisted",
        cd: 1.15,
        cross: 0.45,
        mass: 1.0,
        cost: 1.18,
        icon: "M12 36C12 24 28 16 28 4M28 36C28 24 12 16 12 4M12 36H28M12 4H28",
    },
    ShapeDef {
        key: "hollow",
        name: "Hollow core",
        cd: 1.0,
        cross: 0.55,
        mass: 0.92,
        cost: 1.1,
        icon: "M12 36V4H28V36ZM16 14H24V18H16ZM16 26H24V30H16Z",
    },
];

impl Shape {
    /// All shapes in tile order.
    pub const ALL: [Shape; 4] = [
        Shape::Straight,
        Shape::Tapered,
        Shape::Twisted,
        Shape::Hollow,
    ];

    /// This shape's data.
    pub fn def(self) -> &'static ShapeDef {
        &SHAPES[self as usize]
    }

    /// The shape with the mockup key `key`.
    pub fn from_key(key: &str) -> Option<Shape> {
        Shape::ALL.into_iter().find(|s| s.def().key == key)
    }
}

// ============================================================================================
// Input and the model

/// The screen's settings (mockup `state` without the view-only `ex`, `playing`, `t`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TowerInput {
    /// Floors, 5–120.
    pub floors: u32,
    /// Structural system.
    pub system: System,
    /// Shape.
    pub shape: Shape,
    /// Wind at 10 m, m/s (5–50).
    pub v10: f64,
    /// Tuned mass damper at the top.
    pub damper: bool,
}

impl Default for TowerInput {
    /// The mockup's start: 60 floors, core, straight, 30 m/s, no damper.
    fn default() -> Self {
        Self {
            floors: 60,
            system: System::Core,
            shape: Shape::Straight,
            v10: 30.0,
            damper: false,
        }
    }
}

impl TowerInput {
    /// Floors and wind clamped to the screen's ranges (a non-finite wind becomes 30 m/s).
    pub fn sanitized(self) -> Self {
        let v10 = if self.v10.is_finite() {
            self.v10.clamp(V10_MIN, V10_MAX)
        } else {
            Self::default().v10
        };
        Self {
            floors: self.floors.clamp(FLOORS_MIN, FLOORS_MAX),
            v10,
            ..self
        }
    }

    /// Height, m.
    pub fn height(&self) -> f64 {
        FLOOR_H * self.floors as f64
    }
}

/// [`calc`]'s results (SI unless noted).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TowerResult {
    /// Height `H`, m.
    pub height: f64,
    /// Base shear `ΣF`, N.
    pub base_shear: f64,
    /// Overturning moment at the base `ΣF·z`, N·m.
    pub overturning: f64,
    /// Static top deflection `δ`, m.
    pub delta: f64,
    /// First natural frequency, Hz.
    pub f1: f64,
    /// Damping ratio used (1.5 % or 5 %).
    pub zeta: f64,
    /// Peak acceleration at the top, milli-g.
    pub mg: f64,
    /// Wind at the top, m/s.
    pub v_top: f64,
    /// Vortex shedding frequency, Hz.
    pub fs: f64,
    /// `fs / f1`.
    pub ratio: f64,
    /// Inside the lock-in band.
    pub lock: bool,
    /// Lock-in nearness 0–1 (1 at `fs = f1`).
    pub near: f64,
    /// Game cost, $M.
    pub cost: f64,
    /// Dynamic sway amplitude `a/(2πf1)²`, m (the part that swings while gusting).
    pub dyn_amp: f64,
}

impl TowerResult {
    /// Sway period `1/f1`, s.
    pub fn period(&self) -> f64 {
        1.0 / self.f1
    }

    /// Drift ratio `H/δ` (the `x` of "H/x"; larger is stiffer).
    pub fn drift_ratio(&self) -> f64 {
        self.height / self.delta
    }

    /// Drift at least `H/500`.
    pub fn sway_ok(&self) -> bool {
        self.drift_ratio() >= DRIFT_AIM
    }

    /// Top floor under 10 milli-g.
    pub fn comfort_ok(&self) -> bool {
        self.mg < COMFORT_BANDS[1]
    }

    /// Within the challenge budget.
    pub fn within_budget(&self) -> bool {
        self.cost <= BUDGET
    }
}

/// Width factor of the floor at height `z` (1 at the base; the tapered shape narrows linearly to
/// 45 % at the top).
pub fn width_factor(shape: Shape, z: f64, height: f64) -> f64 {
    if shape == Shape::Tapered {
        1.0 - 0.55 * z / height
    } else {
        1.0
    }
}

/// Mean wind speed at height `z` for the floor forces: `V10·(max(z, 10)/10)^0.2`, m/s.
pub fn wind_at(v10: f64, z: f64) -> f64 {
    v10 * (z.max(10.0) / 10.0).powf(ALPHA)
}

/// The mockup's `calc()`. Inputs are [sanitized](TowerInput::sanitized) first.
pub fn calc(input: &TowerInput) -> TowerResult {
    let input = input.sanitized();
    let sy = input.system.def();
    let sh = input.shape.def();
    let n = input.floors;
    let h = FLOOR_H * n as f64;
    let ei = EI_CORE * sy.k;
    let (mut v, mut m, mut delta) = (0.0, 0.0, 0.0);
    for i in 1..=n {
        let z = FLOOR_H * i as f64 - 2.0;
        let vz = wind_at(input.v10, z);
        // Same operation order as the mockup (left to right).
        let f = HALF_RHO * vz * vz * sh.cd * PLAN_B * width_factor(input.shape, z, h) * FLOOR_H;
        v += f;
        m += f * z;
        delta += f * z * z * (3.0 * h - z) / (6.0 * ei);
    }
    let mu = MU * sh.mass;
    let f1 = 0.5596 * (ei / (mu * h.powf(4.0))).sqrt();
    let zeta = if input.damper { ZETA_DAMPER } else { ZETA };
    let damp = (ZETA / zeta).sqrt();
    let w2 = (2.0 * PI * f1).powf(2.0);
    let mut a = w2 * 0.25 * delta * damp;
    let v_top = input.v10 * (h / 10.0).powf(ALPHA);
    let fs = STROUHAL * v_top / PLAN_B;
    let ratio = fs / f1;
    let near = (1.0 - (ratio - 1.0).abs() / LOCK_BAND).max(0.0);
    let lock = near > 0.0;
    a *= 1.0 + 3.0 * near * sh.cross * damp;
    let mg = a / G * 1000.0;
    let cost = n as f64 * sy.cost * sh.cost + if input.damper { DAMPER_COST } else { 0.0 };
    TowerResult {
        height: h,
        base_shear: v,
        overturning: m,
        delta,
        f1,
        zeta,
        mg,
        v_top,
        fs,
        ratio,
        lock,
        near,
        cost,
        dyn_amp: a / w2,
    }
}

// ============================================================================================
// Comfort, status, explain

/// Occupant comfort band (the comfort bar and its sentence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Comfort {
    /// Under 5 milli-g.
    Unnoticed,
    /// 5–10 milli-g.
    Noticed,
    /// 10–20 milli-g.
    Uneasy,
    /// 20 milli-g and more.
    Seasick,
}

impl Comfort {
    /// The band for a peak acceleration in milli-g.
    pub fn of(mg: f64) -> Comfort {
        if mg >= COMFORT_BANDS[2] {
            Comfort::Seasick
        } else if mg >= COMFORT_BANDS[1] {
            Comfort::Uneasy
        } else if mg >= COMFORT_BANDS[0] {
            Comfort::Noticed
        } else {
            Comfort::Unnoticed
        }
    }

    /// The sentence under the bar.
    pub fn sentence(self) -> &'static str {
        match self {
            Comfort::Unnoticed => "Nobody can feel it.",
            Comfort::Noticed => "Some people on the top floors notice.",
            Comfort::Uneasy => "Hanging lamps swing. People get uneasy.",
            Comfort::Seasick => "Seasick. Nobody would rent this floor.",
        }
    }

    /// The sentence's and the value's colour role.
    pub fn tone(self) -> Tone {
        match self {
            Comfort::Unnoticed => Tone::Ok,
            Comfort::Noticed => Tone::Marginal,
            Comfort::Uneasy => Tone::Caution,
            Comfort::Seasick => Tone::Alert,
        }
    }
}

/// Where the comfort bar's marker sits, percent of the bar (0–99).
pub fn comfort_pos(mg: f64) -> f64 {
    (mg / COMFORT_MAX * 100.0).min(99.0)
}

/// The status pill (TW-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    /// Drift within H/500 and under 10 milli-g.
    Comfortable,
    /// Drift worse than H/500 or 10 milli-g and more.
    TooMuchSway,
    /// Vortex shedding near the sway frequency.
    VortexLockIn,
    /// Drift worse than H/200.
    DangerouslyFlexible,
}

impl Status {
    /// The mockup's order of precedence: flexible > lock-in > too much sway > comfortable.
    pub fn of(r: &TowerResult) -> Status {
        let drift = r.drift_ratio();
        if drift < DRIFT_DANGER {
            Status::DangerouslyFlexible
        } else if r.lock {
            Status::VortexLockIn
        } else if !r.sway_ok() || r.mg >= COMFORT_BANDS[1] {
            Status::TooMuchSway
        } else {
            Status::Comfortable
        }
    }

    /// The pill's label.
    pub fn label(self) -> &'static str {
        match self {
            Status::Comfortable => "Comfortable",
            Status::TooMuchSway => "Too much sway",
            Status::VortexLockIn => "Vortex lock-in",
            Status::DangerouslyFlexible => "Dangerously flexible",
        }
    }

    /// The pill's colour role.
    pub fn tone(self) -> Tone {
        match self {
            Status::Comfortable => Tone::Ok,
            Status::TooMuchSway => Tone::Marginal,
            Status::VortexLockIn => Tone::Alert,
            Status::DangerouslyFlexible => Tone::Danger,
        }
    }
}

/// "What's happening" (the mockup's `explain`).
pub fn explain(input: &TowerInput, r: &TowerResult) -> String {
    let input = input.sanitized();
    if r.lock {
        format!(
            "Wind peels off the corners in alternating swirls (vortex shedding) about {} times a \
             second, almost exactly the tower’s own sway frequency of {} Hz. Each swirl pushes at \
             the right moment, like pushing a swing. Twist or taper the tower to confuse the \
             swirls, or add the damper.",
            fixed(r.fs, 2),
            fixed(r.f1, 2)
        )
    } else if !r.sway_ok() && input.system == System::Core {
        "A concrete core alone is a slender stick. Moving stiffness to the outside (a tube) or \
         tying the core to the outer columns (outriggers) makes the whole footprint work in \
         bending, which is how real supertall towers stay stiff."
            .to_owned()
    } else if input.damper {
        "The damper is a heavy weight hung near the top, tuned to the tower’s sway frequency. \
         When the tower leans one way the weight lags behind and pulls it back, soaking up \
         energy. It mainly cuts acceleration, not the static lean."
            .to_owned()
    } else {
        "Wind pressure grows with the square of wind speed, and wind is faster higher up, so the \
         top floors catch most of the push. Sway grows roughly with height to the fourth power: \
         add floors and watch the drift ratio fall apart."
            .to_owned()
    }
}

// ============================================================================================
// Readouts

/// One readout tile: eyebrow, value, note, value colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    /// The `.lbl` eyebrow ("Top sway").
    pub key: &'static str,
    /// The mono value ("928 mm").
    pub value: String,
    /// The muted note ("drift H/259 · aim H/500").
    pub note: String,
    /// The value's colour role.
    pub tone: Tone,
}

/// The six readout tiles (TW-6).
pub fn tiles(r: &TowerResult) -> [Tile; 6] {
    [
        Tile {
            key: "Top sway",
            value: format!("{} mm", fixed(r.delta * 1000.0, 0)),
            note: format!("drift H/{} · aim H/500", round_str(r.drift_ratio())),
            tone: if r.sway_ok() {
                Tone::Plain
            } else {
                Tone::Marginal
            },
        },
        Tile {
            key: "Sway period",
            value: format!("{} s", fixed(r.period(), 1)),
            note: format!("{} Hz", fixed(r.f1, 3)),
            tone: Tone::Plain,
        },
        Tile {
            key: "Base shear",
            value: format!("{} MN", fixed(r.base_shear / 1e6, 1)),
            note: "total wind push".to_owned(),
            tone: Tone::Plain,
        },
        Tile {
            key: "Overturning",
            value: format!("{} GN·m", fixed(r.overturning / 1e9, 2)),
            note: "goes to the foundation".to_owned(),
            tone: Tone::Plain,
        },
        Tile {
            key: "Vortex / sway",
            value: format!("{}×", fixed(r.ratio, 2)),
            note: if r.lock {
                "lock-in zone 0.75–1.25"
            } else {
                "clear of lock-in"
            }
            .to_owned(),
            tone: if r.lock { Tone::Alert } else { Tone::Plain },
        },
        Tile {
            key: "Cost",
            value: format!("${}M", fixed(r.cost, 0)),
            note: "game money".to_owned(),
            tone: if r.within_budget() {
                Tone::Plain
            } else {
                Tone::Alert
            },
        },
    ]
}

/// The comfort readout ("9.5 milli-g").
pub fn accel_label(r: &TowerResult) -> String {
    format!("{} milli-g", fixed(r.mg, 1))
}

/// The pinned chip at the top of the tower ("top 928 mm · H/259").
pub fn sway_chip(r: &TowerResult) -> String {
    format!(
        "top {} mm · H/{}",
        fixed(r.delta * 1000.0, 0),
        round_str(r.drift_ratio())
    )
}

/// The "Wind at the top" chip ("57 m/s · 127 mph").
pub fn v_top_label(r: &TowerResult) -> String {
    format!(
        "{} m/s · {} mph",
        fixed(r.v_top, 0),
        fixed(r.v_top * MPH, 0)
    )
}

/// The storm slider's readout ("30 m/s · 67 mph").
pub fn v10_label(v10: f64) -> String {
    format!("{} m/s · {} mph", fmt::num(v10), fixed(v10 * MPH, 0))
}

/// The header clock ("T = 9.9 s").
pub fn clock(r: &TowerResult) -> String {
    format!("T = {} s", fixed(r.period(), 1))
}

/// The floors well's note ("floors · 240 m").
pub fn floors_note(input: &TowerInput) -> String {
    format!("floors · {} m", fmt::num(input.sanitized().height()))
}

/// The stage's accessible summary (the mockup's `stageLabel`).
pub fn stage_label(input: &TowerInput, r: &TowerResult) -> String {
    let input = input.sanitized();
    format!(
        "{}-floor {} tower with a {} system in {} m/s wind. {}.",
        input.floors,
        input.shape.def().name.to_lowercase(),
        input.system.def().name.to_lowercase(),
        fmt::num(input.v10),
        Status::of(r).label()
    )
}

/// One bar of "Same storm, four shapes".
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShapeBar {
    /// The shape.
    pub shape: Shape,
    /// Its peak acceleration, milli-g.
    pub mg: f64,
    /// In lock-in.
    pub lock: bool,
    /// Bar width, percent of the largest (or of 1 milli-g if all are smaller).
    pub width: f64,
}

impl ShapeBar {
    /// The value ("9.5 mg").
    pub fn label(&self) -> String {
        format!("{} mg", fixed(self.mg, 1))
    }

    /// The bar's colour role: lock-in, under 10 milli-g, or over.
    pub fn tone(&self) -> Tone {
        if self.lock {
            Tone::Alert
        } else if self.mg < COMFORT_BANDS[1] {
            Tone::Ok
        } else {
            Tone::Caution
        }
    }
}

/// "Same storm, four shapes": the four shapes' peak accelerations with the same floors, system,
/// wind and damper.
pub fn compare(input: &TowerInput) -> [ShapeBar; 4] {
    let mut bars = Shape::ALL.map(|shape| {
        let r = calc(&TowerInput { shape, ..*input });
        ShapeBar {
            shape,
            mg: r.mg,
            lock: r.lock,
            width: 0.0,
        }
    });
    let max = bars.iter().map(|b| b.mg).fold(1.0_f64, f64::max);
    for b in &mut bars {
        b.width = b.mg / max * 100.0;
    }
    bars
}

/// One challenge goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    /// The goal's text.
    pub text: &'static str,
    /// Met right now.
    pub done: bool,
}

/// "Challenge · 100 floors": the four live goals.
pub fn goals(input: &TowerInput, r: &TowerResult) -> [Goal; 4] {
    let input = input.sanitized();
    let storm = input.v10 >= CHALLENGE_V10;
    [
        Goal {
            text: "Reach 100 floors",
            done: input.floors >= CHALLENGE_FLOORS,
        },
        Goal {
            text: "Drift better than H/500 at 30 m/s",
            done: storm && r.sway_ok(),
        },
        Goal {
            text: "Top floor under 10 milli-g",
            done: storm && r.comfort_ok(),
        },
        Goal {
            text: "Under $160M (game money)",
            done: r.within_budget(),
        },
    ]
}

/// The Share line (NAV-6): the settings as `.story` / `.wind` directives (Series "Netlist
/// directives": `.story n height mass system`, `.wind V10 alpha`) with the results as a comment.
pub fn share_line(input: &TowerInput, r: &TowerResult) -> String {
    let input = input.sanitized();
    let floor_mass_t = MU * input.shape.def().mass * FLOOR_H / 1000.0;
    format!(
        ".story {} {} {}t {} shape={} damper={}  .wind {} {}  ; top {} · {} · {}",
        input.floors,
        fmt::num(FLOOR_H),
        fmt::num(floor_mass_t),
        input.system.def().key,
        input.shape.def().key,
        if input.damper { "on" } else { "off" },
        fmt::num(input.v10),
        fmt::num(ALPHA),
        sway_chip(r).trim_start_matches("top "),
        clock(r),
        accel_label(r)
    )
}

// ============================================================================================
// Stage helpers (geometry in metres / factors; the screen scales to pixels)

/// The first mode's shape at `x = z/H`: `x²(3 − x)/2` (1 at the top). The static lean and the
/// dynamic sway are both drawn along it.
pub fn mode_shape(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - x) / 2.0
}

/// Plan of floor `i` (0-based) of an `n`-floor tower, as factors of the base width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloorPlan {
    /// Mid-floor height, m.
    pub z_mid: f64,
    /// Depth factor (the 2.5D side face offset follows it).
    pub w: f64,
    /// Front face width factor (the twisted shape's rotating face is narrower).
    pub front: f64,
    /// Sideways offset factor of the front face (the twist), × base width.
    pub offset: f64,
}

/// The mockup's per-floor taper / twist (`w`, `wf`, `off` in `renderVals()`).
pub fn floor_plan(shape: Shape, i: u32, n: u32) -> FloorPlan {
    let n = n.max(1);
    let h = FLOOR_H * n as f64;
    let z_mid = FLOOR_H * i as f64 + 2.0;
    let w = width_factor(shape, z_mid, h);
    let (front, offset) = if shape == Shape::Twisted {
        let th = (z_mid / h) * PI / 2.0;
        (
            w * (0.78 + 0.22 * (4.0 * th).cos()),
            w * 0.18 * (2.0 * th).sin(),
        )
    } else {
        (w, 0.0)
    };
    FloorPlan {
        z_mid,
        w,
        front,
        offset,
    }
}

/// The hollow-core shape's sky slots: floors 12–14 of every 20.
pub fn is_sky_slot(i: u32) -> bool {
    (12..=14).contains(&(i % 20))
}

/// The outriggers' floors (a third and two thirds of the way up).
pub fn is_outrigger_floor(i: u32, n: u32) -> bool {
    let third = fmt::js_round(n as f64 / 3.0) as u32;
    let two = fmt::js_round(2.0 * n as f64 / 3.0) as u32;
    i == third || i == two
}

/// Heights of the wind arrows on the stage, m: from 20 m every `max(20, round(H/70)·10)` m up to
/// the top.
pub fn arrow_heights(height: f64) -> Vec<f64> {
    let step = (fmt::js_round(height / 7.0 / 10.0) * 10.0).max(20.0);
    let mut out = Vec::new();
    let mut z = 20.0;
    while z <= height + 1.0 && out.len() < 64 {
        out.push(z);
        z += step;
    }
    out
}

/// A wind arrow's length on the stage, px at the 900-wide stage: `min(300, V(z)²/9)` with
/// `V(z) = V10·(z/10)^0.2`.
pub fn arrow_len(v10: f64, z: f64) -> f64 {
    let v = v10 * (z / 10.0).powf(ALPHA);
    (v * v / 9.0).min(300.0)
}

// ============================================================================================
// Play gusts

/// Mass ratio of the tuned mass damper to the first mode's generalised mass (2 %: tuned by
/// Den Hartog's rule it gives the first mode about 5 % equivalent damping, [`ZETA_DAMPER`]).
pub const DAMPER_MASS_RATIO: f64 = 0.02;
/// Turbulence intensity of the gusts (σᵤ / V̄): the default tower's gust peaks (≈ 3σ) come out
/// near the mockup's dynamic amplitude `dyn`.
pub const TURBULENCE: f64 = 0.08;
/// Amplitude of the vortex-shedding force at full lock-in, as a fraction of the mean wind
/// force (× the lock-in nearness × the shape's cross-wind factor). Drawn in the plane of the
/// stage for simplicity (real shedding pushes across the wind).
pub const VORTEX_FORCE: f64 = 0.06;
/// The drawn rest pose leans by this fraction of `δ` (the mockup's `delta*0.85`): the mean of the
/// gusting wind.
pub const MEAN_LEAN: f64 = 0.85;

/// The Play gusts model for this tower: the first mode (frequency `f1`, structural ζ 1.5 %) as an
/// oscillator whose mean deflection is the drawn lean `0.85·δ`, with a tuned mass damper
/// (2 % mass, Den Hartog tuning) when the damper is on, driven by gusts around the mean wind
/// and, inside the lock-in band, by vortex shedding that has locked onto the tower's own
/// frequency (that is what lock-in means: the swirls synchronise with the sway), with a
/// strength that grows with the nearness of `fs/f1` to 1.
pub fn sway_params(input: &TowerInput, r: &TowerResult, seed: u64) -> SwayParams {
    let input = input.sanitized();
    let modal_mass = MU * input.shape.def().mass * r.height * MODAL_MASS_FACTOR;
    SwayParams {
        f1: r.f1,
        zeta: ZETA,
        modal_mass,
        mass_ratio: if input.damper { DAMPER_MASS_RATIO } else { 0.0 },
        mean_top: MEAN_LEAN * r.delta,
        turbulence: TURBULENCE,
        vortex: VORTEX_FORCE * r.near * input.shape.def().cross,
        fs: if r.lock { r.f1 } else { r.fs },
        seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_mockup_numbers() {
        let input = TowerInput::default();
        let r = calc(&input);
        assert_eq!(r.height, 240.0);
        assert_eq!(fixed(r.delta * 1000.0, 0), "928");
        assert_eq!(round_str(r.drift_ratio()), "259");
        assert_eq!(fixed(r.period(), 1), "9.9");
        assert_eq!(fixed(r.f1, 3), "0.101");
        assert_eq!(fixed(r.base_shear / 1e6, 1), "19.0");
        assert_eq!(fixed(r.overturning / 1e9, 2), "2.64");
        assert_eq!(fixed(r.ratio, 2), "1.69");
        assert_eq!(fixed(r.cost, 0), "60");
        assert_eq!(fixed(r.mg, 1), "9.5");
        assert_eq!(Status::of(&r), Status::TooMuchSway);
        let bars: Vec<String> = compare(&input).iter().map(|b| b.label()).collect();
        assert_eq!(bars, ["9.5 mg", "6.6 mg", "7.8 mg", "7.4 mg"]);
    }

    #[test]
    fn keys_round_trip() {
        for s in System::ALL {
            assert_eq!(System::from_key(s.def().key), Some(s));
        }
        for s in Shape::ALL {
            assert_eq!(Shape::from_key(s.def().key), Some(s));
        }
        assert_eq!(System::from_key("nope"), None);
    }

    #[test]
    fn arrows_cover_the_height() {
        assert_eq!(
            arrow_heights(240.0),
            [20.0, 50.0, 80.0, 110.0, 140.0, 170.0, 200.0, 230.0]
        );
        assert_eq!(arrow_heights(20.0), [20.0]);
        assert!(arrow_heights(480.0).len() <= 10);
    }
}
