//! The Ground world (SCR-4, `docs/design/mockup/Ground.dc.html`): soils and foundations,
//! **simplified** (PROV-1: "Bearing + settlement: simplified", "Soil presets: illustrative").
//!
//! [`calc`] is the mockup's `calc(s)` operation for operation (golden test
//! `tests/ground_goldens.rs`, generated from the mockup by `tests/tower_ground_goldens.mjs`):
//!
//! - building load 12 kPa per floor over the 40 m × 40 m footprint (1600 m²);
//! - pressure on the bearing area: the load, or ÷ 0.35 on footings (they cover 35 %);
//! - capacity: the soil's allowable bearing; piles `skin friction × length` (+ 900 kPa end
//!   bearing on rock at ≥ 30 m); caissons 1800 kPa (on rock); liquefaction (quake + loose wet
//!   sand, not founded on rock) cuts it to 12 %;
//! - safety factor `capacity / pressure`;
//! - final settlement `pressure / ks` × 1 (shallow), 0.5 (friction piles), 0.12 (piles on
//!   rock), 0.08 (caissons); differential `settlement × vr` (× 0.3 deep) → tilt
//!   `atan(diff / 40 m)`, plus up to 7° as the safety factor drops under 1.6; bearing failure
//!   settles at least 400 mm, liquefaction at least 900 mm and 9°;
//! - consolidation over time `1 − e^(−t/τ)` (τ from the soil; 0.08 yr when liquefied).
//!
//! Around it: statuses, the "What's happening" sentences, the readout tiles, the safety-factor
//! bar, the scenarios, the "Challenge · The lean of Pisa" goals and the Tower hand-off note.
//! Soil swatch colours live in `sp_theme::ground` (the engine has no colours).
//!
//! Everything is total: inputs are clamped to the screen's ranges, nothing panics.

use crate::tower::Tone;
use crate::tower::fmt::{fixed, num};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Load per floor, kPa.
pub const FLOOR_LOAD: f64 = 12.0;
/// Footprint side, m.
pub const FOOTPRINT_B: f64 = 40.0;
/// Footprint area, m².
pub const FOOTPRINT_AREA: f64 = 1600.0;
/// Fraction of the footprint footings cover.
pub const FOOTING_COVER: f64 = 0.35;
/// Depth of bedrock, m.
pub const BEDROCK_DEPTH: f64 = 30.0;
/// Pile end bearing on rock, kPa.
pub const ROCK_END_BEARING: f64 = 900.0;
/// Caisson capacity, kPa.
pub const CAISSON_CAPACITY: f64 = 1800.0;
/// Liquefied capacity, fraction.
pub const LIQUEFIED_CAPACITY: f64 = 0.12;
/// Settlement of a bearing failure, at least, mm.
pub const FAILURE_SETTLEMENT: f64 = 400.0;
/// Settlement when liquefied, at least, mm.
pub const LIQUEFIED_SETTLEMENT: f64 = 900.0;
/// Tilt when liquefied, at least, degrees.
pub const LIQUEFIED_TILT: f64 = 9.0;
/// Consolidation time when liquefied, years.
pub const LIQUEFIED_TAU: f64 = 0.08;

/// Floors slider range.
pub const FLOORS_MIN: u32 = 2;
/// See [`FLOORS_MIN`].
pub const FLOORS_MAX: u32 = 80;
/// Pile length slider range, m.
pub const PILE_MIN: f64 = 8.0;
/// See [`PILE_MIN`].
pub const PILE_MAX: f64 = 40.0;
/// Years slider range (step 0.5; Play years steps 0.25 every 60 ms).
pub const YEARS_MAX: f64 = 30.0;
/// Play years: years per tick.
pub const YEARS_STEP: f64 = 0.25;
/// Play years: tick length, s.
pub const YEARS_TICK: f64 = 0.06;
/// Safety factor bar full scale ("5+").
pub const FS_MAX: f64 = 5.0;
/// Tilt above which a building is "Leaning", degrees.
pub const LEAN_TILT: f64 = 0.5;
/// Pisa's challenge: 6 floors on soft clay, under 0.5° at 30 years.
pub const PISA_FLOORS: u32 = 6;
/// "Challenge · The lean of Pisa" is rung 10 of the series' 12-rung challenge ladder.
pub const CHALLENGE_RUNG: (u32, u32) = (10, 12);

