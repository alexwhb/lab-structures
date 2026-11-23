//! The sample drawer's state (LAB-1): which piece type, Pull or Push, the temperature, the
//! sample's length for Push, and how far it has been stretched or squeezed — stepped like v1's
//! tensile bench (`materials::Bench`: 20 steps a second, paced, deterministic), with the status
//! pill and the "What's happening" sentences in the v1 voice.

use super::curve::{DEFAULT_LENGTH_M, Failure, MIN_LENGTH_M, SampleCurve, Test, safe_temp, tonnes};
use crate::builder::data::PieceType;
use crate::builder::fire::{AMBIENT_C, FireMaterial};
use crate::jsnum::{to_fixed, to_string};

/// One step every 50 ms (v1's `setInterval(…, 50)`): 20 a second.
pub const TICK_SECONDS: f64 = 0.05;
/// Most steps one [`Sample::advance`] runs.
pub const MAX_TICKS_PER_ADVANCE: usize = 400;
/// The straight-line part takes this many steps (so "Elastic" shows for a moment).
pub const ELASTIC_STEPS: f64 = 12.0;
/// The rest of the axis takes this many steps.
pub const AXIS_STEPS: f64 = 160.0;
/// Temperature slider range and step, °C (SPEC LAB-1: −40…900 °C).
pub const TEMP_MIN_C: f64 = -40.0;
/// See [`TEMP_MIN_C`].
pub const TEMP_MAX_C: f64 = 900.0;
/// See [`TEMP_MIN_C`].
pub const TEMP_STEP_C: f64 = 10.0;

/// The colour role of the status pill (the screen maps it to `sp_theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Ready.
    Muted,
    /// On the straight line.
    Ok,
    /// Yielding, squashing, bowing.
    Accent,
    /// Past its peak.
    Caution,
    /// Broken.
    Danger,
}

/// Where the sample is on its curve (the status pill).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    /// Not loaded yet.
    Ready,
    /// On the straight line: it springs back.
    Elastic,
    /// Pulled past yield ("Plastic: permanently stretched").
    Plastic,
    /// Past its peak, one spot thinning.
    Necking,
    /// Pushed past yield (steel): "Squashing: permanently shortened".
    Squashing,
    /// Pushed past the Euler load: bowing sideways.
    Bowing,
    /// Pushed past its peak: cracking (concrete) / fibres folding (wood).
    PastPeak,
    /// Failed at the end of its curve, at this strain %.
    Failed {
        /// How.
        how: Failure,
        /// Strain at failure, %.
        at: f64,
    },
    /// Nothing left to test at this temperature.
    BurnedThrough,
}

impl Status {
    /// The pill's text for a sample of `kind`.
    pub fn label(&self, kind: PieceType) -> String {
        match *self {
            Status::Ready => "Ready".into(),
            Status::Elastic => "Elastic: springs back".into(),
            Status::Plastic => "Plastic: permanently stretched".into(),
            Status::Necking => "Necking".into(),
            Status::Squashing => "Squashing: permanently shortened".into(),
            Status::Bowing => "Bowing sideways".into(),
            Status::PastPeak => {
                if FireMaterial::of(kind) == FireMaterial::Wood {
                    "Fibres folding: past its peak".into()
                } else {
                    "Cracking: past its peak".into()
                }
            }
            Status::Failed { how, at } => {
                let word = match how {
                    Failure::Fractured => "Fractured",
                    Failure::Snapped => "Snapped",
                    Failure::Cracked => "Cracked",
                    Failure::Squashed => "Squashed",
                    Failure::Crushed => "Crushed",
                    Failure::Buckled => "Buckled",
                    Failure::BurnedThrough => "Burned through",
                };
                format!("{word} at {}%", fmt_strain(at))
            }
            Status::BurnedThrough => {
                if FireMaterial::of(kind) == FireMaterial::Rope {
                    "Melted through".into()
                } else {
                    "Burned through".into()
                }
            }
        }
    }

