//! Netlist → text: [`write`] for any [`Netlist`], [`write_truss`] for a structure built
//! elsewhere (the Bridge's "Open in Advanced", NAV-4 / ADV-9). Parsing the output gives the same
//! model back (to ~12 significant digits).

use super::text::{js_number, tidy};
use super::{MovingLoad, NetMember, Netlist, Plot, Section, StaticLoad, fix_text};
use crate::truss::{G, NodalLoad, Truss};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// A structure to write as a netlist (the Bridge's request in `PROGRESS.md`).
#[derive(Debug, Clone, Copy)]
pub struct WriteTruss<'a> {
    /// The document name: a file stem (`sandbox`) or file name (`sandbox.str`); written as the
    /// first comment line `* sandbox.str`.
    pub name: &'a str,
    /// A second comment line (`Bridge · River crossing`); empty for none.
    pub comment: &'a str,
    /// The structure.
    pub truss: &'a Truss,
    /// The section name of each member (`Thin`, `Medium`, `Thick`); members past the end of the
    /// slice (or an empty slice) get automatic names (`s1`, `s2`, … per distinct area).
    pub sections: &'a [String],
    /// Point loads, N (several on one node add up).
    pub loads: &'a [NodalLoad],
    /// Add the self-weight, half of each member's weight at each end (`.load self +self`).
    pub self_weight: bool,
}

/// Writes a structure as a netlist (`.units SI kN m`, `.material`s, one `.section` per
/// section name, `.node`s with `fix=`, `.member`s, `.load`s in kN (or `t` when the force is a
/// whole number of tonnes to 3 decimals), `.static linear`, `.buckle euler K=1.0`).
pub fn write_truss(w: &WriteTruss) -> String {
    write(&Netlist::from_truss(w))
}

/// A name that parses back: letters, digits, `_`, `-`, `'`, starting with a letter; made unique
/// within `used`.
fn safe_id(raw: &str, prefix: char, used: &mut HashSet<String>) -> String {
    let mut s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '\'') {
                c
            } else {
                '_'
            }
        })
        .take(28)
        .collect();
    if !s
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        s.insert(0, prefix);
    }
    let base = s.clone();
    let mut k = 2;
    while used.contains(&s) {
        s = format!("{base}_{k}");
        k += 1;
    }
    used.insert(s.clone());
    s
}

