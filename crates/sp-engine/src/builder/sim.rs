//! The settling engine (`makeSim`, `support`, `step` and the per-frame part of `tick`): one
//! XPBD simulation for everything — pieces as distance constraints with compliance
//! 1/(E/15·A/L), tension-only pieces that go slack, buckling, foundations as soil springs that
//! creep, rip out and liquefy, gravity ramped in, snow, wind with gusts and deck flutter,
//! earthquakes and vehicles. Operation for operation as the mockup, so it matches it bit for
//! bit (the golden test runs the mockup's own JS).
//!
//! Hooks for the fold-ins (all default to "no effect", which is bit-identical to the mockup):
//! - per piece [`Constraint::strength`] and [`Constraint::stiffness`] factors (fire: agent
//!   `lab`'s `fire::update`, called once per frame by [`Sim::frame`] when `cond.fire > 0`),
//!   [`Constraint::temp`] (°C) and [`Constraint::heat_mode`] (the break mode a heat-weakened
//!   piece reports: "softened" / "burned through");
//! - [`Sim::drag`] and [`Sim::shedding`] (CMF-2's shape variants, agent `time`);
//! - [`Sim::fire_start`] (when the fire was lit).

use super::data::{Foundation, PieceType, Scene, VehicleKind};
use super::design::Design;
use super::jsmath;
use super::setup::{Conditions, Setup};
use crate::jsnum::round;
use crate::truss::hypot;
use std::f64::consts::PI;

/// Standard gravity, m/s² (the mockup's 9.81).
pub const G: f64 = 9.81;
/// Substeps per running frame.
pub const SUBSTEPS: usize = 20;
/// A running frame, s (1/30).
pub const FRAME: f64 = 1.0 / 30.0;
/// Slow motion's time factor.
pub const SLOW: f64 = 0.3;
/// An open-ground test lasts this long, s.
pub const TEST_SECONDS: f64 = 18.0;
/// A river run ends at the latest after this long, s.
pub const RIVER_MAX_SECONDS: f64 = 45.0;
/// … or this long after the last vehicle is done, s.
pub const RIVER_TAIL_SECONDS: f64 = 1.2;

/// How a piece broke (`firstBreak.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakMode {
    /// Pulled apart ("snapped").
    Snapped,
    /// Concrete pulled apart ("cracked").
    Cracked,
    /// Pushed, bowed sideways ("buckled").
    Buckled,
    /// Pushed past its strength ("crushed").
    Crushed,
    /// Steel or cable weakened by heat ("softened"; SPEC FIRE-1, set by `fire`).
    Softened,
    /// Wood or rope burned ("burned through"; SPEC FIRE-1, set by `fire`).
    BurnedThrough,
}

impl BreakMode {
    /// The word in the sentences: "snapped", "cracked", "buckled", "crushed", "softened",
    /// "burned through".
    pub fn text(self) -> &'static str {
        match self {
            BreakMode::Snapped => "snapped",
            BreakMode::Cracked => "cracked",
            BreakMode::Buckled => "buckled",
            BreakMode::Crushed => "crushed",
            BreakMode::Softened => "softened",
            BreakMode::BurnedThrough => "burned through",
        }
    }
}

/// How a foundation failed (`firstFound.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoundMode {
    /// Pushed into the soil past its capacity ("sank").
    Sank,
    /// Sank while the wet sand was shaking ("liquefied").
    Liquefied,
    /// Pulled out of the ground ("ripped out").
    RippedOut,
}

impl FoundMode {
    /// "sank", "liquefied", "ripped out".
    pub fn text(self) -> &'static str {
        match self {
            FoundMode::Sank => "sank",
            FoundMode::Liquefied => "liquefied",
            FoundMode::RippedOut => "ripped out",
        }
    }
}

/// The first piece that broke (`S.firstBreak`).
#[derive(Debug, Clone, PartialEq)]
pub struct FirstBreak {
    /// Piece id.
    pub id: String,
    /// Its type.
    pub kind: PieceType,
    /// How.
    pub mode: BreakMode,
    /// When, s.
    pub t: f64,
}

/// The first foundation that failed (`S.firstFound`).
#[derive(Debug, Clone, PartialEq)]
pub struct FirstFound {
    /// Joint id.
    pub id: String,
    /// How.
    pub mode: FoundMode,
}

