//! The builder's structure as a `.str` netlist and back (SPEC NAV-4 "Open in Advanced",
//! NAV-6 Share and ⌘V), on [`crate::netlist`]'s writer and parser.
//!
//! [`write`]: one `.material` per piece type used (`road`, `steel`, …: E, f_t as `fy`, and a
//! density that gives the type's mass per metre), one `.section` per type (`Road A=200cm2
//! I=…`, I = k·A²), `.node`s with foundations as `fix=xy`, `.member`s, the crates as `.load
//! weights … Fy=-5t`, `.load self +self`, `.static linear`, `.buckle euler K=1.0`. The level,
//! scene, conditions and each foundation's kind ride along as `*` comment lines:
//!
//! ```text
//! * river-level.str
//! * Structures Playground builder · 1 · First crossing
//! * level river · scene river
//! * cond wind=0 dir=1 snow=0 quake=0 soil=dense traffic=level fire=0
//! * found J1=footing J2=footing
//! ```
//!
//! [`read`] takes any netlist whose joints sit on whole metres inside a scene: the scene its
//! `* level … · scene …` note names when the joints fit it, else the river scene when every
//! joint fits the river's view and none is underground, else open ground (y ≥ 0).
//! Piece types come from the member's material (or section) name; unknown materials build in
//! Steel. Supports become footings unless a `* found` line names a pile.

use super::data::{Foundation, LevelKey, PieceType, Scene, Soil, TrafficSet};
use super::design::{Design, Joint, Piece};
use super::setup::Conditions;
use crate::jsnum::to_string;
use crate::netlist::{self, Netlist, Section, WriteTruss};
use crate::truss::{Fix, Material, Member, NodalLoad, Node, Truss};

/// Why a netlist can't go into the builder.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ReadError {
    /// It doesn't parse (the first error, with its line).
    #[error("line {line}: {message}")]
    Parse {
        /// 1-based line.
        line: usize,
        /// The parser's message.
        message: String,
    },
    /// No joints.
    #[error("it has no joints")]
    Empty,
    /// More joints than the builder takes.
    #[error("it has {0} joints; the builder takes at most {MAX_JOINTS}")]
    TooBig(usize),
    /// A joint is not on whole metres, or outside both scenes.
    #[error("joint {0} is not on the building grid")]
    OffGrid(String),
    /// Two joints at one spot.
    #[error("joints {0} and {1} are at the same spot")]
    Overlap(String, String),
    /// A member joins a joint to itself or repeats a pair.
    #[error("member {0} doubles up")]
    Duplicate(String),
}

/// The most joints a netlist may bring in.
pub const MAX_JOINTS: usize = 400;

/// What a netlist brings in.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    /// The structure (joints renamed `J1…`, pieces `M1…` in order).
    pub design: Design,
    /// The scene it fits.
    pub scene: Scene,
    /// The level named in a `* level` line, if any.
    pub level: Option<LevelKey>,
    /// The conditions named in a `* cond` line, if any.
    pub cond: Option<Conditions>,
}

fn material_of(k: PieceType) -> Material {
    let t = k.spec();
    Material {
        name: k.key().to_string(),
        e: t.e,
        fy: t.f_t,
        rho: k.mass_per_m() / t.area,
    }
}

