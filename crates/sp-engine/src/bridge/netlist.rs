//! The Bridge's structure as a `.str` netlist (Share, "Open in Advanced": NAV-4, NAV-6,
//! ADV-9), in the Advanced workbench's grammar (SPEC ADV-3): `.units SI kN m`, `.material`,
//! one `.section` per size used (`A=..cm2 I=A^2`), `.node` with `fix=xy` / `fix=y`, `.member`
//! with its size, one `.load` line per hung weight (kN) with `+self`, `.static`,
//! `.buckle euler K=1.0`.
//!
//! Reading a netlist back is the Advanced parser's job (`sp_engine::netlist`); the Bridge
//! turns its truss into a design with [`super::Design::from_truss`].

use super::data::{LevelKey, MaterialKey, N_PER_TONNE, SizeKey, Support};
use super::design::Design;
use super::fmt::num;

/// Formats a number for the netlist: shortest exact form, at most 6 decimals.
fn n(v: f64) -> String {
    let r = (v * 1e6).round() / 1e6;
    num(r)
}

/// Writes the design as a netlist named `name` (the file stem, e.g. `river-level`).
pub fn write(design: &Design, mat: MaterialKey, level: LevelKey, name: &str) -> String {
    let spec = mat.spec();
    let mut out = String::new();
    let line = |out: &mut String, s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    line(&mut out, format!("* {name}.str"));
    line(
        &mut out,
        format!(
            "* Bridge · {} · {} joints, {} members",
            level.level().name,
            design.nodes.len(),
            design.members.len()
        ),
    );
    line(&mut out, ".units  SI  kN  m".into());
    line(
        &mut out,
        format!(
            ".material  {}  E={}GPa  fy={}MPa  rho={}  ; representative",
            spec.name.replace(' ', "-"),
            n(spec.e / 1e9),
            n(spec.fy / 1e6),
            n(spec.rho)
        ),
    );
    for s in SizeKey::ALL {
        if design.members.iter().any(|m| m.size == s) {
            line(
                &mut out,
                format!(
                    ".section  {}  A={}cm2  I=A^2  ; box, b/t = 24",
                    s.name(),
                    n(s.area() * 1e4)
                ),
            );
        }
    }
    for j in &design.nodes {
        let fix = match j.sup {
            Some(Support::Pin) => "  fix=xy  ; pin",
            Some(Support::Roller) => "  fix=y  ; roller",
            None => "",
        };
        line(&mut out, format!(".node  {}  {}  {}{fix}", j.id, j.x, j.y));
    }
    for m in &design.members {
        line(
            &mut out,
            format!(".member  {}  {}  {}  {}", m.id, m.a, m.b, m.size.name()),
        );
    }
    let mut first = true;
    for (id, t) in &design.loads {
        let self_w = if first { "  +self" } else { "" };
        first = false;
        line(
            &mut out,
            format!(
                ".load  bridge  {}  Fy={}{self_w}  ; {} t",
                id,
                n(-t * N_PER_TONNE / 1000.0),
                num(*t)
            ),
        );
    }
    if design.loads.is_empty() {
        line(&mut out, ".load  bridge  +self  ; self-weight only".into());
    }
    line(&mut out, ".static  ; linear, small displacement".into());
    line(&mut out, ".buckle  euler  K=1.0".into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_starter_truss() {
        let d = Design::starter();
        let t = write(&d, MaterialKey::Steel, LevelKey::Sandbox, "sandbox");
        assert!(t.starts_with("* sandbox.str\n"));
        assert!(t.contains(".material  Steel  E=200GPa  fy=250MPa  rho=7850"));
        assert!(t.contains(".section  Medium  A=80cm2  I=A^2"));
        assert!(!t.contains(".section  Thin"));
        assert!(t.contains(".node  J1  8  0  fix=xy"));
        assert!(t.contains(".node  J6  48  0  fix=y"));
        assert!(t.contains(".member  M16  J10  J4  Medium"));
        assert_eq!(t.lines().filter(|l| l.starts_with(".node")).count(), 10);
    }

    #[test]
    fn writes_level_loads_in_kn() {
        let d = Design::for_level(LevelKey::River);
        let t = write(&d, MaterialKey::CarbonFiber, LevelKey::River, "river-level");
        assert!(t.contains(".material  Carbon-fiber  E=70GPa  fy=600MPa  rho=1600"));
        assert!(
            t.contains(".load  bridge  J3  Fy=-196.2  +self  ; 20 t"),
            "{t}"
        );
    }
}
