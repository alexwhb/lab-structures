//! Temperature reduction factors for strength and stiffness (FIRE-1, LAB-1).
//!
//! Published tables, interpolated linearly between their points (as both Eurocodes allow), flat
//! below 20 °C and beyond the last point:
//!
//! - **Steel** (and Cable): EN 1993-1-2 Table 3.1, carbon steel — the effective yield strength
//!   `k_y,θ` and the slope of the linear elastic range `k_E,θ`.
//! - **Concrete** (also Road and Floor): EN 1992-1-2 Table 3.1, normal-weight concrete with
//!   siliceous aggregate — compressive strength `f_c,θ / f_ck`; tensile strength `k_c,t(θ)`
//!   (§3.2.2.2: 1.0 up to 100 °C, falling linearly to 0 at 600 °C); stiffness as the secant
//!   modulus to the peak, `f_c,θ / ε_c1,θ` relative to 20 °C (derived from the same table).
//! - **Wood**: full strength to 100 °C, nothing left at 300 °C (the char line of EN 1995-1-2
//!   §3.4.2 is the 300 °C isotherm); stiffness the same (illustrative).
//! - **Rope** (synthetic fibre): full strength to 100 °C, burned through by 250 °C (nylon and
//!   polyester melt at about 215–260 °C; illustrative).
//!
//! Sources and classes: `docs/DATA-SOURCES.md` § Fire.

use crate::builder::data::PieceType;

/// Room temperature, °C: every factor is 1 here and below.
pub const AMBIENT_C: f64 = 20.0;

/// EN 1993-1-2 Table 3.1: steel temperature θ, °C.
pub const STEEL_T: [f64; 13] = [
    20.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1100.0, 1200.0,
];
/// EN 1993-1-2 Table 3.1: reduction factor for the effective yield strength, `k_y,θ`.
pub const STEEL_KY: [f64; 13] = [
    1.0, 1.0, 1.0, 1.0, 1.0, 0.78, 0.47, 0.23, 0.11, 0.06, 0.04, 0.02, 0.0,
];
/// EN 1993-1-2 Table 3.1: reduction factor for the slope of the linear elastic range, `k_E,θ`.
pub const STEEL_KE: [f64; 13] = [
    1.0, 1.0, 0.9, 0.8, 0.7, 0.6, 0.31, 0.13, 0.09, 0.0675, 0.045, 0.0225, 0.0,
];

/// EN 1992-1-2 Table 3.1 (siliceous aggregate): concrete temperature θ, °C.
pub const CONCRETE_T: [f64; 13] = STEEL_T;
/// EN 1992-1-2 Table 3.1 (siliceous): compressive strength `f_c,θ / f_ck`.
pub const CONCRETE_KC: [f64; 13] = [
    1.0, 1.0, 0.95, 0.85, 0.75, 0.6, 0.45, 0.3, 0.15, 0.08, 0.04, 0.01, 0.0,
];
/// EN 1992-1-2 Table 3.1 (siliceous): strain at the peak stress `ε_c1,θ` (the table gives none at
/// 1200 °C, where the strength is 0; the last value is repeated).
pub const CONCRETE_EPS_C1: [f64; 13] = [
    0.0025, 0.004, 0.0055, 0.007, 0.01, 0.015, 0.025, 0.025, 0.025, 0.025, 0.025, 0.025, 0.025,
];

/// Wood: full strength up to here, °C (illustrative).
pub const WOOD_SOUND_C: f64 = 100.0;
/// Wood: burned through (char line, EN 1995-1-2 §3.4.2) at this temperature, °C.
pub const WOOD_CHAR_C: f64 = 300.0;
/// Rope: full strength up to here, °C (illustrative).
pub const ROPE_SOUND_C: f64 = 100.0;
/// Rope: melted / burned through at this temperature, °C (illustrative).
pub const ROPE_GONE_C: f64 = 250.0;
/// Concrete in tension: full strength up to here, °C (EN 1992-1-2 §3.2.2.2).
pub const CONCRETE_TENSION_SOUND_C: f64 = 100.0;
/// Concrete in tension: no strength left from here, °C (EN 1992-1-2 §3.2.2.2).
pub const CONCRETE_TENSION_GONE_C: f64 = 600.0;

/// How a material responds to heat: which table it follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FireMaterial {
    /// Structural steel: EN 1993-1-2 Table 3.1.
    Steel,
    /// Steel cable: the same table as Steel (SPEC FIRE-1).
    Cable,
    /// Concrete, Road and Floor: EN 1992-1-2 Table 3.1, siliceous aggregate.
    Concrete,
    /// Wood: chars through by 300 °C.
    Wood,
    /// Synthetic fibre rope: burned through by 250 °C.
    Rope,
}

impl FireMaterial {
    /// Every one, for tests and tables.
    pub const ALL: [FireMaterial; 5] = [
        FireMaterial::Steel,
        FireMaterial::Cable,
        FireMaterial::Concrete,
        FireMaterial::Wood,
        FireMaterial::Rope,
    ];

    /// The table a piece type follows (Road and Floor are concrete decks).
    pub fn of(t: PieceType) -> FireMaterial {
        match t {
            PieceType::Steel => FireMaterial::Steel,
            PieceType::Cable => FireMaterial::Cable,
            PieceType::Road | PieceType::Floor | PieceType::Concrete => FireMaterial::Concrete,
            PieceType::Wood => FireMaterial::Wood,
            PieceType::Rope => FireMaterial::Rope,
        }
    }

