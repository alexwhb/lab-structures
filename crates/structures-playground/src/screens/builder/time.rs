//! TIME-1 "Fast-forward 30 years" and TIME-3 the Leaning tower's run (owner: agent `time`).
//!
//! A stub created by the `screen` agent with the agreed hook ([`FoldIn`], see
//! `host.rs`); the owner fills it in.

use super::host::FoldIn;

/// The fold-in's state.
#[derive(Debug, Default)]
pub struct Time;

impl FoldIn for Time {}
