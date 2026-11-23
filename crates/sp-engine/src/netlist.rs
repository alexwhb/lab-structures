//! `.str` truss netlists: "SPICE for things that stand up" (SPEC ADV-2, ADV-3, ADV-9).
//!
//! A netlist is plain text, one directive per line, comments as `*` lines and `; …` tails:
//!
//! ```text
//! * river-crossing.str
//! .units SI  kN  m
//! .material steel  E=200GPa  fy=250MPa  rho=7850   ; representative
//! .section chord  A=100cm2  I=A^2                 ; box, b/t = 24
//! .node B0  0  0  fix=xy                          ; pin
//! .node B1  8  0
//! .member M1  B0  B1  chord
//! .load dead  B1..B5  Fy=-80  +self
//! .load truck  40t  x=24  lane=B0..B6  share=0.5
//! .static linear
//! .buckle euler  K=1.0
//! .plot N(M*)  vs truck.x  0 48 1
//! ```
//!
//! [`parse`] turns text into a [`Netlist`] (or line/column [`Diagnostic`]s; it never panics),
//! [`Netlist::to_truss`] / [`Netlist::loads_at`] / [`Netlist::solve_at`] feed the one solver
//! ([`crate::truss`]), and [`write`] / [`write_truss`] turn a model back into text (the Bridge's
//! "Open in Advanced" and Share). [`set_truck`] edits the moving load in place (the toolbar's
//! Truck and x sliders).
//!
//! # Directives and units
//!
//! - `.units SI kN m` — the force unit (`N`, `kN`, `MN`) and length unit (`m`, `cm`, `mm`) for
//!   bare numbers. Default `kN` and `m`. Read before anything else, wherever it is.
//! - `.material name E=… fy=… [rho=…]` (or positional `name E fy rho`): `E` and `fy` take `Pa`,
//!   `kPa`, `MPa`, `GPa` (bare: force/length², kN/m² by default); `rho` in kg/m³. A known name
//!   (`steel`, `alu`, `timber`, `cf`, …) fills in missing values with the representative ones.
//! - `.section name A=… [I=…]` (or positional): `A` in `m2`, `cm2`, `mm2` (bare: length²); `I` in
//!   `m4`, `cm4`, `mm4` (bare: length⁴) or `A^2` (the box section, `I = A²` in SI; the default).
//! - `.node name x y [fix=xy|y|x|pin|roller]`.
//! - `.member id a b section [material]` (default material: the first one declared).
//! - `.load case nodes… Fx Fy` or `Fx=… Fy=…` (forces per node; bare numbers in the force unit,
//!   or with `N`, `kN`, `MN`, or `t` for tonnes of weight, `t × 9.81 kN`); nodes as names,
//!   `B1..B5` ranges or comma lists; `+self` adds the self-weight: shared equally among the
//!   listed nodes (the mockup's deck convention), or, with no nodes (`.load self +self`), half of
//!   each member's weight at each end (the Bridge's convention).
//! - `.load truck 40t x=24 lane=B0..B6 [share=0.5]` — the one moving load: its weight (`t` or a
//!   force), position, the lane it rolls along (split between the two lane nodes either side of
//!   `x`, by distance) and the share this truss carries.
//! - `.static [linear]`, `.buckle euler [K=1.0]`, `.plot N(M*|id) vs truck.x [from to step]`.
//! - `.story`, `.wind`, `.soil`, `.foundation`, `.dynamic` parse to warnings: they belong to the
//!   Tower / Ground worlds.
//!
//! Directives are case-insensitive; names are case-sensitive and start with a letter. All
//! values are stored in SI (m, N, Pa, m², m⁴, kg/m³).

use crate::truss::Truss;
use crate::truss::{Fix, G, Material, Member, NodalLoad, Node, Solution, SolveError, SolveOptions};
use serde::{Deserialize, Serialize};

mod edit;
mod parse;
mod write;

pub use edit::set_truck;
pub use parse::{MAX_MEMBERS, MAX_NODES, MAX_PLOT_POINTS, parse};
pub use write::{WriteTruss, write, write_truss};

/// How bad a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// The netlist can't be used until it's fixed.
    Error,
    /// Parsed, but worth knowing (another world's directive, a value filled in).
    Warning,
}

/// A problem on one line of a netlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column (in characters) where the problem starts.
    pub column: usize,
    /// Length in characters of the offending text (at least 1).
    pub len: usize,
    /// Error or warning.
    pub severity: Severity,
    /// What's wrong, in plain words ("unknown node B9").
    pub message: String,
}

