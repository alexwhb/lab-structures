//! Moving-load analysis and the Advanced screen's results (SPEC ADV-4 … ADV-7, ADV-10).
//!
//! [`analyse`] solves a [`Netlist`] with its truck where it stands and returns a [`Report`]:
//! the solution, the member table sorted by use, the status pill and the five checks, each
//! with the exact text the mockup's `renderVals()` shows. [`influence_line`] moves the truck
//! across the lane (0 → span in 1 m steps, or the `.plot` range), solving each position in full
//! (49 solves of the mockup truss, well under ENG-4's 5 ms), and finds the peak and the limits.
//!
//! Everything here is UI-free: the screen maps [`Tone`] and [`Level`] to `sp_theme` colours.

use crate::jsnum::{js_number, js_round, minus_fixed, to_fixed};
use crate::netlist::Netlist;
use crate::truss::{
    FailureMode, Member, MemberResult, NodalLoad, Solution, SolveError, SolveOptions, Truss, check,
};
use serde::{Deserialize, Serialize};

mod report;

pub use report::{Check, Level, Row, Status, UseTone};

/// Forces below this many newtons draw grey in the elevation and the table (the mockup's
/// `Math.abs(N) >= 1000`).
pub const NEAR_ZERO_N: f64 = 1000.0;

/// Above this many nodes the influence line is swept in at most [`BIG_SWEEP`] steps.
pub const BIG_TRUSS: usize = 120;
/// Steps of a big truss's sweep.
pub const BIG_SWEEP: usize = 24;

/// How a member's force reads (elevation colour, table colour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    /// |N| < 1 kN (or no solution, in the table): grey.
    Zero,
    /// Tension: blue.
    Tension,
    /// Compression: orange.
    Compression,
    /// Over its limit, or the structure has no solution: dashed red (elevation only).
    Failed,
}

/// The two supports a span is measured between and its midspan node.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Span {
    /// Leftmost support (node index).
    pub left: usize,
    /// Rightmost support.
    pub right: usize,
    /// Horizontal distance between them, m.
    pub length: f64,
    /// The node nearest the middle of the line between them (deflection is read there).
    pub mid: usize,
}

/// Everything the Advanced screen shows for one netlist at one truck position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// The truss solved.
    pub truss: Truss,
    /// The solve with the truck at [`Report::x`].
    pub solution: Solution,
    /// The truck position solved, m.
    pub x: f64,
    /// How many identical trusses carry the deck ([`Netlist::copies`]).
    pub copies: u32,
    /// Supported nodes, left to right.
    pub supports: Vec<usize>,
    /// The span between the outer supports, if there are two at different x.
    pub span: Option<Span>,
    /// Elevation colour of each member (in member order).
    pub tones: Vec<Tone>,
    /// The member table, sorted by use (highest first; ties keep member order).
    pub rows: Vec<Row>,
    /// The status pill.
    pub status: Status,
    /// Determinacy, Reactions, Midspan deflection, Worst member, Steel weight.
    pub checks: Vec<Check>,
    /// Mass of all the trusses' members, kg (`copies × Σ A·L·ρ`).
    pub mass: f64,
}

impl Report {
    /// True when the solve has no solution, a load hangs on nothing, or the worst member is
    /// over its limit.
    pub fn failed(&self) -> bool {
        !self.solution.stable
            || !self.solution.loose_loads.is_empty()
            || self.worst_utilisation() > 1.0
    }

    /// The worst member's utilisation (0 when there are no members).
    pub fn worst_utilisation(&self) -> f64 {
        self.solution
            .worst_result()
            .map(|r| r.utilisation)
            .unwrap_or(0.0)
    }

    /// Member `m`'s result.
    pub fn member(&self, m: usize) -> Option<&MemberResult> {
        self.solution.members.get(m)?.as_ref()
    }
}

/// Solves the netlist with its truck where it stands ([`Netlist::truck_x`]).
pub fn analyse(net: &Netlist) -> Result<Report, SolveError> {
    analyse_at(net, net.truck_x())
}

