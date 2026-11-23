//! LAB-1 sample curves: built from each piece type's builder properties, and the end of the
//! straight line is exactly the builder's capacity — checked against a real `Sim` built by the
//! core (`Sim::new`) for a piece of that type and length, heated by the same fire factors.

use sp_engine::builder::fire::{self, FireMaterial};
use sp_engine::builder::sample::curve::{DUCTILE_STEEL, N_PER_TONNE, NECK_MAX, piece_at};
use sp_engine::builder::sample::{Failure, SampleCurve, Test, capacity, tonnes};
use sp_engine::builder::{
    Conditions, Design, Foundation, Joint, LevelKey, Piece, PieceType, Scene, Setup, Sim,
};

const TEMPS: [f64; 10] = [
    -40.0, 20.0, 100.0, 250.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0,
];

/// The core's own constraint for one piece of `kind`, `len` m long (horizontal, between two
/// footings in the open-ground Sandbox), heated to `temp_c`.
fn engine_piece(kind: PieceType, len: f64, temp_c: f64) -> sp_engine::builder::Constraint {
    let j = |id: &str, x: f64| Joint {
        id: id.into(),
        x,
        y: 0.0,
        sup: Some(Foundation::Footing),
        locked: false,
    };
    let d = Design {
        nodes: vec![j("J1", 100.0), j("J2", 100.0 + len)],
        members: vec![Piece {
            id: "M1".into(),
            a: "J1".into(),
            b: "J2".into(),
            kind,
            locked: false,
        }],
        loads: vec![],
        n_n: 2,
        n_m: 1,
    };
    let cond = Conditions::for_level(&LevelKey::Sandbox.level());
    let setup = Setup::new(LevelKey::Sandbox, Scene::Flat, &cond);
    let sim = Sim::new(&d, &setup, &cond, true);
    let mut c = sim.c[0].clone();
    c.temp = temp_c;
    fire::apply(&mut c);
    c
}

#[test]
fn the_straight_line_ends_at_the_engines_capacity() {
    for kind in PieceType::ALL {
        let max = kind.spec().max_len;
        for len in [1.0, 2.0, 5.0, max] {
            for t in TEMPS {
                let c = engine_piece(kind, len, t);
                // Our piece model is the core's, field for field.
                let mine = piece_at(kind, len, t);
                assert_eq!(mine.cap_t, c.cap_t, "{kind} {len} m");
                assert_eq!(mine.crush, c.crush, "{kind} {len} m");
                assert_eq!(mine.pcr, c.pcr, "{kind} {len} m");
                assert_eq!(mine.kk, c.kk, "{kind} {len} m");
                assert_eq!(mine.strength, c.strength, "{kind} {t} °C");
                assert_eq!(mine.stiffness, c.stiffness, "{kind} {t} °C");

                let pull = SampleCurve::new(kind, Test::Pull, t, len).expect("everything pulls");
                assert_eq!(pull.capacity_n, c.cap_t_now(), "{kind} pull {t} °C");
                assert_eq!(capacity(kind, Test::Pull, len, t), Some(c.cap_t_now()));
                if pull.capacity_n > 0.0 {
                    let f = pull.points[1][1] * 1e6 * pull.area;
                    assert!(
                        (f - c.cap_t_now()).abs() <= 1e-9 * c.cap_t_now(),
                        "{kind} pull {t} °C: {f} vs {}",
                        c.cap_t_now()
                    );
                    assert_eq!(pull.limit_mpa, pull.points[1][1]);
                    // "a piece this size breaks at N t" — the inspector's N / 9810 rounding.
                    assert_eq!(
                        sp_engine::jsnum::to_fixed(tonnes(pull.capacity_n), 0),
                        sp_engine::jsnum::to_fixed(c.cap_t_now() / 9810.0, 0)
                    );
                }
                match SampleCurve::new(kind, Test::Push, t, len) {
                    None => assert!(kind.spec().t_only, "{kind} must push"),
                    Some(push) => {
                        assert!(!kind.spec().t_only);
                        assert_eq!(push.capacity_n, c.cap_c_now(), "{kind} push {len} m {t} °C");
                        assert_eq!(push.buckles, c.pcr * c.stiffness < c.crush * c.strength);
                        if push.capacity_n > 0.0 {
                            let f = push.points[1][1] * 1e6 * push.area;
                            assert!((f - c.cap_c_now()).abs() <= 1e-9 * c.cap_c_now());
                        }
                    }
                }
            }
        }
    }
    assert_eq!(N_PER_TONNE, 9810.0);
}

#[test]
fn ductile_types_yield_harden_neck_and_tear() {
    for kind in [PieceType::Steel, PieceType::Cable] {
        for t in [20.0, 500.0, 700.0] {
            let c = SampleCurve::new(kind, Test::Pull, t, 1.0).unwrap();
            assert_eq!(c.failure, Failure::Fractured);
            let p = &c.points;
            assert!(p.windows(2).all(|w| w[1][0] > w[0][0]), "{kind} {t}: {p:?}");
            // Plateau at the builder's strength, then above it, then down into necking.
            assert_eq!(p[2][1], p[1][1], "{kind} plateau");
            assert!(c.peak_mpa() > c.limit_mpa * 1.1, "{kind} hardens");
            let u = c.ultimate_strain.unwrap();
            assert_eq!(c.necking(u), 0.0);
            assert_eq!(c.necking(c.break_strain()), NECK_MAX);
            assert!(c.stress_at(c.break_strain() + 1e-9).broken);
            assert_eq!(c.stress_at(c.break_strain() + 1e-9).mpa, 0.0);
            // Linear at E (× k_E) to the strength.
            let e = kind.spec().e * c.stiffness;
            assert!((c.elastic_strain - c.limit_mpa * 1e6 / e * 100.0).abs() < 1e-12);
        }
    }
    // Steel at room temperature is v1's S275 shape, scaled to the builder's 250 MPa.
    let s = SampleCurve::new(PieceType::Steel, Test::Pull, 20.0, 1.0).unwrap();
    assert_eq!(s.points.len(), 2 + DUCTILE_STEEL.corners.len());
    assert!((s.break_strain() - 22.0).abs() < 1e-12);
    assert!((s.peak_mpa() - 250.0 * 430.0 / 275.0).abs() < 1e-9);
}

