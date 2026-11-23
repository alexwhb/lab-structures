//! FIRE-1 in the builder's simulation: the fire heats only pieces within its reach, with a lag;
//! factors follow the tables (steel recovers when it cools, wood does not); a steel frame over a
//! Big fire softens and breaks in a deterministic run; nothing panics on odd input.
//!
//! The runs here step the simulation themselves (`fire::update` once per frame, then the core's
//! 20 substeps), so they hold whether or not the core's frame hook calls the fire yet.

use sp_engine::builder::fire::{self, AMBIENT_C, Blaze, FireLevel, FireMaterial, heat};
use sp_engine::builder::sim::{FRAME, SUBSTEPS};
use sp_engine::builder::{
    BreakMode, Conditions, Design, Foundation, Joint, LevelKey, Piece, PieceType, Scene, Setup, Sim,
};

fn joint(id: &str, x: f64, y: f64, sup: Option<Foundation>) -> Joint {
    Joint {
        id: id.into(),
        x,
        y,
        sup,
        locked: false,
    }
}

fn piece(id: &str, a: &str, b: &str, kind: PieceType) -> Piece {
    Piece {
        id: id.into(),
        a: a.into(),
        b: b.into(),
        kind,
        locked: false,
    }
}

fn sim_for(design: &Design, scene: Scene, fire_c: f64) -> Sim {
    let mut cond = Conditions::for_level(&LevelKey::Sandbox.level());
    cond.fire = fire_c;
    let setup = Setup::new(LevelKey::Sandbox, scene, &cond);
    Sim::new(design, &setup, &cond, true)
}

/// River Sandbox: a steel post on the riverbed under mid-span (in the fire) and one on the far
/// left bank (out of reach).
fn river_posts() -> Design {
    let f = Some(Foundation::Footing);
    Design {
        nodes: vec![
            joint("J1", 28.0, -12.0, f),
            joint("J2", 28.0, -4.0, None),
            joint("J3", -10.0, 0.0, f),
            joint("J4", -10.0, 6.0, None),
            joint("J5", 26.0, -12.0, f),
            joint("J6", 26.0, -4.0, None),
        ],
        members: vec![
            piece("M1", "J1", "J2", PieceType::Steel),
            piece("M2", "J3", "J4", PieceType::Steel),
            piece("M3", "J5", "J6", PieceType::Wood),
        ],
        loads: vec![],
        n_n: 6,
        n_m: 3,
    }
}

/// Runs only the fire (positions frozen) from t = 0 for `secs`, sampling each piece's
/// temperature every frame.
fn heat_only(sim: &mut Sim, secs: f64) -> Vec<Vec<f64>> {
    let mut trace = Vec::new();
    let n = (secs / FRAME).round() as usize;
    for k in 0..n {
        sim.t = k as f64 * FRAME;
        fire::update(sim, FRAME);
        trace.push(sim.c.iter().map(|c| c.temp).collect());
    }
    trace
}

/// One running frame as the core runs it, with the fire called first.
fn frame(sim: &mut Sim) {
    fire::update(sim, FRAME);
    let h = FRAME / SUBSTEPS as f64;
    for _ in 0..SUBSTEPS {
        sim.step(h, 1.0 - 0.4 * h);
    }
}

#[test]
fn only_pieces_in_reach_heat_and_they_lag_the_fire() {
    let mut sim = sim_for(&river_posts(), Scene::River, FireLevel::Big.setting());
    let trace = heat_only(&mut sim, 30.0);
    let lit = sim.fire_start.expect("a fire set before the run is lit");
    assert!(lit <= heat::FIRE_START_S + 1e-9);
    let at = |t: f64| &trace[((t / FRAME).round() as usize).min(trace.len() - 1)];

    // Nothing heats before it lights.
    for (k, temps) in trace.iter().enumerate().take((lit / FRAME) as usize) {
        assert!(temps.iter().all(|&t| t == AMBIENT_C), "frame {k}");
    }
    // The post under mid-span heats, lagging behind the fire; the far post stays cold.
    let grown = lit + heat::GROW_S;
    let blaze = Blaze::at(sim.cond.fire, sim.fire_start, grown, heat::RIVER_SOURCE);
    assert_eq!(blaze.temp_c, 700.0);
    let near_at_grown = at(grown)[0];
    assert!(near_at_grown > 300.0, "{near_at_grown}");
    assert!(near_at_grown < 650.0, "lags the fire: {near_at_grown}");
    assert!(
        trace.iter().all(|f| f[1] == AMBIENT_C),
        "the far post stays at 20 °C"
    );
    // It approaches the fire's temperature after a few time constants.
    assert!((at(29.9)[0] - 700.0).abs() < 2.0, "{}", at(29.9)[0]);
    // Steadily, never overshooting.
    for w in trace.windows(2) {
        assert!(w[1][0] >= w[0][0] - 1e-9);
        assert!(w[1][0] <= 700.0 + 1e-9);
    }
    // Wood heats more slowly than steel (lag 4 s vs 3 s) at the same exposure.
    assert!(at(lit + 2.0)[2] < at(lit + 2.0)[0]);
}

