//! The player's structure: joints, members and hung weights, with the mockup's ids (`J<n>`,
//! `M<n>`, never reused within a level), and its solve ([`Design::analyse`], the mockup's
//! `solve()` on top of [`crate::truss`]).

use super::data::{
    JOINT_COST_K, LevelKey, MAX_LOAD_T, MaterialKey, N_PER_TONNE, SizeKey, Support, X_MAX, X_MIN,
    Y_MAX, Y_MIN, underground,
};
use crate::truss::{FailureMode, Member, MemberResult, NodalLoad, Node, SolveOptions, Truss};
use serde::{Deserialize, Serialize};

/// A joint on the grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Joint {
    /// `J1`, `J2`, …
    pub id: String,
    /// Grid x, m (even, 0–58).
    pub x: i32,
    /// Grid y, m (even, −12–16).
    pub y: i32,
    /// Its support, if any.
    pub sup: Option<Support>,
    /// Part of the level (can't be moved or deleted; keeps its support and minimum load).
    pub locked: bool,
}

/// A member between two joints (by id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bar {
    /// `M1`, `M2`, …
    pub id: String,
    /// First joint's id.
    pub a: String,
    /// Second joint's id.
    pub b: String,
    /// Its size.
    pub size: SizeKey,
}

/// The structure being built: what undo snapshots (`nodes`, `members`, `loads`, `nN`, `nM`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Design {
    /// Joints in creation order.
    pub nodes: Vec<Joint>,
    /// Members in creation order.
    pub members: Vec<Bar>,
    /// Hung weights (joint id, tonnes > 0) in the order they were first hung (the mockup's
    /// object key order).
    pub loads: Vec<(String, f64)>,
    /// Joints ever made in this level (the next joint is `J<n_n + 1>`).
    pub n_n: u32,
    /// Members ever made in this level.
    pub n_m: u32,
}

/// One member's solve result, in the Bridge's terms.
pub type MemberState = MemberResult;

/// The solve of a design at a load factor (`solve(st, lam, broken)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    /// The stiffness matrix was regular (`ok`): forces are meaningful.
    pub ok: bool,
    /// The joint the elimination found free to move (`bad`), by index into
    /// [`Design::nodes`].
    pub bad: Option<usize>,
    /// Displacement (dx, dy) of every joint, m (zero when unstable or unused).
    pub displacement: Vec<[f64; 2]>,
    /// Per member in [`Design::members`]; `None` for broken members.
    pub members: Vec<Option<MemberState>>,
    /// The member with the highest utilisation (first on ties).
    pub worst: Option<usize>,
    /// Mass of the unbroken members, kg.
    pub mass: f64,
    /// Cost, $k: `mass · $/kg / 1000 + $0.25k per joint`.
    pub cost: f64,
    /// Unbroken members (`nAct`).
    pub n_act: usize,
    /// Joints that take part (attached to an unbroken member, or supported): the mockup's
    /// `ids`.
    pub used: Vec<bool>,
    /// Reactions (Rx, Ry) at every joint, N.
    pub reactions: Vec<[f64; 2]>,
}

impl Analysis {
    /// The worst member's result.
    pub fn worst_state(&self) -> Option<(usize, &MemberState)> {
        let w = self.worst?;
        Some((w, self.members.get(w)?.as_ref()?))
    }
}

/// What can go wrong building a design from a netlist / truss ("Open in Playground").
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ImportError {
    /// A joint isn't on the 2 m grid inside the stage.
    #[error("joint {0} is not on the 2 m grid of the stage (x 0–58, y −12–16, even metres)")]
    OffGrid(String),
    /// A joint is underground.
    #[error("joint {0} is underground")]
    Underground(String),
    /// Two joints share a spot.
    #[error("joints {0} and {1} are at the same spot")]
    Overlap(String, String),
    /// A member is bad (unknown joint, joins a joint to itself, duplicate).
    #[error("member {0} doesn't join two different joints")]
    BadMember(String),
    /// Too many joints or members for the stage.
    #[error("too big for the Bridge stage ({0})")]
    TooBig(String),
    /// A load isn't a downward weight on a joint.
    #[error("{0}")]
    BadLoad(String),
}

/// The most joints / members an imported design may have (the grid has 30 × 15 points).
pub const MAX_IMPORT_JOINTS: usize = 450;
/// The most members an imported design may have.
pub const MAX_IMPORT_MEMBERS: usize = 2000;

