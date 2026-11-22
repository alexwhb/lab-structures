//! Everything the Materials screen shows for a bench state (mockup `renderVals()`): readout
//! strings formatted exactly like the mockup's JavaScript, and the drawing model of the
//! tensile rig and the stress–strain plot in the mockup's SVG viewBox units (the rig is
//! `0 0 420 560`, the plot `0 0 560 400`). The screen scales those into its rectangles; the
//! goldens compare them with the mockup's path strings.

use super::bench::{Bench, CHALLENGE_RANK, Status, Tone, Zoom};
use super::curves::{HOT_C, Kind, SECTION_MM2, STRESS_AXIS_MAX_MPA, load_cell_kn, temp_factor};
use super::jsnum::{round, to_fixed, to_string};

/// The plot's viewBox, px (`0 0 560 400`).
pub const PLOT_VIEW: [f64; 2] = [560.0, 400.0];
/// The plot area inside it: x from 56 to 544, y from 350 (0 MPa) up to 20 (700 MPa).
pub const PLOT_X0: f64 = 56.0;
/// See [`PLOT_X0`].
pub const PLOT_X1: f64 = 544.0;
/// See [`PLOT_X0`].
pub const PLOT_Y0: f64 = 350.0;
/// See [`PLOT_X0`].
pub const PLOT_Y1: f64 = 20.0;
/// Stress axis ticks, MPa.
pub const STRESS_TICKS: [f64; 7] = [100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0];
/// The rig's viewBox, px (`0 0 420 560`).
pub const RIG_VIEW: [f64; 2] = [420.0, 560.0];
/// The sample's centre line, x in the rig viewBox.
pub const RIG_CX: f64 = 210.0;
/// Drawn stretch: rig px per % strain (`ext = e × 4.5`, "stretch exaggerated").
pub const RIG_PX_PER_PERCENT: f64 = 4.5;
/// The unstretched sample's top (upper grip end) and the fixed bottom, rig px.
pub const RIG_TOP_Y: f64 = 190.0;
/// See [`RIG_TOP_Y`].
pub const RIG_BOT_Y: f64 = 452.0;
/// Grip half-width and gauge half-width of the dog-bone, rig px.
pub const RIG_GRIP_HALF: f64 = 24.0;
/// See [`RIG_GRIP_HALF`].
pub const RIG_GAUGE_HALF: f64 = 13.0;

/// A point in viewBox px.
pub type Pt = [f64; 2];

/// One readout tile (label, value, note).
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    /// `.lbl` ("Stiffness E").
    pub key: &'static str,
    /// Mono value ("200 GPa").
    pub value: String,
    /// Muted note ("slope of the first line"; may be empty).
    pub note: &'static str,
}

/// One "Strength per kilogram" bar.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecBar {
    /// Which material.
    pub kind: Kind,
    /// Its name.
    pub name: &'static str,
    /// Specific strength at the bench temperature, kN·m/kg.
    pub value: f64,
    /// The mono value ("55").
    pub label: String,
    /// Bar fill, 0–1 of the track (whole percent, as the mockup's `width: N%`).
    pub fill: f64,
    /// The selected sample (sand bar, bright bold name).
    pub selected: bool,
}

/// A "Break all five" chip.
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeChip {
    /// Which material.
    pub kind: Kind,
    /// Its name.
    pub name: &'static str,
    /// Lit (green) once broken.
    pub lit: bool,
}

/// An axis tick label at a position along its axis (viewBox px: y for stress, x for strain).
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    /// The label ("100", "0.5").
    pub label: String,
    /// Position along the axis, viewBox px.
    pub pos: f64,
}

/// A curve's name label, anchored at its last corner on the axis (text starts 4 px right of
/// the anchor, its bottom just above it).
#[derive(Debug, Clone, PartialEq)]
pub struct CurveLabel {
    /// Which material.
    pub kind: Kind,
    /// Its name.
    pub name: &'static str,
    /// The anchor, plot viewBox px.
    pub at: Pt,
    /// The selected sample (`#F5F5F5`, weight 600) or a ghost (`#8A8A8A`, 400).
    pub selected: bool,
}

/// The stress–strain plot in viewBox px (`0 0 560 400`).
#[derive(Debug, Clone, PartialEq)]
pub struct PlotGeometry {
    /// The axes polyline `M56 20 V350 H544`.
    pub axes: [Pt; 3],
    /// Grid lines (stress every 100 MPa, strain at the ticks).
    pub grid: Vec<[Pt; 2]>,
    /// The other four materials' curves (ghosts), each a polyline.
    pub ghosts: Vec<(Kind, Vec<Pt>)>,
    /// The selected material's curve (dropping to the axis at fracture when on the axis).
    pub curve: Vec<Pt>,
    /// The done-so-far trace (empty before the first stretch).
    pub done: Vec<Pt>,
    /// The marker (sand dot) at the current strain and stress.
    pub marker: Pt,
    /// Stress tick labels (`pos` = y).
    pub ticks_y: Vec<Tick>,
    /// Strain tick labels (`pos` = x).
    pub ticks_x: Vec<Tick>,
    /// The five curve name labels.
    pub labels: Vec<CurveLabel>,
}

