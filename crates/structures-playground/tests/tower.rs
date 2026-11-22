//! SCR-3 Tower in the real app (egui_kittest, AccessKit labels): TW-1 … TW-7, NAV-1, NAV-4,
//! NAV-6, PROV-1, PLAT-3, DONE-2, DONE-4 (the tower half), LOOK-1.

mod common;

use common::*;
use eframe::egui::{self, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::tower::{self, Shape, System, TowerInput};
use structures_playground::{Carry, Route};

/// Mean |Δ| of `region` of `img` against the reference, at the best alignment within ±3 px
/// (sub-pixel line boxes put Chrome's text and controls a pixel or two away from ours; a
/// layout difference larger than that still shows).
fn aligned_diff(
    img: &image::RgbaImage,
    mock: &image::RgbaImage,
    region: (u32, u32, u32, u32),
) -> f64 {
    let (x, y, w, h) = region;
    let mut best = f64::MAX;
    for dy in -3i32..=3 {
        for dx in -3i32..=3 {
            let (ax, ay) = ((x as i32 + dx).max(0) as u32, (y as i32 + dy).max(0) as u32);
            let (mut sum, mut n) = (0u64, 0u64);
            for j in 0..h {
                for i in 0..w {
                    let (px, py) = (ax + i, ay + j);
                    let (qx, qy) = (x + i, y + j);
                    if px >= img.width()
                        || py >= img.height()
                        || qx >= mock.width()
                        || qy >= mock.height()
                    {
                        continue;
                    }
                    let (p, q) = (img.get_pixel(px, py).0, mock.get_pixel(qx, qy).0);
                    sum += (0..3)
                        .map(|c| (p[c] as i32 - q[c] as i32).unsigned_abs() as u64)
                        .sum::<u64>();
                    n += 3;
                }
            }
            if n > 0 {
                best = best.min(sum as f64 / n as f64);
            }
        }
    }
    best
}

fn set_slider(h: &mut App, label: &str, v: f64) {
    h.get_by_role_and_label(Role::Slider, label)
        .set_value_accesskit(v);
    h.run_steps(2);
}

fn tower_input(h: &App) -> TowerInput {
    h.state().screens().tower.input
}

fn goal_done(h: &App, text: &str) -> bool {
    get(h, text, Role::CheckBox).accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

/// The labels the engine says the screen should show for `input` (DONE-2: nothing hard-coded
/// on screen).
fn expect_engine_labels(h: &App, input: &TowerInput) {
    let r = tower::calc(input);
    for tl in tower::tiles(&r) {
        let label = format!("{}: {}, {}", tl.key, tl.value, tl.note);
        assert!(has_label(h, &label), "missing tile {label:?}");
    }
    assert!(has_label(
        h,
        &format!("Occupant comfort: {}", tower::accel_label(&r))
    ));
    assert!(has_label(
        h,
        &format!("Status: {}", tower::Status::of(&r).label())
    ));
    assert!(has_label(h, &tower::sway_chip(&r)));
    assert!(has_label(
        h,
        &format!("Wind at the top: {}", tower::v_top_label(&r))
    ));
    for b in tower::compare(input) {
        assert!(has_label(
            h,
            &format!("{}: {}", b.shape.def().name, b.label())
        ));
    }
    for g in tower::goals(input, &r) {
        assert_eq!(goal_done(h, g.text), g.done, "goal {:?}", g.text);
    }
    assert!(has_label(h, &tower::explain(input, &r)));
    assert!(has_label(h, &tower::stage_label(input, &r)));
}

/// TW-6 / TW-7 / NAV-1 / PROV-1: the defaults show the mockup's numbers.
#[test]
fn defaults_show_the_mockup_numbers() {
    let h = harness(Route::Tower);
    assert_eq!(document(&h), "tower-study");
    assert_eq!(drive_label(&h).as_deref(), Some("Play gusts"));
    assert!(has_label(&h, "T = 9.9 s"));
    for label in [
        "Stack it up, then let the storm in",
        "60 floors · 240 m",
        "Top sway: 928 mm, drift H/259 · aim H/500",
        "Sway period: 9.9 s, 0.101 Hz",
        "Base shear: 19.0 MN, total wind push",
        "Overturning: 2.64 GN·m, goes to the foundation",
        "Vortex / sway: 1.69×, clear of lock-in",
        "Cost: $60M, game money",
        "Occupant comfort: 9.5 milli-g",
        "Some people on the top floors notice.",
        "Straight: 9.5 mg",
        "Tapered: 6.6 mg",
        "Twisted: 7.8 mg",
        "Hollow core: 7.4 mg",
        "Status: Too much sway",
        "top 928 mm · H/259",
        "Wind at the top: 57 m/s · 127 mph",
        "Storm · wind at 10 m: 30 m/s · 67 mph",
        "Show sway: ×40",
        "sway drawn ×40",
        "Wind + sway: simplified cantilever",
        "Comfort bands: illustrative",
        "Game costs",
        "Challenge · 100 floors 7 / 12",
        "60-floor straight tower with a core system in 30 m/s wind. Too much sway.",
    ] {
        assert!(has_label(&h, label), "missing {label:?}");
    }
    assert_eq!(pressed(&h, "Core"), Some(true));
    assert_eq!(pressed(&h, "Tube"), Some(false));
    assert_eq!(pressed(&h, "Straight"), Some(true));
    assert!(!goal_done(&h, "Reach 100 floors"));
    assert!(!goal_done(&h, "Drift better than H/500 at 30 m/s"));
    assert!(goal_done(&h, "Top floor under 10 milli-g"));
    assert!(goal_done(&h, "Under $160M (game money)"));
    let damper = get(&h, "Tuned mass damper at the top", Role::CheckBox);
    assert_eq!(
        damper.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    expect_engine_labels(&h, &TowerInput::default());
}

/// TW-1: the stepper and slider move the floors (5–120), and the readouts follow.
#[test]
fn floors_stepper_and_slider() {
    let mut h = harness(Route::Tower);
    click_button(&mut h, "Add a floor");
    assert_eq!(tower_input(&h).floors, 61);
    assert!(has_label(&h, "61 floors · 244 m"));
    click_button(&mut h, "Remove a floor");
    click_button(&mut h, "Remove a floor");
    assert_eq!(tower_input(&h).floors, 59);
    set_slider(&mut h, "Number of floors", 5.0);
    assert_eq!(tower_input(&h).floors, 5);
    click_button(&mut h, "Remove a floor");
    assert_eq!(tower_input(&h).floors, 5, "clamps at 5 (MB-3)");
    set_slider(&mut h, "Number of floors", 120.0);
    click_button(&mut h, "Add a floor");
    assert_eq!(tower_input(&h).floors, 120, "clamps at 120");
    expect_engine_labels(&h, &tower_input(&h));
    assert!(has_label(&h, "Status: Dangerously flexible"));
}

/// TW-1 / TW-7: systems and shapes change the readouts; a braced tube at 60 floors locks in.
#[test]
fn systems_shapes_and_lock_in() {
    let mut h = harness(Route::Tower);
    click_button(&mut h, "Braced tube");
    assert_eq!(pressed(&h, "Braced tube"), Some(true));
    assert_eq!(pressed(&h, "Core"), Some(false));
    assert_eq!(tower_input(&h).system, System::Braced);
    assert!(has_label(&h, "Status: Vortex lock-in"));
    assert!(has_label_containing(&h, "lock-in zone 0.75–1.25"));
    assert!(has_label_containing(&h, "Wind peels off the corners"));
    expect_engine_labels(&h, &tower_input(&h));

    for (shape, name) in [
        (Shape::Tapered, "Tapered"),
        (Shape::Twisted, "Twisted"),
        (Shape::Hollow, "Hollow core"),
    ] {
        click_button(&mut h, name);
        assert_eq!(tower_input(&h).shape, shape);
        assert_eq!(pressed(&h, name), Some(true));
        expect_engine_labels(&h, &tower_input(&h));
    }
    for name in ["Tube", "Outriggers", "Core"] {
        click_button(&mut h, name);
        expect_engine_labels(&h, &tower_input(&h));
    }
}

/// TW-1 / TW-5: the damper and the storm change the readouts and goals.
#[test]
fn damper_and_storm() {
    let mut h = harness(Route::Tower);
    click(&mut h, "Tuned mass damper at the top");
    assert!(tower_input(&h).damper);
    assert!(has_label(&h, "Cost: $66M, game money"));
    expect_engine_labels(&h, &tower_input(&h));
    set_slider(&mut h, "Storm · wind at 10 m", 12.0);
    assert_eq!(tower_input(&h).v10, 12.0);
    assert!(has_label(&h, "Storm · wind at 10 m: 12 m/s · 27 mph"));
    // below 30 m/s the storm goals can't count
    assert!(!goal_done(&h, "Top floor under 10 milli-g"));
    expect_engine_labels(&h, &tower_input(&h));
    set_slider(&mut h, "Show sway", 100.0);
    assert!(has_label(&h, "sway drawn ×100"));
    assert!(has_label(&h, "Show sway: ×100"));
}

/// TW-4 / PLAT-3: Play gusts runs the dynamics (the sway moves, the header says Pause), Pause
/// freezes it, nothing animates while idle; Reset restores the mockup state.
#[test]
fn play_gusts_animates_and_pauses() {
    let mut h = harness(Route::Tower);
    assert!(h.state().screens().tower.sim.is_none());
    click_drive(&mut h);
    assert_eq!(drive_label(&h).as_deref(), Some("Pause"));
    let t0 = h.state().screens().tower.sim.as_ref().unwrap().time();
    let mut tops = Vec::new();
    for _ in 0..40 {
        h.step();
        tops.push(h.state().screens().tower.sim.as_ref().unwrap().top());
    }
    let sim_t = h.state().screens().tower.sim.as_ref().unwrap().time();
    assert!(sim_t > t0, "the simulation advances");
    let spread = tops.iter().cloned().fold(f64::MIN, f64::max)
        - tops.iter().cloned().fold(f64::MAX, f64::min);
    assert!(spread > 0.0, "the top sways");
    // the clock is the period, not a timer
    assert!(has_label(&h, "T = 9.9 s"));
    click_drive(&mut h);
    assert_eq!(drive_label(&h).as_deref(), Some("Play gusts"));
    let paused_at = h.state().screens().tower.sim.as_ref().unwrap().time();
    h.run_steps(5);
    assert_eq!(
        h.state().screens().tower.sim.as_ref().unwrap().time(),
        paused_at,
        "paused: frozen"
    );
    // Space toggles it too (NAV-5)
    press(&mut h, egui::Key::Space);
    assert_eq!(drive_label(&h).as_deref(), Some("Pause"));
    // changing the tower while playing retunes the simulation (it keeps running)
    click_button(&mut h, "Add a floor");
    assert!(h.state().screens().tower.sim.is_some());
    click_header(&mut h, "Reset");
    assert_eq!(drive_label(&h).as_deref(), Some("Play gusts"));
    assert!(h.state().screens().tower.sim.is_none());
    assert_eq!(tower_input(&h), TowerInput::default());
    assert!(has_label(&h, "top 928 mm · H/259"));
}

/// NAV-6: Share copies a one-line `.story` summary.
#[test]
fn share_copies_a_story_line() {
    let mut h = harness(Route::Tower);
    click_header(&mut h, "Share");
    let copied = last_copied(&h).expect("copied");
    assert!(copied.starts_with(".story 60 4 1600t core shape=straight damper=off"));
    assert!(!copied.contains('\n'));
    assert!(toasts(&h).iter().any(|t| t.contains(".story")));
}

/// NAV-4: "Send base reactions to Ground ›" opens Ground with the building; "‹ Back to the
/// Tower" returns with its floors.
#[test]
fn hand_off_to_ground_and_back() {
    let mut h = harness(Route::Tower);
    set_slider(&mut h, "Number of floors", 40.0);
    let r = tower::calc(&tower_input(&h));
    click_link(&mut h, "Send base reactions to Ground ›");
    settle(&mut h);
    assert_eq!(route(&h), Route::Ground);
    assert!(
        carried(&h)
            .iter()
            .any(|(to, c)| *to == Route::Ground && matches!(c, Carry::Building { floors: 40, .. }))
    );
    assert_eq!(h.state().screens().ground.input.floors, 40);
    let note = sp_engine::ground::FromTower {
        floors: 40,
        overturning: r.overturning,
    }
    .note();
    assert!(has_label(&h, &note), "missing {note:?}");
    assert!(has_label(&h, "Building: 40 floors"));
    click_link(&mut h, "‹ Back to the Tower");
    settle(&mut h);
    assert_eq!(route(&h), Route::Tower);
    assert_eq!(tower_input(&h).floors, 40);
}

/// DONE-4 (tower half): 100 floors with a braced tube and the damper.
#[test]
fn a_100_floor_braced_tube_with_a_damper() {
    let mut h = harness(Route::Tower);
    set_slider(&mut h, "Number of floors", 99.0);
    click_button(&mut h, "Add a floor");
    click_button(&mut h, "Braced tube");
    click_button(&mut h, "Tapered");
    click(&mut h, "Tuned mass damper at the top");
    let input = tower_input(&h);
    assert_eq!(
        (input.floors, input.system, input.shape, input.damper),
        (100, System::Braced, Shape::Tapered, true)
    );
    assert!(goal_done(&h, "Reach 100 floors"));
    assert!(goal_done(&h, "Top floor under 10 milli-g"));
    assert!(goal_done(&h, "Under $160M (game money)"));
    expect_engine_labels(&h, &input);
    // and it plays
    click_drive(&mut h);
    h.run_steps(10);
    assert!(h.state().screens().tower.sim.as_ref().unwrap().has_damper());
}

/// LOOK-1: renders at 1440 and 390 and stays close to the 1440 reference in its stable
/// regions (left column, readouts, the stage's ground and tower area).
#[test]
fn renders_close_to_the_reference() {
    let mut h = harness(Route::Tower);
    let shot = screenshot(&mut h, "tower-1440");
    let img = load(&shot);
    let mock = load(&mockup_png(Route::Tower, 1440));
    let regions = [
        ("parts", (0, 106, 249, 600)),
        ("readouts", (1141, 106, 299, 540)),
        ("title", (266, 120, 860, 56)),
        ("stage", (266, 181, 858, 590)),
        ("bottom bar", (266, 785, 858, 72)),
    ];
    for (name, region) in regions {
        let d = aligned_diff(&img, &mock, region);
        println!("{name}: mean |Δ| {d:.2}");
        assert!(d < 9.0, "{name}: mean |Δ| {d:.2}");
    }

    let mut h = app().at(Route::Tower).phone().build();
    let need = h.state().content_height().ceil();
    h.set_size(egui::vec2(PHONE_W, need.max(PHONE_H)));
    settle(&mut h);
    let img = load(&screenshot(&mut h, "tower-390"));
    assert_eq!(img.width(), PHONE_W as u32);
    // stacked in the mockup's order: parts, stage, readouts
    let y = |label: &str| h.get_by_label(label).rect().top();
    assert!(y("Number of floors") < y("Stack it up, then let the storm in"));
    assert!(y("Stack it up, then let the storm in") < y("Cost: $60M, game money"));
}
