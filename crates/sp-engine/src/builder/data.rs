//! The builder's tables, exactly as the mockup's `types()`, `soils()`, `vehicles()`, `winds()`,
//! `snows()`, `quakes()`, `trafficSets()` and `levels()` (`docs/design/v2/mockup/Builder.dc.html`).
//!
//! Colours are not here: the screen maps [`PieceType`] / [`Soil`] to `sp_theme::builder`.

use std::fmt;

/// The two scenes: a river gap with banks (bridges) or flat open ground (towers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scene {
    /// Banks at x ≤ 8 and x ≥ 48 m, riverbed at −12 m, water −8…−12 m; 18 px per metre.
    River,
    /// Flat soil at y = 0; 5 px per metre.
    Flat,
}

impl Scene {
    /// The mockup's key: `river` / `flat`.
    pub fn key(self) -> &'static str {
        match self {
            Scene::River => "river",
            Scene::Flat => "flat",
        }
    }
    /// From the mockup's key.
    pub fn from_key(k: &str) -> Option<Scene> {
        match k {
            "river" => Some(Scene::River),
            "flat" => Some(Scene::Flat),
            _ => None,
        }
    }
    /// The Sandbox scene chip: "River crossing" / "Open ground".
    pub fn chip_name(self) -> &'static str {
        match self {
            Scene::River => "River crossing",
            Scene::Flat => "Open ground",
        }
    }
    /// The ground level at `x` (`ground(x)`): 0 on the banks and open ground, −12 m over the
    /// riverbed (8 < x < 48).
    pub fn ground(self, x: f64) -> f64 {
        match self {
            Scene::Flat => 0.0,
            Scene::River => {
                if x <= 8.0 || x >= 48.0 {
                    0.0
                } else {
                    -12.0
                }
            }
        }
    }
}

/// A piece type (`types()`): two decks (Road, Floor) and five structural materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PieceType {
    /// Road deck: vehicles drive on it (river scenes only).
    Road,
    /// Floor slab: counts toward height, catches the wind (open ground only).
    Floor,
    /// Wood beam.
    Wood,
    /// Steel beam (ductile).
    Steel,
    /// Concrete (cracks when pulled).
    Concrete,
    /// Rope: tension only.
    Rope,
    /// Steel cable: tension only, ductile.
    Cable,
}

/// The properties of a piece type (one row of `types()`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeSpec {
    /// "Road", "Steel", …
    pub name: &'static str,
    /// Cross-section area A, m².
    pub area: f64,
    /// Young's modulus E, Pa.
    pub e: f64,
    /// Buckling factor k: I = k·A².
    pub k: f64,
    /// Tension strength, Pa.
    pub f_t: f64,
    /// Compression strength, Pa (0 for tension-only pieces).
    pub f_c: f64,
    /// Mass per metre, kg/m, for decks (`mpm`); `None` → `area × rho`.
    pub mpm: Option<f64>,
    /// Density, kg/m³ (0 for decks, which use `mpm`).
    pub rho: f64,
    /// Cost, $ per metre.
    pub cost: f64,
    /// Longest piece, m.
    pub max_len: f64,
    /// Only pulls (Rope, Cable).
    pub t_only: bool,
    /// The palette chip's swatch bar height, px (`h`; drawn at `max(3, h)`).
    pub swatch_h: f64,
    /// Only offered in this scene (`scene`): Road on the river, Floor on open ground.
    pub scene: Option<Scene>,
    /// A deck (drawn as a 2.5D slab; snow piles on it).
    pub deck: bool,
    /// Bends before it breaks (`duct`: Steel, Cable).
    pub ductile: bool,
    /// The Materials card sentence (`blurb`).
    pub blurb: &'static str,
    /// The palette chip's tooltip (`tip`).
    pub tip: &'static str,
}