/// The tensile rig's moving parts in viewBox px (`0 0 420 560`); the frame is static.
#[derive(Debug, Clone, PartialEq)]
pub struct RigGeometry {
    /// Top of the sample (rises as it stretches), the fixed bottom and the gauge middle.
    pub top_y: f64,
    /// See [`RigGeometry::top_y`].
    pub bot_y: f64,
    /// See [`RigGeometry::top_y`].
    pub mid_y: f64,
    /// The crosshead: its block `[x0, y0, x1, y1]` and the beam through the columns.
    pub head: [[f64; 4]; 2],
    /// The crosshead block's top face (2.5D).
    pub head_top: [Pt; 4],
    /// The dog-bone outline: one polygon, or two halves once fractured.
    pub specimen: Vec<Vec<Pt>>,
    /// The extensometer: its bar and two knife edges.
    pub gauge: [[Pt; 2]; 3],
    /// The red crack line at the break (fractured only).
    pub crack: Option<[Pt; 4]>,
    /// Necking amount (0 – 0.55).
    pub necking: f64,
}

/// Everything shown for a bench state (mockup `renderVals()`).
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    /// The selected sample.
    pub kind: Kind,
    /// Status.
    pub status: Status,
    /// The status pill's text.
    pub status_label: String,
    /// The status pill's colour role.
    pub status_tone: Tone,
    /// "What's happening".
    pub explain: String,
    /// The sample note under the tiles.
    pub note: &'static str,
    /// "20 °C".
    pub temp_label: String,
    /// The temperature readout's colour role (accent, caution from 200 °C).
    pub temp_tone: Tone,
    /// "Room-temperature strength." / "Strength now 74% of room temperature (illustrative)."
    pub temp_note: String,
    /// The Stretch readout "0.00 % · 0 MPa".
    pub strain_label: String,
    /// The header clock "ε = 0.00 %".
    pub clock: String,
    /// The load cell "0.0 kN".
    pub force_label: String,
    /// "50 mm² section".
    pub section_label: String,
    /// The six readout tiles.
    pub tiles: Vec<Tile>,
    /// Strength per kilogram bars.
    pub spec: Vec<SpecBar>,
    /// Challenge chips.
    pub challenge: Vec<ChallengeChip>,
    /// The challenge card's rank label ("5 / 12").
    pub challenge_rank: String,
    /// The rig's accessible label.
    pub rig_label: String,
    /// The plot's accessible label.
    pub plot_label: String,
    /// The plot drawing.
    pub plot: PlotGeometry,
    /// The rig drawing.
    pub rig: RigGeometry,
}

/// Plot x for a strain on this axis (mockup `gx`).
pub fn plot_x(strain: f64, zoom: Zoom) -> f64 {
    PLOT_X0 + strain.min(zoom.max()) / zoom.max() * (PLOT_X1 - PLOT_X0)
}

/// Plot y for a stress (mockup `gy`).
pub fn plot_y(mpa: f64) -> f64 {
    PLOT_Y0 - mpa / STRESS_AXIS_MAX_MPA * (PLOT_Y0 - PLOT_Y1)
}

/// A material's curve on the plot at `t_c` °C, clipped to the axis (mockup `pathOf(k)`), or
/// the part done so far up to `up_to` % (mockup `pathOf(k, upTo)`).
pub fn curve_path(kind: Kind, t_c: f64, zoom: Zoom, up_to: Option<f64>) -> Vec<Pt> {
    let c = kind.curve();
    let p = c.points;
    let fk = temp_factor(kind, t_c);
    let zmax = zoom.max();
    let gx = |e: f64| plot_x(e, zoom);
    let mut d: Vec<Pt> = Vec::with_capacity(p.len() + 1);
    for i in 0..p.len() {
        if let Some(u) = up_to
            && p[i][0] > u
        {
            d.push([gx(u), plot_y(c.stress_at(u, t_c).mpa)]);
            break;
        }
        if p[i][0] > zmax * 1.0001 && i > 0 {
            let (a, b) = (p[i - 1], p[i]);
            let v = (a[1] + (b[1] - a[1]) * (zmax - a[0]) / (b[0] - a[0])) * fk;
            d.push([gx(zmax), plot_y(v)]);
            break;
        }
        d.push([gx(p[i][0]), plot_y(p[i][1] * fk)]);
    }
    if up_to.is_none()
        && let Some(l) = p.last()
        && l[0] <= zmax
    {
        d.push([gx(l[0]), PLOT_Y0]);
    }
    d
}

