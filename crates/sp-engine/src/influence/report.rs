//! The member table, the status pill and the checks (the mockup's `rows`, `statusLabel`,
//! `checks`), with their exact text.

use super::{NEAR_ZERO_N, Report, Span, Tone, mode_word, pct};
use crate::netlist::Netlist;
use crate::netlist::text::{minus_fixed, to_fixed};
use crate::truss::{FailureMode, MemberResult, Solution, Truss};
use serde::{Deserialize, Serialize};

/// A check's dot colour (and the status pill's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    /// Green: fine.
    Good,
    /// Sand: worth a look (determinate, near the limit, deflection over the target).
    Accent,
    /// Red: over the limit, no solution.
    Bad,
    /// Grey: information.
    Neutral,
}

/// How a utilisation reads in the table's Use column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseTone {
    /// ≤ 80 %: plain text.
    Normal,
    /// Over 80 %: sand.
    Near,
    /// Over 100 %: red.
    Over,
}

/// One row of the member results table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// The member (index into the netlist's members).
    pub member: usize,
    /// "M9".
    pub id: String,
    /// "T3–T4".
    pub nodes: String,
    /// N in kN: "−1004" (or "—" with no solution).
    pub force: String,
    /// σ in MPa: "−100" (or "—").
    pub stress: String,
    /// "yield", "crush", "buckle 1974" (the Euler load in kN) or "unstable".
    pub limit: String,
    /// "40%" (or "—").
    pub utilisation: String,
    /// Colour of the N column.
    pub tone: Tone,
    /// Colour of the Use column.
    pub use_tone: UseTone,
    /// The raw result (utilisation as a fraction, force in N, …).
    pub result: MemberResult,
}

/// The status pill over the elevation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Stable and every member within its limit.
    Holding {
        /// The worst member's use, %.
        worst_pct: i64,
        /// Over 80 %: the pill turns sand.
        near: bool,
    },
    /// The worst member is over its limit.
    Over {
        /// Its id.
        member: String,
    },
    /// The stiffness matrix is singular.
    Mechanism,
}

impl Status {
    /// "Holding · worst 40%", "M9 over its limit", "Mechanism".
    pub fn label(&self) -> String {
        match self {
            Status::Holding { worst_pct, .. } => format!("Holding · worst {worst_pct}%"),
            Status::Over { member } => format!("{member} over its limit"),
            Status::Mechanism => "Mechanism".into(),
        }
    }

    /// Green holding, sand near the limit, red over it or a mechanism.
    pub fn level(&self) -> Level {
        match self {
            Status::Holding { near: false, .. } => Level::Good,
            Status::Holding { near: true, .. } => Level::Accent,
            _ => Level::Bad,
        }
    }
}

/// One line of the Checks panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    /// "Determinacy".
    pub label: String,
    /// "m + r − 2j = 21 + 3 − 24 = 0" (mono).
    pub value: String,
    /// The sentence under it.
    pub note: String,
    /// The dot.
    pub level: Level,
}

fn check(label: &str, value: String, note: impl Into<String>, level: Level) -> Check {
    Check {
        label: label.into(),
        value,
        note: note.into(),
        level,
    }
}

/// Supports left to right, and the span between the outer two.
fn span(truss: &Truss) -> (Vec<usize>, Option<Span>) {
    let mut sup: Vec<usize> = (0..truss.nodes.len())
        .filter(|&i| truss.nodes[i].fix.is_support())
        .collect();
    sup.sort_by(|&a, &b| truss.nodes[a].x.total_cmp(&truss.nodes[b].x));
    let (Some(&left), Some(&right)) = (sup.first(), sup.last()) else {
        return (sup, None);
    };
    let (l, r) = (&truss.nodes[left], &truss.nodes[right]);
    let length = r.x - l.x;
    if length.is_nan() || length <= 0.0 {
        return (sup, None);
    }
    let (mx, my) = ((l.x + r.x) / 2.0, (l.y + r.y) / 2.0);
    let mut mid = left;
    let mut best = f64::INFINITY;
    for (i, n) in truss.nodes.iter().enumerate() {
        let d = (n.x - mx).hypot(n.y - my);
        if d < best {
            best = d;
            mid = i;
        }
    }
    (
        sup,
        Some(Span {
            left,
            right,
            length,
            mid,
        }),
    )
}

