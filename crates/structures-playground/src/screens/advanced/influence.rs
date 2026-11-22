//! The influence-line panel (ADV-5): `.plot N(M8) vs truck x` with the four picks, the plot in
//! the mockup's `viewBox="0 0 640 220"` (`aspect-ratio: 640 / 220`: axis at y 110, x from 40 to
//! 620, ±95 units for ±scale, clamped at ±1.3) — the fill, the limit dashes, the line, the
//! cursor and its dot — and the three tiles. Side by side when there's room (`flex: 999 1
//! 420px` + `flex: 1 1 200px`, gap 16), stacked otherwise.

use super::controls::{SegWrap, panel_header};
use crate::widgets::text::{css_line_height, galley, lbl_galley, text_top, wrapped};
use eframe::egui::{
    Color32, CornerRadius, Mesh, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use lab_theme::font;
use sp_engine::influence::{InfluenceLine, Pick, Tone, now_text};
use sp_engine::netlist::text::js_number;
use sp_theme::{self as t, advanced as a};

/// What the panel did.
#[derive(Debug, Default)]
pub struct IlOutput {
    /// A pick was clicked (its key).
    pub picked: Option<String>,
}

fn il_color(il: &InfluenceLine) -> Color32 {
    if il.tone() == Tone::Tension {
        a::MEMBER_TENSION
    } else {
        a::MEMBER_COMPRESSION
    }
}

/// The panel's content. `now_kn` is the picked member's force with the truck at `x`.
pub fn influence_panel(
    ui: &mut Ui,
    picks: &[Pick],
    current: Option<&Pick>,
    il: Option<&InfluenceLine>,
    x: f64,
    now_kn: f64,
) -> IlOutput {
    let mut out = IlOutput::default();
    let id = il.map(|l| l.id.clone()).unwrap_or_else(|| "—".into());
    let title = format!(".plot N({id}) vs truck x");
    let names: Vec<String> = picks.iter().map(|p| p.name.clone()).collect();
    let sel = current.and_then(|c| picks.iter().position(|p| p.key == c.key));
    let max_w = ui.available_width() - 28.0;
    let seg_size = SegWrap::new("Member to plot", &names, sel).size(ui, max_w);
    panel_header(ui, &title, seg_size, |ui, r| {
        if !names.is_empty()
            && let Some(i) = SegWrap::new("Member to plot", &names, sel).show_at(ui, r.min, max_w)
        {
            out.picked = picks.get(i).map(|p| p.key.clone());
        }
    });
    let Some(il) = il else {
        let msg =
            "No moving load to sweep: add a line like .load truck 40t x=24 lane=B0..B6 share=0.5";
        let g = wrapped(
            ui,
            msg,
            font::body(13.0),
            t::TEXT_MUTED,
            ui.available_width() - 28.0,
            Some(19.0),
        );
        let (r, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), g.size().y + 24.0),
            Sense::hover(),
        );
        ui.painter()
            .galley(pos2(r.left() + 14.0, r.top() + 12.0), g, t::TEXT_MUTED);
        ui.interact(r, ui.scope_id().with("il-none"), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Label, true, msg));
        return out;
    };

    // body: padding 12 14, the plot and the tiles
    let width = ui.available_width();
    let inner = width - 28.0;
    let origin = ui.cursor().min + vec2(14.0, 12.0);
    let side = inner >= 420.0 + 16.0 + 200.0;
    let (plot_w, tiles_w) = if side {
        let free = inner - 636.0;
        let tiles = 200.0 + free / 1000.0;
        (inner - 16.0 - tiles, tiles)
    } else {
        (inner, inner)
    };
    let aspect_h = plot_w * 220.0 / 640.0;
    let tiles_origin = if side {
        pos2(origin.x + plot_w + 16.0, origin.y)
    } else {
        pos2(origin.x, origin.y + aspect_h + 16.0)
    };
    let color = il_color(il);
    let now_s = now_text(now_kn);
    let peak_s = il.peak_text();
    let at_s = il.peak_at_text();
    let lim_s = il.limit_text();
    let at_label = format!("At x = {} m", js_number(x));
    let mut y = tiles_origin.y;
    y = tile(
        ui,
        pos2(tiles_origin.x, y),
        tiles_w,
        &at_label,
        &now_s,
        color,
        None,
    ) + 8.0;
    y = tile(
        ui,
        pos2(tiles_origin.x, y),
        tiles_w,
        "Worst over the crossing",
        &peak_s,
        t::TEXT,
        Some(&at_s),
    ) + 8.0;
    y = tile(
        ui,
        pos2(tiles_origin.x, y),
        tiles_w,
        "Limit (red dashes)",
        &lim_s,
        t::TEXT,
        None,
    );
    // side by side the plot box stretches to the tiles (`align-items: stretch` beats its
    // aspect ratio) and the SVG inside keeps its own (`preserveAspectRatio` meet, centred)
    let plot_h = if side {
        aspect_h.max(y - origin.y)
    } else {
        aspect_h
    };
    let plot_rect = Rect::from_min_size(origin, vec2(plot_w, plot_h));
    plot(ui, plot_rect, il, x, now_kn);
    let bottom = if side { plot_rect.bottom().max(y) } else { y } + 12.0;
    let used = Rect::from_min_max(ui.cursor().min, pos2(ui.cursor().min.x + width, bottom));
    ui.advance_cursor_after_rect(used);
    out
}