#[test]
fn factors_follow_the_tables_steel_recovers_and_wood_does_not() {
    let mut sim = sim_for(&river_posts(), Scene::River, FireLevel::Small.setting());
    heat_only(&mut sim, 20.0);
    let steel = sim.c[0].clone();
    assert!(steel.temp > 390.0, "{}", steel.temp);
    let f = FireMaterial::Steel.factors(steel.temp);
    assert_eq!(steel.strength, f.strength());
    assert_eq!(steel.stiffness, f.stiffness);
    let wood = sim.c[2].clone();
    assert_eq!(
        wood.strength, 0.0,
        "wood burned through at {} °C",
        wood.temp
    );
    assert_eq!(wood.heat_mode, Some(BreakMode::BurnedThrough));

    // Fire → Off mid-run: it goes out, pieces cool, steel gets its stiffness back, wood stays
    // charred.
    sim.cond.fire = 0.0;
    let t0 = sim.t;
    for k in 1..=900 {
        sim.t = t0 + k as f64 * FRAME;
        fire::update(&mut sim, FRAME);
    }
    assert_eq!(sim.fire_start, None);
    assert!(sim.c[0].temp < 25.0, "{}", sim.c[0].temp);
    assert_eq!(sim.c[0].stiffness, 1.0);
    assert_eq!(sim.c[0].heat_mode, None);
    assert_eq!(sim.c[2].strength, 0.0);
    assert!(fire::hottest(&sim).is_none_or(|(_, t)| t < 25.0));
}

#[test]
fn the_fire_lights_on_time_and_light_it_now() {
    // Switched on mid-run: lights 0.5 s later.
    let mut sim = sim_for(&river_posts(), Scene::River, 0.0);
    sim.t = 4.0;
    fire::update(&mut sim, FRAME);
    assert_eq!(sim.fire_start, None);
    assert!(!fire::can_light(&sim));
    sim.cond.fire = FireLevel::Inferno.setting();
    fire::update(&mut sim, FRAME);
    assert_eq!(sim.fire_start, Some(4.5));
    // "Light it now" before it has lit: 0.2 s from now.
    assert!(fire::can_light(&sim));
    fire::light_now(&mut sim);
    assert!((sim.fire_start.unwrap() - 4.2).abs() < 1e-12);
    sim.t = 5.0;
    assert!(!fire::can_light(&sim), "already burning");
    fire::light_now(&mut sim);
    assert!((sim.fire_start.unwrap() - 4.2).abs() < 1e-12);
    // The HUD reads the fire's temperature while it grows.
    fire::update(&mut sim, FRAME);
    let hud = fire::hud_value(&sim).expect("burning");
    let b = fire::blaze(&sim);
    assert_eq!(
        hud,
        format!("{} \u{b0}C", sp_engine::jsnum::to_fixed(b.temp_c, 0))
    );
    assert!(b.temp_c > 20.0 && b.temp_c < 900.0);
    // A static settle (the build-mode preview) never burns.
    let mut cold = sim_for(&river_posts(), Scene::River, 700.0);
    cold.live = false;
    cold.fire_start = None;
    cold.t = 10.0;
    fire::update(&mut cold, FRAME);
    assert!(
        cold.c
            .iter()
            .all(|c| c.temp == AMBIENT_C && c.strength == 1.0)
    );
}

#[test]
fn open_ground_fire_burns_at_the_tower_base() {
    let f = Some(Foundation::Footing);
    let d = Design {
        nodes: vec![
            joint("J1", 92.0, 0.0, f),
            joint("J2", 112.0, 0.0, f),
            joint("J3", 102.0, 8.0, None),
        ],
        members: vec![
            piece("M1", "J1", "J3", PieceType::Steel),
            piece("M2", "J2", "J3", PieceType::Steel),
        ],
        loads: vec![],
        n_n: 3,
        n_m: 2,
    };
    let sim = sim_for(&d, Scene::Flat, 700.0);
    assert_eq!(fire::source(&sim), [102.0, 0.0]);
    let river = sim_for(&d, Scene::River, 700.0);
    assert_eq!(fire::source(&river), [28.0, -8.0]);
}

