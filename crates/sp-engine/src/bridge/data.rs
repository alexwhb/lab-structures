//! The Bridge's fixed data: game materials and sizes, the four levels, supports and the stage
//! geometry (`Playground.dc.html` `mats()`, `sizes()`, `levels()`, `ground()`, `supOk()`,
//! `snap()`).

use crate::truss::{Fix, Material};
use serde::{Deserialize, Serialize};

// ============================================================================================
// Grid and stage

/// The snap grid: joints sit on even metres.
pub const GRID_STEP: i32 = 2;
/// Joint x range, m (`Math.max(0, Math.min(58, x))`).
pub const X_MIN: i32 = 0;
/// Joint x range, m.
pub const X_MAX: i32 = 58;
/// Joint y range, m (the riverbed is the lowest row).
pub const Y_MIN: i32 = -12;
/// Joint y range, m.
pub const Y_MAX: i32 = 16;
/// The left bank ends here, m (`x <= 8` is bank).
pub const LEFT_BANK: i32 = 8;
/// The right bank starts here, m (`x >= 48` is bank).
pub const RIGHT_BANK: i32 = 48;
/// The riverbed, m below the banks.
pub const RIVERBED: i32 = -12;

/// The stage drawing is 1120 × 600 px (aspect kept, scaled to its column).
pub const STAGE_W: f64 = 1120.0;
/// Stage height, px.
pub const STAGE_H: f64 = 600.0;
/// 1 m = 18 px.
pub const PX_PER_M: f64 = 18.0;
/// Stage x of grid x = 0, px.
pub const ORIGIN_X: f64 = 56.0;
/// Stage y of grid y = 0 (bank level), px.
pub const ORIGIN_Y: f64 = 380.0;

/// Stage x (px) of grid x (m): `56 + x·18`.
pub fn px(x: f64) -> f64 {
    ORIGIN_X + x * PX_PER_M
}

/// Stage y (px) of grid y (m): `380 − y·18`.
pub fn py(y: f64) -> f64 {
    ORIGIN_Y - y * PX_PER_M
}

/// The grid point nearest a stage position (px), clamped to the buildable range (`snap(e)`).
/// NaN snaps to 0; infinities clamp.
pub fn snap(stage_x: f64, stage_y: f64) -> (i32, i32) {
    let grid = |v: f64, lo: i32, hi: i32| {
        let g = super::fmt::js_round(v / PX_PER_M / 2.0) * 2.0;
        if g.is_nan() {
            0
        } else {
            g.clamp(lo as f64, hi as f64) as i32
        }
    };
    (
        grid(stage_x - ORIGIN_X, X_MIN, X_MAX),
        grid(ORIGIN_Y - stage_y, Y_MIN, Y_MAX),
    )
}

/// The ground height at x, m: the banks (0) at `x ≤ 8` and `x ≥ 48`, the riverbed (−12) between
/// (`ground(x)`; continuous x, also used by the collapse simulation).
pub fn ground(x: f64) -> f64 {
    if x <= LEFT_BANK as f64 || x >= RIGHT_BANK as f64 {
        0.0
    } else {
        RIVERBED as f64
    }
}

/// True if a grid point is below the ground ("That spot is underground.").
pub fn underground(x: i32, y: i32) -> bool {
    (y as f64) < ground(x as f64) - 0.01
}

/// The weights the Weight tool hangs, tonnes (chips `1 t`, `5 t`, `10 t`, `25 t`).
pub const WEIGHT_STEPS: [u32; 4] = [1, 5, 10, 25];
/// The most a joint can carry, tonnes.
pub const MAX_LOAD_T: f64 = 200.0;
/// Newtons per tonne of hung weight (`9810`).
pub const N_PER_TONNE: f64 = 9810.0;
/// Cost of a joint, $k (`st.nodes.length * 0.25`).
pub const JOINT_COST_K: f64 = 0.25;

// ============================================================================================
// Materials

/// The four materials the Bridge builds from ("Built from" chips).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MaterialKey {
    /// Steel (the default).
    #[default]
    Steel,
    /// Timber.
    Timber,
    /// Aluminum.
    Aluminum,
    /// Carbon fiber.
    CarbonFiber,
}

/// A game material: the structural constants plus a game price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameMaterial {
    /// Display name ("Steel", "Carbon fiber").
    pub name: &'static str,
    /// Young's modulus E, Pa.
    pub e: f64,
    /// Strength fy, Pa.
    pub fy: f64,
    /// Density ρ, kg/m³.
    pub rho: f64,
    /// Game price, $ per kg.
    pub cost: f64,
}

impl GameMaterial {
    /// As a [`Material`] for the truss solver.
    pub fn to_material(&self) -> Material {
        Material::new(self.name, self.e, self.fy, self.rho)
    }
}

