//! Materials (SCR-5, SPEC MT-1…MT-6, NAV-1, NAV-6, PROV-1) driven through the real app by
//! AccessKit labels: the defaults show the mockup's numbers, Pull runs each sample to fracture
//! and lights its challenge chip, scrubbing, temperature, the strain axis, Fresh sample, Reset,
//! Share, Space, and the narrow (390 px) layout.

mod common;

use common::*;
use eframe::egui::{Key, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::materials::{Bench, Kind};
use structures_playground::Route;

/// The text of every `Label` / `Status` node.
fn texts(h: &App) -> Vec<String> {
    h.root()
        .children_recursive()
        .filter(|n| matches!(n.accesskit_node().role(), Role::Label | Role::Status))
        .map(|n| text_of(&n))
        .collect()
}

fn has_text(h: &App, want: &str) -> bool {
    texts(h).iter().any(|t| t == want)
}

fn has_text_starting(h: &App, start: &str) -> Option<String> {
    texts(h).into_iter().find(|t| t.starts_with(start))
}

fn status(h: &App) -> String {
    let n = h.get_by_role(Role::Status);
    text_of(&n)
}

fn bench(h: &App) -> &Bench {
    h.state().screens().materials.bench()
}

fn materials() -> App {
    let mut h = harness(Route::Materials);
    settle(&mut h);
    h
}

#[test]
fn defaults_show_the_mockup_numbers() {
    let h = materials();
    assert_eq!(document(&h), "tensile-test");
    assert_eq!(drive_label(&h).as_deref(), Some("Pull"));
    for t in [
        "Stiffness E: 200 GPa, slope of the first line",
        "Yield: 275 MPa, where it stops springing back",
        "Peak strength: 430 MPa, ultimate",
        "Stretch at break: 22 %, ductile",
        "Toughness: 83.72 MJ/m\u{b3}, area under the curve",
        "Density: 7850 kg/m\u{b3}",
        "Steel: 55 kN\u{b7}m/kg",
        "Aluminum: 115 kN\u{b7}m/kg",
        "Concrete: 17 kN\u{b7}m/kg",
        "Timber: 160 kN\u{b7}m/kg",
        "Carbon fiber: 375 kN\u{b7}m/kg",
        "Load cell 0.0 kN \u{b7} 50 mm\u{b2} section",
        "Stretch: 0.00 % \u{b7} 0 MPa",
        "Temperature: 20 \u{b0}C",
        "Room-temperature strength.",
        "Steel: not broken yet",
        "Carbon fiber: not broken yet",
        "Curves: representative",
        "Heat effects: illustrative",
        "Not certified data",
        "Fatigue",
        "Grain growth",
        "\u{3b5} = 0.00 %",
    ] {
        assert!(has_text(&h, t), "missing {t:?} in {:#?}", texts(&h));
    }
    assert_eq!(status(&h), "Ready");
    assert!(
        has_text_starting(&h, "At first the sample behaves like a stiff spring").is_some()
            || has_label_containing(&h, "At first the sample behaves like a stiff spring")
    );
    assert_eq!(pressed(&h, "Steel"), Some(true));
    assert_eq!(pressed(&h, "Aluminum"), Some(false));
    assert_eq!(pressed(&h, "0 \u{2013} 25 %"), Some(true));
    let rig = get(
        &h,
        "Steel sample in a tensile test machine, stretched 0.00%. Ready.",
        Role::Image,
    );
    let plot = get(
        &h,
        "Stress-strain curves for five materials. Steel highlighted, now at 0 MPa.",
        Role::Image,
    );
    assert!(
        rig.rect().right() < plot.rect().left(),
        "side by side at 1440"
    );
    assert!(has_text(
        &h,
        "Mild structural steel. Yields, flows, hardens, necks, then tears. Lots of warning."
    ));
}

/// Pull until it stops (time passes in steps of up to 0.1 s: two 50 ms Pull steps a frame).
fn pull_to_stop(h: &mut App) {
    click_drive(h);
    assert_eq!(drive_label(h).as_deref(), Some("Pause"));
    let stopped = step_until(h, 400, |h| !bench(h).playing());
    assert!(stopped, "Pull never stopped (strain {})", bench(h).strain());
    settle(h);
}

#[test]
fn pull_breaks_every_sample_and_lights_its_chip() {
    let mut h = materials();
    for (n, kind) in Kind::ALL.into_iter().enumerate() {
        click_button(&mut h, kind.name());
        assert_eq!(pressed(&h, kind.name()), Some(true));
        assert_eq!(status(&h), "Ready");
        pull_to_stop(&mut h);
        let brk = kind.curve().break_strain();
        assert_eq!(status(&h), format!("Fractured at {brk}%"));
        assert_eq!(drive_label(&h).as_deref(), Some("Pull"));
        assert!(
            has_text(&h, &format!("{}: broken", kind.name())),
            "{kind:?}"
        );
        assert!(has_text(&h, "Load cell 0.0 kN \u{b7} 50 mm\u{b2} section"));
        assert!(
            h.query_all_by_label_contains(&format!(
                "{} sample in a tensile test machine",
                kind.name()
            ))
            .any(|n| n
                .accesskit_node()
                .label()
                .unwrap_or_default()
                .contains("Fractured")),
        );
        assert_eq!(bench(&h).broke_count(), n + 1);
        // The clock shows the stretch at the stop.
        let clock = bench(&h).clock();
        assert!(has_text(&h, &clock), "{clock}");
    }
    assert!(
        Kind::ALL
            .iter()
            .all(|k| has_text(&h, &format!("{}: broken", k.name())))
    );
    // Pull again on a broken sample starts a fresh one.
    click_drive(&mut h);
    assert!(bench(&h).playing());
    assert!(bench(&h).strain() < 0.05);
    // Reset clears the challenge.
    click_header(&mut h, "Reset");
    assert_eq!(status(&h), "Ready");
    assert!(has_text(&h, "Steel: not broken yet"));
    assert!(!bench(&h).playing());
    assert_eq!(pressed(&h, "Steel"), Some(true));
}

#[test]
fn space_pulls_and_pauses() {
    let mut h = materials();
    press(&mut h, Key::Space);
    assert_eq!(drive_label(&h).as_deref(), Some("Pause"));
    for _ in 0..5 {
        h.step();
    }
    let e = bench(&h).strain();
    assert!(e > 0.0);
    press(&mut h, Key::Space);
    assert_eq!(drive_label(&h).as_deref(), Some("Pull"));
    for _ in 0..5 {
        h.step();
    }
    assert_eq!(bench(&h).strain(), e, "paused");
    assert_ne!(status(&h), "Ready");
}

#[test]
fn scrubbing_the_stretch_and_fresh_sample() {
    let mut h = materials();
    // (AccessKit increments: the shared Slider loses focus to egui's arrow-key navigation,
    // reported to the shell in PROGRESS.)
    reveal(&mut h, "Stretch", Some(Role::Slider));
    for _ in 0..5 {
        get(&h, "Stretch", Role::Slider).increment_accesskit();
        h.step();
    }
    settle(&mut h);
    assert_eq!(bench(&h).strain(), 0.25);
    assert!(has_text(&h, "Stretch: 0.25 % \u{b7} 275 MPa"));
    assert_eq!(status(&h), "Plastic: permanently stretched");
    assert!(has_text(&h, "Load cell 13.8 kN \u{b7} 50 mm\u{b2} section"));
    assert!(has_text(&h, "\u{3b5} = 0.25 %"));
    click_button(&mut h, "Fresh sample");
    assert_eq!(bench(&h).strain(), 0.0);
    assert_eq!(status(&h), "Ready");
}

#[test]
fn temperature_changes_the_curves_and_notes() {
    let mut h = materials();
    click_button(&mut h, "Aluminum");
    get(&h, "Temperature", Role::Slider).set_value_accesskit(250.0);
    h.step();
    settle(&mut h);
    assert_eq!(bench(&h).temp(), 250.0);
    assert!(has_text(&h, "Temperature: 250 \u{b0}C"));
    assert!(has_text(
        &h,
        "Strength now 48% of room temperature (illustrative)."
    ));
    assert!(has_text(&h, "Peak strength: 147 MPa, ultimate"));
    assert!(has_text(&h, "Aluminum: 55 kN\u{b7}m/kg"));
    assert!(
        has_label_containing(&h, "At 250\u{b0}C this one has lost much of its strength")
            || texts(&h)
                .iter()
                .any(|t| t.contains("At 250\u{b0}C this one has lost much of its strength"))
    );
    // Back to room temperature, a step at a time.
    for _ in 0..23 {
        get(&h, "Temperature", Role::Slider).decrement_accesskit();
        h.step();
    }
    assert_eq!(bench(&h).temp(), 20.0);
    settle(&mut h);
    assert!(has_text(&h, "Room-temperature strength."));
}

#[test]
fn the_strain_axis_zooms_and_samples_pick_their_axis() {
    let mut h = materials();
    click_button(&mut h, "0 \u{2013} 2 %");
    assert_eq!(pressed(&h, "0 \u{2013} 2 %"), Some(true));
    assert_eq!(pressed(&h, "0 \u{2013} 25 %"), Some(false));
    let slider = get(&h, "Stretch", Role::Slider);
    assert_eq!(slider.accesskit_node().max_numeric_value(), Some(2.0));
    assert_eq!(
        slider.accesskit_node().numeric_value_step(),
        Some(0.005_f32 as f64)
    );
    click_button(&mut h, "Steel");
    assert_eq!(
        pressed(&h, "0 \u{2013} 25 %"),
        Some(true),
        "steel fits 0–25 %"
    );
    click_button(&mut h, "Timber");
    assert_eq!(
        pressed(&h, "0 \u{2013} 2 %"),
        Some(true),
        "timber breaks before 2 %"
    );
    assert!(has_text(&h, "Yield: none, brittle: no yield"));
    assert!(has_text(&h, "Stretch at break: 0.67 %, brittle"));
}

#[test]
fn share_copies_a_material_line() {
    let mut h = materials();
    click_header(&mut h, "Share");
    let copied = last_copied(&h).expect("copied");
    assert_eq!(
        copied,
        ".material steel E=200GPa fy=275MPa rho=7850 curve=steel ; tensile test at 20 \u{b0}C, \u{3b5} = 0.00 %, Ready"
    );
    assert!(
        toasts(&h)
            .iter()
            .any(|t| t == "Copied the sample as a .material line")
    );
}

#[test]
fn stacks_at_390() {
    let mut h = app().at(Route::Materials).phone().build();
    settle(&mut h);
    let rig = get(
        &h,
        "Steel sample in a tensile test machine, stretched 0.00%. Ready.",
        Role::Image,
    )
    .rect();
    let plot = get(
        &h,
        "Stress-strain curves for five materials. Steel highlighted, now at 0 MPa.",
        Role::Image,
    )
    .rect();
    assert!(plot.top() > rig.bottom(), "the plot goes under the rig");
    assert!((rig.width() - plot.width()).abs() < 1.0);
    let parts = get(&h, "Parts", Role::Region).rect();
    let stage = get(&h, "Stage", Role::Region).rect();
    let readouts = get(&h, "Readouts", Role::Region).rect();
    assert!(parts.bottom() <= stage.top() + 1.0 && stage.bottom() <= readouts.top() + 1.0);
    // Everything stays reachable: pick a sample and Pull from the phone layout.
    click_button(&mut h, "Concrete");
    click_button(&mut h, "Fresh sample");
    reveal(&mut h, "Pull", Some(Role::Button));
    pull_to_stop(&mut h);
    assert_eq!(status(&h), "Fractured at 0.35%");
}

/// PLAT-3: while pulling, the screen asks for the next frame only when the next 50 ms step is
/// due (no busy repaint); stopped, it asks for none.
#[test]
fn pull_is_paced_and_idle_is_free() {
    use eframe::egui::ViewportId;
    use std::time::Duration;
    let asked = |h: &App| h.output().viewport_output[&ViewportId::ROOT].repaint_delay;
    let mut h = materials();
    h.run_steps(4);
    assert!(
        asked(&h) >= Duration::from_secs(1),
        "at rest: {:?}",
        asked(&h)
    );
    click_drive(&mut h);
    h.step();
    let d = asked(&h);
    assert!(
        d > Duration::ZERO && d <= Duration::from_millis(60),
        "pulling asks for a frame in {d:?}"
    );
    click_drive(&mut h);
    h.run_steps(4);
    assert!(
        asked(&h) >= Duration::from_secs(1),
        "paused: {:?}",
        asked(&h)
    );
}
