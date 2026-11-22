//! The Advanced toolbar's truss generator: the mockup's `build(style, braced)` and the netlist
//! lines it renders (`Advanced.dc.html`, SPEC ADV-1 / ADV-10).
//!
//! A 48 m through-truss of six 8 m panels, 6 m deep: bottom nodes `B0…B6`, top nodes `T1…T5`,
//! chords and end posts in the `chord` section (`A = 25 cm² × size`), verticals and diagonals in
//! the `web` section (`A = 15 cm² × size`), both box sections (`I = A²`). Pratt, Howe and Warren
//! differ in their four diagonals; "Cross-braced" adds the other diagonal in each inner panel.
//! Loads: 80 kN per deck node plus the self-weight shared over `B1…B5` (10 kN/m of deck), and the
//! truck, half per truss, rolling along `B0…B6`.
//!
//! [`generate`] returns the text, line for line as the mockup shows it, and the parsed model.
//! The model is built with the mockup's own arithmetic (`A = 0.0025 × size`, which for a few
//! sizes is one ulp off the `cm²` value the text shows), so results match the mockup to the
//! last bit; parsing the text gives the same model to within that ulp.

use crate::netlist::text::{js_number, to_fixed};
use crate::netlist::{MovingLoad, NetMember, Netlist, Plot, Section, StaticLoad};
use crate::truss::{Fix, G, Material, Node};
use serde::{Deserialize, Serialize};

/// Truss style (the toolbar's segmented control).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Style {
    /// Diagonals in tension under deck load (slope down to the middle).
    #[default]
    Pratt,
    /// Diagonals in compression (slope up to the middle).
    Howe,
    /// Alternating diagonals.
    Warren,
}

impl Style {
    /// All three, in toolbar order.
    pub const ALL: [Style; 3] = [Style::Pratt, Style::Howe, Style::Warren];

    /// The display name ("Pratt").
    pub fn name(self) -> &'static str {
        match self {
            Style::Pratt => "Pratt",
            Style::Howe => "Howe",
            Style::Warren => "Warren",
        }
    }

    /// The four diagonals (node pairs) of the style.
    fn diagonals(self) -> [(&'static str, &'static str); 4] {
        match self {
            Style::Pratt => [("T1", "B2"), ("T2", "B3"), ("B3", "T4"), ("B4", "T5")],
            Style::Howe => [("B1", "T2"), ("B2", "T3"), ("T3", "B4"), ("T4", "B5")],
            Style::Warren => [("T1", "B2"), ("B2", "T3"), ("T3", "B4"), ("B4", "T5")],
        }
    }
}

/// The toolbar's materials (the mockup's `mats()`; representative values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MaterialKind {
    /// Structural steel: E 200 GPa, fy 250 MPa, ρ 7850.
    #[default]
    Steel,
    /// Aluminium alloy: E 69 GPa, fy 240 MPa, ρ 2700.
    Aluminum,
    /// Glued-laminated timber: E 12 GPa, fy 24 MPa, ρ 500.
    Glulam,
    /// Carbon-fibre composite: E 70 GPa, fy 600 MPa, ρ 1600.
    CarbonFiber,
}

impl MaterialKind {
    /// All four, in the select's order.
    pub const ALL: [MaterialKind; 4] = [
        MaterialKind::Steel,
        MaterialKind::Aluminum,
        MaterialKind::Glulam,
        MaterialKind::CarbonFiber,
    ];

    /// The netlist name (`steel`, `alu`, `timber`, `cf`).
    pub fn key(self) -> &'static str {
        match self {
            MaterialKind::Steel => "steel",
            MaterialKind::Aluminum => "alu",
            MaterialKind::Glulam => "timber",
            MaterialKind::CarbonFiber => "cf",
        }
    }

    /// The display name ("Steel", "Aluminum", "Glulam", "Carbon fiber").
    pub fn name(self) -> &'static str {
        match self {
            MaterialKind::Steel => "Steel",
            MaterialKind::Aluminum => "Aluminum",
            MaterialKind::Glulam => "Glulam",
            MaterialKind::CarbonFiber => "Carbon fiber",
        }
    }

    /// The kind with netlist name `key`.
    pub fn from_key(key: &str) -> Option<MaterialKind> {
        Self::ALL.into_iter().find(|m| m.key() == key)
    }

    /// The material (named by its netlist key): E, fy in Pa, ρ in kg/m³.
    pub fn material(self) -> Material {
        let (e, fy, rho) = match self {
            MaterialKind::Steel => (200e9, 250e6, 7850.0),
            MaterialKind::Aluminum => (69e9, 240e6, 2700.0),
            MaterialKind::Glulam => (12e9, 24e6, 500.0),
            MaterialKind::CarbonFiber => (70e9, 600e6, 1600.0),
        };
        Material::new(self.key(), e, fy, rho)
    }
}