impl Design {
    /// A level's starting design: its joints (`J1…`) and loads (`levelState`).
    pub fn for_level(key: LevelKey) -> Design {
        let l = key.level();
        let nodes: Vec<Joint> = l
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| Joint {
                id: format!("J{}", i + 1),
                x: n.x,
                y: n.y,
                sup: n.sup,
                locked: n.locked,
            })
            .collect();
        let loads = l
            .loads
            .iter()
            .filter_map(|(i, t)| Some((nodes.get(*i)?.id.clone(), *t)))
            .collect();
        Design {
            n_n: nodes.len() as u32,
            nodes,
            members: Vec::new(),
            loads,
            n_m: 0,
        }
    }

    /// The Sandbox's "Drop in a starter truss": a 40 m Pratt-like truss, six bottom joints
    /// (pin at x = 8, roller at x = 48), four top joints at y = 6, 16 Medium members, no loads.
    pub fn starter() -> Design {
        let mut nodes = Vec::new();
        let mut k = 0;
        let mut id = || {
            k += 1;
            format!("J{k}")
        };
        let bot: Vec<Joint> = [8, 16, 24, 32, 40, 48]
            .iter()
            .enumerate()
            .map(|(i, &x)| Joint {
                id: id(),
                x,
                y: 0,
                sup: match i {
                    0 => Some(Support::Pin),
                    5 => Some(Support::Roller),
                    _ => None,
                },
                locked: false,
            })
            .collect();
        let top: Vec<Joint> = [16, 24, 32, 40]
            .iter()
            .map(|&x| Joint {
                id: id(),
                x,
                y: 6,
                sup: None,
                locked: false,
            })
            .collect();
        nodes.extend(bot.iter().cloned());
        nodes.extend(top.iter().cloned());
        let mut members = Vec::new();
        let mut add = |a: &Joint, b: &Joint| {
            let n = members.len() + 1;
            members.push(Bar {
                id: format!("M{n}"),
                a: a.id.clone(),
                b: b.id.clone(),
                size: SizeKey::Medium,
            });
        };
        for i in 0..5 {
            add(&bot[i], &bot[i + 1]);
        }
        for i in 0..3 {
            add(&top[i], &top[i + 1]);
        }
        add(&bot[0], &top[0]);
        add(&top[3], &bot[5]);
        for i in 0..4 {
            add(&bot[i + 1], &top[i]);
        }
        add(&top[0], &bot[2]);
        add(&top[3], &bot[3]);
        Design {
            n_n: nodes.len() as u32,
            n_m: members.len() as u32,
            nodes,
            members,
            loads: Vec::new(),
        }
    }

    /// Index of joint `id`.
    pub fn joint_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Joint `id`.
    pub fn joint(&self, id: &str) -> Option<&Joint> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// The joint at (x, y), if any (`nodeAt`).
    pub fn joint_at(&self, x: i32, y: i32) -> Option<&Joint> {
        self.nodes.iter().find(|n| n.x == x && n.y == y)
    }

    /// Index of member `id`.
    pub fn member_index(&self, id: &str) -> Option<usize> {
        self.members.iter().position(|m| m.id == id)
    }

    /// Member `id`.
    pub fn member(&self, id: &str) -> Option<&Bar> {
        self.members.iter().find(|m| m.id == id)
    }

    /// Weight hung on joint `id`, tonnes (0 if none).
    pub fn load(&self, id: &str) -> f64 {
        self.loads
            .iter()
            .find(|(j, _)| j == id)
            .map(|(_, t)| *t)
            .unwrap_or(0.0)
    }

    /// Sets the weight on joint `id` (tonnes, clamped 0–200; 0 removes it, keeping the others'
    /// order; a new weight goes last).
    pub fn set_load(&mut self, id: &str, t: f64) {
        let t = if t.is_finite() {
            t.clamp(0.0, MAX_LOAD_T)
        } else {
            0.0
        };
        if t == 0.0 {
            self.loads.retain(|(j, _)| j != id);
        } else if let Some(e) = self.loads.iter_mut().find(|(j, _)| j == id) {
            e.1 = t;
        } else {
            self.loads.push((id.to_string(), t));
        }
    }

    /// True if joints `a` and `b` are already joined.
    pub fn joined(&self, a: &str, b: &str) -> bool {
        self.members
            .iter()
            .any(|m| (m.a == a && m.b == b) || (m.a == b && m.b == a))
    }

    /// True if any member touches joint `id`.
    pub fn has_member_at(&self, id: &str) -> bool {
        self.members.iter().any(|m| m.a == id || m.b == id)
    }

    /// Length of member `m`, m.
    pub fn length(&self, m: usize) -> f64 {
        let Some(bar) = self.members.get(m) else {
            return 0.0;
        };
        match (self.joint(&bar.a), self.joint(&bar.b)) {
            (Some(a), Some(b)) => crate::truss::hypot((b.x - a.x) as f64, (b.y - a.y) as f64),
            _ => 0.0,
        }
    }

    /// The design as a solver truss (one material, `I = A²` box sections) plus its hung
    /// weights as point loads at `lam` (N, no self-weight). Members whose joints are missing
    /// are skipped.
    pub fn to_truss(&self, mat: MaterialKey, lam: f64) -> (Truss, Vec<NodalLoad>) {
        let nodes: Vec<Node> = self
            .nodes
            .iter()
            .map(|n| {
                Node::new(
                    n.id.clone(),
                    n.x as f64,
                    n.y as f64,
                    n.sup.map(Support::fix).unwrap_or_default(),
                )
            })
            .collect();
        let members = self
            .members
            .iter()
            .filter_map(|m| {
                let a = self.joint_index(&m.a)?;
                let b = self.joint_index(&m.b)?;
                Some(Member::box_section(m.id.clone(), a, b, m.size.area(), 0))
            })
            .collect();
        let loads = self
            .loads
            .iter()
            .filter_map(|(id, t)| {
                Some(NodalLoad::down(
                    self.joint_index(id)?,
                    t * N_PER_TONNE * lam,
                ))
            })
            .collect();
        (
            Truss {
                nodes,
                members,
                materials: vec![mat.spec().to_material()],
            },
            loads,
        )
    }

    /// Solves the design at load factor `lam` with the members in `broken` gone (the mockup's
    /// `solve(st, lam, broken)`): self-weight lumped half per end at `lam`, then the hung
    /// weights × 9 810 N × `lam`. Never panics: a model the solver rejects (which editing
    /// can't produce) reads as unstable.
    pub fn analyse(&self, mat: MaterialKey, lam: f64, broken: &[String]) -> Analysis {
        let spec = mat.spec();
        let (truss, point) = self.to_truss(mat, lam);
        let removed: Vec<bool> = truss
            .members
            .iter()
            .map(|m| broken.contains(&m.id))
            .collect();
        let mut loads = truss.self_weight(lam, &removed);
        loads.extend(point);
        let nn = self.nodes.len();
        let mass = truss.mass(&removed);
        let cost = mass * spec.cost / 1000.0 + nn as f64 * JOINT_COST_K;
        // Map the truss's member list (which skips dangling members) back onto the design's.
        let mut members = vec![None; self.members.len()];
        let solved = truss.solve(
            &loads,
            &SolveOptions {
                removed: removed.clone(),
                k_factor: 1.0,
            },
        );
        match solved {
            Ok(sol) => {
                let mut worst: Option<(usize, f64)> = None;
                for (ti, r) in sol.members.iter().enumerate() {
                    let Some(r) = r else { continue };
                    let Some(di) = self.member_index(&truss.members[ti].id) else {
                        continue;
                    };
                    members[di] = Some(*r);
                    if worst.is_none_or(|(_, u)| r.utilisation > u) {
                        worst = Some((di, r.utilisation));
                    }
                }
                Analysis {
                    ok: sol.stable,
                    bad: sol.mechanism,
                    displacement: sol.displacements,
                    n_act: sol.members.iter().filter(|m| m.is_some()).count(),
                    members,
                    worst: worst.map(|(i, _)| i),
                    mass,
                    cost,
                    used: sol.active,
                    reactions: sol.reactions,
                }
            }
            Err(_) => {
                let mut used = vec![false; nn];
                for (i, m) in truss.members.iter().enumerate() {
                    if !removed[i] {
                        used[m.a] = true;
                        used[m.b] = true;
                    }
                }
                for (i, n) in self.nodes.iter().enumerate() {
                    if n.sup.is_some() {
                        used[i] = true;
                    }
                }
                Analysis {
                    ok: false,
                    bad: None,
                    displacement: vec![[0.0; 2]; nn],
                    n_act: removed.iter().filter(|r| !**r).count(),
                    members,
                    worst: None,
                    mass,
                    cost,
                    used,
                    reactions: vec![[0.0; 2]; nn],
                }
            }
        }
    }

    /// Builds a design from a truss and its point loads ("Open in Playground", ⌘V): joints
    /// must sit on the 2 m grid of the stage, above ground, one per spot; member areas map to
    /// the nearest size; loads become hung weights (downward only, tonnes = N / 9 810, at most
    /// 200 t per joint). Joint and member names are kept when they look like `J<n>` / `M<n>`
    /// and otherwise renumbered. Nothing is locked.
    pub fn from_truss(truss: &Truss, loads: &[NodalLoad]) -> Result<Design, ImportError> {
        if truss.nodes.len() > MAX_IMPORT_JOINTS {
            return Err(ImportError::TooBig(format!("{} joints", truss.nodes.len())));
        }
        if truss.members.len() > MAX_IMPORT_MEMBERS {
            return Err(ImportError::TooBig(format!(
                "{} members",
                truss.members.len()
            )));
        }
        let numbered = |id: &str, p: char| {
            id.strip_prefix(p)
                .and_then(|n| n.parse::<u32>().ok())
                .filter(|n| *n > 0)
        };
        let keep_j = {
            let mut seen = std::collections::HashSet::new();
            truss
                .nodes
                .iter()
                .all(|n| numbered(&n.id, 'J').is_some_and(|k| seen.insert(k)))
        };
        let keep_m = {
            let mut seen = std::collections::HashSet::new();
            truss
                .members
                .iter()
                .all(|m| numbered(&m.id, 'M').is_some_and(|k| seen.insert(k)))
        };
        let mut nodes = Vec::with_capacity(truss.nodes.len());
        for (i, n) in truss.nodes.iter().enumerate() {
            let on = |v: f64, lo: i32, hi: i32| {
                v.is_finite()
                    && v.fract() == 0.0
                    && (v as i64) % 2 == 0
                    && v >= lo as f64
                    && v <= hi as f64
            };
            if !on(n.x, X_MIN, X_MAX) || !on(n.y, Y_MIN, Y_MAX) {
                return Err(ImportError::OffGrid(n.id.clone()));
            }
            let (x, y) = (n.x as i32, n.y as i32);
            if underground(x, y) {
                return Err(ImportError::Underground(n.id.clone()));
            }
            let id = if keep_j {
                n.id.clone()
            } else {
                format!("J{}", i + 1)
            };
            if let Some(o) = nodes.iter().find(|o: &&Joint| o.x == x && o.y == y) {
                return Err(ImportError::Overlap(o.id.clone(), id));
            }
            nodes.push(Joint {
                id,
                x,
                y,
                sup: Support::from_fix(n.fix),
                locked: false,
            });
        }
        let mut d = Design {
            n_n: nodes
                .iter()
                .filter_map(|n| numbered(&n.id, 'J'))
                .max()
                .unwrap_or(0),
            nodes,
            ..Design::default()
        };
        for (i, m) in truss.members.iter().enumerate() {
            let id = if keep_m {
                m.id.clone()
            } else {
                format!("M{}", i + 1)
            };
            let (Some(a), Some(b)) = (d.nodes.get(m.a), d.nodes.get(m.b)) else {
                return Err(ImportError::BadMember(m.id.clone()));
            };
            if m.a == m.b || d.joined(&a.id, &b.id) {
                return Err(ImportError::BadMember(m.id.clone()));
            }
            let (a, b) = (a.id.clone(), b.id.clone());
            d.members.push(Bar {
                id,
                a,
                b,
                size: SizeKey::nearest(m.area),
            });
        }
        d.n_m = d
            .members
            .iter()
            .filter_map(|m| numbered(&m.id, 'M'))
            .max()
            .unwrap_or(0);
        for l in loads {
            let Some(n) = d.nodes.get(l.node) else {
                return Err(ImportError::BadLoad(
                    "a load is on a joint that doesn't exist".into(),
                ));
            };
            if !(l.fx.is_finite() && l.fy.is_finite()) || l.fx.abs() > 1e-6 * l.fy.abs().max(1.0) {
                return Err(ImportError::BadLoad(format!(
                    "the Bridge only hangs weights: the load on {} pushes sideways",
                    n.id
                )));
            }
            if l.fy > 0.0 {
                return Err(ImportError::BadLoad(format!(
                    "the Bridge only hangs weights: the load on {} pushes up",
                    n.id
                )));
            }
            let id = n.id.clone();
            let t = d.load(&id) + (-l.fy / N_PER_TONNE);
            // keep three significant decimals so 196.2 kN reads back as 20 t
            let t = (t * 1000.0).round() / 1000.0;
            if t > MAX_LOAD_T {
                return Err(ImportError::BadLoad(format!(
                    "{id} carries more than {} t",
                    MAX_LOAD_T
                )));
            }
            d.set_load(&id, t);
        }
        Ok(d)
    }
}