// ============================================================================================
// Soils and foundations (the mockup's `defs()`)

/// Soil preset (illustrative).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Soil {
    /// Dense sand.
    Dense,
    /// Stiff clay.
    Stiff,
    /// Soft clay (Pisa).
    #[default]
    Soft,
    /// Loose wet sand (liquefies in a quake).
    Loose,
}

/// A soil's data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilDef {
    /// The mockup's key (`dense`, `stiff`, `soft`, `loose`).
    pub key: &'static str,
    /// The tile label.
    pub name: &'static str,
    /// Allowable bearing pressure, kPa.
    pub qa: f64,
    /// Settlement stiffness, kPa per mm.
    pub ks: f64,
    /// Variability: differential / total settlement.
    pub vr: f64,
    /// Pile skin friction, kPa per metre of pile.
    pub fric: f64,
    /// Liquefies when shaken.
    pub liquefiable: bool,
    /// Consolidation time constant, years.
    pub tau: f64,
}

const SOILS: [SoilDef; 4] = [
    SoilDef {
        key: "dense",
        name: "Dense sand",
        qa: 400.0,
        ks: 8.0,
        vr: 0.1,
        fric: 25.0,
        liquefiable: false,
        tau: 0.3,
    },
    SoilDef {
        key: "stiff",
        name: "Stiff clay",
        qa: 250.0,
        ks: 4.0,
        vr: 0.15,
        fric: 18.0,
        liquefiable: false,
        tau: 3.0,
    },
    SoilDef {
        key: "soft",
        name: "Soft clay",
        qa: 60.0,
        ks: 0.6,
        vr: 0.5,
        fric: 5.0,
        liquefiable: false,
        tau: 9.0,
    },
    SoilDef {
        key: "loose",
        name: "Loose wet sand",
        qa: 100.0,
        ks: 1.5,
        vr: 0.3,
        fric: 8.0,
        liquefiable: true,
        tau: 0.5,
    },
];

impl Soil {
    /// All soils in tile order.
    pub const ALL: [Soil; 4] = [Soil::Dense, Soil::Stiff, Soil::Soft, Soil::Loose];

    /// This soil's data.
    pub fn def(self) -> &'static SoilDef {
        &SOILS[self as usize]
    }

    /// The soil with the mockup key `key`.
    pub fn from_key(key: &str) -> Option<Soil> {
        Soil::ALL.into_iter().find(|s| s.def().key == key)
    }

    /// Has a water table (drawn as a flowing dashed line).
    pub fn wet(self) -> bool {
        self == Soil::Loose
    }
}

/// Foundation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Foundation {
    /// Spread footings (cover 35 % of the footprint).
    Footing,
    /// A mat slab under the whole footprint.
    #[default]
    Mat,
    /// Piles (friction; on rock at ≥ 30 m).
    Piles,
    /// Caissons to bedrock.
    Caisson,
}

/// A foundation's data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoundationDef {
    /// The mockup's key (`footing`, `mat`, `piles`, `caisson`).
    pub key: &'static str,
    /// The tile label.
    pub name: &'static str,
    /// The tile icon: an SVG path in a 44 × 34 box (drawn under a `M2 8H42` ground line in
    /// `#6A6A6A`; stroke `#D9D9D9`, width 1.5).
    pub icon: &'static str,
}

const FOUNDATIONS: [FoundationDef; 4] = [
    FoundationDef {
        key: "footing",
        name: "Footings",
        icon: "M4 8H12V14H4ZM18 8H26V14H18ZM32 8H40V14H32Z",
    },
    FoundationDef {
        key: "mat",
        name: "Mat slab",
        icon: "M3 8H41V15H3Z",
    },
    FoundationDef {
        key: "piles",
        name: "Piles",
        icon: "M3 8H41V12H3ZM8 12V32M17 12V32M27 12V32M36 12V32",
    },
    FoundationDef {
        key: "caisson",
        name: "Caissons",
        icon: "M3 8H41V12H3ZM9 12H16V32H9ZM28 12H35V32H28Z",
    },
];

