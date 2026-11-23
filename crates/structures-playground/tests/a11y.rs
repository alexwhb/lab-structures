//! Accessibility of the chrome on every route at 1440 and 390 px (NAV-1/5): every control in
//! the header has a role and a name, targets reach 44 × 44 without covering another control's
//! centre, the current mode reports selected; Tab walks the header in reading order; the
//! Series screen's controls are named too.

mod common;

use common::*;
use eframe::egui::{self, Key, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use structures_playground::Route;

const INTERACTIVE: &[Role] = &[Role::Button, Role::Link, Role::Tab];

/// (role, label, rect) of the interactive nodes under the banner.
fn chrome_nodes(h: &App) -> Vec<(Role, String, egui::Rect)> {
    let Some(p) = h.query_by_role(Role::Banner) else {
        return Vec::new();
    };
    p.children_recursive()
        .filter(|n| INTERACTIVE.contains(&n.accesskit_node().role()))
        .map(|n| {
            let ak = n.accesskit_node();
            (ak.role(), ak.label().unwrap_or_default(), n.rect())
        })
        .collect()
}

fn check(w: f32, hgt: f32) {
    let mut h = app().size(w, hgt).build();
    for r in Route::ALL.into_iter().filter(|r| r.has_header()) {
        goto(&mut h, r);
        let nodes = chrome_nodes(&h);
        // brand, 2 modes, drive, Reset, Share
        assert!(nodes.len() >= 6, "{r:?} at {w}: {nodes:?}");
        for (role, label, rect) in &nodes {
            assert!(!label.trim().is_empty(), "{r:?} at {w}: unnamed {role:?}");
            assert!(
                rect.width() >= 43.99 && rect.height() >= 43.99,
                "{r:?} at {w}: {role:?} {label:?} is {:.1} × {:.1}",
                rect.width(),
                rect.height()
            );
        }
        for (i, (_, a, ra)) in nodes.iter().enumerate() {
            for (j, (_, b, rb)) in nodes.iter().enumerate() {
                if i != j {
                    assert!(
                        !ra.contains(rb.center()),
                        "{r:?} at {w}: {a:?}'s target covers {b:?}'s centre"
                    );
                }
            }
        }
    }
}

#[test]
fn chrome_controls_are_named_and_big_enough_desktop() {
    check(W, H);
}

#[test]
fn chrome_controls_are_named_and_big_enough_phone() {
    check(PHONE_W, PHONE_H);
}

/// Tab walks the header in reading order — brand, Playground, Advanced, the drive button,
/// Reset, Share — before the screen's body (NAV-5).
#[test]
fn tab_order_follows_reading_order() {
    for r in [Route::Playground, Route::Advanced] {
        let mut h = harness(r);
        let mut seen = Vec::new();
        for _ in 0..6 {
            press(&mut h, Key::Tab);
            if let Some(l) = focused_label(&h) {
                seen.push(l);
            }
        }
        let drive = drive_label(&h).unwrap();
        let want = [
            "Structures Playground",
            "Playground",
            "Advanced",
            drive.as_str(),
            "Reset",
            "Share",
        ];
        assert_eq!(&seen[..want.len()], &want[..], "{r:?}: {seen:?}");
    }
}

/// Enter on a focused mode link activates it (NAV-5).
#[test]
fn enter_activates_the_focused_mode() {
    let mut h = harness(Route::Playground);
    h.get_by_role_and_label(Role::Link, "Advanced").focus();
    h.step();
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(route(&h), Route::Advanced);
}

/// Every route is reachable from the keyboard alone: Tab to the brand + Enter opens Series,
/// Tab to Back + Enter returns; ⌘1 / ⌘2 do the rest (`nav.rs`) (NAV-5).
#[test]
fn series_and_back_by_keyboard() {
    let mut h = harness(Route::Advanced);
    press(&mut h, Key::Tab);
    assert_eq!(focused_label(&h).as_deref(), Some("Structures Playground"));
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(route(&h), Route::Series);
    press(&mut h, Key::Tab);
    assert_eq!(focused_label(&h).as_deref(), Some("Back"));
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(route(&h), Route::Advanced);
}

/// The Series screen: Back is a named button, each sibling card a button, the Structures card a
/// label; the disclaimer is readable.
#[test]
fn series_controls_are_named() {
    let h = harness(Route::Series);
    get(&h, "Back", Role::Button);
    for app in [
        "Breadbox",
        "Chem Playground",
        "Bio Playground",
        "Weather Playground",
    ] {
        let r = get(&h, app, Role::Button).rect();
        assert!(r.width() >= 44.0 && r.height() >= 44.0, "{app}");
    }
    get(&h, "Structures Playground (this app)", Role::Label);
    get(
        &h,
        "Educational. Not for real structural design.",
        Role::Label,
    );
}
