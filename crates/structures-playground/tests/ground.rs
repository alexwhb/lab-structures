//! SCR-4 Ground in the real app (egui_kittest, AccessKit labels): GD-1 … GD-5, NAV-1, NAV-4,
//! NAV-6, PROV-1, PLAT-3, DONE-2, DONE-4 (the Pisa half), LOOK-1.

mod common;

use common::*;
use eframe::egui::{self, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::ground::{self, Foundation, GroundInput, Soil};
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

fn ground_input(h: &App) -> GroundInput {
    h.state().screens().ground.input
}

fn goal_done(h: &App, text: &str) -> bool {
    get(h, text, Role::CheckBox).accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

/// What the engine says the screen shows for `input` (DONE-2).
fn expect_engine_labels(h: &App, input: &GroundInput) {
    let r = ground::calc(input);
    for tl in ground::tiles(input, &r) {
        let label = format!("{}: {}, {}", tl.key, tl.value, tl.note);
        assert!(has_label(h, &label), "missing tile {label:?}");
    }
    assert!(has_label(
        h,
        &format!("Safety factor: {}", ground::fs_text(&r))
    ));
    assert!(has_label(
        h,
        &format!("Status: {}", ground::Status::of(&r).label())
    ));
    assert!(has_label(h, &ground::stage_chip(&r)));
    assert!(has_label(h, ground::explain(input, &r)));
    assert!(has_label(h, &ground::stage_label(input, &r)));
    assert!(has_label(h, &ground::clock(input.years)));
    for g in ground::goals(input) {
        assert_eq!(goal_done(h, g.text), g.done, "goal {:?}", g.text);
    }
}

/// GD-5 / NAV-1 / PROV-1: the defaults show the mockup's numbers.
#[test]
fn defaults_show_the_mockup_numbers() {
    let h = harness(Route::Ground);
    assert_eq!(document(&h), "foundation-pit");
    assert_eq!(drive_label(&h).as_deref(), Some("Play years"));
    for label in [
        "What’s holding it up",
        "year 10.0",
        "Pressure: 72 kPa, on the bearing area",
        "Capacity: 60 kPa, allowable, illustrative",
        "Settlement: 268 mm, heads to 400 mm",
        "Tilt: 2.37°, Pisa today ≈ 4°",
        "Building load: 115 MN, 6 floors × 12 kPa",
        "Consolidation: 9 yr, time to ~63%",
        "Safety factor: 0.83 · target 3",
        "Status: Bearing failure",
        "tilt 2.37° · sank 268 mm",
        "Years after construction: 10.0 yr",
        "Building: 6 floors",
        "Bearing + settlement: simplified",
        "Soil presets: illustrative",
        "Game costs",
        "Challenge · The lean of Pisa 10 / 12",
        "Legend: Pressure bulb under the foundation",
        "6-floor building on mat slab in soft clay. Bearing failure. Tilt 2.37 degrees.",
    ] {
        assert!(has_label(&h, label), "missing {label:?}");
    }
    assert_eq!(pressed(&h, "Soft clay"), Some(true));
    assert_eq!(pressed(&h, "Mat slab"), Some(true));
    assert!(!has_label(&h, "Pile length"), "pile length only for piles");
    expect_engine_labels(&h, &GroundInput::default());
}

/// GD-1: soils, foundations (pile length for piles only), floors and the quake change the
/// readouts.
#[test]
fn soils_foundations_floors_and_quake() {
    let mut h = harness(Route::Ground);
    for (soil, name) in [
        (Soil::Dense, "Dense sand"),
        (Soil::Stiff, "Stiff clay"),
        (Soil::Loose, "Loose wet sand"),
        (Soil::Soft, "Soft clay"),
    ] {
        click_button(&mut h, name);
        assert_eq!(ground_input(&h).soil, soil);
        assert_eq!(pressed(&h, name), Some(true));
        expect_engine_labels(&h, &ground_input(&h));
    }
    for (f, name) in [
        (Foundation::Footing, "Footings"),
        (Foundation::Caisson, "Caissons"),
        (Foundation::Piles, "Piles"),
    ] {
        click_button(&mut h, name);
        assert_eq!(ground_input(&h).foundation, f);
        expect_engine_labels(&h, &ground_input(&h));
    }
    assert!(has_label(&h, "Pile length: 34 m · on rock"));
    set_slider(&mut h, "Pile length", 20.0);
    assert_eq!(ground_input(&h).pile_len, 20.0);
    assert!(has_label(&h, "Pile length: 20 m"));
    assert!(has_label_containing(&h, "friction only"));
    expect_engine_labels(&h, &ground_input(&h));
    set_slider(&mut h, "Building", 40.0);
    assert_eq!(ground_input(&h).floors, 40);
    expect_engine_labels(&h, &ground_input(&h));
    // the quake liquefies loose wet sand under a mat
    click_button(&mut h, "Mat slab");
    click_button(&mut h, "Loose wet sand");
    click(&mut h, "Earthquake shaking");
    assert!(ground_input(&h).quake);
    assert!(has_label(&h, "Status: Liquefaction"));
    assert!(has_label_containing(&h, "liquefied!"));
    assert!(has_label(
        &h,
        "Legend: Pressure bulb under the foundation, Water table"
    ));
    expect_engine_labels(&h, &ground_input(&h));
}

/// GD-1 / GD-4: each scenario loads its settings at year 0 and plays; Play years runs to 30
/// and stops; the years slider scrubs (and pauses); Play again from 30 restarts at 0.
#[test]
fn scenarios_and_play_years() {
    let mut h = harness(Route::Ground);
    click_button(&mut h, "Liquefaction (wet sand + quake)");
    let i = ground_input(&h);
    assert_eq!(
        (i.soil, i.foundation, i.floors, i.quake),
        (Soil::Loose, Foundation::Mat, 12, true)
    );
    assert_eq!(drive_label(&h).as_deref(), Some("Pause"));
    assert!(step_until(&mut h, 200, |h| ground_input(h).years > 0.5));
    assert!(has_label(&h, "Status: Liquefaction"));

    click_button(&mut h, "60 floors on piles to rock");
    let i = ground_input(&h);
    assert_eq!(
        (i.soil, i.foundation, i.floors, i.pile_len),
        (Soil::Stiff, Foundation::Piles, 60, 34.0)
    );
    click_button(&mut h, "Friction piles in soft clay");
    let i = ground_input(&h);
    assert_eq!(
        (i.soil, i.foundation, i.floors, i.pile_len),
        (Soil::Soft, Foundation::Piles, 10, 20.0)
    );
    // scrub: pauses at the chosen year
    set_slider(&mut h, "Years after construction", 29.5);
    assert_eq!(drive_label(&h).as_deref(), Some("Play years"));
    assert_eq!(ground_input(&h).years, 29.5);
    assert!(has_label(&h, "year 29.5"));
    // play to the end: it stops at 30
    click_drive(&mut h);
    assert!(step_until(&mut h, 400, |h| !h
        .state()
        .screens()
        .ground
        .playing));
    assert_eq!(ground_input(&h).years, 30.0);
    assert_eq!(drive_label(&h).as_deref(), Some("Play years"));
    // from 30 it restarts at 0
    click_drive(&mut h);
    assert!(ground_input(&h).years < 10.0, "restarted from 0");
    click_drive(&mut h);
    click_header(&mut h, "Reset");
    assert_eq!(ground_input(&h), GroundInput::default());
}

/// DONE-4 (Pisa half): load the leaning tower and fix it with piles to rock without removing
/// floors: all three goals light.
#[test]
fn the_pisa_challenge_is_completable() {
    let mut h = harness(Route::Ground);
    assert!(!goal_done(&h, "Start from the leaning-tower scenario"));
    click_button(&mut h, "Leaning tower (soft clay, mat)");
    assert!(goal_done(&h, "Start from the leaning-tower scenario"));
    assert!(goal_done(&h, "Still 6 floors on soft clay"));
    assert!(!goal_done(&h, "Tilt under 0.5° at 30 years"));
    click_button(&mut h, "Piles");
    set_slider(&mut h, "Pile length", 34.0);
    assert!(goal_done(&h, "Tilt under 0.5° at 30 years"));
    assert!(goal_done(&h, "Still 6 floors on soft clay"));
    expect_engine_labels(&h, &ground_input(&h));
}

/// NAV-4: a building from the Tower sets the floors and shows the moment as a note; floors
/// beyond 80 are clamped here and go back unchanged.
#[test]
fn building_from_the_tower() {
    let mut h = harness(Route::Ground);
    hand_off(
        &mut h,
        Route::Ground,
        Carry::Building {
            floors: 100,
            base_shear_n: 3.0e7,
            overturning_nm: 5.0e9,
        },
    );
    settle(&mut h);
    assert_eq!(ground_input(&h).floors, 80);
    assert!(has_label_containing(&h, "From the Tower: 100 floors"));
    assert!(has_label_containing(&h, "5.00 GN·m"));
    click_link(&mut h, "‹ Back to the Tower");
    settle(&mut h);
    assert_eq!(route(&h), Route::Tower);
    assert_eq!(h.state().screens().tower.input.floors, 100);
}

/// NAV-6: Share copies a one-line `.soil` summary.
#[test]
fn share_copies_a_soil_line() {
    let mut h = harness(Route::Ground);
    click_header(&mut h, "Share");
    let copied = last_copied(&h).expect("copied");
    assert!(copied.starts_with(".soil 1 30 soft-clay  .foundation mat"));
    assert!(!copied.contains('\n'));
}

/// LOOK-1: renders at 1440 and 390 and stays close to the 1440 reference.
#[test]
fn renders_close_to_the_reference() {
    let mut h = harness(Route::Ground);
    let shot = screenshot(&mut h, "ground-1440");
    let img = load(&shot);
    let mock = load(&mockup_png(Route::Ground, 1440));
    let regions = [
        ("parts", (0, 106, 249, 780)),
        ("readouts", (1141, 106, 299, 560)),
        ("title", (266, 120, 860, 56)),
        ("stage", (266, 181, 858, 570)),
        ("bottom bar", (266, 766, 858, 72)),
    ];
    for (name, region) in regions {
        let d = aligned_diff(&img, &mock, region);
        println!("{name}: mean |Δ| {d:.2}");
        assert!(d < 9.0, "{name}: mean |Δ| {d:.2}");
    }

    let mut h = app().at(Route::Ground).phone().build();
    let need = h.state().content_height().ceil();
    h.set_size(egui::vec2(PHONE_W, need.max(PHONE_H)));
    settle(&mut h);
    let img = load(&screenshot(&mut h, "ground-390"));
    assert_eq!(img.width(), PHONE_W as u32);
    let y = |label: &str| h.get_by_label(label).rect().top();
    assert!(y("Building") < y("What’s holding it up"));
    assert!(y("What’s holding it up") < y("Consolidation: 9 yr, time to ~63%"));
}