impl Diagnostic {
    /// True for an error.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}:{}: {}", self.line, self.column, self.message)
    }
}

/// The force unit of bare numbers (`.units`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ForceUnit {
    /// Newtons.
    N,
    /// Kilonewtons (the default).
    #[default]
    KN,
    /// Meganewtons.
    MN,
}

impl ForceUnit {
    /// Newtons per unit.
    pub fn newtons(self) -> f64 {
        match self {
            ForceUnit::N => 1.0,
            ForceUnit::KN => 1e3,
            ForceUnit::MN => 1e6,
        }
    }
    /// The symbol (`kN`).
    pub fn symbol(self) -> &'static str {
        match self {
            ForceUnit::N => "N",
            ForceUnit::KN => "kN",
            ForceUnit::MN => "MN",
        }
    }
}

/// The length unit of bare numbers (`.units`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LengthUnit {
    /// Metres (the default).
    #[default]
    M,
    /// Centimetres.
    Cm,
    /// Millimetres.
    Mm,
}

impl LengthUnit {
    /// Units per metre (1, 100, 1000): a bare length `v` is `v / per_metre()` metres.
    pub fn per_metre(self) -> f64 {
        match self {
            LengthUnit::M => 1.0,
            LengthUnit::Cm => 100.0,
            LengthUnit::Mm => 1000.0,
        }
    }
    /// The symbol (`m`).
    pub fn symbol(self) -> &'static str {
        match self {
            LengthUnit::M => "m",
            LengthUnit::Cm => "cm",
            LengthUnit::Mm => "mm",
        }
    }
}

/// `.units SI kN m`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Units {
    /// Force unit of bare numbers.
    pub force: ForceUnit,
    /// Length unit of bare numbers.
    pub length: LengthUnit,
}

/// `.section name A=… I=…`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Name (`chord`, `Thin`).
    pub name: String,
    /// Area A, m².
    pub area: f64,
    /// Second moment of area I, m⁴.
    pub inertia: f64,
}

impl Section {
    /// A box section (`I = A²`, thin square box with b/t = 24).
    pub fn boxed(name: impl Into<String>, area: f64) -> Self {
        Self {
            name: name.into(),
            area,
            inertia: area * area,
        }
    }
    /// True when `I = A²` exactly (written as `I=A^2`).
    pub fn is_box(&self) -> bool {
        self.inertia == self.area * self.area
    }
}

/// `.member id a b section [material]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetMember {
    /// Name (`M1`).
    pub id: String,
    /// Index of the first node in [`Netlist::nodes`].
    pub a: usize,
    /// Index of the second node.
    pub b: usize,
    /// Index into [`Netlist::sections`].
    pub section: usize,
    /// Index into [`Netlist::materials`].
    pub material: usize,
}

/// One static `.load` line: the same force on each of its nodes, maybe plus the self-weight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticLoad {
    /// Load case name (`dead`).
    pub case: String,
    /// Nodes it acts on (indices into [`Netlist::nodes`]); empty with `self_weight` = the
    /// self-weight lumped at member ends.
    pub nodes: Vec<usize>,
    /// Horizontal force on each node, N.
    pub fx: f64,
    /// Vertical force on each node, N (negative = down).
    pub fy: f64,
    /// `+self`: add the truss's self-weight (shared among `nodes`, or at member ends).
    pub self_weight: bool,
}

/// The moving load: `.load truck 40t x=24 lane=B0..B6 share=0.5`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovingLoad {
    /// Load case name (`truck`).
    pub case: String,
    /// The whole vehicle's weight, N.
    pub weight: f64,
    /// The weight as written in tonnes, if it was (`40t`).
    pub tonnes: Option<f64>,
    /// Position along the lane, m.
    pub x: f64,
    /// Lane nodes (indices into [`Netlist::nodes`]), as listed.
    pub lane: Vec<usize>,
    /// Share of the weight this truss carries (0.5: one of two trusses).
    pub share: f64,
}

/// `.plot N(M8) vs truck.x 0 48 1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plot {
    /// The member plotted (`None`: `M*`, the screen's pick).
    pub member: Option<String>,
    /// The sweep `(from, to, step)`, m (`None`: the lane, 1 m steps).
    pub range: Option<(f64, f64, f64)>,
}

/// How a netlist applies the self-weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfWeight {
    /// Not at all.
    None,
    /// Half of each member's weight at each of its ends (`.load self +self`, the Bridge).
    MemberEnds,
    /// The whole weight shared equally among a load line's nodes (the mockup's deck).
    Shared,
}

