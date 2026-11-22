//! The `.str` parser (SPEC ADV-3): every directive, units, ranges, warnings for the other
//! worlds' directives, and errors with line and column.

use sp_engine::netlist::{self, Diagnostic, SelfWeight, Severity};
use sp_engine::{Fix, G};

const HEAD: &str =
    "* t.str\n.material steel E=200GPa fy=250MPa rho=7850\n.section s A=100cm2 I=A^2\n";

fn errors(text: &str) -> Vec<Diagnostic> {
    match netlist::parse(text) {
        Ok(n) => panic!("expected errors, parsed: {n:?}"),
        Err(d) => d.into_iter().filter(Diagnostic::is_error).collect(),
    }
}

/// The first error's (line, column, message).
fn first(text: &str) -> (usize, usize, String) {
    let e = errors(text);
    let d = &e[0];
    (d.line, d.column, d.message.clone())
}

#[test]
fn parses_every_directive() {
    let text = "\
* bridge.str
* a note
.units SI  kN  m
.material steel  E=200GPa  fy=250MPa  rho=7850   ; representative
.material wood  12GPa  24MPa  500
.section chord  A=100cm2  I=A^2
.section web    A=0.006  I=2e-5
.node A  0  0  fix=xy  ; pin
.node B  8  0  fix=y
.node C  4  3
.member M1  A  C  chord
.member M2  C  B  chord  wood
.member M3  A  B  web
.load dead  C  0  -10
.load more  A..B  Fy=-1.5t  ; ranges need a common prefix
.load sw  +self
.load truck  20t  x=2  lane=A,B  share=0.5
.static linear
.buckle euler  K=0.7
.plot N(M3)  vs truck.x  0 8 0.5
";
    // A..B is not a numeric range: expect exactly that error, then fix it and parse
    let e = errors(text);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!((e[0].line, e[0].column), (15, 13));
    let text = text.replace("A..B", "A B");
    let n = netlist::parse(&text).unwrap();
    assert_eq!(n.title.as_deref(), Some("bridge.str"));
    assert_eq!(n.stem(), Some("bridge"));
    assert_eq!(n.notes, vec!["a note".to_string()]);
    assert_eq!(n.materials.len(), 2);
    assert_eq!(n.materials[1].e, 12e9);
    assert_eq!(n.materials[1].fy, 24e6);
    assert_eq!(n.sections[0].area, 0.01);
    assert!(n.sections[0].is_box());
    assert_eq!(n.sections[1].inertia, 2e-5);
    assert_eq!(n.nodes[0].fix, Fix::PIN);
    assert_eq!(n.nodes[1].fix, Fix::ROLLER);
    assert_eq!(n.members[1].material, 1);
    assert_eq!(n.loads[0].fy, -10e3);
    assert_eq!(n.loads[1].nodes, vec![0, 1]);
    assert_eq!(n.loads[1].fy, -(1.5 * G * 1e3));
    assert_eq!(n.self_weight(), SelfWeight::MemberEnds);
    let t = n.truck.as_ref().unwrap();
    assert_eq!(
        (t.weight, t.tonnes, t.x, t.share),
        (20.0 * G * 1e3, Some(20.0), 2.0, 0.5)
    );
    assert_eq!(n.copies(), 2);
    assert_eq!(n.k_factor, 0.7);
    let p = n.plot.as_ref().unwrap();
    assert_eq!(p.member.as_deref(), Some("M3"));
    assert_eq!(p.range, Some((0.0, 8.0, 0.5)));
    assert!(n.warnings.is_empty(), "{:?}", n.warnings);
    // and it solves
    let sol = n.solve_at(2.0).unwrap();
    assert!(sol.stable);
}

#[test]
fn units_change_bare_numbers() {
    let n = netlist::parse(
        ".units N mm\n.material s E=200000 fy=250\n.section a A=1000\n.node A 0 0 fix=xy\n.node B 8000 0 fix=y\n.member M A B a\n.load d A B 0 -1000\n",
    )
    .unwrap();
    assert_eq!(n.materials[0].e, 200e9); // N/mm² = MPa
    assert_eq!(n.materials[0].fy, 250e6);
    assert!((n.sections[0].area - 1e-3).abs() < 1e-18);
    assert_eq!(n.nodes[1].x, 8.0);
    assert_eq!(n.loads[0].fy, -1000.0);
    // explicit units override, and order doesn't matter (units at the end)
    let n = netlist::parse(
        ".node A 0 0 fix=xy\n.node B 800cm 0 fix=y\n.member M A B a\n.material s 200GPa 250MPa 7850\n.section a 10cm2\n.units kN m\n",
    )
    .unwrap();
    assert_eq!(n.nodes[1].x, 8.0);
    assert_eq!(n.sections[0].area, 10.0 / 1e4);
}

