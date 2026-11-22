//! The `.str` parser: text → [`Netlist`] or line/column [`Diagnostic`]s. Never panics.
//!
//! Two passes over the directive lines, so order doesn't matter (as in SPICE): first `.units`,
//! then the declarations (`.material`, `.section`, `.node`), then everything that refers to
//! them (`.member`, `.load`, `.plot`, …).

use super::{
    Diagnostic, ForceUnit, LengthUnit, MovingLoad, NetMember, Netlist, Plot, Section, Severity,
    StaticLoad, Units,
};
use crate::truss::{Fix, G, Material, Node};
use std::collections::HashMap;

/// Most nodes a netlist may have (the dense solve and the 49-solve influence line stay fast).
pub const MAX_NODES: usize = 120;
/// Most members a netlist may have.
pub const MAX_MEMBERS: usize = 600;
/// Most points a `.plot` sweep may have.
pub const MAX_PLOT_POINTS: usize = 400;
/// Longest name.
const MAX_ID: usize = 32;

/// A whitespace- (or comma-) separated token.
#[derive(Debug, Clone, Copy)]
struct Tok<'a> {
    s: &'a str,
    /// 1-based character column.
    col: usize,
    /// Length in characters.
    len: usize,
}

/// One directive line.
#[derive(Debug)]
struct Line<'a> {
    no: usize,
    toks: Vec<Tok<'a>>,
    /// Column just past the last token (for "missing argument" errors).
    end: usize,
}

impl<'a> Line<'a> {
    fn dir(&self) -> String {
        self.toks
            .first()
            .map(|t| t.s.to_ascii_lowercase())
            .unwrap_or_default()
    }
    fn args(&self) -> &[Tok<'a>] {
        self.toks.get(1..).unwrap_or(&[])
    }
}

fn diag(line: usize, t: Tok, sev: Severity, msg: impl Into<String>) -> Diagnostic {
    Diagnostic {
        line,
        column: t.col,
        len: t.len.max(1),
        severity: sev,
        message: msg.into(),
    }
}

/// Splits the text into directive lines (comments dropped) and finds the title and the notes
/// (the comment lines before the first directive).
fn lex(text: &str) -> (Vec<Line<'_>>, Option<String>, Vec<String>) {
    let mut lines = Vec::new();
    let mut title = None;
    let mut notes = Vec::new();
    let mut seen_content = false;
    for (i, raw) in text.lines().enumerate() {
        let no = i + 1;
        let trimmed = raw.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(c) = trimmed.strip_prefix('*') {
            let c = c.trim();
            if !seen_content && c.len() > 4 && c.ends_with(".str") && !c.contains(char::is_whitespace) {
                title = Some(c.to_string());
            } else if lines.is_empty() && !c.is_empty() && notes.len() < 16 {
                notes.push(c.to_string());
            }
            seen_content = true;
            continue;
        }
        seen_content = true;
        let code = match raw.find(';') {
            Some(p) => &raw[..p],
            None => raw,
        };
        let mut toks = Vec::new();
        let mut start: Option<(usize, usize)> = None; // (byte, col)
        let mut col = 0usize;
        let mut end = 1usize;
        for (b, ch) in code.char_indices() {
            col += 1;
            let sep = ch.is_whitespace() || ch == ',';
            match (sep, start) {
                (false, None) => start = Some((b, col)),
                (true, Some((sb, sc))) => {
                    let s = &code[sb..b];
                    toks.push(Tok {
                        s,
                        col: sc,
                        len: s.chars().count(),
                    });
                    end = col;
                    start = None;
                }
                _ => {}
            }
        }
        if let Some((sb, sc)) = start {
            let s = &code[sb..];
            toks.push(Tok {
                s,
                col: sc,
                len: s.chars().count(),
            });
            end = col + 1;
        }
        if !toks.is_empty() {
            lines.push(Line { no, toks, end });
        }
    }
    (lines, title, notes)
}

/// A number token: `[+-]digits[.digits][e±digits]` then a unit suffix. Accepts the true minus
/// sign (U+2212) the screens print.
fn number(s: &str) -> Result<(f64, String), String> {
    let s2;
    let s = if s.contains('\u{2212}') {
        s2 = s.replace('\u{2212}', "-");
        s2.as_str()
    } else {
        s
    };
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut nd = i - digits_start;
    if i < b.len() && b[i] == b'.' {
        // not the start of a `..` range
        if b.get(i + 1) == Some(&b'.') {
            return Err(format!("expected a number, got {s}"));
        }
        i += 1;
        let f0 = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        nd += i - f0;
    }
    if nd == 0 {
        return Err(format!("expected a number, got {s}"));
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let e0 = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > e0 {
            i = j;
        }
    }
    let v: f64 = s[..i]
        .parse()
        .map_err(|_| format!("expected a number, got {s}"))?;
    if !v.is_finite() {
        return Err(format!("{s} is too large"));
    }
    Ok((v, s[i..].to_string()))
}

