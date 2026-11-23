//! End-to-end journeys across the screens (SPEC DONE-4, v2), in the real app, driven and read
//! through AccessKit only: clicks on named buttons, links and grid points of the builder's
//! stage, keys, AccessKit slider values. Nothing reaches into a screen's state; expected
//! numbers come from `sp_engine` run independently (DONE-2 for the tests too).
//!
//! 1. Level 1 built by clicking (Road + Wood triangles) → Run traffic → all across, stars.
//! 2. Heavy haul in a wood truss collapses under the truck and snow → replay in slow motion.
//! 3. A cable-stayed Sandbox bridge with looping traffic and a wind change mid-run.
//! 4. A tower built to 64 m rides out the storm: comfort meter, "Compare shapes" (CMF).
//! 5. Shaky ground: footings liquefy, piles fix it.
//! 6. Leaning tower fixed with piles and a counterweight over 30 years (TIME).
//! 7. A material card's sample pulled to breaking (LAB).
//! 8. A fire softens a steel truss (FIRE).
//! 9. Open in Advanced shows the same structure; an error in the netlist, fixed, then "Open in
//!    Playground" loads it into the Sandbox.
//! 10. Keyboard only: ⌘1 / ⌘2, Space drives, Tab reaches the drive button, brand → Series →
//!     Esc; build and zoom by keyboard.
//!
//! Journeys that need features still landing in v2 wave 1 are `#[ignore]`d with the reason;
//! they are completed in wave 2 (PROGRESS "v2 — the builder").

mod common;