/// The ground line drawn above every foundation icon (`M2 8H42`).
pub const ICON_GROUND_LINE: &str = "M2 8H42";

impl Foundation {
    /// All foundations in tile order.
    pub const ALL: [Foundation; 4] = [
        Foundation::Footing,
        Foundation::Mat,
        Foundation::Piles,
        Foundation::Caisson,
    ];

    /// This foundation's data.
    pub fn def(self) -> &'static FoundationDef {
        &FOUNDATIONS[self as usize]
    }

    /// The foundation with the mockup key `key`.
    pub fn from_key(key: &str) -> Option<Foundation> {
        Foundation::ALL.into_iter().find(|f| f.def().key == key)
    }

    /// Piles or caissons.
    pub fn deep(self) -> bool {
        matches!(self, Foundation::Piles | Foundation::Caisson)
    }
}

// ============================================================================================
// Input and the model

/// The screen's settings (the mockup `state` without `playing`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundInput {
    /// Soil.
    pub soil: Soil,
    /// Foundation.
    pub foundation: Foundation,
    /// Building floors, 2–80.
    pub floors: u32,
    /// Pile length, m (8–40; used by piles only).
    pub pile_len: f64,
    /// Earthquake shaking.
    pub quake: bool,
    /// Years after construction, 0–30.
    pub years: f64,
    /// The leaning-tower scenario has been loaded (challenge goal 1).
    pub pisa_tried: bool,
}

impl Default for GroundInput {
    /// The mockup's start: soft clay, mat, 6 floors, 34 m piles, no quake, year 10.
    fn default() -> Self {
        Self {
            soil: Soil::Soft,
            foundation: Foundation::Mat,
            floors: 6,
            pile_len: 34.0,
            quake: false,
            years: 10.0,
            pisa_tried: false,
        }
    }
}

impl GroundInput {
    /// Floors, pile length and years clamped to the screen's ranges (non-finite values become
    /// the defaults).
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let fin = |v: f64, dv: f64| if v.is_finite() { v } else { dv };
        Self {
            floors: self.floors.clamp(FLOORS_MIN, FLOORS_MAX),
            pile_len: fin(self.pile_len, d.pile_len).clamp(PILE_MIN, PILE_MAX),
            years: fin(self.years, d.years).clamp(0.0, YEARS_MAX),
            ..self
        }
    }

    /// Founded on bedrock: caissons, or piles at least 30 m long.
    pub fn to_rock(&self) -> bool {
        self.foundation == Foundation::Caisson
            || (self.foundation == Foundation::Piles && self.pile_len >= BEDROCK_DEPTH)
    }
}

/// [`calc`]'s results (kPa, mm, degrees, years).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundResult {
    /// Building load over the footprint, kPa.
    pub load: f64,
    /// Pressure on the bearing area, kPa.
    pub pressure: f64,
    /// Capacity, kPa.
    pub capacity: f64,
    /// Safety factor `capacity / pressure`.
    pub fs: f64,
    /// Piles or caissons.
    pub deep: bool,
    /// Founded on bedrock.
    pub to_rock: bool,
    /// The sand has liquefied.
    pub liquefied: bool,
    /// Final settlement, mm.
    pub settle_final: f64,
    /// Final tilt, degrees.
    pub tilt_final: f64,
    /// Settlement at `years`, mm.
    pub settle: f64,
    /// Tilt at `years`, degrees.
    pub tilt: f64,
    /// Consolidation time constant, years.
    pub tau: f64,
}

impl GroundResult {
    /// The safety factor the status aims for: 2 deep, 3 shallow.
    pub fn target(&self) -> f64 {
        if self.deep { 2.0 } else { 3.0 }
    }

    /// Building load, MN (`load × 1600 m² / 1000`).
    pub fn building_load_mn(&self) -> f64 {
        self.load * FOOTPRINT_AREA / 1000.0
    }
}

