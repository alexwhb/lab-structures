//! Everything the sample drawer shows (LAB-1): readout strings and the drawing model of the
//! test rig and the stress–strain plot, in v1's SVG viewBox units (rig `0 0 420 560`, plot
//! `0 0 560 400`, `docs/design/mockup/Materials.dc.html`) so the v1 painters fit. Both axes are
//! fitted to the sample (v1 had two fixed zooms); the drawn stretch is exaggerated to
//! [`RIG_TRAVEL`] px at the axis end.

use super::bench::{Sample, Status, Tone, clean, fmt_kn, fmt_strain, fmt_tonnes, nice_ceil};
use super::curve::{Failure, SampleCurve, Test, tonnes};
use crate::builder::data::PieceType;
use crate::jsnum::{to_fixed, to_string};

/// A point in viewBox px.
pub type Pt = [f64; 2];

/// The plot's viewBox (`0 0 560 400`).
pub const PLOT_VIEW: [f64; 2] = [560.0, 400.0];
/// Plot area: x from 56 to 544, y from 350 (0) up to 20 (the axis end), as v1.
pub const PLOT_X0: f64 = 56.0;
/// See [`PLOT_X0`].
pub const PLOT_X1: f64 = 544.0;
/// See [`PLOT_X0`].
pub const PLOT_Y0: f64 = 350.0;
/// See [`PLOT_X0`].
pub const PLOT_Y1: f64 = 20.0;
/// The rig's viewBox (`0 0 420 560`).
pub const RIG_VIEW: [f64; 2] = [420.0, 560.0];
/// The sample's centre line.
pub const RIG_CX: f64 = 210.0;
/// Pull: the unstretched sample's top; both tests: the fixed bottom (top of the lower grip).
pub const RIG_TOP_Y: f64 = 190.0;
/// See [`RIG_TOP_Y`].
pub const RIG_BOT_Y: f64 = 452.0;
/// Push: the unloaded column's top.
pub const RIG_PUSH_TOP_Y: f64 = 250.0;
/// Dog-bone grip and gauge half-widths (Pull), px.
pub const RIG_GRIP_HALF: f64 = 24.0;
/// See [`RIG_GRIP_HALF`].
pub const RIG_GAUGE_HALF: f64 = 13.0;
/// Push: the column's half-width, px.
pub const RIG_COLUMN_HALF: f64 = 18.0;
/// Drawn travel at the strain axis end, px ("stretch exaggerated").
pub const RIG_TRAVEL: f64 = 110.0;
/// A buckled column bows this far at failure, px.
pub const RIG_BOW: f64 = 46.0;
/// Squashed steel bulges this much at the end of its plateau, px.
pub const RIG_BULGE: f64 = 10.0;

/// An axis tick label at a position along its axis (y for stress, x for strain).
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    /// "100", "0.5".
    pub label: String,
    /// Position along the axis, viewBox px.
    pub pos: f64,
}

/// The stress–strain plot (`0 0 560 400`).
#[derive(Debug, Clone, PartialEq)]
pub struct PlotGeometry {
    /// The axes polyline.
    pub axes: [Pt; 3],
    /// Grid lines at the ticks.
    pub grid: Vec<[Pt; 2]>,
    /// The same test at 20 °C when the heat has changed it (empty otherwise).
    pub ghost: Vec<Pt>,
    /// The curve (dropping to the axis at failure).
    pub curve: Vec<Pt>,
    /// The part done so far (empty before loading).
    pub done: Vec<Pt>,
    /// The marker at the current strain and stress.
    pub marker: Pt,
    /// The builder's limit: a dashed line across the plot at the capacity's stress.
    pub limit: Option<[Pt; 2]>,
    /// Its label ("builder limit 153 t"), anchored at its right end (text right-aligned above).
    pub limit_label: String,
    /// Stress ticks (`pos` = y).
    pub ticks_y: Vec<Tick>,
    /// Strain ticks (`pos` = x).
    pub ticks_x: Vec<Tick>,
    /// "stress MPa".
    pub y_title: &'static str,
    /// "strain %" / "squeeze %".
    pub x_title: &'static str,
}

