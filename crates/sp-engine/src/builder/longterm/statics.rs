//! The settled state of a builder structure, solved exactly: a geometrically nonlinear static
//! equilibrium with the builder's own springs (pieces `E/15·A/L`, foundations `ks` vertically
//! and pinned horizontally — the settling engine's `support` constraint), gravity on the
//! joints' masses, tension-only pieces going slack, joints resting on the ground as one-sided
//! supports, foundations ripping out past their pull capacity and pieces breaking past theirs.
//!
//! Why not the builder's own 520-step preview: on soft soil a tower's rocking mode on its soil
//! springs is over-damped by the preview's damping (time constant ≈ 2 s against the 1.3 s it
//! runs), so the reactions barely shift towards the low side and the P-Δ feedback would depend
//! on the step count. The exact solve converges to what the preview converges to, given time.
//!
//! It minimises the potential energy (piece strain + soil springs + gravity) by Newton steps
//! with a positive-definite tangent and a line search, so it only ever settles into a stable
//! equilibrium. Each call starts from the previous positions, so the geometry follows the
//! tilt: loads hang where the joints are now, which is the P-Δ effect — and a tower past its
//! tipping point keeps sliding over instead of balancing on the unstable side.

use crate::truss::G;

/// How a foundation joint consolidates (TIME-1), from `sp_engine::ground`'s soils.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consolidation {
    /// Consolidation time constant τ, years.
    pub tau: f64,
    /// Long-term settlement per unit of immediate settlement: the long-term stiffness is the
    /// foundation's stiffness divided by this.
    pub ratio: f64,
}

/// A foundation's soil spring and capacities (the builder's `soils()` grip).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// Push capacity, N.
    pub qc: f64,
    /// Pull capacity, N.
    pub qt: f64,
    /// Stiffness, N/m.
    pub ks: f64,
    /// Consolidation over the years (`None`: rock, settles nothing).
    pub consolidation: Option<Consolidation>,
}

impl Spring {
    /// The long-term stiffness, N/m (`ks / ratio`; infinite when it doesn't consolidate).
    pub fn long_term_stiffness(&self) -> f64 {
        match self.consolidation {
            Some(c) if c.ratio > 0.0 && c.tau > 0.0 => self.ks / c.ratio,
            _ => f64::INFINITY,
        }
    }
}

/// A joint of the structure to solve.
#[derive(Debug, Clone, PartialEq)]
pub struct SJoint {
    /// "J3".
    pub id: String,
    /// Design x, m.
    pub x: f64,
    /// Design y, m.
    pub y: f64,
    /// Mass, kg (the settling engine's joint mass: 40 kg, crates, 800 kg per foundation, half
    /// of each piece).
    pub mass: f64,
    /// Its foundation.
    pub found: Option<Spring>,
    /// Rests on the ground without a foundation (a one-sided pin: it can lift off).
    pub on_ground: bool,
}

/// A piece of the structure to solve.
#[derive(Debug, Clone, PartialEq)]
pub struct SPiece {
    /// "M7".
    pub id: String,
    /// One end (joint index).
    pub a: usize,
    /// The other end.
    pub b: usize,
    /// Rest length (the design length), m.
    pub rest: f64,
    /// Axial stiffness `E/15·A/L`, N/m.
    pub kk: f64,
    /// Breaks pulled past this, N.
    pub cap_t: f64,
    /// Breaks pushed past this, N (infinite for tension-only pieces).
    pub cap_c: f64,
    /// Rope or cable: goes slack when pushed.
    pub t_only: bool,
    /// Buckles (rather than crushes) when pushed past capacity.
    pub buckles: bool,
    /// Concrete: "cracked" (rather than "snapped") when pulled past capacity.
    pub cracks: bool,
}

impl SPiece {
    /// The break-mode word for a force `n` past capacity (the mockup's `firstBreak.mode`).
    pub fn mode(&self, n: f64) -> &'static str {
        if n > 0.0 {
            if self.cracks { "cracked" } else { "snapped" }
        } else if self.buckles {
            "buckled"
        } else {
            "crushed"
        }
    }
}