/// A joint in the simulation (`P[i]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    /// Joint id.
    pub id: String,
    /// Position, m.
    pub x: f64,
    /// Position, m.
    pub y: f64,
    /// Position before this step, m.
    pub px: f64,
    /// Position before this step, m.
    pub py: f64,
    /// Velocity, m/s.
    pub vx: f64,
    /// Velocity, m/s.
    pub vy: f64,
    /// Mass, kg: 40 + crate + 800 for a foundation + half of each piece.
    pub m: f64,
    /// Applied force this step, N.
    pub fx: f64,
    /// Applied force this step, N.
    pub fy: f64,
    /// Its foundation (`None` once ripped out).
    pub sup: Option<Foundation>,
    /// Design position, m (a foundation's x follows the ground: `x0 + shake`).
    pub x0: f64,
    /// Design position, m.
    pub y0: f64,
    /// The foundation's settled level, m (creeps down when pushed past capacity).
    pub ys: f64,
    /// Touched by a piece.
    pub used: bool,
    /// Smoothed foundation reaction, N (positive pushing down on the soil).
    pub r: f64,
    /// Ripped out of the ground.
    pub ripped: bool,
    /// Pushed past its push capacity this step.
    pub over_c: bool,
    /// Pulled past its pull capacity this step.
    pub over_t: bool,
    /// Push capacity, N.
    pub qc: f64,
    /// Pull capacity, N.
    pub qt: f64,
    /// Soil spring stiffness, N/m.
    pub ks: f64,
    /// On liquefiable soil.
    pub liq: bool,
    /// 1/m, or 0 for a joint nothing touches or loads (it stays put).
    pub inv: f64,
}

/// A piece in the simulation (`C[i]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// Piece id.
    pub id: String,
    /// Particle index.
    pub a: usize,
    /// Particle index.
    pub b: usize,
    /// Its type.
    pub kind: PieceType,
    /// Rest length, m (the design length).
    pub len: f64,
    /// Axial stiffness E/15·A/L, N/m (`kk`).
    pub kk: f64,
    /// Tension capacity A·f_t, N (before [`Constraint::strength`]).
    pub cap_t: f64,
    /// Compression capacity min(A·f_c, P_cr), N (∞ for tension-only; before the factors).
    pub cap_c: f64,
    /// A·f_c, N (∞ for tension-only).
    pub crush: f64,
    /// Euler load π²·E·k·A²/L², N.
    pub pcr: f64,
    /// Buckles before it crushes (`pcr < A·fC`).
    pub buckles: bool,
    /// Only pulls.
    pub t_only: bool,
    /// A deck (Road, Floor).
    pub deck: bool,
    /// Road (vehicles drive on it; flutters in the wind).
    pub road: bool,
    /// Axial force this step, N (+ tension).
    pub n: f64,
    /// Smoothed force, N (`avg = 0.96·avg + 0.04·N`); breaks and colours use this.
    pub avg: f64,
    /// Broken.
    pub broken: bool,
    /// Past capacity (preview only).
    pub over: bool,
    /// Slack this step (tension-only, shorter than its rest length).
    pub slack: bool,
    /// A deck with nothing above it: snow piles on it.
    pub exposed: bool,
    /// Fire hook: strength factor (1.0), scales the tension capacity and the crushing part of
    /// the compression capacity.
    pub strength: f64,
    /// Fire hook: stiffness factor (1.0), scales the XPBD stiffness and the Euler load.
    pub stiffness: f64,
    /// Fire hook: the piece's temperature, °C (20; drawn as a glow).
    pub temp: f64,
    /// Fire hook: if set, a break reports this mode (softened, burned through).
    pub heat_mode: Option<BreakMode>,
}

impl Constraint {
    /// Tension capacity now, N (`cap_t × strength`).
    pub fn cap_t_now(&self) -> f64 {
        self.cap_t * self.strength
    }
    /// Compression capacity now, N (`min(crush × strength, pcr × stiffness)`, ∞ for
    /// tension-only).
    pub fn cap_c_now(&self) -> f64 {
        if self.t_only {
            f64::INFINITY
        } else {
            (self.crush * self.strength).min(self.pcr * self.stiffness)
        }
    }
    /// Utilisation of the smoothed force (`utOf`): N/capT pulled, −N/capC pushed (0 for a
    /// tension-only piece pushed).
    pub fn utilisation(&self) -> f64 {
        utilisation(self.avg, self.cap_t_now(), self.cap_c_now())
    }
}

/// `utOf(q)`: `N/capT` when pulled, `−N/capC` when pushed, 0 when capC is infinite.
pub fn utilisation(n: f64, cap_t: f64, cap_c: f64) -> f64 {
    if n >= 0.0 {
        n / cap_t
    } else if cap_c == f64::INFINITY {
        0.0
    } else {
        -n / cap_c
    }
}

/// What a vehicle is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VehicleState {
    /// Driving (on a Road piece or a bank).
    Drive,
    /// Falling: an axle ran out of road.
    Fall,
    /// In the river.
    Sunk,
    /// Fell onto the ground or riverbed.
    Crashed,
    /// Across (past x = 70 m).
    Done,
}

