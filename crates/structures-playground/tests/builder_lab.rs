//! LAB-1 and FIRE-1 in the real app, through AccessKit: a Materials card opens the sample drawer
//! over the aside; Pull breaks the sample with readouts equal to the engine's; Push, the
//! temperature slider and Close / Esc; the drawer asks for no frames at rest. The Fire row
//! (chips, note, "Light it now"), the HUD chip "Fire …" and the glow on hot pieces during a run.

mod common;

use common::*;
use eframe::egui::{self, ViewportId, accesskit::Role};
use sp_engine::builder::fire::{self, FireLevel};
use sp_engine::builder::sample::Sample;
use sp_engine::builder::view::Inspector;
use sp_engine::builder::{Design, Foundation, Game, Joint, Piece, PieceType, Scene};
use std::time::Duration;
use structures_playground::Route;
use structures_playground::screens::builder::host::FoldIn;
use structures_playground::screens::builder::{fire as fire_ui, lab};

/// A tall window so the whole aside (and the drawer) fits.
fn tall() -> App {
    app().at(Route::Playground).size(1440.0, 1800.0).build()
}

/// The Materials card's accessible label, from the engine's view of level 1.
fn card(kind: PieceType) -> String {
    let Inspector::Materials(cards) = Game::new().view().inspector else {
        panic!("level 1 starts with the Materials list");
    };
    let c = cards
        .iter()
        .find(|c| c.kind == kind)
        .expect("a card for the piece type");
    format!("{} \u{b7} {} \u{b7} {}", c.name, c.blurb, c.cost)
}

fn lab_of(h: &App) -> &lab::Lab {
    &h.state().screens().builder.fold.lab
}

/// The repaint the last frame asked for.
fn asked(h: &App) -> Duration {
    h.output().viewport_output[&ViewportId::ROOT].repaint_delay
}

#[test]
fn a_material_card_opens_the_sample_drawer_and_esc_closes_it() {
    let mut h = tall();
    assert!(!has_label(&h, lab::GROUP));
    click(&mut h, &card(PieceType::Steel));
    assert!(lab_of(&h).is_open());
    assert!(has_label(&h, lab::GROUP));
    assert!(has_label(&h, "Steel sample"));
    // It takes the aside's place.
    assert!(!has_label(&h, "Conditions"));
    let engine = Sample::new(PieceType::Steel);
    assert!(has_label(&h, &engine.limit_sentence()));
    assert!(has_label(&h, "Ready"));
    assert!(has_label(&h, "Force now 0.0 t \u{b7} 0.0 kN"));
    for t in engine.view().tiles {
        assert!(
            has_label_containing(&h, &format!("{}: {}", t.key, t.value)),
            "{}",
            t.key
        );
    }
    assert!(has_label_containing(
        &h,
        "Steel sample in a test machine, stretched 0 %"
    ));
    // At rest it asks for no frames (PLAT-3).
    h.run_steps(6);
    assert!(asked(&h) >= Duration::from_secs(1), "{:?}", asked(&h));
    // Esc closes it; the aside is back.
    press(&mut h, egui::Key::Escape);
    assert!(!lab_of(&h).is_open());
    assert!(!has_label(&h, lab::GROUP));
    assert!(has_label(&h, "Conditions"));
    // Close does too.
    click(&mut h, &card(PieceType::Concrete));
    assert!(has_label(&h, "Concrete sample"));
    click_button(&mut h, lab::CLOSE);
    assert!(!lab_of(&h).is_open());
}

#[test]
fn pull_breaks_the_sample_with_the_engines_readouts() {
    let mut h = tall();
    click(&mut h, &card(PieceType::Wood));
    click_button(&mut h, "Pull");
    assert!(lab_of(&h).sample().playing());
    assert!(has_label(&h, "Pause pulling"));
    // While it pulls the drawer paces itself at the engine's 20 steps a second.
    h.step();
    assert!(asked(&h) <= Duration::from_millis(60), "{:?}", asked(&h));
    let mut done = Sample::new(PieceType::Wood);
    done.run_to_end();
    let broke = done.status().label(PieceType::Wood);
    assert_eq!(broke, "Snapped at 0.25%");
    assert!(
        step_until(&mut h, 400, |h| has_label(h, &broke)),
        "it snaps"
    );
    let s = lab_of(&h).sample().clone();
    assert!(!s.playing());
    assert_eq!(s.status(), done.status());
    let v = s.view();
    assert!(has_label(
        &h,
        &format!("Force now {} \u{b7} {}", v.force_t, v.force_kn)
    ));
    assert!(has_label_containing(
        &h,
        "It snapped at 0.25% with no yielding"
    ));
    assert!(has_label(&h, "Pull"), "the button is Pull again");
    // Resting again: no frames.
    h.run_steps(4);
    assert!(asked(&h) >= Duration::from_secs(1), "{:?}", asked(&h));
    // The summary for MCP describes the drawer.
    let d = h.state().screens().builder.fold.lab.sample().describe();
    assert!(
        d.starts_with("Sample drawer: Wood pulled at 20 \u{b0}C"),
        "{d}"
    );
}

