//! JavaScript-exact number formatting, shared by every module that reproduces a mockup's text
//! (`docs/design/mockup/*.dc.html` print numbers with `toFixed`, `Math.round` and `String(n)`).
//!
//! Rust's `format!("{:.1}", x)` rounds an exact tie to even (`0.25` → `"0.2"`); JavaScript's
//! `toFixed` rounds it up (`"0.3"`). `Math.round(-2.5)` is −2 in JavaScript, −3 with Rust's
//! `f64::round`. `String(-0)` is `"0"`, Rust prints `"-0"`. These functions follow the
//! ECMAScript definitions; the tests check them against Node when it is installed.
//!
//! | JavaScript | here | aliases (the modules' old names) |
//! |---|---|---|
//! | `x.toFixed(d)` | [`to_fixed`] | [`fixed`] |
//! | `Math.round(x)` | [`round`] | [`js_round`] |
//! | `String(Math.round(x))` | [`round_str`] | |
//! | `String(x)`, `'' + x` | [`to_string`] | [`num`], [`js_number`] |
//! | the mockups' `fmt(v, d)` (true minus sign) | [`minus_fixed`] | |
//! | — (netlist writer: ≤ 12 significant digits) | [`tidy`] | |
//!
//! Owner: the `materials` agent. Formatting that is not about numbers (the Bridge's star row)
//! stays with its module.

/// `x.toFixed(digits)` (ECMAScript `Number.prototype.toFixed`): the decimal with `digits`
/// fraction digits nearest to the exact binary value of `x`, an exact tie going to the larger
/// magnitude; the sign kept for a negative value that rounds to zero (`(-0.04).toFixed(1)` is
/// `"-0.0"`), but `-0` prints unsigned; `NaN`, `Infinity`, `-Infinity`; magnitudes ≥ 1e21 as
/// [`to_string`]. `digits` is capped at 100 (JavaScript's limit).
pub fn to_fixed(x: f64, digits: usize) -> String {
    if !x.is_finite() || x.abs() >= 1e21 {
        return to_string(x);
    }
    let digits = digits.min(100);
    // The exact decimal expansion (Rust's precision formatting is exact) far enough past the
    // kept digits that the digit right after them is never disturbed by Rust's own rounding:
    // that digit alone decides (≥ 5 is a tie or above, which JavaScript rounds up either way).
    let exact = format!("{:.*}", digits + 80, x.abs());
    let Some((int, frac)) = exact.split_once('.') else {
        return exact;
    };
    let mut ds: Vec<u8> = int.bytes().chain(frac.bytes().take(digits)).collect();
    let mut int_len = int.len();
    if frac.as_bytes().get(digits).is_some_and(|&d| d >= b'5') {
        let mut i = ds.len();
        loop {
            if i == 0 {
                ds.insert(0, b'1');
                int_len += 1;
                break;
            }
            i -= 1;
            if ds[i] == b'9' {
                ds[i] = b'0';
            } else {
                ds[i] += 1;
                break;
            }
        }
    }
    let mut out = String::with_capacity(ds.len() + 2);
    if x < 0.0 {
        out.push('-');
    }
    for (i, d) in ds.iter().enumerate() {
        if i == int_len {
            out.push('.');
        }
        out.push(*d as char);
    }
    out
}

/// [`to_fixed`] under `tower::fmt`'s name.
pub fn fixed(x: f64, digits: usize) -> String {
    to_fixed(x, digits)
}

/// `Math.round(x)`: the nearest integer, a tie going towards +∞ (`2.5` → 3, `-2.5` → −2).
/// Non-finite values pass through.
pub fn round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// [`round`] under the `bridge::fmt` / `netlist::text` / `tower::fmt` name.
pub fn js_round(x: f64) -> f64 {
    round(x)
}

/// `String(Math.round(x))` (`"259"`, `"0"` for −0.2, `"NaN"`, `"Infinity"`).
pub fn round_str(x: f64) -> String {
    to_string(round(x))
}

/// `String(x)`: the shortest digits that read back as `x` (Rust's `Display` gives the same
/// digits), `-0` as `"0"`, `NaN`, `Infinity`, and JavaScript's exponent form outside
/// 1e-6 ≤ |x| < 1e21 (`"1e+21"`, `"1.5e-7"`).
pub fn to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if (1e-6..1e21).contains(&x.abs()) {
        return format!("{x}");
    }
    let s = format!("{x:e}"); // "1e21", "1.5e-7"
    match s.split_once('e') {
        Some((m, e)) if !e.starts_with('-') => format!("{m}e+{e}"),
        _ => s,
    }
}

/// [`to_string`] under `bridge::fmt` / `tower::fmt`'s name.
pub fn num(x: f64) -> String {
    to_string(x)
}

/// [`to_string`] under `netlist::text`'s name.
pub fn js_number(x: f64) -> String {
    to_string(x)
}

/// The mockups' `fmt(v, d)`: `toFixed` of the magnitude with a true minus sign (U+2212) for a
/// negative value (`−1004`; `−0` for −0.2).
pub fn minus_fixed(v: f64, digits: usize) -> String {
    let s = to_fixed(v.abs(), digits);
    if v < 0.0 { format!("\u{2212}{s}") } else { s }
}

