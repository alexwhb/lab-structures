//! Navigation (NAV-1/2/3/4/5/6/7) driven through AccessKit on the real app: the brand opens
//! Series and Back / Esc return; both modes and every world tab land on the right route from
//! every screen, at 1440 and at 390 px; ⌘1–⌘5; Space presses the drive button; Reset and Share
//! reach the screen; hand-offs are delivered after `enter`.

mod common;

use common::*;
use eframe::egui::{Key, accesskit::Role};
use egui_kittest::kittest::Queryable;
use structures_playground::app::AppEvent;
use structures_playground::{Carry, Route};

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
        let last = h.state().last_world();
        click_link(&mut h, "Playground");
        assert_eq!(route(&h), last, "Playground from {from:?} at {w}");

        if from.is_world() {
            for tab in Route::WORLDS {
                goto(&mut h, from);
                click_tab(&mut h, tab.tab_label().unwrap());
                assert_eq!(route(&h), tab, "tab {tab:?} from {from:?} at {w}");
            }
        }
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

/// The tab row is a `TabList` "Playground worlds" on the four worlds only, with the current
/// tab selected; the header is everywhere but Series (NAV-1/2).
#[test]
fn chrome_follows_the_route() {
    let mut h = harness(Route::Bridge);
    for r in Route::ALL {
        goto(&mut h, r);
        let has_tabs = h
            .query_by_role_and_label(Role::TabList, "Playground worlds")
            .is_some();
        assert_eq!(has_tabs, r.is_world(), "{r:?}");
        assert_eq!(
            h.query_by_role(Role::Banner).is_some(),
            r.has_header(),
            "{r:?}"
        );
        if r.is_world() {
            let tabs = child_labels(&h, "Playground worlds", Role::TabList, Role::Tab);
            assert_eq!(tabs, ["Bridge", "Tower", "Ground", "Materials"]);
            for w in Route::WORLDS {
                let label = w.tab_label().unwrap();
                assert_eq!(selected(&h, label, Role::Tab), Some(w == r), "{r:?} {w:?}");
            }
        }
        if r.has_header() {
            assert_eq!(
                child_labels(&h, "Mode", Role::Navigation, Role::Link),
                ["Playground", "Advanced"]
            );
            assert_eq!(selected(&h, "Playground", Role::Link), Some(r.is_world()));
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

/// ⌘1–⌘4 switch worlds, ⌘5 opens Advanced (NAV-5), from anywhere.
#[test]
fn command_digits_switch_screens() {
    let mut h = harness(Route::Advanced);
    let keys = [
        (Key::Num1, Route::Bridge),
        (Key::Num2, Route::Tower),
        (Key::Num3, Route::Ground),
        (Key::Num4, Route::Materials),
        (Key::Num5, Route::Advanced),
    ];
    for (k, r) in keys {
        press_cmd(&mut h, k);
        settle(&mut h);
        assert_eq!(route(&h), r, "{k:?}");
    }
    goto(&mut h, Route::Series);
    press_cmd(&mut h, Key::Num3);
    settle(&mut h);
    assert_eq!(route(&h), Route::Ground, "from Series too");
}

/// Esc on the Series screen returns to where it was opened from (NAV-3); "Playground" then
/// opens the last world used.
#[test]
fn escape_leaves_series_and_playground_remembers_the_world() {
    let mut h = harness(Route::Bridge);
    click_tab(&mut h, "Materials");
    click_link(&mut h, "Advanced");
    click_link(&mut h, "Structures Playground");
    assert_eq!(route(&h), Route::Series);
    press(&mut h, Key::Escape);
    settle(&mut h);
    assert_eq!(route(&h), Route::Advanced, "Esc returns to Advanced");
    click_link(&mut h, "Playground");
    assert_eq!(route(&h), Route::Materials, "the last world");
}

/// Space presses the drive button when nothing has focus, and the header's button does the
/// same; Reset and Share reach the screen; Share copies and toasts (NAV-5/6).
#[test]
fn drive_reset_and_share_reach_the_screen() {
    let mut h = harness(Route::Tower);
    let before = drive_label(&h).unwrap();
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(events(&h), [AppEvent::Drive(Route::Tower)], "Space");
    click_drive(&mut h);
    assert_eq!(events(&h).len(), 2, "the header button");
    assert_eq!(drive_label(&h).unwrap(), before, "pressed twice: back");
    click_header(&mut h, "Reset");
    assert_eq!(events(&h).last(), Some(&AppEvent::Reset(Route::Tower)));
    click_header(&mut h, "Share");
    assert_eq!(events(&h).last(), Some(&AppEvent::Shared(Route::Tower)));
    let copied = last_copied(&h).expect("Share copies");
    assert!(!copied.is_empty());
    assert!(!toasts(&h).is_empty(), "Share confirms with a toast");
}

/// Space doesn't drive while a text field has focus (it types).
#[test]
fn space_types_in_a_text_field() {
    let mut h = harness(Route::Bridge);
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
    use structures_playground::Command;
    let mut h = harness(Route::Bridge);
    press_cmd(&mut h, Key::Z);
    press(&mut h, Key::Delete);
    press(&mut h, Key::Backspace);
    h.event(eframe::egui::Event::Paste(".node A 0 0 fix=xy".into()));
    h.step();
    let cmds: Vec<Command> = events(&h)
        .into_iter()
        .filter_map(|e| match e {
            AppEvent::Command(Route::Bridge, c, _) => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(
        cmds,
        [
            Command::Undo,
            Command::Delete,
            Command::Delete,
            Command::Paste(".node A 0 0 fix=xy".into())
        ]
    );
}

/// A hand-off reaches its screen after `enter`, also when the screen was left before it
/// arrived (NAV-4: Tower → Ground, Ground → Tower, Bridge → Advanced, Advanced → Bridge).
#[test]
fn hand_offs_are_delivered() {
    let mut h = harness(Route::Tower);
    let building = Carry::Building {
        floors: 60,
        base_shear_n: 1.5e7,
        overturning_nm: 2.0e9,
    };
    hand_off(&mut h, Route::Ground, building.clone());
    assert_eq!(route(&h), Route::Ground);
    hand_off(&mut h, Route::Tower, Carry::Tower { floors: 60 });
    assert_eq!(route(&h), Route::Tower);
    let netlist = Carry::netlist(".node A 0 0 fix=xy", Some("mine"), Route::Bridge);
    hand_off(&mut h, Route::Advanced, netlist.clone());
    let back = Carry::netlist(".node A 0 0 fix=xy", None, Route::Advanced);
    hand_off(&mut h, Route::Bridge, back.clone());
    assert_eq!(
        carried(&h),
        [
            (Route::Ground, building),
            (Route::Tower, Carry::Tower { floors: 60 }),
            (Route::Advanced, netlist),
            (Route::Bridge, back),
        ]
    );
    // navigating away in the same frame still delivers it to its target
    h.state_mut()
        .navigate_with(Route::Advanced, Carry::netlist("x", None, Route::Bridge));
    h.state_mut().navigate(Route::Tower);
    settle(&mut h);
    assert_eq!(carried(&h).last().unwrap().0, Route::Advanced);
}