/// Mass of all the trusses' members: `copies × Σ (Σ A·L) ρ` per material, kg.
fn mass(truss: &Truss, copies: u32) -> f64 {
    let mut vol = vec![0.0; truss.materials.len()];
    for (i, m) in truss.members.iter().enumerate() {
        if let Some(v) = vol.get_mut(m.material) {
            *v += m.area * truss.length(i);
        }
    }
    let one: f64 = vol
        .iter()
        .zip(&truss.materials)
        .map(|(v, mat)| v * mat.rho)
        .filter(|m| m.is_finite())
        .sum();
    copies as f64 * one
}

pub(super) fn build(net: &Netlist, truss: Truss, solution: Solution, x: f64) -> Report {
    let ok = solution.stable;
    let copies = net.copies();
    let (supports, span) = span(&truss);
    let results: Vec<(usize, MemberResult)> = solution
        .members
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.map(|r| (i, r)))
        .collect();

    let tones = solution
        .members
        .iter()
        .map(|r| match r {
            None => Tone::Zero,
            Some(r) if !ok || r.utilisation > 1.0 => Tone::Failed,
            Some(r) if r.force.abs() >= NEAR_ZERO_N => {
                if r.force > 0.0 {
                    Tone::Tension
                } else {
                    Tone::Compression
                }
            }
            Some(_) => Tone::Zero,
        })
        .collect();

    let node_id = |n: usize| truss.nodes.get(n).map(|n| n.id.as_str()).unwrap_or("?");
    let mut rows: Vec<Row> = results
        .iter()
        .map(|&(i, r)| {
            let m = &truss.members[i];
            let limit = if !ok {
                "unstable".to_string()
            } else if r.force >= 0.0 {
                "yield".to_string()
            } else if r.mode == FailureMode::Buckling {
                format!("buckle {}", to_fixed(r.limit / 1000.0, 0))
            } else {
                "crush".to_string()
            };
            let dash = || "\u{2014}".to_string();
            Row {
                member: i,
                id: m.id.clone(),
                nodes: format!("{}\u{2013}{}", node_id(m.a), node_id(m.b)),
                force: if ok {
                    minus_fixed(r.force / 1000.0, 0)
                } else {
                    dash()
                },
                stress: if ok {
                    minus_fixed(r.stress / 1e6, 0)
                } else {
                    dash()
                },
                limit,
                utilisation: if ok {
                    format!("{}%", pct(r.utilisation))
                } else {
                    dash()
                },
                tone: if !ok || r.force.abs() < NEAR_ZERO_N {
                    Tone::Zero
                } else if r.force > 0.0 {
                    Tone::Tension
                } else {
                    Tone::Compression
                },
                use_tone: if r.utilisation > 1.0 {
                    UseTone::Over
                } else if r.utilisation > 0.8 {
                    UseTone::Near
                } else {
                    UseTone::Normal
                },
                result: r,
            }
        })
        .collect();
    // descending use; stable, so ties keep member order (as the mockup's Array.sort)
    rows.sort_by(|a, b| {
        b.result
            .utilisation
            .partial_cmp(&a.result.utilisation)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let worst = solution.worst.and_then(|w| {
        let r = solution.members.get(w)?.as_ref()?;
        Some((w, *r))
    });
    let worst_ut = worst.map(|(_, r)| r.utilisation).unwrap_or(0.0);
    let failed = !ok || worst_ut > 1.0;
    let status = if !ok {
        Status::Mechanism
    } else if failed {
        Status::Over {
            member: worst
                .map(|(w, _)| truss.members[w].id.clone())
                .unwrap_or_default(),
        }
    } else {
        Status::Holding {
            worst_pct: pct(worst_ut),
            near: worst_ut > 0.8,
        }
    };

    // ---- checks ----
    let m = truss.members.len() as i64;
    let j = truss.nodes.len() as i64;
    let r: i64 = truss.nodes.iter().map(|n| n.fix.reactions() as i64).sum();
    let deg = m + r - 2 * j;
    let determinacy = check(
        "Determinacy",
        format!(
            "m + r \u{2212} 2j = {m} + {r} \u{2212} {} = {deg}",
            2 * j
        ),
        match deg {
            0 => "Statically determinate: lose any member and it becomes a mechanism.".to_string(),
            d if d > 0 => format!("Indeterminate to degree {d}: there are spare load paths."),
            d => format!(
                "Short of {} member{}: it can fold.",
                -d,
                if d == -1 { "" } else { "s" }
            ),
        },
        match deg {
            0 => Level::Accent,
            d if d > 0 => Level::Good,
            _ => Level::Bad,
        },
    );

    let reactions = if supports.is_empty() {
        check(
            "Reactions",
            "none".into(),
            "No supports: give nodes fix=xy and fix=y.",
            Level::Bad,
        )
    } else if !ok {
        check(
            "Reactions",
            "no solution".into(),
            "Stiffness matrix is singular.",
            Level::Bad,
        )
    } else {
        let parts: Vec<String> = supports
            .iter()
            .enumerate()
            .map(|(k, &n)| {
                let letter = if k < 26 {
                    char::from(b'A' + k as u8).to_string()
                } else {
                    truss.nodes[n].id.clone()
                };
                let ry = solution.reactions.get(n).map(|r| r[1]).unwrap_or(0.0);
                format!("{letter} {} kN", to_fixed(ry / 1000.0, 0))
            })
            .collect();
        check(
            "Reactions",
            parts.join(" · "),
            "Vertical, per truss. They always sum to the total load.",
            Level::Good,
        )
    };

    let deflection = match span {
        None => check(
            "Midspan deflection",
            "no span".into(),
            "Needs two supports to measure.",
            Level::Neutral,
        ),
        Some(sp) => {
            let sag = if ok {
                -solution.displacements.get(sp.mid).map(|d| d[1]).unwrap_or(0.0)
            } else {
                0.0
            };
            let value = if ok {
                format!(
                    "{} mm = L/{}",
                    to_fixed(sag * 1000.0, 1),
                    to_fixed(super::js_round(sp.length / sag.max(1e-6)), 0)
                )
            } else {
                "unstable".into()
            };
            check(
                "Midspan deflection",
                value,
                "Illustrative target L/800 for a road bridge.",
                if ok && sp.length / sag >= 800.0 {
                    Level::Good
                } else {
                    Level::Accent
                },
            )
        }
    };

    let worst_check = match (ok, worst) {
        (false, _) => check(
            "Worst member",
            "no solution".into(),
            "Stiffness matrix is singular.",
            Level::Bad,
        ),
        (true, None) => check(
            "Worst member",
            "none".into(),
            "No members yet.",
            Level::Neutral,
        ),
        (true, Some((w, r))) => check(
            "Worst member",
            format!("{} at {}%", truss.members[w].id, pct(r.utilisation)),
            if r.force < 0.0 {
                format!("Compression, governed by {}.", mode_word(r.mode))
            } else {
                "Tension, governed by yield.".into()
            },
            if failed {
                Level::Bad
            } else if r.utilisation > 0.8 {
                Level::Accent
            } else {
                Level::Good
            },
        ),
    };

    let mass = mass(&truss, copies);
    let weight = check(
        "Steel weight",
        format!("{} t", to_fixed(mass / 1000.0, 1)),
        match copies {
            1 => "One truss, members only.".to_string(),
            2 => "Both trusses, members only.".to_string(),
            n => format!("All {n} trusses, members only."),
        },
        Level::Neutral,
    );

    Report {
        truss,
        solution,
        x,
        copies,
        supports,
        span,
        tones,
        rows,
        status,
        checks: vec![determinacy, reactions, deflection, worst_check, weight],
        mass,
    }
}