fn is_number(s: &str) -> bool {
    number(s).is_ok()
}

/// Valid names start with a letter or `_` and use letters, digits, `_`, `-`, `'`.
fn check_id(s: &str) -> Result<(), String> {
    let mut chars = s.chars();
    let ok_first = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if !ok_first {
        return Err(format!("a name must start with a letter: {s}"));
    }
    if s.chars().count() > MAX_ID {
        return Err(format!("name longer than {MAX_ID} characters"));
    }
    if let Some(c) = s
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '\'')))
    {
        return Err(format!("a name can't contain {c:?}: {s}"));
    }
    Ok(())
}

/// Representative presets for the known material names (the mockup's `mats()`).
fn preset(name: &str) -> Option<(f64, f64, f64)> {
    Some(match name.to_ascii_lowercase().as_str() {
        "steel" => (200e9, 250e6, 7850.0),
        "alu" | "aluminum" | "aluminium" => (69e9, 240e6, 2700.0),
        "timber" | "glulam" | "wood" => (12e9, 24e6, 500.0),
        "cf" | "carbon" | "carbon-fiber" | "carbon_fiber" | "cfrp" => (70e9, 600e6, 1600.0),
        _ => return None,
    })
}

struct Parser<'a> {
    units: Units,
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
    net: Netlist,
    node_ix: HashMap<&'a str, (usize, usize)>, // id -> (index, line)
    member_ix: HashMap<&'a str, (usize, usize)>,
    section_ix: HashMap<&'a str, (usize, usize)>,
    material_ix: HashMap<&'a str, (usize, usize)>,
    self_line: Option<usize>,
    truck_line: Option<usize>,
    once: HashMap<&'static str, usize>,
}

/// Parses a netlist. `Ok` carries the model (with any warnings in [`Netlist::warnings`]);
/// `Err` carries every error and warning, sorted by line and column.
pub fn parse(text: &str) -> Result<Netlist, Vec<Diagnostic>> {
    let (lines, title, notes) = lex(text);
    let mut p = Parser {
        units: Units::default(),
        errors: Vec::new(),
        warnings: Vec::new(),
        net: Netlist {
            title,
            notes,
            ..Netlist::default()
        },
        node_ix: HashMap::new(),
        member_ix: HashMap::new(),
        section_ix: HashMap::new(),
        material_ix: HashMap::new(),
        self_line: None,
        truck_line: None,
        once: HashMap::new(),
    };
    // pass 0: units
    for l in lines.iter().filter(|l| l.dir() == ".units") {
        p.units_line(l);
    }
    p.net.units = p.units;
    // pass 1: declarations
    for l in &lines {
        let r = match l.dir().as_str() {
            ".material" => p.material(l),
            ".section" => p.section(l),
            ".node" => p.node(l),
            _ => Ok(()),
        };
        if let Err(d) = r {
            p.errors.push(d);
        }
    }
    // pass 2: members (they only refer to declarations)
    for l in lines.iter().filter(|l| l.dir() == ".member") {
        if let Err(d) = p.member(l) {
            p.errors.push(d);
        }
    }
    // pass 3: everything else
    for l in &lines {
        let dir = l.dir();
        let r = match dir.as_str() {
            ".units" | ".material" | ".section" | ".node" | ".member" => Ok(()),
            ".load" => p.load(l),
            ".static" => p.static_line(l),
            ".buckle" => p.buckle(l),
            ".plot" => p.plot(l),
            ".end" => Ok(()),
            ".story" | ".wind" => p.elsewhere(l, "the Tower world"),
            ".soil" | ".foundation" => p.elsewhere(l, "the Ground world"),
            ".dynamic" => p.elsewhere(l, "the Tower and Ground worlds"),
            d if d.starts_with('.') => Err(diag(
                l.no,
                l.toks[0],
                Severity::Error,
                format!(
                    "unknown directive {} (try .node, .member, .material, .section, .load)",
                    l.toks[0].s
                ),
            )),
            _ => Err(diag(
                l.no,
                l.toks[0],
                Severity::Error,
                "expected a directive such as .node or .member (comments start with * or ;)",
            )),
        };
        if let Err(d) = r {
            p.errors.push(d);
        }
    }
    p.finish()
}