/// The test rig's moving parts (`0 0 420 560`); the frame is static.
#[derive(Debug, Clone, PartialEq)]
pub struct RigGeometry {
    /// The crosshead block `[x0, y0, x1, y1]` and the beam through the columns.
    pub head: [[f64; 4]; 2],
    /// The block's top face (2.5D).
    pub head_top: [Pt; 4],
    /// The sample outline: one polygon, or two once broken.
    pub specimen: Vec<Vec<Pt>>,
    /// The extensometer: its bar and two knife edges.
    pub gauge: [[Pt; 2]; 3],
    /// The red crack line once broken.
    pub crack: Option<Vec<Pt>>,
    /// Necking (Pull), 0–0.55.
    pub necking: f64,
    /// Burned through: draw it charred.
    pub charred: bool,
}

/// One readout tile.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    /// `.lbl` ("Stiffness E").
    pub key: &'static str,
    /// Mono value ("200 GPa").
    pub value: String,
    /// Muted note.
    pub note: String,
}

/// Everything the drawer shows.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleView {
    /// The piece type.
    pub kind: PieceType,
    /// Pull or push.
    pub test: Test,
    /// "Steel sample".
    pub title: String,
    /// "The builder's own section: 60 cm². Pull it until it breaks."
    pub subtitle: String,
    /// The status.
    pub status: Status,
    /// Its pill text.
    pub status_label: String,
    /// Its colour role.
    pub status_tone: Tone,
    /// "What's happening".
    pub explain: String,
    /// The force now: "2.4 t".
    pub force_t: String,
    /// … in kN: "23.5 kN".
    pub force_kn: String,
    /// The strain and stress now: "0.10 % · 200 MPa".
    pub strain_label: String,
    /// "A Steel piece this size breaks at 153 t."
    pub limit: String,
    /// The force has reached the builder's limit (the piece would count as broken).
    pub over_limit: bool,
    /// "Past this the builder counts the piece as broken." when over, else "".
    pub limit_note: String,
    /// "20 °C".
    pub temp_label: String,
    /// At or above 100 °C (the readout turns warm).
    pub temp_hot: bool,
    /// "Room-temperature strength." / the heat note.
    pub temp_note: String,
    /// "60 cm² section".
    pub section_label: String,
    /// "1 m" (Push: the sample's length).
    pub length_label: String,
    /// The readout tiles.
    pub tiles: Vec<Tile>,
    /// "Pull" or "Pause".
    pub pull_label: &'static str,
    /// "Push" or "Pause".
    pub push_label: &'static str,
    /// Push is offered (not Rope or Cable).
    pub can_push: bool,
    /// Why not: "Rope only pulls: pushed, it goes slack." (empty when it can).
    pub push_note: String,
    /// The drawer's clock line "ε = 0.10 %".
    pub clock: String,
    /// The rig's accessible label.
    pub rig_label: String,
    /// The plot's accessible label.
    pub plot_label: String,
    /// The load cell chip's force ("23.5 kN").
    pub load_cell: String,
    /// The load cell chip's note ("2.4 t on 60 cm²").
    pub load_cell_note: String,
    /// The plot.
    pub plot: PlotGeometry,
    /// The rig.
    pub rig: RigGeometry,
}

/// Tick positions for an axis 0…`max`: 4 or 5 round steps.
pub fn ticks(max: f64) -> Vec<f64> {
    if !(max.is_finite() && max > 0.0) {
        return vec![0.0];
    }
    let p = 10f64.powf(max.log10().floor());
    let m = clean(max / p);
    let n = if (m - 2.0).abs() < 1e-9 { 4 } else { 5 };
    (0..=n).map(|i| clean(max * i as f64 / n as f64)).collect()
}

