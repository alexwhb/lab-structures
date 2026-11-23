//! TIME-3: the Leaning tower level — left alone it leans past 4° within 30 years; piles under
//! it (or a counterweight on the high side) keep it under 0.5° for three stars.

use sp_engine::builder::longterm::{self, Ending, pisa};
use sp_engine::builder::{Conditions, Design, Foundation, LevelKey, Scene, Setup};

fn run(design: &Design) -> longterm::LongTerm {
    let level = pisa::level();
    let cond = Conditions::for_level(&level);
    let setup = Setup::new(LevelKey::Pisa, Scene::Flat, &cond);
    longterm::fast_forward_design(design, &setup, &cond)
}

fn with_piles(ids: &[&str]) -> Design {
    let mut d = pisa::prebuilt();
    for n in &mut d.nodes {
        if ids.contains(&n.id.as_str()) {
            n.sup = Some(Foundation::Pile);
        }
    }
    d
}

/// The cost of the player's fix, $k: the design's cost without the level's own (locked)
/// pieces, which cost nothing (requested from `engine` for `Design::cost`).
fn fix_cost(d: &Design) -> f64 {
    let mut c = 0.0;
    for m in d.members.iter().filter(|m| !m.locked) {
        c += d.length(m) * m.kind.spec().cost;
    }
    for n in d.nodes.iter().filter(|n| !n.locked) {
        if let Some(f) = n.sup {
            c += f.cost();
        }
    }
    c / 1000.0
}

fn show(label: &str, lt: &longterm::LongTerm) {
    let at = |y: f64| {
        lt.frames
            .iter()
            .find(|f| (f.year - y).abs() < 1e-9)
            .map_or(f64::NAN, |f| f.tilt)
    };
    eprintln!(
        "{label:28} y10 {:5.2}° y20 {:5.2}° y30 {:5.2}° {:?} — {}",
        at(10.0),
        at(20.0),
        at(30.0),
        lt.ending,
        lt.result_line()
    );
}

#[test]
fn left_alone_it_leans_past_4_degrees_within_30_years() {
    let lt = run(&pisa::prebuilt());
    show("untouched", &lt);
    let last = lt.last().unwrap();
    let leaned = lt.frames.iter().any(|f| f.tilt > 4.0);
    assert!(
        leaned || matches!(lt.ending, Ending::Toppled(_)),
        "tilt {} ({:?})",
        last.tilt,
        lt.ending
    );
    // it leans toward the bells, a little more every year
    assert!(lt.frames.windows(2).all(|w| w[1].tilt >= w[0].tilt - 1e-9));
    let r = pisa::result(&lt, fix_cost(&pisa::prebuilt()), false, pisa::BUDGET);
    assert!(!r.pass);
    assert_eq!(r.stars, 0);
    assert_eq!(r.title, "Still leaning");
}

#[test]
fn the_named_fix_piles_under_all_three_footings_earns_three_stars() {
    let d = with_piles(&["J1", "J2", "J3"]);
    let lt = run(&d);
    show("piles J1 J2 J3", &lt);
    assert_eq!(lt.ending, Ending::Stood);
    let r = pisa::result(&lt, fix_cost(&d), false, pisa::BUDGET);
    assert!(r.pass, "{r:?}");
    assert!(r.tilt < pisa::GOAL_TILT);
    assert_eq!(r.stars, 3, "cost {}", fix_cost(&d));
    assert_eq!(r.title, "Straight enough!");
}

#[test]
fn a_counterweight_on_the_high_side_also_fixes_it() {
    let mut d = pisa::prebuilt();
    let tl = pisa::top_ids()[0].clone();
    d.set_load(&tl, 9.0);
    let lt = run(&d);
    show("counterweight 9 t top-left", &lt);
    let r = pisa::result(&lt, fix_cost(&d), false, pisa::BUDGET);
    assert!(r.pass, "{r:?}");
    assert_eq!(r.stars, 3);
}

#[test]
fn a_lone_pile_under_the_low_side_tips_it_the_other_way() {
    let lt = run(&with_piles(&["J3"]));
    show("pile J3", &lt);
    assert!(lt.last().unwrap().tilt < -pisa::GOAL_TILT);
    let lt = run(&with_piles(&["J2", "J3"]));
    show("piles J2 J3", &lt);
}

#[test]
fn the_level_entry_is_complete() {
    let l = pisa::level();
    assert_eq!(l.key, LevelKey::Pisa);
    assert_eq!(l.name, "7 · Leaning tower");
    assert_eq!(
        l.goal,
        "It leans more every year. Fix it without removing floors: under 0.5° after 30 years."
    );
    let d = l.prebuilt.unwrap();
    assert!(d.members.iter().all(|m| m.locked));
    assert!(d.nodes.iter().all(|n| n.locked == (n.y > 0.0)));
    assert_eq!(d.load(&pisa::bells_joint()), pisa::BELLS_T);
    assert_eq!(d.n_n as usize, d.nodes.len());
    assert_eq!(d.n_m as usize, d.members.len());
}
