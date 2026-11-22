//! The five representative stress–strain curves and the model on them (mockup `defs()`,
//! `tempFactor()`, `stressAt()`).
//!
//! Units: strain in **percent**, stress in **MPa**, stiffness in GPa, density in kg/m³,
//! toughness in MJ/m³ (MPa × strain fraction), specific strength in kN·m/kg.
//!
//! Every arithmetic expression keeps the mockup's operation order so that results agree with
//! the JavaScript to the last bit (the goldens in `tests/materials_goldens.json` compare the
//! formatted strings exactly).

use serde::{Deserialize, Serialize};

/// Room temperature, °C (the bench's default and the curves' reference).
pub const ROOM_TEMP_C: f64 = 20.0;
/// Temperature slider range and step, °C (mockup `<input id="temp" min="-40" max="600"
/// step="10">`).
pub const TEMP_MIN_C: f64 = -40.0;
/// See [`TEMP_MIN_C`].
pub const TEMP_MAX_C: f64 = 600.0;
/// See [`TEMP_MIN_C`].
pub const TEMP_STEP_C: f64 = 10.0;
/// From this temperature on the temperature readout turns warm (`s.temp >= 200`) and the
/// explain sentence warns for aluminium, timber and carbon fibre.
pub const HOT_C: f64 = 200.0;
/// The sample's gauge cross-section, mm² ("Load cell … · 50 mm² section").
pub const SECTION_MM2: f64 = 50.0;
/// Largest necking (fraction of the gauge half-width lost) drawn at fracture (`0.55`).
pub const NECK_MAX: f64 = 0.55;
/// The stress axis runs 0 → 700 MPa (`SMAX`).
pub const STRESS_AXIS_MAX_MPA: f64 = 700.0;

/// One of the five samples on the bench, in the mockup's order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Mild structural steel (S275 / A36 class).
    #[default]
    Steel,
    /// 6061-T6 aluminium.
    Aluminum,
    /// Normal-strength concrete (≈ C30), shown in compression.
    Concrete,
    /// Clear softwood pulled along the grain.
    Timber,
    /// Quasi-isotropic carbon-fibre laminate.
    CarbonFiber,
}

impl Kind {
    /// All five, in the mockup's order (sample tiles, ghosts, bars, challenge chips).
    pub const ALL: [Kind; 5] = [
        Kind::Steel,
        Kind::Aluminum,
        Kind::Concrete,
        Kind::Timber,
        Kind::CarbonFiber,
    ];

    /// Position in [`Kind::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// Stable machine key (`steel`, `aluminum`, `concrete`, `timber`, `carbon-fiber`): Share
    /// lines, MCP, persistence.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Steel => "steel",
            Kind::Aluminum => "aluminum",
            Kind::Concrete => "concrete",
            Kind::Timber => "timber",
            Kind::CarbonFiber => "carbon-fiber",
        }
    }

    /// The mockup's key in `defs()` (`steel`, `alu`, `concrete`, `timber`, `cf`).
    pub fn mockup_key(self) -> &'static str {
        match self {
            Kind::Steel => "steel",
            Kind::Aluminum => "alu",
            Kind::Concrete => "concrete",
            Kind::Timber => "timber",
            Kind::CarbonFiber => "cf",
        }
    }

    /// Parses a key, a mockup key or a display name, ignoring case, spaces, `-` and `_`
    /// ("Carbon fiber", "carbon-fibre", "cf", "aluminium", "wood" all work).
    pub fn parse(s: &str) -> Option<Kind> {
        let k: String = s
            .chars()
            .filter(|c| !matches!(c, ' ' | '-' | '_'))
            .flat_map(char::to_lowercase)
            .collect();
        Some(match k.as_str() {
            "steel" | "mildsteel" | "s275" | "a36" => Kind::Steel,
            "aluminum" | "aluminium" | "alu" | "al" | "6061" | "6061t6" => Kind::Aluminum,
            "concrete" => Kind::Concrete,
            "timber" | "wood" | "softwood" => Kind::Timber,
            "carbonfiber" | "carbonfibre" | "cf" | "cfrp" | "carbon" => Kind::CarbonFiber,
            _ => return None,
        })
    }

    /// The curve and properties.
    pub fn curve(self) -> &'static Curve {
        &CURVES[self.index()]
    }

    /// Display name ("Carbon fiber").
    pub fn name(self) -> &'static str {
        self.curve().name
    }
}

