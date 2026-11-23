//! CMF-1 / CMF-2: the comfort meter (bands, sentence, the level's limit) and "Same storm,
//! four shapes" (the same conditions run headless on four versions of the player's tower).
//!
//! Owner: agent `time` (PROGRESS.md).
//!
//! **Scale.** The meter keeps the v1 Tower's bands and sentences (`sp_engine::tower`:
//! 5 / 10 / 20 milli-g, full scale 30+) but the builder's toy towers sway metres, not
//! centimetres: its comfort number (the settling engine's peak smoothed top-floor
//! acceleration) reads in the hundreds — the storm-tower board 272 mg against level 5's limit
//! of 300. So the bands are scaled by [`GAME_SCALE`] (illustrative, `docs/DATA-SOURCES.md`):
//! 75 / 150 / 300 mg, full scale 450+, which puts level 5's limit exactly on the "Seasick"
//! line ("… without making everyone seasick").
//!
//! **Four shapes** (simplified): the player's tower as built; Tapered — every joint's x drawn
//! toward the centre line in proportion to its height so the top is 60 % as wide (same pieces,
//! piece lengths not re-checked: it's a virtual variant); Twisted and Hollow core — the same
//! tower with the v1 shape factors on the wind: drag × `Cd / 1.3` (the builder's floors use
//! Cd 1.3) and the vortex-shedding gust term × the v1 cross-wind factor
//! (`sp_engine::tower::Shape::{Twisted, Hollow}`: Cd 1.15 / 1.0, cross 0.45 / 0.55).

use super::data::Scene;
use super::design::Design;
use super::game::{Outcome, run_headless};
use super::setup::{Conditions, Setup};
use super::sim::{FRAME, TEST_SECONDS};
use crate::jsnum::fixed;
use crate::tower::{self, Comfort, Tone};

/// The builder's comfort numbers are about this many times livelier than the v1 Tower's
/// real-world ones (see the module docs).
pub const GAME_SCALE: f64 = 15.0;
/// The builder's floor drag coefficient (the mockup's wind line `0.613·v²·1.3·4·4`).
pub const BUILDER_CD: f64 = 1.3;
/// Tapered: the top is this fraction as wide as the base.
pub const TAPER_TOP: f64 = 0.6;

/// The meter's band edges, milli-g (v1 `COMFORT_BANDS` × [`GAME_SCALE`]): 75, 150, 300.
pub fn bands() -> [f64; 3] {
    tower::COMFORT_BANDS.map(|b| b * GAME_SCALE)
}

/// The meter's full scale, milli-g (v1 `COMFORT_MAX` × [`GAME_SCALE`]): 450 ("450+").
pub fn full_scale() -> f64 {
    tower::COMFORT_MAX * GAME_SCALE
}

/// Where a value sits on the meter, percent (0–100; past full scale pins at 100).
pub fn pct(mg: f64) -> f64 {
    if !mg.is_finite() {
        return if mg > 0.0 { 100.0 } else { 0.0 };
    }
    (mg / full_scale() * 100.0).clamp(0.0, 100.0)
}

/// The band a builder comfort value falls in (the v1 band of `mg / GAME_SCALE`).
pub fn band(mg: f64) -> Comfort {
    Comfort::of(if mg.is_finite() { mg } else { f64::MAX } / GAME_SCALE)
}

/// "272 mg" (the HUD chip's `toFixed(0)`).
pub fn mg_text(mg: f64) -> String {
    format!("{} mg", fixed(if mg.is_finite() { mg } else { 0.0 }, 0))
}

/// The CMF-1 comfort meter for an open-ground run.
#[derive(Debug, Clone, PartialEq)]
pub struct Meter {
    /// The peak comfort value, milli-g.
    pub mg: f64,
    /// The level's comfort limit, milli-g (`None`: no limit).
    pub limit: Option<f64>,
    /// The band it falls in.
    pub band: Comfort,
    /// Where the marker sits, percent.
    pub marker_pct: f64,
    /// Where the limit sits, percent (`None`: no limit).
    pub limit_pct: Option<f64>,
    /// Over the level's limit.
    pub over_limit: bool,
}

/// One coloured stretch of the meter: from `from` to `to` milli-g, and its colour role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stretch {
    /// Start, milli-g.
    pub from: f64,
    /// End, milli-g.
    pub to: f64,
    /// Its band.
    pub band: Comfort,
}

impl Meter {
    /// The meter for a peak value and the level's limit.
    pub fn new(mg: f64, limit: Option<f64>) -> Meter {
        let limit = limit.filter(|l| l.is_finite() && *l > 0.0);
        Meter {
            mg,
            limit,
            band: band(mg),
            marker_pct: pct(mg),
            limit_pct: limit.map(pct),
            over_limit: limit.is_some_and(|l| mg > l || mg.is_nan()),
        }
    }

