//! TIME-1 / TIME-2: the 30-year fast-forward on real builder designs — determinism, monotonic
//! settlement, rock vs soft clay, symmetric vs eccentric towers, bridges' differential
//! settlement, the readouts and sentences, and no panics on weird structures.

use sp_engine::builder::game::{Preset, PresetBuild};
use sp_engine::builder::longterm::{self, Ending, LongTerm, pisa};
use sp_engine::builder::{
    Conditions, Design, Foundation, Game, Joint, LevelKey, Piece, PieceType, Scene, Setup, Soil,
};
use sp_engine::dynamics::SplitMix64;

fn pisa_run(d: &Design, soil: Soil) -> LongTerm {
    let level = pisa::level();
    let mut cond = Conditions::for_level(&level);
    cond.soil = soil;
    let setup = Setup::new(LevelKey::Pisa, Scene::Flat, &cond);
    longterm::fast_forward_design(d, &setup, &cond)
}

fn symmetric_tower() -> Design {
    let mut d = pisa::prebuilt();
    d.set_load(&pisa::bells_joint(), 0.0);
    d
}

fn monotonic(lt: &LongTerm) -> bool {
    lt.frames.windows(2).all(|w| {
        w[0].settle
            .iter()
            .zip(&w[1].settle)
            .all(|(a, b)| *b >= *a - 1e-12)
    })
}

#[test]
fn the_same_design_gives_the_same_thirty_years() {
    let a = pisa_run(&pisa::prebuilt(), Soil::Soft);
    let b = pisa_run(&pisa::prebuilt(), Soil::Soft);
    assert_eq!(a, b);
    assert_eq!(a.frames.len(), 61);
    assert_eq!(a.frames[0].year, 0.0);
    assert_eq!(a.end_year(), 30.0);
}

#[test]
fn rock_settles_nothing_soft_clay_the_most() {
    let mut out = vec![];
    for soil in Soil::ALL {
        let lt = pisa_run(&symmetric_tower(), soil);
        assert!(monotonic(&lt), "{soil:?}");
        out.push((soil, lt.last().unwrap().max_settle));
    }
    eprintln!("{out:?}");
    assert_eq!(out[0], (Soil::Rock, 0.0));
    let soft = out.iter().find(|o| o.0 == Soil::Soft).unwrap().1;
    assert!(out.iter().all(|o| o.1 <= soft));
    assert!(soft > 0.1, "soft clay settles tens of centimetres: {soft}");
    let dense = out.iter().find(|o| o.0 == Soil::Dense).unwrap().1;
    assert!(dense < 0.01, "dense sand a few millimetres: {dense}");
    let rock = pisa_run(&symmetric_tower(), Soil::Rock);
    assert_eq!(
        rock.sentence(),
        "Rock doesn\u{2019}t settle: it stands exactly as built."
    );
    assert_eq!(
        rock.result_line(),
        "After 30 years: tilt 0.0°, settled 0 mm."
    );
}

#[test]
fn a_symmetric_tower_does_not_tilt_an_eccentric_one_does() {
    let sym = pisa_run(&symmetric_tower(), Soil::Soft);
    assert_eq!(sym.ending, Ending::Stood);
    assert!(
        sym.last().unwrap().tilt.abs() < 0.05,
        "{}",
        sym.last().unwrap().tilt
    );
    assert_eq!(sym.sentence(), "It sank evenly, so it barely leans.");
    let ecc = pisa_run(&pisa::prebuilt(), Soil::Soft);
    let t = ecc.last().unwrap().tilt;
    assert!(t > 2.0, "leans right, toward the bells: {t}");
    // the tilt grows every step and the low side carries more and more (P-Δ)
    assert!(ecc.frames.windows(2).all(|w| w[1].tilt >= w[0].tilt));
    let r = |f: &longterm::YearFrame, id: &str| f.reaction[ecc.joint_index(id).unwrap()];
    let (first, last) = (&ecc.frames[0], ecc.last().unwrap());
    assert!(r(last, "J3") - r(last, "J1") > r(first, "J3") - r(first, "J1"));
    assert!(ecc.result_line().starts_with("After 30 years: tilt 4."));
}

#[test]
fn readouts_interpolate_between_frames() {
    let lt = pisa_run(&pisa::prebuilt(), Soil::Soft);
    let a = lt.at(12.0);
    let b = lt.at(12.25);
    let c = lt.at(12.5);
    assert!((b.tilt - (a.tilt + c.tilt) / 2.0).abs() < 1e-12);
    assert_eq!(lt.at(-5.0).year, 0.0);
    assert_eq!(lt.at(99.0).year, 30.0);
    assert_eq!(lt.at(f64::NAN).year, 0.0);
    let chips = lt.chips(12.0);
    assert_eq!(chips[0].0, "Year ");
    assert_eq!(chips[0].1, "12.0");
    assert_eq!(chips[1].0, "Tilt ");
    assert!(chips[1].1.ends_with('°'));
    assert_eq!(chips[2].0, "Settled ");
    assert!(chips[2].1.ends_with(" mm"));
    let (base, top) = lt.lean_line(30.0).unwrap();
    let f = lt.last().unwrap();
    // the tower is built plumb, so the lean line's angle is the tilt
    let angle = (top[0] - base[0]).atan2(top[1] - base[1]).to_degrees();
    assert!((angle - f.tilt).abs() < 1e-9, "{angle} vs {}", f.tilt);
    assert_eq!(lt.found.len(), 3);
    assert_eq!(longterm::anim_year(0.0, 30.0), 0.0);
    assert_eq!(longterm::anim_year(3.0, 30.0), 15.0);
    assert_eq!(longterm::anim_year(60.0, 30.0), 30.0);
    assert_eq!(longterm::anim_year(60.0, 17.5), 17.5);
}