impl<'a> Parser<'a> {
    fn finish(mut self) -> Result<Netlist, Vec<Diagnostic>> {
        let key = |d: &Diagnostic| (d.line, d.column);
        self.warnings.sort_by_key(key);
        if self.errors.is_empty() {
            self.net.warnings = self.warnings;
            Ok(self.net)
        } else {
            let mut all = self.errors;
            all.extend(self.warnings);
            all.sort_by_key(key);
            Err(all)
        }
    }

    fn missing(&self, l: &Line, what: &str) -> Diagnostic {
        Diagnostic {
            line: l.no,
            column: l.end,
            len: 1,
            severity: Severity::Error,
            message: format!("{} needs {what}", l.toks[0].s),
        }
    }

    fn err(l: &Line, t: Tok, msg: impl Into<String>) -> Diagnostic {
        diag(l.no, t, Severity::Error, msg)
    }

    fn warn(&mut self, l: &Line, t: Tok, msg: impl Into<String>) {
        self.warnings.push(diag(l.no, t, Severity::Warning, msg));
    }

    /// A directive that should appear once: warns on a repeat (the last one wins).
    fn once(&mut self, l: &Line, key: &'static str) {
        if let Some(first) = self.once.insert(key, l.no) {
            self.warn(
                l,
                l.toks[0],
                format!("{key} is already given on line {first}; this one wins"),
            );
        }
    }

    fn units_line(&mut self, l: &Line) {
        if let Some(first) = self.once.insert(".units", l.no) {
            self.errors.push(Self::err(
                l,
                l.toks[0],
                format!("the units are already set on line {first}"),
            ));
            return;
        }
        for &t in l.args() {
            match t.s.to_ascii_lowercase().as_str() {
                "si" => {}
                "n" => self.units.force = ForceUnit::N,
                "kn" => self.units.force = ForceUnit::KN,
                "mn" => self.units.force = ForceUnit::MN,
                "m" => self.units.length = LengthUnit::M,
                "cm" => self.units.length = LengthUnit::Cm,
                "mm" => self.units.length = LengthUnit::Mm,
                _ => self.errors.push(Self::err(
                    l,
                    t,
                    format!(
                        "unknown unit {} (force: N, kN or MN; length: m, cm or mm)",
                        t.s
                    ),
                )),
            }
        }
    }

    // ---- values with units -------------------------------------------------------------

    fn length(&self, l: &Line, t: Tok) -> Result<f64, Diagnostic> {
        let (v, u) = number(t.s).map_err(|m| Self::err(l, t, m))?;
        let per_m = match u.to_ascii_lowercase().as_str() {
            "" => self.units.length.per_metre(),
            "m" => 1.0,
            "cm" => 100.0,
            "mm" => 1000.0,
            _ => return Err(Self::err(l, t, format!("unknown length unit {u} (m, cm, mm)"))),
        };
        Ok(v / per_m)
    }

    fn stress(&self, l: &Line, t: Tok, v: f64, u: &str) -> Result<f64, Diagnostic> {
        let r = match u.to_ascii_lowercase().as_str() {
            "pa" => v,
            "kpa" => v * 1e3,
            "mpa" => v * 1e6,
            "gpa" => v * 1e9,
            "" => {
                let pm = self.units.length.per_metre();
                v * self.units.force.newtons() * (pm * pm)
            }
            _ => {
                return Err(Self::err(
                    l,
                    t,
                    format!("unknown stress unit {u} (Pa, kPa, MPa, GPa)"),
                ));
            }
        };
        finite(l, t, r)
    }

    fn area(&self, l: &Line, t: Tok, v: f64, u: &str) -> Result<f64, Diagnostic> {
        let r = match u.to_ascii_lowercase().as_str() {
            "m2" | "m²" => v,
            "cm2" | "cm²" => v / 1e4,
            "mm2" | "mm²" => v / 1e6,
            "" => {
                let pm = self.units.length.per_metre();
                v / (pm * pm)
            }
            _ => return Err(Self::err(l, t, format!("unknown area unit {u} (m2, cm2, mm2)"))),
        };
        finite(l, t, r)
    }

    fn inertia(&self, l: &Line, t: Tok, v: f64, u: &str) -> Result<f64, Diagnostic> {
        let r = match u.to_ascii_lowercase().as_str() {
            "m4" | "m⁴" => v,
            "cm4" | "cm⁴" => v / 1e8,
            "mm4" | "mm⁴" => v / 1e12,
            "" => {
                let pm = self.units.length.per_metre();
                v / (pm * pm * pm * pm)
            }
            _ => {
                return Err(Self::err(
                    l,
                    t,
                    format!("unknown inertia unit {u} (m4, cm4, mm4, or A^2)"),
                ));
            }
        };
        finite(l, t, r)
    }

