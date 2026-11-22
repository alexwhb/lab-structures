//! Number formatting that matches the mockup's JavaScript exactly (`toFixed`, `Math.round`,
//! `String(n)`), so the copy reads digit for digit like `Playground.dc.html`.
//!
//! Rust's `{:.1}` rounds exact ties to even (`0.25` → `0.2`) where JavaScript's `toFixed` picks
//! the larger candidate (`0.25` → `0.3`); `Math.round` rounds halves toward +∞ (`-2.5` → `-2`).

/// `x.toFixed(digits)`: fixed-point with ties going to the larger magnitude (JavaScript picks
/// the larger `n` of two equally near candidates, applied to `|x|` with the sign put back).
/// `NaN` → `"NaN"`, infinities → `"Infinity"` / `"-Infinity"`.
pub fn to_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let neg = x < 0.0;
    let a = x.abs();
    // Exact-enough decimal expansion: 25 extra digits separate a true tie from its neighbours
    // for every magnitude the game shows.
    let long = format!("{:.*}", digits + 25, a);
    let (int_part, frac_part) = long.split_once('.').unwrap_or((long.as_str(), ""));
    let mut digs: Vec<u8> = int_part.bytes().chain(frac_part.bytes()).collect();
    let keep = int_part.len() + digits;
    let round_up = digs.get(keep).is_some_and(|d| *d >= b'5');
    digs.truncate(keep);
    if round_up {
        let mut i = digs.len();
        loop {
            if i == 0 {
                digs.insert(0, b'1');
                break;
            }
            i -= 1;
            if digs[i] == b'9' {
                digs[i] = b'0';
            } else {
                digs[i] += 1;
                break;
            }
        }
    }
    let int_len = digs.len() - digits;
    let mut out = String::with_capacity(digs.len() + 2);
    if neg {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&digs[..int_len]).unwrap_or("0"));
    if digits > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&digs[int_len..]).unwrap_or("0"));
    }
    out
}

/// `Math.round(x)`: the nearest integer, halves toward +∞. Non-finite values pass through.
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// `Math.round(x)` printed as JavaScript prints it (`"63"`, `"NaN"`, `"Infinity"`).
pub fn round_str(x: f64) -> String {
    num(js_round(x))
}

/// `String(x)` for the numbers the game shows: integers without a decimal point, other
/// values in the shortest form that reads back exactly (`2.5`), `-0` as `0`.
pub fn num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    format!("{x}")
}

/// The star row: `★★★` for 3, `★☆☆` for 1 (`starsTxt` in the mockup).
pub fn stars(n: u8) -> String {
    let n = n.min(3) as usize;
    "★".repeat(n) + &"☆".repeat(3 - n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_matches_javascript() {
        assert_eq!(to_fixed(0.75, 1), "0.8");
        assert_eq!(to_fixed(0.25, 1), "0.3"); // Rust's {:.1} says 0.2
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(-2.5, 0), "-3");
        assert_eq!(to_fixed(1.005, 2), "1.00"); // 1.005 is 1.00499999… in binary
        assert_eq!(to_fixed(9.96, 1), "10.0");
        assert_eq!(to_fixed(99.5, 0), "100");
        assert_eq!(to_fixed(0.0, 1), "0.0");
        assert_eq!(to_fixed(-0.0, 1), "0.0");
        assert_eq!(to_fixed(-0.04, 1), "-0.0");
        assert_eq!(to_fixed(123456.789, 1), "123456.8");
        assert_eq!(to_fixed(f64::NAN, 1), "NaN");
        assert_eq!(to_fixed(f64::INFINITY, 0), "Infinity");
    }

    #[test]
    fn math_round_matches_javascript() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(js_round(62.5), 63.0);
        assert_eq!(round_str(f64::INFINITY), "Infinity");
    }

    #[test]
    fn numbers_print_like_javascript() {
        assert_eq!(num(20.0), "20");
        assert_eq!(num(2.5), "2.5");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(-12.0), "-12");
        assert_eq!(stars(0), "☆☆☆");
        assert_eq!(stars(2), "★★☆");
        assert_eq!(stars(9), "★★★");
    }
}