impl MaterialKey {
    /// In the chips' order.
    pub const ALL: [MaterialKey; 4] = [
        MaterialKey::Steel,
        MaterialKey::Timber,
        MaterialKey::Aluminum,
        MaterialKey::CarbonFiber,
    ];

    /// The constants (`mats()`; representative values, game prices).
    pub fn spec(self) -> GameMaterial {
        match self {
            MaterialKey::Steel => GameMaterial {
                name: "Steel",
                e: 200e9,
                fy: 250e6,
                rho: 7850.0,
                cost: 3.5,
            },
            MaterialKey::Timber => GameMaterial {
                name: "Timber",
                e: 12e9,
                fy: 30e6,
                rho: 500.0,
                cost: 2.2,
            },
            MaterialKey::Aluminum => GameMaterial {
                name: "Aluminum",
                e: 69e9,
                fy: 240e6,
                rho: 2700.0,
                cost: 8.0,
            },
            MaterialKey::CarbonFiber => GameMaterial {
                name: "Carbon fiber",
                e: 70e9,
                fy: 600e6,
                rho: 1600.0,
                cost: 40.0,
            },
        }
    }

    /// Display name.
    pub fn name(self) -> &'static str {
        self.spec().name
    }

    /// The material whose display name is `name` (case-insensitive; also `carbon-fiber`,
    /// `CarbonFiber`, `alu`).
    pub fn from_name(name: &str) -> Option<MaterialKey> {
        let n: String = name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        match n.as_str() {
            "steel" => Some(MaterialKey::Steel),
            "timber" | "wood" | "glulam" => Some(MaterialKey::Timber),
            "aluminum" | "aluminium" | "alu" => Some(MaterialKey::Aluminum),
            "carbonfiber" | "carbonfibre" | "cf" | "carbon" => Some(MaterialKey::CarbonFiber),
            _ => None,
        }
    }
}

// ============================================================================================
// Sizes

/// Member sizes (Size chips).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SizeKey {
    /// Thin, A = 0.004 m², drawn 3 px wide.
    Thin,
    /// Medium, A = 0.008 m², 4.5 px (the default).
    #[default]
    Medium,
    /// Thick, A = 0.016 m², 6.5 px.
    Thick,
}

impl SizeKey {
    /// In the chips' order.
    pub const ALL: [SizeKey; 3] = [SizeKey::Thin, SizeKey::Medium, SizeKey::Thick];

    /// Display name ("Thin").
    pub fn name(self) -> &'static str {
        match self {
            SizeKey::Thin => "Thin",
            SizeKey::Medium => "Medium",
            SizeKey::Thick => "Thick",
        }
    }

    /// Cross-section area A, m² (`I = A²`, see [`crate::truss::Member::box_section`]).
    pub fn area(self) -> f64 {
        match self {
            SizeKey::Thin => 0.004,
            SizeKey::Medium => 0.008,
            SizeKey::Thick => 0.016,
        }
    }

    /// Stroke width on the stage, px (`stroke-width` 3 / 4.5 / 6.5).
    pub fn stroke_px(self) -> f32 {
        match self {
            SizeKey::Thin => 3.0,
            SizeKey::Medium => 4.5,
            SizeKey::Thick => 6.5,
        }
    }

    /// The size called `name` ("Thin", case-insensitive).
    pub fn from_name(name: &str) -> Option<SizeKey> {
        SizeKey::ALL
            .into_iter()
            .find(|s| s.name().eq_ignore_ascii_case(name.trim()))
    }

    /// The size whose area is nearest `area` (m²; by ratio). Non-finite → Medium.
    pub fn nearest(area: f64) -> SizeKey {
        if !(area.is_finite() && area > 0.0) {
            return SizeKey::Medium;
        }
        let mut best = SizeKey::Medium;
        let mut d = f64::INFINITY;
        for s in SizeKey::ALL {
            let e = (area / s.area()).ln().abs();
            if e < d {
                d = e;
                best = s;
            }
        }
        best
    }
}

// ============================================================================================
// Supports

/// A support on a joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Support {
    /// Held in place, free to rotate.
    Pin,
    /// Held up, free to slide sideways.
    Roller,
}

impl Support {
    /// The solver's fixity.
    pub fn fix(self) -> Fix {
        match self {
            Support::Pin => Fix::PIN,
            Support::Roller => Fix::ROLLER,
        }
    }

