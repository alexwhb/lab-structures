//! The tensile test bench: what the Materials screen's controls do (mockup `state`, `tick()`
//! and the handlers in `renderVals()`), the status and explain sentences, and the Share line.

use super::curves::{
    Curve, HOT_C, Kind, ROOM_TEMP_C, Stress, TEMP_MAX_C, TEMP_MIN_C, sanitize_strain, sanitize_temp,
};
use super::jsnum;
use serde::{Deserialize, Serialize};

/// The mockup advances the Pull every 50 ms (`setInterval(…, 50)`): 20 ticks per second.
pub const TICK_SECONDS: f64 = 0.05;
/// Most ticks one [`Bench::advance`] runs (a long stall must not spin): more than a full pull
/// on either axis (25 / 0.08 ≈ 313, 2 / 0.006 ≈ 334).
pub const MAX_TICKS_PER_ADVANCE: usize = 400;
/// The challenge's place on the bench's card (`Challenge · Break all five` · `5 / 12`, a fixed
/// label in the mockup).
pub const CHALLENGE_RANK: (u32, u32) = (5, 12);

/// The strain axis range (mockup `zoom`: 25 or 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Zoom {
    /// 0 – 25 %: the metals' whole curves.
    #[default]
    Wide,
    /// 0 – 2 %: the brittle materials and the metals' elastic start.
    Close,
}

impl Zoom {
    /// Both, in the segmented control's order.
    pub const ALL: [Zoom; 2] = [Zoom::Wide, Zoom::Close];

    /// The axis end, % strain.
    pub fn max(self) -> f64 {
        match self {
            Zoom::Wide => 25.0,
            Zoom::Close => 2.0,
        }
    }

    /// Strain added per Pull tick, % (0.08 at 25 %, 0.006 at 2 %).
    pub fn pull_step(self) -> f64 {
        match self {
            Zoom::Wide => 0.08,
            Zoom::Close => 0.006,
        }
    }

    /// The Stretch slider's step, % (0.05 / 0.005).
    pub fn slider_step(self) -> f64 {
        match self {
            Zoom::Wide => 0.05,
            Zoom::Close => 0.005,
        }
    }

    /// Fraction digits of the Stretch readout (2 / 3).
    pub fn decimals(self) -> usize {
        match self {
            Zoom::Wide => 2,
            Zoom::Close => 3,
        }
    }

    /// The segmented control's label ("0 – 25 %", "0 – 2 %").
    pub fn label(self) -> &'static str {
        match self {
            Zoom::Wide => "0 \u{2013} 25 %",
            Zoom::Close => "0 \u{2013} 2 %",
        }
    }

    /// The strain axis ticks, % (`[0, 5, …, 25]` / `[0, 0.5, …, 2]`).
    pub fn ticks(self) -> &'static [f64] {
        match self {
            Zoom::Wide => &[0.0, 5.0, 10.0, 15.0, 20.0, 25.0],
            Zoom::Close => &[0.0, 0.5, 1.0, 1.5, 2.0],
        }
    }

    /// The axis that shows a sample's whole curve: 2 % when it breaks before 2 %, else 25 %
    /// (picking a sample switches to it).
    pub fn fitting(kind: Kind) -> Zoom {
        if kind.curve().break_strain() < 2.0 {
            Zoom::Close
        } else {
            Zoom::Wide
        }
    }

    /// Parses `25`, `2`, `"wide"`, `"close"`, `"0-25"`, … (MCP, Share lines).
    pub fn parse(s: &str) -> Option<Zoom> {
        let k: String = s
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '%')
            .flat_map(char::to_lowercase)
            .collect();
        match k.as_str() {
            "25" | "wide" | "0-25" | "0\u{2013}25" => Some(Zoom::Wide),
            "2" | "close" | "0-2" | "0\u{2013}2" => Some(Zoom::Close),
            _ => None,
        }
    }
}

