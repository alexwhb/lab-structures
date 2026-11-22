//! SCR-1 Bridge journeys in the real app (egui_kittest, AccessKit labels; SPEC BR-1…BR-11,
//! NAV-4/5/6, DONE-4): build level 1 from scratch by clicking joints and the grid, test it and
//! watch it hold with stars; a thin timber bridge buckles (banner, replay in slow motion, back
//! to building); a mechanism; the placement rules' toasts; the inspector; undo (⌘Z and the
//! rail); Delete; arrow-key nudge; dragging a joint; level switching; Share, ⌘V and the
//! hand-offs to and from Advanced; the layout at 1440 and 390; closeness to the reference.

mod common;

use common::*;
use eframe::egui::{self, Modifiers, PointerButton, Pos2, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::bridge::data::{px, py};
use sp_engine::bridge::{LevelKey, Phase, SizeKey};
use structures_playground::{Carry, Route, Screen};

/// The stage's rect (the grid surface button).
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

/// Screen position of grid point (x, y) m.
fn at(h: &App, x: f64, y: f64) -> Pos2 {
    let r = stage_rect(h);
    let s = r.width() / 1120.0;
    r.min + egui::vec2(px(x) as f32 * s, py(y) as f32 * s)
}

fn press_at(h: &mut App, p: Pos2, pressed: bool) {
    h.event(egui::Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

/// Clicks empty grid at (x, y) m.
fn click_grid(h: &mut App, x: f64, y: f64) {
    let p = at(h, x, y);
    h.hover_at(p);
    press_at(h, p, true);
    press_at(h, p, false);
    h.run_steps(3);
}

/// The label of joint `id` ("Joint J3, pin support, carrying 20 t").
fn joint_label(h: &App, id: &str) -> String {
    let head = format!("Joint {id}");
    h.root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| *l == head || l.starts_with(&format!("{head},")))
        .unwrap_or_else(|| panic!("no button for {head}"))
}

fn click_joint(h: &mut App, id: &str) {
    let l = joint_label(h, id);
    h.get_by_role_and_label(Role::Button, &l).click();
    h.run_steps(3);
}

fn member_label(h: &App, id: &str) -> String {
    let head = format!("Member {id}");
    h.root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| *l == head || l.starts_with(&format!("{head},")))
        .unwrap_or_else(|| panic!("no handle for {head}"))
}

fn game(h: &App) -> &sp_engine::bridge::Game {
    &h.state().screens().bridge.game
}

/// The status pill's text.
fn status(h: &App) -> String {
    game(h).view().status.label
}

/// Builds the 8-joint Howe-style truss over the river (the golden scenario), by clicks.
fn build_howe(h: &mut App) {
    click_button(h, "Member");
    click_joint(h, "J1");
    click_grid(h, 18.0, 0.0);
    click_joint(h, "J3");
    click_grid(h, 38.0, 0.0);
    click_joint(h, "J2");
    click_button(h, "Finish line");
    click_joint(h, "J1");
    click_grid(h, 18.0, 6.0);
    click_grid(h, 28.0, 6.0);
    click_grid(h, 38.0, 6.0);
    click_joint(h, "J2");
    click_button(h, "Finish line");
    for chain in [&["J4", "J6", "J3", "J7"][..], &["J5", "J8", "J3"][..]] {
        for id in chain {
            click_joint(h, id);
        }
        click_button(h, "Finish line");
    }
}

/// Clicks the result banner's "Back to building" (the aside's big button says the same while
/// a result shows, as in the mockup).
fn banner_back(h: &mut App) {
    let stage = stage_rect(h);
    let node = h
        .root()
        .children_recursive()
        .find(|n| {
            n.accesskit_node().role() == Role::Button
                && n.accesskit_node().label().as_deref() == Some("Back to building")
                && stage.contains_rect(n.rect())
        })
        .expect("the banner's Back to building");
    node.click();
    settle(h);
}

/// Runs frames until the result banner shows (or `max` frames).
fn run_until_result(h: &mut App, max: usize) -> bool {
    step_until(h, max, |h| {
        matches!(game(h).phase(), Phase::Held | Phase::Collapsed)
    })
}

/// DONE-4: build level 1 from scratch by clicking, test it, it holds with a star; best stars
/// show on the level row and survive switching levels.
#[test]
fn level_one_from_scratch_holds_with_stars() {
    let mut h = harness(Route::Bridge);
    assert_eq!(document(&h), "river-level");
    assert!(has_label(&h, "Nothing built yet"));
    assert!(has_label(&h, "Cost $0.8k / $30k"));
    build_howe(&mut h);
    assert_eq!(game(&h).design().members.len(), 13);
    assert_eq!(status(&h), "Holding");
    assert!(has_label(&h, "Holding"));
    assert!(has_label(&h, "Cost $29.4k / $30k"));
    assert!(has_label(
        &h,
        "Structure with 8 joints and 13 members. Holding."
    ));
    assert!(has_label_containing(
        &h,
        "Busiest member: M6, squeezed with 399 kN"
    ));
    click_button(&mut h, "Test it: add the load");
    assert_eq!(game(&h).phase(), Phase::Testing);
    assert_eq!(drive_label(&h).as_deref(), Some("Pause"));
    assert!(has_label_containing(&h, "Loading"));
    assert!(run_until_result(&mut h, 600), "the ramp finishes");
    settle(&mut h);
    assert!(has_label(&h, "It held!"));
    assert!(has_label(&h, "★☆☆"));
    assert_eq!(h.state().screens().bridge.clock(), "held");
    assert_eq!(game(&h).best_for(LevelKey::River), 1);
    assert_eq!(
        h.query_all_by_label("Back to building").count(),
        2,
        "the banner and the aside"
    );
    banner_back(&mut h);
    assert_eq!(game(&h).phase(), Phase::Build);
    click_button(&mut h, "Sandbox");
    assert_eq!(document(&h), "sandbox");
    click_button(&mut h, "1 · River crossing");
    assert_eq!(game(&h).best_for(LevelKey::River), 1, "best stars stay");
    assert!(
        game(&h).design().members.is_empty(),
        "the level starts again"
    );
}

/// DONE-4 / BR-7: a thin timber bridge buckles; the banner names the member and offers the
/// slow-motion replay; "Back to building" returns.
#[test]
fn a_thin_timber_bridge_buckles_and_replays() {
    let mut h = harness(Route::Bridge);
    click_button(&mut h, "Timber");
    click_button(&mut h, "Member");
    click_button(&mut h, "Thin");
    assert_eq!(pressed(&h, "Thin"), Some(true));
    build_howe(&mut h);
    assert!(
        status(&h).starts_with("Will break at 6% load"),
        "{}",
        status(&h)
    );
    click_drive(&mut h);
    assert!(run_until_result(&mut h, 900));
    settle(&mut h);
    assert!(has_label(&h, "Collapse: M6 buckled at 7%"));
    assert!(has_label_containing(
        &h,
        "M6 buckled at 7% of the load. Long, thin members in compression bow sideways"
    ));
    assert_eq!(h.state().screens().bridge.clock(), "collapse");
    click_button(&mut h, "Replay in slow motion");
    assert_eq!(game(&h).phase(), Phase::Collapse);
    assert!(game(&h).sim().is_some_and(|s| s.slow));
    assert!(run_until_result(&mut h, 900));
    settle(&mut h);
    assert!(has_label(&h, "Collapse: M6 buckled at 7%"));
    banner_back(&mut h);
    assert_eq!(game(&h).phase(), Phase::Build);
    assert_eq!(
        game(&h).design().members.len(),
        13,
        "the design is still there"
    );
}

/// BR-5 / BR-10: a mechanism rings its free joint and explains itself; testing it folds.
#[test]
fn a_mechanism_is_named_and_folds() {
    let mut h = harness(Route::Bridge);
    click_button(&mut h, "Member");
    click_joint(&mut h, "J1");
    click_grid(&mut h, 18.0, 0.0);
    click_joint(&mut h, "J3");
    click_button(&mut h, "Finish line");
    click_joint(&mut h, "J4");
    click_grid(&mut h, 18.0, 6.0);
    click_button(&mut h, "Finish line");
    assert_eq!(status(&h), "Wobbly: not stable");
    assert!(has_label(&h, "Wobbly: not stable"));
    let explain = game(&h).view().explain;
    assert!(
        explain.ends_with("can still move without stretching any member, so this would fold under load. Add a diagonal to make a triangle, or a support."),
        "{explain}"
    );
    assert!(game(&h).view().bad.is_some(), "the free joint is ringed");
    click_button(&mut h, "Test it: add the load");
    assert!(run_until_result(&mut h, 600));
    settle(&mut h);
    assert!(has_label(&h, "Collapse: it folded"));
}

/// BR-4: every placement rule answers with the mockup's toast.
#[test]
fn placement_rules_toast() {
    let mut h = harness(Route::Bridge);
    let toast = |h: &App| game(h).toast().map(str::to_string);
    click_button(&mut h, "Joint");
    click_grid(&mut h, 4.0, -2.0);
    assert_eq!(toast(&h).as_deref(), Some("That spot is underground."));
    assert!(has_label(&h, "That spot is underground."));
    click_grid(&mut h, 20.0, 4.0);
    assert_eq!(game(&h).design().nodes.len(), 4);
    click_button(&mut h, "Support");
    click_joint(&mut h, "J3");
    assert_eq!(
        toast(&h).as_deref(),
        Some("This support is part of the level.")
    );
    click_joint(&mut h, "J4");
    assert_eq!(toast(&h).as_deref(), Some("Supports go on a riverbank."));
    click_button(&mut h, "Erase");
    click_joint(&mut h, "J1");
    assert_eq!(
        toast(&h).as_deref(),
        Some("This joint is part of the level.")
    );
    click_button(&mut h, "Member");
    click_joint(&mut h, "J1");
    click_joint(&mut h, "J4");
    click_joint(&mut h, "J1");
    assert_eq!(toast(&h).as_deref(), Some("Those two are already joined."));
    click_button(&mut h, "Finish line");
    click_button(&mut h, "Test it: add the load");
    assert_eq!(
        toast(&h).as_deref(),
        Some("Connect the loaded joint to your structure first.")
    );
    assert_eq!(game(&h).phase(), Phase::Build);
    click_button(&mut h, "Select");
    click_joint(&mut h, "J3");
    click_button(&mut h, "Less weight");
    assert_eq!(toast(&h).as_deref(), Some("The level’s load stays on."));
    // toasts fade after 2.6 s
    assert!(step_until(&mut h, 400, |h| game(h).toast().is_none()));
}

/// BR-9: the member and joint inspectors and "Built from".
#[test]
fn inspector_contents() {
    let mut h = harness(Route::Bridge);
    build_howe(&mut h);
    click_button(&mut h, "Select");
    let m3 = member_label(&h, "M3");
    assert_eq!(m3, "Member M3, 11% of its limit");
    click_button(&mut h, &m3);
    assert!(has_label(&h, "M3 · J3–J5"));
    assert!(has_label(&h, "11% of its limit"));
    assert!(has_label(
        &h,
        "Stretched with 216 kN. It will snap at 2000 kN."
    ));
    assert_eq!(pressed(&h, "Medium"), Some(true));
    click_button(&mut h, "Thick");
    assert_eq!(game(&h).design().member("M3").unwrap().size, SizeKey::Thick);
    click_joint(&mut h, "J3");
    assert!(has_label(&h, "Joint J3 · level"));
    assert!(has_label(&h, "x 28 m · y 0 m"));
    assert!(has_label(&h, "Weight 20 t"));
    assert!(!has_label(&h, "Delete joint"), "level joints stay");
    click_joint(&mut h, "J4");
    assert!(has_label(&h, "Joint J4"));
    assert!(has_label(
        &h,
        "In the air. Supports must sit on the ground."
    ));
    click_button(&mut h, "More weight");
    assert_eq!(game(&h).design().load("J4"), 10.0);
    assert!(has_label(&h, "Weight 10 t"));
    // nothing selected: Built from
    click_grid(&mut h, 30.0, 14.0);
    assert!(has_label(&h, "Joints: 8"));
    assert!(has_label(&h, "Members: 13"));
    assert!(has_label_containing(&h, "Weight: "));
    assert!(has_label(&h, "Open in Advanced"));
}

/// ⌘Z and the rail's Undo; Delete removes the selection; arrows nudge a joint 2 m.
#[test]
fn undo_delete_and_nudge() {
    let mut h = harness(Route::Bridge);
    click_button(&mut h, "Joint");
    click_grid(&mut h, 20.0, 4.0);
    click_grid(&mut h, 24.0, 6.0);
    assert_eq!(game(&h).design().nodes.len(), 5);
    press_cmd(&mut h, egui::Key::Z);
    settle(&mut h);
    assert_eq!(game(&h).design().nodes.len(), 4, "⌘Z");
    click_button(&mut h, "Undo");
    assert_eq!(game(&h).design().nodes.len(), 3, "the rail's Undo");
    click_grid(&mut h, 20.0, 4.0);
    assert!(matches!(
        game(&h).selection(),
        Some(sp_engine::bridge::Selection::Joint(_))
    ));
    // focus nothing so the keys reach the stage
    h.remove_cursor();
    press(&mut h, egui::Key::ArrowRight);
    press(&mut h, egui::Key::ArrowUp);
    settle(&mut h);
    let j = game(&h).design().joint("J4").unwrap();
    assert_eq!((j.x, j.y), (22, 6), "nudged 2 m right and 2 m up");
    press(&mut h, egui::Key::Delete);
    settle(&mut h);
    assert!(game(&h).design().joint("J4").is_none(), "Delete");
}

/// Select-drag moves a joint along the grid; a support dragged off the ground drops.
#[test]
fn dragging_a_joint() {
    let mut h = harness(Route::Bridge);
    click_button(&mut h, "Sandbox");
    click_button(&mut h, "Select");
    let from = at(&h, 48.0, 0.0);
    let to = at(&h, 52.0, 4.0);
    h.hover_at(from);
    h.run_steps(2);
    press_at(&mut h, from, true);
    h.run_steps(2);
    for k in 1..=10 {
        let t = k as f32 / 10.0;
        h.hover_at(from + (to - from) * t);
        h.run_steps(2);
    }
    press_at(&mut h, to, false);
    h.run_steps(3);
    let j = game(&h).design().joint("J2").unwrap();
    assert_eq!((j.x, j.y), (52, 4));
    assert_eq!(j.sup, None, "the roller dropped off the ground");
    assert!(game(&h).dragging().is_none());
}

/// Level switching resets to the level's joints, loads and document name (BR-8).
#[test]
fn level_switching() {
    let mut h = harness(Route::Bridge);
    for (row, doc, joints) in [
        ("2 · Overhang", "overhang-level", 3),
        ("3 · Heavy haul", "haul-level", 4),
        ("Sandbox", "sandbox", 2),
        ("1 · River crossing", "river-level", 3),
    ] {
        click_button(&mut h, row);
        assert_eq!(pressed(&h, row), Some(true));
        assert_eq!(document(&h), doc);
        assert_eq!(game(&h).design().nodes.len(), joints);
    }
    click_button(&mut h, "2 · Overhang");
    assert!(has_label_containing(&h, "Hold 5 t out over the water"));
    assert!(has_label(&h, "Cost $0.8k / $14k"));
    click_button(&mut h, "Sandbox");
    click_button(&mut h, "Drop in a starter truss");
    // MB-5: the starter truss is cross-braced and holds, also with 20 t at a middle joint
    assert_eq!(game(&h).design().members.len(), 18);
    assert_eq!(status(&h), "Holding");
    click_button(&mut h, "Weight");
    click_button(&mut h, "10 t");
    click_joint(&mut h, "J3");
    click_joint(&mut h, "J3");
    assert!(has_label_containing(&h, "Joint J3, carrying 20 t"));
    click_button(&mut h, "Test it: add the load");
    assert!(run_until_result(&mut h, 600));
    settle(&mut h);
    assert!(has_label(&h, "It held!"));
}

/// NAV-6: Share copies the `.str` netlist; ⌘V loads one into the Sandbox. NAV-4: "Open in
/// Advanced" carries the netlist; "Open in Playground" brings one back.
#[test]
fn share_paste_and_hand_offs() {
    let mut h = harness(Route::Bridge);
    build_howe(&mut h);
    click_header(&mut h, "Share");
    let text = last_copied(&h).expect("Share copies");
    assert!(text.starts_with("* river-level.str"), "{text}");
    assert!(text.contains(".node J1  8  0  fix=xy"), "{text}");
    assert!(text.contains(".load weights  J3  Fy=-20t"), "{text}");
    // ⌘V: the netlist goes into the Sandbox
    click_button(&mut h, "Clear");
    h.event(egui::Event::Paste(text.clone()));
    settle(&mut h);
    assert_eq!(game(&h).level(), LevelKey::Sandbox);
    assert_eq!(game(&h).design().members.len(), 13);
    assert_eq!(status(&h), "Holding");
    // Open in Advanced
    click_button(&mut h, "Open in Advanced");
    settle(&mut h);
    assert_eq!(route(&h), Route::Advanced);
    let (to, carry) = carried(&h).last().cloned().expect("a hand-off");
    assert_eq!(to, Route::Advanced);
    let Carry::Netlist {
        text: sent, from, ..
    } = carry
    else {
        panic!("a netlist")
    };
    assert_eq!(from, Some(Route::Bridge));
    assert!(sent.starts_with("* sandbox.str"));
    // and back from Advanced ("Open in Playground")
    let mut fresh = harness(Route::Bridge);
    hand_off(
        &mut fresh,
        Route::Bridge,
        Carry::netlist(sent, None, Route::Advanced),
    );
    assert_eq!(game(&fresh).level(), LevelKey::Sandbox);
    assert_eq!(game(&fresh).design().members.len(), 13);
    // a bad netlist toasts and changes nothing
    h.state_mut().navigate(Route::Bridge);
    settle(&mut h);
    let before = game(&h).design().clone();
    h.event(egui::Event::Paste("not a netlist".into()));
    settle(&mut h);
    assert_eq!(game(&h).design(), &before);
    assert!(
        toasts(&h)
            .iter()
            .any(|t| t.starts_with("Couldn't load that netlist"))
    );
}

/// Every control in the hint bar, the rail and the aside has its own space at 1440 and 390
/// (no overlapping buttons), and the page stacks at 390 in the mockup's order.
#[test]
fn renders_at_1440_and_390_without_overlap() {
    for (w, hgt) in [(W, H), (PHONE_W, 2000.0)] {
        let mut h = harness_at(Route::Bridge, w, hgt);
        click_button(&mut h, "Select"); // handles show with Select; nothing to overlap yet
        settle(&mut h);
        let regions: Vec<(String, egui::Rect)> = nodes_with_roles(&h, &[Role::Region])
            .into_iter()
            .map(|(_, l, r)| (l, r))
            .collect();
        let find = |name: &str| {
            regions
                .iter()
                .find(|(l, _)| l == name)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("region {name}"))
        };
        let (tools, stage, aside) = (find("Tools"), find("Stage"), find("Level and inspector"));
        if w > 1000.0 {
            assert!(tools.right() <= stage.left() + 0.5 && stage.right() <= aside.left() + 0.5);
            assert!((aside.width() - 300.5).abs() < 1.0, "{aside:?}");
        } else {
            assert!(
                tools.bottom() <= stage.top() + 0.5,
                "stacked: rail, then stage"
            );
            assert!(stage.bottom() <= aside.top() + 0.5, "then the aside");
        }
        let buttons: Vec<(String, egui::Rect)> = nodes_with_roles(&h, &[Role::Button])
            .into_iter()
            .filter(|(_, l, r)| {
                !l.starts_with("Building grid")
                    && !l.starts_with("Joint ")
                    && !l.starts_with("Member ")
                    && r.min.y > 100.0
            })
            .map(|(_, l, r)| (l, r))
            .collect();
        for (i, (la, a)) in buttons.iter().enumerate() {
            for (lb, b) in &buttons[i + 1..] {
                let x = a.intersect(*b);
                assert!(
                    !(x.width() > 0.5 && x.height() > 0.5),
                    "{w}: '{la}' {a:?} overlaps '{lb}' {b:?}"
                );
            }
        }
    }
}

