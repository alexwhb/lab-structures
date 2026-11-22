//! Netlist round trips (SPEC ADV-9, NAV-6): the writer's text parses back to the same model,
//! and a Bridge-like structure written with `write_truss` solves to the same forces as solving
//! it directly (the Bridge lumps self-weight half per member end).

use sp_engine::generate::{self, MaterialKind, Params, Style};
use sp_engine::netlist::{self, Netlist, SelfWeight, WriteTruss};
use sp_engine::{Fix, Material, Member, NodalLoad, Node, SolveOptions, Truss};

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(1e-12)
}

fn same_model(a: &Netlist, b: &Netlist) {
    assert_eq!(a.title, b.title);
    assert_eq!(a.units, b.units);
    assert_eq!(a.nodes.len(), b.nodes.len());
    for (x, y) in a.nodes.iter().zip(&b.nodes) {
        assert_eq!(x.id, y.id);
        assert_eq!(x.fix, y.fix);
        assert!(close(x.x, y.x, 1e-11) && close(x.y, y.y, 1e-11));
    }
    assert_eq!(a.members, b.members);
    assert_eq!(a.sections.len(), b.sections.len());
    for (x, y) in a.sections.iter().zip(&b.sections) {
        assert_eq!(x.name, y.name);
        assert!(close(x.area, y.area, 1e-11) && close(x.inertia, y.inertia, 1e-11));
    }
    for (x, y) in a.materials.iter().zip(&b.materials) {
        assert_eq!(x.name, y.name);
        assert!(close(x.e, y.e, 1e-11) && close(x.fy, y.fy, 1e-11) && close(x.rho, y.rho, 1e-11));
    }
    assert_eq!(a.loads.len(), b.loads.len());
    for (x, y) in a.loads.iter().zip(&b.loads) {
        assert_eq!(
            (&x.case, &x.nodes, x.self_weight),
            (&y.case, &y.nodes, y.self_weight)
        );
        assert!(close(x.fx, y.fx, 1e-11) && close(x.fy, y.fy, 1e-11));
    }
    match (&a.truck, &b.truck) {
        (Some(x), Some(y)) => {
            assert_eq!(
                (&x.case, &x.lane, x.share, x.tonnes),
                (&y.case, &y.lane, y.share, y.tonnes)
            );
            assert!(close(x.weight, y.weight, 1e-11) && close(x.x, y.x, 1e-11));
        }
        (None, None) => {}
        _ => panic!("truck differs"),
    }
    assert_eq!(a.k_factor, b.k_factor);
    assert_eq!(a.plot, b.plot);
}

#[test]
fn generated_netlists_round_trip_through_the_writer() {
    for style in Style::ALL {
        for braced in [false, true] {
            for material in MaterialKind::ALL {
                let p = Params {
                    style,
                    braced,
                    material,
                    size: 7,
                    truck_t: 33.0,
                    x: 17.5,
                };
                let parsed = netlist::parse(&generate::generate(&p).text).unwrap();
                let text = netlist::write(&parsed);
                let again = netlist::parse(&text).unwrap_or_else(|e| panic!("{text}\n{e:?}"));
                same_model(&parsed, &again);
                // writing is stable
                assert_eq!(netlist::write(&again), text);
            }
        }
    }
}

/// A Bridge-like structure: a 16 m Warren on the river banks, Thin / Medium / Thick members, a
/// 20 t weight in the middle and self-weight lumped at the member ends.
fn bridge_like() -> (Truss, Vec<String>, Vec<NodalLoad>) {
    let nodes = vec![
        Node::new("J1", 8.0, 0.0, Fix::PIN),
        Node::new("J2", 16.0, 0.0, Fix::FREE),
        Node::new("J3", 24.0, 0.0, Fix::ROLLER),
        Node::new("J4", 12.0, 4.0, Fix::FREE),
        Node::new("J5", 20.0, 4.0, Fix::FREE),
    ];
    let pairs = [
        (0, 1, 0.008, "Medium"),
        (1, 2, 0.008, "Medium"),
        (0, 3, 0.016, "Thick"),
        (3, 1, 0.004, "Thin"),
        (1, 4, 0.004, "Thin"),
        (4, 2, 0.016, "Thick"),
        (3, 4, 0.008, "Medium"),
    ];
    let members = pairs
        .iter()
        .enumerate()
        .map(|(k, (a, b, area, _))| Member::box_section(format!("M{}", k + 1), *a, *b, *area, 0))
        .collect();
    let sections = pairs.iter().map(|p| p.3.to_string()).collect();
    let truss = Truss {
        nodes,
        members,
        materials: vec![Material::new("Steel", 200e9, 250e6, 7850.0)],
    };
    (truss, sections, vec![NodalLoad::down(1, 20.0 * 9810.0)])
}

