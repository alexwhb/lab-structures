//! FIRE-1: fire — a Conditions row (Off / Small 400 °C / Big 700 °C / Inferno 900 °C) that
//! heats the pieces near the fire and takes away their strength and stiffness by the published
//! temperature reduction factors.
//!
//! - [`factors`] — the reduction tables: steel and cable per EN 1993-1-2 Table 3.1, concrete
//!   (and Road / Floor) per EN 1992-1-2 Table 3.1 (siliceous), wood and rope (illustrative).
//! - [`heat`] — where the fire burns (river: on the water under mid-span; open ground: at the
//!   tower's base), its reach growing over the first seconds ([`Blaze`]), each piece's
//!   temperature with a lag of a few seconds.
//! - [`update`] — the builder core calls it once per running frame: it lights the fire on
//!   time, heats and cools every piece (`Constraint::temp`) and sets its strength and
//!   stiffness factors (`Constraint::strength`, `Constraint::stiffness`) and the break mode the
//!   heat gives it (`Constraint::heat_mode`).
//! - [`FireBreak`] — the new break modes "softened" (steel, cable) and "burned through" (wood,
//!   rope), [`break_mode`] and their [`tip`]s.
//!
//! The fire's state lives in the simulation (`Sim::fire_start`, the per-piece fields), so a run
//! replays exactly. Sources: `docs/DATA-SOURCES.md` § Fire.
//!
//! Owner: agent `lab` (PROGRESS.md).

pub mod factors;
pub mod heat;

pub use factors::{AMBIENT_C, Factors, FireMaterial, factors_for};
pub use heat::{Blaze, FIRE_START_S, source_for};

use crate::builder::data::{PieceType, Scene};
use crate::builder::sim::{BreakMode, Constraint, Sim};

/// The Fire condition (BLD-12 / FIRE-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum FireLevel {
    /// No fire.
    #[default]
    Off,
    /// About 400 °C: a car or a small fuel spill.
    Small,
    /// About 700 °C: a burning truck.
    Big,
    /// About 900 °C: a fuel tanker.
    Inferno,
}

impl FireLevel {
    /// Every level, in the Conditions row's order.
    pub const ALL: [FireLevel; 4] = [
        FireLevel::Off,
        FireLevel::Small,
        FireLevel::Big,
        FireLevel::Inferno,
    ];

    /// The chip's name: "Off", "Small", "Big", "Inferno".
    pub fn name(self) -> &'static str {
        match self {
            FireLevel::Off => "Off",
            FireLevel::Small => "Small",
            FireLevel::Big => "Big",
            FireLevel::Inferno => "Inferno",
        }
    }

    /// A stable key: `off`, `small`, `big`, `inferno` (MCP, Share comments).
    pub fn key(self) -> &'static str {
        match self {
            FireLevel::Off => "off",
            FireLevel::Small => "small",
            FireLevel::Big => "big",
            FireLevel::Inferno => "inferno",
        }
    }

    /// From a key or a name (any case), or a temperature in °C ("700", "700 °C", "0").
    pub fn parse(s: &str) -> Option<FireLevel> {
        let k = s.trim().to_ascii_lowercase();
        let k = k.trim_end_matches("°c").trim_end_matches('c').trim();
        if let Some(l) = FireLevel::ALL.into_iter().find(|l| l.key() == k) {
            return Some(l);
        }
        match k.parse::<f64>().ok()? {
            0.0 => Some(FireLevel::Off),
            t => FireLevel::ALL
                .into_iter()
                .find(|l| l.is_on() && l.temp_c() == t),
        }
    }

    /// The fire's temperature once it has grown, °C (20 °C when off).
    pub fn temp_c(self) -> f64 {
        match self {
            FireLevel::Off => AMBIENT_C,
            FireLevel::Small => 400.0,
            FireLevel::Big => 700.0,
            FireLevel::Inferno => 900.0,
        }
    }

    /// How far it reaches once it has grown, m of plume distance (illustrative).
    pub fn reach_m(self) -> f64 {
        match self {
            FireLevel::Off => 0.0,
            FireLevel::Small => 8.0,
            FireLevel::Big => 12.0,
            FireLevel::Inferno => 16.0,
        }
    }

    /// Anything but Off.
    pub fn is_on(self) -> bool {
        self != FireLevel::Off
    }
}