const TYPES: [TypeSpec; 7] = [
    TypeSpec {
        name: "Road",
        area: 0.02,
        e: 30e9,
        k: 2.0,
        f_t: 25e6,
        f_c: 40e6,
        mpm: Some(600.0),
        rho: 0.0,
        cost: 300.0,
        max_len: 10.0,
        t_only: false,
        swatch_h: 7.0,
        scene: Some(Scene::River),
        deck: true,
        ductile: false,
        blurb: "Vehicles drive on it. Heavy, weak when pulled.",
        tip: "Road deck: vehicles drive on this",
    },
    TypeSpec {
        name: "Floor",
        area: 0.02,
        e: 30e9,
        k: 2.0,
        f_t: 25e6,
        f_c: 40e6,
        mpm: Some(350.0),
        rho: 0.0,
        cost: 100.0,
        max_len: 10.0,
        t_only: false,
        swatch_h: 7.0,
        scene: Some(Scene::Flat),
        deck: true,
        ductile: false,
        blurb: "Gives the tower height and catches the wind.",
        tip: "Floor slab: counts toward height, carries people",
    },
    TypeSpec {
        name: "Wood",
        area: 0.02,
        e: 12e9,
        k: 1.0,
        f_t: 30e6,
        f_c: 30e6,
        mpm: None,
        rho: 550.0,
        cost: 90.0,
        max_len: 10.0,
        t_only: false,
        swatch_h: 6.0,
        scene: None,
        deck: false,
        ductile: false,
        blurb: "Cheap. Pushes and pulls, snaps without warning.",
        tip: "Wood beam: cheap, pushes and pulls",
    },
    TypeSpec {
        name: "Steel",
        area: 0.006,
        e: 200e9,
        k: 1.5,
        f_t: 250e6,
        f_c: 250e6,
        mpm: None,
        rho: 7850.0,
        cost: 260.0,
        max_len: 14.0,
        t_only: false,
        swatch_h: 4.0,
        scene: None,
        deck: false,
        ductile: true,
        blurb: "Strong and stiff both ways. Bends before it breaks.",
        tip: "Steel beam: strong both ways",
    },
    TypeSpec {
        name: "Concrete",
        area: 0.06,
        e: 30e9,
        k: 1.0,
        f_t: 2.5e6,
        f_c: 40e6,
        mpm: None,
        rho: 2400.0,
        cost: 110.0,
        max_len: 8.0,
        t_only: false,
        swatch_h: 7.0,
        scene: None,
        deck: false,
        ductile: false,
        blurb: "Great at pushing, terrible at pulling. Heavy.",
        tip: "Concrete: strong when pushed, cracks when pulled",
    },
    TypeSpec {
        name: "Rope",
        area: 0.0008,
        e: 2e9,
        k: 1.0,
        f_t: 100e6,
        f_c: 0.0,
        mpm: None,
        rho: 1100.0,
        cost: 25.0,
        max_len: 40.0,
        t_only: true,
        swatch_h: 2.0,
        scene: None,
        deck: false,
        ductile: false,
        blurb: "Only pulls. Goes slack when pushed. Weak.",
        tip: "Rope: only pulls, cheap and weak",
    },
    TypeSpec {
        name: "Cable",
        area: 0.003,
        e: 160e9,
        k: 1.0,
        f_t: 1500e6,
        f_c: 0.0,
        mpm: None,
        rho: 7850.0,
        cost: 180.0,
        max_len: 40.0,
        t_only: true,
        swatch_h: 2.0,
        scene: None,
        deck: false,
        ductile: true,
        blurb: "Only pulls. Very strong for its weight.",
        tip: "Steel cable: only pulls, very strong",
    },
];