#[test]
fn a_bridge_structure_solves_the_same_through_the_netlist() {
    let (truss, sections, loads) = bridge_like();
    // the Bridge's own solve: weights + half of each member's weight at each end
    let mut all = loads.clone();
    all.extend(truss.self_weight(1.0, &[]));
    let direct = truss.solve(&all, &SolveOptions::default()).unwrap();
    assert!(direct.stable);

    let text = netlist::write_truss(&WriteTruss {
        name: "sandbox",
        comment: "Bridge · Sandbox",
        truss: &truss,
        sections: &sections,
        loads: &loads,
        self_weight: true,
    });
    assert!(
        text.starts_with("* sandbox.str\n* Bridge · Sandbox\n.units SI  kN  m\n"),
        "{text}"
    );
    assert!(text.contains(".section Medium  A=80cm2  I=A^2"), "{text}");
    assert!(text.contains(".section Thin  A=40cm2  I=A^2"), "{text}");
    assert!(text.contains(".node J1  8  0  fix=xy"), "{text}");
    assert!(text.contains(".node J3  24  0  fix=y"), "{text}");
    assert!(text.contains(".load weights  J2  Fy=-20t"), "{text}");
    assert!(text.contains(".load self  +self"), "{text}");

    let net = netlist::parse(&text).unwrap_or_else(|e| panic!("{text}\n{e:?}"));
    assert!(net.warnings.is_empty());
    assert_eq!(net.member_sections(), sections);
    assert_eq!(net.self_weight(), SelfWeight::MemberEnds);
    let back = net.to_truss();
    assert_eq!(back.nodes, truss.nodes);
    assert_eq!(back.members.len(), truss.members.len());
    let pl = net.point_loads();
    assert_eq!(pl.len(), 1);
    assert!((pl[0].fy - loads[0].fy).abs() < 1e-6);

    let via = net.solve_at(0.0).unwrap();
    for (a, b) in direct.members.iter().zip(&via.members) {
        let (a, b) = (a.unwrap(), b.unwrap());
        assert!(
            (a.force - b.force).abs() < 1.0,
            "{} vs {} N",
            a.force,
            b.force
        );
    }
    for (a, b) in direct.reactions.iter().zip(&via.reactions) {
        assert!((a[1] - b[1]).abs() < 1.0);
    }
}

#[test]
fn writer_makes_odd_names_parseable() {
    let (mut truss, _, loads) = bridge_like();
    truss.nodes[0].id = "joint one".into();
    truss.nodes[1].id = "2".into();
    truss.members[0].id = "M2".into(); // duplicate of the second member's id
    truss.materials[0].name = "Carbon fiber".into();
    let text = netlist::write_truss(&WriteTruss {
        name: "",
        comment: "",
        truss: &truss,
        sections: &[],
        loads: &loads,
        self_weight: false,
    });
    let net = netlist::parse(&text).unwrap_or_else(|e| panic!("{text}\n{e:?}"));
    assert_eq!(net.title.as_deref(), Some("untitled.str"));
    assert_eq!(net.nodes[0].id, "joint_one");
    assert_eq!(net.nodes[1].id, "N2");
    assert_eq!(net.materials[0].name, "carbon_fiber");
    assert_eq!(net.sections.len(), 3); // s1, s2, s3 by distinct area
    assert_eq!(net.self_weight(), SelfWeight::None);
    let ids: std::collections::HashSet<_> = net.members.iter().map(|m| &m.id).collect();
    assert_eq!(ids.len(), net.members.len());
    // a load that isn't whole tonnes is written in kN
    let odd = netlist::write_truss(&WriteTruss {
        name: "x",
        comment: "",
        truss: &truss,
        sections: &[],
        loads: &[NodalLoad {
            node: 1,
            fx: 1234.5,
            fy: -1234.0,
        }],
        self_weight: false,
    });
    assert!(odd.contains("Fx=1.2345  Fy=-1.234"), "{odd}");
    let n = netlist::parse(&odd).unwrap();
    assert!((n.loads[0].fx - 1234.5).abs() < 1e-9);
}
