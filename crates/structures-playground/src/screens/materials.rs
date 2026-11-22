//! SCR-5 Playground › Materials — a tensile test (SPEC MT-1…MT-6), mockup
//! `docs/design/mockup/Materials.dc.html` (reference `docs/design/screenshots/Materials-1440.png`
//! / `-390.png`).
//!
//! The model is [`sp_engine::materials`]: the screen holds a [`Bench`] and draws its
//! [`View`] (every number and sentence comes from there, DONE-2):
//!
//! - left (`parts`): Sample tiles with 2.5D swatches and the note, the Temperature slider and
//!   note, the Strain axis segmented control, "Coming to this bench";
//! - centre: "Pull it until it breaks" with the status pill, the tensile rig (`rig`) and the
//!   stress–strain plot (`plot`) side by side (stacked when narrow), the Stretch bar (slider,
//!   readout, Fresh sample);
//! - right (`readouts`): six tiles, Strength per kilogram, Challenge · Break all five, What's
//!   happening;
//! - header: Pull / Pause, Reset, the clock `ε = 0.00 %`, Share (a `.material` line), the
//!   provenance badges.
//!
//! Pull advances 20 steps a second ([`TICK_SECONDS`]); while it runs the screen asks for a
//! repaint only when the next step is due (`Ctx::wake_in`), so it never busy-repaints and
//! idles at 0 % CPU (PLAT-3).

mod draw;
mod parts;
mod plot;
mod readouts;
mod rig;

use super::{Badge, Drive, Screen, Share};
use crate::ctx::Ctx;
use crate::layout::{self, Column};
use crate::widgets::text::{css_line_height, galley, text_top};
use crate::widgets::{Slider, ghost};
use draw::{paint_pill, para, pill_size, tone_color};
use eframe::egui::{
    CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, accesskit::Role, pos2, vec2,
};
use sp_engine::materials::{Bench, Kind, TICK_SECONDS, View, Zoom};
use sp_theme as t;

/// The page row: Parts · Stage · Readouts (the mockup's `aside` / `main` / `aside`).
pub const COLUMNS: [Column; 3] = [
    Column::parts("Parts"),
    // `scroll`: stacked (narrow), the line grows to the content instead of clipping it.
    Column::main("Stage")
        .flex(999.0, 1.0, 520.0)
        .pad(16.0, 16.0)
        .gap(12.0)
        .scroll(true),
    Column::readouts("Readouts"),
];

/// "Pull it until it breaks" and its subtitle.
pub const TITLE: &str = "Pull it until it breaks";
/// See [`TITLE`].
pub const SUBTITLE: &str =
    "A tensile test: grip a dog-bone sample, stretch it, record force against stretch.";

/// SCR-5 state: the bench.
#[derive(Debug, Default)]
pub struct MaterialsScreen {
    bench: Bench,
}

/// # Control API
///
/// What the screen's controls do, for tests and the MCP bridge (`set_material`).
impl MaterialsScreen {
    /// The starting state: steel, 20 °C, the 0–25 % axis.
    pub fn new() -> Self {
        Self::default()
    }

    /// The bench (sample, temperature, axis, stretch, challenge).
    pub fn bench(&self) -> &Bench {
        &self.bench
    }

    /// The bench, to drive it directly (MCP `set_material`, tests).
    pub fn bench_mut(&mut self) -> &mut Bench {
        &mut self.bench
    }

    /// Everything shown right now.
    pub fn view(&self) -> View {
        self.bench.view()
    }

    /// Picks a sample (a sample tile).
    pub fn pick(&mut self, kind: Kind) {
        self.bench.pick(kind);
    }

    /// Sets the temperature, °C (clamped to the slider's −40…600).
    pub fn set_temp(&mut self, t_c: f64) {
        self.bench.set_temp(t_c);
    }

    /// Sets the strain axis.
    pub fn set_zoom(&mut self, zoom: Zoom) {
        self.bench.set_zoom(zoom);
    }

    /// Scrubs the stretch, % (pauses Pull).
    pub fn set_strain(&mut self, strain: f64) {
        self.bench.set_strain(strain);
    }
}

