//! Navigation (NAV-1/2/3/4/5/6/7) driven through AccessKit on the real app: the brand opens
//! Series and Back / Esc return; both modes land on the right route from every screen, at 1440
//! and at 390 px; there is no world tab row (v2); ⌘1 / ⌘2; Space presses the drive button;
//! Reset and Share reach the screen; the builder's keys (zoom, pan, nudge, Esc) arrive as
//! commands on the Playground only; hand-offs are delivered after `enter`.

mod common;

use common::*;
use eframe::egui::{Key, accesskit::Role};
use egui_kittest::kittest::Queryable;
use structures_playground::app::AppEvent;
use structures_playground::{Carry, Command, Route};

fn check_header_paths(w: f32, hgt: f32) {
    let mut h = app().size(w, hgt).build();
    for from in Route::ALL.into_iter().filter(|r| r.has_header()) {
        goto(&mut h, from);
        click_link(&mut h, "Structures Playground");
        assert_eq!(route(&h), Route::Series, "brand from {from:?} at {w}");
        click_button(&mut h, "Back");
        assert_eq!(route(&h), from, "Back to {from:?} at {w}");

        goto(&mut h, from);
        click_link(&mut h, "Advanced");
        assert_eq!(route(&h), Route::Advanced, "Advanced from {from:?} at {w}");

        goto(&mut h, from);
        click_link(&mut h, "Playground");
        assert_eq!(
            route(&h),
            Route::Playground,
            "Playground from {from:?} at {w}"
        );
    }
}

#[test]
fn header_paths_from_every_screen_desktop() {
    check_header_paths(W, H);
}

#[test]
fn header_paths_from_every_screen_phone() {
    check_header_paths(PHONE_W, PHONE_H);
}

/// There is no world tab row anywhere (v2, NAV-2); the header is everywhere but Series, with
/// the current mode selected (NAV-1).
#[test]
fn chrome_follows_the_route() {
    let mut h = harness(Route::Playground);
    for r in Route::ALL {
        goto(&mut h, r);
        assert!(
            h.query_by_role(Role::TabList).is_none(),
            "{r:?}: no world tab row in v2"
        );
        // (the screens draw their own provenance: the builder's line, Advanced's badges)
        assert!(
            h.query_by_role(Role::Banner).is_none_or(|b| b
                .query_by_role_and_label(Role::Group, "Provenance")
                .is_none()),
            "{r:?}: no provenance in the chrome"
        );
        assert_eq!(
            h.query_by_role(Role::Banner).is_some(),
            r.has_header(),
            "{r:?}"
        );
        if r.has_header() {
            assert_eq!(
                child_labels(&h, "Mode", Role::Navigation, Role::Link),
                ["Playground", "Advanced"]
            );
            assert_eq!(
                selected(&h, "Playground", Role::Link),
                Some(r == Route::Playground)
            );
            assert_eq!(
                selected(&h, "Advanced", Role::Link),
                Some(r == Route::Advanced)
            );
            // NAV-1: the document name is shown (and readable)
            assert!(has_label(&h, &document(&h)), "{r:?}: {}", document(&h));
            for b in ["Reset", "Share"] {
                header_button(&h, b);
            }
        }
    }
}

/// ⌘1 opens the Playground, ⌘2 Advanced (NAV-5), from anywhere; the retired ⌘3–⌘5 do
/// nothing.
#[test]
fn command_digits_switch_screens() {
    let mut h = harness(Route::Advanced);
    for (k, r) in [(Key::Num1, Route::Playground), (Key::Num2, Route::Advanced)] {
        press_cmd(&mut h, k);
        settle(&mut h);
        assert_eq!(route(&h), r, "{k:?}");
    }
    for k in [Key::Num3, Key::Num4, Key::Num5] {
        press_cmd(&mut h, k);
        settle(&mut h);
        assert_eq!(route(&h), Route::Advanced, "{k:?} is retired");
    }
    goto(&mut h, Route::Series);
    press_cmd(&mut h, Key::Num1);
    settle(&mut h);
    assert_eq!(route(&h), Route::Playground, "from Series too");
}

