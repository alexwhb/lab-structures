//! CMF-1 / CMF-2 / TIME-1 in the real app (egui_kittest, AccessKit labels): the comfort meter
//! during and after an open-ground run, "Compare shapes" with the engine's four bars and
//! verdict, "Fast-forward 30 years" after a run that stood — the years playing with the HUD
//! chips, the scrubber, the result line — and back to building.

mod common;

use common::*;
use eframe::egui::accesskit::Role;
use sp_engine::builder::comfort::{self, Meter};
use sp_engine::builder::game::Preset;
use sp_engine::builder::longterm;
use sp_engine::builder::{Game, LevelKey, Outcome, Phase, PieceType, Tool};
use structures_playground::Route;
use structures_playground::screens::builder::BuilderScreen;
use structures_playground::screens::builder::time as time_ui;

/// A tall window so the whole aside fits.
fn tall() -> App {
    app().at(Route::Playground).size(1440.0, 2200.0).build()
}

fn screen(h: &App) -> &BuilderScreen {
    &h.state().screens().builder
}

fn screen_mut(h: &mut App) -> &mut BuilderScreen {
    &mut h.state_mut().screens_mut().builder
}

fn game(h: &App) -> &Game {
    &screen(h).game
}

/// The storm-tower board (64 m, steel columns, wood braces, piles on stiff clay, level 5), not
/// running on its own (SPEC MB-10).
fn storm_tower(h: &mut App) {
    let mut p = Preset::board("TowerStorm").expect("the board");
    p.run = false;
    p.demo_loop = false;
    screen_mut(h).game = Game::with_preset(&p);
    h.run_steps(3);
}

/// Runs the open-ground test to its end through the game, then lets the screen draw it.
fn run_to_the_end(h: &mut App) -> f64 {
    screen_mut(h).game.run_test(2000);
    h.run_steps(3);
    assert_eq!(game(h).phase(), Phase::Result);
    match game(h).result().expect("a result").outcome {
        Outcome::Tower { mg, .. } => mg,
        _ => panic!("open ground"),
    }
}

#[test]
fn the_comfort_meter_shows_while_running_and_after() {
    let mut h = tall();
    storm_tower(&mut h);
    assert!(
        !has_label_containing(&h, "milli-g"),
        "no meter while building"
    );
    screen_mut(&mut h).game.start_run(false);
    for _ in 0..90 {
        screen_mut(&mut h).game.tick();
    }
    h.run_steps(2);
    let live = game(&h).sim().unwrap().milli_g();
    let m = Meter::new(live, Some(300.0));
    assert!(has_label(&h, &m.label()), "live meter: {}", m.label());
    assert!(!has_label(&h, "Compare shapes"), "only after the run");

    let mg = run_to_the_end(&mut h);
    let m = Meter::new(mg, game(&h).setup().comfort);
    assert!(has_label(&h, &m.label()), "{}", m.label());
    assert!(has_label(&h, &m.value()));
    assert!(has_label(&h, m.sentence()));
    // the HUD's comfort chip and the meter agree (both the engine's number)
    assert!(has_label(&h, &format!("Comfort {}", comfort::mg_text(mg))));
}

#[test]
fn compare_shapes_shows_the_four_bars_and_the_verdict() {
    let mut h = tall();
    storm_tower(&mut h);
    run_to_the_end(&mut h);
    let (d, s, c) = (
        game(&h).design().clone(),
        game(&h).setup(),
        *game(&h).cond(),
    );
    let want = comfort::compare(&d, &s, &c);
    click_button(&mut h, "Compare shapes");
    h.run_steps(2);
    assert!(
        !has_label(&h, "Compare shapes"),
        "the button gives way to the bars"
    );
    for b in &want.bars {
        assert!(has_label(&h, &b.label()), "{}", b.label());
    }
    assert!(has_label(&h, &want.verdict), "{}", want.verdict);
    assert_eq!(
        screen(&h).fold.comfort.comparison(game(&h)),
        Some(&want),
        "the screen shows the engine's comparison"
    );
    // back to building drops it
    screen_mut(&mut h).game.stop_run();
    h.run_steps(2);
    assert!(screen(&h).fold.comfort.comparison(game(&h)).is_none());
    assert!(!has_label(&h, &want.verdict));
}

#[test]
fn fast_forward_thirty_years_after_a_run_that_stood() {
    let mut h = tall();
    storm_tower(&mut h);
    assert!(
        !has_label(&h, "Fast-forward 30 years"),
        "not while building"
    );
    run_to_the_end(&mut h);
    assert!(time_ui::Time::available(game(&h)));
    let (d, s, c) = (
        game(&h).design().clone(),
        game(&h).setup(),
        *game(&h).cond(),
    );
    let want = longterm::fast_forward_design(&d, &s, &c);
    click_button(&mut h, "Fast-forward 30 years");
    assert!(screen(&h).fold.time.playing());
    // the years play: the HUD shows them moving
    h.run_steps(30);
    let year = screen(&h).fold.time.year().unwrap();
    assert!(year > 0.0 && year < 30.0, "{year}");
    let chips = want.chips(year);
    for (k, v) in &chips {
        assert!(has_label(&h, &format!("{} {v}", k.trim())), "{k}{v}");
    }
    assert!(has_label(&h, "Skip to the end"));
    click_button(&mut h, "Skip to the end");
    h.run_steps(2);
    assert!(!screen(&h).fold.time.playing());
    assert_eq!(screen(&h).fold.time.long_term(), Some(&want));
    assert!(has_label(&h, &want.result_line()), "{}", want.result_line());
    assert!(has_label(&h, &want.sentence()));
    assert!(has_label(&h, "Year 30.0"));
    get(&h, "Years", Role::Slider);
    // the scrubber shows any year
    screen_mut(&mut h).fold.time.set_year(10.0);
    h.run_steps(2);
    for (k, v) in want.chips(10.0) {
        assert!(has_label(&h, &format!("{} {v}", k.trim())), "{k}{v}");
    }
    // at rest it asks for no frames (PLAT-3)
    assert!(!screen(&h).fold.time.playing());
    // back to building: gone
    screen_mut(&mut h).game.stop_run();
    h.run_steps(2);
    assert!(screen(&h).fold.time.long_term().is_none());
    assert!(!has_label_containing(&h, "Year "));
}

#[test]
fn no_fast_forward_after_a_collapse() {
    let mut h = tall();
    {
        // level 4, a lone Floor 40 m up with nothing under it: it falls
        let g = &mut screen_mut(&mut h).game;
        g.pick_level(LevelKey::T1);
        g.pick_tool(Tool::Build);
        g.pick_type(PieceType::Floor);
        g.click_at([100.0, 40.0]);
        g.click_at([104.0, 40.0]);
        assert_eq!(g.design().members.len(), 1);
    }
    h.run_steps(2);
    screen_mut(&mut h).game.run_test(2000);
    h.run_steps(3);
    let Some(Outcome::Tower { standing, .. }) = game(&h).result().map(|r| r.outcome.clone()) else {
        panic!("a result")
    };
    assert!(!standing);
    assert!(!time_ui::Time::available(game(&h)));
    assert!(!has_label(&h, "Fast-forward 30 years"));
    // the meter still shows the run's comfort
    assert!(has_label_containing(&h, "milli-g"));
}