    /// The pill's colour role.
    pub fn tone(&self) -> Tone {
        match self {
            Status::Ready => Tone::Muted,
            Status::Elastic => Tone::Ok,
            Status::Plastic | Status::Squashing | Status::Bowing => Tone::Accent,
            Status::Necking | Status::PastPeak => Tone::Caution,
            Status::Failed { .. } | Status::BurnedThrough => Tone::Danger,
        }
    }

    /// The sample has failed (or there was nothing to test).
    pub fn is_broken(&self) -> bool {
        matches!(self, Status::Failed { .. } | Status::BurnedThrough)
    }
}

/// A strain in % with as many decimals as it needs (≥ 2, up to 4 significant figures for tiny
/// strains): `22`, `0.13`, `0.0083`.
pub fn fmt_strain(e: f64) -> String {
    if !e.is_finite() || e <= 0.0 {
        return "0".into();
    }
    let digits = if e >= 1.0 {
        2
    } else {
        // Two significant figures below 1 %.
        (1.0 - e.log10().floor()).clamp(2.0, 6.0) as usize
    };
    let s = to_fixed(e, digits);
    // Trim trailing zeros like JavaScript's String(Number(..)).
    to_string(s.parse::<f64>().unwrap_or(e))
}

/// A force in the builder's tonnes: one decimal under 10 t, else whole (`fmtT`).
pub fn fmt_tonnes(n: f64) -> String {
    let t = tonnes(n).abs();
    to_fixed(t, if t < 10.0 { 1 } else { 0 })
}

/// A force in kN: one decimal under 100 kN, else whole.
pub fn fmt_kn(n: f64) -> String {
    let k = (n / 1000.0).abs();
    to_fixed(k, if k < 100.0 { 1 } else { 0 })
}

/// The drawer (state only; [`Sample::view`] makes what it shows).
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    kind: PieceType,
    test: Test,
    temp: f64,
    length: f64,
    strain: f64,
    playing: bool,
    acc: f64,
    curve: Option<SampleCurve>,
}

impl Default for Sample {
    /// A fresh Steel sample, Pull, 20 °C, 1 m, paused.
    fn default() -> Self {
        Sample::new(PieceType::Steel)
    }
}

impl Sample {
    /// A fresh sample of `kind`, ready to pull at 20 °C (1 m for pushing).
    pub fn new(kind: PieceType) -> Sample {
        let mut s = Sample {
            kind,
            test: Test::Pull,
            temp: AMBIENT_C,
            length: DEFAULT_LENGTH_M,
            strain: 0.0,
            playing: false,
            acc: 0.0,
            curve: None,
        };
        s.rebuild();
        s
    }

    fn rebuild(&mut self) {
        self.curve = SampleCurve::new(self.kind, self.test, self.temp, self.length);
    }

    /// Opens the drawer on `kind` (a Materials card): a fresh sample to pull; the temperature
    /// is kept (compare materials at the same heat), the length goes back to 1 m.
    pub fn open(&mut self, kind: PieceType) {
        self.kind = kind;
        self.test = Test::Pull;
        self.length = DEFAULT_LENGTH_M;
        self.strain = 0.0;
        self.playing = false;
        self.acc = 0.0;
        self.rebuild();
    }

    /// The piece type.
    pub fn kind(&self) -> PieceType {
        self.kind
    }

    /// Pull or push.
    pub fn test(&self) -> Test {
        self.test
    }

    /// Temperature, °C.
    pub fn temp(&self) -> f64 {
        self.temp
    }

    /// Sample length for Push, m.
    pub fn length(&self) -> f64 {
        self.length
    }

    /// Strain so far, % (shortening when pushing).
    pub fn strain(&self) -> f64 {
        self.strain
    }

    /// Pulling or pushing right now.
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// Whether this piece type can be pushed (not Rope or Cable).
    pub fn can_push(&self) -> bool {
        !self.kind.spec().t_only
    }

    /// The longest sample for Push, m (the piece type's longest piece).
    pub fn max_length(&self) -> f64 {
        self.kind.spec().max_len.max(MIN_LENGTH_M)
    }

