//! Advanced (SCR-2) in the real app, driven through AccessKit (ADV-1…ADV-10, NAV-4, NAV-6,
//! DONE-2: every number checked here comes from `sp_engine`, not from the test).

mod common;
use common::*;
use eframe::egui::{self, accesskit::Role};
use egui_kittest::kittest::NodeT;
use sp_engine::generate::{self, MaterialKind, Params, Style};
use sp_engine::influence;
use sp_engine::netlist::{self, WriteTruss};
use sp_engine::{Fix, Material, Member, NodalLoad, Node, SolveOptions, Truss};
use structures_playground::{Carry, Route};

fn adv() -> App {
    harness_at(Route::Advanced, 1440.0, 1000.0)
}

fn text(h: &App) -> String {
    h.state().screens().advanced.doc.text.clone()
}

/// Every label the engine says the screen shows for `net` (rows, checks, status, IL tiles).
fn expect_engine_labels(h: &mut App, net: &netlist::Netlist, pick_key: &str) {
    let r = influence::analyse(net).unwrap();
    assert!(
        has_label(h, &r.status.label()),
        "status {}",
        r.status.label()
    );
    for row in &r.rows {
        let label = format!(
            "{} {}: N {} kN, σ {} MPa, limit {}, use {}",
            row.id, row.nodes, row.force, row.stress, row.limit, row.utilisation
        );
        assert!(has_label(h, &label), "missing row {label:?}");
    }
    for c in &r.checks {
        let label = format!("{} {}. {}", c.label, c.value, c.note);
        assert!(has_label(h, &label), "missing check {label:?}");
    }
    let picks = influence::picks(net, &r);
    let pick = picks
        .iter()
        .find(|p| p.key == pick_key)
        .unwrap_or(&picks[0]);
    let il = influence::influence_line(net, pick.member).unwrap();
    let now = influence::force_kn(&r, pick.member);
    let at = format!(
        "At x = {} m: {}",
        sp_engine::jsnum::js_number(r.x),
        influence::now_text(now)
    );
    assert!(has_label(h, &at), "missing {at:?}");
    assert!(has_label(
        h,
        &format!(
            "Worst over the crossing: {}, {}",
            il.peak_text(),
            il.peak_at_text()
        )
    ));
    assert!(has_label(
        h,
        &format!("Limit (red dashes): {}", il.limit_text())
    ));
    assert!(has_label(h, &il.label()));
}

/// Replaces the editor's text through the keyboard: focus, ⌘A, type.
fn replace_text(h: &mut App, new: &str) {
    get(h, "Netlist", Role::MultilineTextInput).focus();
    h.step();
    press_cmd(h, egui::Key::A);
    get(h, "Netlist", Role::MultilineTextInput).type_text(new);
    h.run_steps(3);
}

#[test]
fn defaults_show_the_mockups_numbers() {
    let mut h = adv();
    // the screen shows what the engine computes for the mockup's defaults (ADV-10) ...
    let g = generate::generate(&Params::default());
    expect_engine_labels(&mut h, &g.netlist, "topmid");
    // ... which are the mockup's own numbers (golden-tested in sp-engine; spot-checked here)
    for label in [
        "Holding · worst 40%",
        "M9 T3–T4: N −1004 kN, σ −100 MPa, limit crush, use 40%",
        "M12 T5–B6: N −588 kN, σ −59 MPa, limit buckle 1974, use 30%",
        "Reactions A 353 kN · B 353 kN. Vertical, per truss. They always sum to the total load.",
        "Midspan deflection 53.1 mm = L/903. Illustrative target L/800 for a road bridge.",
        "Steel weight 22.3 t. Both trusses, members only.",
        "At x = 24 m: −1004 kN C",
        "Limit (red dashes): +2500 / −2500 kN",
        "Pratt truss elevation with member forces. Holding.",
        "river-crossing.str",
        "example: 48 m truss",
    ] {
        assert!(has_label(&h, label), "missing {label:?}");
    }
    assert!(has_label(&h, ".plot N(M8) vs truck x"));
    assert_eq!(pressed(&h, "Pratt"), Some(true));
    assert_eq!(pressed(&h, "Top chord T2–T3"), Some(true));
    assert_eq!(document(&h), "river-crossing");
    assert!(has_label(&h, "x = 24.0 m"));
    assert_eq!(drive_label(&h).as_deref(), Some("Drive across"));
    // the netlist in the editor is the mockup's, line for line
    assert_eq!(text(&h), g.text);
    let lines: Vec<&str> = g.text.lines().collect();
    assert_eq!(
        lines[3],
        ".material steel  E=200GPa  fy=250MPa  rho=7850  ; representative"
    );
    assert_eq!(
        lines[40],
        ".load truck  40t  x=24  lane=B0..B6  share=0.5  ; per truss"
    );
}