/// How a member is drawn (`B[c + size]` in `renderVals`): tension / compression at three
/// brightnesses, near zero, over its limit, or no solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Band {
    /// Tension, utilisation < 35 % (opacity .45).
    T1,
    /// Tension, < 70 % (.75).
    T2,
    /// Tension, ≥ 70 % (1).
    T3,
    /// Compression, < 35 %.
    C1,
    /// Compression, < 70 %.
    C2,
    /// Compression, ≥ 70 %.
    C3,
    /// |N| < 500 N: carrying almost nothing (grey).
    Zero,
    /// Over its limit (dashed red).
    Over,
    /// No solution (unstable, or in the collapse simulation).
    None,
}

impl Band {
    /// The band of a member result (`c` in `renderVals`).
    pub fn of(r: &MemberState) -> Band {
        if r.utilisation > 1.0 {
            Band::Over
        } else if r.force.abs() < crate::truss::ZERO_FORCE_N {
            Band::Zero
        } else {
            let k = if r.utilisation < 0.35 {
                0
            } else if r.utilisation < 0.7 {
                1
            } else {
                2
            };
            if r.force > 0.0 {
                [Band::T1, Band::T2, Band::T3][k]
            } else {
                [Band::C1, Band::C2, Band::C3][k]
            }
        }
    }

