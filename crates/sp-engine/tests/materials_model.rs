//! `sp_engine::materials` behaviour (SPEC MT-2, MT-5, MT-6, ENG-3, ENG-4): the default
//! readouts, Pull to fracture for every sample and the challenge, the timing, zoom,
//! temperature, the derived quantities and robustness on garbage input (speed:
//! `materials_perf.rs`).

use sp_engine::materials::bench::{CHALLENGE_RUNG, MAX_TICKS_PER_ADVANCE};
use sp_engine::materials::curves::{TEMP_MAX_C, TEMP_MIN_C, load_cell_kn};
use sp_engine::materials::{Bench, CURVES, Kind, Status, TICK_SECONDS, Tone, Zoom};
use sp_engine::materials::{stress_at, temp_factor};
use std::time::Instant;

fn tile<'a>(v: &'a sp_engine::materials::View, key: &str) -> &'a str {
    &v.tiles.iter().find(|t| t.key == key).expect(key).value
}

#[test]
fn default_state_shows_the_mockup_numbers() {
    let v = Bench::default().view();
    assert_eq!(v.kind, Kind::Steel);
    assert_eq!(tile(&v, "Stiffness E"), "200 GPa");
    assert_eq!(tile(&v, "Yield"), "275 MPa");
    assert_eq!(tile(&v, "Peak strength"), "430 MPa");
    assert_eq!(tile(&v, "Stretch at break"), "22 %");
    assert_eq!(tile(&v, "Toughness"), "83.72 MJ/m\u{b3}");
    assert_eq!(tile(&v, "Density"), "7850 kg/m\u{b3}");
    let bars: Vec<&str> = v.spec.iter().map(|b| b.label.as_str()).collect();
    assert_eq!(bars, ["55", "115", "17", "160", "375"]);
    let fills: Vec<f64> = v.spec.iter().map(|b| (b.fill * 100.0).round()).collect();
    assert_eq!(fills, [15.0, 31.0, 4.0, 43.0, 100.0]);
    assert!(v.spec[0].selected && !v.spec[1].selected);
    assert_eq!(v.status, Status::Ready);
    assert_eq!(v.status_label, "Ready");
    assert_eq!(v.status_tone, Tone::Muted);
    assert_eq!(v.clock, "\u{3b5} = 0.00 %");
    assert_eq!(v.force_label, "0.0 kN");
    assert_eq!(v.section_label, "50 mm\u{b2} section");
    assert_eq!(v.strain_label, "0.00 % \u{b7} 0 MPa");
    assert_eq!(v.temp_label, "20 \u{b0}C");
    assert_eq!(v.temp_note, "Room-temperature strength.");
    assert_eq!(v.challenge_rung, "5 / 12");
    assert_eq!(CHALLENGE_RUNG, (5, 12));
    assert!(v.challenge.iter().all(|c| !c.lit));
    assert!(
        v.explain
            .starts_with("At first the sample behaves like a stiff spring")
    );
    assert_eq!(
        v.rig_label,
        "Steel sample in a tensile test machine, stretched 0.00%. Ready."
    );
    assert_eq!(v.plot.ghosts.len(), 4);
    assert!(v.plot.done.is_empty());
    assert_eq!(v.rig.specimen.len(), 1);
    assert!(v.rig.crack.is_none());
}

/// Runs Pull at the app's pace (one `advance` per 50 ms frame) until it stops.
fn pull_to_stop(b: &mut Bench) -> (usize, Option<Kind>) {
    b.toggle_drive();
    assert!(b.playing());
    let mut frames = 0;
    let mut broke = None;
    while b.playing() && frames < 10_000 {
        if let Some(k) = b.advance(TICK_SECONDS) {
            broke = Some(k);
        }
        frames += 1;
    }
    (frames, broke)
}