#[test]
fn toolbar_changes_the_netlist_and_the_results() {
    let mut h = adv();
    click(&mut h, "Howe");
    assert_eq!(pressed(&h, "Howe"), Some(true));
    assert!(text(&h).contains("* 48 m Howe through-truss (one of two)"));
    let howe = generate::generate(&Params {
        style: Style::Howe,
        ..Params::default()
    });
    assert_eq!(text(&h), howe.text);
    expect_engine_labels(&mut h, &howe.netlist, "topmid");

    click_role(&mut h, "Cross-braced", Role::CheckBox);
    assert!(text(&h).contains("Howe, cross-braced through-truss"));
    let braced = generate::generate(&Params {
        style: Style::Howe,
        braced: true,
        ..Params::default()
    });
    let r = influence::analyse(&braced.netlist).unwrap();
    assert!(r.checks[0].note.starts_with("Indeterminate to degree 4"));
    expect_engine_labels(&mut h, &braced.netlist, "topmid");

    // Material: open the select, pick Aluminum
    click_role(&mut h, "Material", Role::ComboBox);
    click(&mut h, "Aluminum");
    assert!(text(&h).contains(".material alu  E=69GPa  fy=240MPa  rho=2700"));
    assert_eq!(
        get(&h, "Material", Role::ComboBox).value().as_deref(),
        Some("Aluminum")
    );

    // Size, Truck and x sliders
    get(&h, "Size", Role::Slider).set_value_accesskit(9.0);
    h.run_steps(2);
    assert!(text(&h).contains(".section chord  A=225cm2  I=A^2"));
    get(&h, "Truck", Role::Slider).set_value_accesskit(77.0);
    h.run_steps(2);
    get(&h, "x", Role::Slider).set_value_accesskit(31.5);
    h.run_steps(2);
    assert!(text(&h).contains(".load truck  77t  x=31.5  lane=B0..B6"));
    assert!(has_label(&h, "x = 31.5 m"));
    let p = Params {
        style: Style::Howe,
        braced: true,
        material: MaterialKind::Aluminum,
        size: 9,
        truck_t: 77.0,
        x: 31.5,
    };
    let g = generate::generate(&p);
    assert_eq!(text(&h), g.text);
    expect_engine_labels(&mut h, &g.netlist, "topmid");

    // ⌘Z undoes the toolbar changes one at a time (no text field focused)
    press_cmd(&mut h, egui::Key::Z);
    h.run_steps(2);
    assert!(text(&h).contains("x=24"), "x undone");
}

#[test]
fn an_error_shows_on_its_line_and_the_last_result_stays() {
    let mut h = adv();
    let good = text(&h);
    let bad = good.replace(".member M9  T3  T4  chord", ".member M9  T3  T9  chord");
    replace_text(&mut h, &bad);
    assert_eq!(text(&h), bad);
    // the error, on its line, with its column
    assert!(has_label(&h, "Line 27: unknown node T9"), "inline message");
    assert!(has_label(&h, "1 error"), "header badge");
    let desc = get(&h, "Netlist", Role::MultilineTextInput)
        .accesskit_node()
        .description();
    assert_eq!(desc.as_deref(), Some("1 error. Line 27: unknown node T9"));
    // the last good result is still on screen (dimmed)
    assert!(h.state().screens().advanced.doc.stale);
    assert!(has_label(&h, "Holding · worst 40%"));
    assert_eq!(
        pressed(&h, "Pratt"),
        Some(false),
        "a custom netlist lights no style"
    );
    // fix it: the generated example is recognised again
    replace_text(&mut h, &good);
    assert!(!has_label(&h, "1 error"));
    assert!(has_label(&h, "example: 48 m truss"));
    assert!(!h.state().screens().advanced.doc.stale);

    // a real edit: a heavier deck is solved and shown
    let heavier = good.replace("Fy=-80", "Fy=-120");
    replace_text(&mut h, &heavier);
    assert!(has_label(&h, "edited"));
    let net = netlist::parse(&heavier).unwrap();
    expect_engine_labels(&mut h, &net, "topmid");
    // the Truck slider still edits the custom netlist in place
    get(&h, "Truck", Role::Slider).set_value_accesskit(10.0);
    h.run_steps(2);
    assert!(text(&h).contains("Fy=-120"));
    assert!(text(&h).contains(".load truck  10t  x=24"));
}