/// Why an equilibrium could not be found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// A part can move without straining anything (a mechanism): it falls.
    Mechanism,
    /// Newton's method ran away (no equilibrium near: it tips over or folds).
    Diverged,
}

/// What happened while settling one step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Events {
    /// Pieces that broke (index, mode).
    pub breaks: Vec<(usize, &'static str)>,
    /// Foundations that ripped out (joint index).
    pub rips: Vec<usize>,
}

/// The mutable state of a solve: positions and what has given way.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// Joint positions, m.
    pub pos: Vec<[f64; 2]>,
    /// Broken pieces.
    pub broken: Vec<bool>,
    /// Foundations that ripped out.
    pub ripped: Vec<bool>,
    /// Ground-resting joints that lifted off.
    pub lifted: Vec<bool>,
    /// Consolidation settlement per joint (foundations), m, positive down.
    pub settle: Vec<f64>,
}

/// The structure to solve.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Structure {
    /// Joints.
    pub joints: Vec<SJoint>,
    /// Pieces.
    pub pieces: Vec<SPiece>,
}

/// Convergence: the largest Newton step, m.
const TOL: f64 = 1e-7;
/// Newton iterations per equilibrium.
const MAX_ITER: usize = 40;
/// A step larger than this means there is no equilibrium nearby, m.
const RUNAWAY: f64 = 25.0;
/// Rounds of "something gave way, solve again" per step.
const MAX_ROUNDS: usize = 24;
/// Diagonal regularisation relative to the largest stiffness, so a perfectly straight chain
/// of pieces (no load across it) isn't reported singular.
const REG: f64 = 1e-8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Support {
    Free,
    /// x pinned, y on the soil spring.
    Spring,
    /// Resting on the ground: x and y pinned while it pushes down.
    Ground,
}

impl Structure {
    /// The state before anything settles: design positions, nothing broken.
    pub fn initial(&self) -> State {
        let n = self.joints.len();
        State {
            pos: self.joints.iter().map(|j| [j.x, j.y]).collect(),
            broken: self.pieces.iter().map(|p| !self.valid_piece(p)).collect(),
            ripped: vec![false; n],
            lifted: vec![false; n],
            settle: vec![0.0; n],
        }
    }

    fn valid_piece(&self, p: &SPiece) -> bool {
        p.a < self.joints.len()
            && p.b < self.joints.len()
            && p.a != p.b
            && p.rest.is_finite()
            && p.rest > 1e-9
            && p.kk.is_finite()
            && p.kk > 0.0
    }

    /// Joints attached to a piece that still stands.
    pub fn active(&self, st: &State) -> Vec<bool> {
        let mut act = vec![false; self.joints.len()];
        for (i, p) in self.pieces.iter().enumerate() {
            if !st.broken[i] {
                act[p.a] = true;
                act[p.b] = true;
            }
        }
        act
    }

    fn support(&self, st: &State, j: usize) -> Support {
        let jt = &self.joints[j];
        if jt.found.is_some() {
            if st.ripped[j] {
                Support::Free
            } else {
                Support::Spring
            }
        } else if jt.on_ground && !st.lifted[j] {
            Support::Ground
        } else {
            Support::Free
        }
    }

    /// The axial force in piece `i` at the state's positions, N (tension positive; 0 when
    /// broken or slack).
    pub fn force(&self, st: &State, i: usize) -> f64 {
        if st.broken[i] {
            return 0.0;
        }
        let p = &self.pieces[i];
        let (a, b) = (st.pos[p.a], st.pos[p.b]);
        let l = (b[0] - a[0]).hypot(b[1] - a[1]);
        if p.t_only && l < p.rest {
            return 0.0;
        }
        p.kk * (l - p.rest)
    }

    /// Whether piece `i` is a slack rope / cable at the state's positions.
    pub fn slack(&self, st: &State, i: usize) -> bool {
        let p = &self.pieces[i];
        if st.broken[i] || !p.t_only {
            return false;
        }
        let (a, b) = (st.pos[p.a], st.pos[p.b]);
        (b[0] - a[0]).hypot(b[1] - a[1]) < p.rest
    }