#[test]
fn pull_breaks_each_material_and_lights_its_chip() {
    let mut b = Bench::default();
    for (n, kind) in Kind::ALL.into_iter().enumerate() {
        b.pick(kind);
        assert_eq!(b.zoom(), Zoom::fitting(kind));
        let (frames, broke) = pull_to_stop(&mut b);
        let brk = kind.curve().break_strain();
        assert_eq!(broke, Some(kind), "{kind:?}");
        assert!(b.broke(kind));
        assert_eq!(b.broke_count(), n + 1);
        // Fracture comes on the first step past the last corner.
        let step = b.zoom().pull_step();
        let ideal = brk / step;
        assert!(
            (frames as f64) >= ideal - 1e-6 && (frames as f64) <= ideal + 1.0 + 1e-6,
            "{kind:?}: {frames} frames"
        );
        assert!(b.strain() > brk && b.strain() <= brk + step + 1e-9);
        let v = b.view();
        assert_eq!(v.status, Status::Fractured { at: brk });
        assert_eq!(v.status_tone, Tone::Danger);
        assert_eq!(v.force_label, "0.0 kN");
        assert!(v.challenge[kind.index()].lit);
        assert_eq!(v.rig.specimen.len(), 2, "two halves");
        assert!(v.rig.crack.is_some());
        assert!(v.explain.ends_with("Try another material, or heat it up."));
    }
    assert!(b.all_broken());
    // Steel and aluminium tear after necking; the brittle three snap.
    b.pick(Kind::Timber);
    pull_to_stop(&mut b);
    assert!(
        b.explain()
            .starts_with("It snapped at 0.67% with no yielding")
    );
    b.reset();
    assert_eq!(b.broke_count(), 0);
    assert_eq!(b, Bench::default());
}

#[test]
fn steel_passes_elastic_plastic_necking_then_fracture() {
    let mut b = Bench::default();
    b.toggle_drive();
    let mut seen = Vec::new();
    while b.playing() {
        b.tick();
        let s = b.status();
        let tag = match s {
            Status::Ready => 0,
            Status::Elastic => 1,
            Status::Plastic => 2,
            Status::Necking => 3,
            Status::Fractured { .. } => 4,
        };
        if seen.last() != Some(&tag) {
            seen.push(tag);
        }
    }
    assert_eq!(seen, [1, 2, 3, 4]);
}

#[test]
fn timing_is_twenty_steps_a_second_and_paced() {
    let mut b = Bench::default();
    assert_eq!(b.next_tick_in(), None);
    assert_eq!(b.advance(1.0), None);
    assert_eq!(b.strain(), 0.0, "paused: nothing moves");
    b.toggle_drive();
    assert_eq!(b.next_tick_in(), Some(TICK_SECONDS));
    b.advance(1.0);
    let mut want = 0.0;
    for _ in 0..20 {
        want = 25.0_f64.min(want + 0.08);
    }
    assert_eq!(b.strain(), want, "1 s = 20 steps of 0.08 %");
    b.advance(0.02);
    assert!((b.next_tick_in().unwrap() - 0.03).abs() < 1e-9);
    // Garbage dt does nothing; a huge dt is capped and cannot spin.
    let before = b.strain();
    b.advance(f64::NAN);
    b.advance(-5.0);
    b.advance(f64::NEG_INFINITY);
    assert_eq!(b.strain(), before);
    let t = Instant::now();
    b.advance(f64::INFINITY);
    b.advance(1e300);
    assert!(t.elapsed().as_millis() < 50);
    assert!(!b.playing(), "a long stall finishes the pull");
    assert!(b.broke(Kind::Steel));
    assert!(MAX_TICKS_PER_ADVANCE as f64 * 0.08 > 25.0);
}

#[test]
fn pull_stops_at_the_axis_end_and_restarts_from_zero() {
    let mut b = Bench::default();
    b.set_zoom(Zoom::Close);
    let (_, broke) = pull_to_stop(&mut b);
    assert_eq!(broke, None);
    assert_eq!(b.strain(), 2.0);
    assert!(!b.broke(Kind::Steel));
    assert_eq!(b.status(), Status::Plastic);
    b.toggle_drive();
    assert!(b.playing());
    assert_eq!(b.strain(), 0.0, "Pull at the axis end starts a fresh pull");
    b.tick();
    assert_eq!(b.strain(), 0.006);
    b.toggle_drive();
    assert!(!b.playing(), "Pause");
    b.toggle_drive();
    assert_eq!(b.strain(), 0.006, "resume keeps the stretch");
}