impl Netlist {
    /// The netlist for a structure built elsewhere (see [`WriteTruss`]). Names are made safe to
    /// parse back; a section name used for two different areas gets a suffix.
    pub fn from_truss(w: &WriteTruss) -> Netlist {
        let t = w.truss;
        let name = w.name.trim();
        let title = if name.is_empty() {
            "untitled.str".to_string()
        } else if name.ends_with(".str") {
            name.to_string()
        } else {
            format!("{name}.str")
        };
        let mut used = HashSet::new();
        let materials = t
            .materials
            .iter()
            .map(|m| {
                let mut m = m.clone();
                m.name = safe_id(&m.name.to_ascii_lowercase(), 'm', &mut used);
                m
            })
            .collect();
        let mut node_names = HashSet::new();
        let nodes = t
            .nodes
            .iter()
            .map(|n| {
                let mut n = n.clone();
                n.id = safe_id(&n.id, 'N', &mut node_names);
                n
            })
            .collect();
        let mut sections: Vec<Section> = Vec::new();
        let mut by_key: HashMap<(String, u64, u64), usize> = HashMap::new();
        let mut section_names = HashSet::new();
        let mut member_names = HashSet::new();
        let mut members = Vec::with_capacity(t.members.len());
        for (i, m) in t.members.iter().enumerate() {
            let wanted = w.sections.get(i).cloned().unwrap_or_default();
            let key = (wanted.clone(), m.area.to_bits(), m.inertia.to_bits());
            let section = match by_key.get(&key) {
                Some(&s) => s,
                None => {
                    let raw = if wanted.is_empty() {
                        format!("s{}", sections.len() + 1)
                    } else {
                        wanted
                    };
                    let s = sections.len();
                    sections.push(Section {
                        name: safe_id(&raw, 's', &mut section_names),
                        area: m.area,
                        inertia: m.inertia,
                    });
                    by_key.insert(key, s);
                    s
                }
            };
            members.push(NetMember {
                id: safe_id(&m.id, 'M', &mut member_names),
                a: m.a,
                b: m.b,
                section,
                material: m.material,
            });
        }
        // Loads: sum per node, then one line per distinct force.
        let mut per_node: Vec<(usize, f64, f64)> = Vec::new();
        for l in w.loads {
            if l.node >= t.nodes.len() || !(l.fx.is_finite() && l.fy.is_finite()) {
                continue;
            }
            match per_node.iter_mut().find(|(n, _, _)| *n == l.node) {
                Some(p) => {
                    p.1 += l.fx;
                    p.2 += l.fy;
                }
                None => per_node.push((l.node, l.fx, l.fy)),
            }
        }
        let mut loads: Vec<StaticLoad> = Vec::new();
        for (n, fx, fy) in per_node {
            if fx == 0.0 && fy == 0.0 {
                continue;
            }
            match loads.iter_mut().find(|l| l.fx == fx && l.fy == fy) {
                Some(l) => l.nodes.push(n),
                None => loads.push(StaticLoad {
                    case: "weights".into(),
                    nodes: vec![n],
                    fx,
                    fy,
                    self_weight: false,
                }),
            }
        }
        if w.self_weight {
            loads.push(StaticLoad {
                case: "self".into(),
                nodes: Vec::new(),
                fx: 0.0,
                fy: 0.0,
                self_weight: true,
            });
        }
        let comment = w.comment.trim();
        Netlist {
            title: Some(title),
            materials,
            sections,
            nodes,
            members,
            loads,
            notes: if comment.is_empty() {
                Vec::new()
            } else {
                vec![comment.to_string()]
            },
            ..Netlist::default()
        }
    }
}

/// A force for a `.load` line: `-20t` when it is a whole number of tonnes (to 3 decimals),
/// else in the force unit.
fn force_text(n: f64, per_unit: f64) -> String {
    let t = n / (G * 1e3);
    let t3 = (t * 1000.0).round() / 1000.0;
    if t != 0.0 && (t - t3).abs() <= 1e-9 * t.abs().max(1.0) {
        return format!("{}t", js_number(t3));
    }
    tidy(n / per_unit)
}

/// Node lists with runs written as ranges (`B1..B5`).
fn node_list(net: &Netlist, nodes: &[usize]) -> String {
    let parts: Vec<(String, Option<u64>)> = nodes
        .iter()
        .map(|&n| {
            let id = net.nodes.get(n).map(|n| n.id.clone()).unwrap_or_default();
            let digits = id.len() - id.trim_end_matches(|c: char| c.is_ascii_digit()).len();
            let num = if digits > 0 && digits <= 9 && !id[id.len() - digits..].starts_with('0') {
                id[id.len() - digits..].parse().ok()
            } else {
                None
            };
            (id, num)
        })
        .collect();
    let prefix = |s: &str| s.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let mut j = i;
        if let Some(n0) = parts[i].1 {
            let p = prefix(&parts[i].0);
            while j + 1 < parts.len()
                && parts[j + 1].1 == Some(n0 + (j + 1 - i) as u64)
                && prefix(&parts[j + 1].0) == p
            {
                j += 1;
            }
        }
        if j - i >= 2 {
            out.push(format!("{}..{}", parts[i].0, parts[j].0));
        } else {
            for p in &parts[i..=j] {
                out.push(p.0.clone());
            }
        }
        i = j + 1;
    }
    out.join("  ")
}