impl Screen for MaterialsScreen {
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let v = self.bench.view();
        let bench = &mut self.bench;
        layout::columns(ui, ctx, &COLUMNS, |ui, _ctx, i| match i {
            0 => parts::show(ui, bench, &v),
            1 => stage(ui, bench, &v),
            _ => readouts::show(ui, &v),
        });
    }

    fn drive_button(&self) -> Option<Drive> {
        Some(Drive::new("Pull").running(self.bench.playing()))
    }

    fn drive(&mut self, ctx: &mut Ctx) {
        self.bench.toggle_drive();
        if self.bench.playing() {
            ctx.wake_in(TICK_SECONDS as f32);
        }
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.bench.reset();
    }

    fn clock(&self) -> String {
        self.bench.clock()
    }

    fn share(&mut self, _ctx: &mut Ctx) -> Option<Share> {
        Some(Share::new(self.bench.share()).toast("Copied the sample as a .material line"))
    }

    fn provenance(&self) -> Vec<Badge> {
        vec![
            Badge::simplified("Curves: representative"),
            Badge::illustrative("Heat effects: illustrative"),
            Badge::illustrative("Not certified data"),
        ]
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        if !self.bench.playing() {
            return;
        }
        self.bench.advance(ctx.dt() as f64);
        if let Some(next) = self.bench.next_tick_in() {
            ctx.wake_in(next as f32);
        }
    }

    fn describe(&self) -> String {
        self.bench.describe()
    }
}

/// The centre column: title row, rig + plot, the Stretch bar.
fn stage(ui: &mut Ui, bench: &mut Bench, v: &View) {
    title_row(ui, v);
    rig_and_plot(ui, v);
    stretch_bar(ui, bench, v);
}