#[test]
fn zoom_scrub_and_fresh_sample() {
    let mut b = Bench::default();
    b.set_strain(12.5);
    assert_eq!(b.strain(), 12.5);
    b.set_zoom(Zoom::Close);
    assert_eq!(b.strain(), 2.0, "the stretch clips to the new axis");
    b.set_zoom(Zoom::Wide);
    assert_eq!(b.strain(), 2.0);
    b.set_strain(99.0);
    assert_eq!(b.strain(), 25.0);
    b.toggle_drive();
    b.set_strain(3.0);
    assert!(!b.playing(), "scrubbing pauses");
    b.fresh_sample();
    assert_eq!((b.strain(), b.playing()), (0.0, false));
    for k in Kind::ALL {
        b.pick(k);
        let want = if k.curve().break_strain() < 2.0 {
            Zoom::Close
        } else {
            Zoom::Wide
        };
        assert_eq!(b.zoom(), want);
    }
    assert_eq!(Zoom::Wide.label(), "0 \u{2013} 25 %");
    assert_eq!(Zoom::Close.ticks(), [0.0, 0.5, 1.0, 1.5, 2.0]);
    assert_eq!(Zoom::parse("0-2 %"), Some(Zoom::Close));
    assert_eq!(Zoom::parse("25"), Some(Zoom::Wide));
    assert_eq!(Zoom::parse("13"), None);
}

#[test]
fn temperature_factors_and_notes() {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
    assert_eq!(temp_factor(Kind::Steel, 300.0), 1.0);
    assert!(close(temp_factor(Kind::Steel, 450.0), 0.735));
    assert!(close(temp_factor(Kind::Steel, 600.0), 0.47));
    assert!(close(temp_factor(Kind::Aluminum, 300.0), 0.3));
    assert!(close(temp_factor(Kind::Aluminum, 400.0), 0.08));
    assert!(close(temp_factor(Kind::Concrete, 600.0), 0.45));
    assert!(close(temp_factor(Kind::Timber, 200.0), 0.5));
    assert_eq!(temp_factor(Kind::Timber, 300.0), 0.0);
    assert!(close(temp_factor(Kind::CarbonFiber, 200.0), 0.25));
    for k in Kind::ALL {
        assert_eq!(temp_factor(k, -40.0), 1.0);
        assert_eq!(temp_factor(k, 20.0), 1.0);
        assert_eq!(temp_factor(k, f64::NAN), 1.0, "NaN = room temperature");
        assert!((0.0..=1.0).contains(&temp_factor(k, f64::INFINITY)));
        // Monotone: heat never makes a material stronger.
        let mut prev = 1.0;
        for t in (-40..=600).step_by(10) {
            let f = temp_factor(k, t as f64);
            assert!(f <= prev + 1e-15, "{k:?} at {t}");
            prev = f;
        }
    }
    let mut b = Bench::default();
    b.set_temp(450.0);
    let v = b.view();
    assert_eq!(
        v.temp_note,
        "Strength now 74% of room temperature (illustrative)."
    );
    assert_eq!(v.temp_tone, Tone::Caution);
    assert_eq!(tile(&v, "Peak strength"), "316 MPa");
    assert_eq!(tile(&v, "Yield"), "202 MPa");
    assert_eq!(
        tile(&v, "Toughness"),
        "83.72 MJ/m\u{b3}",
        "as the mockup: room temperature"
    );
    b.pick(Kind::Aluminum);
    assert!(
        b.explain()
            .ends_with(" At 450°C this one has lost much of its strength.")
    );
    b.set_temp(1e9);
    assert_eq!(b.temp(), TEMP_MAX_C);
    b.set_temp(-1e9);
    assert_eq!(b.temp(), TEMP_MIN_C);
    b.set_temp(f64::NAN);
    assert_eq!(b.temp(), TEMP_MIN_C, "NaN is ignored");
}

#[test]
fn derived_quantities() {
    let tough: Vec<f64> = CURVES.iter().map(|c| c.toughness()).collect();
    let want = [83.7234375, 35.575, 0.1025, 0.268, 2.58];
    for (got, want) in tough.iter().zip(want) {
        assert!((got - want).abs() < 1e-9, "{got} vs {want}");
    }
    assert!((Kind::Steel.curve().toughness_at(450.0) - 83.7234375 * 0.735).abs() < 1e-9);
    let spec: Vec<f64> = CURVES.iter().map(|c| c.specific_strength(20.0)).collect();
    for (got, want) in spec.iter().zip([54.777, 114.815, 16.667, 160.0, 375.0]) {
        assert!((got - want).abs() < 1e-3, "{got} vs {want}");
    }
    // Necking: only the metals, only after the ultimate strain, 0.55 at fracture.
    let steel = Kind::Steel.curve();
    assert_eq!(steel.necking(15.0), 0.0);
    assert!((steel.necking(18.5) - 0.275).abs() < 1e-12);
    assert!((steel.necking(22.0) - 0.55).abs() < 1e-12);
    assert!((steel.necking(30.0) - 0.55).abs() < 1e-12);
    for k in [Kind::Concrete, Kind::Timber, Kind::CarbonFiber] {
        assert_eq!(k.curve().necking(100.0), 0.0);
    }
    // Load cell: 430 MPa on 50 mm² = 21.5 kN.
    assert!((load_cell_kn(stress_at(Kind::Steel, 15.0, 20.0).mpa) - 21.5).abs() < 1e-12);
    // Fracture past the last corner, not at it.
    let at = stress_at(Kind::Steel, 22.0, 20.0);
    assert!(!at.broken && at.mpa == 360.0);
    let past = stress_at(Kind::Steel, 22.000001, 20.0);
    assert!(past.broken && past.mpa == 0.0 && past.break_strain == 22.0);
    // Yield where defined.
    assert_eq!(Kind::Steel.curve().yield_at(20.0), Some(275.0));
    assert_eq!(Kind::Aluminum.curve().yield_at(20.0), Some(276.0));
    assert_eq!(Kind::Timber.curve().yield_at(20.0), None);
    assert!(Kind::Steel.curve().is_ductile() && !Kind::Concrete.curve().is_ductile());
}

