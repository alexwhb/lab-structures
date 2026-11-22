//! `sp-engine` — the Structures Playground engine. No UI dependencies.
//!
//! Modules (see `docs/ARCHITECTURE.md` for owners and the full contract):
//!
//! - [`linalg`] — dense Gaussian elimination with partial pivoting that reports the column where
//!   the matrix is singular (the shared `K·u = F` solve).
//! - [`truss`] — the 2D pin-jointed truss model, the direct stiffness solve, reactions, member
//!   forces and the member checks (tension yield, Euler buckling, crushing). Exact for the model.
//!
//! Units are SI throughout: metres, newtons, pascals, kilograms. Forces are positive in tension.
//! Nothing here panics on any input: degenerate geometry (zero-length members, bad indices,
//! non-finite numbers) is reported as an error or treated as a mechanism.
#![warn(missing_docs)]

pub mod linalg;
pub mod materials;
pub mod truss;

pub use truss::{
    FailureMode, Fix, G, Material, Member, MemberResult, NodalLoad, Node, Solution, SolveError,
    SolveOptions, Truss,
};