#[test]
fn push_heat_and_length() {
    let mut h = tall();
    click(&mut h, &card(PieceType::Steel));
    // The temperature slider (AccessKit set value) scales the builder's limit by EN 1993-1-2.
    get(&h, "Temperature", Role::Slider).set_value_accesskit(600.0);
    settle(&mut h);
    assert_eq!(lab_of(&h).sample().temp(), 600.0);
    let mut hot = Sample::new(PieceType::Steel);
    hot.set_temp(600.0);
    assert!(has_label(&h, &hot.limit_sentence()));
    assert_eq!(
        hot.limit_sentence(),
        "A Steel piece this size breaks at 72 t."
    );
    assert!(has_label(
        &h,
        "At 600 \u{b0}C steel keeps 47% of its strength and 31% of its stiffness (EN 1993-1-2)."
    ));
    // Push: a 1 m sample gives way at the builder's compression capacity.
    click_button(&mut h, "Push");
    assert!(has_label(&h, "Pause pushing"));
    hot.push();
    assert!(has_label(&h, &hot.limit_sentence()));
    // A longer sample buckles.
    get(&h, "Sample length", Role::Slider).set_value_accesskit(14.0);
    settle(&mut h);
    let s = lab_of(&h).sample();
    assert_eq!(s.length(), 14.0);
    assert!(
        s.limit_sentence().contains("buckles at"),
        "{}",
        s.limit_sentence()
    );
    assert!(has_label(&h, &s.limit_sentence()));
    // Rope only pulls.
    click_button(&mut h, lab::CLOSE);
    click(&mut h, &card(PieceType::Rope));
    assert!(disabled(&h, "Push"));
    assert!(has_label(&h, "Rope only pulls: pushed, it goes slack."));
}

/// Open-ground Sandbox: a steel A-frame on two footings with a 40 t crate on top — right over
/// where the fire burns (the tower's base).
fn steel_frame() -> Design {
    let j = |id: &str, x: f64, y: f64, f: bool| Joint {
        id: id.into(),
        x,
        y,
        sup: f.then_some(Foundation::Footing),
        locked: false,
    };
    let p = |id: &str, a: &str, b: &str| Piece {
        id: id.into(),
        a: a.into(),
        b: b.into(),
        kind: PieceType::Steel,
        locked: false,
    };
    Design {
        nodes: vec![
            j("J1", 92.0, 0.0, true),
            j("J2", 108.0, 0.0, true),
            j("J3", 92.0, 8.0, false),
            j("J4", 100.0, 8.0, false),
            j("J5", 108.0, 8.0, false),
        ],
        members: vec![
            p("M1", "J1", "J3"),
            p("M2", "J2", "J5"),
            p("M3", "J3", "J4"),
            p("M4", "J4", "J5"),
            p("M5", "J1", "J4"),
            p("M6", "J2", "J4"),
        ],
        loads: vec![("J4".into(), 40.0)],
        n_n: 5,
        n_m: 6,
    }
}

/// Pixels of the stage that look like fire or a hot glow (strong red, little blue).
fn warm_pixels(img: &image::RgbaImage) -> usize {
    img.pixels()
        .filter(|p| p[0] > 150 && p[2] < 90 && (p[0] as i32 - p[1] as i32) > 40)
        .count()
}

#[test]
fn the_fire_row_the_hud_chip_and_the_glow() {
    let mut h = tall();
    h.state_mut()
        .screens_mut()
        .builder
        .game
        .open_in_sandbox(Scene::Flat, steel_frame());
    settle(&mut h);
    // The Fire row: four chips, Off pressed, the note.
    assert_eq!(pressed(&h, fire_ui::CHIP_NAMES[0]), Some(true));
    click_button(&mut h, "Big fire");
    assert_eq!(pressed(&h, "Big fire"), Some(true));
    assert_eq!(pressed(&h, fire_ui::CHIP_NAMES[0]), Some(false));
    assert_eq!(
        h.state().screens().builder.game.cond().fire,
        FireLevel::Big.setting()
    );
    assert!(has_label(&h, &fire::note(FireLevel::Big, false)));
    assert!(!has_label(&h, "Light it now"), "only while running");
    let before = warm_pixels(&render(&mut h));

    // Run the test: "Light it now" shows until the fire lights.
    click_drive(&mut h);
    h.step();
    let lit_at = h
        .state()
        .screens()
        .builder
        .game
        .sim()
        .and_then(|s| s.fire_start);
    if lit_at.is_some_and(|l| l > 0.1) {
        assert!(has_label(&h, "Light it now"));
        click_button(&mut h, "Light it now");
    }
    // The HUD chip reads the engine's fire temperature.
    assert!(
        step_until(&mut h, 200, |h| has_label_containing(h, "Fire ")
            && h.state()
                .screens()
                .builder
                .game
                .sim()
                .is_some_and(|s| fire::blaze(s).burning())),
        "the fire lights"
    );
    h.run_steps(150);
    let game = &h.state().screens().builder.game;
    let sim = game.sim().expect("running");
    let value = fire::hud_value(sim).expect("burning");
    assert!(
        has_label(&h, &format!("Fire {value}")),
        "HUD chip Fire {value}"
    );
    assert!(fire::hottest(sim).is_some_and(|(_, t)| t > 300.0));
    // Flames and hot pieces glow on the stage.
    let after = warm_pixels(&render(&mut h));
    assert!(after > before + 200, "warm pixels {before} → {after}");
    let d = fire_ui::Fire
        .describe(&h.state().screens().builder.game)
        .expect("fire is set");
    assert!(d.starts_with("Fire: Big (700 \u{b0}C), burning at"), "{d}");
}