/// The colour role of a status pill or readout (the screen maps it to `sp_theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    /// `#A3A3A3` (ready; unlit challenge chips use `#6A6A6A`).
    Muted,
    /// `#5FBF8A` (elastic; a lit challenge chip).
    Ok,
    /// `#D6C7A1` (plastic; room-temperature readout).
    Accent,
    /// `#E7A47C` (necking; hot readout).
    Caution,
    /// `#E5534B` (fractured).
    Danger,
}

/// Where the sample is on its curve (the status pill).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// Not stretched yet ("Ready").
    Ready,
    /// On the first straight line ("Elastic: springs back").
    Elastic,
    /// Past the first line ("Plastic: permanently stretched").
    Plastic,
    /// Past the ultimate strain, thinning ("Necking").
    Necking,
    /// Broken ("Fractured at 22%").
    Fractured {
        /// The strain at fracture, %.
        at: f64,
    },
}

impl Status {
    /// The pill's text.
    pub fn label(&self) -> String {
        match self {
            Status::Ready => "Ready".into(),
            Status::Elastic => "Elastic: springs back".into(),
            Status::Plastic => "Plastic: permanently stretched".into(),
            Status::Necking => "Necking".into(),
            Status::Fractured { at } => format!("Fractured at {}%", jsnum::to_string(*at)),
        }
    }

    /// The pill's colour role.
    pub fn tone(&self) -> Tone {
        match self {
            Status::Ready => Tone::Muted,
            Status::Elastic => Tone::Ok,
            Status::Plastic => Tone::Accent,
            Status::Necking => Tone::Caution,
            Status::Fractured { .. } => Tone::Danger,
        }
    }
}

/// The bench (mockup `this.state`: `mat`, `e`, `temp`, `zoom`, `playing`, `broke`).
///
/// Invariants: `0 ≤ strain ≤ zoom.max()`, `TEMP_MIN_C ≤ temp ≤ TEMP_MAX_C`, all finite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bench {
    kind: Kind,
    strain: f64,
    temp: f64,
    zoom: Zoom,
    playing: bool,
    broke: [bool; 5],
    #[serde(skip)]
    acc: f64,
}

impl Default for Bench {
    /// Steel, unstretched, 20 °C, 0–25 % axis, paused, nothing broken yet.
    fn default() -> Self {
        Self {
            kind: Kind::Steel,
            strain: 0.0,
            temp: ROOM_TEMP_C,
            zoom: Zoom::Wide,
            playing: false,
            broke: [false; 5],
            acc: 0.0,
        }
    }
}