impl PieceType {
    /// Every type, in the mockup's order (palette, inspector chips, Materials list).
    pub const ALL: [PieceType; 7] = [
        PieceType::Road,
        PieceType::Floor,
        PieceType::Wood,
        PieceType::Steel,
        PieceType::Concrete,
        PieceType::Rope,
        PieceType::Cable,
    ];
    /// The mockup's key (`road`, `steel`, …).
    pub fn key(self) -> &'static str {
        match self {
            PieceType::Road => "road",
            PieceType::Floor => "floor",
            PieceType::Wood => "wood",
            PieceType::Steel => "steel",
            PieceType::Concrete => "concrete",
            PieceType::Rope => "rope",
            PieceType::Cable => "cable",
        }
    }
    /// From the mockup's key.
    pub fn from_key(k: &str) -> Option<PieceType> {
        PieceType::ALL.into_iter().find(|t| t.key() == k)
    }
    /// From the display name ("Steel"), case-insensitive (netlists).
    pub fn from_name(name: &str) -> Option<PieceType> {
        PieceType::ALL
            .into_iter()
            .find(|t| t.name().eq_ignore_ascii_case(name.trim()))
    }
    /// Its properties.
    pub fn spec(self) -> &'static TypeSpec {
        &TYPES[self as usize]
    }
    /// "Steel".
    pub fn name(self) -> &'static str {
        self.spec().name
    }
    /// Mass per metre, kg/m (`t.mpm || t.A * t.rho`).
    pub fn mass_per_m(self) -> f64 {
        let t = self.spec();
        t.mpm.unwrap_or(t.area * t.rho)
    }
    /// Offered in this scene (`!T[k].scene || T[k].scene === L.scene`).
    pub fn allowed_in(self, scene: Scene) -> bool {
        self.spec().scene.is_none_or(|s| s == scene)
    }
    /// The types offered in a scene, in order (6 of the 7).
    pub fn for_scene(scene: Scene) -> Vec<PieceType> {
        PieceType::ALL
            .into_iter()
            .filter(|t| t.allowed_in(scene))
            .collect()
    }
    /// "Steel pieces can be at most 14 m." (the shorter toast).
    pub fn too_long(self) -> String {
        format!(
            "{} pieces can be at most {} m.",
            self.name(),
            crate::jsnum::to_string(self.spec().max_len)
        )
    }
    /// "Steel pieces can be at most 14 m. Add a joint in between." (`addMember`'s toast).
    pub fn too_long_add_joint(self) -> String {
        format!("{} Add a joint in between.", self.too_long())
    }
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A soil (`soils()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Soil {
    /// Rock.
    Rock,
    /// Dense sand.
    Dense,
    /// Stiff clay.
    Stiff,
    /// Soft clay.
    Soft,
    /// Loose wet sand (liquefies while shaking).
    Loose,
}

/// A soil's foundation properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilSpec {
    /// "Soft clay".
    pub name: &'static str,
    /// Liquefies under shaking (`liq`).
    pub liq: bool,
    /// Footing: `[push capacity N, pull capacity N, stiffness N/m]`.
    pub footing: [f64; 3],
    /// Pile: `[push capacity N, pull capacity N, stiffness N/m]`.
    pub pile: [f64; 3],
}

const SOILS: [SoilSpec; 5] = [
    SoilSpec {
        name: "Rock",
        liq: false,
        footing: [3e7, 3e6, 5e9],
        pile: [4e7, 2e7, 8e9],
    },
    SoilSpec {
        name: "Dense sand",
        liq: false,
        footing: [1.6e6, 1e5, 2e7],
        pile: [6e6, 3e6, 1e8],
    },
    SoilSpec {
        name: "Stiff clay",
        liq: false,
        footing: [1e6, 1.5e5, 1e7],
        pile: [4e6, 2.5e6, 6e7],
    },
    SoilSpec {
        name: "Soft clay",
        liq: false,
        footing: [2.5e5, 5e4, 1.5e6],
        pile: [1.6e6, 1e6, 2e7],
    },
    SoilSpec {
        name: "Loose wet sand",
        liq: true,
        footing: [5e5, 5e4, 4e6],
        pile: [2.5e6, 1.5e6, 3e7],
    },
];

impl Soil {
    /// Every soil, in the Conditions row's order.
    pub const ALL: [Soil; 5] = [
        Soil::Rock,
        Soil::Dense,
        Soil::Stiff,
        Soil::Soft,
        Soil::Loose,
    ];
    /// The mockup's key (`rock`, `dense`, `stiff`, `soft`, `loose`).
    pub fn key(self) -> &'static str {
        match self {
            Soil::Rock => "rock",
            Soil::Dense => "dense",
            Soil::Stiff => "stiff",
            Soil::Soft => "soft",
            Soil::Loose => "loose",
        }
    }
    /// From the mockup's key.
    pub fn from_key(k: &str) -> Option<Soil> {
        Soil::ALL.into_iter().find(|s| s.key() == k)
    }
    /// Its properties.
    pub fn spec(self) -> &'static SoilSpec {
        &SOILS[self as usize]
    }
    /// "Soft clay".
    pub fn name(self) -> &'static str {
        self.spec().name
    }
    /// `[push, pull, stiffness]` for a foundation kind.
    pub fn grip(self, f: Foundation) -> [f64; 3] {
        match f {
            Foundation::Footing => self.spec().footing,
            Foundation::Pile => self.spec().pile,
        }
    }
}

/// A foundation kind (Found tool).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Foundation {
    /// A concrete pad: cheap, weak when pulled ($300).
    Footing,
    /// A deep shaft: carries more, grips when pulled ($1 500; drawn 14 m deep).
    Pile,
}

impl Foundation {
    /// Both kinds, in the hint bar's order.
    pub const ALL: [Foundation; 2] = [Foundation::Footing, Foundation::Pile];
    /// `footing` / `pile` (also the word in "on a pile").
    pub fn key(self) -> &'static str {
        match self {
            Foundation::Footing => "footing",
            Foundation::Pile => "pile",
        }
    }
    /// From the key.
    pub fn from_key(k: &str) -> Option<Foundation> {
        Foundation::ALL.into_iter().find(|f| f.key() == k)
    }
    /// "Footing" / "Pile" (inspector chips).
    pub fn name(self) -> &'static str {
        match self {
            Foundation::Footing => "Footing",
            Foundation::Pile => "Pile",
        }
    }
    /// The hint bar chip: "Footing · $0.3k" / "Pile · $1.5k".
    pub fn chip(self) -> &'static str {
        match self {
            Foundation::Footing => "Footing · $0.3k",
            Foundation::Pile => "Pile · $1.5k",
        }
    }
    /// The hint bar chip's tooltip.
    pub fn tip(self) -> &'static str {
        match self {
            Foundation::Footing => "A concrete pad: cheap, weak when pulled",
            Foundation::Pile => "A deep shaft: carries more, grips when pulled",
        }
    }
    /// Cost, $.
    pub fn cost(self) -> f64 {
        match self {
            Foundation::Footing => 300.0,
            Foundation::Pile => 1500.0,
        }
    }
}

/// A vehicle kind (`vehicles()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VehicleKind {
    /// Car · 1.5 t.
    Car,
    /// Bus · 12 t.
    Bus,
    /// Truck · 30 t.
    Truck,
}

/// A vehicle's properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSpec {
    /// Mass, kg.
    pub m: f64,
    /// Length, m.
    pub len: f64,
    /// Height, m.
    pub ht: f64,
    /// Axle half-spacing, m (axles at x ± d).
    pub d: f64,
    /// Speed, m/s.
    pub v: f64,
    /// "Car · 1.5 t" (goal panel chip).
    pub label: &'static str,
}

