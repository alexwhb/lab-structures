//! How arguments from a person or a model are read and checked, so the app's bridge and
//! `structures-mcp`'s engine tools accept exactly the same values and refuse the rest with a
//! message that says what to send instead. Nothing here clamps silently: a value outside the
//! screen's range is an error (the engine would clamp it, and the model would never know).

use crate::report::DiagnosticInfo;
use serde::{Deserialize, Serialize};
use sp_engine::ground::{self, Foundation, GroundInput, Soil};
use sp_engine::materials::curves::{self, Kind};
use sp_engine::netlist::{self, Netlist};
use sp_engine::tower::{self, Shape, System, TowerInput};

/// The longest netlist accepted, in characters (the bridge reads 1 MiB per request; a whole
/// Bridge sandbox is a few kB).
pub const NETLIST_MAX_LEN: usize = 200_000;

/// The strain axis the tools use: 0–25 % (the Materials screen's wide axis).
pub const STRAIN_MAX_PCT: f64 = 25.0;

/// Parses a netlist. `Err` holds its errors (and warnings), each with line and column.
pub fn netlist(text: &str) -> Result<Netlist, Vec<DiagnosticInfo>> {
    if text.chars().count() > NETLIST_MAX_LEN {
        return Err(vec![DiagnosticInfo {
            line: 1,
            column: 1,
            severity: "error".into(),
            message: format!(
                "the netlist is longer than {NETLIST_MAX_LEN} characters; send a smaller one"
            ),
        }]);
    }
    netlist::parse(text).map_err(|d| d.iter().map(DiagnosticInfo::from).collect())
}

/// "The netlist has 2 errors: line 3, column 9: unknown node B9; line 5, column 1: …".
pub fn netlist_errors_text(diags: &[DiagnosticInfo]) -> String {
    let errors: Vec<&DiagnosticInfo> = diags.iter().filter(|d| d.severity == "error").collect();
    let shown: Vec<String> = errors.iter().take(10).map(|d| d.text()).collect();
    format!(
        "The netlist has {} error{}: {}{}",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" },
        shown.join("; "),
        if errors.len() > 10 { "; …" } else { "" }
    )
}

/// A sample: `steel`, `aluminum` (or `aluminium`), `concrete`, `timber`, `carbon-fiber` (or
/// `carbon fiber`, `carbon`, `cf`), any case.
pub fn material(s: &str) -> Result<Kind, String> {
    let k = s.trim().to_ascii_lowercase().replace([' ', '_'], "-");
    let k = match k.as_str() {
        "aluminium" => "aluminum",
        "carbon" | "cf" | "carbonfiber" => "carbon-fiber",
        other => other,
    };
    Kind::parse(k).ok_or_else(|| {
        format!(
            "unknown material `{s}`; materials are {}",
            Kind::ALL.map(Kind::key).join(", ")
        )
    })
}

/// A temperature in °C within the Materials screen's −40…600.
pub fn temperature(t: f64) -> Result<f64, String> {
    in_range(
        t,
        curves::TEMP_MIN_C,
        curves::TEMP_MAX_C,
        "temperature_c",
        "°C",
    )
}

/// A stretch in % within 0–25.
pub fn strain(e: f64) -> Result<f64, String> {
    in_range(e, 0.0, STRAIN_MAX_PCT, "strain_pct", "%")
}

fn in_range(v: f64, lo: f64, hi: f64, what: &str, unit: &str) -> Result<f64, String> {
    if v.is_finite() && (lo..=hi).contains(&v) {
        Ok(v)
    } else {
        Err(format!("`{what}` must be {lo} to {hi} {unit}, not {v}"))
    }
}

fn int_range(v: u32, lo: u32, hi: u32, what: &str) -> Result<u32, String> {
    if (lo..=hi).contains(&v) {
        Ok(v)
    } else {
        Err(format!("`{what}` must be {lo} to {hi}, not {v}"))
    }
}

/// Changes to a tower (unset fields keep the base's value).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TowerChange {
    /// Floors, 5–120 (4 m each).
    #[serde(default)]
    pub floors: Option<u32>,
    /// `core`, `tube`, `outrigger`, `braced`.
    #[serde(default)]
    pub system: Option<String>,
    /// `straight`, `tapered`, `twisted`, `hollow`.
    #[serde(default)]
    pub shape: Option<String>,
    /// Wind at 10 m, m/s, 5–50.
    #[serde(default)]
    pub wind_ms: Option<f64>,
    /// Tuned mass damper at the top.
    #[serde(default)]
    pub damper: Option<bool>,
}

impl TowerChange {
    /// `base` with these changes, or what's wrong with them.
    pub fn apply(&self, base: TowerInput) -> Result<TowerInput, String> {
        let mut i = base;
        if let Some(f) = self.floors {
            i.floors = int_range(f, tower::FLOORS_MIN, tower::FLOORS_MAX, "floors")?;
        }
        if let Some(s) = &self.system {
            i.system = key(
                s,
                System::from_key,
                System::ALL.map(|x| x.def().key),
                "system",
            )?;
        }
        if let Some(s) = &self.shape {
            i.shape = key(s, Shape::from_key, Shape::ALL.map(|x| x.def().key), "shape")?;
        }
        if let Some(v) = self.wind_ms {
            i.v10 = in_range(v, tower::V10_MIN, tower::V10_MAX, "wind_ms", "m/s")?;
        }
        if let Some(d) = self.damper {
            i.damper = d;
        }
        Ok(i)
    }

    /// Nothing to change.
    pub fn is_empty(&self) -> bool {
        self == &TowerChange::default()
    }
}

