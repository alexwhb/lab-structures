//! TIME-1 / TIME-2: the long-term model on structures written out directly (the builder's own
//! piece and soil numbers), independent of the builder's editing code.

use sp_engine::builder::longterm::statics::{Consolidation, SJoint, SPiece, Spring, Structure};
use sp_engine::builder::longterm::{Ending, Kind, fast_forward};
use std::f64::consts::PI;

/// The builder's piece table rows we need: (A, E, k, fT, fC, mass per m, tension-only).
#[derive(Clone, Copy)]
struct T {
    a: f64,
    e: f64,
    k: f64,
    ft: f64,
    fc: f64,
    mpm: f64,
    t_only: bool,
    cracks: bool,
}
const FLOOR: T = T {
    a: 0.02,
    e: 30e9,
    k: 2.0,
    ft: 25e6,
    fc: 40e6,
    mpm: 350.0,
    t_only: false,
    cracks: false,
};
const CONCRETE: T = T {
    a: 0.06,
    e: 30e9,
    k: 1.0,
    ft: 2.5e6,
    fc: 40e6,
    mpm: 0.06 * 2400.0,
    t_only: false,
    cracks: true,
};
const STEEL: T = T {
    a: 0.006,
    e: 200e9,
    k: 1.5,
    ft: 250e6,
    fc: 250e6,
    mpm: 0.006 * 7850.0,
    t_only: false,
    cracks: false,
};

/// Soils: footing [Qc, Qt, ks], pile, and the consolidation (τ, ratio = 1 / ks_ground).
fn soil(key: &str, pile: bool) -> Spring {
    let (f, p, cons) = match key {
        "rock" => ([3e7, 3e6, 5e9], [4e7, 2e7, 8e9], None),
        "dense" => ([1.6e6, 1e5, 2e7], [6e6, 3e6, 1e8], Some((0.3, 8.0))),
        "stiff" => ([1e6, 1.5e5, 1e7], [4e6, 2.5e6, 6e7], Some((3.0, 4.0))),
        "soft" => ([2.5e5, 5e4, 1.5e6], [1.6e6, 1e6, 2e7], Some((9.0, 0.6))),
        _ => ([5e5, 5e4, 4e6], [2.5e6, 1.5e6, 3e7], Some((0.5, 1.5))),
    };
    let g = if pile { p } else { f };
    Spring {
        qc: g[0],
        qt: g[1],
        ks: g[2],
        consolidation: cons.map(|(tau, ksg)| Consolidation {
            tau,
            ratio: 1.0 / ksg,
        }),
    }
}

struct B {
    s: Structure,
    top: Vec<usize>,
}

impl B {
    fn new() -> B {
        B {
            s: Structure::default(),
            top: vec![],
        }
    }
    fn joint(&mut self, x: f64, y: f64, found: Option<Spring>) -> usize {
        let i = self.s.joints.len();
        self.s.joints.push(SJoint {
            id: format!("J{}", i + 1),
            x,
            y,
            mass: 40.0 + if found.is_some() { 800.0 } else { 0.0 },
            found,
            on_ground: false,
        });
        i
    }
    fn piece(&mut self, t: T, a: usize, b: usize) {
        let (ja, jb) = (&self.s.joints[a], &self.s.joints[b]);
        let l = (jb.x - ja.x).hypot(jb.y - ja.y);
        let pcr = PI * PI * t.e * t.k * t.a * t.a / (l * l);
        let m = t.mpm * l;
        self.s.joints[a].mass += m / 2.0;
        self.s.joints[b].mass += m / 2.0;
        let i = self.s.pieces.len();
        self.s.pieces.push(SPiece {
            id: format!("M{}", i + 1),
            a,
            b,
            rest: l,
            kk: t.e / 15.0 * t.a / l,
            cap_t: t.a * t.ft,
            cap_c: if t.t_only {
                f64::INFINITY
            } else {
                (t.a * t.fc).min(pcr)
            },
            t_only: t.t_only,
            buckles: !t.t_only && pcr < t.a * t.fc,
            cracks: t.cracks,
        });
    }
    fn crate_t(&mut self, j: usize, t: f64) {
        self.s.joints[j].mass += t * 1000.0;
    }
}

