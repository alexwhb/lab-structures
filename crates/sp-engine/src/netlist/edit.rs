//! In-place edits of netlist text (the toolbar's Truck and x sliders change the moving-load
//! line without touching anything else the user wrote).

use crate::jsnum::js_number;

/// Rewrites the moving-load line (the first `.load` with `lane=`): its weight as `<tonnes>t`
/// and/or its `x=` (inserted after the weight if missing). `x` is in metres and is written as
/// a bare number in the netlist's `.units` length unit (600 for 6 m under `.units SI kN cm`).
/// Everything else — spacing, comments, other lines, line endings — is kept. `None` when there
/// is no moving-load line.
///
/// For the generated netlist this gives exactly the text the generator would write for the
/// new truck and position.
pub fn set_truck(text: &str, tonnes: Option<f64>, x: Option<f64>) -> Option<String> {
    let per_metre = length_unit(text).per_metre();
    let x = x.map(|x| x * per_metre);
    let mut out = String::with_capacity(text.len() + 8);
    let mut done = false;
    for raw in text.split_inclusive('\n') {
        if done {
            out.push_str(raw);
            continue;
        }
        match patch_line(raw, tonnes, x) {
            Some(p) => {
                out.push_str(&p);
                done = true;
            }
            None => out.push_str(raw),
        }
    }
    done.then_some(out)
}

/// The length unit the text's `.units` line sets (metres without one).
fn length_unit(text: &str) -> super::LengthUnit {
    use super::LengthUnit;
    let mut unit = LengthUnit::M;
    for raw in text.lines() {
        let code = raw.split(';').next().unwrap_or("");
        let mut toks = code.split_whitespace();
        if !toks
            .next()
            .is_some_and(|d| d.eq_ignore_ascii_case(".units"))
        {
            continue;
        }
        for t in toks {
            match t.to_ascii_lowercase().as_str() {
                "m" => unit = LengthUnit::M,
                "cm" => unit = LengthUnit::Cm,
                "mm" => unit = LengthUnit::Mm,
                _ => {}
            }
        }
        break;
    }
    unit
}

/// Byte spans of the whitespace-separated tokens of `code`.
fn spans(code: &str) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    let mut start = None;
    for (i, c) in code.char_indices() {
        let sep = c.is_whitespace();
        match (sep, start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                v.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        v.push((s, code.len()));
    }
    v
}

fn patch_line(raw: &str, tonnes: Option<f64>, x: Option<f64>) -> Option<String> {
    if raw.trim_start().starts_with('*') {
        return None;
    }
    let code_end = raw
        .find(';')
        .unwrap_or_else(|| raw.trim_end_matches(['\n', '\r']).len());
    let code = &raw[..code_end];
    let toks = spans(code);
    let first = *toks.first()?;
    if !code[first.0..first.1].eq_ignore_ascii_case(".load") {
        return None;
    }
    let key = |(s, e): (usize, usize)| {
        code[s..e]
            .split_once('=')
            .map(|(k, _)| k.to_ascii_lowercase())
    };
    if !toks.iter().any(|&t| key(t).as_deref() == Some("lane")) {
        return None;
    }
    // (span, replacement) edits, applied right to left
    let mut edits: Vec<((usize, usize), String)> = Vec::new();
    let weight = toks.iter().skip(2).copied().find(|&t| key(t).is_none());
    if let Some(t) = tonnes {
        let w = format!("{}t", js_number(t));
        match weight {
            Some(sp) => edits.push((sp, w)),
            None => {
                let after = toks.get(1).map(|t| t.1).unwrap_or(first.1);
                edits.push(((after, after), format!("  {w}")));
            }
        }
    }
    if let Some(x) = x {
        let xs = js_number(x);
        match toks
            .iter()
            .copied()
            .find(|&t| key(t).as_deref() == Some("x"))
        {
            Some((s, e)) => {
                let eq = code[s..e].find('=').map(|p| s + p + 1).unwrap_or(e);
                edits.push(((eq, e), xs));
            }
            None => {
                let after = weight
                    .or(toks.get(1).copied())
                    .map(|t| t.1)
                    .unwrap_or(first.1);
                edits.push(((after, after), format!("  x={xs}")));
            }
        }
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0.0));
    let mut s = raw.to_string();
    for ((a, b), r) in edits {
        s.replace_range(a..b, &r);
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_weight_and_x_only() {
        let t = "* a\n.load truck  40t  x=24  lane=B0..B6  share=0.5  ; per truss\n.static\n";
        let p = set_truck(t, Some(55.0), Some(24.5)).unwrap();
        assert_eq!(
            p,
            "* a\n.load truck  55t  x=24.5  lane=B0..B6  share=0.5  ; per truss\n.static\n"
        );
        let p = set_truck(t, None, Some(0.0)).unwrap();
        assert!(p.contains("40t  x=0  lane"));
        assert_eq!(set_truck("* none\n.static\n", Some(1.0), None), None);
        // x missing: inserted after the weight
        let p = set_truck(".load truck 40t lane=B0..B6\r\n", None, Some(3.0)).unwrap();
        assert_eq!(p, ".load truck 40t  x=3 lane=B0..B6\r\n");
        // a comment that mentions lane= is not a moving load
        assert_eq!(
            set_truck(".load dead B1 Fy=-1 ; lane=B0..B6\n", None, Some(1.0)),
            None
        );
    }
}
