//! The Bridge game (Playground › Bridge, SPEC BR-1…BR-11): build a truss over the river, hang
//! the level's weights on it, press Test and watch it hold or fall.
//!
//! This is the logic of `docs/design/mockup/Playground.dc.html` (`class Component`) without
//! the drawing, reproducing its behaviour, numbers and copy:
//!
//! - [`data`] — game materials (Steel, Timber, Aluminum, Carbon fiber: E, fy, ρ, $/kg), sizes
//!   (Thin / Medium / Thick areas), the four levels, supports and their rules, the stage
//!   geometry and grid snapping, stars.
//! - [`design`] — the structure (joints `J<n>`, members `M<n>`, hung weights), its solve on
//!   [`crate::truss`] ([`Design::analyse`]: self-weight lumped half per end, weights × 9 810 N,
//!   cost) and the member colour bands, plus truss ↔ design conversion for netlists.
//! - [`game`] — the state machine: tools, placement rules and their toasts, 40-step undo, the
//!   Test ramp (λ += 1/55 per tick, up to 30 breaks per tick, loose-load and mechanism
//!   collapses, log), the held result with stars and best stars.
//! - [`sim`] — the collapse: position-based dynamics exactly as `initSim()` / `stepSim()`.
//! - [`view`] — everything the screen shows ([`Game::view`]): positions, bands, handles and
//!   joints with accessible labels, status, explain and inspector sentences, overlays, header.
//! - [`netlist`] — the structure as a `.str` netlist (Share, "Open in Advanced").
//!
//! Numbers print with [`crate::jsnum`] (JavaScript-exact `toFixed` / `Math.round` / `String`).
//!
//! ```
//! use sp_engine::bridge::{Game, Phase, Tool};
//! let mut g = Game::new(); // level 1, Member tool
//! g.click_joint("J1");     // start a member at the left bank
//! g.click_grid(18, 6);     // drop a joint and chain a member to it
//! g.click_joint("J3");     // … down to the middle joint
//! assert_eq!(g.design().members.len(), 2);
//! assert_eq!(g.view().status.label, "Wobbly: not stable");
//! g.pick_tool(Tool::Select);
//! assert_eq!(g.run_test(400), Phase::Collapsed);
//! ```

pub mod data;
pub mod design;
pub mod game;
pub mod netlist;
pub mod sim;
pub mod view;

pub use data::{LevelKey, MaterialKey, SizeKey, Support, SupportRule};
pub use design::{Analysis, Band, Bar, Design, ImportError, Joint};
pub use game::{Game, LogEntry, Phase, Selection, Tool};
pub use sim::Sim;
pub use view::{Inspector, Status, Tone, View};