    /// Lost strength stays lost when it cools (charred wood, melted rope, cracked concrete); steel
    /// gets most of its strength back, so it follows its current temperature.
    pub fn damage_is_permanent(self) -> bool {
        !matches!(self, FireMaterial::Steel | FireMaterial::Cable)
    }

    /// The factors at `t_c` °C. NaN counts as room temperature.
    pub fn factors(self, t_c: f64) -> Factors {
        factors(self, t_c)
    }
}

/// Strength and stiffness multipliers at a temperature (1 = as at room temperature).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Factors {
    /// × the tension strength.
    pub strength_t: f64,
    /// × the compression (crushing) strength.
    pub strength_c: f64,
    /// × the stiffness E (axial stiffness and the Euler buckling load).
    pub stiffness: f64,
}

impl Factors {
    /// Room temperature: all 1.
    pub const ONE: Factors = Factors {
        strength_t: 1.0,
        strength_c: 1.0,
        stiffness: 1.0,
    };

    /// The smaller strength factor (how much of its strength is left, the weaker way).
    pub fn strength(&self) -> f64 {
        self.strength_t.min(self.strength_c)
    }
}

impl Default for Factors {
    fn default() -> Self {
        Factors::ONE
    }
}

/// Linear interpolation in a table (`xs` ascending); flat outside it. NaN → the first value.
pub fn interp(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let n = xs.len().min(ys.len());
    if n == 0 {
        return 1.0;
    }
    if x.is_nan() || x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }
    for i in 1..n {
        if x == xs[i] {
            return ys[i];
        }
        if x < xs[i] {
            let (x0, x1, y0, y1) = (xs[i - 1], xs[i], ys[i - 1], ys[i]);
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    ys[n - 1]
}

/// 1 up to `sound`, falling linearly to 0 at `gone`, 0 beyond.
fn ramp_down(t: f64, sound: f64, gone: f64) -> f64 {
    if t.is_nan() || t <= sound {
        1.0
    } else if t >= gone {
        0.0
    } else {
        1.0 - (t - sound) / (gone - sound)
    }
}

/// EN 1993-1-2 `k_y,θ` at `t_c` °C.
pub fn steel_ky(t_c: f64) -> f64 {
    interp(&STEEL_T, &STEEL_KY, t_c)
}

/// EN 1993-1-2 `k_E,θ` at `t_c` °C.
pub fn steel_ke(t_c: f64) -> f64 {
    interp(&STEEL_T, &STEEL_KE, t_c)
}

/// EN 1992-1-2 (siliceous) `f_c,θ / f_ck` at `t_c` °C.
pub fn concrete_kc(t_c: f64) -> f64 {
    interp(&CONCRETE_T, &CONCRETE_KC, t_c)
}

/// EN 1992-1-2 §3.2.2.2 `k_c,t(θ)` at `t_c` °C.
pub fn concrete_kct(t_c: f64) -> f64 {
    ramp_down(t_c, CONCRETE_TENSION_SOUND_C, CONCRETE_TENSION_GONE_C)
}

/// Concrete's secant stiffness to the peak relative to 20 °C: `(f_c,θ / ε_c1,θ) / (f_ck /
/// ε_c1,20)`, each factor interpolated from Table 3.1.
pub fn concrete_ke(t_c: f64) -> f64 {
    let eps = interp(&CONCRETE_T, &CONCRETE_EPS_C1, t_c);
    if eps <= 0.0 {
        return 0.0;
    }
    (concrete_kc(t_c) * CONCRETE_EPS_C1[0] / eps).clamp(0.0, 1.0)
}

/// The factors for `m` at `t_c` °C (see the module docs). NaN counts as room temperature.
pub fn factors(m: FireMaterial, t_c: f64) -> Factors {
    let t = if t_c.is_nan() { AMBIENT_C } else { t_c };
    match m {
        FireMaterial::Steel | FireMaterial::Cable => {
            let ky = steel_ky(t);
            Factors {
                strength_t: ky,
                strength_c: ky,
                stiffness: steel_ke(t),
            }
        }
        FireMaterial::Concrete => Factors {
            strength_t: concrete_kct(t),
            strength_c: concrete_kc(t),
            stiffness: concrete_ke(t),
        },
        FireMaterial::Wood => {
            let k = ramp_down(t, WOOD_SOUND_C, WOOD_CHAR_C);
            Factors {
                strength_t: k,
                strength_c: k,
                stiffness: k,
            }
        }
        FireMaterial::Rope => {
            let k = ramp_down(t, ROPE_SOUND_C, ROPE_GONE_C);
            Factors {
                strength_t: k,
                strength_c: k,
                stiffness: k,
            }
        }
    }
}

/// The factors for a piece type at `t_c` °C.
pub fn factors_for(t: PieceType, t_c: f64) -> Factors {
    factors(FireMaterial::of(t), t_c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interp_is_flat_outside_and_linear_inside() {
        assert_eq!(steel_ky(-40.0), 1.0);
        assert_eq!(steel_ky(450.0), 0.89);
        assert_eq!(steel_ky(5000.0), 0.0);
        assert_eq!(steel_ky(f64::NAN), 1.0);
        assert_eq!(interp(&[], &[], 3.0), 1.0);
    }

    #[test]
    fn concrete_stiffness_is_one_at_room_temperature() {
        assert_eq!(concrete_ke(20.0), 1.0);
        assert!((concrete_ke(400.0) - 0.75 * 0.25).abs() < 1e-12);
        assert_eq!(concrete_ke(1200.0), 0.0);
    }
}