    /// The curve of the current test (`None` only for pushing a tension-only piece, which the
    /// drawer never does).
    pub fn curve(&self) -> Option<&SampleCurve> {
        self.curve.as_ref()
    }

    /// The curve for the other test at the same temperature and length, if there is one.
    pub fn curve_for(&self, test: Test) -> Option<SampleCurve> {
        SampleCurve::new(self.kind, test, self.temp, self.length)
    }

    /// The same test at 20 °C (the ghost curve when heated).
    pub fn room_curve(&self) -> Option<SampleCurve> {
        SampleCurve::new(self.kind, self.test, AMBIENT_C, self.length)
    }

    /// The stress now, MPa, and whether it has broken.
    pub fn stress(&self) -> super::curve::Stress {
        self.curve.as_ref().map_or(
            super::curve::Stress {
                mpa: 0.0,
                broken: false,
            },
            |c| c.stress_at(self.strain),
        )
    }

    /// The force on the builder's section now, N (positive; pulled or pushed per the test).
    pub fn force_n(&self) -> f64 {
        self.curve.as_ref().map_or(0.0, |c| c.force_at(self.strain))
    }

    /// The builder's capacity for this piece this way, N.
    pub fn capacity_n(&self) -> f64 {
        self.curve.as_ref().map_or(0.0, |c| c.capacity_n)
    }

    /// True once the force has reached what the builder allows (the piece would count as
    /// broken there): a tiny tolerance for the straight line's last step.
    pub fn at_builder_limit(&self) -> bool {
        let cap = self.capacity_n();
        self.strain > 0.0 && cap > 0.0 && {
            let c = self.curve.as_ref();
            c.is_some_and(|c| self.strain + 1e-12 >= c.elastic_strain)
        }
    }

    /// The strain axis end, %: a round number past the failure strain (v1's two zooms, fitted).
    pub fn axis_max(&self) -> f64 {
        let brk = self.curve.as_ref().map_or(0.0, |c| c.break_strain());
        nice_ceil(brk * 1.1)
    }

    /// Strain per step, %: the straight line in [`ELASTIC_STEPS`], the rest of the axis in
    /// [`AXIS_STEPS`].
    pub fn step(&self) -> f64 {
        let Some(c) = &self.curve else {
            return 0.0;
        };
        let el = c.elastic_strain;
        if self.strain < el - 1e-12 {
            (el / ELASTIC_STEPS).max(f64::MIN_POSITIVE)
        } else {
            (self.axis_max() / AXIS_STEPS).max(f64::MIN_POSITIVE)
        }
    }

    /// "Pull": starts pulling (a fresh sample if it was pushed or broken); pauses while pulling.
    pub fn pull(&mut self) {
        self.drive(Test::Pull);
    }

    /// "Push": starts pushing (a fresh sample if it was pulled or broken); pauses while pushing.
    /// Does nothing for Rope and Cable.
    pub fn push(&mut self) {
        if self.can_push() {
            self.drive(Test::Push);
        }
    }

    fn drive(&mut self, test: Test) {
        if self.test == test && self.playing {
            self.playing = false;
            self.acc = 0.0;
            return;
        }
        if self.test != test {
            self.test = test;
            self.strain = 0.0;
            self.rebuild();
        } else if self.stress().broken || self.strain >= self.axis_max() {
            self.strain = 0.0;
        }
        self.playing = !self.stress().broken;
        self.acc = 0.0;
    }

    /// "Fresh sample": unloaded and paused, same test.
    pub fn fresh_sample(&mut self) {
        self.strain = 0.0;
        self.playing = false;
        self.acc = 0.0;
    }

    /// The temperature slider: clamped to −40…900 °C (NaN ignored). The sample keeps its strain
    /// (heating a loaded sample can break it).
    pub fn set_temp(&mut self, t_c: f64) {
        if t_c.is_nan() {
            return;
        }
        self.temp = safe_temp(t_c).clamp(TEMP_MIN_C, TEMP_MAX_C);
        self.rebuild();
        if self.stress().broken {
            self.playing = false;
        }
    }