/// Esc on the Series screen returns to where it was opened from (NAV-3).
#[test]
fn escape_leaves_series() {
    let mut h = harness(Route::Playground);
    click_link(&mut h, "Advanced");
    click_link(&mut h, "Structures Playground");
    assert_eq!(route(&h), Route::Series);
    press(&mut h, Key::Escape);
    settle(&mut h);
    assert_eq!(route(&h), Route::Advanced, "Esc returns to Advanced");
    click_link(&mut h, "Structures Playground");
    click_button(&mut h, "Back");
    assert_eq!(route(&h), Route::Advanced);
    // opened first thing: Back goes to the Playground
    let mut h = harness(Route::Series);
    press(&mut h, Key::Escape);
    settle(&mut h);
    assert_eq!(route(&h), Route::Playground);
}

/// Space presses the drive button when nothing has focus, and the header's button does the
/// same; Reset and Share reach the screen; Share copies and toasts (NAV-5/6).
#[test]
fn drive_reset_and_share_reach_the_screen() {
    let mut h = harness(Route::Advanced);
    let before = drive_label(&h).unwrap();
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(events(&h), [AppEvent::Drive(Route::Advanced)], "Space");
    click_drive(&mut h);
    assert_eq!(events(&h).len(), 2, "the header button");
    assert_eq!(drive_label(&h).unwrap(), before, "pressed twice: back");
    click_header(&mut h, "Reset");
    assert_eq!(events(&h).last(), Some(&AppEvent::Reset(Route::Advanced)));
    click_header(&mut h, "Share");
    assert_eq!(events(&h).last(), Some(&AppEvent::Shared(Route::Advanced)));
    let copied = last_copied(&h).expect("Share copies");
    assert!(!copied.is_empty());
    assert!(!toasts(&h).is_empty(), "Share confirms with a toast");
}

/// The Playground's drive button is "Run traffic" on the first level and Space presses it
/// (NAV-1, NAV-5).
#[test]
fn space_runs_the_builder() {
    let mut h = harness(Route::Playground);
    assert_eq!(drive_label(&h).as_deref(), Some("Run traffic"));
    assert!(has_label(&h, "river-level"), "the level's document name");
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(events(&h), [AppEvent::Drive(Route::Playground)]);
}

/// Space doesn't drive while a text field has focus (it types).
#[test]
fn space_types_in_a_text_field() {
    let mut h = harness(Route::Playground);
    // focus something, then nothing: Space only drives with no focus
    h.get_by_role_and_label(Role::Link, "Advanced").focus();
    h.step();
    press(&mut h, Key::Space);
    settle(&mut h);
    assert!(
        !events(&h).iter().any(|e| matches!(e, AppEvent::Drive(_))),
        "a focused control takes Space itself"
    );
}

/// ⌘Z, Delete / Backspace and ⌘V reach the screen as commands when no text field has focus
/// (NAV-5 / NAV-6).
#[test]
fn keyboard_commands_reach_the_screen() {
    for r in [Route::Playground, Route::Advanced] {
        let mut h = harness(r);
        press_cmd(&mut h, Key::Z);
        press(&mut h, Key::Delete);
        press(&mut h, Key::Backspace);
        h.event(eframe::egui::Event::Paste(".node A 0 0 fix=xy".into()));
        h.step();
        let cmds: Vec<Command> = commands(&h).into_iter().map(|(_, c, _)| c).collect();
        assert_eq!(
            cmds,
            [
                Command::Undo,
                Command::Delete,
                Command::Delete,
                Command::Paste(".node A 0 0 fix=xy".into())
            ],
            "{r:?}"
        );
    }
}

