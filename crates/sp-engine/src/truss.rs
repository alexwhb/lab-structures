//! The 2D pin-jointed truss: model, direct stiffness solve and member checks.
//!
//! This is nodal analysis with springs instead of resistors (`Series.dc.html`: "Breadbox's DC
//! solver and the truss solver can be one module"): every member is an axial spring
//! `k = E·A / L`, every free joint has two unknown displacements, and `K·u = F` is solved once.
//! Forces follow from the stretch of each member. It is **exact for the model** (linear,
//! small-displacement statics of pin-jointed members), which is what the provenance badge says.
//!
//! Member checks (`Playground.dc.html` / `Advanced.dc.html` `solve()`):
//!
//! - tension: capacity `A·fy` (mode [`FailureMode::Yield`], "snapped" in the Playground);
//! - compression: limit `min(A·fy, P_cr)` with Euler's `P_cr = π²·E·I / (K·L)²`; the mode is
//!   [`FailureMode::Buckling`] when `P_cr < A·fy`, else [`FailureMode::Crushing`];
//! - utilisation `|N| / limit` (1.0 = at the limit).
//!
//! Sections in the games use `I = A²`, which is exact for a thin-walled square box with
//! `b/t = 24` (the Advanced netlist's "box, b/t = 24" comment): `A ≈ 4bt = b²/6`,
//! `I ≈ (2/3)·b³·t = b⁴/36 = A²`. See [`Member::box_section`].

use crate::linalg::{Singular, solve_dense};
use serde::{Deserialize, Serialize};

/// Standard gravity used by every screen, m/s² (the mockups' `9.81`).
pub const G: f64 = 9.81;

/// Members whose axial force is below this (newtons) count as "carrying almost nothing"
/// (`Math.abs(q.N) < 500` in the Playground).
pub const ZERO_FORCE_N: f64 = 500.0;

/// A structural material (representative values; see `docs/DATA-SOURCES.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// Display name ("Steel").
    pub name: String,
    /// Young's modulus E, Pa.
    pub e: f64,
    /// Yield (or failure) strength fy, Pa.
    pub fy: f64,
    /// Density ρ, kg/m³.
    pub rho: f64,
}

impl Material {
    /// A material from E (Pa), fy (Pa) and ρ (kg/m³).
    pub fn new(name: impl Into<String>, e: f64, fy: f64, rho: f64) -> Self {
        Self {
            name: name.into(),
            e,
            fy,
            rho,
        }
    }
}

/// Which displacements a joint's support holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Fix {
    /// Horizontal displacement held.
    pub x: bool,
    /// Vertical displacement held.
    pub y: bool,
}

impl Fix {
    /// No support.
    pub const FREE: Fix = Fix { x: false, y: false };
    /// Pin: held in place, free to rotate (netlist `fix=xy`).
    pub const PIN: Fix = Fix { x: true, y: true };
    /// Roller: held up, free to slide sideways (netlist `fix=y`).
    pub const ROLLER: Fix = Fix { x: false, y: true };

    /// True if the joint is supported in any direction.
    pub fn is_support(self) -> bool {
        self.x || self.y
    }

    /// Number of reaction components (0–2).
    pub fn reactions(self) -> usize {
        self.x as usize + self.y as usize
    }
}

/// A joint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Name ("J1", "B0").
    pub id: String,
    /// Position, m (x to the right, y up).
    pub x: f64,
    /// Position, m.
    pub y: f64,
    /// Support conditions.
    pub fix: Fix,
}

impl Node {
    /// A joint at (x, y) with the given support.
    pub fn new(id: impl Into<String>, x: f64, y: f64, fix: Fix) -> Self {
        Self {
            id: id.into(),
            x,
            y,
            fix,
        }
    }
}

/// A pin-ended axial member between two joints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// Name ("M1").
    pub id: String,
    /// Index of the first joint in [`Truss::nodes`].
    pub a: usize,
    /// Index of the second joint.
    pub b: usize,
    /// Cross-section area A, m².
    pub area: f64,
    /// Second moment of area I, m⁴ (buckling).
    pub inertia: f64,
    /// Index into [`Truss::materials`].
    pub material: usize,
}

impl Member {
    /// A member with the games' box section: `I = A²` (thin square box, b/t = 24).
    pub fn box_section(
        id: impl Into<String>,
        a: usize,
        b: usize,
        area: f64,
        material: usize,
    ) -> Self {
        Self {
            id: id.into(),
            a,
            b,
            area,
            inertia: area * area,
            material,
        }
    }
}

