//! The v2 builder (SPEC "Builder", BLD-*): one settling engine for bridges and towers — pieces,
//! foundations on soil, crates, vehicles, wind, snow, earthquakes — and the game around it
//! (levels, editing, the camera, results, every sentence). A port of
//! `docs/design/v2/mockup/Builder.dc.html`'s `class Component`, checked against the mockup's own
//! JS run in Node (`tests/builder_goldens.{mjs,rs}`).
//!
//! - [`data`] — the tables: piece types, soils, foundations, vehicles, the condition chips, the
//!   levels ([`data::levels`]).
//! - [`design`] — the structure: joints, pieces, crates, cost.
//! - [`setup`] — the conditions and the level as it runs with them (`L()`), placement rules,
//!   practice runs.
//! - [`camera`] — world metres ↔ stage pixels, snapping, zoom, fit.
//! - [`sim`] — the settling engine (XPBD) with the fold-in hooks.
//! - [`preview`] — the live preview and the static settle with settlement offsets.
//! - [`game`] — the state machine: tools, editing rules and toasts, undo, runs, results.
//! - [`view`] — everything the screen draws ([`Game::view`]).
//! - [`jsmath`] — V8's `Math.sin` / `cos` / `atan2` / `pow` bit for bit.
//! - [`netlist`] — the design as a `.str` netlist and back (Share, "Open in Advanced", ⌘V).
//!
//! Owners (PROGRESS.md): the core is agent `engine`; [`comfort`] and [`longterm`] are agent
//! `time`; [`sample`] and [`fire`] are agent `lab`.
//!
//! ```
//! use sp_engine::builder::{Game, Tool, PieceType};
//! let mut g = Game::new(); // 1 · First crossing, Build tool, Road
//! g.click_node("J1"); // start a chain at the left bank
//! g.click_at([16.0, 0.0]); // drop a joint 8 m out and lay Road to it
//! assert_eq!(g.design().members.len(), 1);
//! assert_eq!(g.design().members[0].kind, PieceType::Road);
//! g.pick_tool(Tool::Select);
//! assert_eq!(g.view().status.label, "Sags and folds");
//! ```

pub mod camera;
pub mod data;
pub mod design;
pub mod game;
pub mod jsmath;
pub mod netlist;
pub mod preview;
pub mod setup;
pub mod sim;
pub mod view;

pub mod comfort;
pub mod fire;
pub mod longterm;
pub mod sample;

pub use camera::{Camera, ViewState};
pub use data::{
    Anchor, Foundation, Group, Level, LevelKey, PieceType, RunKind, Scene, Soil, SupportRule,
    TrafficSet, VehicleKind,
};
pub use design::{Design, Joint, Piece};
pub use game::{
    Game, Outcome, Phase, Preset, PresetBuild, RunResult, Selection, Tool, Years, result_of,
    run_headless,
};
pub use preview::{FoundForce, PieceForce, SettleOptions, Settled, preview, settle};
pub use setup::{Conditions, Setup};
pub use sim::{
    BreakMode, Constraint, FirstBreak, FirstFound, FoundMode, Particle, Sim, Vehicle, VehicleState,
};
pub use view::{Band, Inspector, View};
