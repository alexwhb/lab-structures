//! The live preview (`preview(st)`): the settling engine run headless — 520 steps at
//! h = 1/400 s, damping 0.985, gravity ramped over 0.3 s, snow at its setting, no wind, quake
//! or traffic — and what it reports: each piece's smoothed force and whether it is over
//! capacity, each foundation's reaction, and the joint that dropped the most.
//!
//! [`settle`] is the same settle with options: per-foundation settlement offsets (SPEC TIME-1:
//! each offset lowers that foundation's settled level `ys`), and the step count / step /
//! damping, for agent `time`'s 30-year fast-forward. [`preview`] = `settle` with the defaults.

use super::design::Design;
use super::setup::{Conditions, Setup};
use super::sim::Sim;
use crate::truss::hypot;

/// Preview steps.
pub const PREVIEW_STEPS: usize = 520;
/// Preview step, s.
pub const PREVIEW_H: f64 = 1.0 / 400.0;
/// Preview velocity damping per step.
pub const PREVIEW_DAMPING: f64 = 0.985;

/// A piece in the settled state (`pv.res[id]`).
#[derive(Debug, Clone, PartialEq)]
pub struct PieceForce {
    /// Piece id.
    pub id: String,
    /// Smoothed axial force, N (+ tension; `N: c.avg`).
    pub n: f64,
    /// Past capacity.
    pub over: bool,
    /// Slack (tension-only, pushed).
    pub slack: bool,
    /// Tension capacity, N.
    pub cap_t: f64,
    /// Compression capacity, N (∞ for tension-only).
    pub cap_c: f64,
    /// Length, m.
    pub len: f64,
    /// Snow piles on it.
    pub exposed: bool,
}

impl PieceForce {
    /// Utilisation (`utOf`).
    pub fn utilisation(&self) -> f64 {
        super::sim::utilisation(self.n, self.cap_t, self.cap_c)
    }
}

/// A foundation in the settled state (`pv.found[id]`).
#[derive(Debug, Clone, PartialEq)]
pub struct FoundForce {
    /// Joint id.
    pub id: String,
    /// Smoothed reaction, N (+ pushing down on the soil).
    pub r: f64,
    /// Push capacity, N.
    pub qc: f64,
    /// Pull capacity, N.
    pub qt: f64,
    /// Pushed past capacity: it would sink.
    pub over_c: bool,
    /// Pulled past capacity: it would rip out.
    pub over_t: bool,
}

/// The settled state (`preview()`'s `{ res, found, moved, movedId }`, plus positions).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settled {
    /// Each piece, in the design's order.
    pub pieces: Vec<PieceForce>,
    /// Each joint's foundation (`None` for joints without one), in the design's order.
    pub found: Vec<Option<FoundForce>>,
    /// Each joint's settled position, m.
    pub positions: Vec<[f64; 2]>,
    /// The largest drop of a free (unfounded, moving) joint from its design spot, m.
    pub moved: f64,
    /// That joint's id.
    pub moved_id: Option<String>,
}

impl Settled {
    /// A piece's result by id.
    pub fn piece(&self, id: &str) -> Option<&PieceForce> {
        self.pieces.iter().find(|p| p.id == id)
    }
    /// A foundation's result by joint id.
    pub fn foundation(&self, id: &str) -> Option<&FoundForce> {
        self.found.iter().flatten().find(|f| f.id == id)
    }
    /// Any piece over capacity (`overAny`).
    pub fn over_any(&self) -> bool {
        self.pieces.iter().any(|p| p.over)
    }
    /// Foundations over capacity, in joint order (`foundOver`).
    pub fn found_over(&self) -> Vec<&FoundForce> {
        self.found
            .iter()
            .flatten()
            .filter(|f| f.over_c || f.over_t)
            .collect()
    }
}

/// Options for [`settle`].
#[derive(Debug, Clone, PartialEq)]
pub struct SettleOptions {
    /// Steps (520).
    pub steps: usize,
    /// Step, s (1/400).
    pub h: f64,
    /// Damping per step (0.985).
    pub damping: f64,
    /// Settlement per joint, m, in the design's joint order (missing / 0 = none): a founded
    /// joint's settled level `ys` is its design y minus this.
    pub offsets: Vec<f64>,
}

impl Default for SettleOptions {
    fn default() -> SettleOptions {
        SettleOptions {
            steps: PREVIEW_STEPS,
            h: PREVIEW_H,
            damping: PREVIEW_DAMPING,
            offsets: vec![],
        }
    }
}

/// The static settle with options (see the module docs).
pub fn settle(design: &Design, setup: &Setup, cond: &Conditions, opts: &SettleOptions) -> Settled {
    let mut s = Sim::new(design, setup, cond, false);
    for (p, off) in s.p.iter_mut().zip(&opts.offsets) {
        if p.sup.is_some() && *off != 0.0 && off.is_finite() {
            p.ys -= off;
        }
    }
    let steps = opts.steps.min(1_000_000);
    for _ in 0..steps {
        s.step(opts.h, opts.damping);
    }
    settled_of(&s, design)
}

/// The preview (`preview(st)`): [`settle`] with the defaults.
pub fn preview(design: &Design, setup: &Setup, cond: &Conditions) -> Settled {
    settle(design, setup, cond, &SettleOptions::default())
}

/// What a (settled) simulation reports.
pub fn settled_of(s: &Sim, design: &Design) -> Settled {
    let mut moved = 0.0;
    let mut moved_id = None;
    for (p, n) in s.p.iter().zip(&design.nodes) {
        let d = hypot(p.x - n.x, p.y - n.y);
        if p.inv > 0.0 && p.sup.is_none() && d > moved {
            moved = d;
            moved_id = Some(p.id.clone());
        }
    }
    Settled {
        pieces: s
            .c
            .iter()
            .map(|c| PieceForce {
                id: c.id.clone(),
                n: c.avg,
                over: c.over,
                slack: c.slack,
                cap_t: c.cap_t_now(),
                cap_c: c.cap_c_now(),
                len: c.len,
                exposed: c.exposed,
            })
            .collect(),
        found: s
            .p
            .iter()
            .map(|p| {
                p.sup.map(|_| FoundForce {
                    id: p.id.clone(),
                    r: p.r,
                    qc: p.qc,
                    qt: p.qt,
                    over_c: p.over_c,
                    over_t: p.over_t,
                })
            })
            .collect(),
        positions: s.p.iter().map(|p| [p.x, p.y]).collect(),
        moved,
        moved_id,
    }
}