/// Writes the design as a netlist named `name` (`river-level`, `sandbox`).
pub fn write(
    design: &Design,
    level: LevelKey,
    scene: Scene,
    cond: &Conditions,
    name: &str,
) -> String {
    let mut kinds: Vec<PieceType> = Vec::new();
    for m in &design.members {
        if !kinds.contains(&m.kind) {
            kinds.push(m.kind);
        }
    }
    let nodes: Vec<Node> = design
        .nodes
        .iter()
        .map(|n| {
            Node::new(
                n.id.clone(),
                n.x,
                n.y,
                if n.sup.is_some() { Fix::PIN } else { Fix::FREE },
            )
        })
        .collect();
    let mut members = Vec::new();
    let mut sections = Vec::new();
    for m in &design.members {
        let (Some(a), Some(b)) = (design.node_index(&m.a), design.node_index(&m.b)) else {
            continue;
        };
        let t = m.kind.spec();
        members.push(Member {
            id: m.id.clone(),
            a,
            b,
            area: t.area,
            inertia: t.k * t.area * t.area,
            material: kinds.iter().position(|&k| k == m.kind).unwrap_or(0),
        });
        sections.push(m.kind.name().to_string());
    }
    let truss = Truss {
        nodes,
        members,
        materials: kinds.iter().map(|&k| material_of(k)).collect(),
    };
    let loads: Vec<NodalLoad> = design
        .loads
        .iter()
        .filter_map(|(id, t)| {
            design
                .node_index(id)
                .map(|i| NodalLoad::down(i, t * 9810.0))
        })
        .collect();
    let mut net = Netlist::from_truss(&WriteTruss {
        name,
        comment: &format!(
            "Structures Playground builder · {}",
            super::setup::spec(level, scene).name
        ),
        truss: &truss,
        sections: &sections,
        loads: &loads,
        self_weight: true,
    });
    net.notes
        .push(format!("level {} · scene {}", level.key(), scene.key()));
    net.notes.push(format!(
        "cond wind={} dir={} snow={} quake={} soil={} traffic={} fire={}",
        to_string(cond.wind),
        to_string(cond.dir),
        to_string(cond.snow),
        to_string(cond.quake),
        cond.soil.key(),
        cond.traffic.key(),
        to_string(cond.fire)
    ));
    let found: Vec<String> = design
        .nodes
        .iter()
        .filter_map(|n| n.sup.map(|f| format!("{}={}", n.id, f.key())))
        .collect();
    if !found.is_empty() {
        net.notes.push(format!("found {}", found.join(" ")));
    }
    netlist::write(&net)
}

fn parse_cond(line: &str) -> Option<Conditions> {
    let mut c = Conditions {
        wind: 0.0,
        dir: 1.0,
        snow: 0.0,
        quake: 0.0,
        soil: Soil::Dense,
        traffic: TrafficSet::Level,
        fire: 0.0,
    };
    for kv in line.split_whitespace() {
        let (k, v) = kv.split_once('=')?;
        let num = || v.parse::<f64>().ok().filter(|x| x.is_finite());
        match k {
            "wind" => c.wind = num()?.clamp(0.0, 100.0),
            "dir" => c.dir = if num()? < 0.0 { -1.0 } else { 1.0 },
            "snow" => c.snow = num()?.clamp(0.0, 2000.0),
            "quake" => c.quake = num()?.clamp(0.0, 2.0),
            "fire" => c.fire = num()?.clamp(0.0, 1200.0),
            "soil" => c.soil = Soil::from_key(v)?,
            "traffic" => c.traffic = TrafficSet::from_key(v)?,
            _ => {}
        }
    }
    Some(c)
}

