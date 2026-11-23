//! TIME-1…TIME-3: settlement over decades (consolidation per foundation, tilt with P-Δ
//! feedback) and the Leaning tower level.
//!
//! Owner: agent `time` (PROGRESS.md).
//!
//! **The model** (simplified; `docs/DATA-SOURCES.md`). From the settled state of a design,
//! each foundation consolidates over 0–30 years:
//!
//! - `s(t) = s∞·(1 − e^(−t/τ))`, with `s∞ = R / k_long` for its long-term load `R` (the settled
//!   reaction) and the soil's long-term stiffness `k_long = ks / ratio`: the builder's own
//!   foundation stiffness `ks` divided by the soil's consolidation ratio, `ratio = 1 kPa/mm ÷
//!   ks_ground` from `sp_engine::ground`'s settlement stiffness (soft clay 0.6 kPa/mm → its
//!   foundations end up settling 1.7 × their first sink; dense sand 8 → an eighth; rock none).
//!   A foundation pushed past its capacity creeps on: `+ 3 × (R − Qc) / k_long`.
//! - τ is the soil's consolidation time from `sp_engine::ground` (dense sand 0.3 yr, stiff clay
//!   3, soft clay 9, loose wet sand 0.5; rock doesn't settle).
//! - The load changes as the structure tilts, so the year is stepped (every [`STEP`] years):
//!   each foundation relaxes toward its current `s∞` exactly for a constant load
//!   (`s ← s∞ + (s − s∞)·e^(−Δt/τ)`, never rising back), then the structure is settled again on
//!   the lowered foundations ([`statics`]): unequal settlement tilts it, the tilt moves weight
//!   toward the low side, which sinks more (P-Δ). Pieces can break and foundations rip out
//!   while it settles; a tower leaning past [`TOPPLE_DEG`] topples.
//! - Long-term loads are the structure's own weight and its crates (no wind, snow, quakes or
//!   traffic: they come and go).
//!
//! **Tilt** (TIME-2) is the angle between vertical and the line from the foundations' centroid
//! to the top floor's centroid, measured from that line as built (a tower built crooked starts
//! at 0°; the first, immediate settling counts); bridges report the largest differential
//! settlement instead. "Settled" and the differential count the consolidation since year 0.

pub mod pisa;
pub mod statics;

use super::data::{PieceType, Scene, Soil};
use super::design::Design;
use super::setup::{Conditions, Setup};
use super::sim::Sim;
use crate::ground;
use crate::jsnum::fixed;
use statics::{Consolidation, Failure, SJoint, SPiece, Spring, State, Structure};

/// The fast-forward covers this many years.
pub const YEARS: f64 = 30.0;
/// The solve steps this many years (frames at 0, 0.5, …, 30).
pub const STEP: f64 = 0.5;
/// A tower leaning past this topples, degrees.
pub const TOPPLE_DEG: f64 = 5.0;
/// The consolidation ratio's reference stiffness, kPa/mm (`ratio = KS_REF / ks_ground`).
pub const KS_REF: f64 = 1.0;
/// Extra long-term settlement per unit of overload past a foundation's push capacity.
pub const OVERLOAD_CREEP: f64 = 3.0;
/// The fast-forward animation lasts this long, s (TIME-1 "about 6 s").
pub const ANIM_SECONDS: f64 = 6.0;
/// How far a toppled tower is drawn falling over, degrees ([`LongTerm::fall`]).
pub const FALL_DEG: f64 = 80.0;
/// The fall lasts this long on screen, s.
pub const FALL_SECONDS: f64 = 1.2;
/// Tilt under this is "barely leans" (and the Leaning tower's goal), degrees.
pub const LEAN_OK_DEG: f64 = 0.5;

