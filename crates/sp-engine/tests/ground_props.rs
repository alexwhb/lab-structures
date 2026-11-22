//! `sp_engine::ground` over its whole input range (ENG-3, ENG-4): no panics, finite results,
//! the physics moves the right way.

use sp_engine::ground::{self, Foundation, FromTower, GroundInput, Soil, Status};

fn all_inputs() -> impl Iterator<Item = GroundInput> {
    let floors = [0, 1, 2, 6, 40, 80, 81, u32::MAX];
    let piles = [f64::NAN, -5.0, 0.0, 8.0, 29.0, 30.0, 40.0, 1e9];
    let years = [f64::NAN, -1.0, 0.0, 0.25, 10.0, 30.0, 31.0, f64::INFINITY];
    floors.into_iter().flat_map(move |floors| {
        piles.into_iter().flat_map(move |pile_len| {
            years.into_iter().flat_map(move |years| {
                Soil::ALL.into_iter().flat_map(move |soil| {
                    Foundation::ALL.into_iter().flat_map(move |foundation| {
                        [(false, false), (true, false), (false, true), (true, true)].map(
                            |(quake, pisa_tried)| GroundInput {
                                soil,
                                foundation,
                                floors,
                                pile_len,
                                quake,
                                years,
                                pisa_tried,
                            },
                        )
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
        let r = ground::calc(&input);
        for v in [
            r.load,
            r.pressure,
            r.capacity,
            r.fs,
            r.settle_final,
            r.tilt_final,
            r.settle,
            r.tilt,
            r.tau,
        ] {
            assert!(v.is_finite() && v >= 0.0, "{input:?} → {r:?}");
        }
        assert!(r.settle <= r.settle_final + 1e-9 && r.tilt <= r.tilt_final + 1e-9);
        for t in ground::tiles(&input, &r) {
            assert!(!t.value.contains("NaN") && !t.value.contains("Infinity"));
        }
        let _ = Status::of(&r);
        let _ = ground::goals(&input);
        let _ = ground::share_line(&input, &r);
        assert!((0.0..=99.0).contains(&ground::fs_pos(&r)));
        n += 1;
    }
    assert!(n > 10_000);
}

#[test]
fn the_physics_moves_the_right_way() {
    let base = GroundInput::default();
    // Settlement and tilt grow with the years and approach the final values.
    let mut last = -1.0;
    for y in 0..=30 {
        let r = ground::calc(&GroundInput {
            years: y as f64,
            ..base
        });
        assert!(r.settle > last || y == 0);
        last = r.settle;
    }
    assert_eq!(
        ground::calc(&GroundInput { years: 0.0, ..base }).settle,
        0.0
    );
    // More floors: lower safety factor.
    let heavy = ground::calc(&GroundInput { floors: 40, ..base });
    assert!(heavy.fs < ground::calc(&base).fs);
    // Longer friction piles grip more; reaching rock adds end bearing.
    let piles = |l: f64| {
        ground::calc(&GroundInput {
            foundation: Foundation::Piles,
            pile_len: l,
            ..base
        })
    };
    assert!(piles(20.0).capacity > piles(10.0).capacity);
    assert!(piles(30.0).capacity - piles(29.0).capacity > 900.0);
    assert!(piles(30.0).to_rock && !piles(29.0).to_rock);
    // Footings concentrate the load.
    let foot = ground::calc(&GroundInput {
        foundation: Foundation::Footing,
        ..base
    });
    assert!((foot.pressure - 72.0 / 0.35).abs() < 1e-12);
    // Liquefaction needs the quake and the loose sand, and rock ignores it.
    let liq = |quake, soil, foundation| {
        ground::calc(&GroundInput {
            quake,
            soil,
            foundation,
            ..base
        })
        .liquefied
    };
    assert!(liq(true, Soil::Loose, Foundation::Mat));
    assert!(!liq(false, Soil::Loose, Foundation::Mat));
    assert!(!liq(true, Soil::Dense, Foundation::Mat));
    assert!(!liq(true, Soil::Loose, Foundation::Caisson));
}

#[test]
fn every_status_is_reachable() {
    let at = |soil, foundation, floors, quake| {
        Status::of(&ground::calc(&GroundInput {
            soil,
            foundation,
            floors,
            quake,
            years: 30.0,
            ..GroundInput::default()
        }))
    };
    assert_eq!(at(Soil::Dense, Foundation::Mat, 6, false), Status::Solid);
    assert_eq!(
        at(Soil::Stiff, Foundation::Mat, 12, false),
        Status::Marginal
    );
    assert_eq!(at(Soil::Soft, Foundation::Mat, 4, false), Status::Leaning);
    assert_eq!(
        at(Soil::Soft, Foundation::Mat, 6, false),
        Status::BearingFailure
    );
    assert_eq!(
        at(Soil::Loose, Foundation::Mat, 12, true),
        Status::Liquefaction
    );
}

#[test]
fn from_tower_is_total() {
    for (floors, m) in [(0, f64::NAN), (60, 2.64e9), (u32::MAX, f64::INFINITY)] {
        let t = FromTower {
            floors,
            overturning: m,
        };
        assert!((2..=80).contains(&t.ground_floors()));
        assert!(t.edge_pressure().is_finite());
        assert!(!t.note().is_empty());
    }
}