/// The builder's keys (BLD-2, NAV-5): ⌘= / ⌘+ zoom in, ⌘− out, ⌘0 fits, the arrows pan, ⌥
/// arrows nudge, Esc ends a chain — as commands to the Playground, not egui's whole-window
/// zoom.
#[test]
fn builder_keys_reach_the_playground() {
    use eframe::egui::Modifiers;
    let mut h = harness(Route::Playground);
    let zoom = h.ctx.zoom_factor();
    press_cmd(&mut h, Key::Equals);
    press_cmd(&mut h, Key::Plus);
    press_cmd(&mut h, Key::Minus);
    press_cmd(&mut h, Key::Num0);
    for k in [
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
    ] {
        press(&mut h, k);
    }
    press_alt(&mut h, Key::ArrowUp);
    press_alt(&mut h, Key::ArrowLeft);
    press(&mut h, Key::Escape);
    // ⌘⇧= (⌘+ on a US keyboard) zooms in too
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Equals);
    h.step();
    settle(&mut h);
    let cmds: Vec<Command> = commands(&h).into_iter().map(|(_, c, _)| c).collect();
    assert_eq!(
        cmds,
        [
            Command::ZoomIn,
            Command::ZoomIn,
            Command::ZoomOut,
            Command::ZoomFit,
            Command::Pan { dx: -1, dy: 0 },
            Command::Pan { dx: 1, dy: 0 },
            Command::Pan { dx: 0, dy: 1 },
            Command::Pan { dx: 0, dy: -1 },
            Command::Nudge { dx: 0, dy: 1 },
            Command::Nudge { dx: -1, dy: 0 },
            Command::Escape,
            Command::ZoomIn,
        ]
    );
    assert!(commands(&h).iter().all(|(r, _, _)| *r == Route::Playground));
    assert_eq!(h.ctx.zoom_factor(), zoom, "the window's zoom is untouched");
    assert_eq!(route(&h), Route::Playground);
}

/// Elsewhere the builder's keys are not commands: Advanced keeps egui's arrow-key focus moves
/// and Esc, and the arrows don't become pans while a button has focus on the Playground.
#[test]
fn builder_keys_stay_on_the_playground() {
    let mut h = harness(Route::Advanced);
    press(&mut h, Key::ArrowLeft);
    press(&mut h, Key::Escape);
    press_cmd(&mut h, Key::Num0);
    assert!(commands(&h).is_empty(), "{:?}", commands(&h));
    let mut h = harness(Route::Playground);
    h.get_by_role_and_label(Role::Link, "Advanced").focus();
    h.step();
    press(&mut h, Key::ArrowRight);
    settle(&mut h);
    assert!(
        !commands(&h)
            .iter()
            .any(|(_, c, _)| matches!(c, Command::Pan { .. })),
        "a focused button keeps egui's arrow-key focus moves"
    );
}

/// A hand-off reaches its screen after `enter`, also when the screen was left before it
/// arrived (NAV-4: builder → Advanced, Advanced → builder).
#[test]
fn hand_offs_are_delivered() {
    let mut h = harness(Route::Playground);
    let netlist = Carry::netlist(".node A 0 0 fix=xy", Some("mine"), Route::Playground);
    hand_off(&mut h, Route::Advanced, netlist.clone());
    assert_eq!(route(&h), Route::Advanced);
    let back = Carry::netlist(".node A 0 0 fix=xy", None, Route::Advanced);
    hand_off(&mut h, Route::Playground, back.clone());
    assert_eq!(route(&h), Route::Playground);
    assert_eq!(
        carried(&h),
        [(Route::Advanced, netlist), (Route::Playground, back)]
    );
    // navigating away in the same frame still delivers it to its target
    h.state_mut().navigate_with(
        Route::Advanced,
        Carry::netlist("x", None, Route::Playground),
    );
    h.state_mut().navigate(Route::Series);
    settle(&mut h);
    assert_eq!(carried(&h).last().unwrap().0, Route::Advanced);
    assert_eq!(route(&h), Route::Series);
}

/// Space still drives after a mouse click on a button (clicking doesn't leave keyboard focus
/// on it) (NAV-5).
#[test]
fn space_drives_after_a_mouse_click() {
    let mut h = harness(Route::Playground);
    click_header(&mut h, "Reset");
    press(&mut h, Key::Space);
    settle(&mut h);
    assert!(
        events(&h).contains(&AppEvent::Drive(Route::Playground)),
        "{:?}, focus {:?}",
        events(&h),
        focused_label(&h)
    );
}