#[test]
fn brittle_types_snap_or_crack_at_their_strength() {
    for (kind, how) in [
        (PieceType::Wood, Failure::Snapped),
        (PieceType::Rope, Failure::Snapped),
        (PieceType::Concrete, Failure::Cracked),
        (PieceType::Road, Failure::Cracked),
        (PieceType::Floor, Failure::Cracked),
    ] {
        let c = SampleCurve::new(kind, Test::Pull, 20.0, 1.0).unwrap();
        assert_eq!(c.failure, how, "{kind}");
        assert_eq!(c.points.len(), 2, "{kind}: straight to failure");
        assert_eq!(c.peak_mpa(), kind.spec().f_t / 1e6);
        assert!(c.ultimate_strain.is_none());
        assert_eq!(c.necking(c.break_strain()), 0.0);
    }
}

#[test]
fn pushing_crushes_short_samples_and_buckles_long_ones() {
    // 1 m: every type that pushes crushes (or squashes); none buckles.
    for kind in PieceType::ALL.into_iter().filter(|k| !k.spec().t_only) {
        let c = SampleCurve::new(kind, Test::Push, 20.0, 1.0).unwrap();
        assert!(!c.buckles, "{kind}");
        let want = if kind.spec().ductile {
            Failure::Squashed
        } else {
            Failure::Crushed
        };
        assert_eq!(c.failure, want, "{kind}");
        assert_eq!(c.capacity_n, kind.spec().area * kind.spec().f_c);
    }
    // Concrete crushes at EN 1992-1-1's 0.35 % at room temperature.
    let conc = SampleCurve::new(PieceType::Concrete, Test::Push, 20.0, 1.0).unwrap();
    assert!((conc.break_strain() - 0.35).abs() < 1e-12);
    // A 14 m steel piece buckles at the Euler load, well under its crushing load.
    let long = SampleCurve::new(PieceType::Steel, Test::Push, 20.0, 14.0).unwrap();
    assert!(long.buckles);
    assert_eq!(long.failure, Failure::Buckled);
    assert!(long.capacity_n < 0.4 * long.crush_n);
    assert_eq!(
        sp_engine::jsnum::to_fixed(tonnes(long.capacity_n), 0),
        "55",
        "π²·200 GPa·1.5·(0.006 m²)²/(14 m)² = 544 kN"
    );
    // Rope and Cable only pull.
    assert!(SampleCurve::new(PieceType::Rope, Test::Push, 20.0, 1.0).is_none());
    assert!(SampleCurve::new(PieceType::Cable, Test::Push, 600.0, 3.0).is_none());
}

#[test]
fn heat_scales_the_curve_by_the_fire_factors() {
    let room = SampleCurve::new(PieceType::Steel, Test::Pull, 20.0, 1.0).unwrap();
    let hot = SampleCurve::new(PieceType::Steel, Test::Pull, 600.0, 1.0).unwrap();
    assert_eq!(hot.strength, 0.47);
    assert_eq!(hot.stiffness, 0.31);
    assert!((hot.capacity_n - 0.47 * room.capacity_n).abs() < 1e-6);
    assert!((hot.peak_mpa() - 0.47 * room.peak_mpa()).abs() < 1e-9);
    // Monotonic: hotter never stronger, for every type and test.
    for kind in PieceType::ALL {
        for test in [Test::Pull, Test::Push] {
            let mut last = f64::INFINITY;
            let mut t = -40.0;
            while t <= 900.0 {
                if let Some(c) = SampleCurve::new(kind, test, t, 3.0) {
                    assert!(c.capacity_n <= last + 1e-6, "{kind} {test:?} at {t} °C");
                    last = c.capacity_n;
                }
                t += 10.0;
            }
        }
    }
    // Wood and rope burn through: nothing left to test.
    for (kind, t) in [(PieceType::Wood, 300.0), (PieceType::Rope, 250.0)] {
        let c = SampleCurve::new(kind, Test::Pull, t, 1.0).unwrap();
        assert!(c.burned_through(), "{kind} at {t} °C");
        assert_eq!(c.capacity_n, 0.0);
        assert!(c.stress_at(0.0).broken);
        assert_eq!(FireMaterial::of(kind).factors(t).strength(), 0.0);
    }
}

#[test]
fn odd_input_is_safe() {
    for kind in PieceType::ALL {
        for t in [f64::NAN, f64::INFINITY, -1e9, 1e9] {
            for len in [f64::NAN, 0.0, -3.0, 1e-9, 1e6] {
                for test in [Test::Pull, Test::Push] {
                    if let Some(c) = SampleCurve::new(kind, test, t, len) {
                        assert!(c.capacity_n.is_finite() && c.capacity_n >= 0.0);
                        assert!(c.points.iter().flatten().all(|v| v.is_finite()));
                        for e in [f64::NAN, -1.0, 0.0, 1.0, 1e9] {
                            let s = c.stress_at(e);
                            assert!(s.mpa.is_finite());
                        }
                    }
                }
            }
        }
    }
}