const VEHICLES: [VehicleSpec; 3] = [
    VehicleSpec {
        m: 1500.0,
        len: 4.4,
        ht: 1.4,
        d: 1.4,
        v: 10.0,
        label: "Car · 1.5 t",
    },
    VehicleSpec {
        m: 12000.0,
        len: 11.0,
        ht: 3.0,
        d: 3.8,
        v: 8.0,
        label: "Bus · 12 t",
    },
    VehicleSpec {
        m: 30000.0,
        len: 12.0,
        ht: 3.4,
        d: 4.6,
        v: 7.0,
        label: "Truck · 30 t",
    },
];

impl VehicleKind {
    /// `car` / `bus` / `truck`.
    pub fn key(self) -> &'static str {
        match self {
            VehicleKind::Car => "car",
            VehicleKind::Bus => "bus",
            VehicleKind::Truck => "truck",
        }
    }
    /// From the key.
    pub fn from_key(k: &str) -> Option<VehicleKind> {
        [VehicleKind::Car, VehicleKind::Bus, VehicleKind::Truck]
            .into_iter()
            .find(|v| v.key() == k)
    }
    /// Its properties.
    pub fn spec(self) -> &'static VehicleSpec {
        &VEHICLES[self as usize]
    }
}

/// The traffic row (`trafficSets()`), river scenes only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TrafficSet {
    /// The level's own traffic.
    Level,
    /// Eight cars.
    Rush,
    /// Three buses.
    Buses,
    /// Two trucks.
    Trucks,
    /// Car, bus, truck, car, truck.
    Convoy,
}