/// A parsed netlist, in SI units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Netlist {
    /// The file name from the first comment line (`river-crossing.str`), if it names one.
    pub title: Option<String>,
    /// The `*` comment lines at the top after the title, without the `*` (written back).
    pub notes: Vec<String>,
    /// Units of bare numbers (kept for writing the netlist back).
    pub units: Units,
    /// Materials; [`Material::name`] is the netlist name (`steel`).
    pub materials: Vec<Material>,
    /// Sections.
    pub sections: Vec<Section>,
    /// Nodes, with their supports.
    pub nodes: Vec<Node>,
    /// Members.
    pub members: Vec<NetMember>,
    /// Static loads, in the order written (all applied together).
    pub loads: Vec<StaticLoad>,
    /// The moving load, if any.
    pub truck: Option<MovingLoad>,
    /// Effective-length factor for Euler buckling (`.buckle euler K=…`, default 1).
    pub k_factor: f64,
    /// The `.plot` request, if any.
    pub plot: Option<Plot>,
    /// Warnings found while parsing (other worlds' directives, values filled in).
    pub warnings: Vec<Diagnostic>,
}

impl Default for Netlist {
    fn default() -> Self {
        Self {
            title: None,
            notes: Vec::new(),
            units: Units::default(),
            materials: Vec::new(),
            sections: Vec::new(),
            nodes: Vec::new(),
            members: Vec::new(),
            loads: Vec::new(),
            truck: None,
            k_factor: 1.0,
            plot: None,
            warnings: Vec::new(),
        }
    }
}

impl Netlist {
    /// The file stem of [`Netlist::title`] (`river-crossing`), if any.
    pub fn stem(&self) -> Option<&str> {
        let t = self.title.as_deref()?;
        Some(t.strip_suffix(".str").unwrap_or(t))
    }

    /// Index of the node called `id`.
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Index of the member called `id`.
    pub fn member_index(&self, id: &str) -> Option<usize> {
        self.members.iter().position(|m| m.id == id)
    }

    /// The truss to solve: nodes, members (area and inertia from their section) and materials.
    pub fn to_truss(&self) -> Truss {
        let members = self
            .members
            .iter()
            .map(|m| {
                let (area, inertia) = self
                    .sections
                    .get(m.section)
                    .map(|s| (s.area, s.inertia))
                    .unwrap_or((0.0, 0.0));
                Member {
                    id: m.id.clone(),
                    a: m.a,
                    b: m.b,
                    area,
                    inertia,
                    material: m.material,
                }
            })
            .collect();
        Truss {
            nodes: self.nodes.clone(),
            members,
            materials: self.materials.clone(),
        }
    }