/// A representative material on the bench (mockup `defs()`; sources in `docs/DATA-SOURCES.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    /// Which one.
    pub kind: Kind,
    /// Display name.
    pub name: &'static str,
    /// The stress–strain curve as `[strain %, stress MPa]` corners, starting at `[0, 0]`; the
    /// sample fractures past the last corner.
    pub points: &'static [[f64; 2]],
    /// Young's modulus E, GPa (the tile; the curve's first slope is consistent with it).
    pub e_gpa: f64,
    /// Density ρ, kg/m³.
    pub rho: f64,
    /// Strain at the ultimate (peak) stress, % (mockup `uts`): necking starts after it for the
    /// ductile metals.
    pub ultimate_strain: f64,
    /// Yield strength at room temperature, MPa, where the mockup defines one (steel 275,
    /// aluminium 276 — its 0.4 % corner); `None` for the brittle three.
    pub yield_mpa: Option<f64>,
    /// The note under the sample tiles.
    pub note: &'static str,
}

/// The five curves, in [`Kind::ALL`] order (mockup `defs()`, verbatim).
pub static CURVES: [Curve; 5] = [
    Curve {
        kind: Kind::Steel,
        name: "Steel",
        points: &[
            [0.0, 0.0],
            [0.1375, 275.0],
            [1.5, 275.0],
            [5.0, 360.0],
            [10.0, 415.0],
            [15.0, 430.0],
            [19.0, 405.0],
            [22.0, 360.0],
        ],
        e_gpa: 200.0,
        rho: 7850.0,
        ultimate_strain: 15.0,
        yield_mpa: Some(275.0),
        note: "Mild structural steel. Yields, flows, hardens, necks, then tears. Lots of warning.",
    },
    Curve {
        kind: Kind::Aluminum,
        name: "Aluminum",
        points: &[
            [0.0, 0.0],
            [0.4, 276.0],
            [1.0, 290.0],
            [4.0, 305.0],
            [8.0, 310.0],
            [11.0, 300.0],
            [12.0, 290.0],
        ],
        e_gpa: 69.0,
        rho: 2700.0,
        ultimate_strain: 8.0,
        yield_mpa: Some(276.0),
        note: "6061-T6 aluminum. A third of steel\u{2019}s weight and stiffness, no clean yield point.",
    },
    Curve {
        kind: Kind::Concrete,
        name: "Concrete",
        points: &[
            [0.0, 0.0],
            [0.1, 30.0],
            [0.2, 40.0],
            [0.3, 34.0],
            [0.35, 28.0],
        ],
        e_gpa: 30.0,
        rho: 2400.0,
        ultimate_strain: 0.2,
        yield_mpa: None,
        note: "Shown in compression. In tension it cracks at about a tenth of this, which is why it gets steel rebar.",
    },
    Curve {
        kind: Kind::Timber,
        name: "Timber",
        points: &[[0.0, 0.0], [0.67, 80.0]],
        e_gpa: 12.0,
        rho: 500.0,
        ultimate_strain: 0.67,
        yield_mpa: None,
        note: "Clear wood pulled along the grain. Strong for its weight, but it snaps without stretching.",
    },
    Curve {
        kind: Kind::CarbonFiber,
        name: "Carbon fiber",
        points: &[[0.0, 0.0], [0.86, 600.0]],
        e_gpa: 70.0,
        rho: 1600.0,
        ultimate_strain: 0.86,
        yield_mpa: None,
        note: "Quasi-isotropic laminate. Huge strength, straight line to failure, no yield at all.",
    },
];

/// The stress in a sample at a strain (mockup `stressAt()` result).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stress {
    /// Stress, MPa (0 once fractured).
    pub mpa: f64,
    /// True past the last corner of the curve.
    pub broken: bool,
    /// Strain at fracture, % (the last corner).
    pub break_strain: f64,
}

/// A temperature made safe for the model: NaN becomes room temperature (±∞ are fine).
pub fn sanitize_temp(t_c: f64) -> f64 {
    if t_c.is_nan() { ROOM_TEMP_C } else { t_c }
}