/// Reads a netlist into the builder (see the module docs).
pub fn read(text: &str) -> Result<Imported, ReadError> {
    let net = netlist::parse(text).map_err(|diags| {
        let d = diags
            .iter()
            .find(|d| d.is_error())
            .or(diags.first())
            .cloned();
        match d {
            Some(d) => ReadError::Parse {
                line: d.line,
                message: d.message,
            },
            None => ReadError::Parse {
                line: 1,
                message: "not a netlist".into(),
            },
        }
    })?;
    if net.nodes.is_empty() {
        return Err(ReadError::Empty);
    }
    if net.nodes.len() > MAX_JOINTS {
        return Err(ReadError::TooBig(net.nodes.len()));
    }
    let mut level = None;
    let mut named_scene = None;
    let mut cond = None;
    let mut piles: Vec<String> = Vec::new();
    for note in &net.notes {
        let note = note.trim();
        if let Some(rest) = note.strip_prefix("level ") {
            level = rest.split_whitespace().next().and_then(LevelKey::from_key);
            named_scene = rest
                .split_once("scene ")
                .and_then(|(_, k)| k.split_whitespace().next())
                .and_then(Scene::from_key);
        } else if let Some(rest) = note.strip_prefix("cond ") {
            cond = parse_cond(rest);
        } else if let Some(rest) = note.strip_prefix("found ") {
            for kv in rest.split_whitespace() {
                if let Some((id, k)) = kv.split_once('=')
                    && Foundation::from_key(k) == Some(Foundation::Pile)
                {
                    piles.push(id.to_string());
                }
            }
        }
    }
    // grid and scene
    for n in &net.nodes {
        if !(n.x.is_finite() && n.y.is_finite())
            || (n.x - n.x.round()).abs() > 1e-6
            || (n.y - n.y.round()).abs() > 1e-6
        {
            return Err(ReadError::OffGrid(n.id.clone()));
        }
    }
    let fits = |scene: Scene| {
        net.nodes.iter().all(|n| {
            let (x, y) = (n.x.round(), n.y.round());
            match scene {
                Scene::River => {
                    (-20.0..=76.0).contains(&x)
                        && (-12.0..=40.0).contains(&y)
                        && y >= Scene::River.ground(x)
                }
                Scene::Flat => (-40.0..=240.0).contains(&x) && (0.0..=200.0).contains(&y),
            }
        })
    };
    // the `* level … · scene …` note wins when the joints fit that scene; else the river when
    // they fit it, else open ground
    let scene = match named_scene {
        Some(sc) if fits(sc) => sc,
        _ if fits(Scene::River) => Scene::River,
        _ => {
            if let Some(n) = net.nodes.iter().find(|n| {
                let (x, y) = (n.x.round(), n.y.round());
                !((-40.0..=240.0).contains(&x) && (0.0..=200.0).contains(&y))
            }) {
                return Err(ReadError::OffGrid(n.id.clone()));
            }
            Scene::Flat
        }
    };
    let mut nodes: Vec<Joint> = Vec::with_capacity(net.nodes.len());
    for (i, n) in net.nodes.iter().enumerate() {
        let (x, y) = (n.x.round() + 0.0, n.y.round() + 0.0);
        if let Some(o) = nodes.iter().find(|o| o.x == x && o.y == y) {
            return Err(ReadError::Overlap(o.id.clone(), n.id.clone()));
        }
        let fixed = n.fix.x || n.fix.y;
        nodes.push(Joint {
            id: format!("J{}", i + 1),
            x,
            y,
            sup: fixed.then(|| {
                if piles.contains(&n.id) {
                    Foundation::Pile
                } else {
                    Foundation::Footing
                }
            }),
            locked: false,
        });
    }
    let mut members: Vec<Piece> = Vec::with_capacity(net.members.len());
    for (i, m) in net.members.iter().enumerate() {
        let (a, b) = (format!("J{}", m.a + 1), format!("J{}", m.b + 1));
        if m.a == m.b
            || m.a >= nodes.len()
            || m.b >= nodes.len()
            || members
                .iter()
                .any(|o| (o.a == a && o.b == b) || (o.a == b && o.b == a))
        {
            return Err(ReadError::Duplicate(m.id.clone()));
        }
        let by_mat = net
            .materials
            .get(m.material)
            .and_then(|x| PieceType::from_key(&x.name).or(PieceType::from_name(&x.name)));
        let by_sec = net
            .sections
            .get(m.section)
            .and_then(|s: &Section| PieceType::from_name(&s.name));
        let mut kind = by_mat.or(by_sec).unwrap_or(PieceType::Steel);
        if !kind.allowed_in(scene) {
            kind = if scene == Scene::River {
                PieceType::Road
            } else {
                PieceType::Floor
            };
        }
        members.push(Piece {
            id: format!("M{}", i + 1),
            a,
            b,
            kind,
            locked: false,
        });
    }
    let mut design = Design {
        n_n: nodes.len() as u32,
        n_m: members.len() as u32,
        nodes,
        members,
        loads: vec![],
    };
    for l in net.point_loads() {
        if l.node < design.nodes.len() && l.fy < 0.0 {
            let id = design.nodes[l.node].id.clone();
            let t = (design.load(&id) + -l.fy / 9810.0).min(super::data::MAX_CRATE_T);
            let t = (t * 1000.0).round() / 1000.0;
            design.set_load(&id, t);
        }
    }
    Ok(Imported {
        design,
        scene,
        level,
        cond,
    })
}
