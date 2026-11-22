//! `sp_engine::tower` over its whole input range (ENG-3, ENG-4): no panics, finite results,
//! the physics moves the right way, and it's fast enough to run every frame.

use sp_engine::tower::{self, Shape, Status, System, TowerInput};

fn all_inputs() -> impl Iterator<Item = TowerInput> {
    let floors = [
        0,
        1,
        4,
        5,
        6,
        20,
        47,
        60,
        99,
        100,
        119,
        120,
        121,
        1000,
        u32::MAX,
    ];
    let winds = [
        f64::NAN,
        f64::NEG_INFINITY,
        -10.0,
        0.0,
        5.0,
        17.5,
        30.0,
        50.0,
        51.0,
        f64::INFINITY,
    ];
    floors.into_iter().flat_map(move |floors| {
        winds.into_iter().flat_map(move |v10| {
            System::ALL.into_iter().flat_map(move |system| {
                Shape::ALL.into_iter().flat_map(move |shape| {
                    [false, true].map(|damper| TowerInput {
                        floors,
                        system,
                        shape,
                        v10,
                        damper,
                    })
                })
            })
        })
    })
}

#[test]
fn nothing_panics_and_everything_is_finite() {
    let mut n = 0;
    for input in all_inputs() {
        let r = tower::calc(&input);
        for v in [
            r.height,
            r.base_shear,
            r.overturning,
            r.delta,
            r.f1,
            r.mg,
            r.v_top,
            r.fs,
            r.ratio,
            r.near,
            r.cost,
            r.dyn_amp,
            r.period(),
            r.drift_ratio(),
        ] {
            assert!(v.is_finite() && v >= 0.0, "{input:?} → {r:?}");
        }
        let s = input.sanitized();
        assert!((5..=120).contains(&s.floors) && (5.0..=50.0).contains(&s.v10));
        for t in tower::tiles(&r) {
            assert!(!t.value.contains("NaN") && !t.value.contains("Infinity"));
        }
        let _ = Status::of(&r);
        assert!(!tower::explain(&input, &r).is_empty());
        assert!(tower::compare(&input).iter().all(|b| b.width.is_finite()));
        let _ = tower::goals(&input, &r);
        let _ = tower::share_line(&input, &r);
        let _ = tower::stage_label(&input, &r);
        let p = tower::sway_params(&input, &r, 1);
        assert!(p.f1 > 0.0 && p.modal_mass > 0.0);
        n += 1;
    }
    assert!(n > 4000);
}

#[test]
fn the_physics_moves_the_right_way() {
    let base = TowerInput::default();
    let at = |f: &dyn Fn(TowerInput) -> TowerInput| tower::calc(&f(base));
    let r0 = tower::calc(&base);
    // Taller: more sway, lower frequency, more base shear and overturning.
    let tall = at(&|i| TowerInput { floors: 90, ..i });
    assert!(tall.delta > r0.delta && tall.f1 < r0.f1 && tall.overturning > r0.overturning);
    // Sway grows roughly with H⁴ (the explain sentence): doubling floors ≈ ×16+ (taller wind).
    let half = at(&|i| TowerInput { floors: 30, ..i });
    let growth = r0.delta / half.delta;
    assert!(growth > 14.0 && growth < 24.0, "{growth}");
    // Stiffer systems sway less, in tile order.
    let deltas: Vec<f64> = System::ALL
        .iter()
        .map(|&system| at(&|i| TowerInput { system, ..i }).delta)
        .collect();
    assert!(deltas.windows(2).all(|w| w[1] < w[0]), "{deltas:?}");
    // More wind: more of everything, ∝ V².
    let windy = at(&|i| TowerInput {
        v10: 60.0_f64.min(50.0),
        ..i
    });
    assert!((windy.base_shear / r0.base_shear - (50.0_f64 / 30.0).powi(2)).abs() < 1e-9);
    // The damper cuts acceleration, not the static lean.
    let damped = at(&|i| TowerInput { damper: true, ..i });
    assert!(damped.mg < r0.mg && damped.delta == r0.delta && damped.cost == r0.cost + 6.0);
    // The tapered shape catches less wind.
    let tapered = at(&|i| TowerInput {
        shape: Shape::Tapered,
        ..i
    });
    assert!(tapered.base_shear < r0.base_shear);
}

#[test]
fn dangerously_flexible_and_comfortable_states_exist() {
    let flex = TowerInput {
        floors: 120,
        v10: 50.0,
        ..TowerInput::default()
    };
    assert_eq!(Status::of(&tower::calc(&flex)), Status::DangerouslyFlexible);
    let calm = TowerInput {
        floors: 20,
        v10: 10.0,
        system: System::Tube,
        ..TowerInput::default()
    };
    assert_eq!(Status::of(&tower::calc(&calm)), Status::Comfortable);
}

#[test]
fn stage_helpers() {
    assert_eq!(tower::mode_shape(0.0), 0.0);
    assert_eq!(tower::mode_shape(1.0), 1.0);
    assert!((tower::mode_shape(0.5) - 0.3125).abs() < 1e-15);
    let p = tower::floor_plan(Shape::Tapered, 59, 60);
    assert!((p.w - (1.0 - 0.55 * 238.0 / 240.0)).abs() < 1e-12);
    let tw = tower::floor_plan(Shape::Twisted, 30, 60);
    assert!(tw.front < tw.w && tw.offset > 0.0);
    assert!(tower::is_sky_slot(12) && tower::is_sky_slot(34) && !tower::is_sky_slot(15));
    assert!(tower::is_outrigger_floor(20, 60) && tower::is_outrigger_floor(40, 60));
    assert!(
        (tower::arrow_len(30.0, 230.0)
            - 300.0_f64.min(30.0_f64.powi(2) * 23.0_f64.powf(0.4) / 9.0))
        .abs()
            < 1e-9
    );
}

/// ENG-4: the screen calls `calc` five times a frame (the tower and the four-shape comparison);
/// a 120-floor tower must cost well under a millisecond for all of them.
#[test]
fn calc_is_fast() {
    let input = TowerInput {
        floors: 120,
        ..TowerInput::default()
    };
    let t0 = std::time::Instant::now();
    let mut acc = 0.0;
    for _ in 0..1000 {
        let r = tower::calc(&input);
        acc += r.mg + tower::compare(&input)[0].mg;
    }
    let per_frame = t0.elapsed().as_secs_f64() / 1000.0;
    assert!(acc.is_finite());
    assert!(per_frame < 5e-4, "{per_frame} s per frame");
}
