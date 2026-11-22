//! `sp_engine::bridge`: no panics on any input (SPEC ENG-3), editing invariants, undo, the
//! truss round trip, and the performance budget (ENG-4).

use sp_engine::bridge::data::{MAX_LOAD_T, X_MAX, X_MIN, Y_MAX, Y_MIN, underground};
use sp_engine::bridge::game::UNDO_DEPTH;
use sp_engine::bridge::{
    Design, Game, Joint, LevelKey, MaterialKey, Phase, Selection, SizeKey, Support, Tool,
};
use sp_engine::{Fix, Member, NodalLoad, Node, Truss};
use std::time::Instant;

/// A tiny deterministic PRNG (xorshift64*), so failures reproduce.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn coord(&mut self) -> (i32, i32) {
        match self.below(10) {
            0 => (i32::MAX, i32::MIN),
            1 => (-7, 99),
            _ => (
                (self.below(31) as i32) * 2 + self.below(2) as i32 - 1,
                (self.below(16) as i32) * 2 - 12,
            ),
        }
    }
    fn f(&mut self) -> f64 {
        match self.below(12) {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => -1e300,
            3 => 1e300,
            _ => (self.below(60) as f64) - 25.0,
        }
    }
}

fn joint_id(g: &Game, r: &mut Rng) -> String {
    let n = &g.design().nodes;
    if n.is_empty() || r.below(8) == 0 {
        format!("J{}", r.below(40))
    } else {
        n[r.below(n.len() as u64) as usize].id.clone()
    }
}

fn member_id(g: &Game, r: &mut Rng) -> String {
    let m = &g.design().members;
    if m.is_empty() || r.below(8) == 0 {
        format!("M{}", r.below(40))
    } else {
        m[r.below(m.len() as u64) as usize].id.clone()
    }
}

fn invariants(g: &Game, ctx: &str) {
    let d = g.design();
    for (i, a) in d.nodes.iter().enumerate() {
        assert!(
            (X_MIN..=X_MAX).contains(&a.x) && (Y_MIN..=Y_MAX).contains(&a.y),
            "{ctx}: {a:?}"
        );
        assert!(!underground(a.x, a.y), "{ctx}: underground {a:?}");
        for b in &d.nodes[i + 1..] {
            assert_ne!(a.id, b.id, "{ctx}: duplicate id");
            assert!(
                a.x != b.x || a.y != b.y,
                "{ctx}: {} and {} overlap",
                a.id,
                b.id
            );
        }
        if let Some(s) = a.sup
            && !a.locked
        {
            // a player's support always sits where the level allows it
            let _ = s;
            assert!(
                g.level().level().sup.allows(a.x, a.y),
                "{ctx}: support at {a:?}"
            );
        }
    }
    for (i, m) in d.members.iter().enumerate() {
        assert!(
            d.joint(&m.a).is_some() && d.joint(&m.b).is_some(),
            "{ctx}: {m:?}"
        );
        assert_ne!(m.a, m.b);
        for o in &d.members[i + 1..] {
            assert_ne!(m.id, o.id, "{ctx}: duplicate member id");
            assert!(
                !((m.a == o.a && m.b == o.b) || (m.a == o.b && m.b == o.a)),
                "{ctx}: duplicate member"
            );
        }
    }
    for (id, t) in &d.loads {
        assert!(d.joint(id).is_some(), "{ctx}: load on missing {id}");
        assert!(*t > 0.0 && *t <= MAX_LOAD_T, "{ctx}: load {t}");
    }
    assert!(g.undo_depth() <= UNDO_DEPTH);
    let v = g.view();
    for p in &v.positions {
        assert!(
            p[0].is_finite() && p[1].is_finite(),
            "{ctx}: position {p:?}"
        );
    }
    assert!(!v.status.label.is_empty());
    assert!(v.cost_text.starts_with('$'));
}