/// The mockup's `calc()`. Inputs are [sanitized](GroundInput::sanitized) first.
pub fn calc(input: &GroundInput) -> GroundResult {
    let s = input.sanitized();
    let so = s.soil.def();
    let load = s.floors as f64 * FLOOR_LOAD;
    let deep = s.foundation.deep();
    let to_rock = s.to_rock();
    let q = if s.foundation == Foundation::Footing {
        load / FOOTING_COVER
    } else {
        load
    };
    let mut cap = so.qa;
    if s.foundation == Foundation::Piles {
        cap = so.fric * s.pile_len + if to_rock { ROCK_END_BEARING } else { 0.0 };
    }
    if s.foundation == Foundation::Caisson {
        cap = CAISSON_CAPACITY;
    }
    let liquefied = s.quake && so.liquefiable && !to_rock;
    if liquefied {
        cap *= LIQUEFIED_CAPACITY;
    }
    let fs = cap / q;
    let factor = if to_rock {
        if s.foundation == Foundation::Caisson {
            0.08
        } else {
            0.12
        }
    } else if deep {
        0.5
    } else {
        1.0
    };
    let mut settle = q / so.ks * factor;
    let diff = settle * so.vr * if deep { 0.3 } else { 1.0 };
    let mut tilt = (diff / 1000.0 / FOOTPRINT_B).atan() * 180.0 / PI;
    if fs < 1.6 {
        tilt += ((1.6 - fs) * 4.5).min(7.0);
    }
    let mut tau = so.tau;
    if liquefied {
        settle = settle.max(LIQUEFIED_SETTLEMENT);
        tilt = tilt.max(LIQUEFIED_TILT);
        tau = LIQUEFIED_TAU;
    }
    if fs < 1.0 && !liquefied {
        settle = settle.max(FAILURE_SETTLEMENT);
    }
    let k = 1.0 - (-s.years / tau).exp();
    GroundResult {
        load,
        pressure: q,
        capacity: cap,
        fs,
        deep,
        to_rock,
        liquefied,
        settle_final: settle,
        tilt_final: tilt,
        settle: settle * k,
        tilt: tilt * k,
        tau,
    }
}

// ============================================================================================
// Status, explain, readouts

/// The status pill (GD-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    /// At or above the target safety factor, not leaning.
    Solid,
    /// Under the target safety factor.
    Marginal,
    /// Tilting more than 0.5°.
    Leaning,
    /// Safety factor under 1.
    BearingFailure,
    /// The sand liquefied.
    Liquefaction,
}

impl Status {
    /// The mockup's precedence: liquefaction > bearing failure > leaning > marginal > solid.
    pub fn of(r: &GroundResult) -> Status {
        if r.liquefied {
            Status::Liquefaction
        } else if r.fs < 1.0 {
            Status::BearingFailure
        } else if r.tilt > LEAN_TILT {
            Status::Leaning
        } else if r.fs < r.target() {
            Status::Marginal
        } else {
            Status::Solid
        }
    }

    /// The pill's label.
    pub fn label(self) -> &'static str {
        match self {
            Status::Solid => "Solid",
            Status::Marginal => "Marginal",
            Status::Leaning => "Leaning",
            Status::BearingFailure => "Bearing failure",
            Status::Liquefaction => "Liquefaction",
        }
    }

    /// The pill's colour role.
    pub fn tone(self) -> Tone {
        match self {
            Status::Solid => Tone::Ok,
            Status::Marginal => Tone::Marginal,
            Status::Leaning => Tone::Caution,
            Status::BearingFailure | Status::Liquefaction => Tone::Danger,
        }
    }
}