    /// A force token: bare (the force unit), `N`, `kN`, `MN`, or `t` (tonnes of weight,
    /// `t × 9.81 × 1000` N). Returns (newtons, tonnes if written in t).
    fn force(&self, l: &Line, t: Tok, s: &str) -> Result<(f64, Option<f64>), Diagnostic> {
        let (v, u) = number(s).map_err(|m| Self::err(l, t, m))?;
        let r = match u.to_ascii_lowercase().as_str() {
            "" => (v * self.units.force.newtons(), None),
            "n" => (v, None),
            "kn" => (v * 1e3, None),
            "mn" => (v * 1e6, None),
            "t" => (v * G * 1e3, Some(v)),
            _ => {
                return Err(Self::err(
                    l,
                    t,
                    format!("unknown force unit {u} (N, kN, MN, or t for tonnes)"),
                ));
            }
        };
        finite(l, t, r.0)?;
        Ok(r)
    }

    fn new_id(
        l: &Line,
        t: Tok<'a>,
        map: &HashMap<&'a str, (usize, usize)>,
        what: &str,
    ) -> Result<(), Diagnostic> {
        check_id(t.s).map_err(|m| Self::err(l, t, m))?;
        if let Some((_, first)) = map.get(t.s) {
            return Err(Self::err(
                l,
                t,
                format!("duplicate {what} {} (first on line {first})", t.s),
            ));
        }
        Ok(())
    }

    // ---- declarations ------------------------------------------------------------------

    fn material(&mut self, l: &Line<'a>) -> Result<(), Diagnostic> {
        let args = l.args();
        let name = *args.first().ok_or_else(|| self.missing(l, "a name"))?;
        Self::new_id(l, name, &self.material_ix, "material")?;
        let (mut e, mut fy, mut rho) = (None, None, None);
        let mut positional = 0;
        for &t in &args[1..] {
            let (key, val) = match t.s.split_once('=') {
                Some((k, v)) => (k.to_ascii_lowercase(), v),
                None => {
                    positional += 1;
                    let k = match positional {
                        1 => "e",
                        2 => "fy",
                        3 => "rho",
                        _ => return Err(Self::err(l, t, format!("unexpected {}", t.s))),
                    };
                    (k.to_string(), t.s)
                }
            };
            match key.as_str() {
                "e" => {
                    let (v, u) = number(val).map_err(|m| Self::err(l, t, m))?;
                    e = Some(self.stress(l, t, v, &u)?);
                }
                "fy" => {
                    let (v, u) = number(val).map_err(|m| Self::err(l, t, m))?;
                    fy = Some(self.stress(l, t, v, &u)?);
                }
                "rho" => {
                    let (v, u) = number(val).map_err(|m| Self::err(l, t, m))?;
                    let v = match u.to_ascii_lowercase().as_str() {
                        "" | "kg/m3" | "kg/m³" => v,
                        "t/m3" | "t/m³" => v * 1e3,
                        _ => {
                            return Err(Self::err(l, t, format!("unknown density unit {u} (kg/m3)")));
                        }
                    };
                    rho = Some(v);
                }
                "curve" => self.warn(
                    l,
                    t,
                    "curve= is used by the Materials bench; Advanced uses E and fy",
                ),
                _ => {
                    return Err(Self::err(
                        l,
                        t,
                        format!("unknown material property {key} (E, fy, rho)"),
                    ));
                }
            }
        }
        let pre = preset(name.s);
        let (e, fy, rho) = match (e, fy, rho, pre) {
            (Some(e), Some(fy), Some(rho), _) => (e, fy, rho),
            (e, fy, rho, Some((pe, pfy, prho))) => {
                (e.unwrap_or(pe), fy.unwrap_or(pfy), rho.unwrap_or(prho))
            }
            (Some(e), Some(fy), None, None) => {
                self.warn(l, name, "no rho= given: the members weigh nothing");
                (e, fy, 0.0)
            }
            _ => return Err(self.missing(l, "E=… and fy=…")),
        };
        if e.is_nan() || e <= 0.0 {
            return Err(Self::err(l, name, "E must be more than 0"));
        }
        if fy.is_nan() || fy <= 0.0 {
            return Err(Self::err(l, name, "fy must be more than 0"));
        }
        if rho.is_nan() || rho < 0.0 {
            return Err(Self::err(l, name, "rho can't be negative"));
        }
        self.material_ix
            .insert(name.s, (self.net.materials.len(), l.no));
        self.net.materials.push(Material::new(name.s, e, fy, rho));
        Ok(())
    }