#[test]
fn unused_level_anchors_dont_move_the_fire() {
    // engine's review: a tower on its own footings at x = 20…36 with the open-ground anchors
    // (footings at 92 and 108 m) left unused — the fire burns at the tower, not in between.
    let f = Some(Foundation::Footing);
    let mut d = Design {
        nodes: vec![
            joint("J1", 92.0, 0.0, f),
            joint("J2", 108.0, 0.0, f),
            joint("J3", 20.0, 0.0, f),
            joint("J4", 36.0, 0.0, f),
            joint("J5", 20.0, 8.0, None),
            joint("J6", 36.0, 8.0, None),
        ],
        members: vec![
            piece("M1", "J3", "J5", PieceType::Steel),
            piece("M2", "J4", "J6", PieceType::Steel),
            piece("M3", "J5", "J6", PieceType::Floor),
        ],
        loads: vec![],
        n_n: 6,
        n_m: 3,
    };
    let sim = sim_for(&d, Scene::Flat, FireLevel::Big.setting());
    assert_eq!(fire::source(&sim), [28.0, 0.0]);
    // Nothing built: nothing carries a piece, so it burns at the default tower's centre line.
    d.members.clear();
    let bare = sim_for(&d, Scene::Flat, FireLevel::Big.setting());
    assert_eq!(fire::source(&bare), [heat::FLAT_DEFAULT_X, 0.0]);
}

/// Open-ground Sandbox: a steel A-frame on the two footings with a 40 t crate at its apex,
/// right over the fire.
fn steel_frame() -> Design {
    let f = Some(Foundation::Footing);
    Design {
        nodes: vec![
            joint("J1", 92.0, 0.0, f),
            joint("J2", 108.0, 0.0, f),
            joint("J3", 92.0, 8.0, None),
            joint("J4", 100.0, 8.0, None),
            joint("J5", 108.0, 8.0, None),
        ],
        members: vec![
            piece("M1", "J1", "J3", PieceType::Steel),
            piece("M2", "J2", "J5", PieceType::Steel),
            piece("M3", "J3", "J4", PieceType::Steel),
            piece("M4", "J4", "J5", PieceType::Steel),
            piece("M5", "J1", "J4", PieceType::Steel),
            piece("M6", "J2", "J4", PieceType::Steel),
        ],
        loads: vec![("J4".into(), 40.0)],
        n_n: 5,
        n_m: 6,
    }
}

fn run_frame(fire_c: f64, secs: f64) -> Sim {
    let mut sim = sim_for(&steel_frame(), Scene::Flat, fire_c);
    let n = (secs / FRAME).round() as usize;
    for _ in 0..n {
        frame(&mut sim);
    }
    sim
}

#[test]
fn a_steel_frame_over_a_big_fire_softens_and_breaks() {
    // Cold, it carries the crate for the whole test.
    let cold = run_frame(0.0, 18.0);
    assert!(cold.first_break.is_none(), "{:?}", cold.first_break);
    assert!(cold.c.iter().all(|c| !c.broken));

    // Over a Big fire a steel piece softens and gives way, after the fire lit.
    let hot = run_frame(FireLevel::Big.setting(), 18.0);
    let fb = hot.first_break.clone().expect("the fire brings it down");
    assert_eq!(fb.kind, PieceType::Steel);
    assert_eq!(fb.mode, BreakMode::Softened, "{fb:?}");
    let lit = hot.fire_start.expect("lit");
    assert!(fb.t > lit, "broke at {} s, lit at {lit} s", fb.t);
    let broken = hot.c.iter().find(|c| c.id == fb.id).expect("piece");
    assert!(broken.temp > 400.0, "{} °C", broken.temp);
    assert_eq!(
        fire::tip(fire::FireBreak::Softened, PieceType::Steel),
        "Steel keeps only about half its strength at 600 \u{b0}C. Real steel frames are wrapped in fireproofing."
    );

    // Deterministic: the same run breaks the same piece at the same moment.
    let again = run_frame(FireLevel::Big.setting(), 18.0);
    assert_eq!(again.first_break, hot.first_break);
    assert_eq!(
        again.c.iter().map(|c| c.temp.to_bits()).collect::<Vec<_>>(),
        hot.c.iter().map(|c| c.temp.to_bits()).collect::<Vec<_>>()
    );
}

