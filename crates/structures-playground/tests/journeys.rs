//! End-to-end journeys across the screens (SPEC DONE-4, v2), in the real app, driven and read
//! through AccessKit only: clicks on named buttons, links and grid points of the builder's
//! stage, keys, AccessKit slider values. Nothing reaches into a screen's state; expected
//! numbers come from `sp_engine` run independently (DONE-2 for the tests too).
//!
//! 1. Level 1 built by clicking (Road + Wood triangles) → Run traffic → all across, stars.
//! 2. Heavy haul in a wood truss collapses under the truck and snow → replay in slow motion.
//! 3. A cable-stayed Sandbox bridge with looping traffic and a wind change mid-run.
//! 4. A tower built to 64 m rides out the storm: comfort meter, "Compare shapes" (CMF).
//! 5. Shaky ground: footings liquefy, piles fix it.
//! 6. The Leaning tower fixed over 30 years with piles under its three footings (TIME-3).
//! 7. A material card's sample pulled to breaking (LAB).
//! 8. A fire softens a steel truss (FIRE).
//! 9. Open in Advanced shows the same structure; an error in the netlist, fixed, then "Open in
//!    Playground" loads it into the Sandbox.
//! 10. Keyboard only: ⌘1 / ⌘2, Space drives, Tab reaches the drive button, brand → Series →
//!     Esc; build and zoom by keyboard.
//!
//! Click positions on the stage come from an engine `Game` given the same clicks (the app's
//! camera isn't read); a netlist pasted with ⌘V goes into the Sandbox as a user's would.

mod common;

use common::*;
use eframe::egui::{self, Key, Modifiers, PointerButton, Pos2, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use sp_engine::builder::{
    Design, Foundation, Game, Inspector, Joint, LevelKey, Piece, PieceType, PresetBuild, Scene,
    Tool,
};
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
    const DRIVES: [&str; 7] = [
        "Run traffic",
        "Run the test",
        "Fast-forward 30 years",
        "Back to building",
        "Stop",
        "Pause",
        "Drive across",
    ];
    header_labels(h)
        .into_iter()
        .find(|l| DRIVES.contains(&l.as_str()))
        .expect("a drive button in the header")
}

// ============================================================================================
// The builder's stage, driven by AccessKit labels and grid clicks

/// The stage's drawing box: the grid surface button ("Building grid") inside its 1 px border.
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

/// Clicks the stage at world (x, y) m. `cam` is an engine game in the same level and view
/// (the app's camera is not read from its state).
fn click_grid(h: &mut App, cam: &Game, x: f64, y: f64) {
    let r = stage_inner(h);
    let s = r.width() / 1120.0;
    let c = cam.camera();
    let p: Pos2 = r.min + egui::vec2(c.x(x) as f32 * s, c.y(y) as f32 * s);
    h.hover_at(p);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(2);
}

/// Clicks joint `id`'s button ("Joint J3, on a pile, …").
fn click_joint(h: &mut App, id: &str) {
    let head = format!("Joint {id}");
    let label = labels(h)
        .into_iter()
        .find(|l| *l == head || l.starts_with(&format!("{head},")))
        .unwrap_or_else(|| panic!("no button for {head}"));
    h.get_by_role_and_label(Role::Button, &label).click();
    h.run_steps(2);
}

/// Clicks the button `label` inside the group `group` (scrolled into view: the aside scrolls).
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

