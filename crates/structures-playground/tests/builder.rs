//! SCR-1 builder journeys in the real app (egui_kittest, AccessKit labels; SPEC BLD-1…BLD-14,
//! NAV-4/5/6): level 1 built from scratch by clicking (a Road chain and Wood triangles), run,
//! all across with stars; the placement rules' toasts; the piece and joint inspectors;
//! materials chips; foundations; crates; zoom / pan / fit by buttons, keys and the wheel;
//! conditions while running; slow motion; the Sandbox scenes; Share, ⌘V and the hand-offs;
//! undo / Delete / Esc / nudge; frame pacing.

mod common;

use common::*;
use eframe::egui::{self, Modifiers, PointerButton, Pos2, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::builder::{Foundation, Game, LevelKey, Phase, PieceType, Scene, Selection, Tool};
use structures_playground::{Carry, Command, Route};

fn game(h: &App) -> &Game {
    &h.state().screens().builder.game
}

fn game_mut(h: &mut App) -> &mut Game {
    &mut h.state_mut().screens_mut().builder.game
}

/// The stage's drawing box: the grid surface button's rect inside its 1 px border.
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

/// Screen position of world point (x, y) m with the current camera.
fn at(h: &App, x: f64, y: f64) -> Pos2 {
    let r = stage_inner(h);
    let s = r.width() / 1120.0;
    let cam = game(h).camera();
    r.min + egui::vec2(cam.x(x) as f32 * s, cam.y(y) as f32 * s)
}

fn press_at(h: &mut App, p: Pos2, pressed: bool) {
    h.event(egui::Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

/// Clicks the stage at world (x, y) m (empty grid, or a joint drawn there).
fn click_grid(h: &mut App, x: f64, y: f64) {
    let p = at(h, x, y);
    h.hover_at(p);
    press_at(h, p, true);
    h.step();
    press_at(h, p, false);
    h.run_steps(3);
}

/// The label of joint `id` ("Joint J3, on a pile, carrying a 5 t crate").
fn joint_label(h: &App, id: &str) -> String {
    let head = format!("Joint {id}");
    h.root()
        .children_recursive()
        .filter(|n| n.accesskit_node().role() == Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| *l == head || l.starts_with(&format!("{head},")))
        .unwrap_or_else(|| panic!("no button for {head}"))
}

fn click_joint(h: &mut App, id: &str) {
    let l = joint_label(h, id);
    h.get_by_role_and_label(Role::Button, &l).click();
    h.run_steps(3);
}

/// The label of piece `id`'s handle ("Steel M7, 63% of its limit").
fn handle_label(h: &App, id: &str) -> String {
    h.root()
        .children_recursive()
        .filter(|n| n.accesskit_node().role() == Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| {
            let mut w = l.split([' ', ',']);
            w.next().is_some() && w.next() == Some(id)
        })
        .unwrap_or_else(|| panic!("no handle for {id}"))
}

fn click_handle(h: &mut App, id: &str) {
    let l = handle_label(h, id);
    h.get_by_role_and_label(Role::Button, &l).click();
    h.run_steps(3);
}

/// Clicks the chip / button labelled `label` inside the group `group`, scrolled into view
/// first (the aside scrolls at 1440).
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

/// Clicks a button scrolled into view.
fn click_shown(h: &mut App, label: &str) {
    reveal(h, label, Some(Role::Button));
    click_button(h, label);
}

fn pressed_in(h: &App, group: &str, label: &str) -> bool {
    use egui::accesskit::Toggled;
    h.get_by_role_and_label(Role::Group, group)
        .get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .toggled()
        == Some(Toggled::True)
}

/// The stage summary ("Bridge with 9 joints and 15 pieces. Stands up.").
fn stage_label(h: &App) -> String {
    h.get_by_role(Role::Image)
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

/// Ticks the run to its end (the engine's 1/30 s frames) and draws the result.
fn finish_run(h: &mut App) {
    for _ in 0..2000 {
        if game(h).phase() != Phase::Run {
            break;
        }
        game_mut(h).tick();
    }
    settle(h);
}

/// Builds level 1's Road deck and a Wood truss over it by clicking: the deck J1 → 16, 24, 32,
/// 40 → J2; the top chord J1 → (16…40, 6) → J2; verticals and diagonals.
fn build_level_one(h: &mut App) {
    click_in(h, "Piece to build", "Road");
    click_joint(h, "J1");
    for x in [16.0, 24.0, 32.0, 40.0] {
        click_grid(h, x, 0.0);
    }
    click_joint(h, "J2");
    click_button(h, "Finish line");
    click_in(h, "Piece to build", "Wood");
    click_joint(h, "J1");
    for x in [16.0, 24.0, 32.0, 40.0] {
        click_grid(h, x, 6.0);
    }
    click_joint(h, "J2");
    click_button(h, "Finish line");
    for chain in [
        &["J3", "J7", "J4", "J8", "J5", "J9"][..],
        &["J6", "J10", "J5"][..],
    ] {
        for id in chain {
            click_joint(h, id);
        }
        click_button(h, "Finish line");
    }
}

/// DONE-4 / BLD-6/7/9/10: level 1 built from scratch by clicks holds up, runs, gets every car
/// across with a star, and the level row remembers it.
#[test]
fn level_one_from_scratch_gets_all_across_with_stars() {
    let mut h = harness(Route::Playground);
    assert_eq!(document(&h), "river-level");
    assert_eq!(drive_label(&h).as_deref(), Some("Run traffic"));
    assert!(has_label(&h, "Nothing built yet"));
    build_level_one(&mut h);
    let d = game(&h).design();
    assert_eq!(d.nodes.len(), 10, "{:?}", d.nodes);
    assert_eq!(d.members.len(), 17);
    assert_eq!(
        d.members
            .iter()
            .filter(|m| m.kind == PieceType::Road)
            .count(),
        5
    );
    let label = stage_label(&h);
    assert!(
        label.starts_with("Bridge with 10 joints and 17 pieces."),
        "{label}"
    );
    let status = game(&h).view().status.label;
    assert!(
        status == "Stands up" || status == "Stands, but strained",
        "{status}"
    );
    assert!(has_label(&h, &status));
    // run from the goal panel's button
    click_in(&mut h, "Goal", "Run traffic");
    assert_eq!(game(&h).phase(), Phase::Run);
    assert_eq!(drive_label(&h).as_deref(), Some("Pause"));
    assert!(has_label_containing(&h, " across"));
    assert!(has_label(&h, "Slow motion"));
    finish_run(&mut h);
    let r = game(&h).result().expect("a result").clone();
    assert!(r.pass, "{r:?}");
    assert!(r.stars >= 1, "{r:?}");
    assert!(has_label(&h, "All across!"));
    let stars = game(&h).view().result.expect("banner").stars;
    assert!(has_label(&h, &stars), "{stars}");
    // the level row keeps the best stars
    let row = h.get_by_role_and_label(Role::Button, "1 · First crossing");
    assert_eq!(
        row.accesskit_node().description().as_deref(),
        Some(format!("Best: {} of 3 stars", r.stars).as_str())
    );
    // back to building (the banner's button; the goal panel's says the same)
    click_in(&mut h, "Goal", "Back to building");
    assert_eq!(game(&h).phase(), Phase::Build);
    assert_eq!(drive_label(&h).as_deref(), Some("Run traffic"));
}

/// BLD-6: the placement rules' toasts, shown on the stage.
#[test]
fn placement_rules_toast() {
    let mut h = harness(Route::Playground);
    // underground on a bank
    click_grid(&mut h, 4.0, -2.0);
    assert!(
        has_label(&h, "That spot is underground."),
        "{:?}",
        toasts(&h)
    );
    // too long a Road
    click_joint(&mut h, "J1");
    click_grid(&mut h, 30.0, 0.0);
    assert!(has_label_containing(&h, "Road pieces can be at most 10 m"));
    click_button(&mut h, "Finish line");
    // the level's own foundations stay put
    click_button(&mut h, "Found");
    click_joint(&mut h, "J1");
    assert!(has_label(&h, "Level foundations stay put."));
    // foundations go on the ground
    click_grid(&mut h, 20.0, 6.0);
    assert!(has_label(&h, "Foundations go on the ground."));
    // the toast fades after 2.6 s (and the frame wakes for it)
    game_mut(&mut h).advance(3.0);
    settle(&mut h);
    assert!(!has_label(&h, "Foundations go on the ground."));
}

/// BLD-13: the piece and joint inspectors, the material chips, delete.
#[test]
fn inspectors() {
    let mut h = harness(Route::Playground);
    build_level_one(&mut h);
    click_button(&mut h, "Select");
    assert!(pressed(&h, "Select").unwrap_or(false));
    // a piece
    click_handle(&mut h, "M1");
    assert!(has_label(&h, "Road · M1"), "{:?}", game(&h).selection());
    assert!(has_label_containing(&h, "% of its limit"));
    assert!(
        has_label_containing(&h, "Pulled with")
            || has_label_containing(&h, "Pushed with")
            || has_label_containing(&h, "Doing almost nothing")
    );
    // change it to Steel (Floor isn't offered on the river)
    let group = "Change this piece";
    assert!(
        !h.get_by_role_and_label(Role::Group, group)
            .query_by_label("Floor")
            .is_some()
    );
    click_in(&mut h, group, "Steel");
    assert_eq!(game(&h).design().members[0].kind, PieceType::Steel);
    assert!(has_label(&h, "Steel · M1"));
    click_shown(&mut h, "Delete piece");
    assert!(game(&h).design().member("M1").is_none());
    // a joint
    click_joint(&mut h, "J3");
    assert!(has_label(&h, "Joint J3"), "{:?}", game(&h).selection());
    assert!(has_label_containing(&h, "x 16 m"));
    click_shown(&mut h, "Heavier crate");
    assert_eq!(game(&h).design().load("J3"), 5.0);
    assert!(has_label(&h, "Crate 5 t"));
    assert!(joint_label(&h, "J3").contains("carrying a 5 t crate"));
    click_shown(&mut h, "Lighter crate");
    assert_eq!(game(&h).design().load("J3"), 0.0);
    // the level's own joint
    click_joint(&mut h, "J1");
    assert!(has_label(&h, "Joint J1 · level foundation"));
    assert!(!has_label(&h, "Delete joint"));
    click_joint(&mut h, "J10");
    click_shown(&mut h, "Delete joint");
    assert!(game(&h).design().node("J10").is_none());
    // nothing selected: the Materials list
    press(&mut h, egui::Key::Escape);
    assert!(game(&h).selection().is_none());
    assert!(has_label_containing(&h, "Road · Vehicles drive on it."));
}

/// BLD-1/3: the piece palette (scene-filtered, pressed state), foundation and crate chips.
#[test]
fn palette_foundations_and_crates() {
    let mut h = harness(Route::Playground);
    let palette = "Piece to build";
    for k in ["Road", "Wood", "Steel", "Concrete", "Rope", "Cable"] {
        assert!(
            h.get_by_role_and_label(Role::Group, palette)
                .query_by_label(k)
                .is_some(),
            "{k}"
        );
    }
    assert!(pressed_in(&h, palette, "Road"));
    click_in(&mut h, palette, "Cable");
    assert!(pressed_in(&h, palette, "Cable"));
    assert_eq!(game(&h).piece_type(), PieceType::Cable);
    // the Sandbox allows foundations on the riverbed
    click_button(&mut h, "Sandbox");
    assert_eq!(document(&h), "sandbox");
    click_button(&mut h, "Found");
    click_in(&mut h, "Foundation type", "Pile · $1.5k");
    assert_eq!(game(&h).found_kind(), Foundation::Pile);
    click_grid(&mut h, 20.0, -12.0);
    let id = game(&h).design().nodes.last().expect("a joint").id.clone();
    assert!(
        joint_label(&h, &id).contains("on a pile"),
        "{}",
        joint_label(&h, &id)
    );
    // crates
    click_button(&mut h, "Crate");
    click_in(&mut h, "Weight to add", "10 t");
    click_joint(&mut h, "J1");
    assert_eq!(game(&h).design().load("J1"), 10.0);
    assert!(joint_label(&h, "J1").contains("carrying a 10 t crate"));
    assert!(has_label(&h, "Sandbox scene"));
    // the open-ground scene: Floor instead of Road
    click_in(&mut h, "Sandbox scene", "Open ground");
    assert_eq!(game(&h).scene(), Scene::Flat);
    click_button(&mut h, "Build");
    let g = h.get_by_role_and_label(Role::Group, palette);
    assert!(g.query_by_label("Floor").is_some());
    assert!(g.query_by_label("Road").is_none());
    assert!(stage_label(&h).starts_with("Tower with 2 joints"));
    assert_eq!(drive_label(&h).as_deref(), Some("Run the test"));
}

/// BLD-2 / NAV-5: zoom by the buttons, ⌘= / ⌘− / ⌘0, the wheel at the pointer; pan by the
/// arrows and by dragging empty space with Select; Fit.
#[test]
fn zoom_pan_and_fit() {
    let mut h = harness(Route::Playground);
    let z = |h: &App| game(h).view_state().z;
    assert!(has_label(&h, "Zoom 100%"));
    click_button(&mut h, "Zoom in");
    assert!((z(&h) - 1.3).abs() < 1e-9);
    assert!(has_label(&h, "Zoom 130%"));
    click_button(&mut h, "Zoom out");
    assert!((z(&h) - 1.0).abs() < 1e-9);
    press_cmd(&mut h, egui::Key::Equals);
    settle(&mut h);
    assert!((z(&h) - 1.3).abs() < 1e-9);
    press_cmd(&mut h, egui::Key::Minus);
    settle(&mut h);
    assert!((z(&h) - 1.0).abs() < 1e-9);
    // the wheel zooms about the pointer: the world point under it stays put
    let p = at(&h, 40.0, 0.0);
    h.hover_at(p);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    settle(&mut h);
    assert!((z(&h) - 1.15).abs() < 1e-9, "{}", z(&h));
    let q = at(&h, 40.0, 0.0);
    assert!((p - q).length() < 1.0, "{p:?} {q:?}");
    // arrows pan
    let cx = game(&h).view_state().cx;
    press(&mut h, egui::Key::ArrowRight);
    settle(&mut h);
    assert!(game(&h).view_state().cx > cx);
    // drag empty space with Select pans
    click_button(&mut h, "Select");
    let cx = game(&h).view_state().cx;
    let a = at(&h, 28.0, 6.0);
    h.hover_at(a);
    press_at(&mut h, a, true);
    h.step();
    for k in 1..=6 {
        h.hover_at(a + egui::vec2(-10.0 * k as f32, 0.0));
        h.step();
    }
    press_at(&mut h, a + egui::vec2(-60.0, 0.0), false);
    settle(&mut h);
    assert!(
        game(&h).view_state().cx > cx + 1.0,
        "{cx} -> {}",
        game(&h).view_state().cx
    );
    assert!(game(&h).design().nodes.len() == 2, "a pan builds nothing");
    // fit (⌘0) frames the structure
    press_cmd(&mut h, egui::Key::Num0);
    settle(&mut h);
    let v = game(&h).view_state();
    click_button(&mut h, "Fit");
    assert_eq!(game(&h).view_state(), v);
    assert!(
        commands(&h)
            .iter()
            .any(|(_, c, handled)| *c == Command::ZoomFit && *handled)
    );
}

/// BLD-9/12: conditions change while running (wind eases in on the HUD, a quake with "Shake
/// now"); soil and traffic refuse with toasts; slow motion; Stop.
#[test]
fn conditions_while_running_and_slow_motion() {
    let mut h = harness(Route::Playground);
    build_level_one(&mut h);
    click_drive(&mut h);
    assert_eq!(game(&h).phase(), Phase::Run);
    assert!(has_label(
        &h,
        "Building is paused while it runs. You can still change the conditions, zoom and pan."
    ));
    click_in(&mut h, "Wind", "Storm");
    assert_eq!(game(&h).cond().wind, 35.0);
    for _ in 0..90 {
        game_mut(&mut h).tick();
    }
    settle(&mut h);
    assert!(has_label_containing(&h, "Wind "), "the HUD shows the wind");
    click_in(&mut h, "Earthquake", "Strong");
    assert!(has_label(&h, "Shake now"));
    click_shown(&mut h, "Shake now");
    click_in(&mut h, "Soil", "Rock");
    assert!(has_label(&h, "Soil can only change while building."));
    click_in(&mut h, "Traffic", "Buses");
    assert!(has_label(&h, "Traffic changes on the next run."));
    click_button(&mut h, "Slow motion");
    assert!(game(&h).slow());
    assert!(has_label(&h, "Normal speed"));
    click_in(&mut h, "Goal", "Stop");
    assert_eq!(game(&h).phase(), Phase::Build);
}

/// NAV-4/6: Share copies the structure as a netlist; ⌘V loads one into the Sandbox; "Open in
/// Advanced" hands it over; Advanced's "Open in Playground" lands in the Sandbox.
#[test]
fn share_paste_and_hand_offs() {
    let mut h = harness(Route::Playground);
    build_level_one(&mut h);
    click_header(&mut h, "Share");
    let text = last_copied(&h).expect("copied");
    assert!(text.contains(".node"), "{text}");
    assert!(text.contains("* river-level.str"), "{text}");
    // ⌘V into the Sandbox
    click_button(&mut h, "Clear");
    h.event(egui::Event::Paste(text.clone()));
    settle(&mut h);
    assert_eq!(game(&h).level(), LevelKey::Sandbox);
    assert_eq!(game(&h).design().nodes.len(), 10);
    assert!(has_label_containing(&h, "Loaded a 10-joint structure"));
    // Open in Advanced
    click_button(&mut h, "Open in Advanced");
    settle(&mut h);
    assert_eq!(route(&h), Route::Advanced);
    let carried = carried(&h);
    let (to, Carry::Netlist { text: t, from, .. }) = carried.last().expect("a carry").clone();
    assert_eq!(to, Route::Advanced);
    assert_eq!(from, Some(Route::Playground));
    assert!(t.contains(".member"));
    // and back
    hand_off(
        &mut h,
        Route::Playground,
        Carry::netlist(text, None, Route::Advanced),
    );
    assert_eq!(game(&h).level(), LevelKey::Sandbox);
    assert_eq!(game(&h).design().members.len(), 17);
}

/// NAV-5: ⌘Z undoes, Delete removes the selection, ⌥ arrows nudge a joint, Esc ends a chain;
/// Reset starts the level again.
#[test]
fn keyboard_editing() {
    let mut h = harness(Route::Playground);
    click_joint(&mut h, "J1");
    click_grid(&mut h, 16.0, 0.0);
    assert_eq!(game(&h).design().members.len(), 1);
    assert!(game(&h).pending().is_some());
    press(&mut h, egui::Key::Escape);
    assert!(game(&h).pending().is_none(), "Esc ends the chain");
    press_cmd(&mut h, egui::Key::Z);
    settle(&mut h);
    assert_eq!(game(&h).design().members.len(), 0);
    assert_eq!(game(&h).design().nodes.len(), 2);
    // nudge a joint with ⌥ arrows
    click_grid(&mut h, 20.0, 4.0);
    click_button(&mut h, "Finish line");
    click_button(&mut h, "Select");
    let id = game(&h).design().nodes.last().expect("joint").id.clone();
    click_joint(&mut h, &id);
    assert_eq!(game(&h).selection(), Some(&Selection::Node(id.clone())));
    press_alt(&mut h, egui::Key::ArrowUp);
    settle(&mut h);
    let n = game(&h).design().node(&id).expect("joint").clone();
    assert_eq!((n.x, n.y), (20.0, 6.0));
    press(&mut h, egui::Key::Delete);
    settle(&mut h);
    assert!(game(&h).design().node(&id).is_none());
    // Reset
    click_joint(&mut h, "J1");
    click_header(&mut h, "Reset");
    assert_eq!(game(&h).design().nodes.len(), 2);
    assert_eq!(game(&h).tool(), Tool::Select);
}

/// PLAT-3 / BLD-15: nothing repaints at rest; a run asks for ~30 frames a second and
/// advances.
#[test]
fn frame_pacing() {
    let asked = |h: &App| h.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    let mut h = harness(Route::Playground);
    build_level_one(&mut h);
    game_mut(&mut h).advance(5.0);
    h.run_steps(8);
    assert!(
        asked(&h) >= std::time::Duration::from_secs(1),
        "at rest: {:?} ({:?})",
        asked(&h),
        h.ctx.repaint_causes()
    );
    click_drive(&mut h);
    h.run_steps(3);
    assert!(game(&h).animating());
    let d = asked(&h);
    assert!(
        d >= std::time::Duration::from_millis(10) && d <= std::time::Duration::from_millis(40),
        "running: {d:?}"
    );
    let t0 = game(&h).sim().expect("running").t;
    h.run_steps(30);
    assert!(game(&h).sim().expect("running").t > t0, "the run advances");
}
