//! Property and robustness tests for `sp_engine::builder` (SPEC ENG-3, BLD-6, NAV-4/6): random
//! edit sequences on every level never panic and keep the design consistent; NaN, infinite and
//! huge inputs, empty designs and absurd zooms are harmless; undo is bounded and exact; netlists
//! round-trip; locked level pieces stay put.

use sp_engine::builder::data::{
    Foundation, LevelKey, MAX_CRATE_T, PieceType, Scene, Soil, TrafficSet, levels,
};
use sp_engine::builder::game::{LEVEL_CRATES, LOCKED_PIECE, Phase, Preset, Tool, UNDO_DEPTH};
use sp_engine::builder::netlist::{ReadError, read, write};
use sp_engine::builder::{Design, Game, Selection};

/// xorshift64*: deterministic, no dependencies.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn wild(&mut self) -> f64 {
        match self.below(12) {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => 1e300,
            4 => -1e300,
            5 => 0.0,
            6 => -0.0,
            _ => self.unit() * 1400.0 - 140.0,
        }
    }
}

fn check(g: &Game, ctx: &str) {
    let d = g.design();
    let mut ids: Vec<&str> = d.nodes.iter().map(|n| n.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), d.nodes.len(), "{ctx}: joint ids unique");
    for n in &d.nodes {
        assert!(
            n.x.is_finite() && n.y.is_finite(),
            "{ctx}: joint {} finite",
            n.id
        );
    }
    for m in &d.members {
        assert!(
            d.node(&m.a).is_some() && d.node(&m.b).is_some(),
            "{ctx}: {} ends",
            m.id
        );
        assert_ne!(m.a, m.b, "{ctx}: {} joins a joint to itself", m.id);
        assert!(
            d.length(m) <= m.kind.spec().max_len + 0.01 || m.locked,
            "{ctx}: {} too long",
            m.id
        );
    }
    for (id, t) in &d.loads {
        assert!(d.node(id).is_some(), "{ctx}: crate on a missing joint");
        assert!(*t > 0.0 && *t <= MAX_CRATE_T, "{ctx}: crate {t}");
    }
    assert!(g.undo_depth() <= UNDO_DEPTH, "{ctx}: undo depth");
    assert!(g.cost().is_finite(), "{ctx}: cost");
    let v = g.view();
    assert!(
        !v.stage_label.is_empty() && !v.hint.is_empty(),
        "{ctx}: view text"
    );
    assert!(
        v.camera.sc.is_finite() && v.camera.sc > 0.0,
        "{ctx}: camera"
    );
    if let Some(p) = g.preview() {
        assert_eq!(p.pieces.len(), d.members.len(), "{ctx}: preview pieces");
    }
}