/// Writes a netlist as text (one directive per line, two spaces between fields, as the mockup).
pub fn write(net: &Netlist) -> String {
    let mut s = String::new();
    let per_m = net.units.length.per_metre();
    let per_f = net.units.force.newtons();
    let title = net.title.as_deref().unwrap_or("untitled.str");
    let _ = writeln!(s, "* {title}");
    for n in &net.notes {
        let _ = writeln!(s, "* {}", n.trim());
    }
    let _ = writeln!(
        s,
        ".units SI  {}  {}",
        net.units.force.symbol(),
        net.units.length.symbol()
    );
    for m in &net.materials {
        let _ = writeln!(
            s,
            ".material {}  E={}GPa  fy={}MPa  rho={}",
            m.name,
            tidy(m.e / 1e9),
            tidy(m.fy / 1e6),
            tidy(m.rho)
        );
    }
    for sec in &net.sections {
        let i = if sec.is_box() {
            "A^2".to_string()
        } else {
            format!("{}cm4", tidy(sec.inertia * 1e8))
        };
        let _ = writeln!(
            s,
            ".section {}  A={}cm2  I={i}",
            sec.name,
            tidy(sec.area * 1e4)
        );
    }
    for n in &net.nodes {
        let fix = fix_text(n.fix)
            .map(|f| format!("  fix={f}"))
            .unwrap_or_default();
        let _ = writeln!(
            s,
            ".node {}  {}  {}{fix}",
            n.id,
            tidy(n.x * per_m),
            tidy(n.y * per_m)
        );
    }
    for m in &net.members {
        let id = |n: usize| net.nodes.get(n).map(|n| n.id.as_str()).unwrap_or("?");
        let sec = net
            .sections
            .get(m.section)
            .map(|s| s.name.as_str())
            .unwrap_or("?");
        let mat = if m.material == 0 {
            String::new()
        } else {
            net.materials
                .get(m.material)
                .map(|x| format!("  {}", x.name))
                .unwrap_or_default()
        };
        let _ = writeln!(s, ".member {}  {}  {}  {sec}{mat}", m.id, id(m.a), id(m.b));
    }
    for l in &net.loads {
        let mut line = format!(".load {}", l.case);
        if !l.nodes.is_empty() {
            let _ = write!(line, "  {}", node_list(net, &l.nodes));
            if l.fx != 0.0 {
                let _ = write!(line, "  Fx={}", force_text(l.fx, per_f));
            }
            if l.fy != 0.0 || l.fx == 0.0 {
                let _ = write!(line, "  Fy={}", force_text(l.fy, per_f));
            }
        }
        if l.self_weight {
            line.push_str("  +self");
        }
        let _ = writeln!(s, "{line}");
    }
    if let Some(MovingLoad {
        case,
        weight,
        tonnes,
        x,
        lane,
        share,
    }) = &net.truck
    {
        let w = match tonnes {
            Some(t) => format!("{}t", js_number(*t)),
            None => format!("{}{}", tidy(weight / per_f), net.units.force.symbol()),
        };
        let lane = node_list(net, lane).replace("  ", ",");
        let _ = writeln!(
            s,
            ".load {case}  {w}  x={}  lane={lane}  share={}",
            tidy(x * per_m),
            js_number(*share)
        );
    }
    let _ = writeln!(s, ".static linear");
    let k = if net.k_factor.fract() == 0.0 {
        format!("{:.1}", net.k_factor)
    } else {
        tidy(net.k_factor)
    };
    let _ = writeln!(s, ".buckle euler  K={k}");
    if let Some(Plot { member, range }) = &net.plot {
        let m = member.as_deref().unwrap_or("M*");
        let r = range
            .map(|(a, b, st)| {
                format!(
                    "  {} {} {}",
                    tidy(a * per_m),
                    tidy(b * per_m),
                    tidy(st * per_m)
                )
            })
            .unwrap_or_default();
        let _ = writeln!(s, ".plot N({m})  vs truck.x{r}");
    }
    s
}