#[test]
fn random_edits_never_panic() {
    for seed in 1..=24u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut g = Game::new();
        g.pick_level(LevelKey::ALL[(seed % 4) as usize]);
        for step in 0..900 {
            let ctx = format!("seed {seed} step {step}");
            match r.below(30) {
                0..=6 => {
                    let (x, y) = r.coord();
                    g.click_grid(x, y)
                }
                7..=11 => {
                    let id = joint_id(&g, &mut r);
                    g.click_joint(&id)
                }
                12 => g.pick_tool(Tool::ALL[r.below(6) as usize]),
                13 => {
                    let id = member_id(&g, &mut r);
                    g.click_member(&id)
                }
                14 => {
                    let id = joint_id(&g, &mut r);
                    g.joint_down(&id);
                    let (x, y) = r.coord();
                    g.pointer_move(x, y);
                    g.pointer_up();
                }
                15 => {
                    let id = joint_id(&g, &mut r);
                    let s = [None, Some(Support::Pin), Some(Support::Roller)][r.below(3) as usize];
                    g.set_support(&id, s)
                }
                16 => {
                    let id = joint_id(&g, &mut r);
                    let t = r.f();
                    g.add_load(&id, t)
                }
                17 => {
                    let id = member_id(&g, &mut r);
                    g.set_member_size(&id, SizeKey::ALL[r.below(3) as usize])
                }
                18 => g.undo(),
                19 => g.delete_selected(),
                20 => g.nudge(r.below(5) as i32 - 2, r.below(5) as i32 - 2),
                21 => g.pick_material(MaterialKey::ALL[r.below(4) as usize]),
                22 => g.pick_weight(r.next() as u32),
                23 => {
                    if g.can_starter() {
                        g.starter()
                    }
                }
                24 => g.start_or_stop(),
                25 => {
                    for _ in 0..r.below(40) {
                        g.tick()
                    }
                }
                26 => g.replay(),
                27 => g.back_to_build(),
                28 => {
                    if r.below(10) == 0 {
                        g.pick_level(LevelKey::ALL[r.below(4) as usize])
                    } else {
                        g.finish_line()
                    }
                }
                _ => {
                    let (x, y) = r.coord();
                    g.pointer_move(x, y);
                    g.advance(r.f());
                    g.clear_hover();
                }
            }
            invariants(&g, &ctx);
        }
    }
}

#[test]
fn undo_keeps_forty_steps_and_restores_exactly() {
    let mut g = Game::new();
    g.pick_level(LevelKey::Sandbox);
    g.pick_tool(Tool::Joint);
    let mut states = vec![g.design().clone()];
    for i in 0..45 {
        g.click_grid((i % 29) * 2, 2 + (i / 29) * 2);
        states.push(g.design().clone());
    }
    assert_eq!(g.undo_depth(), UNDO_DEPTH);
    for k in (5..45).rev() {
        g.undo();
        assert_eq!(g.design(), &states[k], "undo to step {k}");
    }
    g.undo(); // nothing left: no change
    assert_eq!(g.design(), &states[5]);
}

#[test]
fn nudge_follows_the_drag_rules() {
    let mut g = Game::new();
    g.pick_tool(Tool::Joint);
    g.click_grid(20, 4);
    assert_eq!(g.selection(), Some(&Selection::Joint("J4".into())));
    g.nudge(1, 0);
    assert_eq!(g.design().joint("J4").map(|j| (j.x, j.y)), Some((22, 4)));
    g.nudge(0, -10); // y −16: off the stage
    assert_eq!(g.design().joint("J4").map(|j| (j.x, j.y)), Some((22, 4)));
    // into the bank: underground
    g.nudge(-8, -3);
    assert_eq!(g.toast(), Some("That spot is underground."));
    // onto J3 (28, 0): occupied
    g.nudge(3, -2);
    assert_eq!(g.design().joint("J4").map(|j| (j.x, j.y)), Some((22, 4)));
    // level joints stay put
    g.pick_tool(Tool::Select);
    g.click_joint("J3");
    g.nudge(1, 0);
    assert_eq!(g.toast(), Some("This joint is part of the level."));
    assert_eq!(g.design().joint("J3").map(|j| (j.x, j.y)), Some((28, 0)));
}

#[test]
fn a_support_dragged_off_the_ground_drops() {
    let mut g = Game::new();
    g.pick_level(LevelKey::Sandbox);
    g.pick_tool(Tool::Select);
    g.joint_down("J2");
    g.pointer_move(52, 0);
    assert_eq!(g.design().joint("J2").unwrap().sup, Some(Support::Roller));
    g.pointer_move(52, 4);
    assert_eq!(g.design().joint("J2").unwrap().sup, None);
    g.pointer_up();
    assert!(g.dragging().is_none());
    g.undo();
    assert_eq!(g.design().joint("J2").unwrap().sup, Some(Support::Roller));
}

