//! Text helpers: one-line galleys (with CSS letter-spacing), wrapped paragraphs with a CSS
//! `line-height`, the `.lbl` eyebrow and plain text blocks.

use eframe::egui::{
    Align, Color32, FontId, Galley, Rect, Response, Sense, Ui, WidgetInfo,
    accesskit::Role,
    epaint::text::{LayoutJob, TextFormat, TextWrapping},
    pos2, vec2,
};
use sp_theme as t;
use std::sync::Arc;

pub use lab_ui::paint::{galley, galley_spaced, text_top, text_width};

/// `.lbl` letter-spacing: `.08em` of 11 px.
pub const LBL_SPACING: f32 = 0.88;

/// Lays out `text` wrapped at `width` with a CSS `line-height` in px (`None`: normal) and
/// `letter-spacing` in px. `break_anywhere` is CSS `word-break: break-all`.
pub fn wrapped(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    line_height: Option<f32>,
) -> Arc<Galley> {
    wrapped_job(ui, text, font, color, width, line_height, 0.0, false)
}

/// [`wrapped`] with every option.
#[allow(clippy::too_many_arguments)]
pub fn wrapped_job(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    line_height: Option<f32>,
    letter_spacing: f32,
    break_anywhere: bool,
) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font,
            color,
            line_height,
            extra_letter_spacing: letter_spacing,
            valign: Align::Center,
            ..Default::default()
        },
    );
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        break_anywhere,
        ..Default::default()
    };
    let g = ui.painter().layout_job(job);
    match line_height {
        Some(h) => css_rows(g, h),
        None => g,
    }
}

/// Re-places a galley's rows on CSS line boxes `line_h` apart, with Chrome's baseline in each
/// (half-leading + rounded ascent, like [`text_top`]). egui rounds every row's height to whole
/// pixels (12 px / 1.65 = 19.8 gives 20 px rows, 1.6 px of drift over nine lines) and centres
/// glyphs its own way; Chrome keeps fractional line boxes and snaps only the glyphs. The
/// galley becomes `n · line_h` tall.
pub fn css_rows(galley: Arc<Galley>, line_h: f32) -> Arc<Galley> {
    let Some(glyph) = galley.rows.iter().find_map(|r| r.row.glyphs.first()) else {
        return galley;
    };
    let ascent = glyph.font_ascent.round();
    let descent = (glyph.font_height - glyph.font_ascent).round();
    // from a line box's top to the row's top, so the baseline lands where Chrome puts it
    let offset = (line_h - (ascent + descent)) / 2.0 + ascent - glyph.pos.y;
    let row_y = |i: usize| (i as f32 * line_h + offset).round();
    let height = galley.rows.len().max(1) as f32 * line_h;
    let placed = galley
        .rows
        .iter()
        .enumerate()
        .all(|(i, r)| r.pos.y == row_y(i));
    if placed && galley.rect.height() == height {
        return galley;
    }
    let mut g = (*galley).clone();
    for (i, row) in g.rows.iter_mut().enumerate() {
        row.pos.y = row_y(i);
    }
    g.rect.max.y = g.rect.min.y + height;
    let rows = g.rows.iter().fold(g.rect, |r, row| {
        r.union(row.rect().translate(g.rect.min.to_vec2()))
    });
    g.mesh_bounds = g.mesh_bounds.union(rows.expand(2.0));
    Arc::new(g)
}

/// One line truncated with `…` to fit `width` (CSS `text-overflow: ellipsis`).
pub fn ellipsized(ui: &Ui, text: &str, font: FontId, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_string(), font, color);
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}

/// The height of a CSS `line-height: normal` line box for `font`, as Chrome computes it:
/// `round(ascent) + round(descent)` (e.g. Inter 11 px → 14, Space Grotesk 17 px → 22).
pub fn css_line_height(ui: &Ui, font: &FontId) -> f32 {
    let g = ui
        .painter()
        .layout_no_wrap("Ag".into(), font.clone(), Color32::WHITE);
    match g.rows.first().and_then(|r| r.row.glyphs.first()) {
        Some(gl) => gl.font_ascent.round() + (gl.font_height - gl.font_ascent).round(),
        None => g.size().y,
    }
}