impl Bench {
    /// Everything the screen shows (mockup `renderVals()`).
    pub fn view(&self) -> View {
        let kind = self.kind();
        let m = kind.curve();
        let temp = self.temp();
        let zoom = self.zoom();
        let e = self.strain();
        let now = self.now();
        let f = temp_factor(kind, temp);
        let status = self.status();
        let status_label = status.label();

        // Plot.
        let mut grid = Vec::new();
        for v in STRESS_TICKS {
            let y = plot_y(v);
            grid.push([[PLOT_X0, y], [PLOT_X1, y]]);
        }
        for &x in zoom.ticks() {
            let gx = plot_x(x, zoom);
            grid.push([[gx, PLOT_Y1], [gx, PLOT_Y0]]);
        }
        let ghosts = Kind::ALL
            .into_iter()
            .filter(|k| *k != kind)
            .map(|k| (k, curve_path(k, temp, zoom, None)))
            .collect();
        let done = if e > 0.0 {
            curve_path(kind, temp, zoom, Some(e.min(now.break_strain)))
        } else {
            Vec::new()
        };
        let labels = Kind::ALL
            .into_iter()
            .map(|k| {
                let p = k.curve().points;
                let mut idx = p.len() - 1;
                while idx > 0 && p[idx][0] > zoom.max() {
                    idx -= 1;
                }
                let pk = p[idx];
                CurveLabel {
                    kind: k,
                    name: k.name(),
                    at: [plot_x(pk[0], zoom), plot_y(pk[1] * temp_factor(k, temp))],
                    selected: k == kind,
                }
            })
            .collect();
        let plot = PlotGeometry {
            axes: [[PLOT_X0, PLOT_Y1], [PLOT_X0, PLOT_Y0], [PLOT_X1, PLOT_Y0]],
            grid,
            ghosts,
            curve: curve_path(kind, temp, zoom, None),
            done,
            marker: [plot_x(e.min(now.break_strain), zoom), plot_y(now.mpa)],
            ticks_y: STRESS_TICKS
                .iter()
                .map(|&v| Tick {
                    label: to_string(v),
                    pos: plot_y(v),
                })
                .collect(),
            ticks_x: zoom
                .ticks()
                .iter()
                .map(|&v| Tick {
                    label: to_string(v),
                    pos: plot_x(v, zoom),
                })
                .collect(),
            labels,
        };

        // Rig.
        let necking = self.necking();
        let ext = e.min(now.break_strain) * RIG_PX_PER_PERCENT;
        let top_y = RIG_TOP_Y - ext;
        let bot_y = RIG_BOT_Y;
        let mid_y = (top_y + 40.0 + bot_y) / 2.0;
        let (cx, gw, sw) = (RIG_CX, RIG_GAUGE_HALF, RIG_GRIP_HALF);
        let gw_n = gw * (1.0 - necking);
        let side = |sgn: f64| -> [Pt; 7] {
            [
                [cx + sgn * sw, top_y],
                [cx + sgn * sw, top_y + 40.0],
                [cx + sgn * gw, top_y + 60.0],
                [cx + sgn * gw_n, mid_y],
                [cx + sgn * gw, bot_y - 20.0],
                [cx + sgn * sw, bot_y],
                [cx + sgn * sw, bot_y + 20.0],
            ]
        };
        let specimen = if !now.broken {
            let mut poly: Vec<Pt> = side(-1.0).to_vec();
            poly.extend(side(1.0).iter().rev());
            vec![poly]
        } else {
            let gap = 10.0;
            let w2 = if m.necks() { gw_n } else { gw };
            vec![
                vec![
                    [cx - sw, top_y],
                    [cx + sw, top_y],
                    [cx + sw, top_y + 40.0],
                    [cx + gw, top_y + 60.0],
                    [cx + w2, mid_y - gap],
                    [cx - w2, mid_y - gap],
                    [cx - gw, top_y + 60.0],
                    [cx - sw, top_y + 40.0],
                ],
                vec![
                    [cx - w2, mid_y + gap],
                    [cx + w2, mid_y + gap],
                    [cx + gw, bot_y - 20.0],
                    [cx + sw, bot_y],
                    [cx + sw, bot_y + 20.0],
                    [cx - sw, bot_y + 20.0],
                    [cx - sw, bot_y],
                    [cx - gw, bot_y - 20.0],
                ],
            ]
        };
        let crack = now.broken.then_some([
            [cx - gw_n - 4.0, mid_y - 10.0],
            [cx - 4.0, mid_y - 4.0],
            [cx + 4.0, mid_y - 9.0],
            [cx + gw_n + 4.0, mid_y - 3.0],
        ]);
        let gauge = [
            [
                [cx + sw + 10.0, top_y + 60.0],
                [cx + sw + 10.0, bot_y - 20.0],
            ],
            [
                [cx + sw + 4.0, top_y + 60.0],
                [cx + sw + 16.0, top_y + 60.0],
            ],
            [
                [cx + sw + 4.0, bot_y - 20.0],
                [cx + sw + 16.0, bot_y - 20.0],
            ],
        ];
        let hy = top_y - 30.0;
        let rig = RigGeometry {
            top_y,
            bot_y,
            mid_y,
            head: [
                [150.0, hy, 270.0, hy + 30.0],
                [112.0, hy + 6.0, 308.0, hy + 18.0],
            ],
            head_top: [
                [150.0, hy],
                [162.0, hy - 8.0],
                [282.0, hy - 8.0],
                [270.0, hy],
            ],
            specimen,
            gauge,
            crack,
            necking,
        };

        // Readouts.
        let yield_s = m.yield_mpa;
        let peak = m.room_peak_mpa() * f;
        let tiles = vec![
            Tile {
                key: "Stiffness E",
                value: format!("{} GPa", to_string(m.e_gpa)),
                note: "slope of the first line",
            },
            Tile {
                key: "Yield",
                value: match yield_s {
                    Some(y) => format!("{} MPa", to_fixed(y * f, 0)),
                    None => "none".into(),
                },
                note: if yield_s.is_some() {
                    "where it stops springing back"
                } else {
                    "brittle: no yield"
                },
            },
            Tile {
                key: "Peak strength",
                value: format!("{} MPa", to_fixed(peak, 0)),
                note: if kind == Kind::Concrete {
                    "in compression"
                } else {
                    "ultimate"
                },
            },
            Tile {
                key: "Stretch at break",
                value: format!("{} %", to_string(now.break_strain)),
                note: if m.has_plastic_range() && now.break_strain > 2.0 {
                    "ductile"
                } else {
                    "brittle"
                },
            },
            Tile {
                key: "Toughness",
                value: format!("{} MJ/m\u{b3}", to_fixed(m.toughness(), 2)),
                note: "area under the curve",
            },
            Tile {
                key: "Density",
                value: format!("{} kg/m\u{b3}", to_string(m.rho)),
                note: "",
            },
        ];
        let vals: Vec<f64> = Kind::ALL
            .iter()
            .map(|k| k.curve().specific_strength(temp))
            .collect();
        let max_spec = vals.iter().copied().fold(1.0_f64, f64::max);
        let spec = Kind::ALL
            .iter()
            .zip(&vals)
            .map(|(&k, &val)| {
                let w = to_fixed(val / max_spec * 100.0, 0);
                SpecBar {
                    kind: k,
                    name: k.name(),
                    value: val,
                    label: to_fixed(val, 0),
                    fill: w.parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0) / 100.0,
                    selected: k == kind,
                }
            })
            .collect();
        let challenge = Kind::ALL
            .iter()
            .map(|&k| ChallengeChip {
                kind: k,
                name: k.name(),
                lit: self.broke(k),
            })
            .collect();