impl VehicleState {
    /// The mockup's word: `drive`, `fall`, `sunk`, `crashed`, `done`.
    pub fn key(self) -> &'static str {
        match self {
            VehicleState::Drive => "drive",
            VehicleState::Fall => "fall",
            VehicleState::Sunk => "sunk",
            VehicleState::Crashed => "crashed",
            VehicleState::Done => "done",
        }
    }
}

/// Where an axle rests (`support()`'s result).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axle {
    /// The Road constraint under it (`None`: on a bank).
    pub c: Option<usize>,
    /// Where along the piece, 0…1.
    pub f: f64,
    /// Road height there, m.
    pub y: f64,
    /// Distance from the hint height, m.
    pub dist: f64,
}

/// A vehicle (`V[i]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Vehicle {
    /// Its kind.
    pub kind: VehicleKind,
    /// Mass, kg.
    pub m: f64,
    /// Length, m.
    pub len: f64,
    /// Height, m.
    pub ht: f64,
    /// Axle half-spacing, m.
    pub d: f64,
    /// Speed, m/s.
    pub speed: f64,
    /// Centre x, m.
    pub x: f64,
    /// Road height under it, m.
    pub y: f64,
    /// Body angle, rad.
    pub ang: f64,
    /// State.
    pub state: VehicleState,
    /// Falling velocity, m/s.
    pub vx: f64,
    /// Falling velocity, m/s.
    pub vy: f64,
    /// Rear and front axle supports (from the last step).
    pub ax: [Option<Axle>; 2],
    /// Sandbox crossings.
    pub laps: u32,
}

/// A windward floor-edge joint (`windL` / `windR`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindJoint {
    /// Particle index.
    pub i: usize,
    /// The floor level, m (`Math.round(y)`).
    pub y: f64,
}

/// A splash where a vehicle sank.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Splash {
    /// x, m.
    pub x: f64,
    /// When, s.
    pub t: f64,
}

/// The simulation (`makeSim`'s `S`).
#[derive(Debug, Clone, PartialEq)]
pub struct Sim {
    /// Joints.
    pub p: Vec<Particle>,
    /// Pieces.
    pub c: Vec<Constraint>,
    /// Vehicles.
    pub v: Vec<Vehicle>,
    /// The level as it runs.
    pub setup: Setup,
    /// The live conditions (the Game copies them in every frame: wind, snow and quake can
    /// change while running).
    pub cond: Conditions,
    /// Live run (wind, quake, vehicles, breaks) or a static settle (preview).
    pub live: bool,
    /// Time, s.
    pub t: f64,
    /// Red ✕ where pieces broke or foundations ripped out, m.
    pub sparks: Vec<[f64; 2]>,
    /// Splashes.
    pub splashes: Vec<Splash>,
    /// The first piece that broke.
    pub first_break: Option<FirstBreak>,
    /// The first foundation that failed.
    pub first_found: Option<FirstFound>,
    /// River runs end at this time (1.2 s after the last vehicle is done).
    pub end_at: Option<f64>,
    /// Windward joints when the wind blows from the left (leftmost floor joint per level).
    pub wind_l: Vec<WindJoint>,
    /// … from the right.
    pub wind_r: Vec<WindJoint>,
    /// Structure width, m (≥ 8).
    pub width: f64,
    /// The top floor joint (highest joint touched by a Floor).
    pub top_i: Option<usize>,
    /// Its design height, m (0 with no floors).
    pub top_y0: f64,
    /// Peak sway of the top floor joint, m.
    pub sway: f64,
    /// Peak smoothed top-floor acceleration, m/s².
    pub peak_a: f64,
    /// Smoothed acceleration, m/s².
    pub a_s: f64,
    /// Top joint's x-velocity at the last frame.
    pub last_vx: f64,
    /// Quake envelope now, 0…1.
    pub env: f64,
    /// Wind now, m/s (eases toward the setting).
    pub wind_now: f64,
    /// Snow now, kg/m (rises toward the setting).
    pub snow_now: f64,
    /// When the quake (envelope) starts, s.
    pub quake_start: Option<f64>,
    /// The quake setting at the last step.
    pub last_quake: f64,
    /// The Sandbox: never ends, vehicles loop.
    pub sandbox: bool,
    /// CMF-2 hook: scales the floor wind force (1.0; the mockup's Cd 1.3 is built in).
    pub drag: f64,
    /// CMF-2 hook: scales the 0.3 vortex-shedding gust term (1.0).
    pub shedding: f64,
    /// FIRE-1 hook: when the fire lights / lit, s (`None` until `fire::update` lights it).
    pub fire_start: Option<f64>,
}