    /// The sample's length for Push, m: whole metres from 1 to the type's longest piece; a fresh
    /// sample. NaN ignored.
    pub fn set_length(&mut self, len: f64) {
        if len.is_nan() {
            return;
        }
        self.length = len.round().clamp(MIN_LENGTH_M, self.max_length());
        self.strain = 0.0;
        self.playing = false;
        self.rebuild();
    }

    /// Sets the strain directly (tests, MCP), clamped to the axis; pauses.
    pub fn set_strain(&mut self, strain: f64) {
        let e = if strain.is_nan() || strain < 0.0 {
            0.0
        } else {
            strain
        };
        self.strain = e.min(self.axis_max());
        self.playing = false;
    }

    /// One step while pulling / pushing: adds [`Sample::step`]; stops at failure or the axis end.
    /// Returns `true` if this step broke it.
    pub fn tick(&mut self) -> bool {
        if !self.playing {
            return false;
        }
        let step = self.step();
        let el = self.curve.as_ref().map_or(0.0, |c| c.elastic_strain);
        let mut e = (self.strain + step).min(self.axis_max());
        // Land exactly on the end of the straight line (the builder's limit) once.
        if self.strain < el && e > el {
            e = el;
        }
        self.strain = e;
        let broken = self.stress().broken;
        if broken || e >= self.axis_max() {
            self.playing = false;
        }
        broken
    }

    /// Lets `dt` seconds pass: one [`Sample::tick`] per [`TICK_SECONDS`] while running (at most
    /// [`MAX_TICKS_PER_ADVANCE`]). Returns `true` if it broke.
    pub fn advance(&mut self, dt: f64) -> bool {
        if !self.playing {
            self.acc = 0.0;
            return false;
        }
        if dt.is_finite() && dt > 0.0 {
            self.acc = (self.acc + dt).min(TICK_SECONDS * MAX_TICKS_PER_ADVANCE as f64);
        }
        let mut broke = false;
        let mut n = 0;
        while self.playing && self.acc + 1e-9 >= TICK_SECONDS && n < MAX_TICKS_PER_ADVANCE {
            self.acc = (self.acc - TICK_SECONDS).max(0.0);
            n += 1;
            broke |= self.tick();
        }
        if !self.playing {
            self.acc = 0.0;
        }
        broke
    }

    /// Seconds until the next step is due (paced repaints), `None` while paused.
    pub fn next_tick_in(&self) -> Option<f64> {
        self.playing
            .then(|| (TICK_SECONDS - self.acc).clamp(0.0, TICK_SECONDS))
    }

    /// Runs the current test to its end (MCP, tests): starts it if needed and steps until it
    /// stops. Returns whether it broke.
    pub fn run_to_end(&mut self) -> bool {
        if !self.playing {
            self.drive(self.test);
        }
        let mut n = 0;
        while self.playing && n < 10_000 {
            self.tick();
            n += 1;
        }
        self.stress().broken
    }

    /// The status pill.
    pub fn status(&self) -> Status {
        let Some(c) = &self.curve else {
            return Status::Ready;
        };
        if c.burned_through() {
            return Status::BurnedThrough;
        }
        let now = c.stress_at(self.strain);
        if now.broken {
            return Status::Failed {
                how: c.failure,
                at: c.break_strain(),
            };
        }
        if self.strain <= 0.0 {
            return Status::Ready;
        }
        if self.strain < c.elastic_strain - 1e-12 {
            return Status::Elastic;
        }
        match c.failure {
            Failure::Fractured => {
                if c.necking(self.strain) > 0.02 {
                    Status::Necking
                } else if self.strain > c.elastic_strain + 1e-12 {
                    Status::Plastic
                } else {
                    Status::Elastic
                }
            }
            Failure::Squashed if self.strain > c.elastic_strain + 1e-12 => Status::Squashing,
            Failure::Buckled if self.strain > c.elastic_strain + 1e-12 => Status::Bowing,
            Failure::Crushed if self.strain > c.elastic_strain + 1e-12 => Status::PastPeak,
            _ => Status::Elastic,
        }
    }