/// The polyline of `c` on a plot with these axis ends, optionally only up to `up_to` %.
pub fn curve_path(c: &SampleCurve, axis: f64, smax: f64, up_to: Option<f64>) -> Vec<Pt> {
    let gx = |e: f64| PLOT_X0 + e.min(axis) / axis * (PLOT_X1 - PLOT_X0);
    let gy = |s: f64| PLOT_Y0 - (s / smax).min(1.05) * (PLOT_Y0 - PLOT_Y1);
    let mut d: Vec<Pt> = Vec::with_capacity(c.points.len() + 1);
    for (i, q) in c.points.iter().enumerate() {
        if let Some(u) = up_to
            && q[0] > u
        {
            d.push([gx(u), gy(c.stress_at(u).mpa)]);
            return d;
        }
        if q[0] > axis * 1.0001 && i > 0 {
            let a = c.points[i - 1];
            let v = a[1] + (q[1] - a[1]) * (axis - a[0]) / (q[0] - a[0]);
            d.push([gx(axis), gy(v)]);
            return d;
        }
        d.push([gx(q[0]), gy(q[1])]);
    }
    if up_to.is_none() && c.break_strain() <= axis && !c.burned_through() {
        d.push([gx(c.break_strain()), PLOT_Y0]);
    }
    d
}