/// A Pisa-like tower: `bays` × 4 m wide, `levels` × 4 m tall, concrete walls and diagonals,
/// floors at every level, footings (or piles) under every column on `soil`.
fn tower(bays: usize, levels: usize, soil_key: &str, piles: &[bool], col: T, brace: T) -> B {
    let mut b = B::new();
    let x0 = 100.0 - bays as f64 * 2.0;
    let mut below: Vec<usize> = (0..=bays)
        .map(|i| {
            let pile = piles.get(i).copied().unwrap_or(false);
            b.joint(x0 + i as f64 * 4.0, 0.0, Some(soil(soil_key, pile)))
        })
        .collect();
    for k in 1..=levels {
        let y = k as f64 * 4.0;
        let cur: Vec<usize> = (0..=bays)
            .map(|i| b.joint(x0 + i as f64 * 4.0, y, None))
            .collect();
        for i in 0..=bays {
            b.piece(col, below[i], cur[i]);
        }
        for i in 0..bays {
            b.piece(FLOOR, cur[i], cur[i + 1]);
        }
        for i in 0..bays {
            if (k + i) % 2 == 1 {
                b.piece(brace, below[i], cur[i + 1]);
            } else {
                b.piece(brace, below[i + 1], cur[i]);
            }
        }
        below = cur;
    }
    b.top = below;
    b
}

fn run(b: &B) -> sp_engine::builder::longterm::LongTerm {
    fast_forward(&b.s, Kind::Tower { top: b.top.clone() })
}

#[test]
fn a_symmetric_tower_settles_evenly_and_does_not_tilt() {
    let b = tower(2, 8, "soft", &[], CONCRETE, CONCRETE);
    let lt = run(&b);
    assert_eq!(lt.ending, Ending::Stood);
    let last = lt.last().unwrap();
    assert!(last.tilt.abs() < 0.01, "tilt {}", last.tilt);
    assert!(
        last.max_settle > 0.1,
        "soft clay settles tens of cm: {}",
        last.max_settle
    );
}

#[test]
fn rock_settles_nothing_and_soft_clay_the_most() {
    let mut s = vec![];
    for soil_key in ["rock", "dense", "stiff", "loose", "soft"] {
        let lt = run(&tower(2, 6, soil_key, &[], CONCRETE, CONCRETE));
        s.push((soil_key, lt.last().unwrap().max_settle));
    }
    assert_eq!(s[0].1, 0.0);
    let soft = s[4].1;
    assert!(s.iter().all(|&(_, v)| v <= soft));
}

#[test]
fn a_heavy_corner_load_tips_the_tower_toward_it() {
    let mut b = tower(2, 8, "soft", &[], CONCRETE, CONCRETE);
    let tr = *b.top.last().unwrap();
    b.crate_t(tr, 20.0);
    let lt = run(&b);
    // a 20 t crate on the top-right corner overloads the right footing on soft clay: the left
    // footing is pulled out of the ground and the tower tips over to the right
    assert!(matches!(lt.ending, Ending::Toppled(_)), "{:?}", lt.ending);
    assert!(
        lt.frames.iter().all(|f| f.tilt >= 0.0),
        "it leans right, toward the crate"
    );
}

#[test]
fn piles_under_the_low_side_stop_the_lean() {
    let mut b = tower(2, 8, "soft", &[false, true, true], CONCRETE, STEEL);
    let tr = *b.top.last().unwrap();
    b.crate_t(tr, 20.0);
    let lt = run(&b);
    assert_eq!(lt.ending, Ending::Stood);
    assert!(lt.last().unwrap().tilt.abs() < 0.5, "{}", lt.result_line());
}