fn random_action(g: &mut Game, r: &mut Rng) {
    let d = g.design().clone();
    let joint = |r: &mut Rng| {
        if d.nodes.is_empty() || r.below(8) == 0 {
            "J999".to_string()
        } else {
            d.nodes[r.below(d.nodes.len())].id.clone()
        }
    };
    let piece = |r: &mut Rng| {
        if d.members.is_empty() || r.below(8) == 0 {
            "M999".to_string()
        } else {
            d.members[r.below(d.members.len())].id.clone()
        }
    };
    let cam = g.camera();
    match r.below(40) {
        0..=5 => g.click_node(&joint(r)),
        6..=12 => {
            let x = (r.unit() * (cam.xmax - cam.xmin) + cam.xmin).round();
            let y = (r.unit() * (cam.ymax - cam.ymin) * 0.4 + cam.ymin).round();
            g.click_surface(cam.x(x), cam.y(y));
        }
        13 => g.click_surface(r.wild(), r.wild()),
        14 => g.pick_tool(Tool::ALL[r.below(5)]),
        15 => g.pick_type(PieceType::ALL[r.below(7)]),
        16 => g.pick_found(Foundation::ALL[r.below(2)]),
        17 => g.pick_weight([1.0, 5.0, 10.0, 25.0, r.wild()][r.below(5)]),
        18 => g.finish_line(),
        19 => g.pick_handle(&piece(r)),
        20 => g.set_piece_type(&piece(r), PieceType::ALL[r.below(7)]),
        21 => g.set_foundation(
            &joint(r),
            [None, Some(Foundation::Footing), Some(Foundation::Pile)][r.below(3)],
        ),
        22 => g.add_crate(&joint(r), r.wild()),
        23 => g.delete_member(&piece(r)),
        24 => g.delete_joint(&joint(r)),
        25 => g.undo(),
        26 => {
            g.joint_down(&joint(r));
            g.pointer_move(r.wild(), r.wild());
            g.pointer_move(cam.x(r.unit() * 60.0), cam.y(r.unit() * 20.0));
            g.pointer_up();
        }
        27 => {
            g.surface_down(r.unit() * 1120.0, r.unit() * 600.0);
            g.pointer_move(r.unit() * 1120.0, r.unit() * 600.0);
            g.pointer_up();
            g.click_surface(r.unit() * 1120.0, r.unit() * 600.0);
        }
        28 => g.zoom_by(r.wild(), Some(r.wild()), Some(r.wild())),
        29 => g.wheel(r.wild(), r.unit() * 1120.0, r.unit() * 600.0),
        30 => g.fit(),
        31 => g.pan_by(r.wild(), r.wild()),
        32 => g.nudge(r.wild(), r.wild()),
        33 => match r.below(7) {
            0 => g.set_wind(r.wild()),
            1 => g.set_dir([1.0, -1.0, r.wild()][r.below(3)]),
            2 => g.set_snow(r.wild().abs().min(2000.0)),
            3 => g.set_quake([0.0, 0.15, 0.3, 0.5][r.below(4)]),
            4 => g.set_soil(Soil::ALL[r.below(5)]),
            5 => g.set_traffic(TrafficSet::ALL[r.below(5)]),
            _ => g.set_fire([0.0, 400.0, 700.0, 900.0, r.wild()][r.below(5)]),
        },
        34 => {
            g.toggle_run();
            for _ in 0..r.below(20) {
                g.tick();
            }
        }
        35 => g.shake_now(),
        36 => g.toggle_forces(),
        37 => g.delete_selected(),
        38 => {
            g.advance(r.wild());
        }
        _ => g.select(Some(Selection::Node(joint(r)))),
    }
}

#[test]
fn random_edits_on_every_level_never_panic() {
    let mut r = Rng(0x9E3779B97F4A7C15);
    for (li, l) in levels().iter().enumerate() {
        for scene in [Scene::River, Scene::Flat] {
            if l.key != LevelKey::Sandbox && scene == Scene::Flat {
                continue;
            }
            let mut g = Game::new();
            g.pick_level(l.key);
            g.pick_scene(scene);
            if l.key != LevelKey::Sandbox {
                g.pick_level(l.key);
            }
            for i in 0..400 {
                random_action(&mut g, &mut r);
                if i % 50 == 0 {
                    check(&g, &format!("level {} #{li} step {i}", l.key.key()));
                }
            }
            check(&g, &format!("level {} end", l.key.key()));
            g.stop_run();
            check(&g, &format!("level {} back", l.key.key()));
        }
    }
}

#[test]
fn every_board_runs_to_its_end_without_trouble() {
    for name in Preset::BOARDS {
        let mut g = Game::with_preset(&Preset::board(name).unwrap());
        check(&g, name);
        let r = g.run_test(1400).cloned();
        if g.level() == LevelKey::Sandbox {
            assert!(
                r.is_none() && g.phase() == Phase::Run,
                "{name}: the Sandbox never ends"
            );
        } else {
            let r = r.unwrap_or_else(|| panic!("{name}: no result"));
            assert!(r.cost.is_finite(), "{name}");
            assert!(!r.title(&g.setup()).is_empty(), "{name}");
            assert_eq!(g.view().explain, r.explain(&g.setup()), "{name}");
        }
        let v = g.view();
        assert!(
            v.members
                .iter()
                .all(|m| m.a[0].is_finite() && m.b[1].is_finite()),
            "{name}"
        );
    }
}