impl TrafficSet {
    /// Every set, in the row's order.
    pub const ALL: [TrafficSet; 5] = [
        TrafficSet::Level,
        TrafficSet::Rush,
        TrafficSet::Buses,
        TrafficSet::Trucks,
        TrafficSet::Convoy,
    ];
    /// `level`, `rush`, `buses`, `trucks`, `convoy`.
    pub fn key(self) -> &'static str {
        match self {
            TrafficSet::Level => "level",
            TrafficSet::Rush => "rush",
            TrafficSet::Buses => "buses",
            TrafficSet::Trucks => "trucks",
            TrafficSet::Convoy => "convoy",
        }
    }
    /// From the key.
    pub fn from_key(k: &str) -> Option<TrafficSet> {
        TrafficSet::ALL.into_iter().find(|t| t.key() == k)
    }
    /// "Level traffic", "Rush hour", …
    pub fn name(self) -> &'static str {
        match self {
            TrafficSet::Level => "Level traffic",
            TrafficSet::Rush => "Rush hour",
            TrafficSet::Buses => "Buses",
            TrafficSet::Trucks => "Trucks",
            TrafficSet::Convoy => "Convoy",
        }
    }
    /// The vehicles, given the level's own list.
    pub fn vehicles(self, level: &[VehicleKind]) -> Vec<VehicleKind> {
        use VehicleKind::*;
        match self {
            TrafficSet::Level => level.to_vec(),
            TrafficSet::Rush => vec![Car; 8],
            TrafficSet::Buses => vec![Bus; 3],
            TrafficSet::Trucks => vec![Truck; 2],
            TrafficSet::Convoy => vec![Car, Bus, Truck, Car, Truck],
        }
    }
}

/// Wind chips (`winds()`): m/s and name.
pub const WINDS: [(f64, &str); 5] = [
    (0.0, "Calm"),
    (15.0, "Breezy"),
    (25.0, "Strong"),
    (35.0, "Storm"),
    (50.0, "Hurricane"),
];
/// Snow chips (`snows()`): kg/m and name.
pub const SNOWS: [(f64, &str); 4] = [
    (0.0, "None"),
    (150.0, "Light"),
    (400.0, "Heavy"),
    (800.0, "Blizzard"),
];
/// Earthquake chips (`quakes()`): ground-motion amplitude, m (BLD-12), and name.
pub const QUAKES: [(f64, &str); 4] = [
    (0.0, "Off"),
    (0.15, "Moderate"),
    (0.3, "Strong"),
    (0.5, "Huge"),
];
/// Fire chips (SPEC FIRE-1, new in the app): °C and name (0 = off).
pub const FIRES: [(f64, &str); 4] = [
    (0.0, "Off"),
    (400.0, "Small"),
    (700.0, "Big"),
    (900.0, "Inferno"),
];
/// Wind direction chips: `1` from the left, `-1` from the right.
pub const DIRS: [(f64, &str); 2] = [(1.0, "From the left →"), (-1.0, "← From the right")];
/// Crate chips (Crate tool), tonnes.
pub const WEIGHTS: [f64; 4] = [1.0, 5.0, 10.0, 25.0];
/// The heaviest crate on one joint, t.
pub const MAX_CRATE_T: f64 = 100.0;

/// Where foundations may go (`sup`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportRule {
    /// River banks only (levels 1, 2).
    Banks,
    /// Banks and riverbed (level 3, Sandbox), or anywhere on open ground.
    Any,
}

/// The level groups in the aside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    /// Levels 1–3.
    Bridges,
    /// Levels 4–7.
    Towers,
    /// The Sandbox.
    Sandbox,
}

impl Group {
    /// In the aside's order.
    pub const ALL: [Group; 3] = [Group::Bridges, Group::Towers, Group::Sandbox];
    /// "Bridges", "Towers", "Sandbox".
    pub fn name(self) -> &'static str {
        match self {
            Group::Bridges => "Bridges",
            Group::Towers => "Towers",
            Group::Sandbox => "Sandbox",
        }
    }
}

/// A level (`levels()`; Pisa comes from [`super::longterm`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LevelKey {
    /// 1 · First crossing.
    River,
    /// 2 · Bus route.
    Bus,
    /// 3 · Heavy haul.
    Haul,
    /// 4 · First tower.
    T1,
    /// 5 · Storm tower.
    T2,
    /// 6 · Shaky ground.
    T3,
    /// 7 · Leaning tower (SPEC TIME-3; its table entry is the `time` agent's).
    Pisa,
    /// The Sandbox.
    Sandbox,
}