#[test]
fn presets_fill_in_known_materials() {
    let n = netlist::parse(".material steel\n.material alu fy=200MPa\n").unwrap();
    assert_eq!(n.materials[0].e, 200e9);
    assert_eq!(n.materials[0].rho, 7850.0);
    assert_eq!(n.materials[1].fy, 200e6);
    assert_eq!(n.materials[1].e, 69e9);
}

#[test]
fn other_worlds_are_warnings() {
    let text = format!(
        "{HEAD}.story 60 4 900t core\n.wind 30 0.2\n.soil 1 10 clay\n.foundation piles 30 16\n.dynamic quake 10 0.01 0.05\n.static nonlinear\n.buckle eigen 3\n"
    );
    let n = netlist::parse(&text).unwrap();
    let msgs: Vec<&str> = n.warnings.iter().map(|w| w.message.as_str()).collect();
    assert_eq!(n.warnings.len(), 7, "{msgs:?}");
    assert!(msgs[0].contains(".story is used by the Tower world"));
    assert!(msgs[1].contains(".wind is used by the Tower world"));
    assert!(msgs[2].contains(".soil is used by the Ground world"));
    assert!(msgs[3].contains(".foundation is used by the Ground world"));
    assert!(msgs[4].contains("Tower and Ground"));
    assert!(n.warnings.iter().all(|w| w.severity == Severity::Warning));
    assert_eq!(n.warnings[0].line, 4);
}

#[test]
fn errors_point_at_their_line_and_column() {
    // unknown directive
    assert_eq!(
        first(&format!("{HEAD}.nod A 0 0\n")),
        (
            4,
            1,
            "unknown directive .nod (try .node, .member, .material, .section, .load)".into()
        )
    );
    // bad number
    let (l, c, m) = first(&format!("{HEAD}.node A 0 zero\n"));
    assert_eq!((l, c), (4, 11));
    assert!(m.contains("expected a number"), "{m}");
    // unknown node
    let (l, c, m) = first(&format!("{HEAD}.node A 0 0\n.member M1  A  Q  s\n"));
    assert_eq!((l, c, m.as_str()), (5, 16, "unknown node Q"));
    // unknown section
    let (l, c, m) = first(&format!(
        "{HEAD}.node A 0 0\n.node B 1 0\n.member M1 A B big\n"
    ));
    assert_eq!((l, c, m.as_str()), (6, 16, "unknown section big"));
    // duplicate id
    let (l, c, m) = first(&format!("{HEAD}.node A 0 0\n.node A 1 0\n"));
    assert_eq!((l, c), (5, 7));
    assert!(m.contains("duplicate node A (first on line 4)"), "{m}");
    // missing args
    let (l, _, m) = first(&format!("{HEAD}.node A 0\n"));
    assert_eq!(l, 4);
    assert!(m.contains(".node needs y"), "{m}");
    let (_, _, m) = first(&format!("{HEAD}.node A 0 0\n.node B 1 0\n.member M1 A B\n"));
    assert!(m.contains("needs a section"), "{m}");
    // not a directive at all
    let (l, c, _) = first("node A 0 0\n");
    assert_eq!((l, c), (1, 1));
    // zero-length member
    let (_, _, m) = first(&format!(
        "{HEAD}.node A 0 0\n.node B 0 0\n.member M1 A B s\n"
    ));
    assert!(m.contains("no length"), "{m}");
    // unknown unit, bad fix, range over the edge, bad share
    assert!(
        first(&format!("{HEAD}.node A 0 0 fix=z\n"))
            .2
            .contains("fix=z")
    );
    assert!(
        first(".material s E=200Gpb fy=1\n")
            .2
            .contains("unknown stress unit")
    );
    let (_, c, m) = first(&format!(
        "{HEAD}.node B0 0 0\n.node B1 1 0\n.load d B0..B5 Fy=-1\n"
    ));
    assert!(m.contains("B0..B5"), "{m}");
    assert_eq!(c, 9);
    assert!(
        first(&format!(
            "{HEAD}.node B0 0 0\n.node B1 1 0\n.load t 1t lane=B0..B1 share=2\n"
        ))
        .2
        .contains("share")
    );
    // a column counts characters, not bytes
    let (_, c, m) = first(".material s E=1GPa fy=1MPa curve=ä bad=1\n");
    assert_eq!(c, 36, "{m}");
    // comma lists keep their columns
    let (_, c, m) = first(&format!("{HEAD}.node B0 0 0\n.load d B0,Bx Fy=-1\n"));
    assert_eq!((c, m.as_str()), (12, "unknown node Bx"));
}