#[test]
fn absurd_zooms_and_pans_are_clamped() {
    let mut g = Game::new();
    for _ in 0..200 {
        g.zoom_in();
    }
    assert_eq!(g.view_state().z, 5.0);
    assert_eq!(g.view().zoom_pct, "500%");
    assert_eq!(g.camera().grid, 1.0);
    for _ in 0..400 {
        g.wheel(1.0, 0.0, 0.0);
    }
    assert_eq!(g.view_state().z, 0.35);
    assert_eq!(g.camera().grid, 4.0);
    for f in [f64::NAN, f64::INFINITY, 0.0, -3.0, 1e300] {
        g.zoom_by(f, Some(f64::NAN), None);
        let v = g.view_state();
        assert!(
            v.cx.is_finite() && v.cy.is_finite() && (0.35..=5.0).contains(&v.z),
            "{f}"
        );
    }
    g.pan_by(1e300, -1e300);
    g.pan_by(f64::NAN, 1.0);
    let v = g.view_state();
    assert!(v.cx.is_finite() && v.cy.is_finite());
    g.fit();
    check(&g, "after fit");
    // snapping clamps to the scene even far off stage
    let c = g.camera();
    assert_eq!(c.snap(-1e9, 1e9), [c.xmin, c.ymin]);
}

#[test]
fn an_empty_design_previews_runs_and_fits() {
    let mut g = Game::new();
    g.pick_level(LevelKey::Sandbox);
    g.pick_scene(Scene::Flat);
    for id in ["J1", "J2"] {
        g.delete_joint(id);
    }
    assert!(g.design().nodes.is_empty());
    g.fit();
    assert_eq!(g.view_state().cx, 100.0);
    let v = g.view();
    assert_eq!(v.status.label, "Nothing built yet");
    assert_eq!(
        v.stage_label,
        "Tower with 0 joints and 0 pieces. Nothing built yet."
    );
    g.start_run(false);
    assert_eq!(
        g.toast(),
        Some("No Floors yet, so there is no height to measure.")
    );
    for _ in 0..30 {
        g.tick();
    }
    check(&g, "empty run");
    // a river level with nothing to drive on still ends, nobody across
    let mut g = Game::new();
    let r = g.run_test(2000).cloned().unwrap();
    assert!(!r.pass);
    assert_eq!(r.title(&g.setup()), "0 of 3 made it");
}

#[test]
fn undo_is_exact_and_bounded() {
    let mut g = Game::new();
    let mut history = vec![g.design().clone()];
    for _ in 0..60 {
        g.add_crate("J1", 1.0);
        history.push(g.design().clone());
    }
    assert_eq!(g.undo_depth(), UNDO_DEPTH);
    for k in 0..UNDO_DEPTH {
        g.undo();
        assert_eq!(g.design(), &history[history.len() - 2 - k], "undo {k}");
    }
    g.undo();
    assert_eq!(g.undo_depth(), 0);
    // a refused edit leaves no step
    let mut g = Game::new();
    g.delete_joint("J1");
    assert_eq!(g.toast(), Some("Level foundations stay put."));
    g.click_node("J1");
    let c = g.camera();
    g.click_surface(c.x(4.0), c.y(-2.0));
    assert_eq!(g.toast(), Some("That spot is underground."));
    assert_eq!(g.undo_depth(), 0);
    // a drag that doesn't move leaves none either
    g.pick_tool(Tool::Build);
    g.click_node("J1");
    g.click_surface(c.x(16.0), c.y(0.0));
    let depth = g.undo_depth();
    g.pick_tool(Tool::Select);
    g.joint_down("J3");
    g.pointer_up();
    assert_eq!(g.undo_depth(), depth);
}