impl Sample {
    /// Everything the drawer shows for this state.
    pub fn view(&self) -> SampleView {
        let kind = self.kind();
        let spec = kind.spec();
        let test = self.test();
        let name = kind.name();
        let lower = name.to_lowercase();
        let status = self.status();
        let status_label = status.label(kind);
        let now = self.stress();
        let force = self.force_n();
        let axis = self.axis_max();
        let curve = self.curve().cloned();
        let room = self.room_curve();
        let heated = curve
            .as_ref()
            .is_some_and(|c| c.strength < 1.0 || c.stiffness < 1.0);
        let area_cm2 = spec.area * 1e4;

        // Plot.
        let peak = curve.as_ref().map_or(0.0, |c| c.peak_mpa());
        let ghost_peak = if heated {
            room.as_ref().map_or(0.0, |c| c.peak_mpa())
        } else {
            0.0
        };
        let smax = nice_ceil(peak.max(ghost_peak) * 1.15);
        let gx = |e: f64| PLOT_X0 + e.min(axis) / axis * (PLOT_X1 - PLOT_X0);
        let gy = |s: f64| PLOT_Y0 - (s / smax).min(1.05) * (PLOT_Y0 - PLOT_Y1);
        let tx = ticks(axis);
        let ty = ticks(smax);
        let mut grid = Vec::new();
        for &v in ty.iter().skip(1) {
            grid.push([[PLOT_X0, gy(v)], [PLOT_X1, gy(v)]]);
        }
        for &v in &tx {
            grid.push([[gx(v), PLOT_Y1], [gx(v), PLOT_Y0]]);
        }
        let (curve_pts, done, limit, limit_label) = match &curve {
            Some(c) => {
                let done = if self.strain() > 0.0 && !c.burned_through() {
                    curve_path(c, axis, smax, Some(self.strain().min(c.break_strain())))
                } else {
                    Vec::new()
                };
                let limit = (c.limit_mpa > 0.0).then(|| {
                    let y = gy(c.limit_mpa);
                    [[PLOT_X0, y], [PLOT_X1, y]]
                });
                (
                    curve_path(c, axis, smax, None),
                    done,
                    limit,
                    format!("builder limit {} t", to_fixed(tonnes(c.capacity_n), 0)),
                )
            }
            None => (Vec::new(), Vec::new(), None, String::new()),
        };
        let ghost = match (&room, heated) {
            (Some(r), true) => curve_path(r, axis, smax, None),
            _ => Vec::new(),
        };
        let marker_e = curve
            .as_ref()
            .map_or(0.0, |c| self.strain().min(c.break_strain()));
        let plot = PlotGeometry {
            axes: [[PLOT_X0, PLOT_Y1], [PLOT_X0, PLOT_Y0], [PLOT_X1, PLOT_Y0]],
            grid,
            ghost,
            curve: curve_pts,
            done,
            marker: [gx(marker_e), gy(now.mpa)],
            limit,
            limit_label,
            ticks_y: ty
                .iter()
                .skip(1)
                .map(|&v| Tick {
                    label: to_string(v),
                    pos: gy(v),
                })
                .collect(),
            ticks_x: tx
                .iter()
                .map(|&v| Tick {
                    label: to_string(v),
                    pos: gx(v),
                })
                .collect(),
            y_title: "stress MPa",
            x_title: match test {
                Test::Pull => "strain %",
                Test::Push => "squeeze %",
            },
        };

        let rig = self.rig(axis);

        // Readouts.
        let e_gpa = curve.as_ref().map_or(spec.e, |c| c.e) / 1e9;
        let pull_cap = self.curve_for(Test::Pull).map_or(0.0, |c| c.capacity_n);
        let push = self.curve_for(Test::Push);
        let mut tiles = vec![
            Tile {
                key: "Stiffness E",
                value: format!("{} GPa", to_string(clean_sig(e_gpa))),
                note: "slope of the first line".into(),
            },
            Tile {
                key: "Breaks pulled",
                value: format!("{} t", to_fixed(tonnes(pull_cap), 0)),
                note: format!("{} MPa", to_string(clean_sig(pull_cap / spec.area / 1e6))),
            },
        ];
        tiles.push(match &push {
            Some(p) => Tile {
                key: "Gives way pushed",
                value: format!("{} t", to_fixed(tonnes(p.capacity_n), 0)),
                note: if p.buckles {
                    format!("buckles at {} m", to_string(p.length_m))
                } else {
                    format!("crushes at {} m", to_string(p.length_m))
                },
            },
            None => Tile {
                key: "Gives way pushed",
                value: "slack".into(),
                note: "only pulls".into(),
            },
        });
        tiles.push(Tile {
            key: "Stretch at break",
            value: match self.curve_for(Test::Pull) {
                Some(c) if !c.burned_through() => format!("{} %", fmt_strain(c.break_strain())),
                _ => "none".into(),
            },
            note: if spec.ductile {
                "ductile".into()
            } else {
                "brittle".into()
            },
        });

        let temp = self.temp();
        let temp_note = self.heat_note().unwrap_or_else(|| match &curve {
            Some(c) if c.burned_through() => {
                format!("At {} \u{b0}C nothing is left of it.", to_string(temp))
            }
            _ => "Room-temperature strength.".into(),
        });
        let over = self.at_builder_limit() || now.broken;
        let can_push = self.can_push();
        let strain_word = match test {
            Test::Pull => "stretched",
            Test::Push => "squeezed",
        };
        let rig_label = format!(
            "{name} sample in a test machine, {strain_word} {} %. {status_label}.",
            fmt_strain(self.strain())
        );
        let plot_label = format!(
            "Stress-strain curve of {lower} {} at {} \u{b0}C, now at {} MPa; the builder's limit is {} MPa.",
            match test {
                Test::Pull => "pulled",
                Test::Push => "pushed",
            },
            to_string(temp),
            to_fixed(now.mpa, 0),
            to_fixed(curve.as_ref().map_or(0.0, |c| c.limit_mpa), 0)
        );
        SampleView {
            kind,
            test,
            title: format!("{name} sample"),
            subtitle: format!(
                "The builder\u{2019}s own section, {} cm\u{b2}. {}",
                to_string(clean_sig(area_cm2)),
                match test {
                    Test::Pull => "Pull it until it breaks.".to_string(),
                    Test::Push => format!(
                        "Push a {} m length until it gives way.",
                        to_string(self.length())
                    ),
                }
            ),
            status,
            status_tone: status.tone(),
            status_label,
            explain: self.explain(),
            force_t: format!("{} t", fmt_tonnes(force)),
            force_kn: format!("{} kN", fmt_kn(force)),
            strain_label: format!(
                "{} % \u{b7} {} MPa",
                fmt_strain(self.strain()),
                to_fixed(now.mpa, 0)
            ),
            limit: self.limit_sentence(),
            over_limit: over,
            limit_note: if over && !now.broken && !matches!(status, Status::BurnedThrough) {
                "Past this the builder counts the piece as broken.".into()
            } else {
                String::new()
            },
            temp_label: format!("{} \u{b0}C", to_string(temp)),
            temp_hot: temp >= 100.0,
            temp_note,
            section_label: format!("{} cm\u{b2} section", to_string(clean_sig(area_cm2))),
            length_label: format!("{} m", to_string(self.length())),
            tiles,
            pull_label: if self.playing() && test == Test::Pull {
                "Pause"
            } else {
                "Pull"
            },
            push_label: if self.playing() && test == Test::Push {
                "Pause"
            } else {
                "Push"
            },
            can_push,
            push_note: if can_push {
                String::new()
            } else {
                format!("{name} only pulls: pushed, it goes slack.")
            },
            clock: format!(
                "{} = {} %",
                match test {
                    Test::Pull => "\u{3b5}",
                    Test::Push => "\u{2212}\u{3b5}",
                },
                fmt_strain(self.strain())
            ),
            rig_label,
            plot_label,
            load_cell: format!("{} kN", fmt_kn(force)),
            load_cell_note: format!(
                "{} t on {} cm\u{b2}",
                fmt_tonnes(force),
                to_string(clean_sig(area_cm2))
            ),
            plot,
            rig,
        }
    }

