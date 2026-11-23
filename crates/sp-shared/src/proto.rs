//! The Structures bridge methods (beyond `status` / `navigate` / `screenshot` / `describe`,
//! which every lab-series app answers; see `lab_mcp::bridge::proto`) and their payloads.
//!
//! | method | params | result |
//! |---|---|---|
//! | [`BRIDGE_GET`] | none | [`BridgeState`] |
//! | [`BRIDGE_LOAD_NETLIST`] | [`BridgeLoadNetlist`] | [`BridgeState`] |
//! | [`BRIDGE_TEST`] | [`BridgeTest`] | [`BridgeTestResult`] |
//! | [`TOWER_SET`] | [`TowerSet`] | [`TowerReport`] |
//! | [`GROUND_SET`] | [`GroundSet`] | [`GroundReport`] |
//! | [`MATERIALS_SET`] | [`MaterialsSet`] | [`MaterialsState`] |
//!
//! Methods that change a screen act on its state whether or not it is showing; with `show`
//! (the default) the app also switches to that screen so the person at the keyboard sees the
//! change. A bad argument (a netlist that doesn't parse or doesn't fit the grid, a floor count
//! out of range, an unknown soil) is an `INVALID_PARAMS` error reply whose message says what to
//! send instead, and nothing changes.

pub use crate::input::{GroundChange, TowerChange};
pub use crate::report::{
    BridgeState, BridgeTestResult, FailureInfo, GroundReport, TensileReport, TowerReport,
};
use serde::{Deserialize, Serialize};

/// The Bridge game's state.
pub const BRIDGE_GET: &str = "bridge.get";
/// Loads a netlist into the Bridge's Sandbox (it must fit the 2 m grid above ground).
pub const BRIDGE_LOAD_NETLIST: &str = "bridge.load_netlist";
/// Runs the Bridge's Test to the end and reports it.
pub const BRIDGE_TEST: &str = "bridge.test";
/// Changes (or reads) the Tower.
pub const TOWER_SET: &str = "tower.set";
/// Changes (or reads) the Ground.
pub const GROUND_SET: &str = "ground.set";
/// Changes (or reads) the Materials bench.
pub const MATERIALS_SET: &str = "materials.set";

/// Every Structures method, in table order.
pub const METHODS: [&str; 6] = [
    BRIDGE_GET,
    BRIDGE_LOAD_NETLIST,
    BRIDGE_TEST,
    TOWER_SET,
    GROUND_SET,
    MATERIALS_SET,
];

/// The most 33 ms ticks a Test may run (the slowest Test ramps 55 ticks and collapses in
/// about 100 more; this leaves a wide margin).
pub const MAX_TEST_TICKS: usize = 5_000;

fn yes() -> bool {
    true
}

/// Params of [`BRIDGE_LOAD_NETLIST`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeLoadNetlist {
    /// A `.str` netlist whose nodes sit on the 2 m grid (x 0–58 m, y −12–16 m, not
    /// underground), like the one the Bridge's Share copies.
    pub netlist: String,
    /// Switch to the Bridge (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`BRIDGE_TEST`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeTest {
    /// Pick this level first (`river`, `overhang`, `haul`, `sandbox`): the level's joints
    /// only, so test an empty level to see it fail, or load a netlist first.
    #[serde(default)]
    pub level: Option<String>,
    /// Switch to the Bridge (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`TOWER_SET`]: the changes (none: just read it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerSet {
    /// What to change.
    #[serde(flatten)]
    pub change: TowerChange,
    /// Switch to the Tower (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`GROUND_SET`]: the changes (none: just read it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundSet {
    /// What to change.
    #[serde(flatten)]
    pub change: GroundChange,
    /// Switch to the Ground (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`MATERIALS_SET`]; they apply in this order: `material`, `temperature_c`,
/// `fresh`, `strain_pct`, `pull`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialsSet {
    /// Pick a sample: `steel`, `aluminum`, `concrete`, `timber`, `carbon-fiber` (a fresh
    /// sample on the axis that fits it).
    #[serde(default)]
    pub material: Option<String>,
    /// Temperature, °C (−40 to 600).
    #[serde(default)]
    pub temperature_c: Option<f64>,
    /// A fresh, unstretched sample.
    #[serde(default)]
    pub fresh: bool,
    /// Stretch it to this, % (0 to the axis end: 25, or 2 on the close-up axis).
    #[serde(default)]
    pub strain_pct: Option<f64>,
    /// Pull until it breaks or reaches the axis end (as pressing Pull and waiting).
    #[serde(default)]
    pub pull: bool,
    /// Switch to Materials (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// The Materials bench after [`MATERIALS_SET`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialsState {
    /// The test as it stands (with the whole curve).
    pub test: TensileReport,
    /// The strain axis: `0–25 %` or `0–2 %`.
    pub axis: String,
    /// Samples broken so far ("Challenge · Break all five").
    pub broken_so_far: Vec<String>,
    /// What the call did, when worth saying ("Pulled until it broke at 22 %").
    pub note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn params_default_and_refuse_unknown_fields() {
        let p: BridgeLoadNetlist = serde_json::from_value(json!({"netlist": ".end"})).unwrap();
        assert!(p.show);
        assert!(serde_json::from_value::<BridgeTest>(json!({"nope": 1})).is_err());
        let t: TowerSet = serde_json::from_value(json!({"floors": 80, "show": false})).unwrap();
        assert_eq!(t.change.floors, Some(80));
        assert!(!t.show);
        let g: GroundSet = serde_json::from_value(json!({})).unwrap();
        assert!(g.change.is_empty() && g.show);
        let m: MaterialsSet =
            serde_json::from_value(json!({"material": "timber", "pull": true})).unwrap();
        assert!(m.pull && m.show && !m.fresh);
        assert_eq!(METHODS.len(), 6);
    }
}
