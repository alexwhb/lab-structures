//! The Bridge's structure as a `.str` netlist and back (Share, "Open in Advanced", "Open in
//! Playground", ⌘V: SPEC NAV-4, NAV-6, ADV-9), on the Advanced workbench's
//! [`crate::netlist`] writer and parser.
//!
//! [`write`] gives `.material` (the game material), one `.section` per size used (`Thin` /
//! `Medium` / `Thick`, `I=A^2`), `.node`s with `fix=xy` (pin) / `fix=y` (roller), `.member`s
//! with their size, the hung weights as `.load weights … Fy=-20t`, `.load self +self` (half of
//! each member's weight at each end, the Bridge's lumping), `.static linear`, `.buckle euler
//! K=1.0`: solving it in Advanced gives the Bridge's forces to the newton. [`read`] takes any
//! netlist whose nodes fit the stage's 2 m grid into a [`Design`] for the Sandbox.

use super::data::{LevelKey, MaterialKey};
use super::design::{Design, ImportError};
use crate::netlist::{self, WriteTruss};

/// Why a netlist can't go into the Bridge.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ReadError {
    /// It doesn't parse (the first error, with its line).
    #[error("line {line}: {message}")]
    Parse {
        /// 1-based line.
        line: usize,
        /// The parser's message.
        message: String,
    },
    /// It parses but doesn't fit the Bridge (off the grid, underground, sideways loads, …).
    #[error("{0}")]
    Import(#[from] ImportError),
    /// No joints at all.
    #[error("it has no joints")]
    Empty,
}

/// Writes the design as a netlist named `name` (the file stem: `river-level`, `sandbox`).
pub fn write(design: &Design, mat: MaterialKey, level: LevelKey, name: &str) -> String {
    let (truss, loads) = design.to_truss(mat, 1.0);
    // to_truss keeps the members whose joints exist, in order
    let sections: Vec<String> = design
        .members
        .iter()
        .filter(|m| design.joint(&m.a).is_some() && design.joint(&m.b).is_some())
        .map(|m| m.size.name().to_string())
        .collect();
    let comment = format!(
        "Bridge · {} · {} joints, {} members",
        level.level().name,
        design.nodes.len(),
        design.members.len()
    );
    netlist::write_truss(&WriteTruss {
        name,
        comment: &comment,
        truss: &truss,
        sections: &sections,
        loads: &loads,
        self_weight: true,
    })
}

/// Reads a netlist into a design and its material (the first `.material`, matched to the
/// Bridge's four by name; anything else builds in steel). Static point loads and the moving
/// truck where it stands become hung weights.
pub fn read(text: &str) -> Result<(Design, MaterialKey), ReadError> {
    let net = netlist::parse(text).map_err(|diags| {
        let d = diags
            .iter()
            .find(|d| d.is_error())
            .or(diags.first())
            .cloned();
        match d {
            Some(d) => ReadError::Parse {
                line: d.line,
                message: d.message,
            },
            None => ReadError::Parse {
                line: 1,
                message: "not a netlist".into(),
            },
        }
    })?;
    if net.nodes.is_empty() {
        return Err(ReadError::Empty);
    }
    let truss = net.to_truss();
    let mut loads = net.point_loads();
    if net.truck.is_some() {
        loads.extend(net.truck_loads(net.truck_x()));
    }
    let mat = truss
        .materials
        .first()
        .and_then(|m| MaterialKey::from_name(&m.name))
        .unwrap_or_default();
    let design = Design::from_truss(&truss, &loads)?;
    Ok((design, mat))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::data::SizeKey;

    #[test]
    fn writes_the_starter_truss() {
        let d = Design::starter();
        let t = write(&d, MaterialKey::Steel, LevelKey::Sandbox, "sandbox");
        assert!(t.starts_with("* sandbox.str\n"), "{t}");
        assert!(t.contains("Medium"));
        assert!(!t.contains("Thin"));
        assert!(t.contains("fix=xy"));
        assert!(t.contains("fix=y"));
        assert_eq!(t.lines().filter(|l| l.starts_with(".node")).count(), 10);
        assert_eq!(t.lines().filter(|l| l.starts_with(".member")).count(), 16);
    }

    #[test]
    fn round_trips_designs_and_materials() {
        for mat in MaterialKey::ALL {
            let mut d = Design::starter();
            d.members[2].size = SizeKey::Thick;
            d.members[9].size = SizeKey::Thin;
            d.set_load("J3", 20.0);
            d.set_load("J9", 7.5);
            let text = write(&d, mat, LevelKey::Sandbox, "sandbox");
            let (back, m) = read(&text).unwrap_or_else(|e| panic!("{mat:?}: {e}\n{text}"));
            assert_eq!(m, mat, "{text}");
            assert_eq!(back, d, "{text}");
        }
    }

    /// ADV-9: the netlist solved by the Advanced side gives the Bridge's forces to the newton.
    #[test]
    fn advanced_solves_it_to_the_newton() {
        let mut d = Design::starter();
        d.members.push(crate::bridge::Bar {
            id: "M17".into(),
            a: "J8".into(),
            b: "J4".into(),
            size: SizeKey::Medium,
        });
        d.set_load("J3", 25.0);
        d.set_load("J4", 25.0);
        let text = write(&d, MaterialKey::Steel, LevelKey::Sandbox, "sandbox");
        let net = crate::netlist::parse(&text).unwrap();
        let sol = net.solve_at(net.truck_x()).unwrap();
        let ours = d.analyse(MaterialKey::Steel, 1.0, &[]);
        assert!(sol.stable && ours.ok);
        for (i, m) in ours.members.iter().enumerate() {
            let a = m.unwrap().force;
            let b = sol.members[i].unwrap().force;
            assert!((a - b).abs() < 1.0, "M{}: {a} vs {b}", i + 1);
        }
    }

    #[test]
    fn reads_the_advanced_example_and_refuses_bad_ones() {
        let text = "* river-crossing.str\n.units SI kN m\n.material glulam E=12GPa fy=30MPa rho=500\n.section chord A=100cm2 I=A^2\n.node B0 0 0 fix=xy\n.node B1 8 0\n.node T1 8 6\n.node B2 16 0 fix=y\n.member M1 B0 B1 chord\n.member M2 B1 B2 chord\n.member M3 B0 T1 chord\n.member M4 T1 B2 chord\n.member M5 B1 T1 chord\n.load dead B1 Fy=-98.1\n.load truck 10t x=8 lane=B0..B2 share=0.5\n";
        let (d, m) = read(text).unwrap();
        assert_eq!(m, MaterialKey::Timber);
        assert_eq!(d.nodes.len(), 4);
        assert_eq!(d.nodes[0].id, "J1");
        assert_eq!(d.members[0].size, SizeKey::Medium); // 100 cm² is nearest 80 by ratio
        assert_eq!(d.loads, vec![("J2".to_string(), 15.0)]);
        assert!(matches!(read("hello"), Err(ReadError::Parse { .. })));
        assert!(matches!(
            read(".node A 1 0\n"),
            Err(ReadError::Import(ImportError::OffGrid(_)))
        ));
        assert!(read("").is_err());
    }
}
