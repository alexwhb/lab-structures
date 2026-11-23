//! LAB-1: the sample drawer — a piece type's own builder properties turned into a stress–strain
//! curve, pulled (and, for types that resist pushing, pushed) until it breaks, at a temperature.
//!
//! - [`curve`] — [`SampleCurve`]: straight at E to the builder's strength, then a yield plateau,
//!   hardening and necking for ductile types (Steel, Cable), a snap for brittle ones; pushing
//!   crushes or buckles a sample of the builder's section (1 m unless chosen) at the builder's
//!   compression capacity. Capacities come from the builder's own piece model
//!   ([`curve::piece_at`]) heated by the FIRE-1 factors, so "a piece this size breaks at N t"
//!   is the engine's number.
//! - [`bench`] — [`Sample`]: the drawer's state and stepper (Pull / Push / Fresh sample /
//!   temperature / length), 20 steps a second like v1's tensile bench, the status and the
//!   explain sentences.
//! - [`view`] — [`SampleView`]: every string and the rig / plot drawing in v1's viewBox units.
//!
//! Assumptions (curve shapes, post-peak tails): `docs/DATA-SOURCES.md` § Sample curves.
//!
//! Owner: agent `lab` (PROGRESS.md).

pub mod bench;
pub mod curve;
pub mod view;

pub use bench::{Sample, Status, Tone};
pub use curve::{Failure, SampleCurve, Stress, Test, capacity, tonnes};
pub use view::{PlotGeometry, RigGeometry, SampleView, Tick, Tile};