    /// The four stretches, left to right (0–75, 75–150, 150–300, 300–450+).
    pub fn stretches() -> [Stretch; 4] {
        let b = bands();
        [
            Stretch {
                from: 0.0,
                to: b[0],
                band: Comfort::Unnoticed,
            },
            Stretch {
                from: b[0],
                to: b[1],
                band: Comfort::Noticed,
            },
            Stretch {
                from: b[1],
                to: b[2],
                band: Comfort::Uneasy,
            },
            Stretch {
                from: b[2],
                to: full_scale(),
                band: Comfort::Seasick,
            },
        ]
    }

    /// The tick labels with their positions (percent): "0", "75", "150", "300", "450+ mg".
    pub fn ticks() -> Vec<(String, f64)> {
        let b = bands();
        let mut v = vec![("0".to_string(), 0.0)];
        for e in b {
            v.push((fixed(e, 0), pct(e)));
        }
        v.push((format!("{}+ mg", fixed(full_scale(), 0)), 100.0));
        v
    }

    /// The v1 sentence ("Hanging lamps swing. People get uneasy.").
    pub fn sentence(&self) -> &'static str {
        self.band.sentence()
    }

    /// The value's colour role (the v1 band's).
    pub fn tone(&self) -> Tone {
        if self.over_limit {
            Tone::Alert
        } else {
            self.band.tone()
        }
    }

    /// "272 mg".
    pub fn value(&self) -> String {
        mg_text(self.mg)
    }

    /// "limit 300 mg" (empty without a limit).
    pub fn limit_text(&self) -> String {
        self.limit
            .map(|l| format!("limit {}", mg_text(l)))
            .unwrap_or_default()
    }

    /// The accessible summary: "Comfort 272 milli-g, limit 300: Hanging lamps swing. People
    /// get uneasy."
    pub fn label(&self) -> String {
        let lim = self
            .limit
            .map(|l| format!(", limit {}", fixed(l, 0)))
            .unwrap_or_default();
        format!(
            "Comfort {} milli-g{lim}: {}",
            fixed(if self.mg.is_finite() { self.mg } else { 0.0 }, 0),
            self.sentence()
        )
    }
}

// ============================================================================================
// CMF-2: same storm, four shapes

/// One of the four versions of the player's tower.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variant {
    /// The tower as the player built it.
    AsBuilt,
    /// Drawn in toward the centre line with height: the top 60 % as wide.
    Tapered,
    /// The v1 Twisted shape's wind factors.
    Twisted,
    /// The v1 Hollow core shape's wind factors.
    Hollow,
}

impl Variant {
    /// In bar order (the player's first).
    pub const ALL: [Variant; 4] = [
        Variant::AsBuilt,
        Variant::Tapered,
        Variant::Twisted,
        Variant::Hollow,
    ];

    /// The bar's name.
    pub fn name(self) -> &'static str {
        match self {
            Variant::AsBuilt => "As built",
            Variant::Tapered => "Tapered",
            Variant::Twisted => "Twisted",
            Variant::Hollow => "Hollow core",
        }
    }

    /// The wind factors `(drag, shedding)` (the v1 shape's `Cd / 1.3` and cross-wind factor;
    /// 1, 1 for the as-built and tapered towers).
    pub fn factors(self) -> (f64, f64) {
        let shape = match self {
            Variant::AsBuilt | Variant::Tapered => return (1.0, 1.0),
            Variant::Twisted => tower::Shape::Twisted,
            Variant::Hollow => tower::Shape::Hollow,
        };
        let d = shape.def();
        (d.cd / BUILDER_CD, d.cross)
    }

    /// The verdict when this one sways least.
    fn verdict(self) -> &'static str {
        match self {
            Variant::AsBuilt => {
                "Your tower already rides it out best: none of the other shapes sways less."
            }
            Variant::Tapered => "Tapering cuts the sway the most: the top catches less wind.",
            Variant::Twisted => {
                "Twisting cuts the sway the most: the gusts can\u{2019}t shed in step with the sway."
            }
            Variant::Hollow => {
                "A hollow core cuts the sway the most: the wind slips through instead of pushing."
            }
        }
    }
}

/// One bar of "Same storm, four shapes".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    /// Which version.
    pub variant: Variant,
    /// Its peak comfort value, milli-g.
    pub mg: f64,
    /// It stood through the test.
    pub standing: bool,
    /// The bar's length, percent of the largest standing value (100 for one that fell).
    pub width: f64,
}

impl Bar {
    /// "272 mg", or "fell".
    pub fn value(&self) -> String {
        if self.standing {
            mg_text(self.mg)
        } else {
            "fell".into()
        }
    }

    /// The bar's colour role: the comfort band, Alert over the level's limit, Danger if it fell.
    pub fn tone(&self, limit: Option<f64>) -> Tone {
        if !self.standing {
            Tone::Danger
        } else if limit.is_some_and(|l| self.mg > l) {
            Tone::Alert
        } else {
            band(self.mg).tone()
        }
    }

