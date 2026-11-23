//! LAB-1 the sample drawer (owner: agent `lab`).
//!
//! A stub created by the `screen` agent with the agreed hook ([`FoldIn`], see
//! `host.rs`); the owner fills it in.

use super::host::FoldIn;

/// The fold-in's state.
#[derive(Debug, Default)]
pub struct Lab;

impl FoldIn for Lab {}
