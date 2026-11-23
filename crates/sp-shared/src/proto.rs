//! The Structures bridge methods (beyond `status` / `navigate` / `screenshot` / `describe`,
//! which every lab-series app answers; see `lab_mcp::bridge::proto`) and their payloads: the
//! v2 builder (SPEC MCP-2).
//!
//! | method | params | result |
//! |---|---|---|
//! | [`BUILDER_GET`] | none | [`BuilderState`] |
//! | [`BUILDER_LEVEL`] | [`BuilderLevel`] | [`BuilderState`] |
//! | [`BUILDER_LOAD_NETLIST`] | [`BuilderLoadNetlist`] | [`BuilderState`] |
//! | [`BUILDER_CONDITIONS`] | [`BuilderConditions`] | [`BuilderState`] |
//! | [`BUILDER_RUN`] | [`BuilderRun`] | [`RunReport`] |
//! | [`BUILDER_FAST_FORWARD`] | [`BuilderFastForward`] | [`RunReport`] |
//! | [`SAMPLE_TEST`] | [`SampleTest`] | [`SampleReport`] |
//!
//! Methods that change the builder act on it whether or not it is showing; with `show` (the
//! default) the app also switches to the Playground so the person at the keyboard sees the
//! change. A bad argument (a netlist that doesn't parse or doesn't fit the builder's grid, an
//! unknown level or soil, a wind out of range) is an `INVALID_PARAMS` error reply whose message
//! says what to send instead, and nothing changes. The v1 `bridge.*`, `tower.set`, `ground.set`
//! and `materials.set` methods are retired.

pub use crate::report::{BuilderState, RunReport, SampleReport};
use serde::{Deserialize, Serialize};

/// The builder as it stands.
pub const BUILDER_GET: &str = "builder.get";
/// Picks a level, or the Sandbox and its scene (resets to the level's anchors and conditions).
pub const BUILDER_LEVEL: &str = "builder.level";
/// Loads a netlist into the Sandbox (as pasting one, or Advanced's "Open in Playground").
pub const BUILDER_LOAD_NETLIST: &str = "builder.load_netlist";
/// Changes the conditions (wind, direction, snow, quake, soil, traffic, fire).
pub const BUILDER_CONDITIONS: &str = "builder.conditions";
/// Runs the structure to the end and reports the result.
pub const BUILDER_RUN: &str = "builder.run";
/// Fast-forwards the structure 30 years (TIME-1) and reports the settlement.
pub const BUILDER_FAST_FORWARD: &str = "builder.fast_forward";
/// Tests a sample of a piece type in the sample lab (LAB-1).
pub const SAMPLE_TEST: &str = "sample.test";

/// Every Structures method, in table order.
pub const METHODS: [&str; 7] = [
    BUILDER_GET,
    BUILDER_LEVEL,
    BUILDER_LOAD_NETLIST,
    BUILDER_CONDITIONS,
    BUILDER_RUN,
    BUILDER_FAST_FORWARD,
    SAMPLE_TEST,
];

/// The most 1/30 s frames a run may take headless (open ground runs 541 frames, river runs up
/// to 1351 — 45 s; this leaves a wide margin and bounds a Sandbox run, which never ends).
pub const MAX_RUN_FRAMES: usize = 3_000;

/// The Sandbox runs this long headless when asked to (it loops until stopped in the app), s.
pub const SANDBOX_RUN_SECONDS: f64 = 20.0;

fn yes() -> bool {
    true
}

/// Params of [`BUILDER_LEVEL`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderLevel {
    /// `river`, `bus`, `haul` (bridges), `t1`, `t2`, `t3`, `pisa` (towers) or `sandbox`.
    pub level: String,
    /// The Sandbox's scene: `river` (River crossing) or `flat` (Open ground). Sandbox only.
    #[serde(default)]
    pub scene: Option<String>,
    /// Switch to the Playground (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`BUILDER_LOAD_NETLIST`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderLoadNetlist {
    /// A `.str` netlist whose nodes fit the builder's grid (the river scene's 2 m grid when they
    /// sit on it, else open ground's 4 m grid), like the one the builder's Share copies.
    pub netlist: String,
    /// Switch to the Playground (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// The conditions a call may set (unset ones stay as they are). Shared by