#[test]
fn every_error_is_reported_with_warnings_sorted() {
    let text = format!("{HEAD}.wind 30\n.nod\n.node A x 0\n.member M9 A B s\n");
    let all = netlist::parse(&text).unwrap_err();
    let lines: Vec<(usize, Severity)> = all.iter().map(|d| (d.line, d.severity)).collect();
    assert_eq!(
        lines,
        vec![
            (4, Severity::Warning),
            (5, Severity::Error),
            (6, Severity::Error),
            // line 7's "unknown node A" is dropped: A's own line (6) has the error
        ]
    );
}

#[test]
fn one_mistake_one_message() {
    // a node with a bad coordinate is still a name: its members don't pile on errors
    let text = format!(
        "{HEAD}.node A 0 0 fix=xy\n.node B oops 0\n.node C 8 0 fix=y\n.member M1 A B s\n.member M2 B C s\n.load d A B C Fy=-1\n"
    );
    let e = errors(&text);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].line, 5);
    // but a name that was never declared still errors
    let e = errors(&format!(
        "{HEAD}.node A 0 0\n.node B 1 0\n.member M1 A Z s\n"
    ));
    assert_eq!(e[0].message, "unknown node Z");
}

#[test]
fn materials_bench_share_line_parses() {
    // Materials' Share (SPEC NAV-6): accepted, curve= warned about
    let n = netlist::parse(
        ".material steel E=200GPa fy=275MPa rho=7850 curve=steel ; tensile test at 20 °C, ε = 0.00 %, Ready",
    )
    .unwrap();
    assert_eq!(n.materials[0].fy, 275e6);
    assert_eq!(n.warnings.len(), 1);
    assert!(n.warnings[0].message.contains("curve="));
}

#[test]
fn comments_and_blank_lines() {
    let n = netlist::parse(
        "\n   \n* x.str\n; a whole-line ; comment\n.material steel ; tail\n\t* indented comment\n",
    )
    .unwrap();
    assert_eq!(n.materials.len(), 1);
    assert_eq!(n.title.as_deref(), Some("x.str"));
    // an empty netlist is fine (nothing to solve)
    let n = netlist::parse("").unwrap();
    assert!(n.nodes.is_empty());
    assert!(n.solve_at(0.0).unwrap().stable);
}

#[test]
fn self_weight_shared_among_nodes() {
    let text = format!(
        "{HEAD}.node A 0 0 fix=xy\n.node B 8 0 fix=y\n.node C 4 3\n.member M1 A C s\n.member M2 C B s\n.member M3 A B s\n.load dead A C B Fy=0 +self\n"
    );
    let n = netlist::parse(&text).unwrap();
    assert_eq!(n.self_weight(), SelfWeight::Shared);
    let truss = n.to_truss();
    let loads = n.static_loads(&truss);
    let total: f64 = loads.iter().map(|l| -l.fy).sum();
    let want = 0.01 * (5.0 + 5.0 + 8.0) * 7850.0 * G;
    assert!((total - want).abs() < 1e-9 * want);
    assert_eq!(loads.len(), 3);
}

#[test]
fn the_truck_splits_between_lane_nodes() {
    let text = format!(
        "{HEAD}.node B0 0 0 fix=xy\n.node B1 8 0\n.node B2 16 0 fix=y\n.load t 10t x=6 lane=B2..B0 share=1\n"
    );
    let n = netlist::parse(&text).unwrap();
    // lane written backwards still rolls left to right
    assert_eq!(n.lane_nodes(), vec![0, 1, 2]);
    let l = n.truck_loads(6.0);
    let w = 10.0 * G * 1e3;
    assert_eq!(l.len(), 2);
    assert_eq!((l[0].node, l[1].node), (0, 1));
    assert!((l[0].fy + w * 0.25).abs() < 1e-9 && (l[1].fy + w * 0.75).abs() < 1e-9);
    assert!(n.truck_loads(-0.1).is_empty() && n.truck_loads(16.1).is_empty());
    assert_eq!(n.truck_loads(16.0)[1].node, 2);
}