/// A tidy number for netlists: at most 12 significant digits in the shortest form, no float
/// noise (`80` rather than `80.00000000000001`); non-finite values write `0`.
pub fn tidy(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let r: f64 = format!("{v:.11e}").parse().unwrap_or(v);
    to_string(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The union of the cases the three earlier formatters (`bridge::fmt`, `netlist::text`,
    /// `tower::fmt`) and `materials::jsnum` tested, all checked against Node.
    #[test]
    fn to_fixed_matches_javascript() {
        let cases: &[(f64, usize, &str)] = &[
            (0.0, 1, "0.0"),
            (0.0, 2, "0.00"),
            (-0.0, 0, "0"),
            (-0.0, 1, "0.0"),
            (0.75, 1, "0.8"),
            (0.25, 1, "0.3"),
            (0.5, 0, "1"),
            (1.5, 0, "2"),
            (2.5, 0, "3"),
            (-2.5, 0, "-3"),
            (-1.25, 1, "-1.3"),
            (-0.04, 1, "-0.0"),
            (-0.3, 0, "-0"),
            (-0.001, 2, "-0.00"),
            (0.125, 2, "0.13"),
            (1.005, 2, "1.00"),
            (1.045, 2, "1.04"),
            (9.95, 1, "9.9"),
            (9.96, 1, "10.0"),
            (9.995, 2, "9.99"),
            (9.9951, 2, "10.00"),
            (99.5, 0, "100"),
            (99.96, 1, "100.0"),
            (13.75, 1, "13.8"),
            (53.123, 1, "53.1"),
            (927.83, 0, "928"),
            (1004.4, 0, "1004"),
            (0.10073, 3, "0.101"),
            (75.00000000000001, 0, "75"),
            (83.7234375, 2, "83.72"),
            (54.777_070_063_694_27, 0, "55"),
            (1234.5678, 3, "1234.568"),
            (123456.789, 1, "123456.8"),
            (123456789.987, 2, "123456789.99"),
            (1e-12, 2, "0.00"),
            (0.08 + 0.08 + 0.08, 2, "0.24"),
        ];
        for &(x, d, want) in cases {
            assert_eq!(to_fixed(x, d), want, "({x}).toFixed({d})");
            assert_eq!(fixed(x, d), want);
        }
        assert_eq!(to_fixed(f64::NAN, 1), "NaN");
        assert_eq!(to_fixed(f64::INFINITY, 0), "Infinity");
        assert_eq!(to_fixed(f64::NEG_INFINITY, 2), "-Infinity");
        assert_eq!(to_fixed(1e21, 2), "1e+21");
        assert_eq!(to_fixed(0.5, 500).len(), 102, "digits capped at 100");
    }

    #[test]
    fn round_matches_javascript() {
        for (x, want) in [
            (2.5, 3.0),
            (-2.5, -2.0),
            (0.49999999999999994, 0.0),
            (62.5, 63.0),
            (73.5, 74.0),
            (258.7, 259.0),
            (902.6, 903.0),
            (2.4, 2.0),
        ] {
            assert_eq!(round(x), want, "Math.round({x})");
            assert_eq!(js_round(x), want);
        }
        assert!(round(f64::NAN).is_nan());
        assert_eq!(round_str(-0.2), "0");
        assert_eq!(round_str(258.7), "259");
        assert_eq!(round_str(f64::INFINITY), "Infinity");
    }

    #[test]
    fn to_string_matches_javascript() {
        for (x, want) in [
            (22.0, "22"),
            (20.0, "20"),
            (24.5, "24.5"),
            (0.86, "0.86"),
            (0.5, "0.5"),
            (-40.0, "-40"),
            (-12.0, "-12"),
            (-0.0, "0"),
            (200e9 / 1e9, "200"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e15, "1000000000000000"),
            (1e21, "1e+21"),
            (1.5e-7, "1.5e-7"),
        ] {
            assert_eq!(to_string(x), want);
            assert_eq!(num(x), want);
            assert_eq!(js_number(x), want);
        }
    }

    #[test]
    fn minus_and_tidy() {
        assert_eq!(minus_fixed(-1004.2, 0), "\u{2212}1004");
        assert_eq!(minus_fixed(805.0, 0), "805");
        assert_eq!(minus_fixed(-0.2, 0), "\u{2212}0");
        assert_eq!(tidy(0.008 * 1e4), "80");
        assert_eq!(tidy(196.20000000000002), "196.2");
        assert_eq!(tidy(f64::NAN), "0");
    }

    /// A few hundred values (ties, near-ties, carries, negatives, big and tiny magnitudes)
    /// against Node's own `toFixed`, `Math.round` and `String` (skipped without Node).
    #[test]
    fn agrees_with_node() {
        let mut xs: Vec<f64> = vec![0.0, -0.0, 1e-7, -1e-7, 5e-324, 1e20, -1e20, 1e21];
        for i in 0..400 {
            let k = i as f64;
            xs.push(k / 8.0 - 20.0); // exact binary ties
            xs.push(k * 0.05); // decimal near-ties
            xs.push(-k * 0.125 + 3.3);
            xs.push((k * 7.123_456_789).powf(1.7) / 1000.0);
            xs.push(k * 0.005 + 0.0049999999999999);
        }
        let list: Vec<String> = xs.iter().map(|x| format!("{x:e}")).collect();
        let script = format!(
            "const xs=[{}];const o=[];for(const x of xs){{for(const d of [0,1,2,3,6])o.push(x.toFixed(d));o.push(String(Math.round(x)));o.push(String(x));}}console.log(o.join('\\n'));",
            list.join(",")
        );
        let Ok(out) = std::process::Command::new("node")
            .arg("-e")
            .arg(&script)
            .output()
        else {
            eprintln!("node not found: skipping");
            return;
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8(out.stdout).expect("utf-8");
        let mut want = text.lines();
        for &x in &xs {
            for d in [0, 1, 2, 3, 6] {
                assert_eq!(
                    Some(to_fixed(x, d).as_str()),
                    want.next(),
                    "({x:e}).toFixed({d})"
                );
            }
            assert_eq!(
                Some(round_str(x).as_str()),
                want.next(),
                "Math.round({x:e})"
            );
            assert_eq!(Some(to_string(x).as_str()), want.next(), "String({x:e})");
        }
    }
}