/// A point load on a joint, N (`fy < 0` pushes down).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodalLoad {
    /// Index into [`Truss::nodes`].
    pub node: usize,
    /// Horizontal force, N.
    pub fx: f64,
    /// Vertical force, N.
    pub fy: f64,
}

impl NodalLoad {
    /// A downward weight of `newtons` on `node`.
    pub fn down(node: usize, newtons: f64) -> Self {
        Self {
            node,
            fx: 0.0,
            fy: -newtons,
        }
    }
}

/// A truss: joints, members and the materials they use.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Truss {
    /// Joints.
    pub nodes: Vec<Node>,
    /// Members.
    pub members: Vec<Member>,
    /// Materials referenced by [`Member::material`].
    pub materials: Vec<Material>,
}

/// How a member gives out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureMode {
    /// Tension past `A·fy` (the Playground says "snapped", Advanced "yield").
    Yield,
    /// Compression past Euler's critical load, which is below `A·fy`.
    Buckling,
    /// Compression past `A·fy` (Euler's load is higher).
    Crushing,
}

impl FailureMode {
    /// The Playground's verb: "snapped", "buckled", "crushed".
    pub fn past_tense(self) -> &'static str {
        match self {
            FailureMode::Yield => "snapped",
            FailureMode::Buckling => "buckled",
            FailureMode::Crushing => "crushed",
        }
    }
}

/// One member's result.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemberResult {
    /// Axial force N, newtons, positive in tension (0 when the structure is unstable).
    pub force: f64,
    /// Length L, m.
    pub length: f64,
    /// Axial stress σ = N / A, Pa.
    pub stress: f64,
    /// Tension capacity `A·fy`, N.
    pub tension_capacity: f64,
    /// Euler critical load `π²·E·I / (K·L)²`, N.
    pub euler: f64,
    /// Compression limit `min(A·fy, P_cr)`, N.
    pub compression_limit: f64,
    /// The limit that applies to the current sign of the force, N.
    pub limit: f64,
    /// `|N| / limit`.
    pub utilisation: f64,
    /// The governing mode for the current sign of the force.
    pub mode: FailureMode,
}

/// Solve options.
#[derive(Debug, Clone, Default)]
pub struct SolveOptions {
    /// Members treated as gone (broken, removed). Indices into [`Truss::members`].
    pub removed: Vec<bool>,
    /// Effective-length factor K for Euler buckling (`.buckle euler K=1.0`); 0 or non-finite
    /// means 1.0.
    pub k_factor: f64,
}

impl SolveOptions {
    fn is_removed(&self, m: usize) -> bool {
        self.removed.get(m).copied().unwrap_or(false)
    }
    fn k(&self) -> f64 {
        if self.k_factor.is_finite() && self.k_factor > 0.0 {
            self.k_factor
        } else {
            1.0
        }
    }
}

/// The input can't be solved at all (as opposed to a mechanism, which is a valid result).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SolveError {
    /// A member names a joint that doesn't exist.
    #[error("member {member} joins a joint that doesn't exist")]
    BadNode {
        /// Member id.
        member: String,
    },
    /// A member names a material that doesn't exist.
    #[error("member {member} uses a material that doesn't exist")]
    BadMaterial {
        /// Member id.
        member: String,
    },
    /// A member has zero length (both ends at the same place) or a non-finite geometry.
    #[error("member {member} has zero length")]
    ZeroLength {
        /// Member id.
        member: String,
    },
    /// A number in the model is not finite or a property is not positive.
    #[error("{what} must be a positive, finite number")]
    BadValue {
        /// What was wrong ("area of M3").
        what: String,
    },
    /// A load names a joint that doesn't exist.
    #[error("a load is on a joint that doesn't exist")]
    BadLoad,
}