impl Sim {
    /// `makeSim(st, live)`.
    pub fn new(design: &Design, setup: &Setup, cond: &Conditions, live: bool) -> Sim {
        let mut p: Vec<Particle> = design
            .nodes
            .iter()
            .map(|n| {
                let mut q = Particle {
                    id: n.id.clone(),
                    x: n.x,
                    y: n.y,
                    px: n.x,
                    py: n.y,
                    vx: 0.0,
                    vy: 0.0,
                    m: 40.0 + design.load(&n.id) * 1000.0,
                    fx: 0.0,
                    fy: 0.0,
                    sup: n.sup,
                    x0: n.x,
                    y0: n.y,
                    ys: n.y,
                    used: false,
                    r: 0.0,
                    ripped: false,
                    over_c: false,
                    over_t: false,
                    qc: 0.0,
                    qt: 0.0,
                    ks: 0.0,
                    liq: false,
                    inv: 0.0,
                };
                if let Some(f) = n.sup {
                    let so = setup.soil_at(n);
                    let g = so.grip(f);
                    q.qc = g[0];
                    q.qt = g[1];
                    q.ks = g[2];
                    q.liq = so.spec().liq;
                    q.m += 800.0;
                }
                q
            })
            .collect();
        let mut c: Vec<Constraint> = Vec::with_capacity(design.members.len());
        for m in &design.members {
            let (Some(a), Some(b)) = (design.node_index(&m.a), design.node_index(&m.b)) else {
                continue;
            };
            let t = m.kind.spec();
            let lm = hypot(p[b].x - p[a].x, p[b].y - p[a].y);
            let mass = m.kind.mass_per_m() * lm;
            p[a].m += mass / 2.0;
            p[b].m += mass / 2.0;
            p[a].used = true;
            p[b].used = true;
            let cap = capacity(m.kind, lm);
            c.push(Constraint {
                id: m.id.clone(),
                a,
                b,
                kind: m.kind,
                len: lm,
                kk: cap.kk,
                cap_t: cap.cap_t,
                cap_c: cap.cap_c,
                crush: cap.crush,
                pcr: cap.pcr,
                buckles: cap.buckles,
                t_only: t.t_only,
                deck: t.deck,
                road: m.kind == PieceType::Road,
                n: 0.0,
                avg: 0.0,
                broken: false,
                over: false,
                slack: false,
                exposed: false,
                strength: 1.0,
                stiffness: 1.0,
                temp: 20.0,
                heat_mode: None,
            });
        }
        // snow sits on decks and floors with nothing above them
        let nodes = &design.nodes;
        let exposed: Vec<bool> = c
            .iter()
            .enumerate()
            .map(|(ci, cc)| {
                if !cc.deck {
                    return false;
                }
                let (a, b) = (&nodes[cc.a], &nodes[cc.b]);
                let mx = (a.x + b.x) / 2.0;
                let my = (a.y + b.y) / 2.0;
                !c.iter().enumerate().any(|(oi, o)| {
                    if oi == ci || !o.deck {
                        return false;
                    }
                    let (pp, qq) = (&nodes[o.a], &nodes[o.b]);
                    pp.x.min(qq.x) <= mx && pp.x.max(qq.x) >= mx && (pp.y + qq.y) / 2.0 > my + 0.5
                })
            })
            .collect();
        for (cc, e) in c.iter_mut().zip(exposed) {
            cc.exposed = e;
        }
        for q in &mut p {
            q.inv = if !q.used && design.load(&q.id) == 0.0 {
                0.0
            } else {
                1.0 / q.m
            };
        }
        // windward floor edges: the leftmost / rightmost floor joint per (rounded) level > 0
        let mut lv: Vec<(i64, usize, usize)> = Vec::new(); // (level, left, right)
        let mut top_i: Option<usize> = None;
        for cc in c.iter().filter(|cc| cc.kind == PieceType::Floor) {
            for i in [cc.a, cc.b] {
                let y = round(nodes[i].y);
                if y > 0.0 {
                    let key = y as i64;
                    match lv.iter_mut().find(|e| e.0 == key) {
                        Some(e) => {
                            if nodes[i].x < nodes[e.1].x {
                                e.1 = i;
                            }
                            if nodes[i].x > nodes[e.2].x {
                                e.2 = i;
                            }
                        }
                        None => lv.push((key, i, i)),
                    }
                }
                if top_i.is_none_or(|t| nodes[i].y > nodes[t].y) {
                    top_i = Some(i);
                }
            }
        }
        // JS objects list integer keys in ascending order
        lv.sort_by_key(|e| e.0);
        let wind_l = lv
            .iter()
            .map(|e| WindJoint {
                i: e.1,
                y: e.0 as f64,
            })
            .collect();
        let wind_r = lv
            .iter()
            .map(|e| WindJoint {
                i: e.2,
                y: e.0 as f64,
            })
            .collect();
        let xs: Vec<f64> = nodes
            .iter()
            .zip(&p)
            .filter(|(_, q)| q.used)
            .map(|(n, _)| n.x)
            .collect();
        let width = if xs.is_empty() {
            8.0
        } else {
            let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
            8f64.max(hi - lo)
        };
        let mut x0 = 2.0;
        let v = setup
            .traffic
            .iter()
            .map(|&k| {
                let s = k.spec();
                let o = Vehicle {
                    kind: k,
                    m: s.m,
                    len: s.len,
                    ht: s.ht,
                    d: s.d,
                    speed: s.v,
                    x: x0,
                    y: 0.0,
                    ang: 0.0,
                    state: VehicleState::Drive,
                    vx: 0.0,
                    vy: 0.0,
                    ax: [None, None],
                    laps: 0,
                };
                x0 -= s.len + 12.0;
                o
            })
            .collect();
        Sim {
            p,
            c,
            v,
            setup: setup.clone(),
            cond: *cond,
            live,
            t: 0.0,
            sparks: vec![],
            splashes: vec![],
            first_break: None,
            first_found: None,
            end_at: None,
            wind_l,
            wind_r,
            width,
            top_i,
            top_y0: top_i.map_or(0.0, |i| nodes[i].y),
            sway: 0.0,
            peak_a: 0.0,
            a_s: 0.0,
            last_vx: 0.0,
            env: 0.0,
            wind_now: 0.0,
            snow_now: if live { 0.0 } else { cond.snow },
            quake_start: if cond.quake > 0.0 { Some(3.0) } else { None },
            last_quake: cond.quake,
            sandbox: setup.sandbox,
            drag: 1.0,
            shedding: 1.0,
            fire_start: None,
        }
    }