/// Solves the netlist with the truck at `x`.
pub fn analyse_at(net: &Netlist, x: f64) -> Result<Report, SolveError> {
    let truss = net.to_truss();
    let loads = net.loads_at(&truss, x);
    let solution = solve_ordered(&truss, &loads, &net.solve_options())?;
    Ok(report::build(net, truss, solution, x))
}

/// An influence-line pick (the segmented control above the plot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pick {
    /// Stable key (`topmid`, `botmid`, `post`, `vert`, or the member id for a custom netlist).
    pub key: String,
    /// The button text ("Top chord T2–T3").
    pub name: String,
    /// The member (index into the netlist's members).
    pub member: usize,
}

/// The four influence-line picks: the mockup's Top chord T2–T3, Bottom chord B2–B3, End post
/// B0–T1 and Vertical B2–T2 when all four exist as members; otherwise the four most-used
/// members of `report` ("M8 T2–T3"). A member the netlist names in `.plot N(M3) …` comes first
/// (added if it isn't one of them).
pub fn picks(net: &Netlist, report: &Report) -> Vec<Pick> {
    let find = |a: &str, b: &str| {
        let (ia, ib) = (net.node_index(a)?, net.node_index(b)?);
        net.members
            .iter()
            .position(|m| (m.a == ia && m.b == ib) || (m.a == ib && m.b == ia))
    };
    let std = [
        ("topmid", "Top chord T2–T3", "T2", "T3"),
        ("botmid", "Bottom chord B2–B3", "B2", "B3"),
        ("post", "End post B0–T1", "B0", "T1"),
        ("vert", "Vertical B2–T2", "B2", "T2"),
    ];
    let found: Vec<Option<usize>> = std.iter().map(|(_, _, a, b)| find(a, b)).collect();
    let mut picks: Vec<Pick> = if found.iter().all(Option::is_some) {
        std.iter()
            .zip(found)
            .map(|((key, name, _, _), m)| Pick {
                key: (*key).into(),
                name: (*name).into(),
                member: m.unwrap_or(0),
            })
            .collect()
    } else {
        report
            .rows
            .iter()
            .take(4)
            .map(|r| Pick {
                key: r.id.clone(),
                name: format!("{} {}", r.id, r.nodes),
                member: r.member,
            })
            .collect()
    };
    if let Some(m) = plot_member(net)
        && !picks.iter().any(|p| p.member == m)
    {
        let mem = &net.members[m];
        let id = |n: usize| net.nodes.get(n).map(|n| n.id.as_str()).unwrap_or("?");
        picks.insert(
            0,
            Pick {
                key: mem.id.clone(),
                name: format!("{} {}\u{2013}{}", mem.id, id(mem.a), id(mem.b)),
                member: m,
            },
        );
        picks.truncate(4);
    }
    picks
}

/// The member a `.plot N(M3) vs truck.x` line names (`None` for `N(M*)` or no `.plot`).
pub fn plot_member(net: &Netlist) -> Option<usize> {
    let id = net.plot.as_ref()?.member.as_deref()?;
    net.member_index(id)
}

/// The force in one member as the truck moves (`.plot N(M8) vs truck.x 0 48 1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfluenceLine {
    /// The member (index into the netlist's members).
    pub member: usize,
    /// Its id (`M8`).
    pub id: String,
    /// `(truck x m, N kN)` samples, positive in tension (0 where there's no solution).
    pub points: Vec<[f64; 2]>,
    /// The sample with the largest |N| (the first one on ties), kN.
    pub peak: f64,
    /// Where the truck stands for the peak, m.
    pub peak_at: f64,
    /// Tension limit `A·fy`, kN.
    pub limit_tension: f64,
    /// Compression limit `min(A·fy, P_cr)`, kN (positive).
    pub limit_compression: f64,
    /// The plot's half-height in kN: `1.15 × max(|peak|, limits / 4, 1)`.
    pub scale: f64,
    /// First truck position, m.
    pub from: f64,
    /// Last truck position, m.
    pub to: f64,
}

impl InfluenceLine {
    /// Tension (blue) if the peak pulls, else compression (orange).
    pub fn tone(&self) -> Tone {
        if self.peak >= 0.0 {
            Tone::Tension
        } else {
            Tone::Compression
        }
    }