#[test]
fn share_and_describe() {
    let mut b = Bench::default();
    assert_eq!(
        b.share(),
        ".material steel E=200GPa fy=275MPa rho=7850 curve=steel ; tensile test at 20 \u{b0}C, \u{3b5} = 0.00 %, Ready"
    );
    b.pick(Kind::CarbonFiber);
    b.set_temp(150.0);
    b.set_strain(0.5);
    assert_eq!(
        b.share(),
        ".material carbon-fiber E=70GPa fy=375MPa rho=1600 curve=carbon-fiber ; tensile test at 150 \u{b0}C, \u{3b5} = 0.50 %, Elastic: springs back"
    );
    assert!(
        b.describe()
            .starts_with("Materials lab: Carbon fiber at 150 °C")
    );
    assert!(!b.share().contains('\n'));
}

#[test]
fn bench_round_trips_through_serde() {
    let mut b = Bench::default();
    b.pick(Kind::Aluminum);
    b.set_temp(250.0);
    b.set_strain(3.3);
    let json = serde_json::to_string(&b).unwrap();
    let back: Bench = serde_json::from_str(&json).unwrap();
    assert_eq!(back, b);
}

fn assert_view_finite(b: &Bench) {
    let v = b.view();
    let pts = v
        .plot
        .ghosts
        .iter()
        .flat_map(|(_, p)| p.iter())
        .chain(&v.plot.curve)
        .chain(&v.plot.done)
        .chain(std::iter::once(&v.plot.marker))
        .chain(v.rig.specimen.iter().flatten())
        .chain(v.plot.labels.iter().map(|l| &l.at));
    for p in pts {
        assert!(p[0].is_finite() && p[1].is_finite(), "{p:?}");
    }
    assert!(v.rig.top_y.is_finite() && (0.0..=0.55).contains(&v.rig.necking));
    assert!(
        v.spec
            .iter()
            .all(|s| s.value.is_finite() && (0.0..=1.0).contains(&s.fill))
    );
    assert!(!v.strain_label.contains("NaN") && !v.force_label.contains("NaN"));
    assert!(!b.share().contains("NaN"));
}

#[test]
fn garbage_input_never_panics_or_leaks_nan() {
    let nasty = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        -1e300,
        0.0,
        -0.0,
        1e-320,
        0.1375,
        22.0,
        1e300,
        f64::MAX,
        f64::MIN_POSITIVE,
    ];
    for k in Kind::ALL {
        for &e in &nasty {
            for &t in &nasty {
                let s = stress_at(k, e, t);
                assert!(
                    s.mpa.is_finite() && s.mpa >= 0.0,
                    "{k:?} e={e} t={t}: {s:?}"
                );
                assert!(temp_factor(k, t).is_finite());
                assert!(k.curve().necking(e).is_finite());
                let mut b = Bench::default();
                b.pick(k);
                b.set_temp(t);
                b.set_strain(e);
                assert!(b.strain().is_finite() && b.temp().is_finite());
                assert_view_finite(&b);
                b.toggle_drive();
                b.advance(e);
                b.advance(t);
                assert_view_finite(&b);
                for z in Zoom::ALL {
                    b.set_zoom(z);
                    assert_view_finite(&b);
                }
            }
        }
    }
    assert_eq!(Kind::parse(""), None);
    assert_eq!(Kind::parse("\u{0}\u{ffff}"), None);
    assert_eq!(Zoom::parse("NaN"), None);
}