/// How a builder soil consolidates: τ from `sp_engine::ground`'s soil and the consolidation
/// ratio `KS_REF / ks_ground` (`None` for rock, which doesn't settle).
pub fn consolidation(soil: Soil) -> Option<Consolidation> {
    let g = match soil {
        Soil::Rock => return None,
        Soil::Dense => ground::Soil::Dense,
        Soil::Stiff => ground::Soil::Stiff,
        Soil::Soft => ground::Soil::Soft,
        Soil::Loose => ground::Soil::Loose,
    }
    .def();
    Some(Consolidation {
        tau: g.tau,
        ratio: KS_REF / g.ks,
    })
}

/// The structure the long-term solve works on, from a design as the settling engine sets it
/// up (`Sim::new(…, live = false)`: joint masses with crates, foundations and half of each
/// piece; each foundation's soil grip; each piece's stiffness and capacities), and whether it
/// is a tower (open ground with Floors) or a bridge.
pub fn structure(design: &Design, setup: &Setup, cond: &Conditions) -> (Structure, Kind) {
    let sim = Sim::new(design, setup, cond, false);
    let joints = design
        .nodes
        .iter()
        .zip(&sim.p)
        .map(|(n, p)| SJoint {
            id: n.id.clone(),
            x: n.x,
            y: n.y,
            mass: p.m,
            found: p.sup.map(|_| Spring {
                qc: p.qc,
                qt: p.qt,
                ks: p.ks,
                consolidation: consolidation(setup.soil_at(n)),
            }),
            on_ground: n.sup.is_none() && n.y <= setup.ground(n.x) + 0.01,
        })
        .collect();
    let pieces = sim
        .c
        .iter()
        .map(|c| SPiece {
            id: c.id.clone(),
            a: c.a,
            b: c.b,
            rest: c.len,
            kk: c.kk,
            cap_t: c.cap_t,
            cap_c: c.cap_c,
            t_only: c.t_only,
            buckles: c.buckles,
            cracks: c.kind == PieceType::Concrete,
        })
        .collect();
    let top_y = design.top();
    let tower = setup.scene == Scene::Flat && design.has(PieceType::Floor);
    let kind = if tower {
        let top = design
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                (n.y - top_y).abs() < 1e-6
                    && design
                        .members
                        .iter()
                        .any(|m| m.kind == PieceType::Floor && (m.a == n.id || m.b == n.id))
            })
            .map(|(i, _)| i)
            .collect();
        Kind::Tower { top }
    } else {
        Kind::Bridge
    };
    (Structure { joints, pieces }, kind)
}

/// The 30-year fast-forward of a design (TIME-1): [`structure`] + [`fast_forward`].
pub fn fast_forward_design(design: &Design, setup: &Setup, cond: &Conditions) -> LongTerm {
    let (s, kind) = structure(design, setup, cond);
    fast_forward(&s, kind)
}

/// Whether a design can be fast-forwarded: at least one foundation carries a piece.
pub fn has_foundations(design: &Design) -> bool {
    design
        .nodes
        .iter()
        .any(|n| n.sup.is_some() && design.members.iter().any(|m| m.a == n.id || m.b == n.id))
}

/// Whether the structure is a tower (it reports tilt) or a bridge (differential settlement).
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A tower: the joints of its top floor (indices), for the tilt line.
    Tower {
        /// The top floor's joints.
        top: Vec<usize>,
    },
    /// A bridge or anything else: report the largest differential settlement.
    Bridge,
}

/// The structure at one moment of the 30 years.
#[derive(Debug, Clone, PartialEq)]
pub struct YearFrame {
    /// Years since the start.
    pub year: f64,
    /// Joint positions, m (unused joints stay where they were drawn).
    pub pos: Vec<[f64; 2]>,
    /// Axial force per piece, N (tension positive; 0 when broken or slack).
    pub force: Vec<f64>,
    /// Broken pieces.
    pub broken: Vec<bool>,
    /// Slack ropes and cables.
    pub slack: Vec<bool>,
    /// Consolidation settlement per joint since year 0, m (0 for joints without a foundation).
    pub settle: Vec<f64>,
    /// Foundation reaction per joint, N (pushing down on the soil positive).
    pub reaction: Vec<f64>,
    /// Tilt from the line as built, degrees (positive leans right; towers).
    pub tilt: f64,
    /// The largest consolidation settlement of any foundation, m.
    pub max_settle: f64,
    /// The largest difference between two foundations' settlements, m.
    pub differential: f64,
}

