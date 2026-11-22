//! The Materials lab: a tensile test on five representative materials (SCR-5, SPEC MT-1…MT-6).
//!
//! The model is the Materials mockup's (`docs/design/mockup/Materials.dc.html`: `defs()`,
//! `tempFactor()`, `stressAt()`, `tick()`, `renderVals()`), reproduced number for number:
//!
//! - [`curves`] — the five **representative** stress–strain curves (piecewise linear, strain in
//!   %, stress in MPa), their stiffness, density, notes, the **illustrative** temperature
//!   strength factors, [`curves::stress_at`] (fracture past the last point), necking after the
//!   ultimate strain, toughness (area under the curve), yield, peak and specific strength, and
//!   the load cell force for the 50 mm² section.
//! - [`bench`] — the test bench state machine: the selected sample, temperature, strain axis
//!   zoom, the Pull stepping (0.08 %/tick on the 25 % axis, 0.006 on the 2 % axis, 20 ticks/s),
//!   Fresh sample, Reset, and the "Break all five" challenge; status and explain sentences and
//!   the Share line.
//! - [`view`] — everything the screen shows for a bench state (`renderVals()`): the readout
//!   strings formatted exactly like the mockup's JavaScript, and the drawing model of the rig
//!   and the plot in the mockup's SVG viewBox units.
//! - [`jsnum`] — JavaScript-compatible number formatting (`Number.prototype.toFixed`,
//!   `String(number)`, `Math.round`), so every readout matches the mockup to the character.
//!
//! Provenance (PROV-1, `docs/DATA-SOURCES.md`): the curves are *representative* (textbook-typical
//! shapes and values, not certified data), the heat effects *illustrative*.
//!
//! ```
//! use sp_engine::materials::{Bench, Kind};
//! let mut bench = Bench::default(); // steel, 20 °C, 0–25 % axis
//! bench.toggle_drive(); // Pull
//! while bench.playing() {
//!     bench.tick();
//! }
//! assert!(bench.broke(Kind::Steel));
//! assert_eq!(bench.view().status_label, "Fractured at 22%");
//! ```

pub mod bench;
pub mod curves;
pub mod jsnum;
pub mod view;

pub use bench::{Bench, Status, TICK_SECONDS, Tone, Zoom};
pub use curves::{CURVES, Curve, Kind, Stress, stress_at, temp_factor};
pub use view::{PlotGeometry, RigGeometry, View};