/// Smallest and largest Size (the mockup's slider: 1–20).
pub const SIZE_RANGE: (u32, u32) = (1, 20);
/// Truck weight range, t (the mockup's slider: 0–120).
pub const TRUCK_RANGE: (f64, f64) = (0.0, 120.0);
/// Span, m: the x slider runs 0–48 in 0.5 m steps.
pub const SPAN: f64 = 48.0;
/// The document's file name.
pub const FILE_NAME: &str = "river-crossing.str";

/// The toolbar state that generates a netlist.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// Pratt / Howe / Warren.
    pub style: Style,
    /// Cross-braced: both diagonals in each inner panel.
    pub braced: bool,
    /// Material.
    pub material: MaterialKind,
    /// Size 1–20: chord `A = 25 cm² × size`, web `A = 15 cm² × size`.
    pub size: u32,
    /// Truck weight, tonnes (0–120).
    pub truck_t: f64,
    /// Truck position, m (0–48).
    pub x: f64,
}

impl Default for Params {
    /// The mockup's defaults: Pratt, not braced, steel, size 4, 40 t at x = 24 m.
    fn default() -> Self {
        Self {
            style: Style::Pratt,
            braced: false,
            material: MaterialKind::Steel,
            size: 4,
            truck_t: 40.0,
            x: 24.0,
        }
    }
}

impl Params {
    /// The same state with every value inside the toolbar's ranges (x snapped to 0.5 m is not
    /// required; any finite x in 0–48 is kept).
    pub fn clamped(mut self) -> Self {
        self.size = self.size.clamp(SIZE_RANGE.0, SIZE_RANGE.1);
        self.truck_t = if self.truck_t.is_finite() {
            self.truck_t.clamp(TRUCK_RANGE.0, TRUCK_RANGE.1)
        } else {
            0.0
        };
        self.x = if self.x.is_finite() {
            self.x.clamp(0.0, SPAN)
        } else {
            0.0
        };
        self
    }
}

/// What a member is, for its section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bottom,
    Top,
    Post,
    Vertical,
    Diagonal,
}

impl Kind {
    fn chord(self) -> bool {
        matches!(self, Kind::Bottom | Kind::Top | Kind::Post)
    }
}

/// One rendered netlist line: the directive (or `*` comment) in the accent, its arguments, and
/// a `; …` comment (without the `; `).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// `.node`, `* river-crossing.str`.
    pub dir: String,
    /// `B0  0  0  fix=xy`.
    pub args: String,
    /// `pin` (empty for none).
    pub comment: String,
}

impl Line {
    /// The line as text: `dir args  ; comment` (trailing spaces trimmed).
    pub fn text(&self) -> String {
        let mut s = format!("{} {}", self.dir, self.args);
        if !self.comment.is_empty() {
            s.push_str("  ; ");
            s.push_str(&self.comment);
        }
        s.truncate(s.trim_end().len());
        s
    }
}

/// A generated netlist.
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    /// The state it came from (clamped).
    pub params: Params,
    /// The lines, as the mockup renders them.
    pub lines: Vec<Line>,
    /// The text (the lines joined with `\n`, ending in a newline).
    pub text: String,
    /// The model, with the mockup's exact arithmetic.
    pub netlist: Netlist,
}