/// A `.well` tile (`padding: 10px 12px; gap: 2px`): `.lbl`, mono 16 value, optional muted
/// 11 px note. Returns its bottom.
fn tile(
    ui: &mut Ui,
    origin: Pos2,
    width: f32,
    label: &str,
    value: &str,
    color: Color32,
    note: Option<&str>,
) -> f32 {
    let lbl_line = css_line_height(ui, &font::semibold(11.0));
    let value_line = css_line_height(ui, &font::mono(16.0));
    let note_line = css_line_height(ui, &font::body(11.0));
    let lg = wrapped_lbl(ui, label, width - 26.0, lbl_line);
    let vg = galley(ui, value, font::mono(16.0), color);
    let ng = note.map(|n| {
        wrapped(
            ui,
            n,
            font::body(11.0),
            t::TEXT_MUTED,
            width - 26.0,
            Some(note_line),
        )
    });
    let lbl_h = lg.rows.len().max(1) as f32 * lbl_line;
    // 1 px border + padding 10 / 12, children `gap: 2px`
    let h = 1.0
        + 10.0
        + lbl_h
        + 2.0
        + value_line
        + ng.as_ref().map(|g| 2.0 + g.size().y).unwrap_or(0.0)
        + 10.0
        + 1.0;
    let rect = Rect::from_min_size(origin, vec2(width, h));
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        t::WELL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let x = rect.left() + 13.0;
    let mut y = rect.top() + 11.0;
    if lg.rows.len() > 1 {
        p.galley(pos2(x, y), lg.clone(), t::TEXT_MUTED);
    } else {
        p.galley(
            pos2(x, text_top(y + lbl_line / 2.0, &lg)),
            lg.clone(),
            t::TEXT_MUTED,
        );
    }
    y += lbl_h + 2.0;
    p.galley(pos2(x, text_top(y + value_line / 2.0, &vg)), vg, color);
    y += value_line;
    if let Some(g) = ng {
        p.galley(pos2(x, y + 2.0), g, t::TEXT_MUTED);
    }
    let s = match note {
        Some(n) => format!("{label}: {value}, {n}"),
        None => format!("{label}: {value}"),
    };
    ui.interact(rect, ui.scope_id().with(("il-tile", label)), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    rect.bottom()
}

/// The `.lbl` eyebrow (uppercase, 11 px 600, `.08em`), wrapped to `width` on `line` px rows.
fn wrapped_lbl(ui: &Ui, text: &str, width: f32, line: f32) -> std::sync::Arc<eframe::egui::Galley> {
    let one = lbl_galley(ui, text, t::TEXT_MUTED);
    if one.size().x <= width {
        return one;
    }
    crate::widgets::text::wrapped_job(
        ui,
        &text.to_uppercase(),
        font::semibold(11.0),
        t::TEXT_MUTED,
        width,
        Some(line),
        crate::widgets::text::LBL_SPACING,
        false,
    )
}

/// The plot in its `.graph` box (`border: 1px solid #242424; border-radius: 8px`).
fn plot(ui: &mut Ui, rect: Rect, il: &InfluenceLine, x: f64, now_kn: f64) {
    let painter = ui.painter().with_clip_rect(rect);
    lab_ui::paint::paint_grid(&painter, rect, rect.min, &t::GRAPH);
    // the viewBox scaled to fit and centred (`preserveAspectRatio="xMidYMid meet"`)
    let k = (rect.width() / 640.0).min(rect.height() / 220.0);
    let vb = Rect::from_center_size(rect.center(), vec2(640.0 * k, 220.0 * k));
    let span = (il.to - il.from).max(1e-9);
    let gx = |xm: f64| vb.left() + (40.0 + ((xm - il.from) / span) as f32 * 580.0) * k;
    let gy = |v: f64| vb.top() + (110.0 - il.level(v) as f32 * 95.0) * k;
    let axis_y = vb.top() + 110.0 * k;
    painter.line_segment(
        [
            pos2(vb.left() + 40.0 * k, axis_y),
            pos2(vb.left() + 620.0 * k, axis_y),
        ],
        Stroke::new(1.0, a::IL_AXIS),
    );
    // limit dashes
    let dash = |y: f32| {
        let mut shapes = Vec::new();
        let (x0, x1) = (vb.left() + 40.0 * k, vb.left() + 620.0 * k);
        let mut s = x0;
        while s < x1 {
            let e = (s + 5.0 * k).min(x1);
            shapes.push(Shape::line_segment(
                [pos2(s, y), pos2(e, y)],
                Stroke::new(1.2 * k, a::IL_LIMIT),
            ));
            s += 10.0 * k;
        }
        shapes
    };
    if il.shows_tension_limit() {
        painter.extend(dash(gy(il.limit_tension)));
    }
    if il.shows_compression_limit() {
        painter.extend(dash(gy(-il.limit_compression)));
    }
    // the fill between the line and the axis (strips, split where the line crosses it)
    let color = il_color(il);
    let fill = color.gamma_multiply(a::IL_FILL_OPACITY);
    let pts: Vec<Pos2> = il.points.iter().map(|p| pos2(gx(p[0]), gy(p[1]))).collect();
    let mut mesh = Mesh::default();
    let mut quad = |a: Pos2, b: Pos2| {
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(a, fill);
        mesh.colored_vertex(b, fill);
        mesh.colored_vertex(pos2(b.x, axis_y), fill);
        mesh.colored_vertex(pos2(a.x, axis_y), fill);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    };
    for w in pts.windows(2) {
        let (p, q) = (w[0], w[1]);
        let (dp, dq) = (p.y - axis_y, q.y - axis_y);
        if dp * dq < 0.0 {
            let tcross = dp / (dp - dq);
            let c = pos2(p.x + (q.x - p.x) * tcross, axis_y);
            quad(p, c);
            quad(c, q);
        } else {
            quad(p, q);
        }
    }
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::line(pts, Stroke::new(2.5 * k, color)));
    // cursor
    let cx = gx(x);
    painter.line_segment(
        [
            pos2(cx, vb.top() + 14.0 * k),
            pos2(cx, vb.top() + 206.0 * k),
        ],
        Stroke::new(1.5 * k, a::IL_CURSOR),
    );
    painter.circle_filled(pos2(cx, gy(now_kn)), 5.0 * k, a::IL_CURSOR);
    // corner labels
    let note = |text: &str, pos: Pos2, right: bool, bottom: bool| {
        let g = galley(ui, text, font::mono(10.0), a::IL_NOTE);
        let x = if right { pos.x - g.size().x } else { pos.x };
        let y = if bottom { pos.y - g.size().y } else { pos.y };
        painter.galley(pos2(x, y), g, a::IL_NOTE);
    };
    note(
        "+ tension",
        pos2(rect.left() + 8.0, rect.top() + 6.0),
        false,
        false,
    );
    note(
        "\u{2212} compression",
        pos2(rect.left() + 8.0, rect.bottom() - 6.0),
        false,
        true,
    );
    note(
        &il.range_text(),
        pos2(rect.right() - 10.0, rect.bottom() - 6.0),
        true,
        true,
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(t::radius::WELL),
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let label = il.label();
    ui.interact(rect, ui.scope_id().with("il-plot"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
}