/// A single line in a CSS line box `line_h` tall starting at `top` (baseline placed like
/// Chrome).
pub fn paint_line(
    painter: &eframe::egui::Painter,
    g: &Arc<Galley>,
    left: f32,
    top: f32,
    line_h: f32,
    color: Color32,
) {
    painter.galley(
        pos2(left, text_top(top + line_h / 2.0, g)),
        g.clone(),
        color,
    );
}

/// Allocates and paints a text block (wrapped to the available width) and gives it an
/// AccessKit label. Returns the response of the block.
pub fn text_block(
    ui: &mut Ui,
    text: &str,
    font: FontId,
    color: Color32,
    line_height: Option<f32>,
) -> Response {
    let width = ui.available_width();
    let g = wrapped(ui, text, font, color, width, line_height);
    let (rect, resp) =
        ui.allocate_exact_size(vec2(g.size().x.min(width), g.size().y), Sense::hover());
    ui.painter().galley(rect.min, g, color);
    let s = text.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    resp
}

/// A single line of text, allocated at its own size.
pub fn line(ui: &mut Ui, text: &str, font: FontId, color: Color32) -> Response {
    let g = galley(ui, text, font, color);
    let (rect, resp) = ui.allocate_exact_size(g.size(), Sense::hover());
    ui.painter().galley(rect.min, g, color);
    let s = text.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    resp
}

/// `.lbl` — the uppercase section eyebrow: Inter 600 11 px, letter-spacing .08em,
/// [`TEXT_MUTED`](sp_theme::TEXT_MUTED). The accessible name keeps the source casing.
pub fn lbl(ui: &mut Ui, text: &str) -> Response {
    lbl_colored(ui, text, t::TEXT_MUTED)
}

/// [`lbl`] in another colour (the Series cards use [`INK`](sp_theme::INK)).
pub fn lbl_colored(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let g = lbl_galley(ui, text, color);
    let h = css_line_height(ui, &lab_theme::font::semibold(11.0));
    let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x, h), Sense::hover());
    ui.painter()
        .galley(pos2(rect.left(), text_top(rect.center().y, &g)), g, color);
    let s = text.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    resp
}

/// The galley of a `.lbl` eyebrow.
pub fn lbl_galley(ui: &Ui, text: &str, color: Color32) -> Arc<Galley> {
    galley_spaced(
        ui,
        &text.to_uppercase(),
        lab_theme::font::semibold(11.0),
        color,
        LBL_SPACING,
    )
}

/// A row with a `.lbl` on the left and a caption on the right, baselines aligned (`display:
/// flex; justify-content: space-between; align-items: baseline`), e.g. "SPEED · 10 gen/s".
pub fn lbl_row(
    ui: &mut Ui,
    label: &str,
    caption: &str,
    caption_font: FontId,
    caption_color: Color32,
) {
    let width = ui.available_width();
    let lg = lbl_galley(ui, label, t::TEXT_MUTED);
    let cg = galley(ui, caption, caption_font, caption_color);
    let h = lg.size().y.max(cg.size().y);
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let base = |g: &Galley| {
        g.rows.first().map_or(g.size().y, |r| {
            r.row.glyphs.first().map_or(r.size.y, |gl| gl.pos.y)
        })
    };
    let baseline = rect.top() + base(&lg).max(base(&cg));
    let ly = baseline - base(&lg);
    let cy = baseline - base(&cg);
    let painter = ui.painter();
    painter.galley(pos2(rect.left(), ly), lg, t::TEXT_MUTED);
    painter.galley(pos2(rect.right() - cg.size().x, cy), cg, caption_color);
    let (l, c) = (label.to_string(), caption.to_string());
    let id = ui.scope_id().with(("lbl_row", label));
    let r = ui.interact(
        Rect::from_min_size(rect.min, vec2(width, h)),
        id,
        Sense::hover(),
    );
    r.widget_info(|| WidgetInfo::labeled(Role::Label, true, format!("{l}: {c}")));
}
