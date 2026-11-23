//! End-to-end journeys across the screens (SPEC DONE-4), in the real app, driven and read
//! through AccessKit only: clicks on named buttons, links, tabs and grid points of the
//! Bridge's stage, keys, AccessKit slider values. Nothing reaches into a screen's state;
//! expected numbers come from `sp_engine` run independently (DONE-2 for the tests too).
//!
//! 1. Level 1 from scratch → Test → it holds, stars, best stars remembered on the level row.
//! 2. A thin timber bridge buckles → "Collapse: M6 buckled at 7%" → slow-motion replay → back.
//! 3. Open it in Advanced: every member's force and use match the Bridge inspector; Share in
//!    Advanced, paste on the Bridge → the same structure in the Sandbox.
//! 4. Edit the netlist to an error → the error on its line, the last result dimmed → fix it →
//!    "Open in Playground" → the Sandbox shows it.
//! 5. Tower: 100 floors, braced tapered tube, damper → the challenge's goals light → Ground
//!    with that building → fix the lean of Pisa (scenario, piles to rock, 30 years) → back.
//! 6. Materials: Pull all five samples to fracture → all five challenge chips lit.
//! 7. Keyboard only: ⌘1–⌘5, Space drives, Tab reaches the drive button, brand → Series → Esc.

mod common;