    /// "A Steel piece this size breaks at 153 t." (Pull) / "A 1 m Steel piece gives way at 153
    /// t." (Push; "buckles at" when it buckles).
    pub fn limit_sentence(&self) -> String {
        let name = self.kind.name();
        let Some(c) = &self.curve else {
            return format!("{name} only pulls.");
        };
        let t = to_fixed(tonnes(c.capacity_n), 0);
        match self.test {
            Test::Pull => format!("A {name} piece this size breaks at {t} t."),
            Test::Push => {
                let verb = if c.buckles { "buckles" } else { "gives way" };
                let len = to_string(self.length);
                let article = if len.starts_with('8') || len == "11" || len == "18" {
                    "An"
                } else {
                    "A"
                };
                format!("{article} {len} m {name} piece {verb} at {t} t.")
            }
        }
    }

    /// The "What's happening" sentence (v1's voice).
    pub fn explain(&self) -> String {
        let name = self.kind.name();
        let lower = name.to_lowercase();
        let Some(c) = &self.curve else {
            return format!("{name} only pulls: pushed, it just goes slack.");
        };
        let cap_t = to_fixed(tonnes(c.capacity_n), 0);
        let spring = "At first the sample behaves like a stiff spring: let go and it returns to its length. The slope of that first straight line is the stiffness, E.";
        let mut x = match self.status() {
            Status::BurnedThrough => {
                if FireMaterial::of(self.kind) == FireMaterial::Rope {
                    format!(
                        "At {} \u{b0}C the rope has melted through: there is nothing left to pull. Synthetic fibres soften from about 100 \u{b0}C and melt by about 250 \u{b0}C.",
                        to_string(self.temp)
                    )
                } else {
                    format!(
                        "At {} \u{b0}C the {lower} has charred right through: there is nothing left to test. Wood loses strength from about 100 \u{b0}C and has none left by 300 \u{b0}C.",
                        to_string(self.temp)
                    )
                }
            }
            Status::Ready | Status::Elastic => match self.test {
                Test::Pull => format!("{spring} Press Pull to keep pulling."),
                Test::Push => format!("{spring} Press Push to keep squeezing."),
            },
            Status::Plastic => format!(
                "Past the yield point the atoms start slipping past each other. The metal stretches a lot for little extra force, and it will not spring back fully. In the builder a {lower} piece counts as broken right here, at {cap_t} t: it would keep stretching."
            ),
            Status::Necking => "The sample has passed its peak strength. One spot thins out (necking), all the stretching concentrates there, and the force it can carry drops until it tears.".into(),
            Status::Squashing => format!(
                "Past its yield strength the steel squashes for good: it keeps shortening and bulging for no extra force. It does not shatter. In the builder that counts as crushed, at {cap_t} t."
            ),
            Status::Bowing => format!(
                "A long, slim sample does not crush: at {cap_t} t it bows sideways and the force it carries falls away. Twice as long buckles at a quarter of the push."
            ),
            Status::PastPeak => {
                if FireMaterial::of(self.kind) == FireMaterial::Wood {
                    "Past its peak the wood fibres fold over each other in a band across the sample. It still carries most of the load for a while, then it crushes.".into()
                } else {
                    "Past its peak strength the cracks join up and run along the sample. Each step of squeezing carries less force, until it crushes.".into()
                }
            }
            Status::Failed { how, at } => {
                let at = fmt_strain(at);
                match how {
                    Failure::Fractured => format!(
                        "It tore after stretching {at}%. The area under the curve is the energy it soaked up first: its toughness. That long warning is why bridges hang from steel."
                    ),
                    Failure::Snapped => format!(
                        "It snapped at {at}% with no yielding and no warning. Strong but brittle: tiny area under the curve, so little energy absorbed."
                    ),
                    Failure::Cracked => {
                        let ratio = self
                            .curve_for(Test::Push)
                            .filter(|p| p.capacity_n > 0.0 && c.capacity_n > 0.0)
                            .map(|p| p.capacity_n / c.capacity_n);
                        let tail = match ratio {
                            Some(r) if r >= 1.5 => format!(
                                " Pushed, it carries about {} times as much: use it where things push.",
                                to_fixed(r, 0)
                            ),
                            _ => String::new(),
                        };
                        format!("It cracked at {at}%, almost before it stretched at all.{tail}")
                    }
                    Failure::Squashed => format!(
                        "It squashed to {at}% shorter and bulged, but it never shattered: steel gives plenty of warning in compression too."
                    ),
                    Failure::Crushed => format!(
                        "It crushed at {at}% squeezing, with little warning: a small area under the curve, so little energy absorbed."
                    ),
                    Failure::Buckled => format!(
                        "It buckled at {cap_t} t, long before the {lower} itself would crush ({} t). Shorter pieces, or a brace halfway, push the buckling load up.",
                        to_fixed(tonnes(c.crush_n), 0)
                    ),
                    Failure::BurnedThrough => String::new(),
                }
            }
        };
        if let Some(h) = self.heat_note() {
            x.push(' ');
            x.push_str(&h);
        }
        x
    }