    /// The mockup's class key (`t1` … `c3`, `z`, `x`, `n`).
    pub fn key(self) -> &'static str {
        match self {
            Band::T1 => "t1",
            Band::T2 => "t2",
            Band::T3 => "t3",
            Band::C1 => "c1",
            Band::C2 => "c2",
            Band::C3 => "c3",
            Band::Zero => "z",
            Band::Over => "x",
            Band::None => "n",
        }
    }

    /// Utilisation band 0, 1, 2 for the force colours (opacity .45 / .75 / 1); `None` for
    /// the others.
    pub fn level(self) -> Option<usize> {
        match self {
            Band::T1 | Band::C1 => Some(0),
            Band::T2 | Band::C2 => Some(1),
            Band::T3 | Band::C3 => Some(2),
            _ => None,
        }
    }
}

/// The verb for a failure mode ("snapped", "buckled", "crushed").
pub fn mode_word(m: FailureMode) -> &'static str {
    m.past_tense()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_designs_have_the_level_joints() {
        let d = Design::for_level(LevelKey::Haul);
        assert_eq!(d.nodes.len(), 4);
        assert_eq!(d.n_n, 4);
        assert_eq!(d.loads, vec![("J3".into(), 25.0), ("J4".into(), 25.0)]);
        assert!(d.nodes.iter().all(|n| n.locked));
        let s = Design::for_level(LevelKey::Sandbox);
        assert!(s.nodes.iter().all(|n| !n.locked) && s.loads.is_empty());
    }

    #[test]
    fn starter_truss_shape() {
        let d = Design::starter();
        assert_eq!(d.nodes.len(), 10);
        assert_eq!(d.members.len(), 16);
        assert_eq!(d.n_m, 16);
        // m + r − 2j = 16 + 3 − 20 = −1: the middle panel has no diagonal, so the starter is
        // a mechanism until the player closes it (the mockup's teaching moment).
        let a = d.analyse(MaterialKey::Steel, 1.0, &[]);
        assert!(!a.ok);
        assert!(a.bad.is_some());
        assert_eq!(a.n_act, 16);
    }

    #[test]
    fn loads_keep_their_order() {
        let mut d = Design::default();
        d.set_load("J3", 5.0);
        d.set_load("J1", 10.0);
        d.set_load("J3", 15.0);
        assert_eq!(d.loads, vec![("J3".into(), 15.0), ("J1".into(), 10.0)]);
        d.set_load("J3", 0.0);
        d.set_load("J3", 1.0);
        assert_eq!(d.loads, vec![("J1".into(), 10.0), ("J3".into(), 1.0)]);
        d.set_load("J1", f64::NAN);
        assert_eq!(d.loads, vec![("J3".into(), 1.0)]);
        d.set_load("J3", 1e9);
        assert_eq!(d.load("J3"), 200.0);
    }
}