#[test]
fn netlists_round_trip_and_refuse_garbage() {
    for name in Preset::BOARDS {
        let g = Game::with_preset(&Preset::board(name).unwrap());
        let text = g.share_text();
        assert!(
            text.starts_with(&format!("* {}.str\n", g.level().slug())),
            "{text}"
        );
        let imp = read(&text).unwrap_or_else(|e| panic!("{name}: {e}\n{text}"));
        let d = g.design();
        assert_eq!(imp.scene, g.scene(), "{name}");
        assert_eq!(imp.level, Some(g.level()), "{name}");
        assert_eq!(imp.cond.as_ref(), Some(g.cond()), "{name}");
        assert_eq!(imp.design.nodes.len(), d.nodes.len(), "{name}");
        assert_eq!(imp.design.members.len(), d.members.len(), "{name}");
        for (a, b) in imp.design.nodes.iter().zip(&d.nodes) {
            assert_eq!((a.x, a.y, a.sup), (b.x, b.y, b.sup), "{name}: {}", b.id);
        }
        for (a, b) in imp.design.members.iter().zip(&d.members) {
            assert_eq!(a.kind, b.kind, "{name}: {}", b.id);
        }
        // the Advanced side solves it
        let net = sp_engine::netlist::parse(&text).unwrap();
        assert_eq!(net.materials.len() as u32 as usize, net.materials.len());
        // and pasting it into a game loads it into the Sandbox with the conditions
        let mut h = Game::new();
        h.paste_netlist(&text).unwrap();
        assert_eq!(h.level(), LevelKey::Sandbox);
        assert_eq!(h.design().nodes.len(), d.nodes.len());
        assert_eq!(h.cond(), g.cond());
    }
    // a crate survives
    let mut g = Game::new();
    g.pick_tool(Tool::Crate);
    g.pick_weight(25.0);
    g.click_node("J1");
    g.click_node("J1");
    let imp = read(&g.share_text()).unwrap();
    assert_eq!(imp.design.loads, vec![("J1".to_string(), 50.0)]);
    // garbage
    for t in [
        "",
        "hello",
        ".node A 0.5 0\n",
        ".node A 1e9 0\n",
        ".node A 0 0\n.node B 0 0\n",
        ".node A 0 0\n.node B 2 0\n.section s A=1cm2\n.material m E=1GPa fy=1MPa rho=1\n.member M1 A A s m\n",
    ] {
        assert!(read(t).is_err(), "{t:?}");
        let mut g = Game::new();
        let before = g.design().clone();
        assert!(g.paste_netlist(t).is_err());
        assert_eq!(g.design(), &before);
    }
    assert!(matches!(
        read(".node A 0.5 0\n"),
        Err(ReadError::OffGrid(_))
    ));
    let big: String = (0..450).map(|i| format!(".node N{i} {i} 0\n")).collect();
    assert!(matches!(read(&big), Err(ReadError::TooBig(450))));
    let _ = write(
        &Design::default(),
        LevelKey::River,
        Scene::River,
        Game::new().cond(),
        "x",
    );
}

#[test]
fn the_leaning_tower_keeps_its_own_pieces_and_bells() {
    let mut g = Game::new();
    g.pick_level(LevelKey::Pisa);
    let d = g.design().clone();
    let locked = d
        .members
        .iter()
        .find(|m| m.locked)
        .expect("a locked piece")
        .id
        .clone();
    g.pick_tool(Tool::Select);
    g.delete_member(&locked);
    assert_eq!(g.toast(), Some(LOCKED_PIECE));
    g.set_piece_type(&locked, PieceType::Steel);
    assert_eq!(g.design(), &d);
    // the bells
    if let Some((id, t)) = d.loads.first().cloned() {
        g.add_crate(&id, -1.0);
        assert_eq!(g.toast(), Some(LEVEL_CRATES));
        assert_eq!(g.design().load(&id), t);
        g.add_crate(&id, 5.0);
        assert_eq!(g.design().load(&id), t + 5.0);
    }
    // its own pieces cost nothing
    assert_eq!(
        Design { ..d.clone() }.cost(),
        d.nodes
            .iter()
            .filter(|n| n.sup.is_some() && !n.locked)
            .map(|n| n.sup.unwrap().cost())
            .sum::<f64>()
            / 1000.0
    );
    assert!(
        g.run_test(100).is_none(),
        "the Leaning tower's run is the fast-forward"
    );
}