impl FireLevel {
    /// The level a Fire setting in °C is (`Conditions::fire`): the nearest level at or below
    /// it (anything above 0 is at least Small).
    pub fn from_temp(c: f64) -> FireLevel {
        let c = heat::sanitize_fire_c(c);
        if c <= 0.0 {
            FireLevel::Off
        } else if c < FireLevel::Big.temp_c() {
            FireLevel::Small
        } else if c < FireLevel::Inferno.temp_c() {
            FireLevel::Big
        } else {
            FireLevel::Inferno
        }
    }

    /// The `Conditions::fire` value for this level, °C (0 = off).
    pub fn setting(self) -> f64 {
        if self.is_on() { self.temp_c() } else { 0.0 }
    }
}

/// The single strength factor the builder applies to a piece (`Constraint::strength`): the
/// smaller of the two for steel, cable, wood and rope (they are equal), and for concrete, Road
/// and Floor the compressive factor (EN 1992-1-2 Table 3.1) — concrete pieces work in
/// compression; the sample drawer uses the same factor so its "breaks at" equals the builder's.
pub fn strength_factor(m: FireMaterial, f: &Factors) -> f64 {
    match m {
        FireMaterial::Concrete => f.strength_c,
        _ => f.strength(),
    }
}

/// Sets a piece's factors from its temperature: steel and cable follow the temperature now
/// (they recover as they cool), the others never get back what they lost (charred, melted,
/// cracked). Also sets the break mode the heat gives it.
pub fn apply(c: &mut Constraint) {
    let m = FireMaterial::of(c.kind);
    let f = m.factors(c.temp);
    let (s, k) = (strength_factor(m, &f), f.stiffness);
    if m.damage_is_permanent() {
        c.strength = c.strength.min(s);
        c.stiffness = c.stiffness.min(k);
    } else {
        c.strength = s;
        c.stiffness = k;
    }
    c.heat_mode = break_mode(c.kind, c.strength).map(BreakMode::from);
}

/// Where the fire burns in this simulation (see [`heat::source_for`]): under mid-span on the
/// river; on open ground the mean x of the intact foundations that carry a piece (a level's
/// unused anchors don't count), else of the used joints at ground level, else the default
/// tower's centre line.
pub fn source(sim: &Sim) -> [f64; 2] {
    if sim.setup.scene == Scene::River {
        return heat::RIVER_SOURCE;
    }
    let mean = |pick: &dyn Fn(&crate::builder::sim::Particle) -> bool| {
        let (sum, n) = sim
            .p
            .iter()
            .filter(|p| pick(p) && p.x0.is_finite())
            .fold((0.0, 0usize), |(s, n), p| (s + p.x0, n + 1));
        (n > 0).then(|| sum / n as f64)
    };
    // only foundations that carry something: a level's unused anchors don't count
    let x = mean(&|p| p.sup.is_some() && !p.ripped && p.used)
        .or_else(|| mean(&|p| p.used && p.y0.abs() < 0.5))
        .unwrap_or(heat::FLAT_DEFAULT_X);
    [x, 0.0]
}

/// The fire in this simulation now.
pub fn blaze(sim: &Sim) -> Blaze {
    Blaze::at(sim.cond.fire, sim.fire_start, sim.t, source(sim))
}