#[test]
fn a_tower_leaning_past_five_degrees_topples() {
    let mut d = pisa::prebuilt();
    d.set_load(&pisa::bells_joint(), 14.0);
    let lt = pisa_run(&d, Soil::Soft);
    let Ending::Toppled(y) = lt.ending else {
        panic!("{:?}", lt.ending)
    };
    assert!(y > 0.0 && y < 30.0);
    assert!(lt.result_line().starts_with("It toppled in year"));
    assert!(lt.sentence().contains("tipped over"));
    assert!(lt.at(30.0).fallen);
    assert!(!lt.at(0.0).fallen);
}

#[test]
fn bridges_report_differential_settlement() {
    // the storm board's wood truss over the river, with a pier footing on the soft riverbed
    let mut g = Game::with_preset(&Preset::board("BridgeBuild").unwrap());
    g.pick_level(LevelKey::Haul);
    g.build_preset(PresetBuild::Truss {
        chord: PieceType::Steel,
    });
    let (d, s, c) = (g.design().clone(), g.setup(), *g.cond());
    let lt = longterm::fast_forward_design(&d, &s, &c);
    assert!(!lt.is_tower());
    assert_eq!(lt.chips(30.0)[1].0, "Differential ");
    assert!(
        lt.result_line().contains("differential settlement"),
        "{}",
        lt.result_line()
    );
    assert!(monotonic(&lt));
    // banks of dense sand: little settlement, about even
    let f = lt.last().unwrap();
    assert!(f.max_settle < 0.05, "{}", f.max_settle);
}

#[test]
fn nothing_to_settle_without_foundations() {
    let level = pisa::level();
    let cond = Conditions::for_level(&level);
    let setup = Setup::new(LevelKey::Pisa, Scene::Flat, &cond);
    let mut d = symmetric_tower();
    for n in &mut d.nodes {
        n.sup = None;
    }
    assert!(!longterm::has_foundations(&d));
    let lt = longterm::fast_forward_design(&d, &setup, &cond);
    assert_eq!(lt.ending, Ending::Nothing);
    assert_eq!(lt.frames.len(), 1);
    assert!(longterm::has_foundations(&pisa::prebuilt()));
    let lt = longterm::fast_forward_design(&Design::default(), &setup, &cond);
    assert_eq!(lt.ending, Ending::Nothing);
}

/// Random structures on both scenes: nothing panics, every number is finite, settlement never
/// comes back up.
#[test]
fn weird_structures_do_not_panic() {
    let mut rng = SplitMix64::new(7);
    let mut r = move |n: u64| (rng.next_u64() % n.max(1)) as usize;
    for case in 0..120 {
        let river = case % 2 == 0;
        let (key, scene) = if river {
            (LevelKey::Sandbox, Scene::River)
        } else {
            (LevelKey::Sandbox, Scene::Flat)
        };
        let mut cond = Conditions::for_level(&key.level());
        cond.soil = Soil::ALL[r(5)];
        let setup = Setup::new(key, scene, &cond);
        let mut d = Design::default();
        let nj = 2 + r(14);
        for i in 0..nj {
            let (x0, step) = if river { (0.0, 2.0) } else { (88.0, 4.0) };
            let x = x0 + step * r(14) as f64;
            let y = if r(3) == 0 { 0.0 } else { step * r(8) as f64 };
            let y = if river && r(5) == 0 { -12.0 } else { y };
            d.nodes.push(Joint {
                id: format!("J{}", i + 1),
                x,
                y,
                sup: match (y <= 0.0, r(3)) {
                    (true, 0) => Some(Foundation::Footing),
                    (true, 1) => Some(Foundation::Pile),
                    _ => None,
                },
                locked: false,
            });
            if r(4) == 0 {
                d.set_load(&format!("J{}", i + 1), [1.0, 5.0, 25.0][r(3)]);
            }
        }
        d.n_n = nj as u32;
        for k in 0..(nj + r(2 * nj as u64)) {
            let (a, b) = (r(nj as u64), r(nj as u64));
            let kind = PieceType::ALL[r(7)];
            d.members.push(Piece {
                id: format!("M{}", k + 1),
                a: format!("J{}", a + 1),
                b: format!("J{}", b + 1),
                kind,
                locked: false,
            });
        }
        d.n_m = d.members.len() as u32;
        let lt = longterm::fast_forward_design(&d, &setup, &cond);
        assert!(!lt.frames.is_empty(), "case {case}");
        assert!(monotonic(&lt), "case {case}");
        for f in &lt.frames {
            assert!(f.tilt.is_finite() && f.max_settle.is_finite() && f.differential.is_finite());
            assert!(f.force.iter().all(|v| v.is_finite()), "case {case}");
        }
        let _ = (
            lt.result_line(),
            lt.sentence(),
            lt.chips(lt.end_year() / 2.0),
        );
    }
}
