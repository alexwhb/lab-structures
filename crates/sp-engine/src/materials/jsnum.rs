//! JavaScript-compatible number formatting, so readouts match the mockups to the character.
//!
//! Rust's `format!("{:.1}", x)` rounds an exact tie to even (`2.5` → `"2"`), JavaScript's
//! `toFixed` rounds it up (`"3"`); `Math.round(-2.5)` is `-2` in JavaScript. These helpers
//! follow the ECMAScript definitions. They are not materials-specific; other modules may use
//! them (`sp_engine::materials::jsnum`).

/// `x.toFixed(digits)` (ECMAScript `Number.prototype.toFixed`): the nearest decimal with
/// `digits` fraction digits, an exact tie rounding away from zero; `-` kept for negative
/// inputs that round to zero (`(-0.001).toFixed(2)` is `"-0.00"`); `NaN`, `Infinity`; numbers
/// ≥ 1e21 in magnitude as [`to_string`]. `digits` is capped at 20.
pub fn to_fixed(x: f64, digits: usize) -> String {
    if !x.is_finite() || x.abs() >= 1e21 {
        return to_string(x);
    }
    let digits = digits.min(20);
    // 60 exact fraction digits (Rust's precision formatting is exact): enough to tell a value
    // below a tie (next digit < 5) from a tie or above (next digit ≥ 5, which JavaScript rounds
    // up either way).
    let exact = format!("{:.60}", x.abs());
    let Some((int, frac)) = exact.split_once('.') else {
        return exact;
    };
    let mut ds: Vec<u8> = int.bytes().chain(frac.bytes().take(digits)).collect();
    let round_up = frac.as_bytes().get(digits).is_some_and(|&d| d >= b'5');
    let mut int_len = int.len();
    if round_up {
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

/// `String(x)` for the numbers the mockups print: the shortest round-trip digits (Rust's
/// `Display` gives the same digits), `-0` as `"0"`, `NaN`, `Infinity`, and JavaScript's
/// exponent form outside 1e-6 ≤ |x| < 1e21 (`1e+21`, `1e-7`).
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
    let a = x.abs();
    if (1e-6..1e21).contains(&a) {
        return format!("{x}");
    }
    let s = format!("{x:e}"); // "1e21", "1.5e-7"
    match s.split_once('e') {
        Some((m, e)) if !e.starts_with('-') => format!("{m}e+{e}"),
        _ => s,
    }
}

/// `Math.round(x)`: the nearest integer, a tie rounding towards +∞ (`Math.round(-2.5)` is −2).
pub fn round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_matches_javascript() {
        // Values checked against Node's Number.prototype.toFixed.
        let cases: &[(f64, usize, &str)] = &[
            (0.0, 2, "0.00"),
            (2.5, 0, "3"),
            (1.5, 0, "2"),
            (0.5, 0, "1"),
            (-2.5, 0, "-3"),
            (1.005, 2, "1.00"), // 1.00499999999999989…
            (1.045, 2, "1.04"),
            (0.125, 2, "0.13"),
            (83.7234375, 2, "83.72"),
            (9.995, 2, "9.99"),
            (9.9951, 2, "10.00"),
            (99.96, 1, "100.0"),
            (-0.001, 2, "-0.00"),
            (54.777_070_063_694_27, 0, "55"),
            (13.75, 1, "13.8"),
            (1234.5678, 3, "1234.568"),
            (0.08 + 0.08 + 0.08, 2, "0.24"),
        ];
        for &(x, d, want) in cases {
            assert_eq!(to_fixed(x, d), want, "({x}).toFixed({d})");
        }
        assert_eq!(to_fixed(f64::NAN, 2), "NaN");
        assert_eq!(to_fixed(f64::INFINITY, 1), "Infinity");
        assert_eq!(to_fixed(1e21, 2), "1e+21");
    }

    #[test]
    fn to_string_matches_javascript() {
        assert_eq!(to_string(22.0), "22");
        assert_eq!(to_string(0.86), "0.86");
        assert_eq!(to_string(-40.0), "-40");
        assert_eq!(to_string(-0.0), "0");
        assert_eq!(to_string(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(to_string(1e21), "1e+21");
        assert_eq!(to_string(1.5e-7), "1.5e-7");
    }

    #[test]
    fn round_matches_javascript() {
        assert_eq!(round(73.5), 74.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.49999999999999994), 0.0);
        assert_eq!(round(2.4), 2.0);
    }
}