    /// The rig drawing for this state on a strain axis ending at `axis` %.
    fn rig(&self, axis: f64) -> RigGeometry {
        let curve = self.curve();
        let now = self.stress();
        let brk = curve.map_or(0.0, |c| c.break_strain());
        let e = self.strain().min(brk);
        let ext = if axis > 0.0 {
            e / axis * RIG_TRAVEL
        } else {
            0.0
        };
        let charred = curve.is_some_and(|c| c.burned_through());
        let broken = now.broken || charred;
        let cx = RIG_CX;
        match self.test() {
            Test::Pull => {
                let necking = curve.map_or(0.0, |c| c.necking(self.strain()));
                let top_y = RIG_TOP_Y - ext;
                let bot_y = RIG_BOT_Y;
                let mid_y = (top_y + 40.0 + bot_y) / 2.0;
                let (gw, sw) = (RIG_GAUGE_HALF, RIG_GRIP_HALF);
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
                let specimen = if !broken {
                    let mut poly: Vec<Pt> = side(-1.0).to_vec();
                    poly.extend(side(1.0).iter().rev());
                    vec![poly]
                } else {
                    let gap = if charred { 16.0 } else { 10.0 };
                    let w2 = gw_n;
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
                let crack = (now.broken && !charred).then(|| {
                    vec![
                        [cx - gw_n - 4.0, mid_y - 10.0],
                        [cx - 4.0, mid_y - 4.0],
                        [cx + 4.0, mid_y - 9.0],
                        [cx + gw_n + 4.0, mid_y - 3.0],
                    ]
                });
                let hy = top_y - 30.0;
                RigGeometry {
                    head: head(hy),
                    head_top: head_top(hy),
                    specimen,
                    gauge: gauge(cx + sw, top_y + 60.0, bot_y - 20.0),
                    crack,
                    necking,
                    charred,
                }
            }
            Test::Push => {
                let top_y = RIG_PUSH_TOP_Y + ext;
                let bot_y = RIG_BOT_Y;
                let hw = RIG_COLUMN_HALF;
                let (el, how) =
                    curve.map_or((0.0, Failure::Crushed), |c| (c.elastic_strain, c.failure));
                let past = if brk > el && self.strain() > el {
                    ((self.strain().min(brk) - el) / (brk - el)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (bow, bulge) = match how {
                    Failure::Buckled => (RIG_BOW * if now.broken { 1.0 } else { past }, 0.0),
                    Failure::Squashed => (0.0, RIG_BULGE * past),
                    _ => (0.0, 3.0 * past),
                };
                let n = 16;
                let side = |sgn: f64| -> Vec<Pt> {
                    (0..=n)
                        .map(|i| {
                            let u = i as f64 / n as f64;
                            let y = top_y + (bot_y - top_y) * u;
                            let s = (std::f64::consts::PI * u).sin();
                            [cx + sgn * (hw + bulge * s) + bow * s, y]
                        })
                        .collect()
                };
                let mid_y = (top_y + bot_y) / 2.0;
                let crushed = now.broken && matches!(how, Failure::Crushed);
                let specimen = if crushed {
                    // Split along a diagonal shear crack, the halves sliding apart.
                    let (y0, y1) = (mid_y - 18.0, mid_y + 18.0);
                    vec![
                        vec![
                            [cx - hw - 3.0, top_y],
                            [cx + hw - 3.0, top_y],
                            [cx + hw - 3.0, y1 - 4.0],
                            [cx - hw - 3.0, y0 - 4.0],
                        ],
                        vec![
                            [cx - hw + 3.0, y0 + 4.0],
                            [cx + hw + 3.0, y1 + 4.0],
                            [cx + hw + 3.0, bot_y],
                            [cx - hw + 3.0, bot_y],
                        ],
                    ]
                } else {
                    let mut poly = side(-1.0);
                    poly.extend(side(1.0).into_iter().rev());
                    vec![poly]
                };
                let crack = crushed.then(|| {
                    vec![
                        [cx - hw - 6.0, mid_y - 20.0],
                        [cx - 4.0, mid_y - 2.0],
                        [cx + 4.0, mid_y + 3.0],
                        [cx + hw + 6.0, mid_y + 20.0],
                    ]
                });
                let hy = top_y - 30.0;
                RigGeometry {
                    head: head(hy),
                    head_top: head_top(hy),
                    specimen,
                    gauge: gauge(cx + hw + bulge.max(bow) + 6.0, top_y + 10.0, bot_y - 10.0),
                    crack,
                    necking: 0.0,
                    charred,
                }
            }
        }
    }
}

/// The crosshead block and beam with the block's top at `hy` (v1's geometry).
fn head(hy: f64) -> [[f64; 4]; 2] {
    [
        [150.0, hy, 270.0, hy + 30.0],
        [112.0, hy + 6.0, 308.0, hy + 18.0],
    ]
}

/// The crosshead block's top face.
fn head_top(hy: f64) -> [Pt; 4] {
    [
        [150.0, hy],
        [162.0, hy - 8.0],
        [282.0, hy - 8.0],
        [270.0, hy],
    ]
}

/// The extensometer bar at `x` from `y0` to `y1` with its two knife edges.
fn gauge(x: f64, y0: f64, y1: f64) -> [[Pt; 2]; 3] {
    let bar = x + 10.0;
    [
        [[bar, y0], [bar, y1]],
        [[bar - 6.0, y0], [bar + 6.0, y0]],
        [[bar - 6.0, y1], [bar + 6.0, y1]],
    ]
}

/// A value rounded to 4 significant figures (tile values: 60 cm², 200 GPa, 62 GPa).
fn clean_sig(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let mag = 10f64.powf(3.0 - v.abs().log10().floor());
    (v * mag).round() / mag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_are_round() {
        assert_eq!(ticks(25.0), vec![0.0, 5.0, 10.0, 15.0, 20.0, 25.0]);
        assert_eq!(ticks(2.0), vec![0.0, 0.5, 1.0, 1.5, 2.0]);
        assert_eq!(ticks(0.01), vec![0.0, 0.002, 0.004, 0.006, 0.008, 0.01]);
    }

    #[test]
    fn default_view_reads_like_v1() {
        let v = Sample::default().view();
        assert_eq!(v.status_label, "Ready");
        assert_eq!(v.force_kn, "0.0 kN");
        assert_eq!(v.limit, "A Steel piece this size breaks at 153 t.");
        assert_eq!(v.section_label, "60 cm\u{b2} section");
        assert_eq!(v.plot.ticks_x.last().map(|t| t.label.as_str()), Some("25"));
    }
}