/// Something that gave way during the 30 years.
#[derive(Debug, Clone, PartialEq)]
pub enum Giving {
    /// A piece broke.
    Broke {
        /// Piece index.
        piece: usize,
        /// Piece id ("M7").
        id: String,
        /// "snapped" / "cracked" / "buckled" / "crushed".
        mode: &'static str,
        /// The year.
        year: f64,
    },
    /// A foundation ripped out of the ground.
    Ripped {
        /// Joint index.
        joint: usize,
        /// Joint id.
        id: String,
        /// The year.
        year: f64,
    },
}

impl Giving {
    /// When it happened, years.
    pub fn year(&self) -> f64 {
        match self {
            Giving::Broke { year, .. } | Giving::Ripped { year, .. } => *year,
        }
    }
}

/// How the 30 years ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ending {
    /// Still standing after 30 years.
    Stood,
    /// A tower leaned past [`TOPPLE_DEG`] in this year.
    Toppled(f64),
    /// Something gave way and the structure came down in this year.
    CameDown(f64),
    /// There was nothing to settle (no foundation carries anything).
    Nothing,
}

/// The 30-year fast-forward of one structure (TIME-1).
#[derive(Debug, Clone, PartialEq)]
pub struct LongTerm {
    /// Tower or bridge.
    pub kind: Kind,
    /// The frames, every [`STEP`] years from 0 until the end.
    pub frames: Vec<YearFrame>,
    /// Pieces that broke and foundations that ripped out, in order.
    pub giving: Vec<Giving>,
    /// How it ended.
    pub ending: Ending,
    /// Joint ids, in the frames' joint order (the design's).
    pub joint_ids: Vec<String>,
    /// Piece ids, in the frames' piece order.
    pub piece_ids: Vec<String>,
    /// The joints with a foundation (indices).
    pub found: Vec<usize>,
}

/// One moment between frames, interpolated (what the animation and the scrubber show).
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// The year asked for (clamped to the computed span).
    pub year: f64,
    /// Joint positions, m.
    pub pos: Vec<[f64; 2]>,
    /// Axial force per piece, N.
    pub force: Vec<f64>,
    /// Broken pieces (as of the frame at or before `year`).
    pub broken: Vec<bool>,
    /// Slack pieces (as of the frame at or before `year`).
    pub slack: Vec<bool>,
    /// Tilt, degrees.
    pub tilt: f64,
    /// The largest settlement, m.
    pub max_settle: f64,
    /// The largest differential settlement, m.
    pub differential: f64,
    /// Fallen over or come down by this year.
    pub fallen: bool,
}

fn tilt_angle(s: &Structure, kind: &Kind, pos: &[[f64; 2]]) -> f64 {
    let Kind::Tower { top } = kind else {
        return 0.0;
    };
    let found: Vec<usize> = (0..s.joints.len())
        .filter(|&j| s.joints[j].found.is_some())
        .collect();
    let top: Vec<usize> = top.iter().copied().filter(|&j| j < pos.len()).collect();
    if found.is_empty() || top.is_empty() {
        return 0.0;
    }
    let mean = |ix: &[usize]| {
        let n = ix.len() as f64;
        let (sx, sy) = ix
            .iter()
            .fold((0.0, 0.0), |(x, y), &j| (x + pos[j][0], y + pos[j][1]));
        (sx / n, sy / n)
    };
    let (fx, fy) = mean(&found);
    let (tx, ty) = mean(&top);
    (tx - fx).atan2(ty - fy).to_degrees()
}