/// LOOK-1: in the reference state (level 1, nothing built, Member tool) the stable regions
/// stay close to `Playground-1440.png`.
#[test]
fn close_to_the_reference() {
    let mut h = harness(Route::Bridge);
    let img = render(&mut h);
    let mock = load(&mockup_png(Route::Bridge, 1440));
    for (name, region, bound) in [
        ("tools rail", (0, 106, 76, 794), 4.0),
        ("hint bar", (93, 121, 1032, 52), 8.0),
        ("stage", (93, 182, 1032, 553), 4.0),
        ("level rows", (1152, 115, 276, 200), 5.0),
        ("goal panel", (1152, 328, 276, 155), 9.0),
        ("built from", (1152, 495, 276, 165), 9.0),
    ] {
        let d = mean_diff(&img, &mock, Some(region));
        println!("{name}: mean |Δ| {d:.2}");
        assert!(d < bound, "{name}: mean |Δ| {d:.2} ≥ {bound}");
    }
}

/// Screenshots of the Bridge's states for review (`SP_BRIDGE_SHOTS=1`; written to the test's
/// tmp dir): built and holding, forces shown with a member selected, mid-test, mid-collapse,
/// the collapse banner, a mechanism, and the 390 layout with a structure.
#[test]
fn state_gallery() {
    if std::env::var_os("SP_BRIDGE_SHOTS").is_none() {
        return;
    }
    let mut h = harness(Route::Bridge);
    build_howe(&mut h);
    click_button(&mut h, "Select");
    click_button(&mut h, "Show forces");
    let m6 = member_label(&h, "M6");
    click_button(&mut h, &m6);
    println!("{}", screenshot(&mut h, "bridge-built").display());
    click_button(&mut h, "Test it: add the load");
    h.run_steps(40);
    println!("{}", screenshot(&mut h, "bridge-testing").display());
    let mut t = harness(Route::Bridge);
    click_button(&mut t, "Timber");
    click_button(&mut t, "Member");
    click_button(&mut t, "Thin");
    build_howe(&mut t);
    println!("{}", screenshot(&mut t, "bridge-thin").display());
    click_drive(&mut t);
    step_until(&mut t, 400, |h| game(h).phase() == Phase::Collapse);
    t.run_steps(30);
    println!("{}", screenshot(&mut t, "bridge-collapsing").display());
    run_until_result(&mut t, 900);
    settle(&mut t);
    println!("{}", screenshot(&mut t, "bridge-collapsed").display());
    let mut m = harness(Route::Bridge);
    click_button(&mut m, "Sandbox");
    click_button(&mut m, "Drop in a starter truss");
    click_button(&mut m, "Weight");
    click_joint(&mut m, "J3");
    println!("{}", screenshot(&mut m, "bridge-starter").display());
    let mut p = harness_at(Route::Bridge, PHONE_W, 1900.0);
    click_button(&mut p, "Sandbox");
    click_button(&mut p, "Drop in a starter truss");
    println!("{}", screenshot(&mut p, "bridge-390").display());
}
