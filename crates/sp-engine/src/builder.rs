//! The v2 builder (SPEC "Builder", BLD-*): one settling engine for bridges and towers — pieces,
//! foundations on soil, crates, vehicles, wind, snow, earthquakes — and the game around it
//! (levels, editing, the camera, results, every sentence). A port of
//! `docs/design/v2/mockup/Builder.dc.html`'s `class Component`, checked against the mockup's own
//! JS run in Node.
//!
//! Owners (PROGRESS.md): the core is agent `engine`; [`comfort`] and [`longterm`] are agent
//! `time`; [`sample`] and [`fire`] are agent `lab`.

pub mod comfort;
pub mod fire;
pub mod longterm;
pub mod sample;
