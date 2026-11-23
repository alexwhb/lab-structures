//! The conditions (`state.cond`) and the level as it runs with them (`spec()` + `L()`), plus
//! the placement rule (`supOk`), the soil under a joint (`soilAt`) and practice runs
//! (`practice()`).

use super::data::{
    Anchor, Foundation, Group, Level, LevelKey, RunKind, Scene, Soil, SupportRule, TrafficSet,
    VehicleKind,
};
use super::design::Joint;

/// The Conditions panel (`state.cond`, plus `fire`). Wind, snow, quake and fire may change
/// while running; soil and traffic only while building.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conditions {
    /// Wind speed at 10 m, m/s.
    pub wind: f64,
    /// Wind direction: `1` from the left, `-1` from the right.
    pub dir: f64,
    /// Snow, kg per metre of exposed deck.
    pub snow: f64,
    /// Earthquake peak ground motion, g (0 = off).
    pub quake: f64,
    /// The soil (the riverbed follows the level's own bed while this is the level's soil).
    pub soil: Soil,
    /// The traffic set (river scenes).
    pub traffic: TrafficSet,
    /// Fire temperature, °C (0 = off; SPEC FIRE-1, agent `lab`).
    pub fire: f64,
}

impl Conditions {
    /// A level's own conditions (`levelState().cond`).
    pub fn for_level(level: &Level) -> Conditions {
        Conditions {
            wind: level.wind,
            dir: 1.0,
            snow: level.snow,
            quake: level.quake,
            soil: level.soil,
            traffic: TrafficSet::Level,
            fire: level.fire,
        }
    }
}

/// A level with the Sandbox scene applied (`spec(st)`): the Sandbox gets the scene's traffic
/// (car, bus, truck on the river) and anchors, no target and no comfort limit.
pub fn spec(key: LevelKey, sb_scene: Scene) -> Level {
    let mut base = key.level();
    if key != LevelKey::Sandbox {
        return base;
    }
    let river = sb_scene == Scene::River;
    base.scene = Some(sb_scene);
    base.traffic = if river {
        vec![VehicleKind::Car, VehicleKind::Bus, VehicleKind::Truck]
    } else {
        vec![]
    };
    base.target = None;
    base.comfort = None;
    base.run = if river {
        RunKind::Traffic
    } else {
        RunKind::Test
    };
    let a = if river {
        [(8.0, 0.0), (48.0, 0.0)]
    } else {
        [(92.0, 0.0), (108.0, 0.0)]
    };
    base.anchors = a
        .iter()
        .map(|&(x, y)| Anchor {
            x,
            y,
            found: Foundation::Footing,
        })
        .collect();
    base
}

/// The level as it runs with the conditions (`L(st)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    /// The level.
    pub key: LevelKey,
    /// The Sandbox (no budget, never ends, vehicles loop).
    pub sandbox: bool,
    /// Its group.
    pub group: Group,
    /// "1 · First crossing".
    pub name: String,
    /// The goal sentence.
    pub goal: String,
    /// The scene.
    pub scene: Scene,
    /// The bank / ground soil (the condition's).
    pub soil: Soil,
    /// The riverbed soil: the level's bed while the soil is the level's, else the soil.
    pub bed: Soil,
    /// Wind at 10 m, m/s (the condition's, as at the start of the run).
    pub wind: f64,
    /// Wind direction.
    pub dir: f64,
    /// Snow, kg/m.
    pub snow: f64,
    /// Quake, g.
    pub quake: f64,
    /// Fire, °C.
    pub fire: f64,
    /// Budget, $k.
    pub budget: Option<f64>,
    /// Target height, m.
    pub target: Option<f64>,
    /// Comfort limit, milli-g.
    pub comfort: Option<f64>,
    /// Where foundations may go.
    pub sup: SupportRule,
    /// The traffic that will run (empty on open ground).
    pub traffic: Vec<VehicleKind>,
    /// The level's anchors.
    pub anchors: Vec<Anchor>,
    /// How the run works.
    pub run: RunKind,
}

impl Setup {
    /// `L(st)`: the level (with the Sandbox scene) and the conditions.
    pub fn new(key: LevelKey, sb_scene: Scene, cond: &Conditions) -> Setup {
        let sp = spec(key, sb_scene);
        let scene = sp.scene.unwrap_or(sb_scene);
        let bed = if cond.soil == sp.soil {
            sp.bed.unwrap_or(sp.soil)
        } else {
            cond.soil
        };
        let traffic = if scene == Scene::River {
            cond.traffic.vehicles(&sp.traffic)
        } else {
            vec![]
        };
        Setup {
            key,
            sandbox: key == LevelKey::Sandbox,
            group: sp.group,
            name: sp.name,
            goal: sp.goal,
            scene,
            soil: cond.soil,
            bed,
            wind: cond.wind,
            dir: cond.dir,
            snow: cond.snow,
            quake: cond.quake,
            fire: cond.fire,
            budget: sp.budget,
            target: sp.target,
            comfort: sp.comfort,
            sup: sp.sup,
            traffic,
            anchors: sp.anchors,
            run: sp.run,
        }
    }
    /// Ground level at x (`ground(x, L)`).
    pub fn ground(&self, x: f64) -> f64 {
        self.scene.ground(x)
    }
    /// Whether a foundation may go at (x, y) (`supOk`): open ground at y = 0; river banks at
    /// y = 0 (x ≤ 8 or x ≥ 48), and in levels that allow it the riverbed at y = −12.
    pub fn sup_ok(&self, x: f64, y: f64) -> bool {
        if self.scene == Scene::Flat {
            return y.abs() < 0.01;
        }
        let on_bank = y.abs() < 0.01 && (x <= 8.0 || x >= 48.0);
        let on_bed = (y + 12.0).abs() < 0.01 && x > 8.0 && x < 48.0;
        match self.sup {
            SupportRule::Banks => on_bank,
            SupportRule::Any => on_bank || on_bed,
        }
    }
    /// The soil under a joint (`soilAt`): the riverbed's below −11 m on the river.
    pub fn soil_at(&self, n: &Joint) -> Soil {
        if self.scene == Scene::River && n.y < -11.0 {
            self.bed
        } else {
            self.soil
        }
    }
    /// The refusal when a foundation can't go somewhere (`setFound`).
    pub fn sup_refusal(&self) -> &'static str {
        if self.scene == Scene::Flat {
            "Foundations go on the ground."
        } else if self.sup == SupportRule::Banks {
            "Foundations go on a riverbank in this level."
        } else {
            "Foundations go on a bank or the riverbed."
        }
    }
}

/// A practice run (`practice()`): any condition easier than the level's (less wind, snow,
/// quake or fire, another soil, other traffic). Never in the Sandbox.
pub fn practice(key: LevelKey, sb_scene: Scene, cond: &Conditions) -> bool {
    if key == LevelKey::Sandbox {
        return false;
    }
    let sp = spec(key, sb_scene);
    cond.wind < sp.wind
        || cond.snow < sp.snow
        || cond.quake < sp.quake
        || cond.fire < sp.fire
        || cond.soil != sp.soil
        || cond.traffic != TrafficSet::Level
}