    /// Where a point on the road is (`support(S, x, yHint)`): the nearest unbroken Road piece
    /// under x within 1.3 m of the hint height, or a bank (river scenes).
    pub fn support(&self, x: f64, y_hint: f64) -> Option<Axle> {
        let mut best: Option<Axle> = None;
        for (ci, c) in self.c.iter().enumerate() {
            if c.broken || !c.road {
                continue;
            }
            let (a, b) = (&self.p[c.a], &self.p[c.b]);
            let lo = a.x.min(b.x);
            let hi = a.x.max(b.x);
            if x < lo - 0.05 || x > hi + 0.05 || hi - lo < 0.2 {
                continue;
            }
            let f = 0f64.max(1f64.min((x - a.x) / (b.x - a.x)));
            let y = a.y + (b.y - a.y) * f;
            let dist = (y - y_hint).abs();
            if dist > 1.3 {
                continue;
            }
            if best.is_none_or(|bb| dist < bb.dist) {
                best = Some(Axle {
                    c: Some(ci),
                    f,
                    y,
                    dist,
                });
            }
        }
        if (x <= 8.0 || x >= 48.0)
            && y_hint.abs() < 1.3
            && best.is_none_or(|bb| y_hint.abs() < bb.dist)
        {
            return Some(Axle {
                c: None,
                f: 0.0,
                y: 0.0,
                dist: y_hint.abs(),
            });
        }
        best
    }