#[test]
fn warnings_for_other_worlds_directives() {
    let mut h = adv();
    let t = format!("{}.wind 30 0.2\n", text(&h));
    replace_text(&mut h, &t);
    assert!(has_label_containing(
        &h,
        ".wind is used by the Tower world; Advanced ignores it"
    ));
    assert!(!h.state().screens().advanced.doc.stale);
}

#[test]
fn drive_across_moves_the_truck() {
    let mut h = adv();
    click_drive(&mut h);
    assert_eq!(drive_label(&h).as_deref(), Some("Pause"));
    let x0 = h.state().screens().advanced.doc.x();
    h.run_steps(6);
    let x1 = h.state().screens().advanced.doc.x();
    assert!(x1 > x0, "{x0} → {x1}");
    assert!(((x1 - x0) * 2.0).fract().abs() < 1e-9, "0.5 m steps");
    assert!(has_label(&h, &format!("x = {:.1} m", x1)));
    // the influence-line tile follows
    assert!(has_label_containing(
        &h,
        &format!("At x = {} m", sp_engine::jsnum::js_number(x1))
    ));
    // dragging x pauses
    get(&h, "x", Role::Slider).set_value_accesskit(5.0);
    h.run_steps(2);
    assert_eq!(drive_label(&h).as_deref(), Some("Drive across"));
    assert_eq!(h.state().screens().advanced.doc.x(), 5.0);
    // it loops back to 0 after 48 m
    get(&h, "x", Role::Slider).set_value_accesskit(48.0);
    h.run_steps(2);
    click_drive(&mut h);
    h.run_steps(2);
    let x = h.state().screens().advanced.doc.x();
    assert!(x < 10.0, "wrapped back to the start: {x}");
}

#[test]
fn influence_line_picks() {
    let mut h = adv();
    for (name, member) in [
        ("Bottom chord B2–B3", "M3"),
        ("End post B0–T1", "M11"),
        ("Vertical B2–T2", "M14"),
        ("Top chord T2–T3", "M8"),
    ] {
        click(&mut h, name);
        assert_eq!(pressed(&h, name), Some(true));
        assert!(
            has_label(&h, &format!(".plot N({member}) vs truck x")),
            "{name}"
        );
        let g = generate::generate(&Params::default());
        let pick = h.state().screens().advanced.doc.pick.clone();
        expect_engine_labels(&mut h, &g.netlist, &pick);
    }
}

/// A Bridge-like structure handed over (ADV-9): the same forces as solving it directly.
#[test]
fn a_bridge_structure_round_trips() {
    let truss = Truss {
        nodes: vec![
            Node::new("J1", 8.0, 0.0, Fix::PIN),
            Node::new("J2", 16.0, 0.0, Fix::FREE),
            Node::new("J3", 24.0, 0.0, Fix::ROLLER),
            Node::new("J4", 12.0, 4.0, Fix::FREE),
            Node::new("J5", 20.0, 4.0, Fix::FREE),
        ],
        members: [(0, 1), (1, 2), (0, 3), (3, 1), (1, 4), (4, 2), (3, 4)]
            .iter()
            .enumerate()
            .map(|(k, (a, b))| Member::box_section(format!("M{}", k + 1), *a, *b, 0.008, 0))
            .collect(),
        materials: vec![Material::new("Steel", 200e9, 250e6, 7850.0)],
    };
    let loads = vec![NodalLoad::down(1, 20.0 * 9810.0)];
    let sections = vec!["Medium".to_string(); 7];
    let text_in = netlist::write_truss(&WriteTruss {
        name: "river-level",
        comment: "Bridge · River crossing",
        truss: &truss,
        sections: &sections,
        loads: &loads,
        self_weight: true,
    });
    let mut h = adv();
    hand_off(
        &mut h,
        Route::Advanced,
        Carry::netlist(text_in.clone(), Some("river-level"), Route::Bridge),
    );
    assert_eq!(text(&h), text_in);
    assert_eq!(document(&h), "river-level");
    assert!(has_label(&h, "from the Bridge"));
    assert!(has_label(&h, "river-level.str"));
    // no truck: Drive across is disabled and the plot says why
    assert!(disabled(&h, "Drive across"));
    assert!(has_label_containing(&h, "No moving load to sweep"));
    // forces match the Bridge's own solve (self-weight half per member end) to the newton
    let mut all = loads.clone();
    all.extend(truss.self_weight(1.0, &[]));
    let direct = truss.solve(&all, &SolveOptions::default()).unwrap();
    let shown = &h.state().screens().advanced.doc.report().unwrap().solution;
    for (a, b) in direct.members.iter().zip(&shown.members) {
        assert!((a.unwrap().force - b.unwrap().force).abs() < 1.0);
    }
    // and the table shows it
    let net = netlist::parse(&text_in).unwrap();
    let r = influence::analyse(&net).unwrap();
    for row in &r.rows {
        assert!(has_label_containing(
            &h,
            &format!("{} {}: N {} kN", row.id, row.nodes, row.force)
        ));
    }
    // "Open in Playground" hands the netlist back to the Bridge
    reveal(&mut h, "Open in Playground", Some(Role::Button));
    click_button(&mut h, "Open in Playground");
    let (to, carry) = carried(&h).last().cloned().unwrap();
    assert_eq!(to, Route::Bridge);
    assert_eq!(route(&h), Route::Bridge);
    match carry {
        Carry::Netlist { text, from, .. } => {
            assert_eq!(text, text_in);
            assert_eq!(from, Some(Route::Advanced));
        }
        c => panic!("{c:?}"),
    }
}

