//! LAB-1 sample drawer state: Pull / Push / Fresh sample / temperature / length, the paced
//! stepper, the status sequence, the sentences and the readouts — deterministic and safe.

use sp_engine::builder::PieceType;
use sp_engine::builder::sample::bench::{TEMP_MAX_C, TEMP_MIN_C, TICK_SECONDS};
use sp_engine::builder::sample::{Failure, Sample, Status, Test, Tone};

fn statuses_to_the_end(s: &mut Sample) -> Vec<Status> {
    let mut seen: Vec<Status> = vec![s.status()];
    let mut n = 0;
    while s.playing() && n < 5000 {
        s.tick();
        let st = s.status();
        if std::mem::discriminant(&st) != std::mem::discriminant(seen.last().unwrap()) {
            seen.push(st);
        }
        n += 1;
    }
    seen
}

#[test]
fn steel_pulled_springs_yields_necks_and_tears() {
    let mut s = Sample::new(PieceType::Steel);
    assert_eq!(s.status(), Status::Ready);
    s.pull();
    assert!(s.playing());
    let seen = statuses_to_the_end(&mut s);
    assert_eq!(
        seen.iter()
            .map(|st| st.label(PieceType::Steel))
            .collect::<Vec<_>>(),
        vec![
            "Ready",
            "Elastic: springs back",
            "Plastic: permanently stretched",
            "Necking",
            "Fractured at 22%"
        ]
    );
    assert!(!s.playing());
    assert!(s.status().is_broken());
    assert_eq!(s.status().tone(), Tone::Danger);
    assert_eq!(s.force_n(), 0.0);
    let v = s.view();
    assert_eq!(v.status_label, "Fractured at 22%");
    assert!(
        v.explain.starts_with("It tore after stretching 22%."),
        "{}",
        v.explain
    );
    assert_eq!(v.limit, "A Steel piece this size breaks at 153 t.");
}

#[test]
fn the_stepper_lands_on_the_builders_limit() {
    let mut s = Sample::new(PieceType::Steel);
    s.pull();
    let cap = s.capacity_n();
    let mut hit = false;
    while s.playing() {
        s.tick();
        if (s.force_n() - cap).abs() <= 1e-6 * cap {
            hit = true;
            assert!(s.at_builder_limit());
            let v = s.view();
            assert_eq!(v.force_t, "153 t");
            assert_eq!(v.force_kn, "1500 kN");
            assert!(v.over_limit);
            assert_eq!(
                v.limit_note,
                "Past this the builder counts the piece as broken."
            );
            break;
        }
        assert!(
            s.force_n() < cap,
            "never past the limit on the straight line"
        );
    }
    assert!(hit, "one step ends exactly at the builder's capacity");
}

#[test]
fn every_type_pulls_to_failure_and_pushes_where_it_can() {
    for kind in PieceType::ALL {
        let mut s = Sample::new(kind);
        assert!(s.run_to_end(), "{kind} breaks when pulled");
        assert!(s.status().is_broken());
        if s.can_push() {
            s.push();
            assert_eq!(s.test(), Test::Push);
            assert_eq!(s.strain(), 0.0, "a fresh sample to push");
            assert!(s.run_to_end(), "{kind} gives way when pushed");
            let how = match s.status() {
                Status::Failed { how, .. } => how,
                other => panic!("{kind}: {other:?}"),
            };
            assert!(
                matches!(how, Failure::Crushed | Failure::Squashed),
                "{kind}: {how:?}"
            );
            // Long enough, everything that pushes buckles.
            s.set_length(s.max_length());
            s.push();
            s.run_to_end();
            if matches!(kind, PieceType::Steel | PieceType::Wood) {
                assert!(
                    matches!(
                        s.status(),
                        Status::Failed {
                            how: Failure::Buckled,
                            ..
                        }
                    ),
                    "{kind} at {} m: {:?}",
                    s.length(),
                    s.status()
                );
                assert!(s.limit_sentence().contains("buckles at"));
            }
        } else {
            let before = s.clone();
            s.push();
            assert_eq!(s, before, "{kind}: Push does nothing");
            assert_eq!(
                s.view().push_note,
                format!("{} only pulls: pushed, it goes slack.", kind.name())
            );
        }
    }
}

#[test]
fn pull_and_push_pause_and_restart() {
    let mut s = Sample::new(PieceType::Concrete);
    s.pull();
    s.tick();
    assert_eq!(s.view().pull_label, "Pause");
    s.pull();
    assert!(!s.playing(), "Pull while pulling pauses");
    assert_eq!(s.view().pull_label, "Pull");
    let e = s.strain();
    s.pull();
    assert!(s.playing());
    assert_eq!(s.strain(), e, "resumes where it was");
    s.run_to_end();
    s.pull();
    assert_eq!(
        s.strain(),
        0.0,
        "pulling a broken sample starts a fresh one"
    );
    s.fresh_sample();
    assert_eq!(s.status(), Status::Ready);
    assert!(!s.playing());
    // Opening another card: fresh, pulling, 1 m; the temperature stays.
    s.set_temp(450.0);
    s.set_length(5.0);
    s.open(PieceType::Wood);
    assert_eq!(s.kind(), PieceType::Wood);
    assert_eq!(s.test(), Test::Pull);
    assert_eq!(s.length(), 1.0);
    assert_eq!(s.temp(), 450.0);
}