    /// What the temperature does now, or `None` at full strength: "At 600 °C steel keeps 47% of
    /// its strength and 31% of its stiffness (EN 1993-1-2)."
    pub fn heat_note(&self) -> Option<String> {
        let c = self.curve.as_ref()?;
        if c.burned_through() || (c.strength >= 1.0 && c.stiffness >= 1.0) {
            return None;
        }
        let src = match FireMaterial::of(self.kind) {
            FireMaterial::Steel | FireMaterial::Cable => " (EN 1993-1-2)",
            FireMaterial::Concrete => " (EN 1992-1-2)",
            _ => "",
        };
        Some(format!(
            "At {} \u{b0}C {} keeps {}% of its strength and {}% of its stiffness{src}.",
            to_string(self.temp),
            self.kind.name().to_lowercase(),
            to_string(crate::jsnum::round(c.strength * 100.0)),
            to_string(crate::jsnum::round(c.stiffness * 100.0)),
        ))
    }

    /// One line for accessibility and the MCP summary.
    pub fn describe(&self) -> String {
        let verb = match self.test {
            Test::Pull => "pulled",
            Test::Push => "pushed",
        };
        format!(
            "Sample drawer: {} {verb} at {} \u{b0}C, {} %, {} t ({} kN), {}. {}",
            self.kind.name(),
            to_string(self.temp),
            fmt_strain(self.strain),
            fmt_tonnes(self.force_n()),
            fmt_kn(self.force_n()),
            self.status().label(self.kind),
            self.limit_sentence()
        )
    }
}

/// The smallest of 1, 2, 2.5, 5 × 10ⁿ at or above `x` (1 for non-positive or non-finite `x`).
pub fn nice_ceil(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 1.0;
    }
    let p = 10f64.powf(x.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        let v = m * p;
        if v >= x * (1.0 - 1e-12) {
            return clean(v);
        }
    }
    clean(10.0 * p)
}

/// Rounds away float noise (`0.30000000000000004` → `0.3`).
pub fn clean(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let mag = 10f64.powf(9.0 - v.abs().log10().floor());
    (v * mag).round() / mag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strain_format() {
        assert_eq!(fmt_strain(22.0), "22");
        assert_eq!(fmt_strain(0.125), "0.13");
        assert_eq!(fmt_strain(0.008333), "0.0083");
        assert_eq!(fmt_strain(0.0), "0");
        assert_eq!(fmt_strain(f64::NAN), "0");
    }

    #[test]
    fn nice_numbers() {
        assert_eq!(nice_ceil(24.2), 25.0);
        assert_eq!(nice_ceil(0.385), 0.5);
        assert_eq!(nice_ceil(0.00917), 0.01);
        assert_eq!(nice_ceil(4.95), 5.0);
        assert_eq!(nice_ceil(0.0), 1.0);
    }
}
