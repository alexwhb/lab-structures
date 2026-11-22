//! The collapse: a position-based-dynamics replay of the structure falling apart
//! (`initSim()` / `stepSim()` in `Playground.dc.html`, step for step).
//!
//! Joints are Verlet particles (60 kg each plus the hung weight and half of every member's
//! mass), members are distance constraints solved in 14 passes per step, and a member tears
//! (with a red spark) when it is stretched or squeezed more than 5 %. Pins stay put, rollers
//! keep their height, joints come to rest on the banks and the riverbed (sliding with 70 %
//! friction), and the water drags anything below −8 m. Illustrative: it shows *how* a
//! structure gives way, not a dynamic analysis.

use super::data::{MaterialKey, Support, ground};
use super::design::Design;
use crate::truss::{G, hypot};

/// Velocity kept per step (`0.996`).
pub const DAMPING: f64 = 0.996;
/// Constraint passes per step.
pub const PASSES: usize = 14;
/// A member tears past this strain.
pub const TEAR_STRAIN: f64 = 0.05;
/// Below this height the water drags (velocity × 0.86 per step).
pub const WATER_Y: f64 = -8.0;
/// Water drag per step.
pub const WATER_DRAG: f64 = 0.86;
/// Joints are kept within x 0–62 m.
pub const X_LIMIT: f64 = 62.0;
/// The collapse plays for 3.2 s; the slow-motion replay for 1.6 s of simulated time at 0.3×.
pub const DURATION: f64 = 3.2;
/// Slow-motion replay length, simulated seconds.
pub const SLOW_DURATION: f64 = 1.6;
/// Slow-motion factor.
pub const SLOW_FACTOR: f64 = 0.3;
/// Steps per 1/30 s tick.
pub const STEPS_PER_TICK: usize = 3;

/// One joint in the simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    /// Position, m.
    pub x: f64,
    /// Position, m.
    pub y: f64,
    /// Previous position (Verlet), m.
    pub qx: f64,
    /// Previous position, m.
    pub qy: f64,
    /// Mass, kg.
    pub m: f64,
    /// 1 / mass, or 0 for a joint that doesn't move (pins, and loose unloaded joints).
    pub inv: f64,
    /// Its support.
    pub sup: Option<Support>,
    /// Starting height (rollers keep it), m.
    pub y0: f64,
    /// Attached to a member.
    pub att: bool,
}

/// A member as a distance constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// Particle index (= joint index in the design).
    pub a: usize,
    /// Particle index.
    pub b: usize,
    /// Rest length, m.
    pub l: f64,
    /// The member's id.
    pub id: String,
}

/// The collapse simulation state.
#[derive(Debug, Clone, PartialEq)]
pub struct Sim {
    /// One particle per design joint, in the design's order.
    pub p: Vec<Particle>,
    /// The members still holding.
    pub cons: Vec<Constraint>,
    /// Where members broke or tore (m), each drawn as a red ×.
    pub sparks: Vec<[f64; 2]>,
    /// Simulated time, s.
    pub t: f64,
    /// Slow-motion replay.
    pub slow: bool,
    /// Members that tore during the simulation, in order.
    pub lost: Vec<String>,
}

impl Sim {
    /// Starts the simulation from the design at rest (`initSim(st, broken, slow)`): broken
    /// members are left out and get a spark at their middle.
    pub fn new(design: &Design, mat: MaterialKey, broken: &[String], slow: bool) -> Sim {
        let rho = mat.spec().rho;
        let mut p: Vec<Particle> = design
            .nodes
            .iter()
            .map(|n| Particle {
                x: n.x as f64,
                y: n.y as f64,
                qx: n.x as f64,
                qy: n.y as f64,
                m: 60.0 + design.load(&n.id) * 1000.0,
                inv: 0.0,
                sup: n.sup,
                y0: n.y as f64,
                att: false,
            })
            .collect();
        let mut cons = Vec::new();
        let mut sparks = Vec::new();
        for bar in &design.members {
            let (Some(a), Some(b)) = (design.joint_index(&bar.a), design.joint_index(&bar.b))
            else {
                continue;
            };
            if broken.contains(&bar.id) {
                continue;
            }
            let l = hypot(p[b].x - p[a].x, p[b].y - p[a].y);
            let w = bar.size.area() * l * rho;
            p[a].m += w / 2.0;
            p[b].m += w / 2.0;
            p[a].att = true;
            p[b].att = true;
            cons.push(Constraint {
                a,
                b,
                l,
                id: bar.id.clone(),
            });
        }
        for (i, q) in p.iter_mut().enumerate() {
            let loaded = design.load(&design.nodes[i].id) != 0.0;
            q.inv = if q.sup == Some(Support::Pin) || (!q.att && q.sup.is_none() && !loaded) {
                0.0
            } else {
                1.0 / q.m
            };
        }
        for bar in &design.members {
            if !broken.contains(&bar.id) {
                continue;
            }
            if let (Some(a), Some(b)) = (design.joint_index(&bar.a), design.joint_index(&bar.b)) {
                sparks.push([(p[a].x + p[b].x) / 2.0, (p[a].y + p[b].y) / 2.0]);
            }
        }
        Sim {
            p,
            cons,
            sparks,
            t: 0.0,
            slow,
            lost: Vec::new(),
        }
    }