        View {
            kind,
            status,
            status_tone: status.tone(),
            explain: self.explain(),
            note: m.note,
            temp_label: format!("{} \u{b0}C", to_string(temp)),
            temp_tone: if temp >= HOT_C {
                Tone::Caution
            } else {
                Tone::Accent
            },
            temp_note: if f < 1.0 {
                format!(
                    "Strength now {}% of room temperature (illustrative).",
                    to_string(round(f * 100.0))
                )
            } else {
                "Room-temperature strength.".into()
            },
            strain_label: format!(
                "{} % \u{b7} {} MPa",
                to_fixed(e, zoom.decimals()),
                to_fixed(now.mpa, 0)
            ),
            clock: self.clock(),
            force_label: format!("{} kN", to_fixed(load_cell_kn(now.mpa), 1)),
            section_label: format!("{} mm\u{b2} section", to_string(SECTION_MM2)),
            tiles,
            spec,
            challenge,
            challenge_rank: format!("{} / {}", CHALLENGE_RANK.0, CHALLENGE_RANK.1),
            rig_label: format!(
                "{} sample in a tensile test machine, stretched {}%. {}.",
                m.name,
                to_fixed(e, 2),
                status_label
            ),
            plot_label: format!(
                "Stress-strain curves for five materials. {} highlighted, now at {} MPa.",
                m.name,
                to_fixed(now.mpa, 0)
            ),
            status_label,
            plot,
            rig,
        }
    }
}