/// The stage summary ("Bridge with 10 joints and 17 pieces. Stands up.").
fn stage_label(h: &App) -> String {
    h.get_by_role(Role::Image)
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

/// A Road deck J1 → 16, 24, 32, 40 → J2 under a truss of `chord` (top chord at 6 m, verticals
/// and diagonals), built by clicking — and the same clicks on the engine `g`.
fn build_truss(h: &mut App, g: &mut Game, chord: &str, kind: PieceType) {
    click_in(h, "Piece to build", "Road");
    g.pick_type(PieceType::Road);
    click_joint(h, "J1");
    g.click_node("J1");
    for x in [16.0, 24.0, 32.0, 40.0] {
        click_grid(h, g, x, 0.0);
        g.click_at([x, 0.0]);
    }
    click_joint(h, "J2");
    g.click_node("J2");
    click_button(h, "Finish line");
    g.finish_line();
    click_in(h, "Piece to build", chord);
    g.pick_type(kind);
    click_joint(h, "J1");
    g.click_node("J1");
    for x in [16.0, 24.0, 32.0, 40.0] {
        click_grid(h, g, x, 6.0);
        g.click_at([x, 6.0]);
    }
    click_joint(h, "J2");
    g.click_node("J2");
    click_button(h, "Finish line");
    g.finish_line();
    for chain in [
        &["J3", "J7", "J4", "J8", "J5", "J9"][..],
        &["J6", "J10", "J5"][..],
    ] {
        for id in chain {
            click_joint(h, id);
            g.click_node(id);
        }
        click_button(h, "Finish line");
        g.finish_line();
    }
}

/// Clicks world point (x, y) and mirrors it on `g`: a joint there takes the click (its button
/// covers it), otherwise the grid does. Press and release in one frame, then one more so the
/// next click finds what this one built.
fn click_world(h: &mut App, g: &mut Game, x: f64, y: f64) {
    let r = stage_inner(h);
    let s = r.width() / 1120.0;
    let c = g.camera();
    let p: Pos2 = r.min + egui::vec2(c.x(x) as f32 * s, c.y(y) as f32 * s);
    h.hover_at(p);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.step();
    h.step();
    let at = g
        .design()
        .nodes
        .iter()
        .find(|n| n.x == x && n.y == y)
        .map(|n| n.id.clone());
    match at {
        Some(id) => g.click_node(&id),
        None => g.click_at([x, y]),
    }
}

/// Esc ends the member chain (NAV-5), mirrored.
fn end_chain(h: &mut App, g: &mut Game) {
    press(h, Key::Escape);
    h.step();
    g.finish_line();
}

/// The joint and piece counts the stage summary reports ("Tower with 27 joints and 58
/// pieces. …").
fn counts(h: &App) -> (usize, usize) {
    let l = stage_label(h);
    let num = |after: &str| {
        l.split(after)
            .next()
            .and_then(|a| a.rsplit(' ').next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0)
    };
    (num(" joints"), num(" pieces"))
}

/// A tower on open ground shaped like the mockup's preset (`buildPreset` tower), by clicks:
/// `bays` 8 m bays centred on x = 100, foundations at the base (Found tool), then each Steel
/// column as one chain up to `height`, a chain of Floors every 4 m, and the `brace` diagonals
/// (bottom left to top right in every bay and storey). The engine `g`, given the same clicks,
/// holds the same design.
fn build_tower(
    h: &mut App,
    g: &mut Game,
    bays: u32,
    height: f64,
    brace: (&str, PieceType),
    found: (&str, Foundation),
) {
    let xs: Vec<f64> = (0..=bays)
        .map(|i| 100.0 - bays as f64 * 4.0 + i as f64 * 8.0)
        .collect();
    click_button(h, "Found");
    g.pick_tool(Tool::Found);
    click_in(h, "Foundation type", found.0);
    g.pick_found(found.1);
    for &x in &xs {
        // the level's own anchors may already stand there on this foundation (clicking one
        // again would take it away)
        let done = g
            .design()
            .nodes
            .iter()
            .any(|n| n.x == x && n.y == 0.0 && n.sup == Some(found.1));
        if !done {
            click_world(h, g, x, 0.0);
        }
    }
    // frame the base and the target before building up
    press_cmd(h, Key::Num0);
    settle(h);
    g.fit();
    click_button(h, "Build");
    g.pick_tool(Tool::Build);
    let storeys: Vec<f64> = (1..)
        .map(|k| k as f64 * 4.0)
        .take_while(|y| *y <= height + 1e-9)
        .collect();
    click_in(h, "Piece to build", "Steel");
    g.pick_type(PieceType::Steel);
    for &x in &xs {
        click_world(h, g, x, 0.0);
        for &y in &storeys {
            click_world(h, g, x, y);
        }
        end_chain(h, g);
    }
    click_in(h, "Piece to build", "Floor");
    g.pick_type(PieceType::Floor);
    for &y in &storeys {
        for &x in &xs {
            click_world(h, g, x, y);
        }
        end_chain(h, g);
    }
    click_in(h, "Piece to build", brace.0);
    g.pick_type(brace.1);
    for &y in &storeys {
        for i in 0..bays as usize {
            click_world(h, g, xs[i], y - 4.0);
            click_world(h, g, xs[i + 1], y);
            end_chain(h, g);
        }
    }
}

/// Pastes `text` as ⌘V would (NAV-6: a netlist goes into the Sandbox).
fn paste(h: &mut App, text: &str) {
    h.event(egui::Event::Paste(text.to_string()));
    settle(h);
}

/// The HUD chip whose label starts with `key` ("Crossings 3", "Wind 25 m/s →").
fn hud(h: &App, key: &str) -> Option<String> {
    labels(h).into_iter().find(|l| l.starts_with(key))
}

/// Steps the app until the result banner shows (its "Back to building" button), up to `max`.
fn wait_for_banner(h: &mut App, max: usize) -> bool {
    step_until(h, max, |h| {
        h.query_all_by_label("Replay in slow motion")
            .next()
            .is_some()
    })
}

// ============================================================================================
// 1–8: the builder and its fold-ins

/// DONE-4: level 1 built from scratch by clicks — a Road deck and Wood triangles — runs, every
/// car crosses and the banner gives the stars the engine gives the same design.
#[test]
fn journey_1_level_one_by_clicks_all_across_with_stars() {
    let mut h = harness(Route::Playground);
    assert!(header_has(&h, "river-level"));
    let mut g = Game::new();
    build_truss(&mut h, &mut g, "Wood", PieceType::Wood);
    let (n, m) = (g.design().nodes.len(), g.design().members.len());
    assert!(
        stage_label(&h).starts_with(&format!("Bridge with {n} joints and {m} pieces.")),
        "{}",
        stage_label(&h)
    );
    let status = g.view().status.label;
    assert!(stage_label(&h).ends_with(&format!("{status}.")), "{status}");
    // the expected run, by the engine alone
    g.run_test(sp_shared::proto::MAX_RUN_FRAMES);
    let want = g.view().result.expect("a banner");
    assert!(want.pass, "the wood truss gets the cars across: {want:?}");
    // run it from the header
    assert_eq!(drive(&h), "Run traffic");
    click_header(&mut h, "Run traffic");
    assert_eq!(drive(&h), "Pause");
    assert!(
        header_labels(&h).iter().any(|l| l.starts_with("t = ")),
        "the clock runs"
    );
    assert!(wait_for_banner(&mut h, 3000), "the run ends");
    assert!(has_label(&h, &want.title), "banner {:?}", want.title);
    assert!(has_label(&h, &want.stars), "stars {:?}", want.stars);
    // the level row remembers the stars
    let stars = g.result().expect("a result").stars;
    let row = h.get_by_role_and_label(Role::Button, "1 · First crossing");
    assert_eq!(
        row.accesskit_node().description().as_deref(),
        Some(format!("Best: {stars} of 3 stars").as_str())
    );
    click_in(&mut h, "Goal", "Back to building");
    assert_eq!(drive(&h), "Run traffic");
    assert!(header_has(&h, "building"));
}

/// DONE-4: Heavy haul in a wood truss collapses under the 30 t truck and the snow; the banner
/// and its sentence are the engine's for the same design; "Replay in slow motion" runs it again
/// slowly to the same banner; "Back to building".
#[test]
fn journey_2_heavy_haul_collapses_and_replays() {
    let mut h = harness(Route::Playground);
    click_button(&mut h, "3 · Heavy haul");
    assert!(header_has(&h, "haul-level"));
    let mut g = Game::new();
    g.pick_level(LevelKey::Haul);
    build_truss(&mut h, &mut g, "Wood", PieceType::Wood);
    g.run_test(sp_shared::proto::MAX_RUN_FRAMES);
    let r = g.result().expect("a result").clone();
    assert!(
        !r.pass,
        "a wood truss can't carry the truck in the snow: {r:?}"
    );
    let want = g.view();
    let banner = want.result.as_ref().expect("a banner");
    press(&mut h, Key::Space);
    assert!(wait_for_banner(&mut h, 3000), "the run ends");
    assert!(has_label(&h, &banner.title), "banner {:?}", banner.title);
    assert!(
        has_label(&h, &want.explain),
        "the sentence {:?} in {:?}",
        want.explain,
        labels(&h)
            .into_iter()
            .filter(|l| l.len() > 40)
            .collect::<Vec<_>>()
    );
    assert!(
        r.first.is_some() || r.first_found.is_some(),
        "something gave way: {r:?}"
    );
    // replay in slow motion: running again, slowly, then the same banner
    click_button(&mut h, "Replay in slow motion");
    assert_eq!(drive(&h), "Pause");
    assert!(has_label(&h, "Normal speed"), "slow motion is on");
    assert!(wait_for_banner(&mut h, 6000), "the replay ends");
    assert!(has_label(&h, &banner.title));
    click_in(&mut h, "Goal", "Back to building");
    assert!(header_has(&h, "building"));
}

/// DONE-4: a cable-stayed bridge in the Sandbox (pasted from the builder's own Share of the
/// mockup's cable preset) carries the looping traffic; mid-run the wind is turned up to Strong
/// from the Conditions chips: the HUD wind eases up to 25 m/s, the cars, buses and trucks keep
/// crossing — as the engine says the same bridge does.
#[test]
fn journey_3_cable_stayed_sandbox_in_changing_wind() {
    let mut src = Game::new();
    src.pick_level(LevelKey::Sandbox);
    src.build_preset(PresetBuild::Cable);
    let text = src.share_text();
    // the engine's own run: traffic crossing, then 25 m/s from 5 s, nothing breaks in 30 s
    let mut g = Game::new();
    g.paste_netlist(&text).expect("the cable bridge loads");
    g.start_run(false);
    for f in 0..900 {
        if f == 150 {
            g.set_wind(25.0);
        }
        g.tick();
    }
    let sim = g.sim().expect("running");
    assert!(sim.first_break.is_none(), "{:?}", sim.first_break);
    assert!(sim.laps() > 0, "traffic loops across");

    let mut h = harness(Route::Playground);
    click_button(&mut h, "Sandbox");
    paste(&mut h, &text);
    assert!(header_has(&h, "sandbox"));
    let (n, m) = (src.design().nodes.len(), src.design().members.len());
    assert_eq!(counts(&h), (n, m), "{}", stage_label(&h));
    click_header(&mut h, "Run traffic");
    assert_eq!(drive(&h), "Pause");
    assert!(has_label_containing(&h, "runs until you stop"));
    let crossings = |h: &App| {
        hud(h, "Crossings")
            .and_then(|l| l.rsplit(' ').next().and_then(|n| n.parse::<u32>().ok()))
            .unwrap_or(0)
    };
    assert!(
        step_until(&mut h, 400, |h| crossings(h) >= 1),
        "a vehicle crosses: {:?}",
        hud(&h, "Crossings")
    );
    let before = crossings(&h);
    // the wind, mid-run
    click_in(&mut h, "Wind", "Strong");
    assert!(
        step_until(&mut h, 400, |h| hud(h, "Wind")
            .is_some_and(|l| l.contains("25 m/s"))),
        "the wind eases up to 25 m/s: {:?}",
        hud(&h, "Wind")
    );
    assert!(
        step_until(&mut h, 600, |h| crossings(h) > before),
        "traffic keeps crossing in the wind: {:?}",
        hud(&h, "Crossings")
    );
    assert_eq!(drive(&h), "Pause", "still running");
}

/// DONE-4: a 64 m tower on piles, built storey by storey by clicks, rides out level 5's storm:
/// "It stood!", the comfort meter's value and sentence, then "Compare shapes" with the four
/// bars and the verdict — every number from the engine for the same design.
#[test]
fn journey_4_storm_tower_comfort_and_shapes() {
    let mut h = harness(Route::Playground);
    click_button(&mut h, "5 · Storm tower");
    assert!(header_has(&h, "t2-level"));
    let mut g = Game::new();
    g.pick_level(LevelKey::T2);
    build_tower(
        &mut h,
        &mut g,
        4,
        64.0,
        ("Wood", PieceType::Wood),
        ("Pile · $1.5k", Foundation::Pile),
    );
    let (n, m) = (g.design().nodes.len(), g.design().members.len());
    assert_eq!(counts(&h), (n, m), "{}", stage_label(&h));
    // the engine's run and comparison of the same tower
    let setup = g.setup();
    let cond = *g.cond();
    let shapes = sp_engine::builder::comfort::compare(g.design(), &setup, &cond);
    g.run_test(sp_shared::proto::MAX_RUN_FRAMES);
    let r = g.result().expect("a result").clone();
    assert!(r.pass, "the 64 m tower rides out the storm: {r:?}");
    let banner = g.view().result.expect("a banner");
    let sp_engine::builder::Outcome::Tower { mg, .. } = r.outcome else {
        panic!("open ground")
    };
    let meter = sp_engine::builder::comfort::Meter::new(mg, setup.comfort);

    click_header(&mut h, "Run the test");
    assert!(wait_for_banner(&mut h, 3000), "the test ends");
    assert!(has_label(&h, &banner.title), "banner {:?}", banner.title);
    assert!(has_label(&h, &meter.label()), "meter {:?}", meter.label());
    assert!(has_label(&h, meter.sentence()));
    reveal(&mut h, "Compare shapes", Some(Role::Button));
    click_button(&mut h, "Compare shapes");
    settle(&mut h);
    for b in &shapes.bars {
        assert!(has_label(&h, &b.label()), "bar {:?}", b.label());
    }
    assert!(
        has_label(&h, &shapes.verdict),
        "verdict {:?}",
        shapes.verdict
    );
}

/// DONE-4: Shaky ground — a 32 m tower on footings in loose wet sand comes down in the quake
/// when its footings liquefy; swapped to piles with the Found tool it stands (banners and
/// sentences from the engine for the same designs).
#[test]
fn journey_5_shaky_ground_piles_fix_liquefaction() {
    let mut h = harness(Route::Playground);
    click_button(&mut h, "6 · Shaky ground");
    let mut g = Game::new();
    g.pick_level(LevelKey::T3);
    build_tower(
        &mut h,
        &mut g,
        2,
        32.0,
        ("Wood", PieceType::Wood),
        ("Footing · $0.3k", Foundation::Footing),
    );
    let base: Vec<String> = g
        .design()
        .nodes
        .iter()
        .filter(|n| n.y == 0.0)
        .map(|n| n.id.clone())
        .collect();
    let mut on_piles = g.clone();
    g.run_test(sp_shared::proto::MAX_RUN_FRAMES);
    let r = g.result().expect("a result").clone();
    assert!(!r.pass, "footings in loose wet sand: {r:?}");
    let ff = r.first_found.as_ref().expect("a foundation gave way");
    assert_eq!(ff.mode.text(), "liquefied", "{ff:?}");
    let failed = g.view();
    click_header(&mut h, "Run the test");
    assert!(wait_for_banner(&mut h, 3000));
    let title = &failed.result.as_ref().expect("a banner").title;
    assert!(has_label(&h, title), "banner {title:?}");
    assert!(
        has_label(&h, &failed.explain),
        "sentence {:?}",
        failed.explain
    );
    // back to building: piles instead
    click_in(&mut h, "Goal", "Back to building");
    click_button(&mut h, "Found");
    on_piles.pick_tool(Tool::Found);
    click_in(&mut h, "Foundation type", "Pile · $1.5k");
    on_piles.pick_found(Foundation::Pile);
    for id in &base {
        click_joint(&mut h, id);
        on_piles.click_node(id);
    }
    assert!(
        on_piles
            .design()
            .nodes
            .iter()
            .filter(|n| n.y == 0.0)
            .all(|n| n.sup == Some(Foundation::Pile))
    );
    on_piles.run_test(sp_shared::proto::MAX_RUN_FRAMES);
    let fixed = on_piles.result().expect("a result").clone();
    assert!(
        fixed.pass,
        "piles reach below the liquefying sand: {fixed:?}"
    );
    let banner = on_piles.view().result.expect("a banner");
    click_header(&mut h, "Run the test");
    assert!(wait_for_banner(&mut h, 3000));
    assert!(has_label(&h, &banner.title), "banner {:?}", banner.title);
}

/// DONE-4 / TIME-3: the Leaning tower, left alone, leans past its goal over 30 years; piles
/// under all three footings (the level's named fix) keep it under 0.5° — the fast-forward's
/// banner and stars are the engine's for the same design.
#[test]
fn journey_6_leaning_tower_fixed_over_30_years() {
    use sp_engine::builder::longterm::{self, pisa};
    let mut h = harness(Route::Playground);
    click_button(&mut h, "7 · Leaning tower");
    assert!(header_has(&h, "pisa-level"));
    let mut g = Game::new();
    g.pick_level(LevelKey::Pisa);
    let lean = |g: &Game| longterm::fast_forward_design(g.design(), &g.setup(), g.cond());
    let left_alone = lean(&g);
    assert!(
        left_alone
            .last()
            .is_some_and(|f| f.tilt.abs() > pisa::GOAL_TILT),
        "left alone it leans past the goal"
    );
    click_button(&mut h, "Found");
    g.pick_tool(Tool::Found);
    click_in(&mut h, "Foundation type", "Pile · $1.5k");
    g.pick_found(Foundation::Pile);
    for id in pisa::footing_ids() {
        click_joint(&mut h, &id);
        g.click_node(&id);
    }
    let lt = lean(&g);
    let budget = g.setup().budget.unwrap_or(pisa::BUDGET);
    let want = pisa::result(&lt, g.cost(), g.practice(), budget);
    assert!(want.pass, "piles under all three footings fix it: {want:?}");
    let stars = sp_engine::builder::data::stars_text(want.stars);
    let label = if want.stars > 0 {
        format!("{} {stars}", want.title)
    } else {
        want.title.clone()
    };
    // the run is the 30-year fast-forward
    let run = drive(&h);
    click_header(&mut h, &run);
    if h.query_by_label("Skip to the end").is_some() {
        click_button(&mut h, "Skip to the end");
    }
    assert!(
        step_until(&mut h, 400, |h| has_label(h, &label)),
        "the result {label:?} in {:?}",
        labels(&h)
            .into_iter()
            .filter(|l| l.contains("year") || l.contains("Straight") || l.contains("lean"))
            .collect::<Vec<_>>()
    );
}

/// DONE-4 / LAB-1: a material card opens the sample drawer; "Pull" pulls the wood sample to
/// breaking; the status reads what the engine's own sample of that piece type says.
#[test]
fn journey_7_sample_pulled_to_breaking() {
    use sp_engine::builder::sample::Sample;
    let mut h = harness(Route::Playground);
    let Inspector::Materials(cards) = Game::new().view().inspector else {
        panic!("nothing selected: the Materials list")
    };
    let wood = cards
        .iter()
        .find(|c| c.kind == PieceType::Wood)
        .expect("a Wood card");
    let card = format!("{} · {} · {}", wood.name, wood.blurb, wood.cost);
    let mut sample = Sample::new(PieceType::Wood);
    sample.pull();
    assert!(sample.run_to_end(), "wood snaps");
    let want = sample.status().label(PieceType::Wood);
    click(&mut h, &card);
    assert!(has_label(&h, "Pull"), "the sample drawer is open");
    click_button(&mut h, "Pull");
    assert!(
        step_until(&mut h, 400, |h| has_label(h, &want)),
        "status {want:?}"
    );
    // Esc closes the drawer
    press(&mut h, Key::Escape);
    settle(&mut h);
    assert!(!has_label(&h, "Pull"), "the drawer closed");
}

/// The lab's steel Warren truss over a Road deck on the river (`fire_heat.rs`), no crates.
fn steel_truss() -> Design {
    let joint = |id: &str, x: f64, y: f64, sup: Option<Foundation>| Joint {
        id: id.into(),
        x,
        y,
        sup,
        locked: false,
    };
    let mut nodes = vec![
        joint("J1", 8.0, 0.0, Some(Foundation::Footing)),
        joint("J2", 48.0, 0.0, Some(Foundation::Footing)),
    ];
    for (i, x) in [18.0, 28.0, 38.0].iter().enumerate() {
        nodes.push(joint(&format!("J{}", 3 + i), *x, 0.0, None));
    }
    for (i, x) in [13.0, 23.0, 33.0, 43.0].iter().enumerate() {
        nodes.push(joint(&format!("J{}", 6 + i), *x, 6.0, None));
    }
    let bottom = ["J1", "J3", "J4", "J5", "J2"];
    let top = ["J6", "J7", "J8", "J9"];
    let mut members: Vec<Piece> = Vec::new();
    let mut add = |a: &str, b: &str, kind: PieceType| {
        let id = format!("M{}", members.len() + 1);
        members.push(Piece {
            id,
            a: a.into(),
            b: b.into(),
            kind,
            locked: false,
        });
    };
    for w in bottom.windows(2) {
        add(w[0], w[1], PieceType::Road);
    }
    for w in top.windows(2) {
        add(w[0], w[1], PieceType::Steel);
    }
    for i in 0..4 {
        add(bottom[i], top[i], PieceType::Steel);
        add(top[i], bottom[i + 1], PieceType::Steel);
    }
    let n_m = members.len() as u32;
    Design {
        nodes,
        members,
        loads: vec![],
        n_n: 9,
        n_m,
    }
}

/// DONE-4 / FIRE-1: a steel truss pasted into the Sandbox carries the traffic cold; with "Big
/// fire" under the span the middle top chord softens — the running explanation names the
/// piece the engine says goes first.
#[test]
fn journey_8_fire_softens_a_steel_truss() {
    let mut src = Game::new();
    src.pick_level(LevelKey::Sandbox);
    let text = sp_engine::builder::netlist::write(
        &steel_truss(),
        LevelKey::Sandbox,
        Scene::River,
        src.cond(),
        "fire-truss",
    );
    // the engine: the same truss, Big fire, run until something gives way
    let mut g = Game::new();
    g.paste_netlist(&text).expect("the truss loads");
    g.set_fire(700.0);
    g.start_run(false);
    let mut first = None;
    for _ in 0..1500 {
        g.tick();
        if let Some(fb) = g.sim().and_then(|s| s.first_break.clone()) {
            first = Some(fb);
            break;
        }
    }
    let fb = first.expect("something gives way in the fire");
    assert_eq!(fb.mode.text(), "softened", "{fb:?}");
    let want = format!("{} just softened!", fb.id);

    let mut h = harness(Route::Playground);
    click_button(&mut h, "Sandbox");
    paste(&mut h, &text);
    assert!(header_has(&h, "sandbox"));
    assert_eq!(counts(&h), (9, 15), "{}", stage_label(&h));
    click_in(&mut h, "Fire", "Big fire");
    click_header(&mut h, "Run traffic");
    assert_eq!(drive(&h), "Pause");
    assert!(
        step_until(&mut h, 900, |h| has_label_containing(h, &want)),
        "{want:?} (the engine: {fb:?})"
    );
    assert!(has_label_containing(&h, "Fire "), "the HUD's fire chip");
}

// ============================================================================================
// 9. Advanced: an error in the netlist, fixed, then "Open in Playground"

/// A small netlist on the builder's river grid: a 16 m Howe-like truss on the banks.
const TRUSS: &str = "* little-truss.str
.units SI kN m
.material steel E=200GPa fy=250MPa rho=7850
.section medium A=80cm2 I=A^2
.node B0 8 0 fix=xy
.node B1 16 0
.node B2 24 0 fix=y
.node T1 16 6
.member M1 B0 B1 medium
.member M2 B1 B2 medium
.member M3 B0 T1 medium
.member M4 T1 B2 medium
.member M5 B1 T1 medium
.load dead B1 Fy=-20t
.static linear
";

#[test]
fn journey_9_advanced_error_fix_and_open_in_playground() {
    let mut h = harness(Route::Advanced);
    let replace = |h: &mut App, t: &str| {
        get(h, "Netlist", Role::MultilineTextInput).focus();
        h.step();
        h.key_press_modifiers(Modifiers::COMMAND, Key::A);
        h.step();
        get(h, "Netlist", Role::MultilineTextInput).type_text(t);
        h.run_steps(4);
    };
    replace(&mut h, TRUSS);
    let rows = |h: &App| {
        labels(h)
            .into_iter()
            .filter(|l| l.contains(": N ") && l.contains(" kN, σ "))
            .count()
    };
    assert_eq!(rows(&h), 5, "{:?}", labels(&h));
    // break it: M3 names a joint that doesn't exist
    let line_no = TRUSS
        .lines()
        .position(|l| l.starts_with(".member M3 "))
        .unwrap()
        + 1;
    replace(
        &mut h,
        &TRUSS.replace(".member M3 B0 T1", ".member M3 B0 T9"),
    );
    let err = format!("Line {line_no}: unknown node T9");
    assert!(has_label(&h, &err), "missing {err:?}");
    assert!(has_label(&h, "1 error"));
    assert_eq!(rows(&h), 5, "the last good result stays");
    assert!(
        get(&h, "Open in Playground", Role::Button)
            .accesskit_node()
            .is_disabled()
    );
    // fix it, then hand it to the builder
    replace(&mut h, TRUSS);
    assert!(!has_label(&h, "1 error"));
    reveal(&mut h, "Open in Playground", Some(Role::Button));
    click_button(&mut h, "Open in Playground");
    settle(&mut h);
    assert_eq!(selected(&h, "Playground", Role::Link), Some(true));
    // TODO(wave 2): the builder shows it in the Sandbox (header `sandbox`, 4 joints, 5 pieces,
    // "Stands up"), and its "Open in Advanced" brings back the same forces.
}

/// DONE-4 / NAV-4: "Open in Advanced" shows the builder's structure as a netlist — every joint
/// a node, every piece a member (a row in the results), the level as the document — and
/// "Open in Playground" brings it back into the Sandbox with the same counts.
#[test]
fn journey_9b_open_in_advanced_shows_the_same_structure() {
    let mut h = harness(Route::Playground);
    let mut g = Game::new();
    build_truss(&mut h, &mut g, "Steel", PieceType::Steel);
    let (n, m) = (g.design().nodes.len(), g.design().members.len());
    reveal(&mut h, "Open in Advanced", Some(Role::Button));
    click_button(&mut h, "Open in Advanced");
    settle(&mut h);
    assert_eq!(selected(&h, "Advanced", Role::Link), Some(true));
    assert!(header_has(&h, "river-level"), "{:?}", header_labels(&h));
    assert!(has_label(&h, "from the Playground"));
    let rows = labels(&h)
        .into_iter()
        .filter(|l| l.contains(": N ") && l.contains(" kN, σ "))
        .count();
    assert_eq!(rows, m, "a member row per piece");
    // the netlist Advanced holds is the builder's Share of the same design
    click_header(&mut h, "Share");
    let text = last_copied(&h).expect("Share copies the netlist");
    assert_eq!(text.lines().filter(|l| l.starts_with(".node ")).count(), n);
    assert_eq!(
        text.lines().filter(|l| l.starts_with(".member ")).count(),
        m
    );
    // and back
    reveal(&mut h, "Open in Playground", Some(Role::Button));
    click_button(&mut h, "Open in Playground");
    settle(&mut h);
    assert_eq!(selected(&h, "Playground", Role::Link), Some(true));
    assert!(header_has(&h, "sandbox"));
    assert!(
        stage_label(&h).starts_with(&format!("Bridge with {n} joints and {m} pieces.")),
        "{}",
        stage_label(&h)
    );
}

// ============================================================================================
// 10. Keyboard only

#[test]
fn journey_10_keyboard_only() {
    let mut h = harness(Route::Playground);
    // ⌘2 / ⌘1 switch screens
    press_cmd(&mut h, Key::Num2);
    settle(&mut h);
    assert_eq!(selected(&h, "Advanced", Role::Link), Some(true));
    assert_eq!(drive(&h), "Drive across");
    press_cmd(&mut h, Key::Num1);
    settle(&mut h);
    assert_eq!(selected(&h, "Playground", Role::Link), Some(true));
    assert!(header_has(&h, "river-level"));
    assert!(header_has(&h, "building"), "the builder's clock");
    // Space drives (nothing focused)
    h.remove_cursor();
    let run = drive(&h);
    assert_eq!(run, "Run traffic");
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_ne!(drive(&h), run, "Space started the run");
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(drive(&h), run, "Space stopped it");
    // Tab reaches the drive button; Enter presses it
    let mut reached = false;
    for _ in 0..40 {
        press(&mut h, Key::Tab);
        if focused_label(&h).as_deref() == Some(run.as_str()) {
            reached = true;
            break;
        }
    }
    assert!(reached, "Tab never reached the drive button");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_ne!(drive(&h), run, "Enter on the focused drive button");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(drive(&h), run);
    // the brand opens the Series screen; Esc comes back
    let mut on_brand = false;
    // (Tab walks the whole builder — chips, joints, levels — before wrapping to the header)
    for _ in 0..400 {
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
        selected(&h, "Playground", Role::Link),
        Some(true),
        "back where it was"
    );
}

/// DONE-4 / NAV-5: building and zooming with the keyboard alone — Tab to the grid surface,
/// Enter drops a joint at the view's centre, the arrows pan, Enter again chains a piece; ⌘= /
/// ⌘− / ⌘0 zoom as the engine's buttons do; Esc ends the chain.
#[test]
fn journey_10b_keyboard_only_build_and_zoom() {
    let mut h = harness(Route::Playground);
    h.remove_cursor();
    let zoom_label = |h: &App| {
        labels(h)
            .into_iter()
            .find(|l| l.starts_with("Zoom ") && l.ends_with('%'))
            .expect("the zoom readout")
    };
    let mut g = Game::new();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    // zoom by keys, checked against the engine's own zoom steps
    press_cmd(&mut h, Key::Equals);
    settle(&mut h);
    g.zoom_in();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    press_cmd(&mut h, Key::Minus);
    press_cmd(&mut h, Key::Minus);
    settle(&mut h);
    g.zoom_out();
    g.zoom_out();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    press_cmd(&mut h, Key::Num0);
    settle(&mut h);
    g.fit();
    assert_eq!(zoom_label(&h), format!("Zoom {}", g.view().zoom_pct));
    // Tab to the grid surface
    let mut on_grid = false;
    for _ in 0..400 {
        press(&mut h, Key::Tab);
        if focused_label(&h).as_deref() == Some("Building grid") {
            on_grid = true;
            break;
        }
    }
    assert!(on_grid, "Tab reaches the building grid");
    let joints = |h: &App| {
        let l = stage_label(h);
        l.split(" joints")
            .next()
            .and_then(|a| a.rsplit(' ').next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0)
    };
    let pieces = |h: &App| {
        let l = stage_label(h);
        l.split(" pieces")
            .next()
            .and_then(|a| a.rsplit(' ').next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0)
    };
    let (j0, p0) = (joints(&h), pieces(&h));
    // Enter drops a joint at the centre (the Build tool is the starting tool)
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(joints(&h), j0 + 1, "{}", stage_label(&h));
    // pan right with the arrows (the grid keeps focus), Enter again: a piece to the new joint
    for _ in 0..3 {
        press(&mut h, Key::ArrowRight);
    }
    settle(&mut h);
    assert_eq!(
        focused_label(&h).as_deref(),
        Some("Building grid"),
        "the grid keeps focus"
    );
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(joints(&h), j0 + 2, "{}", stage_label(&h));
    assert_eq!(pieces(&h), p0 + 1, "{}", stage_label(&h));
    // Esc ends the chain: the next Enter starts a new one (a joint, no piece)
    press(&mut h, Key::Escape);
    for _ in 0..3 {
        press(&mut h, Key::ArrowUp);
    }
    settle(&mut h);
    if focused_label(&h).as_deref() != Some("Building grid") {
        // Esc may have surrendered focus: Tab back to the grid
        for _ in 0..400 {
            press(&mut h, Key::Tab);
            if focused_label(&h).as_deref() == Some("Building grid") {
                break;
            }
        }
    }
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(joints(&h), j0 + 3, "{}", stage_label(&h));
    assert_eq!(
        pieces(&h),
        p0 + 1,
        "no piece after Esc: {}",
        stage_label(&h)
    );
}