/// FIRE-1, once per running frame of `dt` seconds (the builder core calls it before the frame's
/// substeps). Lights the fire on time — [`FIRE_START_S`] into a run when Fire was set before it,
/// [`heat::CHANGE_DELAY_S`] after it is switched on mid-run; Off puts it out — then heats every
/// intact piece toward its exposure and cools the rest, and updates their factors ([`apply`]).
/// Does nothing on a static settle (the build-mode preview), or while Fire is off and nothing
/// is hot.
pub fn update(sim: &mut Sim, dt: f64) {
    if !sim.live {
        return;
    }
    let on = heat::sanitize_fire_c(sim.cond.fire) > 0.0;
    if !on {
        sim.fire_start = None;
    } else if sim.fire_start.is_none() {
        sim.fire_start = Some(if sim.t <= 0.0 {
            FIRE_START_S
        } else {
            sim.t + heat::CHANGE_DELAY_S
        });
    }
    let cold = |c: &Constraint| c.temp <= AMBIENT_C + heat::COLD_MARGIN_C;
    if heat::growth(sim.t, sim.fire_start, on) <= 0.0 && sim.c.iter().all(cold) {
        return;
    }
    let b = blaze(sim);
    let p = &sim.p;
    for c in sim.c.iter_mut().filter(|c| !c.broken) {
        let (Some(pa), Some(pb)) = (p.get(c.a), p.get(c.b)) else {
            continue;
        };
        let target = b.target([pa.x, pa.y], [pb.x, pb.y]);
        c.temp = heat::approach(c.temp, target, dt, FireMaterial::of(c.kind));
        apply(c);
    }
}

/// "Light it now": a fire that is set but not burning yet lights [`heat::LIGHT_DELAY_S`] from
/// now. Does nothing while Fire is off, already burning or on a static settle.
pub fn light_now(sim: &mut Sim) {
    if can_light(sim) {
        sim.fire_start = Some(sim.t + heat::LIGHT_DELAY_S);
    }
}

/// Whether "Light it now" would do something: a running simulation with Fire set and not yet
/// burning.
pub fn can_light(sim: &Sim) -> bool {
    sim.live
        && heat::sanitize_fire_c(sim.cond.fire) > 0.0
        && !sim.fire_start.is_some_and(|l| sim.t >= l)
}

/// The hottest intact piece now (index, °C), if any is above ambient.
pub fn hottest(sim: &Sim) -> Option<(usize, f64)> {
    sim.c
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.broken && c.temp > AMBIENT_C + heat::COLD_MARGIN_C)
        .fold(None, |best: Option<(usize, f64)>, (i, c)| match best {
            Some((_, t)) if t >= c.temp => best,
            _ => Some((i, c.temp)),
        })
}

impl From<FireBreak> for BreakMode {
    fn from(f: FireBreak) -> BreakMode {
        match f {
            FireBreak::Softened => BreakMode::Softened,
            FireBreak::BurnedThrough => BreakMode::BurnedThrough,
        }
    }
}

/// The two ways the fire breaks a piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FireBreak {
    /// Steel or cable lost so much strength in the heat that the load was too much.
    Softened,
    /// Wood charred through or rope melted.
    BurnedThrough,
}

impl FireBreak {
    /// The break mode's words in the result sentence: "softened" / "burned through".
    pub fn name(self) -> &'static str {
        match self {
            FireBreak::Softened => "softened",
            FireBreak::BurnedThrough => "burned through",
        }
    }
}

/// A piece counts as broken by the fire when less than this much of its strength is left.
pub const WEAKENED_BELOW: f64 = 0.9;

/// How the fire broke a piece of type `t` that broke with `strength` (its smaller strength
/// factor) left: [`FireBreak::Softened`] for steel and cable, [`FireBreak::BurnedThrough`] for
/// wood and rope, when the fire had taken more than a tenth of it; `None` otherwise (concrete,
/// Road and Floor keep their usual "cracked" / "crushed").
pub fn break_mode(t: PieceType, strength: f64) -> Option<FireBreak> {
    if strength.is_nan() || strength >= WEAKENED_BELOW {
        return None;
    }
    match FireMaterial::of(t) {
        FireMaterial::Steel | FireMaterial::Cable => Some(FireBreak::Softened),
        FireMaterial::Wood | FireMaterial::Rope => Some(FireBreak::BurnedThrough),
        FireMaterial::Concrete => None,
    }
}