    /// The upward reaction of joint `j`'s foundation, N (0 without a working foundation).
    pub fn reaction(&self, st: &State, j: usize) -> f64 {
        match self.joints[j].found {
            Some(f) if !st.ripped[j] => f.ks * ((self.joints[j].y - st.settle[j]) - st.pos[j][1]),
            _ => 0.0,
        }
    }

    /// The potential energy at `pos`, J: piece strain + soil springs + gravity.
    fn energy(&self, st: &State, act: &[bool], sup: &[Support], pos: &[[f64; 2]]) -> f64 {
        let mut e = 0.0;
        for (i, p) in self.pieces.iter().enumerate() {
            if st.broken[i] {
                continue;
            }
            let (a, b) = (pos[p.a], pos[p.b]);
            let l = (b[0] - a[0]).hypot(b[1] - a[1]);
            if p.t_only && l < p.rest {
                continue;
            }
            e += 0.5 * p.kk * (l - p.rest) * (l - p.rest);
        }
        for (j, jt) in self.joints.iter().enumerate() {
            if !act[j] {
                continue;
            }
            e += jt.mass * G * pos[j][1];
            if sup[j] == Support::Spring
                && let Some(f) = jt.found
            {
                let d = pos[j][1] - (jt.y - st.settle[j]);
                e += 0.5 * f.ks * d * d;
            }
        }
        e
    }