    /// The accessible label: "Tapered: 140 mg".
    pub fn label(&self) -> String {
        format!("{}: {}", self.variant.name(), self.value())
    }
}

/// "Same storm, four shapes": the bars and the verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// The four bars, the player's first.
    pub bars: [Bar; 4],
    /// The one that sways least among those that stood.
    pub best: Option<Variant>,
    /// "Tapering cuts the sway the most: the top catches less wind."
    pub verdict: String,
}

/// The Tapered version of a design: every joint's x drawn toward the centre line (midway
/// between the leftmost and rightmost used joints) in proportion to its height above the
/// lowest used joint, so the highest is [`TAPER_TOP`] as far out. Same pieces, same crates.
pub fn tapered(design: &Design) -> Design {
    let used: Vec<&super::design::Joint> = design
        .nodes
        .iter()
        .filter(|n| design.members.iter().any(|m| m.a == n.id || m.b == n.id))
        .collect();
    let mut out = design.clone();
    if used.is_empty() {
        return out;
    }
    let xmin = used.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
    let xmax = used.iter().map(|n| n.x).fold(f64::NEG_INFINITY, f64::max);
    let ymin = used.iter().map(|n| n.y).fold(f64::INFINITY, f64::min);
    let ymax = used.iter().map(|n| n.y).fold(f64::NEG_INFINITY, f64::max);
    let cx = (xmin + xmax) / 2.0;
    let h = ymax - ymin;
    if h.is_nan() || h <= 0.0 || !cx.is_finite() {
        return out;
    }
    for n in &mut out.nodes {
        let u = ((n.y - ymin) / h).clamp(0.0, 1.0);
        let f = 1.0 - (1.0 - TAPER_TOP) * u;
        n.x = cx + (n.x - cx) * f;
    }
    out
}

/// Frames for a headless open-ground test (18 s at 30 frames per second, and a margin).
fn test_frames() -> usize {
    (TEST_SECONDS / FRAME).ceil() as usize + 30
}

/// Runs one version headless: `(peak milli-g, standing)`.
fn run_variant(design: &Design, setup: &Setup, cond: &Conditions, v: Variant) -> (f64, bool) {
    let d;
    let design = if v == Variant::Tapered {
        d = tapered(design);
        &d
    } else {
        design
    };
    let (drag, shedding) = v.factors();
    let (res, sim) = run_headless(design, setup, cond, false, test_frames(), |s| {
        s.drag = drag;
        s.shedding = shedding;
    });
    match res.map(|r| r.outcome) {
        Some(Outcome::Tower { standing, mg, .. }) => (mg, standing),
        _ => (sim.milli_g(), false),
    }
}

/// Whether "Compare shapes" applies: an open-ground scene and a tower with Floors.
pub fn applies(design: &Design, setup: &Setup) -> bool {
    setup.scene == Scene::Flat && design.top() > 0.0
}

/// CMF-2: runs the same conditions headless on the four versions of the player's tower (each
/// an 18 s open-ground test with the run's level and conditions) and compares their peak
/// comfort values. Deterministic.
pub fn compare(design: &Design, setup: &Setup, cond: &Conditions) -> Comparison {
    let runs: Vec<(f64, bool)> = std::thread::scope(|s| {
        let handles: Vec<_> = Variant::ALL
            .iter()
            .map(|&v| s.spawn(move || run_variant(design, setup, cond, v)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or((f64::NAN, false)))
            .collect()
    });
    let mut bars = [0, 1, 2, 3].map(|i| Bar {
        variant: Variant::ALL[i],
        mg: runs[i].0,
        standing: runs[i].1 && runs[i].0.is_finite(),
        width: 0.0,
    });
    let max = bars
        .iter()
        .filter(|b| b.standing)
        .map(|b| b.mg)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    for b in &mut bars {
        b.width = if b.standing {
            (b.mg / max * 100.0).clamp(0.0, 100.0)
        } else {
            100.0
        };
    }
    let best = bars
        .iter()
        .filter(|b| b.standing)
        .fold(None::<&Bar>, |acc, b| match acc {
            Some(a) if a.mg <= b.mg => Some(a),
            _ => Some(b),
        })
        .map(|b| b.variant);
    let verdict = match best {
        None => "None of the four shapes rides out this storm.".to_string(),
        Some(v) => {
            let mine = bars[0];
            if v != Variant::AsBuilt && mine.standing {
                let best_mg = bars
                    .iter()
                    .find(|b| b.variant == v)
                    .map_or(mine.mg, |b| b.mg);
                // within 2 %: call it a draw in the player's favour
                if best_mg >= mine.mg * 0.98 {
                    Variant::AsBuilt.verdict().to_string()
                } else {
                    v.verdict().to_string()
                }
            } else {
                v.verdict().to_string()
            }
        }
    };
    Comparison {
        bars,
        best,
        verdict,
    }
}