/// The result of one static solve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Solution {
    /// False when the stiffness matrix is singular: some joint can move without straining any
    /// member (a mechanism). Forces are then all zero.
    pub stable: bool,
    /// The joint the elimination found free to move (the mockups' `bad` joint), if unstable.
    pub mechanism: Option<usize>,
    /// Displacement (dx, dy) of every joint, m (zero for unused joints or when unstable).
    pub displacements: Vec<[f64; 2]>,
    /// Reaction (Rx, Ry) at every joint, N (non-zero only on held directions of a stable
    /// solve). Equilibrium: reactions + loads sum to zero.
    pub reactions: Vec<[f64; 2]>,
    /// Per member; `None` for removed members.
    pub members: Vec<Option<MemberResult>>,
    /// Joints that take part (attached to an active member, or supported).
    pub active: Vec<bool>,
    /// Joints that carry a load but take no part: the weight hangs on nothing and falls.
    pub loose_loads: Vec<usize>,
    /// The member with the highest utilisation (first one on ties), if any is active.
    pub worst: Option<usize>,
    /// Mass of the active members, kg.
    pub mass: f64,
}

impl Solution {
    /// The worst member's result.
    pub fn worst_result(&self) -> Option<&MemberResult> {
        self.worst.and_then(|m| self.members.get(m)?.as_ref())
    }

    /// Number of active (not removed) members.
    pub fn active_members(&self) -> usize {
        self.members.iter().filter(|m| m.is_some()).count()
    }
}

impl Truss {
    /// Index of the joint called `id`.
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Index of the member called `id`.
    pub fn member_index(&self, id: &str) -> Option<usize> {
        self.members.iter().position(|m| m.id == id)
    }

    /// Length of member `m`, m (0 for a bad index).
    pub fn length(&self, m: usize) -> f64 {
        let Some(mem) = self.members.get(m) else {
            return 0.0;
        };
        match (self.nodes.get(mem.a), self.nodes.get(mem.b)) {
            (Some(a), Some(b)) => (b.x - a.x).hypot(b.y - a.y),
            _ => 0.0,
        }
    }

    /// Checks indices, lengths and properties.
    pub fn validate(&self) -> Result<(), SolveError> {
        for n in &self.nodes {
            if !n.x.is_finite() || !n.y.is_finite() {
                return Err(SolveError::BadValue {
                    what: format!("position of {}", n.id),
                });
            }
        }
        for (i, m) in self.members.iter().enumerate() {
            if m.a >= self.nodes.len() || m.b >= self.nodes.len() {
                return Err(SolveError::BadNode {
                    member: m.id.clone(),
                });
            }
            let Some(mat) = self.materials.get(m.material) else {
                return Err(SolveError::BadMaterial {
                    member: m.id.clone(),
                });
            };
            let l = self.length(i);
            if m.a == m.b || !(l.is_finite() && l > 1e-9) {
                return Err(SolveError::ZeroLength {
                    member: m.id.clone(),
                });
            }
            for (what, v) in [
                ("area", m.area),
                ("inertia", m.inertia),
                ("E", mat.e),
                ("fy", mat.fy),
            ] {
                if !(v.is_finite() && v > 0.0) {
                    return Err(SolveError::BadValue {
                        what: format!("{what} of {}", m.id),
                    });
                }
            }
            if !(mat.rho.is_finite() && mat.rho >= 0.0) {
                return Err(SolveError::BadValue {
                    what: format!("density of {}", m.id),
                });
            }
        }
        Ok(())
    }

    /// Self-weight as nodal loads: each active member's weight `A·L·ρ·g·factor`, half at each
    /// end (the Playground's lumping).
    pub fn self_weight(&self, factor: f64, removed: &[bool]) -> Vec<NodalLoad> {
        let mut out = Vec::new();
        for (i, m) in self.members.iter().enumerate() {
            if removed.get(i).copied().unwrap_or(false) {
                continue;
            }
            let Some(mat) = self.materials.get(m.material) else {
                continue;
            };
            let w = m.area * self.length(i) * mat.rho * G * factor;
            if w.is_finite() && w != 0.0 {
                out.push(NodalLoad::down(m.a, w / 2.0));
                out.push(NodalLoad::down(m.b, w / 2.0));
            }
        }
        out
    }

    /// Mass of the members not removed, kg.
    pub fn mass(&self, removed: &[bool]) -> f64 {
        self.members
            .iter()
            .enumerate()
            .filter(|(i, _)| !removed.get(*i).copied().unwrap_or(false))
            .filter_map(|(i, m)| {
                let mat = self.materials.get(m.material)?;
                Some(m.area * self.length(i) * mat.rho)
            })
            .filter(|v| v.is_finite())
            .sum()
    }