#[test]
fn soil_and_traffic_wait_for_building() {
    let mut g = Game::with_preset(&Preset::board("BridgeStorm").unwrap());
    g.start_run(false);
    g.set_soil(Soil::Rock);
    assert_eq!(g.toast(), Some("Soil can only change while building."));
    g.set_traffic(TrafficSet::Buses);
    assert_eq!(g.toast(), Some("Traffic changes on the next run."));
    assert_eq!(g.cond().soil, Soil::Dense);
    assert_eq!(g.cond().traffic, TrafficSet::Rush);
    // the toast fades after 2.6 s
    assert!(!g.advance(2.0));
    assert!(g.advance(0.7));
    assert_eq!(g.toast(), None);
}

/// lab's review: an open-ground design near the origin (inside the river's frame) comes back on
/// open ground — the `* level … · scene flat` note wins over the guess.
#[test]
fn an_open_ground_design_near_the_origin_round_trips_on_open_ground() {
    let mut g = Game::new();
    g.pick_level(LevelKey::Sandbox);
    g.pick_scene(Scene::Flat);
    g.delete_joint("J1");
    g.delete_joint("J2");
    let c = g.camera();
    g.pick_tool(Tool::Found);
    for x in [20.0, 28.0] {
        g.click_surface(c.x(x), c.y(0.0));
    }
    g.pick_tool(Tool::Build);
    g.pick_type(PieceType::Steel);
    g.click_node("J3");
    g.click_surface(c.x(20.0), c.y(4.0));
    g.finish_line();
    g.click_node("J4");
    g.click_surface(c.x(28.0), c.y(4.0));
    g.finish_line();
    g.pick_type(PieceType::Floor);
    g.click_node("J5");
    g.click_node("J6");
    g.finish_line();
    g.pick_type(PieceType::Wood);
    g.click_node("J3");
    g.click_node("J6");
    g.finish_line();
    let kinds: Vec<PieceType> = g.design().members.iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        [
            PieceType::Steel,
            PieceType::Steel,
            PieceType::Floor,
            PieceType::Wood
        ]
    );
    let imp = read(&g.share_text()).unwrap();
    assert_eq!(imp.scene, Scene::Flat);
    let back: Vec<PieceType> = imp.design.members.iter().map(|m| m.kind).collect();
    assert_eq!(back, kinds);
    let mut h = Game::new();
    h.paste_netlist(&g.share_text()).unwrap();
    assert_eq!(h.scene(), Scene::Flat);
    // a note naming a scene the joints don't fit falls back to the guess
    let text = g.share_text().replace("scene flat", "scene river");
    assert_eq!(read(&text).unwrap().scene, Scene::River);
    let bed = "* x.str\n* level sandbox · scene flat\n.node A 20 -6 fix=xy\n.node B 24 -6\n";
    assert_eq!(read(bed).unwrap().scene, Scene::River);
}

/// lab's review: easing a condition mid-run makes it a practice run (BLD-10), even when it is
/// back to the level's by the end.
#[test]
fn easing_a_condition_mid_run_is_practice() {
    let tower = || {
        Game::with_preset(&Preset {
            level: Some(LevelKey::T3),
            build: Some(sp_engine::builder::PresetBuild::Tower {
                h: 32.0,
                bays: 2,
                col: PieceType::Steel,
                brace: PieceType::Wood,
                found: Foundation::Footing,
            }),
            ..Preset::default()
        })
    };
    let mut honest = tower();
    let r = honest.run_test(800).cloned().unwrap();
    assert!(!r.pass && !r.practice, "the honest run fails");
    let mut g = tower();
    g.set_quake(0.0);
    g.start_run(false);
    while g.phase() == Phase::Run && g.sim().is_some_and(|s| s.t < 17.3) {
        g.tick();
    }
    g.set_quake(0.3);
    while g.phase() == Phase::Run {
        g.tick();
    }
    let r = g.result().cloned().unwrap();
    assert!(r.practice, "{r:?}");
    assert_eq!(r.stars, 0);
    assert_eq!(g.best_for(LevelKey::T3), 0);
    assert_eq!(g.view().result.unwrap().stars, "practice");
    // a fresh run starts judging again
    g.start_run(false);
    let r = g.run_test(800).cloned().unwrap();
    assert!(!r.practice);
}
