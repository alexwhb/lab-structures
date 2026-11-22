//! Number formatting that matches what the mockups print (JavaScript's `toFixed`, `Math.round`
//! and `Number → String`), plus the netlist writer's tidy numbers.
//!
//! Every number on the Advanced screen goes through these, so the screen shows exactly the
//! mockup's text for the same value: `toFixed` rounds a tie up (Rust's `{:.N}` rounds a tie to
//! even), and the mockup's `fmt()` writes negatives with a true minus sign (`−1004`).

/// JavaScript's `Math.round`: the nearest integer, ties toward +∞ (`2.5 → 3`, `−2.5 → −2`).
pub fn js_round(v: f64) -> f64 {
    if !v.is_finite() {
        return v;
    }
    let f = v.floor();
    if v - f >= 0.5 { f + 1.0 } else { f }
}

/// JavaScript's `Number.prototype.toFixed(digits)`: `digits` decimals, a tie rounded up (away
/// from zero), negatives with an ASCII `-` (also `-0` for a small negative), `NaN` /
/// `Infinity` spelled as JavaScript does.
pub fn to_fixed(v: f64, digits: usize) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if v.abs() >= 1e21 {
        return js_number(v);
    }
    let digits = digits.min(100);
    let neg = v < 0.0;
    let a = v.abs();
    let mut s = format!("{a:.digits$}");
    // A tie at this precision: the exact decimal expansion is "...5" with nothing after it.
    let wide = format!("{a:.prec$}", prec = digits + 40);
    if let Some(dot) = wide.find('.') {
        let tail = &wide[dot + 1 + digits..];
        if tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0') {
            s = format!("{:.digits$}", a.next_up());
        }
    }
    if neg { format!("-{s}") } else { s }
}

/// The mockups' `fmt(v, d)`: `toFixed` of the magnitude with a true minus sign (U+2212) for a
/// negative value (`−1004`, `−0` for −0.3).
pub fn minus_fixed(v: f64, digits: usize) -> String {
    let s = to_fixed(v.abs(), digits);
    if v < 0.0 { format!("\u{2212}{s}") } else { s }
}

/// JavaScript's `String(number)` for the values the screens print: the shortest text that reads
/// back as the same number (`24`, `24.5`, `0.5`), `0` for −0, exponent form outside
/// `1e-6 ≤ |v| < 1e21` as JavaScript does.
pub fn js_number(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if v == 0.0 {
        return "0".into();
    }
    let a = v.abs();
    if (1e-6..1e21).contains(&a) {
        return format!("{v}");
    }
    // Exponent form: shortest digits, "1e+21" / "1.5e-7".
    let e = format!("{v:e}");
    match e.split_once('e') {
        Some((m, x)) if !x.starts_with('-') => format!("{m}e+{x}"),
        _ => e,
    }
}

/// A tidy number for the netlist writer: at most 12 significant digits, shortest form, no
/// trailing zeros (`80` rather than `80.00000000000001`).
pub fn tidy(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let r: f64 = format!("{v:.11e}").parse().unwrap_or(v);
    js_number(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_matches_javascript() {
        assert_eq!(to_fixed(1004.4, 0), "1004");
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(0.5, 0), "1");
        assert_eq!(to_fixed(1.5, 0), "2");
        assert_eq!(to_fixed(0.25, 1), "0.3");
        assert_eq!(to_fixed(0.125, 2), "0.13");
        assert_eq!(to_fixed(1.005, 2), "1.00"); // 1.005 is just below in binary
        assert_eq!(to_fixed(53.123, 1), "53.1");
        assert_eq!(to_fixed(-0.3, 0), "-0");
        assert_eq!(to_fixed(-0.0, 0), "0");
        assert_eq!(to_fixed(-2.5, 0), "-3");
        assert_eq!(to_fixed(75.00000000000001, 0), "75");
        assert_eq!(to_fixed(f64::NAN, 1), "NaN");
    }

    #[test]
    fn minus_and_round() {
        assert_eq!(minus_fixed(-1004.2, 0), "\u{2212}1004");
        assert_eq!(minus_fixed(805.0, 0), "805");
        assert_eq!(minus_fixed(-0.2, 0), "\u{2212}0");
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(js_round(902.6), 903.0);
    }

    #[test]
    fn numbers_print_like_javascript() {
        assert_eq!(js_number(24.0), "24");
        assert_eq!(js_number(24.5), "24.5");
        assert_eq!(js_number(-0.0), "0");
        assert_eq!(js_number(0.5), "0.5");
        assert_eq!(js_number(200e9 / 1e9), "200");
        assert_eq!(js_number(1e21), "1e+21");
        assert_eq!(js_number(1.5e-7), "1.5e-7");
        assert_eq!(tidy(0.008 * 1e4), "80");
        assert_eq!(tidy(196.20000000000002), "196.2");
    }
}