#[test]
fn designs_round_trip_through_a_truss() {
    let mut d = Design::starter();
    d.members[3].size = SizeKey::Thick;
    d.members[7].size = SizeKey::Thin;
    d.set_load("J3", 25.0);
    d.set_load("J8", 2.5);
    let (t, loads) = d.to_truss(MaterialKey::Aluminum, 1.0);
    let back = Design::from_truss(&t, &loads).unwrap();
    assert_eq!(back, d);
    // and the solves agree
    let a = d.analyse(MaterialKey::Aluminum, 1.0, &[]);
    let b = back.analyse(MaterialKey::Aluminum, 1.0, &[]);
    assert_eq!(a, b);
}

#[test]
fn imports_refuse_what_the_stage_cannot_show() {
    let mat = MaterialKey::Steel.spec().to_material();
    let truss = |pts: &[(f64, f64)]| Truss {
        nodes: pts
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| Node::new(format!("N{i}"), x, y, Fix::FREE))
            .collect(),
        members: vec![],
        materials: vec![mat.clone()],
    };
    assert!(Design::from_truss(&truss(&[(1.0, 0.0)]), &[]).is_err());
    assert!(Design::from_truss(&truss(&[(f64::NAN, 0.0)]), &[]).is_err());
    assert!(Design::from_truss(&truss(&[(60.0, 0.0)]), &[]).is_err());
    assert!(Design::from_truss(&truss(&[(4.0, -2.0)]), &[]).is_err());
    assert!(Design::from_truss(&truss(&[(4.0, 2.0), (4.0, 2.0)]), &[]).is_err());
    let ok = Design::from_truss(&truss(&[(4.0, 2.0), (6.0, 2.0)]), &[]).unwrap();
    assert_eq!(ok.nodes[0].id, "J1"); // renamed: N0 isn't a Bridge id
    let mut t = truss(&[(4.0, 2.0), (6.0, 2.0)]);
    t.members.push(Member::box_section("A", 0, 1, 0.0079, 0));
    t.members.push(Member::box_section("B", 1, 0, 0.0079, 0));
    assert!(Design::from_truss(&t, &[]).is_err(), "duplicate member");
    t.members.pop();
    let d = Design::from_truss(&t, &[NodalLoad::down(1, 9810.0 * 3.0)]).unwrap();
    assert_eq!(d.members[0].size, SizeKey::Medium);
    assert_eq!(d.loads, vec![("J2".to_string(), 3.0)]);
    let up = NodalLoad {
        node: 0,
        fx: 0.0,
        fy: 10.0,
    };
    assert!(Design::from_truss(&t, &[up]).is_err());
    let side = NodalLoad {
        node: 0,
        fx: 1e4,
        fy: -1.0,
    };
    assert!(Design::from_truss(&t, &[side]).is_err());
    assert!(Design::from_truss(&t, &[NodalLoad::down(7, 1.0)]).is_err());
    assert!(Design::from_truss(&t, &[NodalLoad::down(0, 1e12)]).is_err());
}

#[test]
fn huge_loads_and_empty_structures_finish() {
    // every joint of the closed starter truss carries 200 t: it breaks fast and settles
    let mut g = Game::new();
    g.pick_level(LevelKey::Sandbox);
    g.starter();
    g.pick_tool(Tool::Member);
    g.click_joint("J8");
    g.click_joint("J4");
    g.pick_tool(Tool::Weight);
    g.pick_weight(25);
    let ids: Vec<String> = g.design().nodes.iter().map(|n| n.id.clone()).collect();
    for id in &ids {
        for _ in 0..10 {
            g.click_joint(id);
        }
    }
    assert!(g.design().loads.iter().all(|(_, t)| *t == 200.0));
    assert_eq!(g.run_test(2000), Phase::Collapsed);
    assert!(g.log().iter().any(|l| l.member.is_some()));
    // an empty Sandbox: nothing to hold the supports' free directions — it folds at once
    let mut e = Game::new();
    e.pick_level(LevelKey::Sandbox);
    assert_eq!(e.run_test(500), Phase::Collapsed);
    // a level with its loaded joint unconnected refuses to start
    let mut l = Game::new();
    l.start_or_stop();
    assert_eq!(l.phase(), Phase::Build);
    assert_eq!(
        l.toast(),
        Some("Connect the loaded joint to your structure first.")
    );
}