    fn section(&mut self, l: &Line<'a>) -> Result<(), Diagnostic> {
        let args = l.args();
        let name = *args.first().ok_or_else(|| self.missing(l, "a name"))?;
        Self::new_id(l, name, &self.section_ix, "section")?;
        let (mut a, mut i_box, mut inertia) = (None, false, None);
        let mut positional = 0;
        for &t in &args[1..] {
            let (key, val) = match t.s.split_once('=') {
                Some((k, v)) => (k.to_ascii_lowercase(), v),
                None => {
                    positional += 1;
                    let k = match positional {
                        1 => "a",
                        2 => "i",
                        _ => return Err(Self::err(l, t, format!("unexpected {}", t.s))),
                    };
                    (k.to_string(), t.s)
                }
            };
            match key.as_str() {
                "a" => {
                    let (v, u) = number(val).map_err(|m| Self::err(l, t, m))?;
                    a = Some(self.area(l, t, v, &u)?);
                }
                "i" => {
                    let lower = val.to_ascii_lowercase();
                    if lower == "a^2" || lower == "a²" || lower == "a*a" {
                        i_box = true;
                    } else {
                        let (v, u) = number(val).map_err(|m| Self::err(l, t, m))?;
                        inertia = Some(self.inertia(l, t, v, &u)?);
                    }
                }
                _ => {
                    return Err(Self::err(
                        l,
                        t,
                        format!("unknown section property {key} (A, I)"),
                    ));
                }
            }
        }
        let Some(a) = a else {
            return Err(self.missing(l, "A=… (area)"));
        };
        if a.is_nan() || a <= 0.0 {
            return Err(Self::err(l, name, "the area must be more than 0"));
        }
        let inertia = match (inertia, i_box) {
            (Some(i), false) => i,
            _ => a * a,
        };
        if !(inertia > 0.0 && inertia.is_finite()) {
            return Err(Self::err(l, name, "I must be more than 0"));
        }
        self.section_ix
            .insert(name.s, (self.net.sections.len(), l.no));
        self.net.sections.push(Section {
            name: name.s.to_string(),
            area: a,
            inertia,
        });
        Ok(())
    }

    fn node(&mut self, l: &Line<'a>) -> Result<(), Diagnostic> {
        let args = l.args();
        let name = *args.first().ok_or_else(|| self.missing(l, "a name, x and y"))?;
        Self::new_id(l, name, &self.node_ix, "node")?;
        let xt = *args.get(1).ok_or_else(|| self.missing(l, "x and y"))?;
        let yt = *args.get(2).ok_or_else(|| self.missing(l, "y"))?;
        let x = self.length(l, xt)?;
        let y = self.length(l, yt)?;
        let mut fix = Fix::FREE;
        for &t in &args[3..] {
            let Some((k, v)) = t.s.split_once('=') else {
                return Err(Self::err(l, t, format!("unexpected {} (fix=xy, fix=y)", t.s)));
            };
            if !k.eq_ignore_ascii_case("fix") {
                return Err(Self::err(l, t, format!("unknown node option {k} (fix=)")));
            }
            fix = match v.to_ascii_lowercase().as_str() {
                "xy" | "yx" | "pin" => Fix::PIN,
                "y" | "roller" => Fix::ROLLER,
                "x" => Fix { x: true, y: false },
                "none" | "free" | "" => Fix::FREE,
                _ => {
                    return Err(Self::err(
                        l,
                        t,
                        format!("unknown support fix={v} (xy = pin, y = roller, x)"),
                    ));
                }
            };
        }
        if self.net.nodes.len() >= MAX_NODES {
            return Err(Self::err(
                l,
                name,
                format!("too many nodes (at most {MAX_NODES})"),
            ));
        }
        self.node_ix.insert(name.s, (self.net.nodes.len(), l.no));
        self.net.nodes.push(Node::new(name.s, x, y, fix));
        Ok(())
    }

    // ---- references --------------------------------------------------------------------

    fn node_ref(&self, l: &Line, t: Tok) -> Result<usize, Diagnostic> {
        self.node_ix
            .get(t.s)
            .map(|(i, _)| *i)
            .ok_or_else(|| Self::err(l, t, format!("unknown node {}", t.s)))
    }