/// Two hand-offs before the next frame (two .str files opened together, a hand-off plus an MCP
/// call) both arrive, in order (NAV-4).
#[test]
fn hand_offs_queue_up() {
    let mut h = harness(Route::Advanced);
    h.state_mut().navigate_with(
        Route::Playground,
        Carry::netlist("* p.str", None, Route::Advanced),
    );
    h.state_mut().navigate_with(
        Route::Advanced,
        Carry::netlist("* a.str", None, Route::Playground),
    );
    h.state_mut().navigate_with(
        Route::Advanced,
        Carry::netlist("* b.str", None, Route::Playground),
    );
    settle(&mut h);
    let got: Vec<Route> = carried(&h).into_iter().map(|(r, _)| r).collect();
    assert_eq!(got, [Route::Playground, Route::Advanced, Route::Advanced]);
    assert_eq!(route(&h), Route::Advanced);
}

/// ⌘⇧Z (redo, not offered) is never taken as undo; ⌘Z is (NAV-5).
#[test]
fn command_shift_z_is_not_undo() {
    use eframe::egui::Modifiers;
    let mut h = harness(Route::Playground);
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    h.step();
    let undo = |h: &App| {
        events(h)
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    AppEvent::Command(_, structures_playground::Command::Undo, _)
                )
            })
            .count()
    };
    assert_eq!(undo(&h), 0);
    press_cmd(&mut h, Key::Z);
    assert_eq!(undo(&h), 1);
}

/// Holding Space presses the drive button once, not on every key repeat (NAV-5).
#[test]
fn holding_space_drives_once() {
    use eframe::egui::{Event, Modifiers};
    let mut h = harness(Route::Playground);
    let key = |pressed, repeat| Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed,
        repeat,
        modifiers: Modifiers::NONE,
    };
    h.event(key(true, false));
    h.step();
    for _ in 0..5 {
        h.event(key(true, true));
        h.step();
    }
    h.event(key(false, false));
    h.step();
    let drives = events(&h)
        .iter()
        .filter(|e| matches!(e, AppEvent::Drive(_)))
        .count();
    assert_eq!(drives, 1);
}

/// Space presses the drive button while a slider has keyboard focus (only a text field keeps
/// Space; NAV-5), and the slider keeps focus.
#[test]
fn space_drives_while_a_slider_has_focus() {
    let mut h = harness(Route::Advanced);
    h.get_by_role_and_label(Role::Slider, "Truck").focus();
    h.step();
    press(&mut h, Key::ArrowRight);
    settle(&mut h);
    assert_eq!(focused_label(&h).as_deref(), Some("Truck"));
    press(&mut h, Key::Space);
    settle(&mut h);
    let drives = events(&h)
        .iter()
        .filter(|e| matches!(e, AppEvent::Drive(Route::Advanced)))
        .count();
    assert_eq!(drives, 1);
}

/// A long document name is cut with an ellipsis at 390 px: the brand stays inside the window
/// and the full name stays readable to assistive tech (NAV-1).
#[test]
fn long_document_names_fit_at_390() {
    let name = "river-crossing-warren-truss-final-v2";
    let mut h = app().at(Route::Advanced).phone().build();
    hand_off(
        &mut h,
        Route::Advanced,
        Carry::Netlist {
            text: format!(
                "* {name}.str\n.material steel E=200GPa fy=250MPa rho=7850\n.section c A=100cm2 I=A^2\n.node B0 0 0 fix=xy\n.node B1 8 0 fix=y\n.node T1 4 4\n.member M1 B0 B1 c\n.member M2 B0 T1 c\n.member M3 T1 B1 c\n.load dead T1 Fy=-80\n"
            ),
            name: Some(name.into()),
            from: None,
        },
    );
    assert_eq!(document(&h), name);
    let label = h.get_by_label(name).rect();
    assert!(label.right() <= PHONE_W - 15.0, "{label:?}");
    let brand = get(&h, "Structures Playground", Role::Link).rect();
    assert!(brand.right() <= PHONE_W, "{brand:?}");
}