/// A strain made safe for the model: NaN and negative strains become 0 (the bench only pulls).
pub fn sanitize_strain(e: f64) -> f64 {
    if e.is_nan() || e < 0.0 { 0.0 } else { e }
}

/// The illustrative strength factor at `t_c` °C (1 = room-temperature strength), mockup
/// `tempFactor()`: steel holds to 300 °C then falls to 0.47 at 600 °C; aluminium holds to
/// 100 °C, 0.3 at 300 °C, 0.08 at 400 °C; concrete holds to 200 °C, 0.45 at 600 °C; timber
/// holds to 100 °C, 0.5 at 200 °C, 0 (charred) at 300 °C; carbon fibre (its epoxy) holds to
/// 100 °C, 0.25 at 200 °C. Linear in between, flat outside.
pub fn temp_factor(kind: Kind, t_c: f64) -> f64 {
    let t = sanitize_temp(t_c);
    let lerp = |a: f64, b: f64, t0: f64, t1: f64| {
        if t <= t0 {
            a
        } else if t >= t1 {
            b
        } else {
            a + (b - a) * (t - t0) / (t1 - t0)
        }
    };
    match kind {
        Kind::Steel => {
            if t <= 300.0 {
                1.0
            } else {
                lerp(1.0, 0.47, 300.0, 600.0)
            }
        }
        Kind::Aluminum => {
            if t <= 100.0 {
                1.0
            } else if t <= 300.0 {
                lerp(1.0, 0.3, 100.0, 300.0)
            } else {
                lerp(0.3, 0.08, 300.0, 400.0)
            }
        }
        Kind::Concrete => {
            if t <= 200.0 {
                1.0
            } else {
                lerp(1.0, 0.45, 200.0, 600.0)
            }
        }
        Kind::Timber => {
            if t <= 100.0 {
                1.0
            } else if t <= 200.0 {
                lerp(1.0, 0.5, 100.0, 200.0)
            } else {
                lerp(0.5, 0.0, 200.0, 300.0)
            }
        }
        Kind::CarbonFiber => {
            if t <= 100.0 {
                1.0
            } else {
                lerp(1.0, 0.25, 100.0, 200.0)
            }
        }
    }
}

/// The stress at `strain` % and `t_c` °C (mockup `stressAt()`): linear between the corners,
/// scaled by [`temp_factor`]; past the last corner the sample has fractured (stress 0).
/// NaN or negative strains count as 0.
pub fn stress_at(kind: Kind, strain: f64, t_c: f64) -> Stress {
    kind.curve().stress_at(strain, t_c)
}

impl Curve {
    /// Strain at fracture, % (the last corner).
    pub fn break_strain(&self) -> f64 {
        self.points.last().map_or(0.0, |p| p[0])
    }

    /// End of the first straight line, % (mockup `elasticLimit = p[1][0]`).
    pub fn elastic_limit(&self) -> f64 {
        self.points.get(1).map_or(0.0, |p| p[0])
    }

    /// True when the curve has more than its first straight line (mockup `p.length > 2`):
    /// steel, aluminium and concrete. Timber and carbon fibre are a straight line to failure.
    pub fn has_plastic_range(&self) -> bool {
        self.points.len() > 2
    }

    /// Ductile at break: a plastic range and more than 2 % stretch (the "Stretch at break" note).
    pub fn is_ductile(&self) -> bool {
        self.has_plastic_range() && self.break_strain() > 2.0
    }

    /// Whether the sample necks after its ultimate strain (mockup `m.uts > 1`: the two metals).
    pub fn necks(&self) -> bool {
        self.ultimate_strain > 1.0
    }

    /// The stress at `strain` % and `t_c` °C; see [`stress_at`].
    pub fn stress_at(&self, strain: f64, t_c: f64) -> Stress {
        let e = sanitize_strain(strain);
        let f = temp_factor(self.kind, t_c);
        let p = self.points;
        let brk = self.break_strain();
        if e > brk {
            return Stress {
                mpa: 0.0,
                broken: true,
                break_strain: brk,
            };
        }
        for w in p.windows(2) {
            let (a, b) = (w[0], w[1]);
            if e <= b[0] {
                return Stress {
                    mpa: (a[1] + (b[1] - a[1]) * (e - a[0]) / (b[0] - a[0])) * f,
                    broken: false,
                    break_strain: brk,
                };
            }
        }
        Stress {
            mpa: p.last().map_or(0.0, |q| q[1]) * f,
            broken: false,
            break_strain: brk,
        }
    }