/// River Sandbox: a 40 m Warren truss — a Road deck in four 10 m pieces between the bank
/// footings, Steel top chords (10 m) and diagonals — with the Sandbox's looping car, bus and
/// truck. The fire burns on the water under mid-span.
pub fn steel_truss_bridge() -> Design {
    let mut nodes = vec![
        joint("J1", 8.0, 0.0, Some(Foundation::Footing)),
        joint("J2", 48.0, 0.0, Some(Foundation::Footing)),
    ];
    for (i, x) in [18.0, 28.0, 38.0].iter().enumerate() {
        nodes.push(joint(&format!("J{}", 3 + i), *x, 0.0, None));
    }
    for (i, x) in [13.0, 23.0, 33.0, 43.0].iter().enumerate() {
        nodes.push(joint(&format!("J{}", 6 + i), *x, 6.0, None));
    }
    let bottom = ["J1", "J3", "J4", "J5", "J2"];
    let top = ["J6", "J7", "J8", "J9"];
    let mut members = Vec::new();
    let mut add = |a: &str, b: &str, k: PieceType| {
        let id = format!("M{}", members.len() + 1);
        members.push(piece(&id, a, b, k));
    };
    for w in bottom.windows(2) {
        add(w[0], w[1], PieceType::Road);
    }
    for w in top.windows(2) {
        add(w[0], w[1], PieceType::Steel);
    }
    for i in 0..4 {
        add(bottom[i], top[i], PieceType::Steel);
        add(top[i], bottom[i + 1], PieceType::Steel);
    }
    let n_m = members.len() as u32;
    Design {
        nodes,
        members,
        loads: vec![],
        n_n: 9,
        n_m,
    }
}

fn run_bridge(fire_c: f64, secs: f64) -> Sim {
    let mut sim = sim_for(&steel_truss_bridge(), Scene::River, fire_c);
    for _ in 0..(secs / FRAME).round() as usize {
        frame(&mut sim);
    }
    sim
}

#[test]
fn a_steel_truss_bridge_over_a_big_fire_softens() {
    // Cold, and over a small fire (steel keeps its strength to 400 °C), it carries the traffic.
    for fire_c in [0.0, FireLevel::Small.setting()] {
        let s = run_bridge(fire_c, 30.0);
        assert!(s.first_break.is_none(), "{fire_c} °C: {:?}", s.first_break);
        assert!(s.laps() > 0, "traffic crossed");
    }
    // A Big fire under mid-span softens a middle top chord.
    let hot = run_bridge(FireLevel::Big.setting(), 30.0);
    let fb = hot.first_break.clone().expect("it softens");
    assert_eq!(
        (fb.kind, fb.mode),
        (PieceType::Steel, BreakMode::Softened),
        "{fb:?}"
    );
    assert!(fb.t > hot.fire_start.unwrap() + 1.0, "{fb:?}");
    // An Inferno gets there sooner; both are deterministic.
    let inferno = run_bridge(FireLevel::Inferno.setting(), 30.0);
    let fi = inferno.first_break.clone().expect("it softens");
    assert_eq!(fi.mode, BreakMode::Softened);
    assert!(fi.t < fb.t, "{} < {}", fi.t, fb.t);
    assert_eq!(
        run_bridge(FireLevel::Big.setting(), 30.0).first_break,
        hot.first_break
    );
}

#[test]
fn nothing_panics_on_odd_input() {
    let mut sim = sim_for(&steel_frame(), Scene::Flat, f64::NAN);
    for k in 0..60 {
        sim.t = k as f64 * FRAME;
        fire::update(&mut sim, f64::NAN);
        fire::update(&mut sim, -1.0);
        fire::update(&mut sim, f64::INFINITY);
    }
    assert!(sim.c.iter().all(|c| c.temp.is_finite()));
    let mut sim = sim_for(&steel_frame(), Scene::Flat, 1e9);
    sim.c[0].a = 999; // a dangling index is skipped
    for k in 0..200 {
        sim.t = k as f64 * FRAME;
        fire::update(&mut sim, FRAME);
    }
    assert!(sim.c.iter().all(|c| c.temp.is_finite() && c.temp <= 1200.0));
    let mut empty = sim_for(&Design::default(), Scene::River, 900.0);
    empty.t = 10.0;
    fire::update(&mut empty, FRAME);
    fire::light_now(&mut empty);
    assert!(fire::hottest(&empty).is_none());
    assert_eq!(FireLevel::from_temp(650.0), FireLevel::Small);
    assert_eq!(FireLevel::from_temp(f64::NAN), FireLevel::Off);
}
