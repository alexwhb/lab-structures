//! `sp_engine::builder` speed (SPEC BLD-15, ENG-4): a live-preview solve of a 60-joint design
//! under 20 ms and a running frame (20 substeps) of a 60-joint structure carrying 8 vehicles
//! under 4 ms, in release. Prints `PERF` lines for `scripts/test-all.sh` (release gates); in
//! the debug-assertion test profile the limits are 5× looser so `cargo test` stays honest
//! without flaking.

use sp_engine::builder::data::{Foundation, LevelKey, PieceType, Scene, TrafficSet};
use sp_engine::builder::design::{Design, Joint, Piece};
use sp_engine::builder::game::{Preset, PresetBuild};
use sp_engine::builder::setup::{Conditions, Setup};
use sp_engine::builder::{Game, Sim, preview};
use std::time::Instant;

fn slack() -> f64 {
    if cfg!(debug_assertions) { 5.0 } else { 1.0 }
}

/// A 60-joint tower (4 columns × 15 storeys) on piles, from the preset builder.
fn tower60() -> Game {
    Game::with_preset(&Preset {
        level: Some(LevelKey::T2),
        build: Some(PresetBuild::Tower {
            h: 56.0,
            bays: 3,
            col: PieceType::Steel,
            brace: PieceType::Wood,
            found: Foundation::Pile,
        }),
        ..Preset::default()
    })
}

/// A 60-joint river truss: a Road deck J1–J2 in 2 m panels (21 joints), two chords above it
/// at 4 m (19) and 8 m (20, out to the banks), triangulated.
fn bridge60() -> Design {
    let mut d = Design::default();
    let add = |d: &mut Design, x: f64, y: f64, sup: Option<Foundation>| {
        d.n_n += 1;
        let id = format!("J{}", d.n_n);
        d.nodes.push(Joint {
            id: id.clone(),
            x,
            y,
            sup,
            locked: false,
        });
        id
    };
    let mut bot = vec![];
    for i in 0..=20 {
        let x = 8.0 + 2.0 * i as f64;
        let sup = (i == 0 || i == 20).then_some(Foundation::Pile);
        bot.push(add(&mut d, x, 0.0, sup));
    }
    let mid: Vec<String> = (1..20)
        .map(|i| add(&mut d, 8.0 + 2.0 * i as f64, 4.0, None))
        .collect();
    let top: Vec<String> = (0..20)
        .map(|i| add(&mut d, 9.0 + 2.0 * i as f64, 8.0, None))
        .collect();
    let mem = |d: &mut Design, a: &str, b: &str, kind: PieceType| {
        d.n_m += 1;
        d.members.push(Piece {
            id: format!("M{}", d.n_m),
            a: a.into(),
            b: b.into(),
            kind,
            locked: false,
        });
    };
    for w in bot.windows(2) {
        mem(&mut d, &w[0], &w[1], PieceType::Road);
    }
    for (i, m) in mid.iter().enumerate() {
        mem(&mut d, &bot[i], m, PieceType::Steel);
        mem(&mut d, m, &bot[i + 1], PieceType::Steel);
        mem(&mut d, m, &bot[i + 2], PieceType::Steel);
        mem(&mut d, m, &top[i], PieceType::Steel);
        mem(&mut d, m, &top[i + 1], PieceType::Steel);
    }
    for w in mid.windows(2) {
        mem(&mut d, &w[0], &w[1], PieceType::Steel);
    }
    for w in top.windows(2) {
        mem(&mut d, &w[0], &w[1], PieceType::Steel);
    }
    mem(&mut d, &bot[0], &top[0], PieceType::Steel);
    mem(&mut d, &bot[20], &top[19], PieceType::Steel);
    d
}

#[test]
fn preview_of_60_joints_is_fast() {
    let g = tower60();
    assert_eq!(g.design().nodes.len(), 60);
    let setup = g.setup();
    let cond = *g.cond();
    let n = 20;
    let t = Instant::now();
    let mut moved = 0.0;
    for _ in 0..n {
        moved += std::hint::black_box(preview(g.design(), &setup, &cond)).moved;
    }
    let ms = t.elapsed().as_secs_f64() * 1e3 / n as f64;
    assert!(moved.is_finite());
    println!("PERF builder preview, 60 joints: {ms:.3} ms (limit 20 ms)");
    assert!(ms < 20.0 * slack(), "a 60-joint preview took {ms:.3} ms");
}

#[test]
fn running_frame_of_60_joints_and_8_vehicles_is_fast() {
    let d = bridge60();
    assert_eq!(d.nodes.len(), 60);
    let cond = Conditions {
        traffic: TrafficSet::Rush,
        wind: 25.0,
        snow: 150.0,
        ..Conditions::for_level(&LevelKey::Sandbox.level())
    };
    let setup = Setup::new(LevelKey::Sandbox, Scene::River, &cond);
    let mut sim = Sim::new(&d, &setup, &cond, true);
    assert_eq!(sim.v.len(), 8);
    // let the traffic roll onto the deck first
    for _ in 0..90 {
        sim.frame(false);
    }
    let frames = 300;
    let t = Instant::now();
    for _ in 0..frames {
        sim.frame(false);
    }
    let ms = t.elapsed().as_secs_f64() * 1e3 / frames as f64;
    let on_deck = sim.v.iter().filter(|v| v.x > 8.0 && v.x < 48.0).count();
    assert!(sim.p.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
    println!(
        "PERF builder running frame, 60 joints + 8 vehicles ({on_deck} on the deck): {ms:.3} ms (limit 4 ms)"
    );
    assert!(ms < 4.0 * slack(), "a running frame took {ms:.3} ms");
}

#[test]
fn view_of_a_running_storm_tower_is_cheap() {
    let mut g = Game::with_preset(&Preset::board("TowerStorm").unwrap());
    g.start_run(false);
    for _ in 0..60 {
        g.tick();
    }
    let n = 200;
    let t = Instant::now();
    let mut k = 0;
    for _ in 0..n {
        k += std::hint::black_box(g.view()).members.len();
    }
    let ms = t.elapsed().as_secs_f64() * 1e3 / n as f64;
    assert!(k > 0);
    println!("PERF builder view() of a running 85-joint tower: {ms:.3} ms (limit 2 ms)");
    assert!(ms < 2.0 * slack(), "view() took {ms:.3} ms");
}