    /// The highest corner of the curve at room temperature, MPa.
    pub fn room_peak_mpa(&self) -> f64 {
        self.points
            .iter()
            .map(|q| q[1])
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Peak (ultimate) strength at `t_c` °C, MPa (mockup `peak`; for concrete, in compression).
    pub fn peak_mpa(&self, t_c: f64) -> f64 {
        self.room_peak_mpa() * temp_factor(self.kind, t_c)
    }

    /// Yield strength at `t_c` °C, MPa, where defined (mockup `yieldS * f`).
    pub fn yield_at(&self, t_c: f64) -> Option<f64> {
        self.yield_mpa.map(|y| y * temp_factor(self.kind, t_c))
    }

    /// Toughness: the area under the room-temperature curve to fracture, MJ/m³ (mockup
    /// `tough`, trapezoids of strain/100 × stress). Note the mockup shows this value at every
    /// temperature; [`Curve::toughness_at`] is the heat-scaled variant.
    pub fn toughness(&self) -> f64 {
        let mut tough = 0.0;
        for w in self.points.windows(2) {
            let (a, b) = (w[0], w[1]);
            tough += (b[0] - a[0]) / 100.0 * (b[1] + a[1]) / 2.0;
        }
        tough
    }

    /// The area under the curve as drawn at `t_c` °C (toughness × the temperature factor),
    /// MJ/m³. Not shown by the mockup (see [`Curve::toughness`]).
    pub fn toughness_at(&self, t_c: f64) -> f64 {
        self.toughness() * temp_factor(self.kind, t_c)
    }

    /// Specific strength at `t_c` °C: peak strength ÷ density, kN·m/kg (mockup `spec`).
    pub fn specific_strength(&self, t_c: f64) -> f64 {
        self.peak_mpa(t_c) * 1e6 / self.rho / 1000.0
    }

    /// How much the gauge has thinned at `strain` %: 0 up to the ultimate strain, rising
    /// linearly to [`NECK_MAX`] at fracture (and held there once broken); always 0 for the
    /// materials that do not neck (mockup `necking`).
    pub fn necking(&self, strain: f64) -> f64 {
        let e = sanitize_strain(strain);
        let brk = self.break_strain();
        let uts = self.ultimate_strain;
        if e > uts && self.necks() {
            NECK_MAX.min((e.min(brk) - uts) / (brk - uts) * NECK_MAX)
        } else {
            0.0
        }
    }
}

/// The load cell's force for a stress on the 50 mm² section, kN (MPa × mm² = N).
pub fn load_cell_kn(stress_mpa: f64) -> f64 {
    stress_mpa * SECTION_MM2 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for k in Kind::ALL {
            assert_eq!(Kind::parse(k.key()), Some(k));
            assert_eq!(Kind::parse(k.mockup_key()), Some(k));
            assert_eq!(Kind::parse(k.name()), Some(k));
            assert_eq!(k.curve().kind, k);
        }
        assert_eq!(Kind::parse("Carbon Fibre"), Some(Kind::CarbonFiber));
        assert_eq!(Kind::parse("unobtainium"), None);
    }

    #[test]
    fn curves_start_at_the_origin_and_go_forward() {
        for c in &CURVES {
            assert_eq!(c.points[0], [0.0, 0.0]);
            assert!(c.points.windows(2).all(|w| w[1][0] > w[0][0]), "{}", c.name);
        }
    }

    #[test]
    fn first_slope_agrees_with_e_where_the_mockup_is_consistent() {
        // Steel 275 MPa / 0.1375 % = 200 GPa; timber 80 / 0.67 % ≈ 11.9 GPa (12); carbon fibre
        // 600 / 0.86 % ≈ 69.8 GPa (70); aluminium 276 / 0.4 % = 69 GPa; concrete 30 / 0.1 % = 30.
        for c in &CURVES {
            let slope_gpa = c.points[1][1] / (c.points[1][0] / 100.0) / 1000.0;
            assert!((slope_gpa - c.e_gpa).abs() / c.e_gpa < 0.01, "{}", c.name);
        }
    }
}