    /// Settles to the nearest stable equilibrium for the current supports, slack and broken
    /// pieces: minimises the potential energy by Newton steps with a positive-definite tangent
    /// (material stiffness, soil springs and the geometric stiffness of pulled pieces) and a
    /// backtracking line search, so it never balances on an unstable equilibrium — a tower past
    /// its tipping point keeps leaning instead.
    fn newton(&self, st: &mut State) -> Result<(), Failure> {
        let n = self.joints.len();
        let act = self.active(st);
        let sup: Vec<Support> = (0..n).map(|j| self.support(st, j)).collect();
        // degrees of freedom: x then y of each active joint that isn't pinned that way
        let mut dof = vec![[usize::MAX; 2]; n];
        let mut nd = 0;
        for j in 0..n {
            if !act[j] {
                continue;
            }
            match sup[j] {
                Support::Free => {
                    dof[j] = [nd, nd + 1];
                    nd += 2;
                }
                Support::Spring => {
                    dof[j][1] = nd;
                    nd += 1;
                }
                Support::Ground => {}
            }
        }
        if nd == 0 {
            return Ok(());
        }
        let start = st.pos.clone();
        let mut k = vec![0.0; nd * nd];
        let mut f = vec![0.0; nd];
        let mut trial = st.pos.clone();
        let mut e0 = self.energy(st, &act, &sup, &st.pos);
        for _ in 0..MAX_ITER {
            k.iter_mut().for_each(|v| *v = 0.0);
            f.iter_mut().for_each(|v| *v = 0.0);
            for j in 0..n {
                if act[j] && dof[j][1] != usize::MAX {
                    f[dof[j][1]] -= self.joints[j].mass * G;
                }
            }
            for (i, p) in self.pieces.iter().enumerate() {
                if st.broken[i] {
                    continue;
                }
                let (a, b) = (st.pos[p.a], st.pos[p.b]);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let l = dx.hypot(dy);
                if l < 1e-9 || (p.t_only && l < p.rest) {
                    continue;
                }
                let (ex, ey) = (dx / l, dy / l);
                let nf = p.kk * (l - p.rest);
                let ia = dof[p.a];
                let ib = dof[p.b];
                // the internal force pulls a toward b when stretched
                add_f(&mut f, ia[0], nf * ex);
                add_f(&mut f, ia[1], nf * ey);
                add_f(&mut f, ib[0], -nf * ex);
                add_f(&mut f, ib[1], -nf * ey);
                // geometric stiffness only where it stiffens (pulled): the tangent stays
                // positive definite and each step goes downhill
                let g = (nf / l).max(0.0);
                let km = [
                    [p.kk * ex * ex + g * (1.0 - ex * ex), (p.kk - g) * ex * ey],
                    [(p.kk - g) * ex * ey, p.kk * ey * ey + g * (1.0 - ey * ey)],
                ];
                let ends = [ia, ib];
                for (ra, ri) in ends.iter().enumerate() {
                    for (ca, ci) in ends.iter().enumerate() {
                        let sign = if ra == ca { 1.0 } else { -1.0 };
                        for r in 0..2 {
                            for c in 0..2 {
                                let (dr, dc) = (ri[r], ci[c]);
                                if dr != usize::MAX && dc != usize::MAX {
                                    k[dr * nd + dc] += sign * km[r][c];
                                }
                            }
                        }
                    }
                }
            }
            for j in 0..n {
                if act[j]
                    && sup[j] == Support::Spring
                    && let Some(s) = self.joints[j].found
                {
                    let d = dof[j][1];
                    k[d * nd + d] += s.ks;
                    f[d] -= s.ks * (st.pos[j][1] - (self.joints[j].y - st.settle[j]));
                }
            }
            let kmax = (0..nd).map(|d| k[d * nd + d].abs()).fold(0.0, f64::max);
            if !kmax.is_finite() || kmax == 0.0 {
                return Err(Failure::Mechanism);
            }
            for d in 0..nd {
                k[d * nd + d] += kmax * REG;
            }
            if !cholesky_solve(nd, &mut k, &mut f) {
                return Err(Failure::Mechanism);
            }
            let du = &f;
            let big = du.iter().fold(0.0f64, |m, v| m.max(v.abs()));
            if !big.is_finite() {
                return Err(Failure::Diverged);
            }
            // backtracking line search on the energy
            let mut alpha = 1.0;
            let mut accepted = false;
            for _ in 0..30 {
                for j in 0..n {
                    for c in 0..2 {
                        let d = dof[j][c];
                        trial[j][c] = if d != usize::MAX {
                            st.pos[j][c] + alpha * du[d]
                        } else {
                            st.pos[j][c]
                        };
                    }
                }
                let e1 = self.energy(st, &act, &sup, &trial);
                if e1.is_finite() && e1 <= e0 + 1e-12 * e0.abs().max(1.0) {
                    e0 = e1;
                    accepted = true;
                    break;
                }
                alpha *= 0.5;
            }
            if !accepted {
                // no downhill step left: as settled as it gets
                return Ok(());
            }
            std::mem::swap(&mut st.pos, &mut trial);
            let moved = st.pos.iter().zip(&start).fold(0.0f64, |m, (p, q)| {
                m.max((p[0] - q[0]).abs()).max((p[1] - q[1]).abs())
            });
            if !moved.is_finite() || moved > RUNAWAY {
                return Err(Failure::Diverged);
            }
            if alpha * big < TOL {
                return Ok(());
            }
        }
        // slow convergence (sliding toward a tipping point, or a rope flipping slack / taut):
        // keep where it got to; the next step continues from here
        Ok(())
    }

    /// The vertical reaction the ground gives a resting joint, N (from the residual).
    fn ground_reaction(&self, st: &State, j: usize) -> f64 {
        let mut fy = -self.joints[j].mass * G;
        for (i, p) in self.pieces.iter().enumerate() {
            if p.a != j && p.b != j {
                continue;
            }
            let nf = self.force(st, i);
            if nf == 0.0 {
                continue;
            }
            let (a, b) = (st.pos[p.a], st.pos[p.b]);
            let l = (b[0] - a[0]).hypot(b[1] - a[1]).max(1e-9);
            let ey = (b[1] - a[1]) / l;
            fy += if p.a == j { nf * ey } else { -nf * ey };
        }
        -fy
    }