/// Builds the truss, its netlist lines and its model for a toolbar state.
pub fn generate(params: &Params) -> Generated {
    let p = params.clamped();
    let mat = p.material.material();
    let size = p.size as f64;

    // nodes: B0..B6 then T1..T5
    let mut nodes = Vec::new();
    for i in 0..=6 {
        let fix = match i {
            0 => Fix::PIN,
            6 => Fix::ROLLER,
            _ => Fix::FREE,
        };
        nodes.push(Node::new(format!("B{i}"), i as f64 * 8.0, 0.0, fix));
    }
    for i in 1..6 {
        nodes.push(Node::new(format!("T{i}"), i as f64 * 8.0, 6.0, Fix::FREE));
    }
    let ix = |id: &str| nodes.iter().position(|n| n.id == id).unwrap_or(0);

    // members, deduplicated by node pair, in the mockup's order
    let mut pairs: Vec<(String, String, Kind)> = Vec::new();
    let mut add = |a: String, b: String, kind: Kind| {
        let dup = pairs
            .iter()
            .any(|(x, y, _)| (*x == a && *y == b) || (*x == b && *y == a));
        if !dup {
            pairs.push((a, b, kind));
        }
    };
    for i in 0..6 {
        add(format!("B{i}"), format!("B{}", i + 1), Kind::Bottom);
    }
    for i in 1..5 {
        add(format!("T{i}"), format!("T{}", i + 1), Kind::Top);
    }
    add("B0".into(), "T1".into(), Kind::Post);
    add("T5".into(), "B6".into(), Kind::Post);
    for i in 1..6 {
        add(format!("B{i}"), format!("T{i}"), Kind::Vertical);
    }
    for (a, b) in p.style.diagonals() {
        add(a.into(), b.into(), Kind::Diagonal);
    }
    if p.braced {
        for i in 1..5 {
            add(format!("T{i}"), format!("B{}", i + 1), Kind::Diagonal);
            add(format!("B{i}"), format!("T{}", i + 1), Kind::Diagonal);
        }
    }

    let chord_a = 0.0025 * size;
    let web_a = 0.0015 * size;
    let sections = vec![Section::boxed("chord", chord_a), Section::boxed("web", web_a)];
    let members: Vec<NetMember> = pairs
        .iter()
        .enumerate()
        .map(|(k, (a, b, kind))| NetMember {
            id: format!("M{}", k + 1),
            a: ix(a),
            b: ix(b),
            section: if kind.chord() { 0 } else { 1 },
            material: 0,
        })
        .collect();

    // the netlist lines (renderVals' L(...) calls)
    let mut lines = Vec::new();
    let mut l = |dir: &str, args: String, cmt: &str| {
        lines.push(Line {
            dir: dir.into(),
            args,
            comment: cmt.into(),
        })
    };
    l(&format!("* {FILE_NAME}"), String::new(), "");
    l(
        "*",
        format!(
            "48 m {}{} through-truss (one of two)",
            p.style.name(),
            if p.braced { ", cross-braced" } else { "" }
        ),
        "",
    );
    l(".units", "SI  kN  m".into(), "");
    l(
        ".material",
        format!(
            "{}  E={}GPa  fy={}MPa  rho={}",
            p.material.key(),
            js_number(mat.e / 1e9),
            js_number(mat.fy / 1e6),
            js_number(mat.rho)
        ),
        "representative",
    );
    l(
        ".section",
        format!("chord  A={}cm2  I=A^2", to_fixed(chord_a * 1e4, 0)),
        "box, b/t = 24",
    );
    l(
        ".section",
        format!("web    A={}cm2  I=A^2", to_fixed(web_a * 1e4, 0)),
        "",
    );
    for n in &nodes {
        let (fix, cmt) = match n.id.as_str() {
            "B0" => ("  fix=xy", "pin"),
            "B6" => ("  fix=y", "roller"),
            _ => ("", ""),
        };
        l(
            ".node",
            format!("{}  {}  {}{fix}", n.id, js_number(n.x), js_number(n.y)),
            cmt,
        );
    }
    for (m, (a, b, kind)) in members.iter().zip(&pairs) {
        let k = if kind.chord() { "chord" } else { "web" };
        l(".member", format!("{}  {a}  {b}  {k}", m.id), "");
    }
    l(".load", "dead  B1..B5  Fy=-80  +self".into(), "10 kN/m deck");
    l(
        ".load",
        format!(
            "truck  {}t  x={}  lane=B0..B6  share=0.5",
            js_number(p.truck_t),
            js_number(p.x)
        ),
        "per truss",
    );
    l(".static", String::new(), "linear, small displacement");
    l(".buckle", "euler  K=1.0".into(), "");
    l(".plot", "N(M*)  vs truck.x  0 48 1".into(), "");

    let mut text: String = lines.iter().map(|l| l.text() + "\n").collect();
    text.shrink_to_fit();

    let netlist = Netlist {
        title: Some(FILE_NAME.into()),
        notes: vec![lines[1].args.clone()],
        materials: vec![mat],
        sections,
        members,
        loads: vec![StaticLoad {
            case: "dead".into(),
            nodes: (1..=5).collect(),
            fx: 0.0,
            fy: -80.0 * 1e3,
            self_weight: true,
        }],
        truck: Some(MovingLoad {
            case: "truck".into(),
            weight: p.truck_t * G * 1e3,
            tonnes: Some(p.truck_t),
            x: p.x,
            lane: (0..=6).collect(),
            share: 0.5,
        }),
        k_factor: 1.0,
        plot: Some(Plot {
            member: None,
            range: Some((0.0, SPAN, 1.0)),
        }),
        nodes,
        ..Netlist::default()
    };
    Generated {
        params: p,
        lines,
        text,
        netlist,
    }
}