#[test]
fn toasts_fade_after_two_point_six_seconds() {
    let mut g = Game::new();
    g.pick_tool(Tool::Erase);
    g.click_joint("J1");
    assert_eq!(g.toast(), Some("This joint is part of the level."));
    assert!(g.advance(1.3));
    assert!(g.advance(1.29));
    assert!(!g.advance(0.02));
    assert_eq!(g.toast(), None);
    g.click_joint("J1");
    assert!(g.advance(f64::NAN), "NaN doesn't age it");
}

/// A 60-joint truss across the whole stage: 30 bottom joints (y 0), 30 top (y 6).
fn sixty_joints(size: SizeKey) -> Design {
    let mut d = Design::default();
    for row in [0, 6] {
        for i in 0..30 {
            let n = d.nodes.len() + 1;
            d.nodes.push(Joint {
                id: format!("J{n}"),
                x: i * 2,
                y: row,
                sup: match (row, i) {
                    (0, 0) => Some(Support::Pin),
                    (0, 29) => Some(Support::Roller),
                    _ => None,
                },
                locked: false,
            });
        }
    }
    let mut add = |a: usize, b: usize| {
        let n = d.members.len() + 1;
        d.members.push(sp_engine::bridge::Bar {
            id: format!("M{n}"),
            a: format!("J{}", a + 1),
            b: format!("J{}", b + 1),
            size,
        });
    };
    for i in 0..29 {
        add(i, i + 1);
        add(30 + i, 31 + i);
        add(i, 30 + i + 1);
    }
    for i in 0..30 {
        add(i, 30 + i);
    }
    d.n_n = 60;
    d.n_m = d.members.len() as u32;
    for i in [10usize, 15, 20] {
        d.set_load(&format!("J{}", i + 1), 50.0);
    }
    d
}

/// ENG-4: a 60-joint solve < 1 ms and one Test tick (≤ 30 solves) < 8 ms at release. The test
/// profile (opt-level 2) is held to the same bound with a 2× margin for noisy machines;
/// unoptimised builds only report.
#[test]
fn performance_budget() {
    let strict = !cfg!(debug_assertions);
    let d = sixty_joints(SizeKey::Medium);
    assert_eq!(d.nodes.len(), 60);
    let a = d.analyse(MaterialKey::Steel, 1.0, &[]);
    assert!(a.ok, "the 60-joint truss is stable");
    let n = 200;
    let t0 = Instant::now();
    for _ in 0..n {
        std::hint::black_box(d.analyse(MaterialKey::Steel, 1.0, &[]));
    }
    let per_solve = t0.elapsed().as_secs_f64() / n as f64;
    eprintln!("60-joint solve: {:.3} ms", per_solve * 1e3);

    // one Test tick that breaks the full 30 members: thin timber under 150 t
    let mut g = Game::new();
    g.open_in_sandbox(sixty_joints(SizeKey::Thin), MaterialKey::Timber);
    g.start_or_stop();
    let mut worst = 0.0f64;
    let mut most = 0;
    while g.phase() == Phase::Testing {
        let before = g.broken().len();
        let t = Instant::now();
        g.tick();
        worst = worst.max(t.elapsed().as_secs_f64());
        most = most.max(g.broken().len() - before);
    }
    eprintln!(
        "worst Test tick: {:.3} ms ({most} breaks in one tick)",
        worst * 1e3
    );
    // the worst a tick can do: 30 solves, each with one more member broken
    let t0 = Instant::now();
    let mut broken = Vec::new();
    for k in 0..30 {
        std::hint::black_box(d.analyse(MaterialKey::Steel, 0.5, &broken));
        broken.push(d.members[k * 3].id.clone());
    }
    let thirty = t0.elapsed().as_secs_f64();
    eprintln!("30 solves (a full tick): {:.3} ms", thirty * 1e3);
    if strict {
        assert!(per_solve < 2e-3, "60-joint solve {per_solve:.6} s");
        assert!(worst < 16e-3, "Test tick {worst:.6} s");
        assert!(thirty < 16e-3, "30 solves {thirty:.6} s");
    }
}