impl LevelKey {
    /// Every key, in the table's order.
    pub const ALL: [LevelKey; 8] = [
        LevelKey::River,
        LevelKey::Bus,
        LevelKey::Haul,
        LevelKey::T1,
        LevelKey::T2,
        LevelKey::T3,
        LevelKey::Pisa,
        LevelKey::Sandbox,
    ];
    /// The mockup's key: `river`, `bus`, `haul`, `t1`, `t2`, `t3`, `pisa`, `sandbox`.
    pub fn key(self) -> &'static str {
        match self {
            LevelKey::River => "river",
            LevelKey::Bus => "bus",
            LevelKey::Haul => "haul",
            LevelKey::T1 => "t1",
            LevelKey::T2 => "t2",
            LevelKey::T3 => "t3",
            LevelKey::Pisa => "pisa",
            LevelKey::Sandbox => "sandbox",
        }
    }
    /// From the key.
    pub fn from_key(k: &str) -> Option<LevelKey> {
        LevelKey::ALL.into_iter().find(|l| l.key() == k)
    }
    /// The header's document name: `river-level`, …, `sandbox`.
    pub fn slug(self) -> String {
        match self {
            LevelKey::Sandbox => "sandbox".into(),
            k => format!("{}-level", k.key()),
        }
    }
    /// The level's table entry.
    pub fn level(self) -> Level {
        level(self)
    }
}

/// A foundation the level puts down (`anchors`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    /// x, m.
    pub x: f64,
    /// y, m.
    pub y: f64,
    /// Its kind.
    pub found: Foundation,
}

/// How a level's run works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunKind {
    /// River: traffic crosses ("Run traffic").
    Traffic,
    /// Open ground: an 18 s test ("Run the test").
    Test,
    /// The 30-year fast-forward is the run (SPEC TIME-3, Leaning tower; driven by
    /// [`super::longterm`]).
    Years,
}

/// One level of the table (`levels()[key]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    /// Its key.
    pub key: LevelKey,
    /// Its group.
    pub group: Group,
    /// "1 · First crossing".
    pub name: String,
    /// The goal sentence.
    pub goal: String,
    /// The scene (`None` for the Sandbox, whose scene is a chip).
    pub scene: Option<Scene>,
    /// The bank / ground soil.
    pub soil: Soil,
    /// The riverbed soil (`bed`), river levels and the Sandbox.
    pub bed: Option<Soil>,
    /// Wind, m/s.
    pub wind: f64,
    /// Snow, kg/m.
    pub snow: f64,
    /// Earthquake: ground-motion amplitude, m.
    pub quake: f64,
    /// Fire, °C (0 = off).
    pub fire: f64,
    /// Budget, $k (`None`: Sandbox).
    pub budget: Option<f64>,
    /// Target height, m (towers).
    pub target: Option<f64>,
    /// Comfort limit, milli-g (towers; `None`: no limit).
    pub comfort: Option<f64>,
    /// Where foundations may go.
    pub sup: SupportRule,
    /// The level's traffic (river levels).
    pub traffic: Vec<VehicleKind>,
    /// The foundations the level starts with.
    pub anchors: Vec<Anchor>,
    /// A pre-built structure the level starts with (its joints and pieces `locked`), placed
    /// after the anchors (Leaning tower). Joint ids continue after the anchors'.
    pub prebuilt: Option<super::Design>,
    /// How the run works.
    pub run: RunKind,
}

fn anchors(list: &[(f64, f64, Foundation)]) -> Vec<Anchor> {
    list.iter()
        .map(|&(x, y, found)| Anchor { x, y, found })
        .collect()
}

/// The level table in order (the aside lists them by group in this order).
pub fn levels() -> Vec<Level> {
    LevelKey::ALL
        .into_iter()
        .filter(|&k| k != LevelKey::Pisa || pisa_available())
        .map(level)
        .collect()
}

/// Whether the Leaning tower level has landed (agent `time`, [`super::longterm`]).
pub fn pisa_available() -> bool {
    true
}