/// Changes to a building's ground (unset fields keep the base's value).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GroundChange {
    /// `dense`, `stiff`, `soft`, `loose`.
    #[serde(default)]
    pub soil: Option<String>,
    /// `footing`, `mat`, `piles`, `caisson`.
    #[serde(default)]
    pub foundation: Option<String>,
    /// Building floors, 2–80.
    #[serde(default)]
    pub floors: Option<u32>,
    /// Pile length, m, 8–40.
    #[serde(default)]
    pub pile_length_m: Option<f64>,
    /// Earthquake shaking.
    #[serde(default)]
    pub quake: Option<bool>,
    /// Years after construction, 0–30.
    #[serde(default)]
    pub years: Option<f64>,
}

impl GroundChange {
    /// `base` with these changes, or what's wrong with them.
    pub fn apply(&self, base: GroundInput) -> Result<GroundInput, String> {
        let mut i = base;
        if let Some(s) = &self.soil {
            i.soil = key(s, Soil::from_key, Soil::ALL.map(|x| x.def().key), "soil")?;
        }
        if let Some(s) = &self.foundation {
            i.foundation = key(
                s,
                Foundation::from_key,
                Foundation::ALL.map(|x| x.def().key),
                "foundation",
            )?;
        }
        if let Some(f) = self.floors {
            i.floors = int_range(f, ground::FLOORS_MIN, ground::FLOORS_MAX, "floors")?;
        }
        if let Some(p) = self.pile_length_m {
            i.pile_len = in_range(p, ground::PILE_MIN, ground::PILE_MAX, "pile_length_m", "m")?;
        }
        if let Some(q) = self.quake {
            i.quake = q;
        }
        if let Some(y) = self.years {
            i.years = in_range(y, 0.0, ground::YEARS_MAX, "years", "years")?;
        }
        Ok(i)
    }

    /// Nothing to change.
    pub fn is_empty(&self) -> bool {
        self == &GroundChange::default()
    }
}

fn key<T>(s: &str, parse: fn(&str) -> Option<T>, all: [&str; 4], what: &str) -> Result<T, String> {
    let k = s.trim().to_ascii_lowercase();
    let k = match (what, k.as_str()) {
        ("system", "outriggers") => "outrigger".to_string(),
        ("system", "braced tube" | "braced-tube") => "braced".to_string(),
        ("shape", "hollow core" | "hollow-core") => "hollow".to_string(),
        ("foundation", "footings") => "footing".to_string(),
        ("foundation", "mat slab" | "mat-slab") => "mat".to_string(),
        ("foundation", "pile") => "piles".to_string(),
        ("foundation", "caissons") => "caisson".to_string(),
        ("soil", "dense sand") => "dense".to_string(),
        ("soil", "stiff clay") => "stiff".to_string(),
        ("soil", "soft clay") => "soft".to_string(),
        ("soil", "loose wet sand" | "loose sand") => "loose".to_string(),
        _ => k,
    };
    parse(&k).ok_or_else(|| format!("unknown {what} `{s}`; use one of {}", all.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_by_any_name() {
        assert_eq!(material("Steel"), Ok(Kind::Steel));
        assert_eq!(material("aluminium"), Ok(Kind::Aluminum));
        assert_eq!(material("carbon fiber"), Ok(Kind::CarbonFiber));
        assert_eq!(material("CF"), Ok(Kind::CarbonFiber));
        let e = material("gold").unwrap_err();
        assert!(
            e.contains("steel, aluminum, concrete, timber, carbon-fiber"),
            "{e}"
        );
    }

    #[test]
    fn ranges_are_refused_not_clamped() {
        assert!(temperature(20.0).is_ok());
        assert!(temperature(700.0).unwrap_err().contains("-40 to 600"));
        assert!(temperature(f64::NAN).is_err());
        assert!(strain(30.0).is_err());
        let t = TowerChange {
            floors: Some(200),
            ..Default::default()
        };
        assert_eq!(
            t.apply(TowerInput::default()).unwrap_err(),
            "`floors` must be 5 to 120, not 200"
        );
        let g = GroundChange {
            pile_length_m: Some(2.0),
            ..Default::default()
        };
        assert!(g.apply(GroundInput::default()).is_err());
    }

    #[test]
    fn changes_apply_on_top_of_the_base() {
        let t = TowerChange {
            floors: Some(100),
            system: Some("Braced tube".into()),
            damper: Some(true),
            ..Default::default()
        }
        .apply(TowerInput::default())
        .unwrap();
        assert_eq!((t.floors, t.system, t.damper), (100, System::Braced, true));
        assert_eq!(t.shape, Shape::Straight, "unset keeps the base");
        let e = TowerChange {
            shape: Some("round".into()),
            ..Default::default()
        }
        .apply(TowerInput::default())
        .unwrap_err();
        assert_eq!(
            e,
            "unknown shape `round`; use one of straight, tapered, twisted, hollow"
        );
        let g = GroundChange {
            soil: Some("dense sand".into()),
            foundation: Some("piles".into()),
            pile_length_m: Some(34.0),
            ..Default::default()
        }
        .apply(GroundInput::default())
        .unwrap();
        assert_eq!((g.soil, g.foundation), (Soil::Dense, Foundation::Piles));
    }

    #[test]
    fn netlist_errors_have_places() {
        let e = netlist(".node A 0 0 fix=xy\n.member M1 A B9 s\n").unwrap_err();
        let text = netlist_errors_text(&e);
        assert!(text.starts_with("The netlist has"), "{text}");
        assert!(text.contains("line 2"), "{text}");
    }
}