use common::*;
use eframe::egui::{self, Key, Modifiers, PointerButton, Pos2, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::bridge::data::{px, py, snap};
use sp_engine::bridge::{Game, MaterialKey, SizeKey, Tool};
use sp_engine::ground::{self, GroundInput};
use sp_engine::materials::{Bench, Kind};
use sp_engine::tower::{self, Shape, System, TowerInput};
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

/// The first label starting with `prefix`.
fn label_starting(h: &App, prefix: &str) -> Option<String> {
    labels(h).into_iter().find(|l| l.starts_with(prefix))
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
    const DRIVES: [&str; 6] = [
        "Test",
        "Pause",
        "Drive across",
        "Play gusts",
        "Play years",
        "Pull",
    ];
    header_labels(h)
        .into_iter()
        .find(|l| DRIVES.contains(&l.as_str()))
        .expect("a drive button in the header")
}

fn press_drive(h: &mut App) {
    let d = drive(h);
    click_header(h, &d);
}

fn checked(h: &App, label: &str) -> bool {
    get(h, label, Role::CheckBox).accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

fn slider(h: &mut App, label: &str, v: f64) {
    reveal(h, label, Some(Role::Slider));
    h.get_by_role_and_label(Role::Slider, label)
        .set_value_accesskit(v);
    h.run_steps(3);
}

// ============================================================================================
// The Bridge's stage

fn stage_rect(h: &App) -> egui::Rect {
    h.root()
        .children_recursive()
        .find(|n| {
            n.accesskit_node().role() == Role::Button
                && n.accesskit_node()
                    .label()
                    .is_some_and(|l| l.starts_with("Building grid"))
        })
        .expect("the grid surface")
        .rect()
}

/// Screen position of grid point (x, y) m, from the grid surface's bounds.
fn at(h: &App, x: f64, y: f64) -> Pos2 {
    let r = stage_rect(h);
    let s = r.width() / 1120.0;
    r.min + egui::vec2(px(x) as f32 * s, py(y) as f32 * s)
}

fn click_grid(h: &mut App, x: f64, y: f64) {
    let p = at(h, x, y);
    h.hover_at(p);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.run_steps(3);
}

/// The button whose label is `head` or starts with `head,` ("Joint J3, pin support").
fn labelled(h: &App, head: &str) -> String {
    labels(h)
        .into_iter()
        .find(|l| l == head || l.starts_with(&format!("{head},")))
        .unwrap_or_else(|| panic!("nothing labelled {head:?}"))
}

fn click_joint(h: &mut App, id: &str) {
    let l = labelled(h, &format!("Joint {id}"));
    h.get_by_role_and_label(Role::Button, &l).click();
    h.run_steps(3);
}

/// The 8-joint Howe-style truss over level 1's river, clicked joint by joint: bottom chord
/// 8-18-28-38-48, top chord 18 / 28 / 38 at y = 6, posts and diagonals.
const HOWE: &[&[Step]] = &[
    &[J("J1"), G(18, 0), J("J3"), G(38, 0), J("J2")],
    &[J("J1"), G(18, 6), G(28, 6), G(38, 6), J("J2")],
    &[J("J4"), J("J6"), J("J3"), J("J7")],
    &[J("J5"), J("J8"), J("J3")],
];

#[derive(Clone, Copy)]
enum Step {
    J(&'static str),
    G(i32, i32),
}
use Step::{G, J};

/// Builds the Howe truss on screen with the Member tool (current size).
fn build_howe(h: &mut App) {
    click_button(h, "Member");
    for chain in HOWE {
        for s in *chain {
            match *s {
                J(id) => click_joint(h, id),
                G(x, y) => click_grid(h, x as f64, y as f64),
            }
        }
        click_button(h, "Finish line");
    }
}

/// The same structure built on a headless engine (for the expected numbers).
fn engine_howe(mat: MaterialKey, size: SizeKey) -> Game {
    let mut g = Game::new();
    g.pick_material(mat);
    g.pick_tool(Tool::Member);
    g.pick_size(size);
    for chain in HOWE {
        for s in *chain {
            match *s {
                J(id) => g.click_joint(id),
                G(x, y) => {
                    let (sx, sy) = snap(px(x as f64), py(y as f64));
                    g.click_grid(sx, sy)
                }
            }
        }
        g.finish_line();
    }
    g
}

/// Steps until a result banner shows on the Bridge ("It held…" / "Collapse…").
fn wait_for_result(h: &mut App) -> String {
    let ok = step_until(h, 1500, |h| {
        label_starting(h, "It held").is_some() || label_starting(h, "Collapse:").is_some()
    });
    assert!(ok, "no result banner");
    settle(h);
    label_starting(h, "It held")
        .or_else(|| label_starting(h, "Collapse:"))
        .unwrap()
}

/// Clicks the banner's "Back to building" (the aside's big button has the same name).
fn banner_back(h: &mut App) {
    let stage = stage_rect(h);
    h.root()
        .children_recursive()
        .find(|n| {
            n.accesskit_node().role() == Role::Button
                && n.accesskit_node().label().as_deref() == Some("Back to building")
                && stage.contains_rect(n.rect())
        })
        .expect("the banner's Back to building")
        .click();
    settle(h);
}

/// The member inspector's force as a signed whole kN (+ stretched, − squeezed).
fn inspector_kn(h: &App) -> i64 {
    for l in labels(h) {
        for (head, sign) in [("Stretched with ", 1), ("Squeezed with ", -1)] {
            if let Some(rest) = l.strip_prefix(head) {
                let n: i64 = rest.split(' ').next().unwrap().parse().unwrap();
                return sign * n;
            }
        }
        if l.starts_with("Carrying almost nothing") {
            return 0;
        }
    }
    panic!("no member inspector sentence")
}

/// "M3 J3–J5: N 216 kN, σ 27 MPa, limit yield, use 11%" → (216, 11).
fn advanced_row(h: &App, id: &str) -> (i64, i64) {
    let head = format!("{id} J");
    let row = labels(h)
        .into_iter()
        .find(|l| l.starts_with(&head) && l.contains(": N ") && l.contains(" kN, σ "))
        .unwrap_or_else(|| panic!("no Advanced result row for {id}"));
    let n = row
        .split("N ")
        .nth(1)
        .and_then(|s| s.split(" kN").next())
        .unwrap()
        .replace('−', "-")
        .parse::<i64>()
        .unwrap_or_else(|e| panic!("{row}: {e}"));
    let u = row
        .rsplit("use ")
        .next()
        .and_then(|s| s.strip_suffix('%'))
        .unwrap()
        .parse::<i64>()
        .unwrap();
    (n, u)
}

/// "Member M3, 11% of its limit" → 11.
fn handle_use(h: &App, id: &str) -> i64 {
    let l = labelled(h, &format!("Member {id}"));
    l.split(", ")
        .nth(1)
        .and_then(|s| s.strip_suffix("% of its limit"))
        .unwrap_or_else(|| panic!("{l}"))
        .parse()
        .unwrap()
}

// ============================================================================================
// 1. Level 1 from scratch

#[test]
fn journey_1_level_one_from_scratch_holds_with_stars() {
    let mut h = harness(Route::Bridge);
    assert!(header_has(&h, "river-level") && header_has(&h, "building"));
    assert!(has_label(&h, "Nothing built yet"));
    build_howe(&mut h);
    let expect = engine_howe(MaterialKey::Steel, SizeKey::Medium);
    let v = expect.view();
    assert!(has_label(&h, &v.status.label), "status {}", v.status.label);
    assert!(has_label(&h, &format!("Cost {}", v.cost_text)));
    assert!(has_label(&h, &v.stage_label));
    click_button(&mut h, "Test it: add the load");
    assert_eq!(drive(&h), "Pause");
    let title = wait_for_result(&mut h);
    let mut g = expect.clone();
    g.run_test(2000);
    let result = g.view().result.expect("the engine's result");
    assert_eq!(title, result.title);
    assert_eq!(title, "It held!");
    assert!(has_label(&h, &result.stars), "stars {}", result.stars);
    assert!(header_has(&h, "held"));
    assert_eq!(drive(&h), "Test");
    // best stars stay on the level row, also after visiting another level
    banner_back(&mut h);
    let best = |h: &App| {
        get(h, "1 · River crossing", Role::Button)
            .accesskit_node()
            .description()
    };
    let want = format!("Best: {} of 3 stars", g.stars());
    assert_eq!(best(&h).as_deref(), Some(want.as_str()));
    click_button(&mut h, "2 · Overhang");
    assert!(header_has(&h, "overhang-level"));
    click_button(&mut h, "1 · River crossing");
    assert_eq!(best(&h).as_deref(), Some(want.as_str()));
    assert!(has_label(&h, "Nothing built yet"), "the level starts again");
}

// ============================================================================================
// 2. Break one

#[test]
fn journey_2_a_bridge_buckles_and_replays() {
    let mut h = harness(Route::Bridge);
    click_button(&mut h, "Timber");
    click_button(&mut h, "Member");
    click_button(&mut h, "Thin");
    build_howe(&mut h);
    let mut g = engine_howe(MaterialKey::Timber, SizeKey::Thin);
    let status_before = g.view().status.label;
    assert!(has_label(&h, &status_before), "{status_before}");
    g.run_test(2000);
    let want = g.view().result.unwrap().title;
    assert_eq!(want, "Collapse: M6 buckled at 7%");
    press_drive(&mut h);
    let title = wait_for_result(&mut h);
    assert_eq!(title, want);
    assert!(header_has(&h, "collapse"));
    assert!(label_starting(&h, "M6 buckled at 7% of the load. Long, thin members").is_some());
    // replay in slow motion: the banner goes while it plays, then comes back
    click_button(&mut h, "Replay in slow motion");
    assert!(label_starting(&h, "Collapse:").is_none(), "playing");
    assert_eq!(drive(&h), "Pause");
    assert_eq!(wait_for_result(&mut h), want);
    banner_back(&mut h);
    assert!(header_has(&h, "building"));
    assert!(has_label(&h, &status_before), "the design is still there");
    assert!(has_label(&h, "Test it: add the load"));
}

// ============================================================================================
// 3. Open it in Advanced; Share / paste round trip

#[test]
fn journey_3_advanced_shows_the_same_forces() {
    let mut h = harness(Route::Bridge);
    build_howe(&mut h);
    // read every member's force and use from the Bridge's inspector and handles
    click_button(&mut h, "Select");
    let mut bridge = Vec::new();
    for k in 1..=13 {
        let id = format!("M{k}");
        let u = handle_use(&h, &id);
        let l = labelled(&h, &format!("Member {id}"));
        click_button(&mut h, &l);
        assert!(
            has_label_containing(&h, &format!("{id} · J")),
            "{id} inspected"
        );
        bridge.push((id, inspector_kn(&h), u));
    }
    click_grid(&mut h, 30.0, 14.0); // nothing selected: "Built from" and "Open in Advanced"
    click_button(&mut h, "Open in Advanced");
    settle(&mut h);
    assert!(header_has(&h, "river-level"), "the netlist's name");
    assert!(selected(&h, "Advanced", Role::Link) == Some(true));
    for (id, kn, u) in &bridge {
        assert_eq!(advanced_row(&h, id), (*kn, *u), "{id}");
    }
    assert!(label_starting(&h, "Holding · worst").is_some());
    // Share in Advanced, paste on the Bridge: the same structure lands in the Sandbox
    click_header(&mut h, "Share");
    let text = last_copied(&h).expect("Share copies the netlist");
    assert!(text.starts_with("* river-level.str"), "{text}");
    press_cmd(&mut h, Key::Num1);
    settle(&mut h);
    assert!(selected(&h, "Bridge", Role::Tab) == Some(true));
    h.event(egui::Event::Paste(text));
    settle(&mut h);
    assert!(header_has(&h, "sandbox"));
    assert_eq!(pressed(&h, "Sandbox"), Some(true));
    assert!(has_label(&h, "Members: 13"));
    assert!(has_label(&h, "Holding"));
}

// ============================================================================================
// 4. An error in the netlist, fixed, then back to the Playground

#[test]
fn journey_4_netlist_error_fix_and_open_in_playground() {
    let mut h = harness(Route::Bridge);
    build_howe(&mut h);
    click_button(&mut h, "Select");
    click_grid(&mut h, 30.0, 14.0);
    click_button(&mut h, "Open in Advanced");
    settle(&mut h);
    click_header(&mut h, "Share");
    let good = last_copied(&h).unwrap();
    let rows_before = labels(&h)
        .into_iter()
        .filter(|l| l.contains(": N ") && l.contains(" kN, σ "))
        .count();
    assert_eq!(rows_before, 13);
    // break it: member M3 now names a joint that doesn't exist
    let line_no = good
        .lines()
        .position(|l| l.starts_with(".member M3 ") || l.starts_with(".member  M3 "))
        .expect("M3's line")
        + 1;
    let bad: String = good
        .lines()
        .map(|l| {
            if l.starts_with(".member M3 ") || l.starts_with(".member  M3 ") {
                l.replace("J5", "J99")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let replace = |h: &mut App, t: &str| {
        get(h, "Netlist", Role::MultilineTextInput).focus();
        h.step();
        h.key_press_modifiers(Modifiers::COMMAND, Key::A);
        h.step();
        get(h, "Netlist", Role::MultilineTextInput).type_text(t);
        h.run_steps(4);
    };
    replace(&mut h, &bad);
    let err = format!("Line {line_no}: unknown node J99");
    assert!(has_label(&h, &err), "missing {err:?}");
    assert!(has_label(&h, "1 error"));
    assert!(has_label(
        &h,
        "The last good result stays on screen, dimmed, until the errors are fixed."
    ));
    let rows = labels(&h)
        .into_iter()
        .filter(|l| l.contains(": N ") && l.contains(" kN, σ "))
        .count();
    assert_eq!(rows, 13, "the last good result stays");
    assert!(
        get(&h, "Open in Playground", Role::Button)
            .accesskit_node()
            .is_disabled()
    );
    // fix it
    replace(&mut h, &good);
    assert!(!has_label(&h, "1 error"));
    assert!(!has_label(&h, &err));
    assert!(has_label(
        &h,
        "Edit freely: every change is parsed and solved."
    ));
    // Open in Playground: the Sandbox shows it
    reveal(&mut h, "Open in Playground", Some(Role::Button));
    click_button(&mut h, "Open in Playground");
    settle(&mut h);
    assert!(selected(&h, "Bridge", Role::Tab) == Some(true));
    assert!(header_has(&h, "sandbox"));
    assert!(has_label(&h, "Joints: 8"));
    assert!(has_label(&h, "Members: 13"));
    assert!(has_label(&h, "Holding"));
    assert!(label_starting(&h, "Joint J3, carrying 20 t").is_some());
}

// ============================================================================================
// 5. Tower → Ground → the lean of Pisa → back

#[test]
fn journey_5_tower_to_ground_and_the_lean_of_pisa() {
    let mut h = harness(Route::Tower);
    slider(&mut h, "Number of floors", 99.0);
    click_button(&mut h, "Add a floor");
    click_button(&mut h, "Braced tube");
    click_button(&mut h, "Tapered");
    click(&mut h, "Tuned mass damper at the top");
    let input = TowerInput {
        floors: 100,
        system: System::Braced,
        shape: Shape::Tapered,
        damper: true,
        ..TowerInput::default()
    };
    let r = tower::calc(&input);
    assert!(has_label(&h, &tower::stage_label(&input, &r)));
    for g in tower::goals(&input, &r) {
        assert_eq!(checked(&h, g.text), g.done, "goal {:?}", g.text);
        assert!(
            g.done,
            "the 100-floor challenge goal {:?} lights (MB-6)",
            g.text
        );
    }
    // to Ground with this building
    click_link(&mut h, "Send base reactions to Ground ›");
    settle(&mut h);
    assert!(selected(&h, "Ground", Role::Tab) == Some(true));
    let note = ground::FromTower {
        floors: 100,
        overturning: r.overturning,
    }
    .note();
    assert!(has_label(&h, &note), "missing {note:?}");
    // the lean of Pisa: the scenario, then piles to rock, then 30 years
    let goals = ground::goals(&GroundInput::default());
    click_button(&mut h, "Leaning tower (soft clay, mat)");
    assert!(checked(&h, goals[0].text));
    assert!(!checked(&h, goals[2].text), "still leaning");
    click_button(&mut h, "Piles");
    slider(&mut h, "Pile length", 34.0);
    assert!(has_label(&h, "Pile length: 34 m · on rock"));
    slider(&mut h, "Years after construction", 30.0);
    assert!(header_has(&h, "year 30.0"));
    for g in &goals {
        assert!(checked(&h, g.text), "Pisa goal {:?}", g.text);
    }
    click_link(&mut h, "‹ Back to the Tower");
    settle(&mut h);
    assert!(selected(&h, "Tower", Role::Tab) == Some(true));
    // the Tower takes Ground's building (6 floors after the scenario)
    let floors = TowerInput { floors: 6, ..input };
    let well = format!("6 {}", tower::floors_note(&floors));
    assert!(
        has_label(&h, &well),
        "the Tower shows Ground's building: {well:?}"
    );
}

// ============================================================================================
// 6. Materials: break all five

#[test]
fn journey_6_break_all_five_materials() {
    let mut h = harness(Route::Materials);
    for kind in Kind::ALL {
        let name = kind.name();
        assert!(has_label(&h, &format!("{name}: not broken yet")), "{name}");
        click_button(&mut h, name);
        assert_eq!(pressed(&h, name), Some(true));
        press_drive(&mut h);
        assert_eq!(drive(&h), "Pause");
        assert!(
            step_until(&mut h, 800, |h| drive(h) == "Pull"),
            "{name} never broke"
        );
        settle(&mut h);
        // the status the engine reaches pulling this sample to the end
        let mut bench = Bench::default();
        bench.pick(kind);
        bench.toggle_drive();
        for _ in 0..10_000 {
            if !bench.playing() {
                break;
            }
            bench.tick();
        }
        let want = bench.status().label();
        assert!(bench.status().is_broken(), "{name}: {want}");
        let status = text_of(&h.get_by_role(Role::Status));
        assert_eq!(status, want, "{name}");
        assert!(has_label(&h, &format!("{name}: broken")), "{name} chip");
    }
    for kind in Kind::ALL {
        assert!(has_label(&h, &format!("{}: broken", kind.name())));
    }
}

// ============================================================================================
// 7. Keyboard only

#[test]
fn journey_7_keyboard_only() {
    let mut h = harness(Route::Bridge);
    // ⌘1–⌘5 switch screens
    for (key, tab) in [
        (Key::Num2, "Tower"),
        (Key::Num3, "Ground"),
        (Key::Num4, "Materials"),
        (Key::Num1, "Bridge"),
    ] {
        press_cmd(&mut h, key);
        settle(&mut h);
        assert_eq!(selected(&h, tab, Role::Tab), Some(true), "⌘ → {tab}");
    }
    press_cmd(&mut h, Key::Num5);
    settle(&mut h);
    assert_eq!(selected(&h, "Advanced", Role::Link), Some(true));
    assert!(
        h.query_by_role_and_label(Role::TabList, "Playground worlds")
            .is_none()
    );
    assert_eq!(drive(&h), "Drive across");
    // Space drives (nothing focused)
    press_cmd(&mut h, Key::Num4);
    settle(&mut h);
    h.remove_cursor();
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(drive(&h), "Pause", "Space pressed Pull");
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(drive(&h), "Pull", "Space paused");
    // Tab reaches the drive button; Enter presses it
    let mut reached = false;
    for _ in 0..40 {
        press(&mut h, Key::Tab);
        if focused_label(&h).as_deref() == Some("Pull") {
            reached = true;
            break;
        }
    }
    assert!(reached, "Tab never reached the drive button");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(drive(&h), "Pause", "Enter on the focused drive button");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(drive(&h), "Pull");
    // the brand opens the Series screen; Esc comes back
    let mut on_brand = false;
    for _ in 0..40 {
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
        selected(&h, "Materials", Role::Tab),
        Some(true),
        "back where it was"
    );
}