/// One level's table entry.
pub fn level(key: LevelKey) -> Level {
    use Foundation::*;
    use VehicleKind::*;
    let river_anchors = anchors(&[(8.0, 0.0, Footing), (48.0, 0.0, Footing)]);
    let tower_anchors = anchors(&[(92.0, 0.0, Footing), (108.0, 0.0, Footing)]);
    let base = Level {
        key,
        group: Group::Bridges,
        name: String::new(),
        goal: String::new(),
        scene: Some(Scene::River),
        soil: Soil::Dense,
        bed: Some(Soil::Soft),
        wind: 0.0,
        snow: 0.0,
        quake: 0.0,
        fire: 0.0,
        budget: None,
        target: None,
        comfort: None,
        sup: SupportRule::Banks,
        traffic: vec![],
        anchors: river_anchors.clone(),
        prebuilt: None,
        run: RunKind::Traffic,
    };
    match key {
        LevelKey::River => Level {
            name: "1 · First crossing".into(),
            goal: "Get three cars across. Only Road carries traffic, and on its own it hangs like a chain. Hold it up. Budget $22k.".into(),
            budget: Some(22.0),
            traffic: vec![Car, Car, Car],
            ..base
        },
        LevelKey::Bus => Level {
            name: "2 · Bus route".into(),
            goal: "A 12 t bus between two cars. Piles sit back on each bank: they hold a cable\u{2019}s pull far better than a footing. Budget $30k.".into(),
            budget: Some(30.0),
            traffic: vec![Car, Bus, Car],
            anchors: anchors(&[(8.0, 0.0, Footing), (48.0, 0.0, Footing), (2.0, 0.0, Pile), (54.0, 0.0, Pile)]),
            ..base
        },
        LevelKey::Haul => Level {
            name: "3 · Heavy haul".into(),
            goal: "A 30 t truck in a snowstorm. You may found piers on the riverbed, but it is soft clay: a footing there will sink. Budget $45k.".into(),
            wind: 15.0,
            snow: 400.0,
            budget: Some(45.0),
            sup: SupportRule::Any,
            traffic: vec![Car, Truck],
            ..base
        },
        LevelKey::T1 => Level {
            group: Group::Towers,
            name: "4 · First tower".into(),
            goal: "Stack Floors up to the 40 m line and keep it standing through a breezy afternoon. Floors catch the wind. Budget $70k.".into(),
            scene: Some(Scene::Flat),
            bed: None,
            wind: 15.0,
            budget: Some(70.0),
            target: Some(40.0),
            comfort: Some(100.0),
            sup: SupportRule::Any,
            anchors: tower_anchors,
            run: RunKind::Test,
            ..base
        },
        LevelKey::T2 => Level {
            group: Group::Towers,
            name: "5 · Storm tower".into(),
            goal: "Reach 64 m and ride out a storm without making everyone seasick. A wide base, bracing and piles all help. Budget $260k.".into(),
            scene: Some(Scene::Flat),
            soil: Soil::Stiff,
            bed: None,
            wind: 35.0,
            budget: Some(260.0),
            target: Some(64.0),
            comfort: Some(300.0),
            sup: SupportRule::Any,
            anchors: tower_anchors,
            run: RunKind::Test,
            ..base
        },
        LevelKey::T3 => Level {
            group: Group::Towers,
            name: "6 · Shaky ground".into(),
            goal: "A 32 m tower on loose wet sand, then a strong earthquake. Shaking turns the sand to soup under footings. Budget $65k.".into(),
            scene: Some(Scene::Flat),
            soil: Soil::Loose,
            bed: None,
            quake: 0.3,
            budget: Some(65.0),
            target: Some(32.0),
            sup: SupportRule::Any,
            anchors: tower_anchors,
            run: RunKind::Test,
            ..base
        },
        // SPEC TIME-3: the `time` agent's table entry.
        LevelKey::Pisa => super::longterm::pisa::level(),
        LevelKey::Sandbox => Level {
            group: Group::Sandbox,
            name: "Sandbox".into(),
            goal: "No budget, no rules, no time limit. Build anything, change the weather while it runs, and stop when you like.".into(),
            scene: None,
            bed: Some(Soil::Dense),
            sup: SupportRule::Any,
            anchors: vec![],
            ..base
        },
    }
}

/// The stars a passing run earns: 1 at ≤ budget, 2 at ≤ 80 %, 3 at ≤ 60 %.
pub fn stars_for(cost: f64, budget: f64) -> u8 {
    let mut s = 0;
    if cost <= budget {
        s = 1;
    }
    if cost <= budget * 0.8 {
        s = 2;
    }
    if cost <= budget * 0.6 {
        s = 3;
    }
    s
}

/// The star row: `★★☆` for 2 (`starsTxt`).
pub fn stars_text(n: u8) -> String {
    let n = n.min(3) as usize;
    "★".repeat(n) + &"☆".repeat(3 - n)
}