    /// Static determinacy `m + r − 2j` over all joints (the Advanced "Determinacy" check):
    /// 0 = determinate (lose any member and it becomes a mechanism), > 0 = spare load paths,
    /// < 0 = a mechanism for sure.
    pub fn determinacy(&self) -> i64 {
        let r: usize = self.nodes.iter().map(|n| n.fix.reactions()).sum();
        self.members.len() as i64 + r as i64 - 2 * self.nodes.len() as i64
    }

    /// The direct stiffness solve.
    ///
    /// Only joints attached to an active member, or supported, get unknowns (an unused joint
    /// is not a mechanism). Loads on joints that take no part are reported in
    /// [`Solution::loose_loads`] and otherwise ignored. A singular stiffness matrix gives a
    /// valid [`Solution`] with `stable = false` and the free joint in `mechanism`.
    pub fn solve(&self, loads: &[NodalLoad], opts: &SolveOptions) -> Result<Solution, SolveError> {
        self.validate()?;
        if loads.iter().any(|l| l.node >= self.nodes.len()) {
            return Err(SolveError::BadLoad);
        }
        if loads.iter().any(|l| !l.fx.is_finite() || !l.fy.is_finite()) {
            return Err(SolveError::BadValue {
                what: "a load".into(),
            });
        }
        let nn = self.nodes.len();
        let mut active = vec![false; nn];
        for (i, m) in self.members.iter().enumerate() {
            if !opts.is_removed(i) {
                active[m.a] = true;
                active[m.b] = true;
            }
        }
        for (i, n) in self.nodes.iter().enumerate() {
            if n.fix.is_support() {
                active[i] = true;
            }
        }
        // node index -> position among active nodes
        let mut slot = vec![usize::MAX; nn];
        let mut ids = Vec::new();
        for i in 0..nn {
            if active[i] {
                slot[i] = ids.len();
                ids.push(i);
            }
        }
        let nd = 2 * ids.len();
        let mut k = vec![0.0; nd * nd];
        let mut f = vec![0.0; nd];
        let geo: Vec<(f64, f64, f64)> = (0..self.members.len())
            .map(|i| {
                let m = &self.members[i];
                let (a, b) = (&self.nodes[m.a], &self.nodes[m.b]);
                let l = (b.x - a.x).hypot(b.y - a.y);
                (l, (b.x - a.x) / l, (b.y - a.y) / l)
            })
            .collect();
        for (i, m) in self.members.iter().enumerate() {
            if opts.is_removed(i) {
                continue;
            }
            let mat = &self.materials[m.material];
            let (l, c, s) = geo[i];
            let ks = mat.e * m.area / l;
            let (ia, ib) = (slot[m.a], slot[m.b]);
            let d = [2 * ia, 2 * ia + 1, 2 * ib, 2 * ib + 1];
            let (cc, cs, ss) = (c * c, c * s, s * s);
            let kl = [
                [cc, cs, -cc, -cs],
                [cs, ss, -cs, -ss],
                [-cc, -cs, cc, cs],
                [-cs, -ss, cs, ss],
            ];
            for r in 0..4 {
                for q in 0..4 {
                    k[d[r] * nd + d[q]] += ks * kl[r][q];
                }
            }
        }
        let mut loose = Vec::new();
        for l in loads {
            if active[l.node] {
                let s = slot[l.node];
                f[2 * s] += l.fx;
                f[2 * s + 1] += l.fy;
            } else if (l.fx != 0.0 || l.fy != 0.0) && !loose.contains(&l.node) {
                loose.push(l.node);
            }
        }
        let mut fixed = vec![false; nd];
        for (s, &i) in ids.iter().enumerate() {
            let fix = self.nodes[i].fix;
            fixed[2 * s] = fix.x;
            fixed[2 * s + 1] = fix.y;
        }
        let free: Vec<usize> = (0..nd).filter(|d| !fixed[*d]).collect();
        let n = free.len();
        let mut a = vec![0.0; n * n];
        let mut b = vec![0.0; n];
        for (r, &dr) in free.iter().enumerate() {
            b[r] = f[dr];
            for (q, &dq) in free.iter().enumerate() {
                a[r * n + q] = k[dr * nd + dq];
            }
        }
        let mut u = vec![0.0; nd];
        let (stable, mechanism) = match solve_dense(n, &mut a, &mut b) {
            Ok(x) => {
                for (i, &d) in free.iter().enumerate() {
                    u[d] = x[i];
                }
                (true, None)
            }
            Err(Singular { column }) => {
                let node = free.get(column).map(|d| ids[d / 2]);
                (false, node)
            }
        };
        let mut displacements = vec![[0.0; 2]; nn];
        let mut reactions = vec![[0.0; 2]; nn];
        if stable {
            for (s, &i) in ids.iter().enumerate() {
                displacements[i] = [u[2 * s], u[2 * s + 1]];
            }
            for d in 0..nd {
                if fixed[d] {
                    let mut r = -f[d];
                    for q in 0..nd {
                        r += k[d * nd + q] * u[q];
                    }
                    reactions[ids[d / 2]][d % 2] = r;
                }
            }
        }
        let kf = opts.k();
        let mut members = Vec::with_capacity(self.members.len());
        let mut worst: Option<(usize, f64)> = None;
        for (i, m) in self.members.iter().enumerate() {
            if opts.is_removed(i) {
                members.push(None);
                continue;
            }
            let mat = &self.materials[m.material];
            let (l, c, s) = geo[i];
            let force = if stable {
                let (ua, ub) = (displacements[m.a], displacements[m.b]);
                mat.e * m.area / l * ((ub[0] - ua[0]) * c + (ub[1] - ua[1]) * s)
            } else {
                0.0
            };
            let r = check(force, l, m.area, m.inertia, mat, kf);
            if worst.is_none_or(|(_, u)| r.utilisation > u) {
                worst = Some((i, r.utilisation));
            }
            members.push(Some(r));
        }
        Ok(Solution {
            stable,
            mechanism,
            displacements,
            reactions,
            members,
            active,
            loose_loads: loose,
            worst: worst.map(|(i, _)| i),
            mass: self.mass(&opts.removed),
        })
    }
}