/// "What's happening" (the mockup's `explain`).
pub fn explain(input: &GroundInput, r: &GroundResult) -> &'static str {
    if r.liquefied {
        "Shaking makes the water between the loose sand grains push them apart. For a few seconds \
         the sand behaves like a liquid, and the building sinks and tips into it. Sand boils spurt \
         up at the surface. Fix: piles or caissons that reach bedrock, which ignore the sand."
    } else if r.fs < 1.0 {
        "The pressure under the foundation is more than this soil can carry, so it shears and \
         squeezes out sideways, and the weaker side gives first. That is the Pisa story: a heavy \
         tower on a shallow foundation over soft clay. A wider mat or deep foundations fix it."
    } else if r.tilt > LEAN_TILT {
        "It stands, but the clay is not uniform, so one side settles more and the building tips \
         a little more every year. Watch the years slider: clay consolidation takes decades."
    } else if r.to_rock {
        "The load skips the soft layers and goes straight to bedrock. Settlement is tiny. This is \
         how tall towers stand on bad ground, at the price of deep and expensive foundations."
    } else if input.foundation == Foundation::Piles {
        "These piles stop short of bedrock, so they hold the building up by skin friction alone, \
         the soil gripping their sides. Longer piles grip more. Reach 30 m and they bear on rock \
         too."
    } else {
        "The foundation’s job is to spread the building’s weight until the ground can take it. \
         Pressure fans out under it in a bulb; soft soils squeeze slowly for years \
         (consolidation), stiff ones barely move."
    }
}

pub use crate::tower::Tile;

/// The six readout tiles (GD-5).
pub fn tiles(input: &GroundInput, r: &GroundResult) -> [Tile; 6] {
    let s = input.sanitized();
    let capacity_note = if r.liquefied {
        "liquefied!"
    } else if r.deep {
        if r.to_rock {
            "friction + rock"
        } else {
            "friction only"
        }
    } else {
        "allowable, illustrative"
    };
    [
        Tile {
            key: "Pressure",
            value: format!("{} kPa", fixed(r.pressure, 0)),
            note: "on the bearing area".to_owned(),
            tone: Tone::Plain,
        },
        Tile {
            key: "Capacity",
            value: format!("{} kPa", fixed(r.capacity, 0)),
            note: capacity_note.to_owned(),
            tone: if r.liquefied {
                Tone::Alert
            } else {
                Tone::Plain
            },
        },
        Tile {
            key: "Settlement",
            value: format!("{} mm", fixed(r.settle, 0)),
            note: format!("heads to {} mm", fixed(r.settle_final, 0)),
            tone: if r.settle_final > 100.0 {
                Tone::Caution
            } else {
                Tone::Plain
            },
        },
        Tile {
            key: "Tilt",
            value: format!("{}°", fixed(r.tilt, 2)),
            note: "Pisa today ≈ 4°".to_owned(),
            tone: if r.tilt > LEAN_TILT {
                Tone::Caution
            } else {
                Tone::Plain
            },
        },
        Tile {
            key: "Building load",
            value: format!("{} MN", fixed(r.building_load_mn(), 0)),
            note: format!("{} floors × 12 kPa", s.floors),
            tone: Tone::Plain,
        },
        Tile {
            key: "Consolidation",
            value: if r.tau >= 1.0 {
                format!("{} yr", fixed(r.tau, 0))
            } else {
                "fast".to_owned()
            },
            note: "time to ~63%".to_owned(),
            tone: Tone::Plain,
        },
    ]
}

/// The safety factor readout ("0.83 · target 3"; "10+" from 10).
pub fn fs_text(r: &GroundResult) -> String {
    let v = if r.fs >= 10.0 {
        "10+".to_owned()
    } else {
        fixed(r.fs, 2)
    };
    format!("{v} · target {}", num(r.target()))
}

/// The safety factor readout's colour role.
pub fn fs_tone(r: &GroundResult) -> Tone {
    if r.fs < 1.0 {
        Tone::Alert
    } else if r.fs < r.target() {
        Tone::Marginal
    } else {
        Tone::Ok
    }
}

/// Where the safety-factor bar's marker sits, percent (0–99).
pub fn fs_pos(r: &GroundResult) -> f64 {
    let p = r.fs / FS_MAX * 100.0;
    if p.is_nan() { 0.0 } else { p.min(99.0) }
}

/// The pinned stage chip ("tilt 2.37° · sank 268 mm").
pub fn stage_chip(r: &GroundResult) -> String {
    format!(
        "tilt {}° · sank {} mm",
        fixed(r.tilt, 2),
        fixed(r.settle, 0)
    )
}

/// The years slider's readout ("10.0 yr").
pub fn year_label(years: f64) -> String {
    format!("{} yr", fixed(years, 1))
}

/// The header clock ("year 10.0").
pub fn clock(years: f64) -> String {
    format!("year {}", fixed(years, 1))
}