    /// A force as a fraction of the plot's half-height, clamped to ±1.3 (the mockup's `gy`).
    pub fn level(&self, kn: f64) -> f64 {
        (kn / self.scale).clamp(-1.3, 1.3)
    }

    /// Whether the tension limit is inside the plot (drawn as a red dash).
    pub fn shows_tension_limit(&self) -> bool {
        self.limit_tension < self.scale * 1.3
    }

    /// Whether the compression limit is inside the plot.
    pub fn shows_compression_limit(&self) -> bool {
        self.limit_compression < self.scale * 1.3
    }

    /// "−1004 kN".
    pub fn peak_text(&self) -> String {
        format!("{} kN", minus_fixed(self.peak, 0))
    }

    /// "truck at x = 24 m".
    pub fn peak_at_text(&self) -> String {
        format!("truck at x = {} m", js_number(self.peak_at))
    }

    /// "+2500 / −2500 kN".
    pub fn limit_text(&self) -> String {
        format!(
            "+{} / \u{2212}{} kN",
            to_fixed(self.limit_tension, 0),
            to_fixed(self.limit_compression, 0)
        )
    }

    /// "x 0 → 48 m".
    pub fn range_text(&self) -> String {
        format!("x {} → {} m", js_number(self.from), js_number(self.to))
    }

    /// The plot's accessible label: "Force in M8 as the truck moves from 0 to 48 m. Peak -1004
    /// kN at 24 m."
    pub fn label(&self) -> String {
        format!(
            "Force in {} as the truck moves from {} to {} m. Peak {} kN at {} m.",
            self.id,
            js_number(self.from),
            js_number(self.to),
            to_fixed(self.peak, 0),
            js_number(self.peak_at)
        )
    }
}

/// The "At x = 24 m" tile: "−1004 kN C" (T for tension, C for compression).
pub fn now_text(kn: f64) -> String {
    format!(
        "{} kN {}",
        minus_fixed(kn, 0),
        if kn >= 0.0 { "T" } else { "C" }
    )
}

/// The force in member `member` (kN) from a report, 0 when there's no solution.
pub fn force_kn(report: &Report, member: usize) -> f64 {
    if !report.solution.stable {
        return 0.0;
    }
    report
        .member(member)
        .map(|r| r.force / 1000.0)
        .unwrap_or(0.0)
}

/// The sweep for a netlist: the `.plot` range, else the lane's extent in 1 m steps.
pub fn sweep(net: &Netlist) -> Option<(f64, f64, f64)> {
    net.truck.as_ref()?;
    if let Some((a, b, s)) = net.plot.as_ref().and_then(|p| p.range) {
        return Some((a, b, s));
    }
    let (a, b) = net.lane_extent()?;
    (b > a).then_some((a, b, 1.0))
}

/// Up to this many nodes the influence line is solved in full at every position (the mockup's
/// way, so its truss matches to the bit); above it, by superposition ([`unit_sweep`]).
pub const EXACT_SWEEP_NODES: usize = 40;

