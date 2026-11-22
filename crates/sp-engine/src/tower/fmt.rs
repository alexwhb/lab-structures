//! Number formatting that matches the mockups' JavaScript exactly, so the screens print the same
//! strings as `docs/design/mockup/*.dc.html` (`Number.prototype.toFixed`, `Math.round`).
//!
//! Rust's `{:.N}` rounds exact ties to even (`0.25` → `"0.2"`); JavaScript's `toFixed` rounds
//! them up (`"0.3"`). [`fixed`] reproduces `toFixed` on the exact binary value.

/// `x.toFixed(digits)`: the decimal with `digits` fraction digits nearest to the exact value of
/// `x`, ties going up (away from zero). `NaN` → `"NaN"`, `±∞` → `"Infinity"` / `"-Infinity"`.
/// A negative number that rounds to zero keeps its sign (`-0.04` → `"-0.0"`), like JavaScript;
/// `-0.0` prints as `"0.0"`.
pub fn fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let digits = digits.min(100);
    let neg = x < 0.0;
    // The exact decimal expansion (a double's fraction terminates within ~1075 digits; 80 extra
    // digits cover every value whose magnitude is above 1e-8, and below that only a tie could
    // differ, which needs a value that is a multiple of 10^-(digits+1)).
    let exact = format!("{:.*}", digits + 80, x.abs());
    let (int_part, frac_part) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let mut out: Vec<u8> = int_part.bytes().collect();
    let keep = &frac_part.as_bytes()[..digits.min(frac_part.len())];
    out.push(b'.');
    out.extend_from_slice(keep);
    let round_up = frac_part.as_bytes().get(digits).is_some_and(|&d| d >= b'5');
    if round_up {
        // Propagate the carry from the last digit leftwards, skipping the point.
        let mut i = out.len();
        loop {
            if i == 0 {
                out.insert(0, b'1');
                break;
            }
            i -= 1;
            match out[i] {
                b'.' => continue,
                b'9' => out[i] = b'0',
                d => {
                    out[i] = d + 1;
                    break;
                }
            }
        }
    }
    if digits == 0 {
        out.pop(); // the point
        if let Some(p) = out.iter().position(|&b| b == b'.') {
            out.truncate(p);
        }
    }
    let mut s = String::from_utf8(out).unwrap_or_default();
    if neg {
        s.insert(0, '-');
    }
    s
}

/// `Math.round(x)`: the nearest integer, ties toward +∞ (`2.5` → `3`, `-2.5` → `-2`).
/// Non-finite values pass through.
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// `Math.round(x)` printed as JavaScript prints it (`"259"`; `"-0"` never appears: a value that
/// rounds to zero prints `"0"`). Non-finite values print as JavaScript does.
pub fn round_str(x: f64) -> String {
    let r = js_round(x);
    if r.is_nan() {
        "NaN".to_owned()
    } else if r.is_infinite() {
        if r > 0.0 { "Infinity" } else { "-Infinity" }.to_owned()
    } else if r == 0.0 {
        "0".to_owned()
    } else {
        format!("{r:.0}")
    }
}

/// A plain JavaScript number to string for the integers and short decimals the mockups print
/// with `+` (`30` → `"30"`, `0.5` → `"0.5"`).
pub fn num(x: f64) -> String {
    if x.is_finite() && x == x.trunc() && x.abs() < 1e15 {
        format!("{x:.0}")
    } else {
        format!("{x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_matches_javascript() {
        assert_eq!(fixed(0.25, 1), "0.3");
        assert_eq!(fixed(0.125, 2), "0.13");
        assert_eq!(fixed(2.5, 0), "3");
        assert_eq!(fixed(1.005, 2), "1.00"); // 1.005 is 1.00499… in binary
        assert_eq!(fixed(9.95, 1), "9.9"); // 9.949999…
        assert_eq!(fixed(99.96, 1), "100.0");
        assert_eq!(fixed(0.0, 2), "0.00");
        assert_eq!(fixed(-0.0, 1), "0.0");
        assert_eq!(fixed(-0.04, 1), "-0.0");
        assert_eq!(fixed(-1.25, 1), "-1.3");
        assert_eq!(fixed(927.83, 0), "928");
        assert_eq!(fixed(0.10073, 3), "0.101");
        assert_eq!(fixed(f64::NAN, 1), "NaN");
        assert_eq!(fixed(f64::INFINITY, 1), "Infinity");
        assert_eq!(fixed(1e-12, 2), "0.00");
        assert_eq!(fixed(123456789.987, 2), "123456789.99");
    }

    #[test]
    fn round_matches_javascript() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(258.7), 259.0);
        assert_eq!(round_str(-0.2), "0");
        assert_eq!(round_str(258.7), "259");
        assert_eq!(num(30.0), "30");
        assert_eq!(num(0.5), "0.5");
    }
}