/// The pile length slider's readout ("34 m · on rock").
pub fn pile_label(pile_len: f64) -> String {
    let rock = if pile_len >= BEDROCK_DEPTH {
        " · on rock"
    } else {
        ""
    };
    format!("{} m{rock}", num(pile_len))
}

/// The stage's accessible summary (the mockup's `stageLabel`).
pub fn stage_label(input: &GroundInput, r: &GroundResult) -> String {
    let s = input.sanitized();
    format!(
        "{}-floor building on {} in {}. {}. Tilt {} degrees.",
        s.floors,
        s.foundation.def().name.to_lowercase(),
        s.soil.def().name.to_lowercase(),
        Status::of(r).label(),
        fixed(r.tilt, 2)
    )
}

pub use crate::tower::Goal;

/// "Challenge · The lean of Pisa": the three goals (the third looks 30 years ahead).
pub fn goals(input: &GroundInput) -> [Goal; 3] {
    let s = input.sanitized();
    let tried = s.pisa_tried;
    let still = tried && s.soil == Soil::Soft && s.floors >= PISA_FLOORS;
    let fixed_30 = still
        && calc(&GroundInput {
            years: YEARS_MAX,
            ..s
        })
        .tilt
            < LEAN_TILT;
    [
        Goal {
            text: "Start from the leaning-tower scenario",
            done: tried,
        },
        Goal {
            text: "Still 6 floors on soft clay",
            done: still,
        },
        Goal {
            text: "Tilt under 0.5° at 30 years",
            done: fixed_30,
        },
    ]
}

/// The challenge's description.
pub const CHALLENGE_TEXT: &str = "Load the leaning-tower scenario, then fix it without removing \
                                  floors: tilt under 0.5° after 30 years.";

/// A "Try a scenario" preset: it sets these fields, starts at year 0 and plays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scenario {
    /// The button label.
    pub name: &'static str,
    /// Soil.
    pub soil: Soil,
    /// Foundation.
    pub foundation: Foundation,
    /// Floors.
    pub floors: u32,
    /// Pile length (kept as is when `None`).
    pub pile_len: Option<f64>,
    /// Earthquake shaking.
    pub quake: bool,
    /// Marks the Pisa challenge as tried.
    pub pisa: bool,
}

/// The four scenarios, in button order.
pub const SCENARIOS: [Scenario; 4] = [
    Scenario {
        name: "Leaning tower (soft clay, mat)",
        soil: Soil::Soft,
        foundation: Foundation::Mat,
        floors: 6,
        pile_len: None,
        quake: false,
        pisa: true,
    },
    Scenario {
        name: "Liquefaction (wet sand + quake)",
        soil: Soil::Loose,
        foundation: Foundation::Mat,
        floors: 12,
        pile_len: None,
        quake: true,
        pisa: false,
    },
    Scenario {
        name: "60 floors on piles to rock",
        soil: Soil::Stiff,
        foundation: Foundation::Piles,
        floors: 60,
        pile_len: Some(34.0),
        quake: false,
        pisa: false,
    },
    Scenario {
        name: "Friction piles in soft clay",
        soil: Soil::Soft,
        foundation: Foundation::Piles,
        floors: 10,
        pile_len: Some(20.0),
        quake: false,
        pisa: false,
    },
];

impl Scenario {
    /// `input` with this scenario applied (year 0; `pisa_tried` set by the Pisa scenario and
    /// otherwise kept).
    pub fn apply(&self, input: &GroundInput) -> GroundInput {
        GroundInput {
            soil: self.soil,
            foundation: self.foundation,
            floors: self.floors,
            pile_len: self.pile_len.unwrap_or(input.pile_len),
            quake: self.quake,
            years: 0.0,
            pisa_tried: input.pisa_tried || self.pisa,
        }
    }
}

/// Play years: the next year after one tick (0.25 yr), or `None` when it has reached 30 and
/// stops. Starting from 30 restarts at 0 ([`play_start`]).
pub fn play_tick(years: f64) -> Option<f64> {
    if !years.is_finite() || years >= YEARS_MAX {
        None
    } else {
        Some((years + YEARS_STEP).min(YEARS_MAX))
    }
}