fn frame(s: &Structure, kind: &Kind, st: &State, year: f64, tilt0: f64) -> YearFrame {
    let n = s.joints.len();
    let found: Vec<usize> = (0..n).filter(|&j| s.joints[j].found.is_some()).collect();
    let settle: Vec<f64> = (0..n)
        .map(|j| {
            if s.joints[j].found.is_some() {
                st.settle[j]
            } else {
                0.0
            }
        })
        .collect();
    let max_settle = found.iter().map(|&j| settle[j]).fold(0.0, f64::max);
    let min_settle = found
        .iter()
        .map(|&j| settle[j])
        .fold(f64::INFINITY, f64::min);
    let differential = if found.is_empty() {
        0.0
    } else {
        max_settle - min_settle
    };
    YearFrame {
        year,
        pos: st.pos.clone(),
        force: (0..s.pieces.len()).map(|i| s.force(st, i)).collect(),
        broken: st.broken.clone(),
        slack: (0..s.pieces.len()).map(|i| s.slack(st, i)).collect(),
        settle,
        reaction: (0..n).map(|j| s.reaction(st, j)).collect(),
        tilt: match kind {
            Kind::Tower { .. } => tilt_angle(s, kind, &st.pos) - tilt0,
            Kind::Bridge => 0.0,
        },
        max_settle,
        differential,
    }
}

/// Runs the 30-year fast-forward on a structure (TIME-1): settle it, then step the years.
/// Deterministic; nothing panics on any input (a structure that can't stand ends at year 0).
pub fn fast_forward(s: &Structure, kind: Kind) -> LongTerm {
    let mut out = LongTerm {
        kind,
        frames: Vec::new(),
        giving: Vec::new(),
        ending: Ending::Stood,
        joint_ids: s.joints.iter().map(|j| j.id.clone()).collect(),
        piece_ids: s.pieces.iter().map(|p| p.id.clone()).collect(),
        found: (0..s.joints.len())
            .filter(|&j| s.joints[j].found.is_some())
            .collect(),
    };
    let carrying = s.joints.iter().any(|j| j.found.is_some())
        && s.joints
            .iter()
            .all(|j| j.x.is_finite() && j.y.is_finite() && j.mass.is_finite() && j.mass >= 0.0);
    let mut st = s.initial();
    if !carrying {
        out.ending = Ending::Nothing;
        out.frames.push(frame(
            s,
            &out.kind,
            &st,
            0.0,
            tilt_angle(s, &out.kind, &st.pos),
        ));
        return out;
    }
    // tilt is measured from the line as built (so a tower built crooked starts at 0°), and
    // includes the first, immediate settling
    let tilt0 = tilt_angle(s, &out.kind, &st.pos);
    let first = s.settle(&mut st);
    match first {
        Ok(ev) => record(s, &mut out, &ev, 0.0),
        Err(_) => {
            out.ending = Ending::CameDown(0.0);
            out.frames
                .push(frame(s, &out.kind, &s.initial(), 0.0, tilt0));
            return out;
        }
    }
    out.frames.push(frame(s, &out.kind, &st, 0.0, tilt0));
    let steps = (YEARS / STEP).round() as usize;
    for k in 1..=steps {
        let year = k as f64 * STEP;
        consolidate(s, &mut st, STEP);
        let before = st.clone();
        match s.settle(&mut st) {
            Ok(ev) => record(s, &mut out, &ev, year),
            Err(f) => {
                // it couldn't find a way to stand: a tower tips over, anything else comes down
                let tower = matches!(out.kind, Kind::Tower { .. });
                out.ending = if tower && f == Failure::Diverged {
                    Ending::Toppled(year)
                } else {
                    Ending::CameDown(year)
                };
                out.frames.push(frame(s, &out.kind, &before, year, tilt0));
                return out;
            }
        }
        let fr = frame(s, &out.kind, &st, year, tilt0);
        let tilt = fr.tilt.abs();
        out.frames.push(fr);
        if matches!(out.kind, Kind::Tower { .. }) && tilt > TOPPLE_DEG {
            out.ending = Ending::Toppled(year);
            return out;
        }
    }
    out
}

