//! CMF-1 (the comfort meter) and CMF-2 ("Same storm, four shapes") against the real builder
//! engine.

use sp_engine::builder::comfort::{self, Meter, Variant};
use sp_engine::builder::game::Preset;
use sp_engine::builder::{Design, Game, Outcome};
use sp_engine::tower::{Comfort, Tone};

#[test]
fn bands_are_the_v1_bands_on_the_builder_scale() {
    assert_eq!(comfort::bands(), [75.0, 150.0, 300.0]);
    assert_eq!(comfort::full_scale(), 450.0);
    let cases = [
        (0.0, Comfort::Unnoticed, "Nobody can feel it."),
        (74.9, Comfort::Unnoticed, "Nobody can feel it."),
        (
            75.0,
            Comfort::Noticed,
            "Some people on the top floors notice.",
        ),
        (
            149.0,
            Comfort::Noticed,
            "Some people on the top floors notice.",
        ),
        (
            150.0,
            Comfort::Uneasy,
            "Hanging lamps swing. People get uneasy.",
        ),
        (
            272.0,
            Comfort::Uneasy,
            "Hanging lamps swing. People get uneasy.",
        ),
        (
            300.0,
            Comfort::Seasick,
            "Seasick. Nobody would rent this floor.",
        ),
        (
            9000.0,
            Comfort::Seasick,
            "Seasick. Nobody would rent this floor.",
        ),
    ];
    for (mg, band, s) in cases {
        let m = Meter::new(mg, Some(300.0));
        assert_eq!(m.band, band, "{mg}");
        assert_eq!(m.sentence(), s);
    }
}

#[test]
fn the_meter_marks_the_value_and_the_level_limit() {
    let m = Meter::new(272.0, Some(300.0));
    assert_eq!(m.value(), "272 mg");
    assert_eq!(m.limit_text(), "limit 300 mg");
    assert!((m.marker_pct - 272.0 / 450.0 * 100.0).abs() < 1e-9);
    assert!((m.limit_pct.unwrap() - 300.0 / 450.0 * 100.0).abs() < 1e-9);
    assert!(!m.over_limit);
    assert_eq!(m.tone(), Tone::Caution);
    assert_eq!(
        m.label(),
        "Comfort 272 milli-g, limit 300: Hanging lamps swing. People get uneasy."
    );
    let over = Meter::new(120.0, Some(100.0));
    assert!(over.over_limit);
    assert_eq!(over.tone(), Tone::Alert);
    let none = Meter::new(1e9, None);
    assert_eq!(none.marker_pct, 100.0);
    assert_eq!(none.limit_pct, None);
    assert_eq!(none.limit_text(), "");
    let nan = Meter::new(f64::NAN, Some(100.0));
    assert!(nan.marker_pct.is_finite());
    let ticks = Meter::ticks();
    let names: Vec<&str> = ticks.iter().map(|t| t.0.as_str()).collect();
    assert_eq!(names, ["0", "75", "150", "300", "450+ mg"]);
    let st = Meter::stretches();
    assert_eq!(st[0].from, 0.0);
    assert_eq!(st[3].to, 450.0);
}

#[test]
fn the_variant_factors_come_from_the_v1_shapes() {
    assert_eq!(Variant::AsBuilt.factors(), (1.0, 1.0));
    assert_eq!(Variant::Tapered.factors(), (1.0, 1.0));
    let (d, s) = Variant::Twisted.factors();
    assert!((d - 1.15 / 1.3).abs() < 1e-12 && (s - 0.45).abs() < 1e-12);
    let (d, s) = Variant::Hollow.factors();
    assert!((d - 1.0 / 1.3).abs() < 1e-12 && (s - 0.55).abs() < 1e-12);
    let names: Vec<&str> = Variant::ALL.iter().map(|v| v.name()).collect();
    assert_eq!(names, ["As built", "Tapered", "Twisted", "Hollow core"]);
}