/// Where Play years starts from: 0 if already at the end, else where it is.
pub fn play_start(years: f64) -> f64 {
    if !years.is_finite() || years >= YEARS_MAX {
        0.0
    } else {
        years.max(0.0)
    }
}

/// The building carried from the Tower (NAV-4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FromTower {
    /// The tower's floors (5–120).
    pub floors: u32,
    /// The tower's wind overturning moment at the base, N·m.
    pub overturning: f64,
}

impl FromTower {
    /// The Ground's floors for this tower (its slider stops at 80).
    pub fn ground_floors(&self) -> u32 {
        self.floors.clamp(FLOORS_MIN, FLOORS_MAX)
    }

    /// Extra pressure at the foundation's edges from the overturning moment, kPa:
    /// `M / (B³/6)` over the 40 m square footprint (simplified: a rigid base, linear pressure).
    pub fn edge_pressure(&self) -> f64 {
        let w = FOOTPRINT_B.powi(3) / 6.0;
        let p = self.overturning / w / 1000.0;
        if p.is_finite() { p.abs() } else { 0.0 }
    }

    /// The note shown on the Ground screen.
    pub fn note(&self) -> String {
        let floors = if self.floors > FLOORS_MAX {
            format!(
                "{} floors (drawn as {}, the most this pit takes)",
                self.floors,
                self.ground_floors()
            )
        } else {
            format!("{} floors", self.ground_floors())
        };
        format!(
            "From the Tower: {floors}. Its wind overturning moment, {} GN·m, adds about ±{} kPa \
             at the foundation’s edges.",
            fixed(self.overturning / 1e9, 2),
            fixed(self.edge_pressure(), 0)
        )
    }
}

/// The Share line (NAV-6): `.soil` / `.foundation` directives (Series "Netlist directives":
/// `.soil layer depth type`, `.foundation footing | mat | piles L n | caisson`) with the results
/// as a comment.
pub fn share_line(input: &GroundInput, r: &GroundResult) -> String {
    let s = input.sanitized();
    let soil = s.soil.def().name.to_lowercase().replace(' ', "-");
    let found = match s.foundation {
        Foundation::Piles => format!("piles {} 6", num(s.pile_len)),
        f => f.def().key.to_owned(),
    };
    format!(
        ".soil 1 {} {soil}  .foundation {found}  ; {} floors · quake {} · {} · FS {} · {}",
        num(BEDROCK_DEPTH),
        s.floors,
        if s.quake { "on" } else { "off" },
        clock(s.years),
        fs_text(r).split(" · ").next().unwrap_or_default(),
        stage_chip(r)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_mockup_numbers() {
        let input = GroundInput::default();
        let r = calc(&input);
        assert_eq!(fixed(r.pressure, 0), "72");
        assert_eq!(fixed(r.capacity, 0), "60");
        assert_eq!(fixed(r.settle, 0), "268");
        assert_eq!(fixed(r.settle_final, 0), "400");
        assert_eq!(fixed(r.tilt, 2), "2.37");
        assert_eq!(fixed(r.building_load_mn(), 0), "115");
        assert_eq!(fixed(r.tau, 0), "9");
        assert_eq!(fixed(r.fs, 2), "0.83");
        assert_eq!(Status::of(&r), Status::BearingFailure);
    }

    #[test]
    fn keys_round_trip() {
        for s in Soil::ALL {
            assert_eq!(Soil::from_key(s.def().key), Some(s));
        }
        for f in Foundation::ALL {
            assert_eq!(Foundation::from_key(f.def().key), Some(f));
        }
    }

    #[test]
    fn tower_note() {
        let t = FromTower {
            floors: 60,
            overturning: 2.6437e9,
        };
        assert_eq!(t.ground_floors(), 60);
        assert_eq!(fixed(t.edge_pressure(), 0), "248");
        assert!(t.note().contains("2.64 GN·m"));
        let tall = FromTower {
            floors: 100,
            overturning: 1e10,
        };
        assert_eq!(tall.ground_floors(), 80);
        assert!(tall.note().contains("100 floors"));
    }
}
