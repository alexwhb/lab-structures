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
    // a filled .seg group: equal items across the width
    let (a, b) = (rect(&h, "0–25 %"), rect(&h, "0–2 %"));
    assert!((a.width() - b.width()).abs() < 0.5, "{a:?} {b:?}");
    click(&mut h, "0–2 %", Role::Button);
    assert_eq!(h.state().zoom, 1);
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

/// The slider keeps keyboard focus while the arrow keys step it (NAV-5), also on the first
/// press after it got focus.
#[test]
fn slider_steps_with_the_arrow_keys_and_keeps_focus() {
    let mut h = gallery();
    h.get_by_role_and_label(Role::Slider, "Floors").focus();
    h.step();
    // ArrowUp steps up too, and the checkbox above would take egui's arrow-key focus move
    h.key_press(egui::Key::ArrowUp);
    h.step();
    for _ in 0..2 {
        h.key_press(egui::Key::ArrowRight);
        h.step();
    }
    assert_eq!(h.state().floors, 63.0);
    h.key_press(egui::Key::ArrowLeft);
    h.step();
    assert_eq!(h.state().floors, 62.0);
    assert!(h.get_by_role_and_label(Role::Slider, "Floors").is_focused());
}

/// A wrapping `.seg` group puts what doesn't fit on the next row; the select is a ComboBox
/// whose menu picks an option, and ↓ steps it from the keyboard.
#[test]
fn wrapping_seg_and_select() {
    let mut h = gallery();
    let (a, d) = (rect(&h, "Top chord T2–T3"), rect(&h, "Vertical B2–T2"));
    assert!(
        d.top() > a.top() + 20.0,
        "the fourth pick wraps: {a:?} {d:?}"
    );
    click(&mut h, "Vertical B2–T2", Role::Button);
    assert_eq!(h.state().pick, 3);
    let combo = h.get_by_role_and_label(Role::ComboBox, "Material");
    assert_eq!(combo.accesskit_node().value().as_deref(), Some("Steel"));
    click(&mut h, "Material", Role::ComboBox);
    assert!(h.state().select_open);
    click(&mut h, "Glulam", Role::MenuItem);
    assert_eq!(h.state().select, 2);
    assert!(!h.state().select_open);
    h.get_by_role_and_label(Role::ComboBox, "Material").focus();
    h.step();
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(h.state().select, 3);
}

/// A click near the bottom edge of a menu item picks that item, not the next (the targets
/// tile the column instead of overlapping).
#[test]
fn menu_item_targets_do_not_overlap() {
    let mut h = gallery();
    click(&mut h, "Material", Role::ComboBox);
    let steel = h.get_by_role_and_label(Role::MenuItem, "Steel").rect();
    let alu = h.get_by_role_and_label(Role::MenuItem, "Aluminum").rect();
    assert!(steel.bottom() <= alu.top() + 0.01, "{steel:?} {alu:?}");
    // 2 px above the drawn boundary between them: that's Steel
    let drawn_boundary = (steel.bottom() + alu.top()) / 2.0;
    let pos = egui::pos2(steel.center().x, drawn_boundary - 2.0);
    h.event(egui::Event::PointerMoved(pos));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(!h.state().select_open, "an item was picked");
    assert_eq!(h.state().select, 0, "picked Steel, not Aluminum");
}

/// A disabled slider can't be moved by AccessKit (VoiceOver, MCP) and doesn't offer to be;
/// an enabled one can.
#[test]
fn a_disabled_slider_refuses_accesskit_actions() {
    use eframe::egui::accesskit::Action;
    let mut h = gallery();
    let truck = h.get_by_role_and_label(Role::Slider, "Truck");
    assert!(truck.accesskit_node().is_disabled());
    assert!(
        !truck
            .accesskit_node()
            .data()
            .supports_action(Action::Increment)
    );
    assert!(
        !truck
            .accesskit_node()
            .data()
            .supports_action(Action::SetValue)
    );
    truck.increment_accesskit();
    h.run_steps(2);
    h.get_by_role_and_label(Role::Slider, "Truck")
        .set_value_accesskit(90.0);
    h.run_steps(2);
    h.get_by_role_and_label(Role::Slider, "Truck").click();
    h.run_steps(2);
    assert_eq!(h.state().truck, 40.0, "it didn't move");
    // the enabled one does
    let floors = h.get_by_role_and_label(Role::Slider, "Floors");
    assert!(
        floors
            .accesskit_node()
            .data()
            .supports_action(Action::Increment)
    );
    floors.increment_accesskit();
    h.run_steps(2);
    assert_eq!(h.state().floors, 61.0);
    h.get_by_role_and_label(Role::Slider, "Floors")
        .set_value_accesskit(90.0);
    h.run_steps(2);
    assert_eq!(h.state().floors, 90.0);
}

/// A slider with an inverted or non-finite range (or a NaN value) never panics.
#[test]
fn a_slider_with_a_bad_range_never_panics() {
    use structures_playground::widgets::Slider;
    for (lo, hi, v) in [
        (10.0, 0.0, 5.0),
        (f32::NAN, 1.0, 0.5),
        (0.0, f32::INFINITY, 2.0),
        (0.0, 1.0, f32::NAN),
    ] {
        let mut h = Harness::builder()
            .with_size(egui::vec2(400.0, 120.0))
            .with_context_setup(sp_theme::install)
            .build_ui_state(
                move |ui, v: &mut f32| {
                    Slider::new("Bad", v, lo..=hi).show(ui);
                },
                v,
            );
        h.run_steps(2);
        h.get_by_role_and_label(Role::Slider, "Bad").click();
        h.run_steps(2);
        h.get_by_role_and_label(Role::Slider, "Bad")
            .increment_accesskit();
        h.run_steps(2);
        assert!(h.state().is_finite(), "{lo}..={hi} from {v}: {}", h.state());
    }
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

/// The painters kept from the retired v1 screens for the builder's fold-ins draw and are
/// readable: the tensile rig and the stress–strain plot (`widgets::tensile`, LAB-1), the band
/// bar and comparison rows (`widgets::world`, CMF-1 / CMF-2).
#[test]
fn fold_in_painters_draw() {
    let mut h = gallery();
    h.run();
    for (label, role) in [
        ("A steel sample pulled to 6 %", Role::Image),
        ("Stress against strain for the steel sample", Role::Image),
        ("Load cell 12.4 kN", Role::Label),
        ("Comfort: 12 milli-g", Role::Label),
        ("As built: 12.0 mg", Role::Label),
        ("Tapered: 6.6 mg", Role::Label),
    ] {
        let r = h.get_by_role_and_label(role, label).rect();
        assert!(r.width() > 10.0 && r.height() > 10.0, "{label}: {r:?}");
    }
    // the rig paints the sample's colour inside its box
    let img = h.render().expect("render");
    let rig = h
        .get_by_role_and_label(Role::Image, "A steel sample pulled to 6 %")
        .rect();
    let fill = sp_theme::materials::SAMPLES[0].fill;
    let found = (rig.top() as u32..rig.bottom() as u32).any(|y| {
        (rig.left() as u32..rig.right() as u32).any(|x| {
            let p = img.get_pixel(x, y).0;
            (0..3).all(|i| (p[i] as i32 - fill.to_array()[i] as i32).abs() <= 2)
        })
    });
    assert!(found, "the steel sample is drawn");
}