/// Solves `truss` like [`Truss::solve`], but numbers a big truss's joints (more than
/// [`EXACT_SWEEP_NODES`]) along its longer side first, so the dense elimination meets a banded
/// stiffness matrix whatever order the netlist declared them in; results come back in the
/// truss's own order. Small trusses are solved as they are (bit-identical to the mockups).
pub fn solve_ordered(
    truss: &Truss,
    loads: &[NodalLoad],
    opts: &SolveOptions,
) -> Result<Solution, SolveError> {
    let n = truss.nodes.len();
    if n <= EXACT_SWEEP_NODES {
        return truss.solve(loads, opts);
    }
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in &truss.nodes {
        x0 = x0.min(p.x);
        x1 = x1.max(p.x);
        y0 = y0.min(p.y);
        y1 = y1.max(p.y);
    }
    let along_x = (x1 - x0) >= (y1 - y0);
    let mut perm: Vec<usize> = (0..n).collect();
    perm.sort_by(|&a, &b| {
        let (p, q) = (&truss.nodes[a], &truss.nodes[b]);
        if along_x {
            p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y))
        } else {
            p.y.total_cmp(&q.y).then(p.x.total_cmp(&q.x))
        }
    });
    let mut inv = vec![0usize; n];
    for (new, &old) in perm.iter().enumerate() {
        inv[old] = new;
    }
    let map = |i: usize| inv.get(i).copied().unwrap_or(usize::MAX);
    let sorted = Truss {
        nodes: perm.iter().map(|&o| truss.nodes[o].clone()).collect(),
        members: truss
            .members
            .iter()
            .map(|m| Member {
                a: map(m.a),
                b: map(m.b),
                ..m.clone()
            })
            .collect(),
        materials: truss.materials.clone(),
    };
    let loads: Vec<NodalLoad> = loads
        .iter()
        .map(|l| NodalLoad {
            node: map(l.node),
            ..*l
        })
        .collect();
    let s = sorted.solve(&loads, opts)?;
    let back = |new: usize| perm.get(new).copied().unwrap_or(new);
    Ok(Solution {
        mechanism: s.mechanism.map(back),
        displacements: (0..n).map(|o| s.displacements[inv[o]]).collect(),
        reactions: (0..n).map(|o| s.reactions[inv[o]]).collect(),
        active: (0..n).map(|o| s.active[inv[o]]).collect(),
        loose_loads: s.loose_loads.iter().map(|&i| back(i)).collect(),
        ..s
    })
}

/// The sweep positions for a netlist (a big truss is coarsened to [`BIG_SWEEP`] steps).
fn positions(net: &Netlist, nodes: usize) -> Option<(f64, f64, Vec<f64>)> {
    let (from, to, step) = sweep(net)?;
    let step = if nodes > BIG_TRUSS {
        step.max((to - from) / BIG_SWEEP as f64)
    } else {
        step
    };
    let n = (((to - from) / step) + 1e-9).floor().clamp(0.0, 10_000.0) as usize;
    Some((from, to, (0..=n).map(|i| from + i as f64 * step).collect()))
}

/// Every member's force over the sweep for a **unit** truck, plus the static forces: linear
/// statics, so a truck of `W` newtons (weight × share) gives `base + W·unit` — one static solve
/// and one solve per position, then any truck weight and any member without solving again.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitSweep {
    /// Truck positions, m.
    pub xs: Vec<f64>,
    /// First and last position, m.
    pub from: f64,
    /// See [`UnitSweep::from`].
    pub to: f64,
    /// Static force of each member, N (self-weight and point loads, no truck).
    pub base: Vec<f64>,
    /// Per position, each member's force per newton of truck load (N/N).
    pub unit: Vec<Vec<f64>>,
    /// The structure is stable (else every force reads 0, as the mockup draws it).
    pub stable: bool,
}

/// The [`UnitSweep`] of a netlist (`None` without a moving load or for an unsolvable model).
pub fn unit_sweep(net: &Netlist) -> Option<UnitSweep> {
    let truss = net.to_truss();
    truss.validate().ok()?;
    let (from, to, xs) = positions(net, truss.nodes.len())?;
    let opts = net.solve_options();
    let forces = |s: &Solution| -> Vec<f64> {
        s.members
            .iter()
            .map(|r| r.map(|r| r.force).unwrap_or(0.0))
            .collect()
    };
    let st = solve_ordered(&truss, &net.static_loads(&truss), &opts).ok()?;
    let stable = st.stable;
    let mut unit_net = net.clone();
    if let Some(t) = unit_net.truck.as_mut() {
        t.weight = 1.0;
        t.share = 1.0;
    }
    let unit = if stable {
        xs.iter()
            .map(|&x| {
                let loads = unit_net.truck_loads(x);
                if loads.is_empty() {
                    return vec![0.0; truss.members.len()];
                }
                match solve_ordered(&truss, &loads, &opts) {
                    Ok(s) if s.stable => forces(&s),
                    _ => vec![0.0; truss.members.len()],
                }
            })
            .collect()
    } else {
        vec![vec![0.0; truss.members.len()]; xs.len()]
    };
    Some(UnitSweep {
        xs,
        from,
        to,
        base: forces(&st),
        unit,
        stable,
    })
}