    /// The section name of every member (the Bridge's Thin / Medium / Thick).
    pub fn member_sections(&self) -> Vec<String> {
        self.members
            .iter()
            .map(|m| {
                self.sections
                    .get(m.section)
                    .map(|s| s.name.clone())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// How the self-weight is applied (the first `+self` line decides).
    pub fn self_weight(&self) -> SelfWeight {
        match self.loads.iter().find(|l| l.self_weight) {
            None => SelfWeight::None,
            Some(l) if l.nodes.is_empty() => SelfWeight::MemberEnds,
            Some(_) => SelfWeight::Shared,
        }
    }

    /// The point loads written in the static `.load` lines, one per node and line, N (no
    /// self-weight, no moving load).
    pub fn point_loads(&self) -> Vec<NodalLoad> {
        let mut out = Vec::new();
        for l in &self.loads {
            if l.fx == 0.0 && l.fy == 0.0 {
                continue;
            }
            for &n in &l.nodes {
                out.push(NodalLoad {
                    node: n,
                    fx: l.fx,
                    fy: l.fy,
                });
            }
        }
        out
    }

    /// The self-weight of one truss, N: `Σ A·L` per material times `ρ·g` (the mockup's
    /// `vol × ρ × 9.81`).
    pub fn self_weight_total(&self, truss: &Truss) -> f64 {
        let mut vol = vec![0.0; truss.materials.len()];
        for (i, m) in truss.members.iter().enumerate() {
            if let Some(v) = vol.get_mut(m.material) {
                *v += m.area * truss.length(i);
            }
        }
        vol.iter()
            .zip(&truss.materials)
            .map(|(v, mat)| v * mat.rho * G)
            .filter(|w| w.is_finite())
            .sum()
    }

    /// The static loads as nodal loads, N: each line's force on each of its nodes followed by
    /// its share of the self-weight.
    pub fn static_loads(&self, truss: &Truss) -> Vec<NodalLoad> {
        let mut out = Vec::new();
        let mut total: Option<f64> = None;
        for l in &self.loads {
            for &n in &l.nodes {
                if l.fx != 0.0 || l.fy != 0.0 {
                    out.push(NodalLoad {
                        node: n,
                        fx: l.fx,
                        fy: l.fy,
                    });
                }
            }
            if l.self_weight {
                if l.nodes.is_empty() {
                    out.extend(truss.self_weight(1.0, &[]));
                } else {
                    let w = *total.get_or_insert_with(|| self.self_weight_total(truss));
                    let each = w / l.nodes.len() as f64;
                    out.extend(l.nodes.iter().map(|&n| NodalLoad::down(n, each)));
                }
            }
        }
        out
    }

    /// The lane's nodes ordered by x (ties keep their written order).
    pub fn lane_nodes(&self) -> Vec<usize> {
        let Some(t) = &self.truck else {
            return Vec::new();
        };
        let mut lane: Vec<usize> = t
            .lane
            .iter()
            .copied()
            .filter(|&n| n < self.nodes.len())
            .collect();
        lane.sort_by(|&a, &b| self.nodes[a].x.total_cmp(&self.nodes[b].x));
        lane
    }

    /// The lane's extent `(first x, last x)`, m.
    pub fn lane_extent(&self) -> Option<(f64, f64)> {
        let lane = self.lane_nodes();
        let (first, last) = (lane.first()?, lane.last()?);
        Some((self.nodes[*first].x, self.nodes[*last].x))
    }

    /// The moving load at truck position `x` as nodal loads, N: `weight × share` split between
    /// the lane nodes either side of `x` by distance. Nothing when off the lane or weightless.
    pub fn truck_loads(&self, x: f64) -> Vec<NodalLoad> {
        let Some(t) = &self.truck else {
            return Vec::new();
        };
        let lane = self.lane_nodes();
        if lane.len() < 2 || t.weight.is_nan() || t.weight <= 0.0 || !x.is_finite() {
            return Vec::new();
        }
        let xs: Vec<f64> = lane.iter().map(|&n| self.nodes[n].x).collect();
        let (x0, xn) = (xs[0], xs[xs.len() - 1]);
        if x < x0 || x > xn {
            return Vec::new();
        }
        let pt = t.weight * t.share;
        let j = xs
            .iter()
            .rposition(|&xi| xi <= x)
            .unwrap_or(0)
            .min(lane.len() - 2);
        let span = xs[j + 1] - xs[j];
        let f = if span > 0.0 { (x - xs[j]) / span } else { 0.0 };
        vec![
            NodalLoad::down(lane[j], pt * (1.0 - f)),
            NodalLoad::down(lane[j + 1], pt * f),
        ]
    }

    /// Static loads plus the moving load at `x`.
    pub fn loads_at(&self, truss: &Truss, x: f64) -> Vec<NodalLoad> {
        let mut l = self.static_loads(truss);
        l.extend(self.truck_loads(x));
        l
    }

    /// The solve options the netlist asks for (`.buckle euler K=…`).
    pub fn solve_options(&self) -> SolveOptions {
        SolveOptions {
            removed: Vec::new(),
            k_factor: self.k_factor,
        }
    }

    /// Solves with the truck at `x` (its written position: [`Netlist::truck_x`]).
    pub fn solve_at(&self, x: f64) -> Result<Solution, SolveError> {
        let truss = self.to_truss();
        truss.solve(&self.loads_at(&truss, x), &self.solve_options())
    }

    /// The truck's written position (0 without a truck).
    pub fn truck_x(&self) -> f64 {
        self.truck.as_ref().map(|t| t.x).unwrap_or(0.0)
    }

    /// How many identical trusses carry the deck: `1 / share` of the moving load (2 for the
    /// mockup's `share=0.5`), else 1. Weights are reported for all of them.
    pub fn copies(&self) -> u32 {
        match &self.truck {
            Some(t) if t.share > 0.0 && t.share <= 1.0 => {
                (1.0 / t.share).round().clamp(1.0, 1000.0) as u32
            }
            _ => 1,
        }
    }
}

/// A fix as written after `fix=` (`xy`, `y`, `x`), or `None` for a free node.
pub fn fix_text(fix: Fix) -> Option<&'static str> {
    match (fix.x, fix.y) {
        (true, true) => Some("xy"),
        (false, true) => Some("y"),
        (true, false) => Some("x"),
        (false, false) => None,
    }
}