    /// One step of `dt` seconds (`stepSim(dt)`).
    pub fn step(&mut self, dt: f64) {
        for p in self.p.iter_mut() {
            if p.inv == 0.0 {
                continue;
            }
            let mut vx = (p.x - p.qx) * DAMPING;
            let mut vy = (p.y - p.qy) * DAMPING;
            if p.y < WATER_Y {
                vx *= WATER_DRAG;
                vy *= WATER_DRAG;
            }
            p.qx = p.x;
            p.qy = p.y;
            p.x += vx;
            p.y += vy - G * dt * dt;
        }
        for it in 0..PASSES {
            let mut k = self.cons.len();
            while k > 0 {
                k -= 1;
                let (ia, ib, l) = (self.cons[k].a, self.cons[k].b, self.cons[k].l);
                let (a, b) = (self.p[ia], self.p[ib]);
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let h = hypot(dx, dy);
                let len = if h == 0.0 || h.is_nan() { 1e-6 } else { h };
                if it == 0 && ((len - l) / l).abs() > TEAR_STRAIN {
                    let c = self.cons.remove(k);
                    self.sparks.push([(a.x + b.x) / 2.0, (a.y + b.y) / 2.0]);
                    self.lost.push(c.id);
                    continue;
                }
                let w = a.inv + b.inv;
                if w == 0.0 {
                    continue;
                }
                let f = (len - l) / len / w;
                let pa = &mut self.p[ia];
                pa.x += dx * f * a.inv;
                pa.y += dy * f * a.inv;
                let pb = &mut self.p[ib];
                pb.x -= dx * f * b.inv;
                pb.y -= dy * f * b.inv;
            }
            for p in self.p.iter_mut() {
                if p.sup == Some(Support::Roller) {
                    p.y = p.y0;
                }
            }
        }
        for p in self.p.iter_mut() {
            let g = ground(p.x);
            if p.y < g {
                p.y = g;
                p.qx = p.x - (p.x - p.qx) * 0.3;
                p.qy = p.y;
            }
            p.x = p.x.clamp(0.0, X_LIMIT);
            // keep the drawing sane even if something went non-finite
            if !p.x.is_finite() || !p.y.is_finite() {
                p.x = if p.qx.is_finite() {
                    p.qx.clamp(0.0, X_LIMIT)
                } else {
                    0.0
                };
                p.y = if p.qy.is_finite() { p.qy } else { p.y0 };
                p.qx = p.x;
                p.qy = p.y;
            }
        }
        self.t += dt;
    }

    /// The step length: three steps per 1/30 s tick, × 0.3 in slow motion.
    pub fn dt(&self) -> f64 {
        (1.0 / 30.0) * (if self.slow { SLOW_FACTOR } else { 1.0 }) / STEPS_PER_TICK as f64
    }

    /// How long it plays, simulated seconds.
    pub fn duration(&self) -> f64 {
        if self.slow { SLOW_DURATION } else { DURATION }
    }

    /// One 1/30 s tick (three steps). Returns true when the collapse has finished playing.
    pub fn tick(&mut self) -> bool {
        let dt = self.dt();
        for _ in 0..STEPS_PER_TICK {
            self.step(dt);
        }
        self.t > self.duration()
    }

    /// True if the member is still in one piece in the simulation.
    pub fn holds(&self, id: &str) -> bool {
        !self.lost.iter().any(|l| l == id)
    }
}