/// `display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap:
/// 8px 12px`: the h1 (Space Grotesk 600 22) over the subtitle (13 px muted, gap 2), and the
/// status `.pill`.
fn title_row(ui: &mut Ui, v: &View) {
    let width = ui.available_width();
    let h1 = lab_theme::font::heading(22.0);
    let h1_h = css_line_height(ui, &h1);
    let sub = lab_theme::font::body(13.0);
    let sub_h = css_line_height(ui, &sub);
    let tg = galley(ui, TITLE, h1, t::TEXT);
    let pill = pill_size(ui, &v.status_label);
    let one_line_sub = galley(ui, SUBTITLE, sub.clone(), t::TEXT_MUTED).size().x;
    let block_w = tg.size().x.max(one_line_sub);
    let same_line = block_w + 12.0 + pill.x <= width;
    let text_w = if same_line { block_w } else { width };
    let sg = para(
        ui,
        SUBTITLE,
        sub,
        t::TEXT_MUTED,
        text_w.max(1.0),
        Some(sub_h),
    );
    let block_h = h1_h + 2.0 + sg.size().y;
    let h = if same_line {
        block_h.max(pill.y)
    } else {
        block_h + 8.0 + pill.y
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let top = if same_line {
        rect.center().y - block_h / 2.0
    } else {
        rect.top()
    };
    let p = ui.painter();
    p.galley(
        pos2(rect.left(), text_top(top + h1_h / 2.0, &tg)),
        tg,
        t::TEXT,
    );
    p.galley(pos2(rect.left(), top + h1_h + 2.0), sg, t::TEXT_MUTED);
    let pill_rect = if same_line {
        Rect::from_min_size(
            pos2(rect.right() - pill.x, rect.center().y - pill.y / 2.0),
            pill,
        )
    } else {
        Rect::from_min_size(pos2(rect.left(), rect.top() + block_h + 8.0), pill)
    };
    let colour = tone_color(v.status_tone);
    paint_pill(ui, pill_rect, &v.status_label, colour, colour);
    let label = v.status_label.clone();
    ui.interact(pill_rect, ui.scope_id().with("status"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Status, true, &label));
    let heading = ui.interact(
        Rect::from_min_size(rect.min, vec2(text_w, h1_h)),
        ui.scope_id().with("title"),
        Sense::hover(),
    );
    heading.widget_info(|| WidgetInfo::labeled(Role::Heading, true, TITLE));
}

/// The rig (`flex: 1 1 300px; aspect-ratio: 420 / 560`) and the plot (`flex: 999 1 380px;
/// aspect-ratio: 560 / 400`) in a wrapping row with `gap: 12px`; side by side both take the
/// taller one's height (`align-items: stretch`).
pub fn rig_and_plot_rects(origin: eframe::egui::Pos2, width: f32) -> (Rect, Rect, f32) {
    let gap = 12.0;
    if 300.0 + gap + 380.0 <= width {
        let free = width - 300.0 - gap - 380.0;
        let rig_w = 300.0 + free / 1000.0;
        let plot_w = 380.0 + free * 999.0 / 1000.0;
        let h = (rig_w * 560.0 / 420.0).max(plot_w * 400.0 / 560.0);
        let rig = Rect::from_min_size(origin, vec2(rig_w, h));
        let plot = Rect::from_min_size(origin + vec2(rig_w + gap, 0.0), vec2(plot_w, h));
        (rig, plot, h)
    } else {
        let w = width.max(0.0);
        let rig_h = w * 560.0 / 420.0;
        let plot_h = w * 400.0 / 560.0;
        let rig = Rect::from_min_size(origin, vec2(w, rig_h));
        let plot = Rect::from_min_size(origin + vec2(0.0, rig_h + gap), vec2(w, plot_h));
        (rig, plot, rig_h + gap + plot_h)
    }
}

fn rig_and_plot(ui: &mut Ui, v: &View) {
    let width = ui.available_width();
    let (rig_r, plot_r, h) = rig_and_plot_rects(ui.cursor().min, width);
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let shift = rect.min - rig_r.min;
    rig::show(ui, rig_r.translate(shift), v);
    plot::show(ui, plot_r.translate(shift), v);
}

/// The Stretch bar: a `.panel` (`padding: 12px 16px; gap: 12px 24px; flex-wrap: wrap;
/// align-items: center`) with the slider block (`flex: 1 1 320px`: "Stretch" 13 px 600 and the
/// mono accent readout over the range) and "Fresh sample".
fn stretch_bar(ui: &mut Ui, bench: &mut Bench, v: &View) {
    let width = ui.available_width();
    let inner_w = width - 2.0 - 32.0;
    let label_font = lab_theme::font::semibold(13.0);
    let row_h = css_line_height(ui, &label_font);
    let block_h = row_h + 2.0 + t::size::RANGE;
    let button = ghost("Fresh sample");
    let bsize = button.desired_size(ui);
    let same_line = 320.0 + 24.0 + bsize.x <= inner_w;
    let content_h = if same_line {
        block_h.max(bsize.y)
    } else {
        block_h + 12.0 + bsize.y
    };
    let h = content_h + 24.0 + 2.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(t::radius::PANEL),
        t::PANEL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let inner = Rect::from_min_size(rect.min + vec2(17.0, 13.0), vec2(inner_w, content_h));
    let block_w = if same_line {
        inner_w - 24.0 - bsize.x
    } else {
        inner_w
    };
    let block_top = if same_line {
        inner.center().y - block_h / 2.0
    } else {
        inner.top()
    };
    // The label row (baselines aligned).
    let lg = galley(ui, "Stretch", label_font, t::TEXT);
    let vg = galley(ui, &v.strain_label, lab_theme::font::mono(12.0), t::ACCENT);
    let cy = block_top + row_h / 2.0;
    let p = ui.painter();
    p.galley(pos2(inner.left(), text_top(cy, &lg)), lg, t::TEXT);
    let vx = inner.left() + block_w - vg.size().x;
    p.galley(pos2(vx, text_top(cy, &vg) + 0.5), vg, t::ACCENT);
    let value_rect = Rect::from_min_size(pos2(inner.left(), block_top), vec2(block_w, row_h));
    let label = format!("Stretch: {}", v.strain_label);
    ui.interact(
        value_rect,
        ui.scope_id().with("stretch-value"),
        Sense::hover(),
    )
    .widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
    // The range.
    let zoom = bench.zoom();
    let slider_rect = Rect::from_min_size(
        pos2(inner.left(), block_top + row_h + 2.0),
        vec2(block_w, t::size::RANGE),
    );
    let mut child = ui.new_child(eframe::egui::UiBuilder::new().max_rect(slider_rect));
    let mut e = bench.strain() as f32;
    if Slider::new("Stretch", &mut e, 0.0..=zoom.max() as f32)
        .step(zoom.slider_step() as f32)
        .width(block_w)
        .show(&mut child)
        .changed()
    {
        // Snap in f64 like the browser's range input (n / 20, n / 200).
        let inv = (1.0 / zoom.slider_step()).round();
        bench.set_strain((e as f64 * inv).round() / inv);
    }
    // Fresh sample.
    let brect = if same_line {
        Rect::from_min_size(
            pos2(inner.right() - bsize.x, inner.center().y - bsize.y / 2.0),
            bsize,
        )
    } else {
        Rect::from_min_size(pos2(inner.left(), block_top + block_h + 12.0), bsize)
    };
    if button.show_at(ui, brect).clicked() {
        bench.fresh_sample();
    }
}