/// Member `member`'s influence line from a [`UnitSweep`] for the netlist's truck weight.
pub fn line_from_sweep(net: &Netlist, sw: &UnitSweep, member: usize) -> Option<InfluenceLine> {
    let t = net.truck.as_ref()?;
    let w = t.weight * t.share;
    let base = *sw.base.get(member)?;
    let points: Vec<[f64; 2]> = sw
        .xs
        .iter()
        .zip(&sw.unit)
        .map(|(&x, u)| {
            let v = if sw.stable {
                (base + w * u.get(member).copied().unwrap_or(0.0)) / 1000.0
            } else {
                0.0
            };
            [x, v]
        })
        .collect();
    finish_line(net, member, points, sw.from, sw.to)
}

/// The influence line of member `member`: solved in full at every position for a small truss
/// ([`influence_line_exact`], the mockup's way), by superposition for a big one
/// ([`unit_sweep`]). `None` without a moving load or for a bad member index.
pub fn influence_line(net: &Netlist, member: usize) -> Option<InfluenceLine> {
    if net.nodes.len() <= EXACT_SWEEP_NODES {
        influence_line_exact(net, member)
    } else {
        line_from_sweep(net, &unit_sweep(net)?, member)
    }
}

/// The influence line with a full solve at every truck position (what the mockup does).
pub fn influence_line_exact(net: &Netlist, member: usize) -> Option<InfluenceLine> {
    net.members.get(member)?;
    let truss = net.to_truss();
    truss.validate().ok()?;
    let (from, to, xs) = positions(net, truss.nodes.len())?;
    let base: Vec<NodalLoad> = net.static_loads(&truss);
    let opts = net.solve_options();
    let mut loads = base.clone();
    let points = xs
        .iter()
        .map(|&x| {
            loads.truncate(base.len());
            loads.extend(net.truck_loads(x));
            let v = match solve_ordered(&truss, &loads, &opts) {
                Ok(s) if s.stable => s
                    .members
                    .get(member)
                    .and_then(|r| r.as_ref())
                    .map(|r| r.force / 1000.0)
                    .unwrap_or(0.0),
                _ => 0.0,
            };
            [x, v]
        })
        .collect();
    finish_line(net, member, points, from, to)
}

/// Peak, limits and scale for sampled `points`.
fn finish_line(
    net: &Netlist,
    member: usize,
    points: Vec<[f64; 2]>,
    from: f64,
    to: f64,
) -> Option<InfluenceLine> {
    let mem = net.members.get(member)?;
    let (mut peak, mut peak_at) = (0.0f64, 0.0f64);
    for p in &points {
        if p[1].abs() > peak.abs() {
            peak = p[1];
            peak_at = p[0];
        }
    }
    let truss = net.to_truss();
    let m = truss.members.get(member)?;
    let mat = truss.materials.get(m.material)?;
    let r = check(
        0.0,
        truss.length(member),
        m.area,
        m.inertia,
        mat,
        net.k_factor,
    );
    let limit_tension = r.tension_capacity / 1000.0;
    let limit_compression = r.compression_limit / 1000.0;
    let scale = peak
        .abs()
        .max(limit_tension * 0.25)
        .max(limit_compression * 0.25)
        .max(1.0)
        * 1.15;
    Some(InfluenceLine {
        member,
        id: mem.id.clone(),
        points,
        peak,
        peak_at,
        limit_tension,
        limit_compression,
        scale,
        from,
        to,
    })
}

/// The mode word for a compression failure ("buckling" / "crushing").
fn mode_word(mode: FailureMode) -> &'static str {
    match mode {
        FailureMode::Buckling => "buckling",
        FailureMode::Crushing => "crushing",
        FailureMode::Yield => "yield",
    }
}

/// `Math.round(ut × 100)` as an integer percentage.
fn pct(ut: f64) -> i64 {
    let r = js_round(ut * 100.0);
    if r.is_finite() {
        r.clamp(-1e15, 1e15) as i64
    } else {
        0
    }
}