    /// Settles to equilibrium, letting things give way: ropes go slack, resting joints lift,
    /// foundations rip out past their pull capacity, pieces break past theirs (all at once per
    /// round, then solved again). Returns what gave way.
    pub fn settle(&self, st: &mut State) -> Result<Events, Failure> {
        let mut ev = Events::default();
        for _ in 0..MAX_ROUNDS {
            self.newton(st)?;
            let mut changed = false;
            let act = self.active(st);
            for j in 0..self.joints.len() {
                if !act[j] {
                    continue;
                }
                match self.support(st, j) {
                    Support::Spring => {
                        let f = self.joints[j].found.map_or(0.0, |s| s.qt);
                        if -self.reaction(st, j) > f {
                            st.ripped[j] = true;
                            ev.rips.push(j);
                            changed = true;
                        }
                    }
                    Support::Ground => {
                        if self.ground_reaction(st, j) < 0.0 {
                            st.lifted[j] = true;
                            changed = true;
                        }
                    }
                    Support::Free => {}
                }
            }
            for i in 0..self.pieces.len() {
                if st.broken[i] {
                    continue;
                }
                let nf = self.force(st, i);
                let p = &self.pieces[i];
                if nf > p.cap_t || -nf > p.cap_c {
                    st.broken[i] = true;
                    ev.breaks.push((i, p.mode(nf)));
                    changed = true;
                }
            }
            if !changed {
                return Ok(ev);
            }
        }
        Ok(ev)
    }
}

/// Solves `K·x = b` in place (`b` becomes `x`) for a symmetric positive-definite dense `K`
/// (row-major `n × n`, its lower triangle overwritten with the Cholesky factor). Works inside
/// each row's envelope (from its first non-zero), which Cholesky's fill-in never leaves, so a
/// structure numbered level by level costs about `n·b²`. `false` when `K` isn't positive
/// definite (a mechanism).
fn cholesky_solve(n: usize, k: &mut [f64], b: &mut [f64]) -> bool {
    let first: Vec<usize> = (0..n)
        .map(|r| (0..=r).find(|&q| k[r * n + q] != 0.0).unwrap_or(r))
        .collect();
    for c in 0..n {
        let fc = first[c];
        let mut d = k[c * n + c];
        for q in fc..c {
            d -= k[c * n + q] * k[c * n + q];
        }
        if d.is_nan() || d <= 0.0 || !d.is_finite() {
            return false;
        }
        let d = d.sqrt();
        k[c * n + c] = d;
        for r in c + 1..n {
            if first[r] > c {
                continue;
            }
            let mut v = k[r * n + c];
            for q in first[r].max(fc)..c {
                v -= k[r * n + q] * k[c * n + q];
            }
            k[r * n + c] = v / d;
        }
    }
    for r in 0..n {
        let mut v = b[r];
        for q in first[r]..r {
            v -= k[r * n + q] * b[q];
        }
        b[r] = v / k[r * n + r];
    }
    for r in (0..n).rev() {
        let mut v = b[r];
        for q in r + 1..n {
            if first[q] <= r {
                v -= k[q * n + r] * b[q];
            }
        }
        b[r] = v / k[r * n + r];
    }
    true
}