use common::*;
use eframe::egui::{Key, Modifiers, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use structures_playground::Route;

// ============================================================================================
// Reading through AccessKit

/// A node's text: its label, else its value (egui labels carry their text as the value).
fn text(n: &egui_kittest::Node<'_>) -> Option<String> {
    let a = n.accesskit_node();
    a.label().or_else(|| a.value())
}

/// Every node's text.
fn labels(h: &App) -> Vec<String> {
    h.root()
        .children_recursive()
        .filter_map(|n| text(&n))
        .collect()
}

/// The header's labels (the `Banner` node's subtree).
fn header_labels(h: &App) -> Vec<String> {
    h.get_by_role(Role::Banner)
        .children_recursive()
        .filter_map(|n| text(&n))
        .collect()
}

/// The header shows `text` (document name, clock, a button).
fn header_has(h: &App, text: &str) -> bool {
    header_labels(h).iter().any(|l| l == text)
}

/// The drive button's label, read from the header.
fn drive(h: &App) -> String {
    const DRIVES: [&str; 5] = [
        "Run traffic",
        "Run the test",
        "Stop",
        "Pause",
        "Drive across",
    ];
    header_labels(h)
        .into_iter()
        .find(|l| DRIVES.contains(&l.as_str()))
        .expect("a drive button in the header")
}

// ============================================================================================
// 1–8: the builder and its fold-ins

#[test]
#[ignore = "wave 2: needs the builder screen (agent `screen`) — build level 1 by clicks, run, all across with stars"]
fn journey_1_level_one_by_clicks_all_across_with_stars() {}

#[test]
#[ignore = "wave 2: needs the builder screen — Heavy haul in a wood truss collapses, replay in slow motion"]
fn journey_2_heavy_haul_collapses_and_replays() {}

#[test]
#[ignore = "wave 2: needs the builder screen — cable-stayed Sandbox, looping traffic, wind change mid-run"]
fn journey_3_cable_stayed_sandbox_in_changing_wind() {}

#[test]
#[ignore = "wave 2: needs the builder + CMF (agents `screen`, `time`) — 64 m tower, comfort meter, Compare shapes"]
fn journey_4_storm_tower_comfort_and_shapes() {}

#[test]
#[ignore = "wave 2: needs the builder screen — Shaky ground footings liquefy, piles fix it"]
fn journey_5_shaky_ground_piles_fix_liquefaction() {}

#[test]
#[ignore = "wave 2: needs TIME (agent `time`) — Leaning tower fixed with piles and a counterweight over 30 years"]
fn journey_6_leaning_tower_fixed_over_30_years() {}

#[test]
#[ignore = "wave 2: needs LAB (agent `lab`) — a material card's sample pulled to breaking"]
fn journey_7_sample_pulled_to_breaking() {}

#[test]
#[ignore = "wave 2: needs FIRE (agent `lab`) — a fire softens a steel truss"]
fn journey_8_fire_softens_a_steel_truss() {}

// ============================================================================================
// 9. Advanced: an error in the netlist, fixed, then "Open in Playground"

/// A small netlist on the builder's river grid: a 16 m Howe-like truss on the banks.
const TRUSS: &str = "* little-truss.str
.units SI kN m
.material steel E=200GPa fy=250MPa rho=7850
.section medium A=80cm2 I=A^2
.node B0 8 0 fix=xy
.node B1 16 0
.node B2 24 0 fix=y
.node T1 16 6
.member M1 B0 B1 medium
.member M2 B1 B2 medium
.member M3 B0 T1 medium
.member M4 T1 B2 medium
.member M5 B1 T1 medium
.load dead B1 Fy=-20t
.static linear
";

#[test]
fn journey_9_advanced_error_fix_and_open_in_playground() {
    let mut h = harness(Route::Advanced);
    let replace = |h: &mut App, t: &str| {
        get(h, "Netlist", Role::MultilineTextInput).focus();
        h.step();
        h.key_press_modifiers(Modifiers::COMMAND, Key::A);
        h.step();
        get(h, "Netlist", Role::MultilineTextInput).type_text(t);
        h.run_steps(4);
    };
    replace(&mut h, TRUSS);
    let rows = |h: &App| {
        labels(h)
            .into_iter()
            .filter(|l| l.contains(": N ") && l.contains(" kN, σ "))
            .count()
    };
    assert_eq!(rows(&h), 5, "{:?}", labels(&h));
    // break it: M3 names a joint that doesn't exist
    let line_no = TRUSS
        .lines()
        .position(|l| l.starts_with(".member M3 "))
        .unwrap()
        + 1;
    replace(
        &mut h,
        &TRUSS.replace(".member M3 B0 T1", ".member M3 B0 T9"),
    );
    let err = format!("Line {line_no}: unknown node T9");
    assert!(has_label(&h, &err), "missing {err:?}");
    assert!(has_label(&h, "1 error"));
    assert_eq!(rows(&h), 5, "the last good result stays");
    assert!(
        get(&h, "Open in Playground", Role::Button)
            .accesskit_node()
            .is_disabled()
    );
    // fix it, then hand it to the builder
    replace(&mut h, TRUSS);
    assert!(!has_label(&h, "1 error"));
    reveal(&mut h, "Open in Playground", Some(Role::Button));
    click_button(&mut h, "Open in Playground");
    settle(&mut h);
    assert_eq!(selected(&h, "Playground", Role::Link), Some(true));
    // TODO(wave 2): the builder shows it in the Sandbox (header `sandbox`, 4 joints, 5 pieces,
    // "Stands up"), and its "Open in Advanced" brings back the same forces.
}

#[test]
#[ignore = "wave 2: needs the builder screen — Open in Advanced shows the same structure (forces match the builder's inspector)"]
fn journey_9b_open_in_advanced_shows_the_same_structure() {}

// ============================================================================================
// 10. Keyboard only

#[test]
fn journey_10_keyboard_only() {
    let mut h = harness(Route::Playground);
    // ⌘2 / ⌘1 switch screens
    press_cmd(&mut h, Key::Num2);
    settle(&mut h);
    assert_eq!(selected(&h, "Advanced", Role::Link), Some(true));
    assert_eq!(drive(&h), "Drive across");
    press_cmd(&mut h, Key::Num1);
    settle(&mut h);
    assert_eq!(selected(&h, "Playground", Role::Link), Some(true));
    assert!(header_has(&h, "river-level"));
    assert!(header_has(&h, "building"), "the builder's clock");
    // Space drives (nothing focused)
    h.remove_cursor();
    let run = drive(&h);
    assert_eq!(run, "Run traffic");
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_ne!(drive(&h), run, "Space started the run");
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(drive(&h), run, "Space stopped it");
    // Tab reaches the drive button; Enter presses it
    let mut reached = false;
    for _ in 0..40 {
        press(&mut h, Key::Tab);
        if focused_label(&h).as_deref() == Some(run.as_str()) {
            reached = true;
            break;
        }
    }
    assert!(reached, "Tab never reached the drive button");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_ne!(drive(&h), run, "Enter on the focused drive button");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(drive(&h), run);
    // the brand opens the Series screen; Esc comes back
    let mut on_brand = false;
    // (Tab walks the whole builder — chips, joints, levels — before wrapping to the header)
    for _ in 0..400 {
        press(&mut h, Key::Tab);
        if focused_label(&h).as_deref() == Some("Structures Playground") {
            on_brand = true;
            break;
        }
    }
    assert!(on_brand, "Tab never reached the brand");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert!(
        h.query_by_role(Role::Banner).is_none(),
        "Series has no header"
    );
    assert!(has_label(&h, "Back"));
    press(&mut h, Key::Escape);
    settle(&mut h);
    assert_eq!(
        selected(&h, "Playground", Role::Link),
        Some(true),
        "back where it was"
    );
}

#[test]
#[ignore = "wave 2: needs the builder screen — keyboard-only build (Build tool, grid by keys) and zoom (⌘= / ⌘− / ⌘0, arrows pan)"]
fn journey_10b_keyboard_only_build_and_zoom() {}
