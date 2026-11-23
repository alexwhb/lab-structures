//! SCR-3 Tower — "Stack it up, then let the storm in" (`docs/design/mockup/Tower.dc.html`).
//!
//! Left: floors (stepper + slider, 5–120), structural system and shape tiles, the tuned mass
//! damper (TW-1). Centre: the 2.5D stage ([`stage`]: the tower floor by floor with its system
//! lines, taper / twist / sky slots, the lean and, while playing, the gusting sway from
//! `sp_engine::dynamics`; plumb line, wind arrows and streaks, chips, the damper; TW-3, TW-4),
//! and the bottom bar (storm, sway exaggeration, "Send base reactions to Ground ›"; TW-5).
//! Right: readout tiles, occupant comfort, "Same storm, four shapes", "Challenge · 100 floors",
//! "What's happening" (TW-6), and the status pill (TW-7). Every number comes from
//! `sp_engine::tower` (DONE-2).

pub mod stage;

use super::{Badge, Drive, Screen, Share};
use crate::ctx::{Carry, Ctx};
use crate::layout::{self, Column};
use crate::router::Route;
use crate::widgets::world::{self, Readout, RowLabel, TileArt};
use crate::widgets::{Callout, Checkbox, Slider, ghost, lbl, text};
use eframe::egui::{Color32, Ui, vec2};
use lab_ui::layout::{FlexItem, flex_lines};
use sp_engine::dynamics::{SwaySim, playback_speed};
use sp_engine::tower::{
    self, CHALLENGE_RUNG, Comfort, EX_DEFAULT, EX_MAX, EX_MIN, FLOORS_MAX, FLOORS_MIN, Shape,
    Status, System, Tone, TowerInput, TowerResult, V10_MAX, V10_MIN,
};
use sp_theme as t;

/// The page row: parts · stage · readouts.
const COLUMNS: [Column; 3] = [
    Column::parts("Parts"),
    Column::main("Stage")
        .flex(999.0, 1.0, 520.0)
        .pad(16.0, 16.0)
        .gap(12.0),
    Column::readouts("Readouts"),
];

/// The screen's heading and sub-line.
pub const TITLE: &str = "Stack it up, then let the storm in";
/// See [`TITLE`].
pub const SUBTITLE: &str =
    "40 m square plan, 4 m floors. Wind blows from the left and gets stronger with height.";
/// Frame interval while Play gusts runs, s (~30 fps, PLAT-3).
pub const ANIM_FRAME: f32 = 1.0 / 30.0;
/// The hand-off button (NAV-4).
pub const SEND_TO_GROUND: &str = "Send base reactions to Ground ›";

/// The colour of an engine [`Tone`].
pub fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Plain => t::TEXT,
        Tone::Ok => t::OK,
        Tone::Marginal => t::ACCENT,
        Tone::Caution => t::CAUTION,
        Tone::Alert => t::DANGER_INK,
        Tone::Danger => t::DANGER,
    }
}

/// SCR-3 state.
#[derive(Debug, Clone)]
pub struct TowerScreen {
    /// The settings (floors, system, shape, wind, damper).
    pub input: TowerInput,
    /// "Show sway" exaggeration (1–150).
    pub ex: f32,
    /// Play gusts is running.
    pub playing: bool,
    /// The gust simulation (created by Play gusts; kept while paused; dropped by Reset or
    /// when the settings change while paused, so the tower rests at its lean again).
    pub sim: Option<SwaySim>,
    /// The streaks' dash phase, s (advances while playing).
    pub flow: f32,
    seed: u64,
}

impl Default for TowerScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl TowerScreen {
    /// The mockup's starting state: 60 floors, core, straight, 30 m/s, no damper, ×40.
    pub fn new() -> Self {
        Self {
            input: TowerInput::default(),
            ex: EX_DEFAULT as f32,
            playing: false,
            sim: None,
            flow: 0.0,
            seed: 7,
        }
    }

    /// The model's results for the current settings.
    pub fn result(&self) -> TowerResult {
        tower::calc(&self.input)
    }

    /// The top floor's drawn displacement, m: the simulated sway while a simulation exists,
    /// else the rest lean (`0.85·δ`, the mockup's still pose).
    pub fn top_sway(&self, r: &TowerResult) -> f64 {
        match &self.sim {
            Some(sim) => sim.top(),
            None => tower::MEAN_LEAN * r.delta,
        }
    }