    /// A node name, a `B1..B5` range, in order.
    fn node_list(&self, l: &Line, t: Tok, out: &mut Vec<usize>) -> Result<(), Diagnostic> {
        let Some((a, b)) = t.s.split_once("..") else {
            out.push(self.node_ref(l, t)?);
            return Ok(());
        };
        let split = |s: &str| -> Option<(String, u64)> {
            let digits = s.len() - s.trim_end_matches(|c: char| c.is_ascii_digit()).len();
            if digits == 0 || digits > 9 {
                return None;
            }
            let (p, n) = s.split_at(s.len() - digits);
            Some((p.to_string(), n.parse().ok()?))
        };
        let (Some((pa, na)), Some((pb, nb))) = (split(a), split(b)) else {
            return Err(Self::err(l, t, "a range looks like B1..B5"));
        };
        if pa != pb {
            return Err(Self::err(
                l,
                t,
                format!("a range needs one prefix: {pa}… and {pb}… differ"),
            ));
        }
        let count = na.abs_diff(nb) + 1;
        if count > self.net.nodes.len() as u64 {
            return Err(Self::err(
                l,
                t,
                format!("{} names more nodes than the netlist has", t.s),
            ));
        }
        let ids: Vec<u64> = if na <= nb {
            (na..=nb).collect()
        } else {
            (nb..=na).rev().collect()
        };
        for n in ids {
            let id = format!("{pa}{n}");
            match self.node_ix.get(id.as_str()) {
                Some((i, _)) => out.push(*i),
                None => {
                    return Err(Self::err(
                        l,
                        t,
                        format!("unknown node {id} in the range {}", t.s),
                    ));
                }
            }
        }
        Ok(())
    }

    fn member(&mut self, l: &Line<'a>) -> Result<(), Diagnostic> {
        let args = l.args();
        let id = *args
            .first()
            .ok_or_else(|| self.missing(l, "an id, two nodes and a section"))?;
        Self::new_id(l, id, &self.member_ix, "member")?;
        let at = *args.get(1).ok_or_else(|| self.missing(l, "two nodes and a section"))?;
        let bt = *args.get(2).ok_or_else(|| self.missing(l, "a second node and a section"))?;
        let st = *args.get(3).ok_or_else(|| self.missing(l, "a section"))?;
        let a = self.node_ref(l, at)?;
        let b = self.node_ref(l, bt)?;
        if a == b {
            return Err(Self::err(l, bt, "a member joins two different nodes"));
        }
        let (na, nb) = (&self.net.nodes[a], &self.net.nodes[b]);
        let len = (nb.x - na.x).hypot(nb.y - na.y);
        if !(len > 1e-9 && len.is_finite()) {
            return Err(Self::err(
                l,
                bt,
                format!("{} has no length: {} and {} are at the same place", id.s, at.s, bt.s),
            ));
        }
        let section = self
            .section_ix
            .get(st.s)
            .map(|(i, _)| *i)
            .ok_or_else(|| Self::err(l, st, format!("unknown section {}", st.s)))?;
        let material = match args.get(4) {
            Some(&mt) => self
                .material_ix
                .get(mt.s)
                .map(|(i, _)| *i)
                .ok_or_else(|| Self::err(l, mt, format!("unknown material {}", mt.s)))?,
            None if self.net.materials.is_empty() => {
                return Err(Self::err(
                    l,
                    id,
                    "no material: add a .material line (e.g. .material steel)",
                ));
            }
            None => 0,
        };
        if let Some(&extra) = args.get(5) {
            return Err(Self::err(l, extra, format!("unexpected {}", extra.s)));
        }
        if self.net.members.len() >= MAX_MEMBERS {
            return Err(Self::err(
                l,
                id,
                format!("too many members (at most {MAX_MEMBERS})"),
            ));
        }
        self.member_ix.insert(id.s, (self.net.members.len(), l.no));
        self.net.members.push(NetMember {
            id: id.s.to_string(),
            a,
            b,
            section,
            material,
        });
        Ok(())
    }

    fn load(&mut self, l: &Line<'a>) -> Result<(), Diagnostic> {
        let args = l.args();
        let case = *args
            .first()
            .ok_or_else(|| self.missing(l, "a case name, nodes and a force"))?;
        check_id(case.s).map_err(|m| Self::err(l, case, m))?;
        let rest = &args[1..];
        let moving = rest.iter().any(|t| {
            t.s.split_once('=')
                .is_some_and(|(k, _)| k.eq_ignore_ascii_case("lane"))
        });
        if moving {
            self.moving(l, case, rest)
        } else {
            self.static_load(l, case, rest)
        }
    }