/// The result banner's tip for a piece of type `t` the fire broke.
pub fn tip(mode: FireBreak, t: PieceType) -> &'static str {
    match (mode, FireMaterial::of(t)) {
        (FireBreak::Softened, FireMaterial::Cable) => {
            "Cable wire softens like steel: about half its strength is gone at 600 \u{b0}C. Keep cables out of the fire\u{2019}s reach."
        }
        (FireBreak::Softened, _) => {
            "Steel keeps only about half its strength at 600 \u{b0}C. Real steel frames are wrapped in fireproofing."
        }
        (FireBreak::BurnedThrough, FireMaterial::Rope) => {
            "Rope melts and burns through by about 250 \u{b0}C. Keep it away from the fire, or use steel Cable."
        }
        (FireBreak::BurnedThrough, _) => {
            "Wood chars from the outside in and has nothing left by about 300 \u{b0}C. Real timber is made thick enough to keep a sound core, or boarded over."
        }
    }
}

/// The HUD chip's value while the fire burns: "650 °C" (`None` before it lights).
pub fn hud_value(sim: &Sim) -> Option<String> {
    let b = blaze(sim);
    b.burning()
        .then(|| format!("{} \u{b0}C", crate::jsnum::to_fixed(b.temp_c, 0)))
}

/// The note under the Fire chips: where it burns and when it lights.
pub fn note(level: FireLevel, river: bool) -> String {
    note_at(level.setting(), river)
}

/// The level whose setting is exactly `fire_c` °C (`None` for a setting between the chips, as
/// MCP may set: then no chip is lit).
pub fn exact_level(fire_c: f64) -> Option<FireLevel> {
    FireLevel::ALL
        .into_iter()
        .find(|l| (l.setting() - fire_c).abs() < 1e-9)
}

/// The note for a Fire setting of `fire_c` °C (0 = off), any value: "A big fire at the foot of
/// the tower, about 700 °C. …", or "A fire … about 650 °C. …" between the chips.
pub fn note_at(fire_c: f64, river: bool) -> String {
    let c = heat::sanitize_fire_c(fire_c);
    if c <= 0.0 {
        return "Set a fire to see what heat does to each material.".into();
    }
    let place = if river {
        "on the water under the middle of the span"
    } else {
        "at the foot of the tower"
    };
    let size = match exact_level(c) {
        Some(l) => format!("{} ", l.name().to_lowercase()),
        None => String::new(),
    };
    format!(
        "A {size}fire {place}, about {} \u{b0}C. It lights {} s into the run.",
        crate::jsnum::to_fixed(c, 0),
        crate::jsnum::to_string(heat::FIRE_START_S)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_parse() {
        for l in FireLevel::ALL {
            assert_eq!(FireLevel::parse(l.key()), Some(l));
            assert_eq!(FireLevel::parse(l.name()), Some(l));
        }
        assert_eq!(FireLevel::parse("700"), Some(FireLevel::Big));
        assert_eq!(FireLevel::parse("900 °C"), Some(FireLevel::Inferno));
        assert_eq!(FireLevel::parse("0"), Some(FireLevel::Off));
        assert_eq!(FireLevel::parse("650"), None);
        assert_eq!(FireLevel::parse("lava"), None);
    }

    #[test]
    fn break_modes() {
        assert_eq!(
            break_mode(PieceType::Steel, 0.47),
            Some(FireBreak::Softened)
        );
        assert_eq!(break_mode(PieceType::Steel, 0.95), None);
        assert_eq!(
            break_mode(PieceType::Rope, 0.0),
            Some(FireBreak::BurnedThrough)
        );
        assert_eq!(break_mode(PieceType::Road, 0.1), None);
        assert_eq!(break_mode(PieceType::Wood, f64::NAN), None);
        assert_eq!(FireBreak::BurnedThrough.name(), "burned through");
    }

    #[test]
    fn notes_follow_the_setting() {
        assert_eq!(
            note(FireLevel::Big, false),
            "A big fire at the foot of the tower, about 700 \u{b0}C. It lights 3 s into the run."
        );
        assert_eq!(
            note_at(650.0, true),
            "A fire on the water under the middle of the span, about 650 \u{b0}C. It lights 3 s into the run."
        );
        assert_eq!(exact_level(650.0), None);
        assert_eq!(exact_level(0.0), Some(FireLevel::Off));
        assert!(note_at(f64::NAN, true).starts_with("Set a fire"));
    }
}