#[test]
fn share_copies_the_netlist_and_reset_restores_the_defaults() {
    let mut h = adv();
    click(&mut h, "Warren");
    click_header(&mut h, "Share");
    let copied = last_copied(&h).unwrap();
    assert_eq!(copied, text(&h));
    assert!(copied.contains("Warren"));
    assert!(toasts(&h).iter().any(|t| t.contains("river-crossing.str")));
    click_header(&mut h, "Reset");
    assert_eq!(text(&h), generate::generate(&Params::default()).text);
    assert_eq!(pressed(&h, "Pratt"), Some(true));
}

#[test]
fn pasting_a_netlist_or_a_material() {
    let mut h = adv();
    let g = generate::generate(&Params {
        style: Style::Warren,
        x: 10.0,
        ..Params::default()
    });
    h.event(egui::Event::Paste(g.text.clone()));
    h.run_steps(2);
    assert_eq!(text(&h), g.text);
    assert!(has_label(&h, "x = 10.0 m"));
    // the Materials bench's Share line swaps the material
    h.event(egui::Event::Paste(
        ".material steel E=200GPa fy=275MPa rho=7850 curve=steel ; tensile test".into(),
    ));
    h.run_steps(2);
    assert!(text(&h).contains("fy=275MPa"));
    let doc = &h.state().screens().advanced.doc;
    assert_eq!(doc.errors(), 0);
    assert_eq!(doc.netlist().unwrap().materials[0].fy, 275e6);
}

#[test]
fn files_opened_from_finder_land_here() {
    let mut h = adv();
    hand_off(
        &mut h,
        Route::Advanced,
        Carry::Netlist {
            text: "* tiny.str\n.material steel\n.section s A=10cm2\n.node A 0 0 fix=xy\n.node B 4 0 fix=y\n.node C 2 2\n.member M1 A C s\n.member M2 C B s\n.member M3 A B s\n.load p C 0 -10\n".into(),
            name: Some("tiny".into()),
            from: None,
        },
    );
    assert_eq!(document(&h), "tiny");
    assert!(has_label(&h, "opened file"));
    assert!(has_label(&h, "Holding · worst 1%") || has_label_containing(&h, "Holding · worst"));
    // a custom netlist picks its most-used members for the influence line
    assert!(has_label_containing(&h, "No moving load to sweep"));
}

/// NAV-5: the editor doesn't trap the keyboard; Tab moves on, and Space types (no drive).
#[test]
fn keyboard_moves_through_the_editor() {
    let mut h = adv();
    get(&h, "Netlist", Role::MultilineTextInput).focus();
    h.step();
    assert_eq!(focused_label(&h).as_deref(), Some("Netlist"));
    press(&mut h, egui::Key::Space);
    h.run_steps(2);
    assert_eq!(
        drive_label(&h).as_deref(),
        Some("Drive across"),
        "Space typed, not drove"
    );
    press(&mut h, egui::Key::Tab);
    h.run_steps(2);
    assert_ne!(
        focused_label(&h).as_deref(),
        Some("Netlist"),
        "Tab leaves the editor"
    );
}