    fn moving(&mut self, l: &Line<'a>, case: Tok, rest: &[Tok<'a>]) -> Result<(), Diagnostic> {
        if let Some(first) = self.truck_line {
            return Err(Self::err(
                l,
                case,
                format!("only one moving load (the first is on line {first})"),
            ));
        }
        let (mut weight, mut x, mut lane, mut share) = (None, None, Vec::new(), 1.0);
        let mut lane_tok = None;
        for &t in rest {
            match t.s.split_once('=') {
                None => {
                    if weight.is_some() {
                        return Err(Self::err(l, t, format!("unexpected {}", t.s)));
                    }
                    weight = Some(self.force(l, t, t.s)?);
                }
                Some((k, v)) => match k.to_ascii_lowercase().as_str() {
                    "x" => {
                        let vt = Tok { s: v, ..t };
                        x = Some(self.length(l, vt)?);
                    }
                    "lane" => {
                        lane.clear();
                        let vt = Tok { s: v, ..t };
                        self.node_list(l, vt, &mut lane)?;
                        lane_tok = Some(t);
                    }
                    "share" => {
                        let (s, u) = number(v).map_err(|m| Self::err(l, t, m))?;
                        if !u.is_empty() || !(s > 0.0 && s <= 1.0) {
                            return Err(Self::err(l, t, "share is a fraction: 0 < share ≤ 1"));
                        }
                        share = s;
                    }
                    _ => {
                        return Err(Self::err(
                            l,
                            t,
                            format!("unknown moving-load option {k} (x=, lane=, share=)"),
                        ));
                    }
                },
            }
        }
        let Some((weight, tonnes)) = weight else {
            return Err(self.missing(l, "a weight (40t)"));
        };
        if weight < 0.0 {
            return Err(Self::err(l, case, "a moving load's weight can't be negative"));
        }
        if lane.len() < 2 {
            let t = lane_tok.unwrap_or(case);
            return Err(Self::err(l, t, "a lane needs at least two nodes (lane=B0..B6)"));
        }
        let x = x.unwrap_or_else(|| {
            lane.iter()
                .map(|&n| self.net.nodes[n].x)
                .fold(f64::INFINITY, f64::min)
        });
        self.truck_line = Some(l.no);
        self.net.truck = Some(MovingLoad {
            case: case.s.to_string(),
            weight,
            tonnes,
            x,
            lane,
            share,
        });
        Ok(())
    }

    fn static_load(&mut self, l: &Line<'a>, case: Tok, rest: &[Tok<'a>]) -> Result<(), Diagnostic> {
        let mut nodes = Vec::new();
        let (mut fx, mut fy) = (None, None);
        let mut positional: Vec<f64> = Vec::new();
        let mut self_w = None;
        for &t in rest {
            if t.s.eq_ignore_ascii_case("+self") {
                if self_w.is_some() {
                    return Err(Self::err(l, t, "+self is already on this line"));
                }
                self_w = Some(t);
                continue;
            }
            if let Some((k, v)) = t.s.split_once('=') {
                let f = self.force(l, t, v)?.0;
                match k.to_ascii_lowercase().as_str() {
                    "fx" => fx = Some(f),
                    "fy" => fy = Some(f),
                    _ => return Err(Self::err(l, t, format!("unknown load option {k} (Fx=, Fy=)"))),
                }
                continue;
            }
            if is_number(t.s) {
                if positional.len() == 2 {
                    return Err(Self::err(l, t, format!("unexpected {} (Fx Fy given)", t.s)));
                }
                positional.push(self.force(l, t, t.s)?.0);
                continue;
            }
            if fx.is_some() || fy.is_some() || !positional.is_empty() {
                return Err(Self::err(l, t, "list the nodes before the forces"));
            }
            self.node_list(l, t, &mut nodes)?;
        }
        if positional.len() == 1 {
            return Err(self.missing(l, "both Fx and Fy (or Fy=…)"));
        }
        if positional.len() == 2 {
            if fx.is_some() || fy.is_some() {
                return Err(self.missing(l, "either Fx Fy or Fx=/Fy=, not both"));
            }
            fx = Some(positional[0]);
            fy = Some(positional[1]);
        }
        let has_force = fx.is_some() || fy.is_some();
        if nodes.is_empty() && has_force {
            return Err(self.missing(l, "the nodes the force acts on"));
        }
        if nodes.is_empty() && self_w.is_none() {
            return Err(self.missing(l, "nodes and a force (B1..B5 Fy=-80), or +self"));
        }
        if !nodes.is_empty() && !has_force && self_w.is_none() {
            return Err(self.missing(l, "a force (Fx Fy, or Fy=…)"));
        }
        if let Some(t) = self_w {
            if let Some(first) = self.self_line {
                self.warn(
                    l,
                    t,
                    format!("the self-weight is already added on line {first}; now it counts twice"),
                );
            } else {
                self.self_line = Some(l.no);
            }
        }
        self.net.loads.push(StaticLoad {
            case: case.s.to_string(),
            nodes,
            fx: fx.unwrap_or(0.0),
            fy: fy.unwrap_or(0.0),
            self_weight: self_w.is_some(),
        });
        Ok(())
    }

    fn static_line(&mut self, l: &Line) -> Result<(), Diagnostic> {
        self.once(l, ".static");
        for &t in l.args() {
            match t.s.to_ascii_lowercase().as_str() {
                "linear" => {}
                "nonlinear" => self.warn(
                    l,
                    t,
                    "nonlinear statics aren't available yet: solving linear",
                ),
                _ => {
                    return Err(Self::err(
                        l,
                        t,
                        format!("unknown analysis {} (linear)", t.s),
                    ));
                }
            }
        }
        Ok(())
    }

    fn buckle(&mut self, l: &Line) -> Result<(), Diagnostic> {
        self.once(l, ".buckle");
        let args = l.args();
        let method = *args.first().ok_or_else(|| self.missing(l, "a method (euler K=1.0)"))?;
        match method.s.to_ascii_lowercase().as_str() {
            "euler" => {
                let mut k = 1.0;
                for &t in &args[1..] {
                    let (key, v) = t.s.split_once('=').unwrap_or(("k", t.s));
                    if !key.eq_ignore_ascii_case("k") {
                        return Err(Self::err(l, t, format!("unknown option {key} (K=)")));
                    }
                    let (val, u) = number(v).map_err(|m| Self::err(l, t, m))?;
                    if !u.is_empty() || !(val > 0.0 && val <= 10.0) {
                        return Err(Self::err(l, t, "K is a factor between 0 and 10"));
                    }
                    k = val;
                }
                self.net.k_factor = k;
            }
            "eigen" => {
                self.warn(
                    l,
                    method,
                    "eigen buckling isn't available yet: using Euler, K = 1",
                );
                self.net.k_factor = 1.0;
            }
            _ => {
                return Err(Self::err(
                    l,
                    method,
                    format!("unknown buckling method {} (euler)", method.s),
                ));
            }
        }
        Ok(())
    }

    fn plot(&mut self, l: &Line) -> Result<(), Diagnostic> {
        self.once(l, ".plot");
        let args = l.args();
        let expr = *args
            .first()
            .ok_or_else(|| self.missing(l, "what to plot: N(M*) vs truck.x"))?;
        let inner = expr
            .s
            .strip_prefix("N(")
            .or_else(|| expr.s.strip_prefix("n("))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| Self::err(l, expr, "only member forces can be plotted: N(M8) or N(M*)"))?;
        let member = if inner.ends_with('*') {
            None
        } else if self.member_ix.contains_key(inner) {
            Some(inner.to_string())
        } else {
            return Err(Self::err(l, expr, format!("unknown member {inner}")));
        };
        match args.get(1) {
            Some(t) if t.s.eq_ignore_ascii_case("vs") => {}
            Some(&t) => return Err(Self::err(l, t, format!("expected vs, got {}", t.s))),
            None => return Err(self.missing(l, "vs truck.x")),
        }
        let var = *args.get(2).ok_or_else(|| self.missing(l, "truck.x"))?;
        let is_truck_x = var
            .s
            .rsplit_once('.')
            .is_some_and(|(_, x)| x.eq_ignore_ascii_case("x"));
        if !is_truck_x {
            return Err(Self::err(l, var, "only the moving load's x can be swept: truck.x"));
        }
        let nums = &args[3..];
        let range = match nums.len() {
            0 => None,
            2 | 3 => {
                let from = self.length(l, nums[0])?;
                let to = self.length(l, nums[1])?;
                let step = match nums.get(2) {
                    Some(&t) => self.length(l, t)?,
                    None => 1.0,
                };
                if to.is_nan() || from.is_nan() || to <= from {
                    return Err(Self::err(l, nums[1], "the range must go up: from < to"));
                }
                if step.is_nan() || step <= 0.0 {
                    let t = nums.get(2).copied().unwrap_or(nums[1]);
                    return Err(Self::err(l, t, "the step must be more than 0"));
                }
                let points = ((to - from) / step).floor() + 1.0;
                if points > MAX_PLOT_POINTS as f64 {
                    return Err(Self::err(
                        l,
                        nums[0],
                        format!("too many points (at most {MAX_PLOT_POINTS})"),
                    ));
                }
                Some((from, to, step))
            }
            _ => {
                let t = nums.get(3).copied().unwrap_or(nums[0]);
                return Err(Self::err(l, t, "give the range as: from to step"));
            }
        };
        self.net.plot = Some(Plot { member, range });
        Ok(())
    }

    fn elsewhere(&mut self, l: &Line, world: &str) -> Result<(), Diagnostic> {
        let msg = format!("{} is used by {world}; Advanced ignores it", l.toks[0].s);
        self.warn(l, l.toks[0], msg);
        Ok(())
    }
}

fn finite(l: &Line, t: Tok, v: f64) -> Result<f64, Diagnostic> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(Parser::err(l, t, format!("{} is out of range", t.s)))
    }
}