    /// Lower-case word used in labels ("pin support").
    pub fn word(self) -> &'static str {
        match self {
            Support::Pin => "pin",
            Support::Roller => "roller",
        }
    }

    /// Chip label ("Pin").
    pub fn label(self) -> &'static str {
        match self {
            Support::Pin => "Pin",
            Support::Roller => "Roller",
        }
    }

    /// From a solver fixity: `x+y` → pin, `y` → roller, otherwise none (a joint held only
    /// sideways has no Bridge equivalent).
    pub fn from_fix(fix: Fix) -> Option<Support> {
        match (fix.x, fix.y) {
            (true, true) => Some(Support::Pin),
            (false, true) => Some(Support::Roller),
            _ => None,
        }
    }

    /// The Support tool's cycle: none → pin → roller → none.
    pub fn cycle(cur: Option<Support>) -> Option<Support> {
        match cur {
            None => Some(Support::Pin),
            Some(Support::Pin) => Some(Support::Roller),
            Some(Support::Roller) => None,
        }
    }
}

/// Where a level allows supports (`sup` in `levels()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportRule {
    /// On either bank (`'banks'`).
    Banks,
    /// On the left bank only (`'left'`).
    LeftBank,
    /// On a bank or the riverbed (`'any'`).
    Anywhere,
}

impl SupportRule {
    /// May a support sit at (x, y)? (`supOk`).
    pub fn allows(self, x: i32, y: i32) -> bool {
        let on_bank = y == 0 && (x <= LEFT_BANK || x >= RIGHT_BANK);
        let on_bed = y == RIVERBED && x > LEFT_BANK && x < RIGHT_BANK;
        match self {
            SupportRule::LeftBank => y == 0 && x <= LEFT_BANK,
            SupportRule::Banks => on_bank,
            SupportRule::Anywhere => on_bank || on_bed,
        }
    }

    /// The toast when a support is refused.
    pub fn refusal(self) -> &'static str {
        match self {
            SupportRule::LeftBank => "Supports go on the left bank in this level.",
            SupportRule::Banks => "Supports go on a riverbank.",
            SupportRule::Anywhere => "Supports go on a bank or the riverbed.",
        }
    }
}

// ============================================================================================
// Levels

/// The four levels (aside "Level" list).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LevelKey {
    /// 1 · River crossing (the default).
    #[default]
    River,
    /// 2 · Overhang.
    Overhang,
    /// 3 · Heavy haul.
    Haul,
    /// Sandbox.
    Sandbox,
}

/// A level joint: position, support, locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelJoint {
    /// Grid x, m.
    pub x: i32,
    /// Grid y, m.
    pub y: i32,
    /// Its support.
    pub sup: Option<Support>,
    /// Part of the level: can't be moved or deleted, keeps its support and minimum load.
    pub locked: bool,
}

const fn lj(x: i32, y: i32, sup: Option<Support>, locked: bool) -> LevelJoint {
    LevelJoint { x, y, sup, locked }
}

/// One level's definition (`levels()`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Row name ("1 · River crossing").
    pub name: &'static str,
    /// The goal sentence.
    pub goal: &'static str,
    /// Budget, $k (`None` in the Sandbox: no budget, no stars).
    pub budget: Option<f64>,
    /// Where supports may go.
    pub sup: SupportRule,
    /// The level's joints (become `J1`, `J2`, … in order).
    pub nodes: &'static [LevelJoint],
    /// The level's loads: (index into `nodes`, tonnes).
    pub loads: &'static [(usize, f64)],
}

const RIVER: Level = Level {
    name: "1 · River crossing",
    goal: "Get a 20 t truck across the 40 m gap. It waits at the middle joint. Budget $30k.",
    budget: Some(30.0),
    sup: SupportRule::Banks,
    nodes: &[
        lj(8, 0, Some(Support::Pin), true),
        lj(48, 0, Some(Support::Roller), true),
        lj(28, 0, None, true),
    ],
    loads: &[(2, 20.0)],
};
const OVERHANG: Level = Level {
    name: "2 · Overhang",
    goal: "Hold 5 t out over the water, 12 m from the bank. Supports only on the left bank. Budget $14k.",
    budget: Some(14.0),
    sup: SupportRule::LeftBank,
    nodes: &[
        lj(2, 0, Some(Support::Pin), true),
        lj(8, 0, Some(Support::Pin), true),
        lj(20, 2, None, true),
    ],
    loads: &[(2, 5.0)],
};
const HAUL: Level = Level {
    name: "3 · Heavy haul",
    goal: "Two 25 t loads at once. You may build piers down to the riverbed. Budget $60k.",
    budget: Some(60.0),
    sup: SupportRule::Anywhere,
    nodes: &[
        lj(8, 0, Some(Support::Pin), true),
        lj(48, 0, Some(Support::Roller), true),
        lj(18, 0, None, true),
        lj(38, 0, None, true),
    ],
    loads: &[(2, 25.0), (3, 25.0)],
};
const SANDBOX: Level = Level {
    name: "Sandbox",
    goal: "No rules. Build anything, hang any weight on it, press Test, watch it hold or fall.",
    budget: None,
    sup: SupportRule::Anywhere,
    nodes: &[
        lj(8, 0, Some(Support::Pin), false),
        lj(48, 0, Some(Support::Roller), false),
    ],
    loads: &[],
};