    /// One step of `h` seconds with velocity damping `damp` (`step(S, h, damp, live)`).
    pub fn step(&mut self, h: f64, damp: f64) {
        let live = self.live;
        let cond = self.cond;
        let g = G * 1f64.min(self.t / if live { 0.8 } else { 0.3 });
        for p in &mut self.p {
            p.fx = 0.0;
            p.fy = 0.0;
        }
        // weather changes gradually, never in a jump
        if live {
            let w_t = if self.t < 1.0 { 0.0 } else { cond.wind };
            self.wind_now += (w_t - self.wind_now) * 1f64.min(h * 0.5);
            let sd = cond.snow - self.snow_now;
            self.snow_now += js_sign(sd) * sd.abs().min(90.0 * h);
            if cond.quake != self.last_quake {
                if cond.quake > 0.0 && self.quake_start.is_none_or(|q| self.t - q > 11.0) {
                    self.quake_start = Some(self.t + 0.5);
                }
                self.last_quake = cond.quake;
            }
            for v in &self.v {
                if v.state != VehicleState::Drive {
                    continue;
                }
                for s in v.ax.iter().flatten() {
                    if let Some(ci) = s.c {
                        let w = v.m * 9.81 / 2.0;
                        let (a, b) = (self.c[ci].a, self.c[ci].b);
                        self.p[a].fy -= w * (1.0 - s.f);
                        self.p[b].fy -= w * s.f;
                    }
                }
            }
        }
        if self.snow_now > 0.0 {
            for c in &self.c {
                if c.broken || !c.exposed {
                    continue;
                }
                let w = self.snow_now * 9.81 * c.len * 1f64.min(self.t / 0.3) / 2.0;
                self.p[c.a].fy -= w;
                self.p[c.b].fy -= w;
            }
        }
        if live && self.wind_now > 0.5 {
            let w = self.wind_now;
            let dir = if cond.dir == 0.0 || cond.dir.is_nan() {
                1.0
            } else {
                cond.dir
            };
            let top_y = 10f64.max(self.top_y0);
            let v_top = w * jsmath::pow02(top_y / 10.0);
            let fg = 0.12 * v_top / self.width;
            let t = self.t;
            let gust = 1.0
                + 0.3 * self.shedding * jsmath::sin(2.0 * PI * fg * t)
                + 0.15 * jsmath::sin(2.0 * PI * 0.37 * t)
                + 0.1 * jsmath::sin(2.0 * PI * 0.13 * t + 1.0);
            let list = if dir > 0.0 {
                &self.wind_l
            } else {
                &self.wind_r
            };
            for wj in list {
                let v = w * jsmath::pow02(wj.y.max(4.0) / 10.0);
                self.p[wj.i].fx += dir * 0.613 * v * v * 1.3 * 4.0 * 4.0 * gust * self.drag;
            }
            let q10 = 0.613 * w * w;
            for c in &self.c {
                if c.broken || !c.road {
                    continue;
                }
                let (ax, bx) = (self.p[c.a].x, self.p[c.b].x);
                let lift = q10
                    * 0.25
                    * 10.0
                    * c.len
                    * jsmath::sin(2.0 * PI * 0.45 * t + 0.35 * (ax + bx) / 2.0)
                    / 2.0;
                self.p[c.a].fy += lift;
                self.p[c.b].fy += lift;
            }
        }
        let mut shake = 0.0;
        self.env = 0.0;
        if live
            && cond.quake > 0.0
            && let Some(qs) = self.quake_start
        {
            let t = self.t - qs;
            self.env = if t < 0.0 {
                0.0
            } else if t < 2.0 {
                t / 2.0
            } else if t < 7.0 {
                1.0
            } else if t < 9.0 {
                (9.0 - t) / 2.0
            } else {
                0.0
            };
            shake = cond.quake * self.env * jsmath::sin(2.0 * PI * 1.1 * self.t);
        }
        for p in &mut self.p {
            if p.inv == 0.0 {
                continue;
            }
            p.vx += p.fx * p.inv * h;
            p.vy += (-g + p.fy * p.inv) * h;
            p.vx *= damp;
            p.vy *= damp;
            p.px = p.x;
            p.py = p.y;
            p.x += p.vx * h;
            p.y += p.vy * h;
        }
        let h2 = h * h;
        for ci in 0..self.c.len() {
            let c = &self.c[ci];
            if c.broken {
                continue;
            }
            let (ia, ib) = (c.a, c.b);
            let (ax, ay, ainv) = (self.p[ia].x, self.p[ia].y, self.p[ia].inv);
            let (bx, by, binv) = (self.p[ib].x, self.p[ib].y, self.p[ib].inv);
            let dx = bx - ax;
            let dy = by - ay;
            let mut len = hypot(dx, dy);
            if len == 0.0 || len.is_nan() {
                len = 1e-6;
            }
            let cv = len - c.len;
            let c = &mut self.c[ci];
            if c.t_only && cv <= 0.0 {
                c.n = 0.0;
                c.slack = true;
                continue;
            }
            c.slack = false;
            let w = ainv + binv;
            if w == 0.0 {
                c.n = 0.0;
                continue;
            }
            let at = 1.0 / (c.kk * c.stiffness * h2);
            let dl = -cv / (w + at);
            let nx = dx / len;
            let ny = dy / len;
            c.n = -dl / h2;
            let pa = &mut self.p[ia];
            pa.x -= ainv * dl * nx;
            pa.y -= ainv * dl * ny;
            let pb = &mut self.p[ib];
            pb.x += binv * dl * nx;
            pb.y += binv * dl * ny;
        }
        let env = self.env;
        let t_now = self.t;
        for p in &mut self.p {
            let Some(sup) = p.sup else { continue };
            if p.ripped || p.inv == 0.0 {
                continue;
            }
            let lf = if p.liq && env > 0.2 {
                if sup == Foundation::Pile { 0.6 } else { 0.15 }
            } else {
                1.0
            };
            let ks = p.ks * if lf < 1.0 { 0.3 } else { 1.0 };
            p.x = p.x0 + shake;
            let cv = p.y - p.ys;
            let at = 1.0 / (ks * h2);
            let dl = -cv / (p.inv + at);
            p.y += p.inv * dl;
            let r = dl / h2;
            p.r = p.r * 0.97 + r * 0.03;
            p.over_c = p.r > p.qc * lf;
            p.over_t = -p.r > p.qt * lf;
            if !live {
                continue;
            }
            if p.over_c && t_now > 0.5 {
                p.ys -= (p.r - p.qc * lf) / ks * 0.02;
                if self.first_found.is_none() && p.y0 - p.ys > 0.3 {
                    self.first_found = Some(FirstFound {
                        id: p.id.clone(),
                        mode: if lf < 1.0 {
                            FoundMode::Liquefied
                        } else {
                            FoundMode::Sank
                        },
                    });
                }
            }
            if p.over_t && t_now > 0.5 {
                p.ripped = true;
                p.sup = None;
                self.sparks.push([p.x, p.y]);
                if self.first_found.is_none() {
                    self.first_found = Some(FirstFound {
                        id: p.id.clone(),
                        mode: FoundMode::RippedOut,
                    });
                }
            }
        }
        let scene = self.setup.scene;
        for p in &mut self.p {
            if p.inv == 0.0 {
                continue;
            }
            if p.sup.is_none() {
                let gr = scene.ground(p.x);
                if p.y < gr {
                    p.y = gr;
                    p.px = p.x - (p.x - p.px) * 0.5;
                }
            }
            p.x = p.x.clamp(-60.0, 300.0);
            p.vx = (p.x - p.px) / h;
            p.vy = (p.y - p.py) / h;
            if scene == Scene::River && p.y < -8.0 {
                p.vx *= 0.97;
                p.vy *= 0.97;
            }
        }
        for c in &mut self.c {
            if c.broken {
                continue;
            }
            c.avg = c.avg * 0.96 + c.n * 0.04;
            let cap_t = c.cap_t * c.strength;
            let cap_c = if c.t_only {
                f64::INFINITY
            } else {
                (c.crush * c.strength).min(c.pcr * c.stiffness)
            };
            let over = c.avg > cap_t || -c.avg > cap_c;
            if !live {
                c.over = over;
                continue;
            }
            if over && t_now > 0.5 {
                c.broken = true;
                let (a, b) = (&self.p[c.a], &self.p[c.b]);
                self.sparks.push([(a.x + b.x) / 2.0, (a.y + b.y) / 2.0]);
                if self.first_break.is_none() {
                    let mode = c.heat_mode.unwrap_or(if c.avg > 0.0 {
                        if c.kind == PieceType::Concrete {
                            BreakMode::Cracked
                        } else {
                            BreakMode::Snapped
                        }
                    } else if c.pcr * c.stiffness < c.crush * c.strength {
                        BreakMode::Buckled
                    } else {
                        BreakMode::Crushed
                    });
                    self.first_break = Some(FirstBreak {
                        id: c.id.clone(),
                        kind: c.kind,
                        mode,
                        t: t_now,
                    });
                }
            }
        }
        if live {
            for vi in 0..self.v.len() {
                match self.v[vi].state {
                    VehicleState::Drive => {
                        if self.t < 1.0 {
                            continue;
                        }
                        let (x, y, ang, d) = {
                            let v = &mut self.v[vi];
                            v.x += v.speed * h;
                            (v.x, v.y, v.ang, v.d)
                        };
                        let r = self.support(x + d, y + d * jsmath::sin(ang));
                        let f = self.support(x - d, y - d * jsmath::sin(ang));
                        let sandbox = self.sandbox;
                        let v = &mut self.v[vi];
                        v.ax = [f, r];
                        let (Some(r), Some(f)) = (r, f) else {
                            v.state = VehicleState::Fall;
                            v.vx = v.speed;
                            v.vy = 0.0;
                            continue;
                        };
                        v.y = (r.y + f.y) / 2.0;
                        v.ang = jsmath::atan2(r.y - f.y, 2.0 * v.d);
                        if v.x > 70.0 {
                            if sandbox {
                                v.x = -20.0;
                                v.y = 0.0;
                                v.ang = 0.0;
                                v.ax = [None, None];
                                v.laps += 1;
                            } else {
                                v.state = VehicleState::Done;
                            }
                        }
                    }
                    VehicleState::Fall => {
                        let t = self.t;
                        let v = &mut self.v[vi];
                        v.vy -= 9.81 * h;
                        v.x += v.vx * h;
                        v.y += v.vy * h;
                        v.ang = (-0.9f64).max(v.ang - 0.6 * h);
                        if v.y < -8.5 {
                            v.state = VehicleState::Sunk;
                            self.splashes.push(Splash { x: v.x, t });
                            v.y = -9.2;
                        } else if v.y < scene.ground(v.x) {
                            v.state = VehicleState::Crashed;
                            v.y = scene.ground(v.x);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.t += h;
    }

    /// One running frame (the run part of `tick()`): the fire hook (every frame), 20
    /// substeps of `FRAME/20` (× 0.3 in slow motion) with damping `1 − 0.4·h`, then the top
    /// floor's sway and smoothed acceleration.
    pub fn frame(&mut self, slow: bool) {
        let dt_frame = FRAME * if slow { SLOW } else { 1.0 };
        let h = dt_frame / SUBSTEPS as f64;
        fire_hook(self, dt_frame);
        for _ in 0..SUBSTEPS {
            self.step(h, 1.0 - 0.4 * h);
        }
        if let Some(ti) = self.top_i
            && self.t > 1.2
        {
            let p = &self.p[ti];
            self.sway = self.sway.max((p.x - p.x0).abs());
            let a = (p.vx - self.last_vx) / dt_frame;
            self.last_vx = p.vx;
            self.a_s = self.a_s * 0.6 + a.abs() * 0.4;
            if self.t > 2.0 {
                self.peak_a = self.peak_a.max(self.a_s);
            }
        }
    }

    /// Whether the run is over (`tick()`'s `done`): river — 1.2 s after no vehicle is driving
    /// or falling, or at 45 s; open ground — after 18 s; the Sandbox never ends.
    pub fn check_done(&mut self) -> bool {
        if self.sandbox {
            return false;
        }
        if self.setup.scene == Scene::River {
            let left = self
                .v
                .iter()
                .filter(|v| matches!(v.state, VehicleState::Drive | VehicleState::Fall))
                .count();
            if left == 0 && self.end_at.is_none() {
                self.end_at = Some(self.t + RIVER_TAIL_SECONDS);
            }
            self.end_at.is_some_and(|e| self.t > e) || self.t > RIVER_MAX_SECONDS
        } else {
            self.t > TEST_SECONDS
        }
    }

    /// Peak top-floor acceleration, milli-g.
    pub fn milli_g(&self) -> f64 {
        self.peak_a / 9.81 * 1000.0
    }

    /// Vehicles across.
    pub fn crossed(&self) -> usize {
        self.v
            .iter()
            .filter(|v| v.state == VehicleState::Done)
            .count()
    }

    /// Sandbox crossings so far.
    pub fn laps(&self) -> u32 {
        self.v.iter().map(|v| v.laps).sum()
    }

    /// "Light it now": a fire that is set but not burning yet lights 0.2 s from now
    /// ([`super::fire::light_now`]).
    pub fn light_fire(&mut self) {
        super::fire::light_now(self);
    }
}

/// A piece's capacities and stiffness at length `len` (as `makeSim` computes them; shared with
/// the sample drawer, SPEC LAB-1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capacity {
    /// Tension capacity A·f_t, N.
    pub cap_t: f64,
    /// A·f_c, N (∞ for tension-only).
    pub crush: f64,
    /// Euler load π²·E·k·A²/L², N.
    pub pcr: f64,
    /// Compression capacity min(A·f_c, P_cr), N (∞ for tension-only).
    pub cap_c: f64,
    /// Buckles before it crushes.
    pub buckles: bool,
    /// Axial stiffness E/15·A/L, N/m.
    pub kk: f64,
}

/// A piece's capacities at length `len`, m.
pub fn capacity(kind: PieceType, len: f64) -> Capacity {
    let t = kind.spec();
    let i = t.k * t.area * t.area;
    let pcr = PI * PI * t.e * i / (len * len);
    let crush = if t.t_only {
        f64::INFINITY
    } else {
        t.area * t.f_c
    };
    Capacity {
        cap_t: t.area * t.f_t,
        crush,
        pcr,
        cap_c: if t.t_only {
            f64::INFINITY
        } else {
            (t.area * t.f_c).min(pcr)
        },
        buckles: !t.t_only && pcr < t.area * t.f_c,
        kk: (t.e / 15.0) * t.area / len,
    }
}

/// `Math.sign` (±0 stay ±0, NaN stays NaN).
fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// The fire hook: heats pieces and sets their strength / stiffness factors (agent `lab`,
/// [`super::fire::update`]; it returns at once while Fire is off and nothing is hot).
fn fire_hook(sim: &mut Sim, dt: f64) {
    super::fire::update(sim, dt);
}
