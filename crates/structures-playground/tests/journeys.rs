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
//! Running now: 1, 2, 9 (+ 9b), 10 (+ 10b). Journeys that need features still landing in v2
//! wave 1 (3–8: the Sandbox's cable-stayed run, the fold-ins) are `#[ignore]`d with the reason;
//! they are completed in wave 2 (PROGRESS "v2 — the builder"). Click positions on the stage
//! come from an engine `Game` in the same level and view (the app's camera isn't read).

mod common;

use common::*;
use eframe::egui::{self, Key, Modifiers, PointerButton, Pos2, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::builder::{Game, LevelKey, PieceType};
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
// The builder's stage, driven by AccessKit labels and grid clicks

/// The stage's drawing box: the grid surface button ("Building grid") inside its 1 px border.
fn stage_inner(h: &App) -> egui::Rect {
    h.root()
        .children_recursive()
        .find(|n| {
            n.accesskit_node().role() == Role::Button
                && n.accesskit_node().label().as_deref() == Some("Building grid")
        })
        .expect("the grid surface")
        .rect()
        .shrink(1.0)
}

/// Clicks the stage at world (x, y) m. `cam` is an engine game in the same level and view
/// (the app's camera is not read from its state).
fn click_grid(h: &mut App, cam: &Game, x: f64, y: f64) {
    let r = stage_inner(h);
    let s = r.width() / 1120.0;
    let c = cam.camera();
    let p: Pos2 = r.min + egui::vec2(c.x(x) as f32 * s, c.y(y) as f32 * s);
    h.hover_at(p);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(2);
}

/// Clicks joint `id`'s button ("Joint J3, on a pile, …").
fn click_joint(h: &mut App, id: &str) {
    let head = format!("Joint {id}");
    let label = labels(h)
        .into_iter()
        .find(|l| *l == head || l.starts_with(&format!("{head},")))
        .unwrap_or_else(|| panic!("no button for {head}"));
    h.get_by_role_and_label(Role::Button, &label).click();
    h.run_steps(2);
}

/// Clicks the button `label` inside the group `group` (scrolled into view: the aside scrolls).
fn click_in(h: &mut App, group: &str, label: &str) {
    for _ in 0..4 {
        let node = h
            .get_by_role_and_label(Role::Group, group)
            .get_by_role_and_label(Role::Button, label);
        if viewport(h).contains_rect(node.rect()) {
            break;
        }
        node.scroll_to_me();
        settle(h);
    }
    h.get_by_role_and_label(Role::Group, group)
        .get_by_role_and_label(Role::Button, label)
        .click();
    settle(h);
}

/// The stage summary ("Bridge with 10 joints and 17 pieces. Stands up.").
fn stage_label(h: &App) -> String {
    h.get_by_role(Role::Image)
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

/// A Road deck J1 → 16, 24, 32, 40 → J2 under a truss of `chord` (top chord at 6 m, verticals
/// and diagonals), built by clicking — and the same clicks on the engine `g`.
fn build_truss(h: &mut App, g: &mut Game, chord: &str, kind: PieceType) {
    click_in(h, "Piece to build", "Road");
    g.pick_type(PieceType::Road);
    click_joint(h, "J1");
    g.click_node("J1");
    for x in [16.0, 24.0, 32.0, 40.0] {
        click_grid(h, g, x, 0.0);
        g.click_at([x, 0.0]);
    }
    click_joint(h, "J2");
    g.click_node("J2");
    click_button(h, "Finish line");
    g.finish_line();
    click_in(h, "Piece to build", chord);
    g.pick_type(kind);
    click_joint(h, "J1");
    g.click_node("J1");
    for x in [16.0, 24.0, 32.0, 40.0] {
        click_grid(h, g, x, 6.0);
        g.click_at([x, 6.0]);
    }
    click_joint(h, "J2");
    g.click_node("J2");
    click_button(h, "Finish line");
    g.finish_line();
    for chain in [
        &["J3", "J7", "J4", "J8", "J5", "J9"][..],
        &["J6", "J10", "J5"][..],
    ] {
        for id in chain {
            click_joint(h, id);
            g.click_node(id);
        }
        click_button(h, "Finish line");
        g.finish_line();
    }
}

/// Steps the app until the result banner shows (its "Back to building" button), up to `max`.
fn wait_for_banner(h: &mut App, max: usize) -> bool {
    step_until(h, max, |h| {
        h.query_all_by_label("Replay in slow motion")
            .next()
            .is_some()
    })
}

// ============================================================================================
// 1–8: the builder and its fold-ins

/// DONE-4: level 1 built from scratch by clicks — a Road deck and Wood triangles — runs, every
/// car crosses and the banner gives the stars the engine gives the same design.
#[test]
fn journey_1_level_one_by_clicks_all_across_with_stars() {
    let mut h = harness(Route::Playground);
    assert!(header_has(&h, "river-level"));
    let mut g = Game::new();
    build_truss(&mut h, &mut g, "Wood", PieceType::Wood);
    let (n, m) = (g.design().nodes.len(), g.design().members.len());
    assert!(
        stage_label(&h).starts_with(&format!("Bridge with {n} joints and {m} pieces.")),
        "{}",
        stage_label(&h)
    );
    let status = g.view().status.label;
    assert!(stage_label(&h).ends_with(&format!("{status}.")), "{status}");
    // the expected run, by the engine alone
    g.run_test(sp_shared::proto::MAX_RUN_FRAMES);
    let want = g.view().result.expect("a banner");
    assert!(want.pass, "the wood truss gets the cars across: {want:?}");
    // run it from the header
    assert_eq!(drive(&h), "Run traffic");
    click_header(&mut h, "Run traffic");
    assert_eq!(drive(&h), "Pause");
    assert!(
        header_labels(&h).iter().any(|l| l.starts_with("t = ")),
        "the clock runs"
    );
    assert!(wait_for_banner(&mut h, 3000), "the run ends");
    assert!(has_label(&h, &want.title), "banner {:?}", want.title);
    assert!(has_label(&h, &want.stars), "stars {:?}", want.stars);
    // the level row remembers the stars
    let stars = g.result().expect("a result").stars;
    let row = h.get_by_role_and_label(Role::Button, "1 · First crossing");
    assert_eq!(
        row.accesskit_node().description().as_deref(),
        Some(format!("Best: {stars} of 3 stars").as_str())
    );
    click_in(&mut h, "Goal", "Back to building");
    assert_eq!(drive(&h), "Run traffic");
    assert!(header_has(&h, "building"));
}

/// DONE-4: Heavy haul in a wood truss collapses under the 30 t truck and the snow; the banner
/// and its sentence are the engine's for the same design; "Replay in slow motion" runs it again
/// slowly to the same banner; "Back to building".
#[test]
fn journey_2_heavy_haul_collapses_and_replays() {
    let mut h = harness(Route::Playground);
    click_button(&mut h, "3 · Heavy haul");
    assert!(header_has(&h, "haul-level"));
    let mut g = Game::new();
    g.pick_level(LevelKey::Haul);
    build_truss(&mut h, &mut g, "Wood", PieceType::Wood);
    g.run_test(sp_shared::proto::MAX_RUN_FRAMES);
    let r = g.result().expect("a result").clone();
    assert!(
        !r.pass,
        "a wood truss can't carry the truck in the snow: {r:?}"
    );
    let want = g.view();
    let banner = want.result.as_ref().expect("a banner");
    press(&mut h, Key::Space);
    assert!(wait_for_banner(&mut h, 3000), "the run ends");
    assert!(has_label(&h, &banner.title), "banner {:?}", banner.title);
    assert!(
        has_label(&h, &want.explain),
        "the sentence {:?} in {:?}",
        want.explain,
        labels(&h)
            .into_iter()
            .filter(|l| l.len() > 40)
            .collect::<Vec<_>>()
    );
    assert!(
        r.first.is_some() || r.first_found.is_some(),
        "something gave way: {r:?}"
    );
    // replay in slow motion: running again, slowly, then the same banner
    click_button(&mut h, "Replay in slow motion");
    assert_eq!(drive(&h), "Pause");
    assert!(has_label(&h, "Normal speed"), "slow motion is on");
    assert!(wait_for_banner(&mut h, 6000), "the replay ends");
    assert!(has_label(&h, &banner.title));
    click_in(&mut h, "Goal", "Back to building");
    assert!(header_has(&h, "building"));
}

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

/// DONE-4 / NAV-4: "Open in Advanced" shows the builder's structure as a netlist — every joint
/// a node, every piece a member (a row in the results), the level as the document — and
/// "Open in Playground" brings it back into the Sandbox with the same counts.
#[test]
fn journey_9b_open_in_advanced_shows_the_same_structure() {
    let mut h = harness(Route::Playground);
    let mut g = Game::new();
    build_truss(&mut h, &mut g, "Steel", PieceType::Steel);
    let (n, m) = (g.design().nodes.len(), g.design().members.len());
    reveal(&mut h, "Open in Advanced", Some(Role::Button));
    click_button(&mut h, "Open in Advanced");
    settle(&mut h);
    assert_eq!(selected(&h, "Advanced", Role::Link), Some(true));
    assert!(header_has(&h, "river-level"), "{:?}", header_labels(&h));
    assert!(has_label(&h, "from the Playground"));
    let rows = labels(&h)
        .into_iter()
        .filter(|l| l.contains(": N ") && l.contains(" kN, σ "))
        .count();
    assert_eq!(rows, m, "a member row per piece");
    // the netlist Advanced holds is the builder's Share of the same design
    click_header(&mut h, "Share");
    let text = last_copied(&h).expect("Share copies the netlist");
    assert_eq!(text.lines().filter(|l| l.starts_with(".node ")).count(), n);
    assert_eq!(
        text.lines().filter(|l| l.starts_with(".member ")).count(),
        m
    );
    // and back
    reveal(&mut h, "Open in Playground", Some(Role::Button));
    click_button(&mut h, "Open in Playground");
    settle(&mut h);
    assert_eq!(selected(&h, "Playground", Role::Link), Some(true));
    assert!(header_has(&h, "sandbox"));
    assert!(
        stage_label(&h).starts_with(&format!("Bridge with {n} joints and {m} pieces.")),
        "{}",
        stage_label(&h)
    );
}

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

/// DONE-4 / NAV-5: building and zooming with the keyboard alone — Tab to the grid surface,
/// Enter drops a joint at the view's centre, the arrows pan, Enter again chains a piece; ⌘= /
/// ⌘− / ⌘0 zoom as the engine's buttons do; Esc ends the chain.
#[test]
fn journey_10b_keyboard_only_build_and_zoom() {
    let mut h = harness(Route::Playground);
    h.remove_cursor();
    let zoom_label = |h: &App| {
        labels(h)
            .into_iter()
            .find(|l| l.starts_with("Zoom ") && l.ends_with('%'))
            .expect("the zoom readout")
    };
    let mut g = Game::new();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    // zoom by keys, checked against the engine's own zoom steps
    press_cmd(&mut h, Key::Equals);
    settle(&mut h);
    g.zoom_in();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    press_cmd(&mut h, Key::Minus);
    press_cmd(&mut h, Key::Minus);
    settle(&mut h);
    g.zoom_out();
    g.zoom_out();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    press_cmd(&mut h, Key::Num0);
    settle(&mut h);
    g.fit();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    // Tab to the grid surface
    let mut on_grid = false;
    for _ in 0..400 {
        press(&mut h, Key::Tab);
        if focused_label(&h).as_deref() == Some("Building grid") {
            on_grid = true;
            break;
        }
    }
    assert!(on_grid, "Tab reaches the building grid");
    let joints = |h: &App| {
        let l = stage_label(h);
        l.split(" joints")
            .next()
            .and_then(|a| a.rsplit(' ').next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0)
    };
    let pieces = |h: &App| {
        let l = stage_label(h);
        l.split(" pieces")
            .next()
            .and_then(|a| a.rsplit(' ').next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0)
    };
    let (j0, p0) = (joints(&h), pieces(&h));
    // Enter drops a joint at the centre (the Build tool is the starting tool)
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(joints(&h), j0 + 1, "{}", stage_label(&h));
    // pan right with the arrows (the grid keeps focus), Enter again: a piece to the new joint
    for _ in 0..3 {
        press(&mut h, Key::ArrowRight);
    }
    settle(&mut h);
    assert_eq!(
        focused_label(&h).as_deref(),
        Some("Building grid"),
        "the grid keeps focus"
    );
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(joints(&h), j0 + 2, "{}", stage_label(&h));
    assert_eq!(pieces(&h), p0 + 1, "{}", stage_label(&h));
    // Esc ends the chain: the next Enter starts a new one (a joint, no piece)
    press(&mut h, Key::Escape);
    for _ in 0..3 {
        press(&mut h, Key::ArrowUp);
    }
    settle(&mut h);
    if focused_label(&h).as_deref() != Some("Building grid") {
        // Esc may have surrendered focus: Tab back to the grid
        for _ in 0..400 {
            press(&mut h, Key::Tab);
            if focused_label(&h).as_deref() == Some("Building grid") {
                break;
            }
        }
    }
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(joints(&h), j0 + 3, "{}", stage_label(&h));
    assert_eq!(
        pieces(&h),
        p0 + 1,
        "no piece after Esc: {}",
        stage_label(&h)
    );
}