impl LevelKey {
    /// In the list's order.
    pub const ALL: [LevelKey; 4] = [
        LevelKey::River,
        LevelKey::Overhang,
        LevelKey::Haul,
        LevelKey::Sandbox,
    ];

    /// The definition.
    pub fn level(self) -> &'static Level {
        match self {
            LevelKey::River => &RIVER,
            LevelKey::Overhang => &OVERHANG,
            LevelKey::Haul => &HAUL,
            LevelKey::Sandbox => &SANDBOX,
        }
    }

    /// The mockup's key (`river`, `overhang`, `haul`, `sandbox`).
    pub fn key(self) -> &'static str {
        match self {
            LevelKey::River => "river",
            LevelKey::Overhang => "overhang",
            LevelKey::Haul => "haul",
            LevelKey::Sandbox => "sandbox",
        }
    }

    /// The level with mockup key `key`, or a slug (`river-level`).
    pub fn from_key(key: &str) -> Option<LevelKey> {
        let k = key.trim().trim_end_matches(".str");
        let k = k.strip_suffix("-level").unwrap_or(k);
        LevelKey::ALL.into_iter().find(|l| l.key() == k)
    }

    /// The header's document name: `river-level`, `overhang-level`, `haul-level`, `sandbox`.
    pub fn slug(self) -> &'static str {
        match self {
            LevelKey::River => "river-level",
            LevelKey::Overhang => "overhang-level",
            LevelKey::Haul => "haul-level",
            LevelKey::Sandbox => "sandbox",
        }
    }

    /// Index into [`LevelKey::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Stars for a held design: 1 at cost ≤ budget, 2 at ≤ 75 %, 3 at ≤ 55 % (`finishHold`); 0
/// over budget or without a budget.
pub fn stars_for(cost: f64, budget: Option<f64>) -> u8 {
    let Some(b) = budget else { return 0 };
    let mut s = 0;
    if cost <= b {
        s = 1;
    }
    if cost <= b * 0.75 {
        s = 2;
    }
    if cost <= b * 0.55 {
        s = 3;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_rounds_to_the_two_metre_grid() {
        // stage px → grid: J3 of the river level sits at (56 + 28·18, 380)
        assert_eq!(snap(560.0, 380.0), (28, 0));
        assert_eq!(snap(560.0 + 17.0, 380.0 - 17.0), (28, 0));
        assert_eq!(snap(560.0 + 19.0, 380.0 - 19.0), (30, 2));
        assert_eq!(snap(-100.0, 900.0), (0, -12));
        assert_eq!(snap(5000.0, -900.0), (58, 16));
        assert_eq!(snap(f64::NAN, f64::INFINITY), (0, -12));
    }

    #[test]
    fn ground_and_supports() {
        assert_eq!(ground(8.0), 0.0);
        assert_eq!(ground(8.5), -12.0);
        assert_eq!(ground(48.0), 0.0);
        assert!(!underground(20, -2)); // in the air over the river
        assert!(!underground(20, -12));
        assert!(underground(4, -2));
        assert!(SupportRule::Banks.allows(48, 0));
        assert!(!SupportRule::Banks.allows(28, -12));
        assert!(SupportRule::Anywhere.allows(28, -12));
        assert!(!SupportRule::Anywhere.allows(8, -12));
        assert!(SupportRule::LeftBank.allows(0, 0));
        assert!(!SupportRule::LeftBank.allows(48, 0));
    }

    #[test]
    fn stars_follow_the_budget() {
        assert_eq!(stars_for(31.0, Some(30.0)), 0);
        assert_eq!(stars_for(30.0, Some(30.0)), 1);
        assert_eq!(stars_for(22.5, Some(30.0)), 2);
        assert_eq!(stars_for(16.5, Some(30.0)), 3);
        assert_eq!(stars_for(1.0, None), 0);
    }

    #[test]
    fn names_round_trip() {
        for m in MaterialKey::ALL {
            assert_eq!(MaterialKey::from_name(m.name()), Some(m));
        }
        for s in SizeKey::ALL {
            assert_eq!(SizeKey::from_name(s.name()), Some(s));
            assert_eq!(SizeKey::nearest(s.area()), s);
        }
        for l in LevelKey::ALL {
            assert_eq!(LevelKey::from_key(l.key()), Some(l));
            assert_eq!(LevelKey::from_key(l.slug()), Some(l));
        }
        assert_eq!(SizeKey::nearest(0.0061), SizeKey::Medium);
        assert_eq!(SizeKey::nearest(f64::NAN), SizeKey::Medium);
    }
}