fn record(s: &Structure, out: &mut LongTerm, ev: &statics::Events, year: f64) {
    for &(i, mode) in &ev.breaks {
        out.giving.push(Giving::Broke {
            piece: i,
            id: s.pieces[i].id.clone(),
            mode,
            year,
        });
    }
    for &j in &ev.rips {
        out.giving.push(Giving::Ripped {
            joint: j,
            id: s.joints[j].id.clone(),
            year,
        });
    }
}

/// Moves every foundation's consolidation settlement forward `dt` years for its current load
/// (exact for a constant load; settlement never comes back up).
fn consolidate(s: &Structure, st: &mut State, dt: f64) {
    for j in 0..s.joints.len() {
        let Some(f) = s.joints[j].found else {
            continue;
        };
        let Some(c) = f.consolidation else {
            continue;
        };
        if st.ripped[j] || c.tau.is_nan() || c.tau <= 0.0 {
            continue;
        }
        let k_long = f.long_term_stiffness();
        if !k_long.is_finite() || k_long <= 0.0 {
            continue;
        }
        let r = s.reaction(st, j);
        let s_inf = r.max(0.0) / k_long + OVERLOAD_CREEP * (r - f.qc).max(0.0) / k_long;
        let next = s_inf + (st.settle[j] - s_inf) * (-dt / c.tau).exp();
        if next.is_finite() && next > st.settle[j] {
            st.settle[j] = next;
        }
    }
}

impl LongTerm {
    /// The last computed year (30, or when it toppled / came down).
    pub fn end_year(&self) -> f64 {
        self.frames.last().map_or(0.0, |f| f.year)
    }

    /// The frames' index of piece `id`.
    pub fn piece_index(&self, id: &str) -> Option<usize> {
        self.piece_ids.iter().position(|p| p == id)
    }

    /// The frames' index of joint `id`.
    pub fn joint_index(&self, id: &str) -> Option<usize> {
        self.joint_ids.iter().position(|j| j == id)
    }

    /// The last frame.
    pub fn last(&self) -> Option<&YearFrame> {
        self.frames.last()
    }

    /// Whether it is a tower (tilt) rather than a bridge (differential settlement).
    pub fn is_tower(&self) -> bool {
        matches!(self.kind, Kind::Tower { .. })
    }