#[test]
fn paced_by_real_time() {
    let mut s = Sample::new(PieceType::Wood);
    assert_eq!(s.next_tick_in(), None, "no repaint while idle");
    s.pull();
    assert_eq!(s.next_tick_in(), Some(TICK_SECONDS));
    s.advance(0.02);
    assert_eq!(s.strain(), 0.0);
    assert!((s.next_tick_in().unwrap() - 0.03).abs() < 1e-12);
    s.advance(0.03);
    assert!(s.strain() > 0.0);
    // 20 frames of 50 ms are 20 steps.
    let mut b = Sample::new(PieceType::Steel);
    b.pull();
    for _ in 0..20 {
        b.advance(0.05);
    }
    let mut c = Sample::new(PieceType::Steel);
    c.pull();
    for _ in 0..20 {
        c.tick();
    }
    assert_eq!(b.strain(), c.strain());
    // A long stall doesn't spin forever, and garbage dt does nothing.
    b.advance(f64::NAN);
    b.advance(-3.0);
    b.advance(1e9);
    assert!(b.strain().is_finite());
}

#[test]
fn heat_changes_the_test() {
    let mut s = Sample::new(PieceType::Steel);
    s.set_temp(600.0);
    let v = s.view();
    assert_eq!(v.temp_label, "600 \u{b0}C");
    assert!(v.temp_hot);
    assert_eq!(
        v.temp_note,
        "At 600 \u{b0}C steel keeps 47% of its strength and 31% of its stiffness (EN 1993-1-2)."
    );
    assert_eq!(v.limit, "A Steel piece this size breaks at 72 t.");
    assert!(
        !v.plot.ghost.is_empty(),
        "the room-temperature curve as a ghost"
    );
    // Heating a loaded wood sample past 300 °C burns it through.
    let mut w = Sample::new(PieceType::Wood);
    w.pull();
    for _ in 0..5 {
        w.tick();
    }
    w.set_temp(320.0);
    assert!(!w.playing());
    assert_eq!(w.status(), Status::BurnedThrough);
    assert_eq!(w.view().status_label, "Burned through");
    assert!(w.view().rig.charred);
    let mut r = Sample::new(PieceType::Rope);
    r.set_temp(900.0);
    assert_eq!(r.view().status_label, "Melted through");
    // The slider's range.
    r.set_temp(5000.0);
    assert_eq!(r.temp(), TEMP_MAX_C);
    r.set_temp(-5000.0);
    assert_eq!(r.temp(), TEMP_MIN_C);
    r.set_temp(f64::NAN);
    assert_eq!(r.temp(), TEMP_MIN_C);
    assert!(r.view().plot.ghost.is_empty(), "cold: no ghost");
}

#[test]
fn readouts_and_drawing_follow_the_state() {
    let mut s = Sample::new(PieceType::Steel);
    let v0 = s.view();
    assert_eq!(v0.title, "Steel sample");
    assert_eq!(v0.section_label, "60 cm\u{b2} section");
    assert_eq!(v0.force_t, "0.0 t");
    assert_eq!(v0.tiles[0].value, "200 GPa");
    assert_eq!(v0.tiles[1].value, "153 t");
    assert_eq!(v0.tiles[2].value, "153 t");
    assert_eq!(v0.tiles[3].value, "22 %");
    s.pull();
    for _ in 0..40 {
        s.tick();
    }
    let v = s.view();
    // The crosshead rises as it stretches; the marker sits on the curve.
    assert!(v.rig.head[0][1] < v0.rig.head[0][1]);
    assert!(!v.plot.done.is_empty());
    assert_eq!(v.plot.done.last().copied(), Some(v.plot.marker));
    assert!(
        v.rig_label
            .starts_with("Steel sample in a test machine, stretched")
    );
    assert!(v.plot_label.contains("the builder's limit is 250 MPa"));
    // Pushing moves the crosshead down and squashes the column.
    let mut p = Sample::new(PieceType::Steel);
    p.push();
    let start = p.view().rig.head[0][1];
    p.run_to_end();
    let end = p.view();
    assert!(end.rig.head[0][1] > start);
    assert_eq!(end.plot.x_title, "squeeze %");
    assert!(
        end.clock.starts_with("\u{2212}\u{3b5} = 2"),
        "{}",
        end.clock
    );
    assert_eq!(end.status_label, "Squashed at 2%");
    assert!(
        p.describe()
            .starts_with("Sample drawer: Steel pushed at 20 \u{b0}C")
    );
}

#[test]
fn deterministic() {
    let run = |k: PieceType| {
        let mut s = Sample::new(k);
        s.set_temp(550.0);
        s.pull();
        let mut trace = Vec::new();
        while s.playing() {
            s.advance(0.033);
            trace.push((s.strain().to_bits(), s.force_n().to_bits()));
        }
        trace
    };
    for k in PieceType::ALL {
        assert_eq!(run(k), run(k), "{k}");
    }
}