impl Bench {
    /// The selected sample.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Its curve.
    pub fn curve(&self) -> &'static Curve {
        self.kind.curve()
    }

    /// The stretch, % strain.
    pub fn strain(&self) -> f64 {
        self.strain
    }

    /// The temperature, °C.
    pub fn temp(&self) -> f64 {
        self.temp
    }

    /// The strain axis.
    pub fn zoom(&self) -> Zoom {
        self.zoom
    }

    /// True while Pull is running.
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// True once `kind` has been pulled to fracture (since the last Reset).
    pub fn broke(&self, kind: Kind) -> bool {
        self.broke[kind.index()]
    }

    /// How many of the five have been broken.
    pub fn broke_count(&self) -> usize {
        self.broke.iter().filter(|b| **b).count()
    }

    /// The challenge is done: all five broken.
    pub fn all_broken(&self) -> bool {
        self.broke.iter().all(|b| *b)
    }

    /// The stress now (mockup `now`).
    pub fn now(&self) -> Stress {
        self.curve().stress_at(self.strain, self.temp)
    }

    /// The temperature strength factor now.
    pub fn temp_factor(&self) -> f64 {
        super::curves::temp_factor(self.kind, self.temp)
    }

    /// The necking amount now (0 – 0.55).
    pub fn necking(&self) -> f64 {
        self.curve().necking(self.strain)
    }

    /// The load cell force now, kN.
    pub fn force_kn(&self) -> f64 {
        super::curves::load_cell_kn(self.now().mpa)
    }

    /// Picks a sample (a sample tile): a fresh, paused sample on the axis that fits it.
    pub fn pick(&mut self, kind: Kind) {
        self.kind = kind;
        self.strain = 0.0;
        self.playing = false;
        self.zoom = Zoom::fitting(kind);
    }

    /// Sets the temperature (the slider), clamped to −40…600 °C; NaN is ignored.
    pub fn set_temp(&mut self, t_c: f64) {
        if t_c.is_nan() {
            return;
        }
        self.temp = sanitize_temp(t_c).clamp(TEMP_MIN_C, TEMP_MAX_C);
    }

    /// Changes the strain axis; the stretch is clipped to the new end. Pull keeps running.
    pub fn set_zoom(&mut self, zoom: Zoom) {
        self.zoom = zoom;
        self.strain = self.strain.min(zoom.max());
    }

    /// Scrubs the stretch (the Stretch slider), clamped to the axis; pauses Pull. NaN and
    /// negative values count as 0.
    pub fn set_strain(&mut self, strain: f64) {
        self.strain = sanitize_strain(strain).min(self.zoom.max());
        self.playing = false;
    }

    /// "Fresh sample": unstretched and paused.
    pub fn fresh_sample(&mut self) {
        self.strain = 0.0;
        self.playing = false;
    }

    /// The drive button (Pull / Pause). Pulling a broken sample, or one at the axis end,
    /// starts again from zero.
    pub fn toggle_drive(&mut self) {
        let start = !self.playing;
        if start && (self.now().broken || self.strain >= self.zoom.max()) {
            self.strain = 0.0;
        }
        self.playing = start;
        self.acc = 0.0;
    }

    /// Reset: back to the defaults, challenge progress cleared.
    pub fn reset(&mut self) {
        *self = Bench::default();
    }

    /// One Pull step (mockup `tick()`): adds the axis's step; stops at fracture (marking the
    /// sample broken for the challenge) or at the axis end. Does nothing while paused. Returns
    /// the sample if this step broke it.
    pub fn tick(&mut self) -> Option<Kind> {
        if !self.playing {
            return None;
        }
        let e = self.zoom.max().min(self.strain + self.zoom.pull_step());
        let r = self.curve().stress_at(e, self.temp);
        self.strain = e;
        if r.broken || e >= self.zoom.max() {
            self.playing = false;
            if r.broken {
                self.broke[self.kind.index()] = true;
                return Some(self.kind);
            }
        }
        None
    }

    /// Lets `dt` seconds pass: runs one [`Bench::tick`] per [`TICK_SECONDS`] while playing (at
    /// most [`MAX_TICKS_PER_ADVANCE`]). Non-finite or negative `dt` does nothing. Returns the
    /// sample if it broke.
    pub fn advance(&mut self, dt: f64) -> Option<Kind> {
        if !self.playing {
            self.acc = 0.0;
            return None;
        }
        if dt.is_finite() && dt > 0.0 {
            self.acc = (self.acc + dt).min(TICK_SECONDS * MAX_TICKS_PER_ADVANCE as f64);
        }
        let mut broke = None;
        let mut n = 0;
        // A nanosecond of slack so 20 frames of 50 ms are 20 steps despite float rounding.
        while self.playing && self.acc + 1e-9 >= TICK_SECONDS && n < MAX_TICKS_PER_ADVANCE {
            self.acc = (self.acc - TICK_SECONDS).max(0.0);
            n += 1;
            if let Some(k) = self.tick() {
                broke = Some(k);
            }
        }
        if !self.playing {
            self.acc = 0.0;
        }
        broke
    }

    /// Seconds until the next Pull step is due (for paced repaints), or `None` while paused.
    pub fn next_tick_in(&self) -> Option<f64> {
        self.playing
            .then(|| (TICK_SECONDS - self.acc).clamp(0.0, TICK_SECONDS))
    }

    /// The status pill (mockup `statusLabel`).
    pub fn status(&self) -> Status {
        let c = self.curve();
        let now = self.now();
        let mut s = Status::Elastic;
        if self.strain > c.elastic_limit() && c.has_plastic_range() {
            s = Status::Plastic;
        }
        if self.necking() > 0.02 {
            s = Status::Necking;
        }
        if now.broken {
            s = Status::Fractured {
                at: now.break_strain,
            };
        }
        if self.strain == 0.0 {
            s = Status::Ready;
        }
        s
    }

    /// The "What's happening" sentence (mockup `explain`).
    pub fn explain(&self) -> String {
        let c = self.curve();
        let now = self.now();
        let plastic = c.has_plastic_range();
        let mut x: String = "At first the sample behaves like a stiff spring: let go and it returns to its length. The slope of that first straight line is the stiffness, E. Press Play to keep pulling.".into();
        if self.strain > c.elastic_limit() && !now.broken && plastic {
            x = "Past the yield point the atoms start slipping past each other. The metal stretches a lot for little extra force, and it will not spring back fully. That slack is what lets steel bend instead of snap, a warning before failure.".into();
        }
        if self.necking() > 0.02 && !now.broken {
            x = "The sample has passed its peak strength. One spot thins out (necking), all the stretching concentrates there, and the force it can carry drops until it tears.".into();
        }
        if now.broken {
            let brk = jsnum::to_string(now.break_strain);
            x = if plastic {
                format!(
                    "It tore after stretching {brk}%. The area under the curve is the energy it soaked up first: its toughness."
                )
            } else {
                format!(
                    "It snapped at {brk}% with no yielding and no warning. Strong but brittle: tiny area under the curve, so little energy absorbed."
                )
            };
            x.push_str(" Try another material, or heat it up.");
        }
        if self.temp >= HOT_C
            && matches!(self.kind, Kind::Aluminum | Kind::CarbonFiber | Kind::Timber)
        {
            x.push_str(&format!(
                " At {}\u{b0}C this one has lost much of its strength.",
                jsnum::to_string(self.temp)
            ));
        }
        x
    }

    /// NAV-6 Share: a one-line `.material` summary (Series "Netlist directives": `.material name
    /// E fy rho [curve=…]`), e.g. `.material steel E=200GPa fy=275MPa rho=7850 curve=steel ;
    /// tensile test at 20 °C, ε = 0.00 %, Ready`. `fy` is the yield at this temperature, or the
    /// peak strength for the brittle materials.
    pub fn share(&self) -> String {
        let c = self.curve();
        let fy = c
            .yield_at(self.temp)
            .unwrap_or_else(|| c.peak_mpa(self.temp));
        format!(
            ".material {key} E={e}GPa fy={fy}MPa rho={rho} curve={key} ; tensile test at {t} \u{b0}C, \u{3b5} = {eps} %, {status}",
            key = self.kind.key(),
            e = jsnum::to_string(c.e_gpa),
            fy = jsnum::to_fixed(fy, 0),
            rho = jsnum::to_string(c.rho),
            t = jsnum::to_string(self.temp),
            eps = jsnum::to_fixed(self.strain, 2),
            status = self.status().label(),
        )
    }

    /// The header clock, "ε = 0.00 %" (also [`super::View::clock`]).
    pub fn clock(&self) -> String {
        format!("\u{3b5} = {} %", jsnum::to_fixed(self.strain, 2))
    }

    /// A one-line description for accessibility and the MCP `describe_screen` tool.
    pub fn describe(&self) -> String {
        format!(
            "Materials lab: {} at {} \u{b0}C, stretched {} %, {} MPa, {}. Broken so far: {} of 5.",
            self.kind.name(),
            jsnum::to_string(self.temp),
            jsnum::to_fixed(self.strain, 2),
            jsnum::to_fixed(self.now().mpa, 0),
            self.status().label(),
            self.broke_count()
        )
    }
}
