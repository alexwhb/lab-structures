//! SCR-4 Ground — "What’s holding it up" (`docs/design/mockup/Ground.dc.html`).
//!
//! Left: soil and foundation tiles, pile length (piles only), building floors, earthquake
//! shaking, the scenarios (GD-1). Centre: the 2.5D soil block ([`stage`]: grains, bedrock, the
//! building sinking and tilting on its foundation, the pressure bulb, sand boils, the shake;
//! GD-3) and the years bar with "‹ Back to the Tower" (GD-4). Right: readout tiles, the safety
//! factor bar, "Challenge · The lean of Pisa", "What's happening" (GD-5). Every number comes
//! from `sp_engine::ground` (DONE-2).

pub mod stage;

use super::tower::tone_color;
use super::{Badge, Drive, Screen, Share};
use crate::ctx::{Carry, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::router::Route;
use crate::widgets::world::{self, Readout, RowLabel, TileArt};
use crate::widgets::{Callout, Slider, ghost, lbl, text};
use eframe::egui::{Ui, vec2};
use lab_ui::layout::{FlexItem, flex_lines};
use sp_engine::ground::{
    self, CHALLENGE_RUNG, CHALLENGE_TEXT, FLOORS_MAX, FLOORS_MIN, Foundation, FromTower,
    GroundInput, GroundResult, ICON_GROUND_LINE, PILE_MAX, PILE_MIN, SCENARIOS, Soil, Status,
    YEARS_MAX, YEARS_TICK,
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
pub const TITLE: &str = "What’s holding it up";
/// See [`TITLE`].
pub const SUBTITLE: &str =
    "40 m footprint. Bedrock at 30 m. Settlement and tilt drawn exaggerated.";
/// The hand-off button (NAV-4).
pub const BACK_TO_TOWER: &str = "‹ Back to the Tower";

/// SCR-4 state.
#[derive(Debug, Clone)]
pub struct GroundScreen {
    /// The settings (soil, foundation, floors, pile length, quake, years, Pisa tried).
    pub input: GroundInput,
    /// Play years is running.
    pub playing: bool,
    /// The building carried from the Tower (its note shows until the floors change).
    pub from_tower: Option<FromTower>,
    /// Animation clock for the shake and the flowing water table, s.
    pub anim: f32,
    ticker: Ticker,
}

impl Default for GroundScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundScreen {
    /// The mockup's starting state: soft clay, mat, 6 floors, year 10.
    pub fn new() -> Self {
        Self {
            input: GroundInput::default(),
            playing: false,
            from_tower: None,
            anim: 0.0,
            ticker: Ticker::default(),
        }
    }

    /// The model's results for the current settings.
    pub fn result(&self) -> GroundResult {
        ground::calc(&self.input)
    }

    /// Applies new settings (clamped). Changing the floors drops the Tower note.
    pub fn set_input(&mut self, input: GroundInput) {
        let input = input.sanitized();
        if input.floors != self.input.floors {
            self.from_tower = None;
        }
        self.input = input;
    }

    /// Loads scenario `i` ("Try a scenario"): its settings at year 0, playing.
    pub fn scenario(&mut self, i: usize) {
        if let Some(s) = SCENARIOS.get(i) {
            self.set_input(s.apply(&self.input));
            self.playing = true;
            self.ticker.reset();
        }
    }

    /// The floors "‹ Back to the Tower" carries: the tower's own floors while this is still
    /// its building (it may have been clamped to 80 here), else this building's.
    pub fn floors_for_tower(&self) -> u32 {
        match self.from_tower {
            Some(ft) if ft.ground_floors() == self.input.floors => ft.floors,
            _ => self
                .input
                .floors
                .clamp(sp_engine::tower::FLOORS_MIN, sp_engine::tower::FLOORS_MAX),
        }
    }

    fn parts(&mut self, ui: &mut Ui) {
        let mut input = self.input;
        let mut scenario = None;
        section(ui, |ui| {
            lbl(ui, "Soil");
            let items: Vec<(&str, TileArt)> = Soil::ALL
                .iter()
                .map(|s| {
                    let sw = t::ground::SOILS[*s as usize];
                    (
                        s.def().name,
                        TileArt::Swatch {
                            fill: sw.body,
                            side: sw.side,
                        },
                    )
                })
                .collect();
            let sel = Soil::ALL.iter().position(|s| *s == input.soil).unwrap_or(0);
            if let Some(i) = world::picture_tiles(ui, &items, sel) {
                input.soil = Soil::ALL[i];
            }
        });
        section(ui, |ui| {
            lbl(ui, "Foundation");
            let items: Vec<(&str, TileArt)> = Foundation::ALL
                .iter()
                .map(|f| {
                    (
                        f.def().name,
                        TileArt::Icon {
                            d: f.def().icon,
                            viewbox: vec2(44.0, 34.0),
                            under: Some((ICON_GROUND_LINE, t::ground::ICON_GROUND)),
                        },
                    )
                })
                .collect();
            let sel = Foundation::ALL
                .iter()
                .position(|f| *f == input.foundation)
                .unwrap_or(0);
            if let Some(i) = world::picture_tiles(ui, &items, sel) {
                input.foundation = Foundation::ALL[i];
            }
            if input.foundation == Foundation::Piles {
                tight(ui, |ui| {
                    world::value_row(
                        ui,
                        "Pile length",
                        RowLabel::Plain,
                        &ground::pile_label(input.pile_len),
                        t::ACCENT,
                    );
                    let mut l = input.pile_len as f32;
                    if Slider::new("Pile length", &mut l, PILE_MIN as f32..=PILE_MAX as f32)
                        .step(1.0)
                        .show(ui)
                        .changed()
                    {
                        input.pile_len = l.round() as f64;
                    }
                });
            }
        });
        tight(ui, |ui| {
            world::value_row(
                ui,
                "Building",
                RowLabel::Eyebrow,
                &format!("{} floors", input.floors),
                t::ACCENT,
            );
            let mut f = input.floors as f32;
            if Slider::new("Building", &mut f, FLOORS_MIN as f32..=FLOORS_MAX as f32)
                .step(1.0)
                .show(ui)
                .changed()
            {
                input.floors = f.round() as u32;
            }
        });
        if world::checkbox(ui, "Earthquake shaking", input.quake).clicked() {
            input.quake = !input.quake;
        }
        section(ui, |ui| {
            lbl(ui, "Try a scenario");
            for (i, s) in SCENARIOS.iter().enumerate() {
                if world::wrap_button(ui, s.name, 40.0).clicked() {
                    scenario = Some(i);
                }
            }
        });
        self.set_input(input);
        if let Some(i) = scenario {
            self.scenario(i);
        }
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
        if let Some(ft) = self.from_tower {
            text::text_block(
                ui,
                &ft.note(),
                lab_theme::font::body(12.0),
                t::TEXT_DIM,
                Some(18.0),
            );
        }
        let w = ui.available_width();
        let h = w * stage::H / stage::W;
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), eframe::egui::Sense::hover());
        stage::draw(ui, rect, self, &r);
        self.bottom_bar(ui, ctx);
    }

    fn bottom_bar(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let mut years = self.input.years as f32;
        let mut scrubbed = false;
        let mut back = false;
        crate::widgets::Card::panel()
            .padding(12.0)
            .gap(12.0)
            .label("Years")
            .show(ui, |ui| {
                let full = ui.available_width();
                let inner = full - 8.0;
                let link_w = ghost(BACK_TO_TOWER).desired_size(ui).x;
                let items = [
                    FlexItem::new(1.0, 1.0, 320.0),
                    FlexItem::new(0.0, 0.0, link_w),
                ];
                for line in flex_lines(inner, 24.0, &items) {
                    let y = ui.cursor().min.y;
                    let x0 = ui.cursor().min.x + 4.0;
                    let mut bottom = y;
                    for p in &line {
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
                        if p.index == 0 {
                            world::value_row(
                                &mut cell,
                                "Years after construction",
                                RowLabel::Strong,
                                &ground::year_label(self.input.years),
                                t::ACCENT,
                            );
                            if Slider::new(
                                "Years after construction",
                                &mut years,
                                0.0..=YEARS_MAX as f32,
                            )
                            .step(0.5)
                            .show(&mut cell)
                            .changed()
                            {
                                scrubbed = true;
                            }
                        } else {
                            let line_h: f32 = if line.len() > 1 { 46.0 } else { 36.0 };
                            cell.add_space(((line_h - 36.0) / 2.0).max(0.0));
                            if ghost(BACK_TO_TOWER).link().show(&mut cell).clicked() {
                                back = true;
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
        if scrubbed {
            // the mockup's setT: scrubbing pauses
            self.set_input(GroundInput {
                years: years as f64,
                ..self.input
            });
            self.playing = false;
        }
        if back {
            ctx.go_with(
                Route::Tower,
                Carry::Tower {
                    floors: self.floors_for_tower(),
                },
            );
        }
    }

    fn readouts(&self, ui: &mut Ui) {
        let r = self.result();
        let tiles = ground::tiles(&self.input, &r);
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
        section(ui, |ui| {
            text::lbl_row(
                ui,
                "Safety factor",
                &ground::fs_text(&r),
                lab_theme::font::mono(12.0),
                tone_color(ground::fs_tone(&r)),
            );
            let b = &t::ground::SAFETY_BANDS;
            world::band_bar(
                ui,
                &[(1.0, b[0]), (2.0, b[1]), (2.0, b[2])],
                ground::fs_pos(&r) as f32,
                &["0", "1 fails", "3 target", "5+"],
                "Safety factor bar: 0, 1 fails, 3 target, 5+",
            );
        });
        world::challenge(
            ui,
            "Challenge · The lean of Pisa",
            &format!("{} / {}", CHALLENGE_RUNG.0, CHALLENGE_RUNG.1),
            |ui| {
                text::text_block(
                    ui,
                    CHALLENGE_TEXT,
                    lab_theme::font::body(13.0),
                    t::TEXT_MUTED,
                    Some(19.5),
                );
                for g in ground::goals(&self.input) {
                    world::goal_row(ui, g.text, g.done);
                }
            },
        );
        Callout::happening(ground::explain(&self.input, &r)).show(ui);
    }
}

/// A column section: children 8 px apart.
fn section(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        add(ui);
    });
}

/// A label + slider block: children 2 px apart.
fn tight(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        add(ui);
    });
}

impl Screen for GroundScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => self.parts(ui),
            1 => self.main(ui, ctx),
            _ => self.readouts(ui),
        });
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Play years").running(self.playing))
    }

    fn drive(&mut self, _ctx: &mut Ctx) {
        if self.playing {
            self.playing = false;
        } else {
            self.input.years = ground::play_start(self.input.years);
            self.playing = true;
            self.ticker.reset();
        }
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        *self = Self::new();
    }

    fn clock(&self) -> String {
        ground::clock(self.input.years)
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        let r = self.result();
        Some(
            Share::new(ground::share_line(&self.input, &r))
                .toast("Copied the ground as a .soil line"),
        )
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::simplified("Bearing + settlement: simplified"),
            Badge::illustrative("Soil presets: illustrative"),
            Badge::illustrative("Game costs"),
        ]
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        if !self.playing {
            return;
        }
        let dt = ctx.dt();
        let rate = 1.0 / YEARS_TICK as f32;
        for _ in 0..self.ticker.steps(dt, rate) {
            match ground::play_tick(self.input.years) {
                Some(y) => self.input.years = y,
                None => {
                    self.playing = false;
                    break;
                }
            }
        }
        if !self.playing {
            return;
        }
        let moving = (self.input.quake) || self.input.soil.wet();
        if moving {
            self.anim = (self.anim + dt) % 1000.0;
            ctx.animate();
        } else {
            ctx.wake_in(self.ticker.due_in(rate));
        }
    }

    fn receive(&mut self, carry: Carry, _ctx: &mut Ctx) {
        if let Carry::Building {
            floors,
            overturning_nm,
            ..
        } = carry
        {
            let ft = FromTower {
                floors,
                overturning: overturning_nm,
            };
            self.set_input(GroundInput {
                floors: ft.ground_floors(),
                ..self.input
            });
            self.from_tower = Some(ft);
        }
    }

    fn describe(&self) -> String {
        ground::stage_label(&self.input, &self.result())
    }
}