    /// The damper's drawn stroke relative to the top, m (0 when not simulated).
    pub fn damper_stroke(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.damper_stroke())
    }

    /// Applies new settings: retunes a running simulation (the swing carries over), drops a
    /// paused one.
    pub fn set_input(&mut self, input: TowerInput) {
        let input = input.sanitized();
        if input == self.input {
            return;
        }
        self.input = input;
        if self.playing {
            let r = self.result();
            let p = tower::sway_params(&self.input, &r, self.seed);
            if let Some(sim) = &mut self.sim {
                sim.retune(p);
            } else {
                self.sim = Some(SwaySim::new(p));
            }
        } else {
            self.sim = None;
        }
    }

    fn start(&mut self) {
        if self.sim.is_none() {
            let r = self.result();
            self.sim = Some(SwaySim::new(tower::sway_params(&self.input, &r, self.seed)));
        }
        self.playing = true;
    }

    fn parts(&mut self, ui: &mut Ui) {
        let mut input = self.input;
        section(ui, |ui| {
            lbl(ui, "Floors");
            let step = world::stepper(
                ui,
                &input.floors.to_string(),
                &tower::floors_note(&input),
                "Remove a floor",
                "Add a floor",
                (true, true),
            );
            if step.minus {
                input.floors = input.floors.saturating_sub(1).max(FLOORS_MIN);
            }
            if step.plus {
                input.floors = (input.floors + 1).min(FLOORS_MAX);
            }
            let mut f = input.floors as f32;
            if Slider::new(
                "Number of floors",
                &mut f,
                FLOORS_MIN as f32..=FLOORS_MAX as f32,
            )
            .step(1.0)
            .show(ui)
            .changed()
            {
                input.floors = f.round() as u32;
            }
        });
        section(ui, |ui| {
            lbl(ui, "Structural system");
            let items: Vec<(&str, TileArt)> = System::ALL
                .iter()
                .map(|s| (s.def().name, icon(s.def().icon)))
                .collect();
            let sel = System::ALL
                .iter()
                .position(|s| *s == input.system)
                .unwrap_or(0);
            if let Some(i) = world::picture_tiles(ui, &items, sel) {
                input.system = System::ALL[i];
            }
        });
        section(ui, |ui| {
            lbl(ui, "Shape");
            let items: Vec<(&str, TileArt)> = Shape::ALL
                .iter()
                .map(|s| (s.def().name, icon(s.def().icon)))
                .collect();
            let sel = Shape::ALL
                .iter()
                .position(|s| *s == input.shape)
                .unwrap_or(0);
            if let Some(i) = world::picture_tiles(ui, &items, sel) {
                input.shape = Shape::ALL[i];
            }
        });
        if Checkbox::new("Tuned mass damper at the top", input.damper)
            .show(ui)
            .clicked()
        {
            input.damper = !input.damper;
        }
        self.set_input(input);
    }

    fn main(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let r = self.result();
        let status = Status::of(&r);
        world::title_block(
            ui,
            TITLE,
            SUBTITLE,
            status.label(),
            tone_color(status.tone()),
        );
        let w = ui.available_width();
        let h = w * stage::H / stage::W;
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), eframe::egui::Sense::hover());
        stage::draw(ui, rect, self, &r);
        self.bottom_bar(ui, ctx, &r);
    }

    fn bottom_bar(&mut self, ui: &mut Ui, ctx: &mut Ctx, r: &TowerResult) {
        let mut input = self.input;
        let mut send = false;
        crate::widgets::Card::panel()
            .padding(12.0)
            .gap(12.0)
            .label("Storm and sway")
            .show(ui, |ui| {
                // `.panel` padding is 12px 16px: 4 more at the sides
                let full = ui.available_width();
                let inner = full - 8.0;
                let link_w = ghost(SEND_TO_GROUND).desired_size(ui).x;
                let items = [
                    FlexItem::new(1.0, 1.0, 260.0),
                    FlexItem::new(1.0, 1.0, 200.0),
                    FlexItem::new(0.0, 0.0, link_w),
                ];
                let lines = flex_lines(inner, 24.0, &items);
                for line in &lines {
                    let y = ui.cursor().min.y;
                    let x0 = ui.cursor().min.x + 4.0;
                    let mut bottom = y;
                    for p in line {
                        let rect = eframe::egui::Rect::from_min_size(
                            eframe::egui::pos2(x0 + p.x, y),
                            vec2(p.width, 200.0),
                        );
                        let mut cell = ui.new_child(
                            eframe::egui::UiBuilder::new()
                                .max_rect(rect)
                                .layout(eframe::egui::Layout::top_down(eframe::egui::Align::Min)),
                        );
                        cell.spacing_mut().item_spacing = vec2(2.0, 2.0);
                        match p.index {
                            0 => {
                                world::value_row(
                                    &mut cell,
                                    "Storm · wind at 10 m",
                                    RowLabel::Strong,
                                    &tower::v10_label(input.v10),
                                    t::INFO,
                                );
                                let mut v = input.v10 as f32;
                                if Slider::new(
                                    "Storm · wind at 10 m",
                                    &mut v,
                                    V10_MIN as f32..=V10_MAX as f32,
                                )
                                .step(1.0)
                                .show(&mut cell)
                                .changed()
                                {
                                    input.v10 = v.round() as f64;
                                }
                            }
                            1 => {
                                world::value_row(
                                    &mut cell,
                                    "Show sway",
                                    RowLabel::Strong,
                                    &format!("×{}", self.ex.round()),
                                    t::ACCENT,
                                );
                                let mut ex = self.ex;
                                if Slider::new("Show sway", &mut ex, EX_MIN as f32..=EX_MAX as f32)
                                    .step(1.0)
                                    .show(&mut cell)
                                    .changed()
                                {
                                    self.ex = ex.round();
                                }
                            }
                            _ => {
                                // vertically centred in the line (`align-items: center`)
                                let line_h: f32 = if line.len() > 1 { 46.0 } else { 36.0 };
                                cell.add_space(((line_h - 36.0) / 2.0).max(0.0));
                                if ghost(SEND_TO_GROUND).link().show(&mut cell).clicked() {
                                    send = true;
                                }
                            }
                        }
                        bottom = bottom.max(cell.min_rect().bottom());
                    }
                    let used = eframe::egui::Rect::from_min_max(
                        eframe::egui::pos2(ui.cursor().min.x, y),
                        eframe::egui::pos2(ui.cursor().min.x + full, bottom),
                    );
                    ui.advance_cursor_after_rect(used);
                }
            });
        self.set_input(input);
        if send {
            ctx.go_with(
                Route::Ground,
                Carry::Building {
                    floors: self.input.floors,
                    base_shear_n: r.base_shear,
                    overturning_nm: r.overturning,
                },
            );
        }
    }

    fn readouts(&self, ui: &mut Ui) {
        let r = self.result();
        let tiles = tower::tiles(&r);
        let readouts: Vec<Readout> = tiles
            .iter()
            .map(|tl| Readout {
                key: tl.key,
                value: &tl.value,
                note: &tl.note,
                color: tone_color(tl.tone),
            })
            .collect();
        world::readout_grid(ui, &readouts);

        let comfort = Comfort::of(r.mg);
        let cc = tone_color(comfort.tone());
        section(ui, |ui| {
            text::lbl_row(
                ui,
                "Occupant comfort",
                &tower::accel_label(&r),
                lab_theme::font::mono(12.0),
                cc,
            );
            let b = &t::tower::COMFORT_BANDS;
            world::band_bar(
                ui,
                &[(5.0, b[0]), (5.0, b[1]), (10.0, b[2]), (10.0, b[3])],
                tower::comfort_pos(r.mg) as f32,
                &["0", "5", "10", "20", "30+ milli-g"],
                "Comfort bar: 0, 5, 10, 20, 30+ milli-g",
            );
            text::text_block(
                ui,
                comfort.sentence(),
                lab_theme::font::body(13.0),
                cc,
                None,
            );
        });

        section(ui, |ui| {
            lbl(ui, "Same storm, four shapes");
            for bar in tower::compare(&self.input) {
                world::compare_row(
                    ui,
                    bar.shape.def().name,
                    bar.shape == self.input.shape,
                    sp_engine::jsnum::js_round(bar.width) as f32,
                    tone_color(bar.tone()),
                    &bar.label(),
                );
            }
            text::text_block(
                ui,
                "Peak acceleration at the top floor, same floors, system and wind.",
                lab_theme::font::body(11.0),
                t::TEXT_MUTED,
                None,
            );
        });

        world::challenge(
            ui,
            "Challenge · 100 floors",
            &format!("{} / {}", CHALLENGE_RUNG.0, CHALLENGE_RUNG.1),
            |ui| {
                for g in tower::goals(&self.input, &r) {
                    world::goal_row(ui, g.text, g.done);
                }
            },
        );
        Callout::happening(&tower::explain(&self.input, &r)).show(ui);
    }
}