#[test]
fn tapering_draws_the_top_in_to_60_percent() {
    let g = Game::with_preset(&Preset::board("TowerStorm").unwrap());
    let d = g.design();
    let t = comfort::tapered(d);
    let width = |d: &Design, y: f64| {
        let xs: Vec<f64> = d
            .nodes
            .iter()
            .filter(|n| (n.y - y).abs() < 1e-9)
            .map(|n| n.x)
            .collect();
        xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - xs.iter().cloned().fold(f64::INFINITY, f64::min)
    };
    let top = d.top();
    assert!(
        (width(&t, 0.0) - width(d, 0.0)).abs() < 1e-9,
        "the base stays"
    );
    assert!(
        (width(&t, top) - 0.6 * width(d, top)).abs() < 1e-9,
        "the top is 60 % as wide"
    );
    assert_eq!(t.members, d.members, "same pieces");
    assert_eq!(t.loads, d.loads);
    // nothing built: unchanged
    assert_eq!(comfort::tapered(&Design::default()), Design::default());
}

#[test]
fn as_built_matches_the_live_run_and_the_comparison_is_deterministic() {
    let mut g = Game::with_preset(&Preset::board("TowerStorm").unwrap());
    let (design, setup, cond) = (g.design().clone(), g.setup(), *g.cond());
    let live = g.run_test(2000).cloned().unwrap();
    let Outcome::Tower { mg, standing, .. } = live.outcome else {
        panic!("open ground")
    };
    assert!(standing);
    let c = comfort::compare(&design, &setup, &cond);
    assert_eq!(c.bars[0].variant, Variant::AsBuilt);
    assert_eq!(c.bars[0].mg, mg, "the as-built bar is the run itself");
    assert_eq!(c, comfort::compare(&design, &setup, &cond), "deterministic");
    eprintln!(
        "{}",
        c.bars
            .iter()
            .map(|b| b.label())
            .collect::<Vec<_>>()
            .join(" | ")
    );
    eprintln!("{}", c.verdict);
    let best = c.best.unwrap();
    let best_mg = c.bars.iter().find(|b| b.variant == best).unwrap().mg;
    assert!(
        c.bars
            .iter()
            .filter(|b| b.standing)
            .all(|b| b.mg >= best_mg)
    );
    assert!(c.bars.iter().any(|b| b.width == 100.0));
    assert!(!c.verdict.is_empty());
}

#[test]
fn weird_structures_do_not_panic() {
    let g = Game::with_preset(&Preset::board("TowerStorm").unwrap());
    let (setup, cond) = (g.setup(), *g.cond());
    // nothing built
    let c = comfort::compare(&Design::default(), &setup, &cond);
    assert!(c.bars.iter().all(|b| !b.standing || b.mg.is_finite()));
    // a lone floor floating in the air
    let mut d = Design::default();
    for (i, x) in [0.0, 4.0].into_iter().enumerate() {
        d.nodes.push(sp_engine::builder::Joint {
            id: format!("J{}", i + 1),
            x: 100.0 + x,
            y: 40.0,
            sup: None,
            locked: false,
        });
    }
    d.members.push(sp_engine::builder::Piece {
        id: "M1".into(),
        a: "J1".into(),
        b: "J2".into(),
        kind: sp_engine::builder::PieceType::Floor,
        locked: false,
    });
    d.n_n = 2;
    d.n_m = 1;
    let c = comfort::compare(&d, &setup, &cond);
    assert!(!c.bars[0].standing, "it falls");
    assert_eq!(c.verdict, "None of the four shapes rides out this storm.");
    let _ = comfort::tapered(&d);
}

/// engine's review: "Compare shapes" in the Sandbox runs the same 18 s test (the Sandbox itself
/// never ends a run).
#[test]
fn compare_shapes_works_in_the_sandbox() {
    let g = Game::with_preset(&Preset::board("TowerStorm").unwrap());
    let d = g.design().clone();
    let mut sb = Game::new();
    sb.open_in_sandbox(sp_engine::builder::Scene::Flat, d.clone());
    sb.set_wind(35.0);
    let c = comfort::compare(&d, &sb.setup(), sb.cond());
    assert!(sb.setup().sandbox);
    assert!(c.bars.iter().any(|b| b.standing), "{:?}", c.bars);
    assert!(c.best.is_some());
}
