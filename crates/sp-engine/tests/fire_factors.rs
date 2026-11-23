//! FIRE-1 reduction factors: the published table points exactly, monotonic (never rising with
//! temperature) between them, within 0…1, flat below 20 °C; and the piece types' mapping.

use sp_engine::builder::PieceType;
use sp_engine::builder::fire::factors::{
    CONCRETE_KC, CONCRETE_T, STEEL_KE, STEEL_KY, STEEL_T, concrete_kc, concrete_kct, concrete_ke,
    steel_ke, steel_ky,
};
use sp_engine::builder::fire::{Factors, FireMaterial, factors_for, strength_factor};

#[test]
fn steel_follows_en_1993_1_2_table_3_1_at_its_points() {
    // SPEC FIRE-1: k_y 1.0 up to 400 °C, 0.78 at 500, 0.47 at 600, 0.23 at 700, 0.11 at 800,
    // 0.06 at 900.
    for (t, ky) in [
        (20.0, 1.0),
        (100.0, 1.0),
        (200.0, 1.0),
        (300.0, 1.0),
        (400.0, 1.0),
        (500.0, 0.78),
        (600.0, 0.47),
        (700.0, 0.23),
        (800.0, 0.11),
        (900.0, 0.06),
        (1000.0, 0.04),
        (1100.0, 0.02),
        (1200.0, 0.0),
    ] {
        assert_eq!(steel_ky(t), ky, "k_y at {t} °C");
    }
    for (t, ke) in [
        (20.0, 1.0),
        (100.0, 1.0),
        (200.0, 0.9),
        (300.0, 0.8),
        (400.0, 0.7),
        (500.0, 0.6),
        (600.0, 0.31),
        (700.0, 0.13),
        (800.0, 0.09),
        (900.0, 0.0675),
        (1000.0, 0.045),
        (1100.0, 0.0225),
        (1200.0, 0.0),
    ] {
        assert_eq!(steel_ke(t), ke, "k_E at {t} °C");
    }
    // The tables themselves.
    for (i, t) in STEEL_T.iter().enumerate() {
        assert_eq!(steel_ky(*t), STEEL_KY[i]);
        assert_eq!(steel_ke(*t), STEEL_KE[i]);
    }
}

#[test]
fn concrete_follows_en_1992_1_2_siliceous_at_its_points() {
    for (t, kc) in [
        (20.0, 1.0),
        (100.0, 1.0),
        (200.0, 0.95),
        (300.0, 0.85),
        (400.0, 0.75),
        (500.0, 0.6),
        (600.0, 0.45),
        (700.0, 0.3),
        (800.0, 0.15),
        (900.0, 0.08),
    ] {
        assert_eq!(concrete_kc(t), kc, "f_c,θ/f_ck at {t} °C");
    }
    for (i, t) in CONCRETE_T.iter().enumerate() {
        assert_eq!(concrete_kc(*t), CONCRETE_KC[i]);
    }
    // §3.2.2.2: tension 1.0 to 100 °C, 0 at 600 °C, linear between.
    assert_eq!(concrete_kct(100.0), 1.0);
    assert_eq!(concrete_kct(350.0), 0.5);
    assert_eq!(concrete_kct(600.0), 0.0);
    // Secant stiffness relative to 20 °C: f_c,θ · 0.0025 / ε_c1,θ.
    assert_eq!(concrete_ke(20.0), 1.0);
    assert!((concrete_ke(100.0) - 0.625).abs() < 1e-12);
    assert!((concrete_ke(600.0) - 0.45 * 0.1).abs() < 1e-12);
}

#[test]
fn wood_and_rope_burn_through() {
    let w = |t| factors_for(PieceType::Wood, t);
    assert_eq!(w(100.0), Factors::ONE);
    assert_eq!(w(200.0).strength(), 0.5);
    assert_eq!(w(300.0).strength(), 0.0);
    assert_eq!(w(300.0).stiffness, 0.0);
    let r = |t| factors_for(PieceType::Rope, t);
    assert_eq!(r(100.0), Factors::ONE);
    assert_eq!(r(175.0).strength(), 0.5);
    assert_eq!(r(250.0).strength(), 0.0);
}

#[test]
fn piece_types_map_to_their_tables() {
    assert_eq!(FireMaterial::of(PieceType::Steel), FireMaterial::Steel);
    assert_eq!(FireMaterial::of(PieceType::Cable), FireMaterial::Cable);
    for t in [PieceType::Road, PieceType::Floor, PieceType::Concrete] {
        assert_eq!(FireMaterial::of(t), FireMaterial::Concrete, "{t}");
    }
    // Cable follows the steel table (SPEC FIRE-1).
    for t in [20.0, 450.0, 650.0, 900.0] {
        assert_eq!(
            factors_for(PieceType::Cable, t),
            factors_for(PieceType::Steel, t)
        );
    }
    // The builder's one strength factor: concrete's compressive one.
    let f = factors_for(PieceType::Road, 500.0);
    assert_eq!(strength_factor(FireMaterial::Concrete, &f), 0.6);
}

#[test]
fn every_factor_is_monotonic_and_within_0_to_1() {
    for m in FireMaterial::ALL {
        let mut last = m.factors(-40.0);
        assert_eq!(last, Factors::ONE, "{m:?} below room temperature");
        let mut t = -40.0;
        while t <= 1300.0 {
            let f = m.factors(t);
            for (now, before, what) in [
                (f.strength_t, last.strength_t, "tension"),
                (f.strength_c, last.strength_c, "compression"),
                (f.stiffness, last.stiffness, "stiffness"),
            ] {
                assert!((0.0..=1.0).contains(&now), "{m:?} {what} at {t} °C: {now}");
                assert!(now <= before + 1e-15, "{m:?} {what} rises at {t} °C");
            }
            last = f;
            t += 0.5;
        }
    }
}

#[test]
fn garbage_temperatures_are_safe() {
    for m in FireMaterial::ALL {
        assert_eq!(m.factors(f64::NAN), Factors::ONE);
        assert_eq!(m.factors(f64::NEG_INFINITY), Factors::ONE);
        let hot = m.factors(f64::INFINITY);
        assert!(hot.strength() <= 0.0 + 1e-12 || m == FireMaterial::Concrete);
        assert!(hot.strength().is_finite());
    }
}