/// A column section: children 8 px apart (`display: flex; flex-direction: column; gap: 8px`).
fn section(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        add(ui);
    });
}

fn icon(d: &str) -> TileArt<'_> {
    TileArt::Icon {
        d,
        viewbox: vec2(40.0, 40.0),
        under: None,
    }
}

impl Screen for TowerScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => self.parts(ui),
            1 => self.main(ui, ctx),
            _ => self.readouts(ui),
        });
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Play gusts").running(self.playing))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        if self.playing {
            self.playing = false;
        } else {
            self.start();
        }
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        let seed = self.seed;
        *self = Self::new();
        self.seed = seed;
    }

    fn clock(&self) -> String {
        tower::clock(&self.result())
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        let r = self.result();
        Some(
            Share::new(tower::share_line(&self.input, &r))
                .toast("Copied the tower as a .story line"),
        )
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::simplified("Wind + sway: simplified cantilever"),
            Badge::illustrative("Comfort bands: illustrative"),
            Badge::illustrative("Game costs"),
        ]
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        if !self.playing {
            return;
        }
        let dt = ctx.dt();
        let f1 = self.result().f1;
        if let Some(sim) = &mut self.sim {
            sim.advance(dt as f64 * playback_speed(f1));
        }
        self.flow = (self.flow + dt) % 1.6;
        // ~30 fps while it moves (PLAT-3); SwaySim integrates in its own fixed steps
        ctx.wake_in(ANIM_FRAME);
    }

    fn enter(&mut self, ctx: &mut Ctx) {
        self.seed = ctx.rng_seed();
    }

    fn receive(&mut self, carry: Carry, _ctx: &mut Ctx) {
        if let Carry::Tower { floors } = carry {
            self.set_input(TowerInput {
                floors,
                ..self.input
            });
        }
    }

    fn describe(&self) -> String {
        tower::stage_label(&self.input, &self.result())
    }
}
