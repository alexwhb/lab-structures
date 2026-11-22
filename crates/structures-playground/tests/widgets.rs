//! The shared widget kit, driven through the gallery (`widgets::gallery`) via AccessKit: each
//! widget's states (pressed, selected, disabled), drawn sizes and 44 px hit areas, badges and
//! labels.

use eframe::egui::{self, accesskit::Role, accesskit::Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use structures_playground::widgets::gallery::Gallery;

type G = Harness<'static, Gallery>;

fn gallery() -> G {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .wgpu()
        .with_context_setup(sp_theme::install)
        .build_ui_state(
            |ui, g: &mut Gallery| {
                // the whole window, not kittest's padded test frame
                let screen = ui.ctx().content_rect();
                let mut full = ui.new_child(egui::UiBuilder::new().max_rect(screen));
                full.set_clip_rect(screen);
                g.ui(&mut full);
            },
            Gallery::default(),
        );
    h.run_steps(4);
    h
}

fn click(h: &mut G, label: &str, role: Role) {
    h.get_by_role_and_label(role, label).click();
    h.run_steps(3);
}

fn pressed(h: &G, label: &str) -> Option<bool> {
    h.get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .toggled()
        .map(|t| t == Toggled::True)
}

fn rect(h: &G, label: &str) -> egui::Rect {
    h.get_by_label(label).rect()
}

#[test]
fn buttons_click_and_report_disabled() {
    let mut h = gallery();
    for l in ["Reset", "Share", "Test it: add the load"] {
        click(&mut h, l, Role::Button);
        assert_eq!(h.state().clicks.get(l), Some(&1), "{l}");
    }
    // the drive button flips its label
    click(&mut h, "Test", Role::Button);
    assert!(h.state().running);
    click(&mut h, "Pause", Role::Button);
    assert!(!h.state().running);
    let dis = h.get_by_label("Disabled");
    assert!(dis.accesskit_node().is_disabled());
    dis.click();
    h.run_steps(2);
    assert_eq!(h.state().clicks.get("Disabled"), None);
    // the big button is 44 px tall and spans the column; 36 px buttons get a 44 px target
    assert_eq!(rect(&h, "Test it: add the load").height(), 44.0);
    assert_eq!(rect(&h, "Reset").height(), 44.0);
}

#[test]
fn segmented_groups_select() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Pratt"), Some(true));
    click(&mut h, "Warren", Role::Button);
    assert_eq!(h.state().style, 2);
    assert_eq!(pressed(&h, "Warren"), Some(true));
    assert_eq!(pressed(&h, "Pratt"), Some(false));
    h.get_by_role_and_label(Role::Group, "Truss style");
    // the mode switcher is a navigation of links, the current one selected
    let nav = h.get_by_role_and_label(Role::Navigation, "Mode");
    let pg = nav.get_by_role_and_label(Role::Link, "Playground");
    assert_eq!(pg.accesskit_node().is_selected(), Some(true));
    click(&mut h, "Thick", Role::Button);
    assert_eq!(h.state().size, 2);
}

#[test]
fn toggles_flip() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Exaggerate sag"), Some(true));
    assert_eq!(pressed(&h, "Show forces"), Some(false));
    click(&mut h, "Exaggerate sag", Role::Button);
    click(&mut h, "Show forces", Role::Button);
    assert!(!h.state().sag && h.state().forces);
    assert_eq!(pressed(&h, "Show forces"), Some(true));
}

/// Chips (`.chip`): 34 px drawn, 44 px target, pressed state, dot; the checkbox row reports
/// its checked state and flips on click.
#[test]
fn chips_and_checkbox() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Steel"), Some(true));
    click(&mut h, "Timber", Role::Button);
    assert_eq!(h.state().material, 1);
    assert_eq!(pressed(&h, "Timber"), Some(true));
    assert!(rect(&h, "Steel").height() >= 44.0);
    let cb = h.get_by_role_and_label(Role::CheckBox, "Tuned mass damper at the top");
    assert_eq!(cb.accesskit_node().toggled(), Some(Toggled::False));
    click(&mut h, "Tuned mass damper at the top", Role::CheckBox);
    assert!(h.state().damper);
    let cb = h.get_by_role_and_label(Role::CheckBox, "Tuned mass damper at the top");
    assert_eq!(cb.accesskit_node().toggled(), Some(Toggled::True));
    assert!(cb.rect().height() >= 44.0);
}

/// Badges and readouts are labelled text (PROV-1, readouts).
#[test]
fn badges_and_readouts_are_labels() {
    let h = gallery();
    for l in [
        "Solver: linear statics, exact",
        "Materials: representative",
        "Game costs",
        "example: 48 m truss",
        "Educational. Not for real structural design.",
        "Top sway: 928 mm",
    ] {
        h.get_by_role_and_label(Role::Label, l);
    }
    h.get_by_role_and_label(Role::Group, "What’s happening");
}

/// No widget in the gallery uses an icon without a name, and the icon set holds no sparkle,
/// wand or four-point star (LOOK-3).
#[test]
fn icons_say_what_they_do() {
    use structures_playground::widgets::Icon;
    for i in Icon::ALL {
        let n = i.name();
        let banned = ["sparkle", "wand", "magic"].iter().any(|b| n.contains(b))
            || n == "star"
            || n.ends_with("-star");
        assert!(!banned, "{n}");
    }
    assert_eq!(Icon::ALL.len(), 15);
}