/// The member checks for an axial force `force` (N, + tension) in a member of length `l`,
/// area `area`, inertia `inertia`, with effective-length factor `k_factor`.
pub fn check(
    force: f64,
    l: f64,
    area: f64,
    inertia: f64,
    mat: &Material,
    k_factor: f64,
) -> MemberResult {
    let cap = area * mat.fy;
    let kl = k_factor * l;
    let euler = std::f64::consts::PI.powi(2) * mat.e * inertia / (kl * kl);
    let climit = cap.min(euler);
    let (limit, mode) = if force >= 0.0 {
        (cap, FailureMode::Yield)
    } else if euler < cap {
        (climit, FailureMode::Buckling)
    } else {
        (climit, FailureMode::Crushing)
    };
    MemberResult {
        force,
        length: l,
        stress: force / area,
        tension_capacity: cap,
        euler,
        compression_limit: climit,
        limit,
        utilisation: force.abs() / limit,
        mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steel() -> Material {
        Material::new("Steel", 200e9, 250e6, 7850.0)
    }

    /// A symmetric triangle: two rafters meeting at an apex with a load P; closed form
    /// N = −P / (2 sin θ) in each rafter, and the tie carries N·cos θ.
    #[test]
    fn triangle_matches_closed_form() {
        let t = Truss {
            nodes: vec![
                Node::new("A", 0.0, 0.0, Fix::PIN),
                Node::new("B", 8.0, 0.0, Fix::ROLLER),
                Node::new("C", 4.0, 3.0, Fix::FREE),
            ],
            members: vec![
                Member::box_section("M1", 0, 2, 0.008, 0),
                Member::box_section("M2", 2, 1, 0.008, 0),
                Member::box_section("M3", 0, 1, 0.008, 0),
            ],
            materials: vec![steel()],
        };
        let p = 100e3;
        let sol = t
            .solve(&[NodalLoad::down(2, p)], &SolveOptions::default())
            .unwrap();
        assert!(sol.stable);
        let sin = 3.0 / 5.0;
        let rafter = -p / (2.0 * sin);
        let n1 = sol.members[0].unwrap().force;
        let n3 = sol.members[2].unwrap().force;
        assert!((n1 - rafter).abs() < 1e-6 * p, "{n1} vs {rafter}");
        assert!((n3 - (-rafter * 0.8)).abs() < 1e-6 * p, "{n3}");
        // reactions balance the load
        let ry: f64 = sol.reactions.iter().map(|r| r[1]).sum();
        assert!((ry - p).abs() < 1e-6 * p);
        assert!((sol.reactions[0][1] - p / 2.0).abs() < 1e-6 * p);
        assert_eq!(t.determinacy(), 0);
    }

    /// An open four-sided panel folds: the solve says which joint is free.
    #[test]
    fn square_without_diagonal_is_a_mechanism() {
        let t = Truss {
            nodes: vec![
                Node::new("A", 0.0, 0.0, Fix::PIN),
                Node::new("B", 4.0, 0.0, Fix::ROLLER),
                Node::new("C", 4.0, 4.0, Fix::FREE),
                Node::new("D", 0.0, 4.0, Fix::FREE),
            ],
            members: vec![
                Member::box_section("M1", 0, 1, 0.008, 0),
                Member::box_section("M2", 1, 2, 0.008, 0),
                Member::box_section("M3", 2, 3, 0.008, 0),
                Member::box_section("M4", 3, 0, 0.008, 0),
            ],
            materials: vec![steel()],
        };
        let sol = t
            .solve(&[NodalLoad::down(2, 1e4)], &SolveOptions::default())
            .unwrap();
        assert!(!sol.stable);
        assert!(matches!(sol.mechanism, Some(2) | Some(3)));
        assert!(sol.members.iter().all(|m| m.unwrap().force == 0.0));
        // close it with a diagonal and it stands
        let mut t2 = t.clone();
        t2.members.push(Member::box_section("M5", 0, 2, 0.008, 0));
        let sol2 = t2
            .solve(&[NodalLoad::down(2, 1e4)], &SolveOptions::default())
            .unwrap();
        assert!(sol2.stable);
        // removing the diagonal again through the options folds it
        let opts = SolveOptions {
            removed: vec![false, false, false, false, true],
            ..Default::default()
        };
        assert!(!t2.solve(&[NodalLoad::down(2, 1e4)], &opts).unwrap().stable);
    }

    #[test]
    fn loose_loads_and_unused_joints() {
        let t = Truss {
            nodes: vec![
                Node::new("A", 0.0, 0.0, Fix::PIN),
                Node::new("B", 8.0, 0.0, Fix::ROLLER),
                Node::new("C", 4.0, 0.0, Fix::FREE),
            ],
            members: vec![Member::box_section("M1", 0, 1, 0.008, 0)],
            materials: vec![steel()],
        };
        let sol = t
            .solve(&[NodalLoad::down(2, 1e4)], &SolveOptions::default())
            .unwrap();
        assert!(sol.stable, "an unused joint is not a mechanism");
        assert_eq!(sol.loose_loads, vec![2]);
        assert!(!sol.active[2]);
    }

    #[test]
    fn checks_pick_the_right_mode() {
        let m = steel();
        // short and stocky: crushing
        let r = check(-1e6, 1.0, 0.016, 0.016f64.powi(2), &m, 1.0);
        assert_eq!(r.mode, FailureMode::Crushing);
        assert!((r.limit - 0.016 * 250e6).abs() < 1e-6);
        // long and thin: buckling
        let r = check(-1e5, 20.0, 0.004, 0.004f64.powi(2), &m, 1.0);
        assert_eq!(r.mode, FailureMode::Buckling);
        let pcr = std::f64::consts::PI.powi(2) * 200e9 * 0.004f64.powi(2) / 400.0;
        assert!((r.limit - pcr).abs() < 1e-6 * pcr);
        // tension: yield
        let r = check(5e5, 20.0, 0.004, 0.004f64.powi(2), &m, 1.0);
        assert_eq!(r.mode, FailureMode::Yield);
        assert!((r.utilisation - 0.5).abs() < 1e-12);
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        let mut t = Truss {
            nodes: vec![Node::new("A", 0.0, 0.0, Fix::PIN)],
            members: vec![Member::box_section("M1", 0, 5, 0.008, 0)],
            materials: vec![steel()],
        };
        assert!(matches!(
            t.solve(&[], &SolveOptions::default()),
            Err(SolveError::BadNode { .. })
        ));
        t.nodes.push(Node::new("B", 0.0, 0.0, Fix::FREE));
        t.members[0].b = 1;
        assert!(matches!(
            t.solve(&[], &SolveOptions::default()),
            Err(SolveError::ZeroLength { .. })
        ));
        t.nodes[1].x = f64::NAN;
        assert!(t.solve(&[], &SolveOptions::default()).is_err());
        let empty = Truss::default();
        let s = empty.solve(&[], &SolveOptions::default()).unwrap();
        assert!(s.stable && s.worst.is_none());
    }
}
