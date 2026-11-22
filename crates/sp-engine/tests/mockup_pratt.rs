//! The Advanced mockup's default state, rebuilt by hand, must reproduce the numbers the mockup
//! renders (`docs/design/screenshots/Advanced-1440.png`): 48 m Pratt through-truss, steel,
//! size 4, a 40 t truck at x = 24 m, half per truss.

use sp_engine::{Fix, G, Material, Member, NodalLoad, Node, SolveOptions, Truss};

fn pratt() -> Truss {
    let mut nodes = Vec::new();
    for i in 0..=6 {
        let fix = match i {
            0 => Fix::PIN,
            6 => Fix::ROLLER,
            _ => Fix::FREE,
        };
        nodes.push(Node::new(format!("B{i}"), i as f64 * 8.0, 0.0, fix));
    }
    for i in 1..6 {
        nodes.push(Node::new(format!("T{i}"), i as f64 * 8.0, 6.0, Fix::FREE));
    }
    let idx = |id: &str| nodes.iter().position(|n| n.id == id).unwrap();
    let size = 4.0;
    let (chord, web) = (0.0025 * size, 0.0015 * size);
    let mut pairs: Vec<(String, String, f64)> = Vec::new();
    for i in 0..6 {
        pairs.push((format!("B{i}"), format!("B{}", i + 1), chord));
    }
    for i in 1..5 {
        pairs.push((format!("T{i}"), format!("T{}", i + 1), chord));
    }
    pairs.push(("B0".into(), "T1".into(), chord));
    pairs.push(("T5".into(), "B6".into(), chord));
    for i in 1..6 {
        pairs.push((format!("B{i}"), format!("T{i}"), web));
    }
    for (a, b) in [("T1", "B2"), ("T2", "B3"), ("B3", "T4"), ("B4", "T5")] {
        pairs.push((a.into(), b.into(), web));
    }
    let members = pairs
        .iter()
        .enumerate()
        .map(|(k, (a, b, area))| {
            Member::box_section(format!("M{}", k + 1), idx(a), idx(b), *area, 0)
        })
        .collect();
    Truss {
        nodes,
        members,
        materials: vec![Material::new("Steel", 200e9, 250e6, 7850.0)],
    }
}

#[test]
fn default_advanced_state_matches_the_mockup() {
    let t = pratt();
    assert_eq!(t.members.len(), 21);
    assert_eq!(t.determinacy(), 0);
    let vol: f64 = (0..t.members.len())
        .map(|m| t.members[m].area * t.length(m))
        .sum();
    let self_w = vol * 7850.0 * G;
    let mut loads: Vec<NodalLoad> = (1..=5)
        .map(|i| NodalLoad::down(i, 80e3 + self_w / 5.0))
        .collect();
    let truck = 40.0 * G * 1e3 / 2.0; // 40 t, half per truss
    loads.push(NodalLoad::down(3, truck));
    let sol = t.solve(&loads, &SolveOptions::default()).unwrap();
    assert!(sol.stable);
    let kn = |id: &str| sol.members[t.member_index(id).unwrap()].unwrap().force / 1e3;
    // Member results table: M8/M9 −1004, M7/M10 −805, M3/M4 805, M11/M12 −588, M18/M21 418, M20 248
    for (id, want) in [
        ("M8", -1004.0),
        ("M9", -1004.0),
        ("M7", -805.0),
        ("M3", 805.0),
        ("M12", -588.0),
        ("M18", 418.0),
        ("M1", 470.0),
        ("M20", 248.0),
    ] {
        assert!(
            (kn(id) - want).abs() < 0.5,
            "{id}: {:.2} kN, mockup {want}",
            kn(id)
        );
    }
    // Utilisation and modes: M9 crush 40 %, M12 buckle 1974 kN at 30 %
    let m9 = sol.members[t.member_index("M9").unwrap()].unwrap();
    assert_eq!(m9.mode, sp_engine::FailureMode::Crushing);
    assert_eq!((m9.utilisation * 100.0).round(), 40.0);
    let m12 = sol.members[t.member_index("M12").unwrap()].unwrap();
    assert_eq!(m12.mode, sp_engine::FailureMode::Buckling);
    assert_eq!((m12.limit / 1e3).round(), 1974.0);
    // M8 and M9 tie to rounding; the mockup lists M9 first.
    assert!(sol.worst == t.member_index("M8") || sol.worst == t.member_index("M9"));
    // Reactions A 353 kN · B 353 kN
    assert_eq!((sol.reactions[0][1] / 1e3).round(), 353.0);
    assert_eq!((sol.reactions[6][1] / 1e3).round(), 353.0);
    // Midspan deflection 53.1 mm = L/903
    let sag = -sol.displacements[3][1];
    assert!(
        ((sag * 1000.0) - 53.1).abs() < 0.05,
        "{:.3} mm",
        sag * 1000.0
    );
    assert_eq!((48.0 / sag).round(), 903.0);
    // Steel weight 22.3 t (both trusses)
    assert!(((2.0 * vol * 7850.0 / 1000.0) - 22.3).abs() < 0.05);
}
