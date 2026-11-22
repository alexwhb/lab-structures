//! The Series screen (SCR-6, SR-1) on the real app: every section of the mockup is there with
//! its content, the cards open the siblings through the launcher (installed → "Opening …",
//! not installed, planned), Back / Esc return (NAV-3), it lays out at 390 px with one scroller,
//! and it stays close to its reference at 1440.

mod common;

use common::*;
use eframe::egui::{Key, accesskit::Role};
use egui_kittest::kittest::Queryable;
use structures_playground::{AppId, Launcher, Route};

#[test]
fn every_section_is_there() {
    let h = harness(Route::Series);
    for heading in [
        "Shared engines, and who uses them",
        "Bridges to the other apps",
        "Netlist directives",
        "Challenge ladder",
        "Next layers",
        "Provenance",
    ] {
        get(&h, heading, Role::Group);
        get(&h, heading, Role::Heading);
    }
    for text in [
        "Series foundations · app 5 of 5",
        "What Structures adds to the series",
        "Educational. Not for real structural design.",
        ".foundation footing | mat | piles L n | caisson",
        "River crossing: 20 t, under $30k",
        "Boss: quake + hurricane on soft clay",
        "Vortex shedding in the wind tunnel",
        "Exact for the model: truss statics, Euler buckling",
        "Illustrative: soil presets, comfort bands, game costs",
        "Breadbox · vibration is an RLC circuit",
        "Automata explorer · grain growth",
    ] {
        assert!(has_label(&h, text), "missing {text:?}");
    }
    // the table: a row per engine with what each app does
    assert!(has_label(
        &h,
        "Eigen solver: Circuit not yet, Chem not yet, Bio not yet, Weather not yet; \
         Structures new: modes, buckling"
    ));
    assert!(has_label_containing(
        &h,
        "Playground ⇄ Advanced toggle: Circuit uses it"
    ));
    // no header on the Series screen (NAV-1)
    assert!(h.query_by_role(Role::Banner).is_none());
}

/// Cards open the siblings through the launcher and say what happened (SR-1).
#[test]
fn cards_open_the_siblings() {
    let launcher = Launcher::recording(&[AppId::Chem]);
    let mut h = app().at(Route::Series).launcher(launcher.clone()).build();
    click_button(&mut h, "Chem Playground");
    assert!(toasts(&h).contains(&"Opening Chem Playground…".to_string()));
    click_button(&mut h, "Breadbox");
    assert!(toasts(&h).contains(&"Breadbox isn't installed on this Mac".to_string()));
    click_button(&mut h, "Weather Playground");
    assert!(
        toasts(&h)
            .iter()
            .any(|t| t.contains("Weather Playground is planned")),
        "{:?}",
        toasts(&h)
    );
    assert_eq!(
        launcher.calls(),
        [AppId::Chem, AppId::Breadbox, AppId::Weather]
    );
    assert_eq!(route(&h), Route::Series, "opening an app stays here");
}

/// Back and Esc return to where the Series was opened from (NAV-3).
#[test]
fn back_and_escape_return() {
    let mut h = harness(Route::Ground);
    click_link(&mut h, "Structures Playground");
    assert_eq!(route(&h), Route::Series);
    click_button(&mut h, "Back");
    assert_eq!(route(&h), Route::Ground);
    goto(&mut h, Route::Advanced);
    click_link(&mut h, "Structures Playground");
    press(&mut h, Key::Escape);
    settle(&mut h);
    assert_eq!(route(&h), Route::Advanced);
}

/// At 390 px everything stacks in the mockup's order inside the window's width, and the page
/// is the only scroller.
#[test]
fn phone_layout_stacks() {
    let h = app().at(Route::Series).phone().build();
    let order = [
        "Shared engines, and who uses them",
        "Bridges to the other apps",
        "Netlist directives",
        "Challenge ladder",
        "Next layers",
        "Provenance",
    ];
    let rects: Vec<_> = order
        .iter()
        .map(|l| get(&h, l, Role::Group).rect())
        .collect();
    for w in rects.windows(2) {
        assert!(w[1].top() > w[0].bottom(), "{rects:?}");
    }
    for (l, r) in order.iter().zip(&rects) {
        assert!(
            r.left() >= 15.0 && r.right() <= PHONE_W - 15.0,
            "{l}: {r:?}"
        );
    }
    // the provenance badges wrap inside the panel
    let prov = get(&h, "Provenance", Role::Group).rect();
    let badge = h
        .get_by_label(
            "Simplified: wind profile, sway, soil bearing and settlement, collapse physics",
        )
        .rect();
    assert!(prov.contains_rect(badge), "{badge:?} in {prov:?}");
    assert!(h.state().content_height() > 2000.0, "one long page");
}

/// LOOK-1: the 1440 render stays close to the reference.
#[test]
fn stays_close_to_the_reference() {
    let img = render_full(Route::Series, 1440.0, 1400.0);
    let mock = load(&mockup_png(Route::Series, 1440));
    // the intro and the cards (Back sits in the padding above them)
    let top = mean_diff(&img, &mock, Some((0, 50, 1440, 260)));
    assert!(top < 8.0, "intro + cards: {top:.2}");
    let all = mean_diff(&img, &mock, None);
    assert!(all < 9.0, "whole board: {all:.2}");
}