fn add_f(f: &mut [f64], d: usize, v: f64) {
    if d != usize::MAX {
        f[d] += v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spring(ks: f64) -> Option<Spring> {
        Some(Spring {
            qc: 1e9,
            qt: 1e9,
            ks,
            consolidation: None,
        })
    }

    fn joint(id: &str, x: f64, y: f64, mass: f64, found: Option<Spring>) -> SJoint {
        SJoint {
            id: id.into(),
            x,
            y,
            mass,
            found,
            on_ground: false,
        }
    }

    fn piece(a: usize, b: usize, rest: f64, kk: f64) -> SPiece {
        SPiece {
            id: format!("M{a}{b}"),
            a,
            b,
            rest,
            kk,
            cap_t: 1e12,
            cap_c: 1e12,
            t_only: false,
            buckles: false,
            cracks: false,
        }
    }

    #[test]
    fn a_column_on_a_spring_sinks_by_its_weight_over_the_springs() {
        // two springs side by side under a triangle: each carries half
        let s = Structure {
            joints: vec![
                joint("J1", 0.0, 0.0, 0.0, spring(1e6)),
                joint("J2", 4.0, 0.0, 0.0, spring(1e6)),
                joint("J3", 2.0, 4.0, 1000.0, None),
            ],
            pieces: vec![
                piece(0, 1, 4.0, 1e9),
                piece(0, 2, 20f64.sqrt(), 1e9),
                piece(1, 2, 20f64.sqrt(), 1e9),
            ],
        };
        let mut st = s.initial();
        s.settle(&mut st).unwrap();
        let r0 = s.reaction(&st, 0);
        let r1 = s.reaction(&st, 1);
        assert!((r0 - 500.0 * G).abs() < 1e-3, "{r0}");
        assert!((r1 - 500.0 * G).abs() < 1e-3, "{r1}");
        assert!((st.pos[0][1] + 500.0 * G / 1e6).abs() < 1e-9);
    }

    #[test]
    fn a_slack_rope_lets_its_joint_fall_until_it_hangs() {
        let mut p = piece(0, 1, 2.0, 1e6);
        p.t_only = true;
        let s = Structure {
            joints: vec![
                joint("J1", 0.0, 0.0, 0.0, spring(1e8)),
                joint("J2", 0.0, 1.0, 100.0, None),
            ],
            pieces: vec![p],
        };
        let mut st = s.initial();
        // the rope (rest 2 m) is only 1 m long: it is slack, so the joint falls past J1 and
        // ends up hanging 2 m (and a stretch) below it
        s.settle(&mut st).unwrap();
        assert!(
            st.pos[1][1] < -2.0 && st.pos[1][1] > -2.01,
            "{:?}",
            st.pos[1]
        );
        assert!(st.pos[1][0].abs() < 1e-6);
    }

    #[test]
    fn a_hanging_crate_on_a_rope_hangs() {
        let mut p = piece(0, 1, 2.0, 1e6);
        p.t_only = true;
        let s = Structure {
            joints: vec![
                SJoint {
                    on_ground: false,
                    ..joint("J1", 0.0, 10.0, 0.0, None)
                },
                joint("J2", 0.0, 8.0, 1000.0, None),
            ],
            pieces: vec![p],
        };
        // pin J1 by making it a stiff foundation with x pinned
        let mut s = s;
        s.joints[0].found = spring(1e12);
        let mut st = s.initial();
        s.settle(&mut st).unwrap();
        let stretch = 1000.0 * G / 1e6;
        assert!(
            (st.pos[1][1] - (8.0 - stretch)).abs() < 1e-6,
            "{:?}",
            st.pos[1]
        );
        assert!((s.force(&st, 0) - 1000.0 * G).abs() < 1e-3);
    }

    #[test]
    fn cholesky_solves_a_small_system() {
        let mut k = vec![4.0, 2.0, 0.0, 2.0, 5.0, 1.0, 0.0, 1.0, 3.0];
        let mut b = vec![2.0, 3.0, 4.0];
        assert!(cholesky_solve(3, &mut k, &mut b));
        let x = b;
        let r = [
            4.0 * x[0] + 2.0 * x[1],
            2.0 * x[0] + 5.0 * x[1] + x[2],
            x[1] + 3.0 * x[2],
        ];
        for (got, want) in r.iter().zip([2.0, 3.0, 4.0]) {
            assert!((got - want).abs() < 1e-12);
        }
        let mut k = vec![1.0, 2.0, 2.0, 1.0];
        let mut b = vec![1.0, 1.0];
        assert!(!cholesky_solve(2, &mut k, &mut b), "indefinite");
    }

    #[test]
    fn overloaded_pieces_break() {
        let mut p = piece(0, 1, 2.0, 1e6);
        p.cap_t = 100.0;
        let s = Structure {
            joints: vec![
                joint("J1", 0.0, 10.0, 0.0, spring(1e12)),
                joint("J2", 0.0, 8.0, 1000.0, None),
            ],
            pieces: vec![p],
        };
        let mut st = s.initial();
        let ev = s.settle(&mut st).unwrap();
        assert_eq!(ev.breaks, vec![(0, "snapped")]);
        assert!(st.broken[0]);
    }
}
