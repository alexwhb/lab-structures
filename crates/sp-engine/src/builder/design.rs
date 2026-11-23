//! The structure being built: joints (`J<n>`), pieces (`M<n>`) and crates, exactly the mockup's
//! `state.nodes` / `members` / `loads` / `nN` / `nM`, and its cost (`cost()`).

use super::data::{Foundation, PieceType};
use crate::truss::hypot;

/// A joint (`{ id, x, y, sup, locked }`). Coordinates are world metres (grid points).
#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    /// "J3".
    pub id: String,
    /// x, m.
    pub x: f64,
    /// y, m (up).
    pub y: f64,
    /// Its foundation, if any.
    pub sup: Option<Foundation>,
    /// A level's own foundation (river levels' anchors; the Leaning tower's pre-built joints):
    /// it can't be moved, deleted or re-founded, and costs nothing.
    pub locked: bool,
}

/// A piece between two joints (`{ id, a, b, type }`).
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// "M7".
    pub id: String,
    /// One end's joint id.
    pub a: String,
    /// The other end's joint id.
    pub b: String,
    /// Its type.
    pub kind: PieceType,
    /// A level's own piece (Leaning tower): it can't be deleted or changed. The mockup has no
    /// locked pieces; every v1-mockup level leaves this `false`.
    pub locked: bool,
}

/// The structure: joints, pieces, crates and the id counters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Design {
    /// Joints in creation order.
    pub nodes: Vec<Joint>,
    /// Pieces in creation order.
    pub members: Vec<Piece>,
    /// Crates (joint id, tonnes) in the order they were first hung (a JS object's key order:
    /// removing a crate and hanging it again moves it to the end).
    pub loads: Vec<(String, f64)>,
    /// The joint counter (`nN`): the next joint is `J{n_n + 1}`.
    pub n_n: u32,
    /// The piece counter (`nM`).
    pub n_m: u32,
}

impl Design {
    /// The joint with this id.
    pub fn node(&self, id: &str) -> Option<&Joint> {
        self.nodes.iter().find(|n| n.id == id)
    }
    /// Its index.
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }
    /// The piece with this id.
    pub fn member(&self, id: &str) -> Option<&Piece> {
        self.members.iter().find(|m| m.id == id)
    }
    /// Its index.
    pub fn member_index(&self, id: &str) -> Option<usize> {
        self.members.iter().position(|m| m.id == id)
    }
    /// The crate on a joint, t (0 for none).
    pub fn load(&self, id: &str) -> f64 {
        self.loads
            .iter()
            .find(|(k, _)| k == id)
            .map_or(0.0, |&(_, t)| t)
    }
    /// Sets the crate on a joint (0 removes it), keeping the first-hung order.
    pub fn set_load(&mut self, id: &str, t: f64) {
        match self.loads.iter().position(|(k, _)| k == id) {
            Some(i) if t == 0.0 => {
                self.loads.remove(i);
            }
            Some(i) => self.loads[i].1 = t,
            None if t != 0.0 => self.loads.push((id.to_string(), t)),
            None => {}
        }
    }
    /// A piece's length in the design, m (`Math.hypot`).
    pub fn length(&self, m: &Piece) -> f64 {
        match (self.node(&m.a), self.node(&m.b)) {
            (Some(a), Some(b)) => hypot(b.x - a.x, b.y - a.y),
            _ => 0.0,
        }
    }
    /// Whether a locked piece meets this joint (it can't be moved or deleted then).
    pub fn holds_locked(&self, id: &str) -> bool {
        self.members
            .iter()
            .any(|m| m.locked && (m.a == id || m.b == id))
    }
    /// Whether the design has a piece of this type.
    pub fn has(&self, kind: PieceType) -> bool {
        self.members.iter().any(|m| m.kind == kind)
    }
    /// The cost in $k (`cost()`): Σ length × $/m of the pieces that aren't locked (a level's
    /// own pieces cost nothing; the mockup has none), plus $300 per footing and $1 500 per pile
    /// that isn't locked, / 1000.
    pub fn cost(&self) -> f64 {
        let mut c = 0.0;
        for m in self.members.iter().filter(|m| !m.locked) {
            c += self.length(m) * m.kind.spec().cost;
        }
        for n in &self.nodes {
            if let Some(f) = n.sup
                && !n.locked
            {
                c += f.cost();
            }
        }
        c / 1000.0
    }
    /// The highest joint touched by a Floor, m (`designTop`; 0 with no Floors).
    pub fn top(&self) -> f64 {
        let ys: Vec<f64> = self
            .nodes
            .iter()
            .filter(|n| {
                self.members
                    .iter()
                    .any(|m| m.kind == PieceType::Floor && (m.a == n.id || m.b == n.id))
            })
            .map(|n| n.y)
            .collect();
        if ys.is_empty() {
            0.0
        } else {
            ys.into_iter().fold(f64::NEG_INFINITY, f64::max)
        }
    }
}