/// [`BuilderConditions`] and the engine tool `builder_test`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionsChange {
    /// Wind at 10 m, m/s (0–50; the chips are Calm 0, Breezy 15, Strong 25, Storm 35,
    /// Hurricane 50).
    #[serde(default)]
    pub wind_ms: Option<f64>,
    /// Where the wind comes from: `left` (blowing →) or `right` (←).
    #[serde(default)]
    pub wind_from: Option<String>,
    /// Snow on exposed decks, kg/m (0–800; None 0, Light 150, Heavy 400, Blizzard 800).
    #[serde(default)]
    pub snow_kg_m: Option<f64>,
    /// Earthquake peak ground motion, g (0–0.5; Off, Moderate 0.15, Strong 0.3, Huge 0.5).
    #[serde(default)]
    pub quake_g: Option<f64>,
    /// `rock`, `dense`, `stiff`, `soft` or `loose` (loose wet sand). Building only.
    #[serde(default)]
    pub soil: Option<String>,
    /// River scenes: `level`, `rush`, `buses`, `trucks` or `convoy`. Building only.
    #[serde(default)]
    pub traffic: Option<String>,
    /// Fire, °C (0 off, 400 small, 700 big, 900 inferno; FIRE-1).
    #[serde(default)]
    pub fire_c: Option<f64>,
}

impl ConditionsChange {
    /// Nothing to change.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Params of [`BUILDER_CONDITIONS`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuilderConditions {
    /// What to change.
    #[serde(flatten)]
    pub change: ConditionsChange,
    /// Switch to the Playground (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`BUILDER_RUN`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderRun {
    /// Show the run in the app afterwards: its last frame and the result banner (default
    /// `true`); the run itself is computed at once.
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`BUILDER_FAST_FORWARD`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderFastForward {
    /// How many years (0–30, default 30).
    #[serde(default)]
    pub years: Option<f64>,
    /// Switch to the Playground (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`SAMPLE_TEST`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleTest {
    /// The piece type: `road`, `floor`, `wood`, `steel`, `concrete`, `rope`, `cable`.
    pub piece: String,
    /// `pull` (default) until it breaks, or `push` until it crushes or buckles.
    #[serde(default)]
    pub mode: Option<String>,
    /// Temperature, °C (−40 to 900; default 20).
    #[serde(default)]
    pub temperature_c: Option<f64>,
    /// Open the sample drawer in the app (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn params_default_and_refuse_unknown_fields() {
        let p: BuilderLoadNetlist = serde_json::from_value(json!({"netlist": ".end"})).unwrap();
        assert!(p.show);
        assert!(serde_json::from_value::<BuilderRun>(json!({"nope": 1})).is_err());
        let l: BuilderLevel =
            serde_json::from_value(json!({"level": "sandbox", "scene": "flat"})).unwrap();
        assert_eq!(l.scene.as_deref(), Some("flat"));
        let c: BuilderConditions =
            serde_json::from_value(json!({"wind_ms": 35, "show": false})).unwrap();
        assert_eq!(c.change.wind_ms, Some(35.0));
        assert!(!c.show);
        let e: BuilderConditions = serde_json::from_value(json!({})).unwrap();
        assert!(e.change.is_empty() && e.show);
        assert!(serde_json::from_value::<ConditionsChange>(json!({"rain": 3})).is_err());
        let s: SampleTest = serde_json::from_value(json!({"piece": "steel"})).unwrap();
        assert!(s.mode.is_none() && s.show);
        assert_eq!(METHODS.len(), 7);
    }
}