    /// The structure at `year`, interpolated linearly between frames (positions, forces and
    /// the three readouts; broken / slack as of the frame at or before it).
    pub fn at(&self, year: f64) -> Snapshot {
        let end = self.end_year();
        let y = if year.is_finite() {
            year.clamp(0.0, end)
        } else {
            0.0
        };
        let fallen_at = match self.ending {
            Ending::Toppled(t) | Ending::CameDown(t) => Some(t),
            _ => None,
        };
        if self.frames.is_empty() {
            return Snapshot {
                year: 0.0,
                pos: Vec::new(),
                force: Vec::new(),
                broken: Vec::new(),
                slack: Vec::new(),
                tilt: 0.0,
                max_settle: 0.0,
                differential: 0.0,
                fallen: false,
            };
        }
        let i = self
            .frames
            .iter()
            .rposition(|f| f.year <= y + 1e-9)
            .unwrap_or(0);
        let a = &self.frames[i];
        let b = self.frames.get(i + 1).unwrap_or(a);
        let u = if b.year > a.year {
            ((y - a.year) / (b.year - a.year)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let lerp = |p: f64, q: f64| p + (q - p) * u;
        Snapshot {
            year: y,
            pos: a
                .pos
                .iter()
                .zip(&b.pos)
                .map(|(p, q)| [lerp(p[0], q[0]), lerp(p[1], q[1])])
                .collect(),
            force: a
                .force
                .iter()
                .zip(&b.force)
                .map(|(p, q)| lerp(*p, *q))
                .collect(),
            broken: a.broken.clone(),
            slack: a.slack.clone(),
            tilt: lerp(a.tilt, b.tilt),
            max_settle: lerp(a.max_settle, b.max_settle),
            differential: lerp(a.differential, b.differential),
            fallen: fallen_at.is_some_and(|t| y >= t - 1e-9),
        }
    }

    /// The tilt line at `year` (towers; TIME-2): the foundations' centroid and the top floor's
    /// centroid, world m. The stage draws it against a plumb line.
    pub fn lean_line(&self, year: f64) -> Option<([f64; 2], [f64; 2])> {
        let Kind::Tower { top } = &self.kind else {
            return None;
        };
        let s = self.at(year);
        let mean = |ix: &[usize]| -> Option<[f64; 2]> {
            let pts: Vec<[f64; 2]> = ix.iter().filter_map(|&j| s.pos.get(j).copied()).collect();
            if pts.is_empty() {
                return None;
            }
            let n = pts.len() as f64;
            Some([
                pts.iter().map(|p| p[0]).sum::<f64>() / n,
                pts.iter().map(|p| p[1]).sum::<f64>() / n,
            ])
        };
        Some((mean(&self.found)?, mean(top)?))
    }

    /// A toppled tower falling over (drawing only): the last frame's joints turned about the
    /// outermost foundation on the side it leans to, by `k` × [`FALL_DEG`] (`k` 0 → 1, eased
    /// in), none below the ground line through that foundation. `None` unless it toppled.
    pub fn fall(&self, k: f64) -> Option<Vec<[f64; 2]>> {
        let Ending::Toppled(_) = self.ending else {
            return None;
        };
        let last = self.last()?;
        let right = last.tilt >= 0.0;
        let pivot = self
            .found
            .iter()
            .filter_map(|&j| last.pos.get(j).copied())
            .fold(None::<[f64; 2]>, |best, p| match best {
                Some(b) if (right && b[0] >= p[0]) || (!right && b[0] <= p[0]) => Some(b),
                _ => Some(p),
            })?;
        let k = if k.is_finite() {
            k.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if k == 0.0 {
            return Some(last.pos.clone());
        }
        let th = (FALL_DEG * k * k).to_radians() * if right { -1.0 } else { 1.0 };
        let (sn, cs) = th.sin_cos();
        Some(
            last.pos
                .iter()
                .map(|p| {
                    let (dx, dy) = (p[0] - pivot[0], p[1] - pivot[1]);
                    [
                        pivot[0] + dx * cs - dy * sn,
                        (pivot[1] + dx * sn + dy * cs).max(pivot[1]),
                    ]
                })
                .collect(),
        )
    }

    /// The HUD chips at `year` (TIME-1): `("Year ", "12.0")`, `("Tilt ", "1.8°")` (towers) or
    /// `("Differential ", "12 mm")` (bridges), `("Settled ", "214 mm")`.
    pub fn chips(&self, year: f64) -> Vec<(&'static str, String)> {
        let s = self.at(year);
        let mut v = vec![("Year ", fixed(s.year, 1))];
        if self.is_tower() {
            v.push(("Tilt ", format!("{}°", fixed(s.tilt.abs(), 1))));
        } else {
            v.push(("Differential ", mm(s.differential)));
        }
        v.push(("Settled ", mm(s.max_settle)));
        v
    }

    /// The result line: "After 30 years: tilt 0.3°, settled 41 mm." (towers) / "After 30
    /// years: differential settlement 12 mm, settled 41 mm." (bridges) / "It toppled in year
    /// 17, after settling 640 mm." / "It came down in year 4."
    pub fn result_line(&self) -> String {
        let Some(f) = self.last() else {
            return String::new();
        };
        let numbers = if self.is_tower() {
            format!(
                "tilt {}°, settled {}",
                fixed(f.tilt.abs(), 1),
                mm(f.max_settle)
            )
        } else {
            format!(
                "differential settlement {}, settled {}",
                mm(f.differential),
                mm(f.max_settle)
            )
        };
        match self.ending {
            Ending::Stood | Ending::Nothing => {
                format!("After {} years: {numbers}.", fixed(YEARS, 0))
            }
            Ending::Toppled(y) => format!(
                "It toppled in year {}, after settling {}.",
                year_text(y),
                mm(f.max_settle)
            ),
            Ending::CameDown(y) => format!("It came down in year {}.", year_text(y)),
        }
    }

    /// The sentence after the result line: what happened and why.
    pub fn sentence(&self) -> String {
        let Some(f) = self.last() else {
            return String::new();
        };
        let broke = self.giving.iter().find_map(|g| match g {
            Giving::Broke { id, mode, year, .. } => Some((id.clone(), *mode, *year)),
            _ => None,
        });
        let ripped = self.giving.iter().find_map(|g| match g {
            Giving::Ripped { id, year, .. } => Some((id.clone(), *year)),
            _ => None,
        });
        let cause = match (&broke, &ripped) {
            (Some((id, mode, y)), _) => format!(
                " Uneven settling strained {id} until it {mode} in year {}.",
                year_text(*y)
            ),
            (None, Some((id, y))) => format!(
                " Foundation {id} was pulled out of the ground in year {}.",
                year_text(*y)
            ),
            _ => String::new(),
        };
        match self.ending {
            Ending::Nothing => {
                "Nothing rests on a foundation, so there is nothing to settle.".into()
            }
            Ending::Toppled(_) => format!(
                "The heavy side kept sinking, and every bit of lean put more weight on it, until \
                 it tipped over. Piles under the low side or a counterweight on the high side \
                 stop that.{cause}"
            ),
            Ending::CameDown(_) => {
                format!("The ground moved under it and it could no longer hold itself up.{cause}")
            }
            Ending::Stood => {
                let moved = f.max_settle >= 0.0005;
                let base = if !moved {
                    "Rock doesn\u{2019}t settle: it stands exactly as built.".to_string()
                } else if self.is_tower() {
                    let t = f.tilt.abs();
                    if t < LEAN_OK_DEG {
                        "It sank evenly, so it barely leans.".to_string()
                    } else if t < 2.0 {
                        "It leans a little more every year: the heavier side sinks deeper, and \
                         the lean puts even more weight on it."
                            .to_string()
                    } else {
                        "A clear lean: the heavy side sank deeper and the lean fed itself. Piles \
                         under the low side, or a counterweight on the high side, would stop it."
                            .to_string()
                    }
                } else if f.differential < 0.02 {
                    "Its foundations sank about evenly, so nothing in between is strained."
                        .to_string()
                } else {
                    "One foundation sank more than another, which bends everything between them."
                        .to_string()
                };
                format!("{base}{cause}")
            }
        }
    }
}

/// "214 mm" (whole millimetres).
pub fn mm(m: f64) -> String {
    format!("{} mm", fixed((m * 1000.0).max(0.0), 0))
}

/// A year in a sentence: "17", "0.5".
fn year_text(y: f64) -> String {
    if (y - y.round()).abs() < 1e-9 {
        fixed(y, 0)
    } else {
        fixed(y, 1)
    }
}

/// Where the animation is after `elapsed` seconds (0 → 30 years over [`ANIM_SECONDS`]),
/// stopping at `end` (the last computed year).
pub fn anim_year(elapsed: f64, end: f64) -> f64 {
    if !elapsed.is_finite() {
        return 0.0;
    }
    (elapsed.max(0.0) / ANIM_SECONDS * YEARS).min(end.max(0.0))
}
