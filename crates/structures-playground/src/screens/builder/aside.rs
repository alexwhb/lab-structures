//! The aside (`<aside aria-label="Level, site and inspector">`, BLD-1, BLD-11…BLD-13): the
//! level groups with best stars, the goal panel (goal, chips, status pill, cost, Run), the
//! fold-ins' result slot, the Sandbox scene chips, Conditions (wind + direction, snow,
//! earthquake + "Shake now", soil, traffic, then the fold-ins' rows), the piece / joint
//! inspector or the Materials list (each card opens the sample drawer), "Open in Advanced" and
//! What's happening. While the sample drawer is open it takes the aside's place.

use super::Action;
use super::bits::{
    ChipSpec, bar, chip_group, chip_rows, eyebrow, framed, info_pill, info_pill_width, label_node,
    lbl_wrapped, level_row, para, pill_height, section, status_pill,
};
use super::host::FoldIns;
use super::scene;
use super::stage::status_color;
use crate::ctx::Ctx;
use crate::widgets::text::{css_line_height, galley, text_top, wrapped};
use crate::widgets::{Callout, Icon, ghost, interact, prim};
use eframe::egui::{CornerRadius, Id, Rect, Stroke, StrokeKind, Ui, accesskit::Role, pos2, vec2};
use lab_ui::paint::focus_ring_if;
use sp_engine::builder::Game;
use sp_engine::builder::view::{
    BarTone, Chip, Inspector, JointInspector, MaterialCard, PieceInspector, View,
};
use sp_theme::{self as t, builder as b};

/// Draws the aside's content (inside the column's padding).
pub fn show(
    ui: &mut Ui,
    ctx: &mut Ctx,
    game: &mut Game,
    fold: &mut FoldIns,
    v: &View,
    actions: &mut Vec<Action>,
) {
    for f in fold.each() {
        if f.drawer(ui, game, ctx) {
            return;
        }
    }
    levels(ui, v, actions);
    goal(ui, v, actions);
    for f in fold.each() {
        f.results(ui, game, ctx);
    }
    if v.is_sandbox {
        let specs: Vec<ChipSpec> = v
            .sb_scenes
            .iter()
            .map(|c| ChipSpec::new(c.name.clone(), c.on))
            .collect();
        if let Some(i) = chip_group(ui, "scenes", "Sandbox scene", &specs)
            && let Some(c) = v.sb_scenes.get(i)
        {
            actions.push(Action::Scene(c.value));
        }
    }
    conditions(ui, ctx, game, fold, v, actions);
    match &v.inspector {
        Inspector::Piece(pi) => piece(ui, pi, actions),
        Inspector::Joint(ji) => joint(ui, ji, actions),
        Inspector::Materials(cards) => materials(ui, cards, actions),
    }
    if ghost("Open in Advanced")
        .icon(Icon::Chart)
        .full_width()
        .centered()
        .show(ui)
        .clicked()
    {
        actions.push(Action::OpenInAdvanced);
    }
    Callout::happening(&v.explain).show(ui);
}

/// The level groups (`<section gap 6>`, each group a `.lbl` heading over its `.lvl` rows).
fn levels(ui: &mut Ui, v: &View, actions: &mut Vec<Action>) {
    section(ui, "Levels", 6.0, |ui| {
        for g in &v.level_groups {
            let w = ui.available_width();
            let top = ui.cursor().min;
            // `<h2 class="lbl" style="margin-top: 4px">` then rows, gap 4
            let lg = crate::widgets::text::lbl_galley(ui, g.name, t::TEXT_MUTED);
            let lh = css_line_height(ui, &lab_theme::font::semibold(11.0));
            let head = Rect::from_min_size(pos2(top.x, top.y + 4.0), vec2(lg.size().x, lh));
            ui.painter().galley(
                pos2(head.left(), head.top() + (lh - lg.size().y) / 2.0),
                lg,
                t::TEXT_MUTED,
            );
            label_node(ui, ("builder-group", g.name), head, g.name);
            let mut y = head.bottom() + 4.0;
            for row in &g.items {
                let rect = Rect::from_min_size(pos2(top.x, y), vec2(w, t::size::ROW));
                let stars = (!row.best.is_empty())
                    .then(|| row.best.chars().filter(|c| *c == '★').count() as u8);
                if level_row(
                    ui,
                    Id::unique(("builder-level", row.key.key())),
                    rect,
                    &row.name,
                    stars,
                    row.on,
                )
                .clicked()
                {
                    actions.push(Action::Level(row.key));
                }
                y += t::size::ROW + 4.0;
            }
            ui.advance_cursor_after_rect(Rect::from_min_max(top, pos2(top.x + w, y - 4.0)));
        }
    });
}

/// The goal panel (`.panel`, padding 12, gap 10).
fn goal(ui: &mut Ui, v: &View, actions: &mut Vec<Action>) {
    framed(ui, "Goal", t::PANEL, t::radius::PANEL, |ui| {
        para(ui, &v.goal, 13.0, 1.5, t::TEXT_BODY);
        // the chips (`flex-wrap: wrap; gap: 6px`)
        if !v.chips_info.is_empty() {
            let w = ui.available_width();
            let top = ui.cursor().min;
            let ph = pill_height(ui);
            let (mut x, mut y) = (0.0f32, 0.0f32);
            for c in &v.chips_info {
                let cw = info_pill_width(ui, c);
                if x > 0.0 && x + cw > w {
                    x = 0.0;
                    y += ph + 6.0;
                }
                info_pill(ui, pos2(top.x + x, top.y + y), c);
                x += cw + 6.0;
            }
            ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(w, y + ph)));
        }
        // status pill · cost (`justify-content: space-between; gap: 8px`)
        let top = ui.cursor().min;
        let iw = ui.available_width();
        let cost_color = if v.over_budget {
            t::DANGER_INK
        } else {
            t::TEXT
        };
        let mono = lab_theme::font::mono(13.0);
        let cg = galley(ui, &v.cost_text, mono.clone(), cost_color);
        // the cost wraps under narrow widths as the mockup's does ("$20.8k /" "$22k")
        let pill_max = (iw - cg.size().x - 8.0).max(iw * 0.5);
        let pr = status_pill(
            ui,
            top,
            &v.status.label,
            status_color(v.status.tone),
            pill_max,
        );
        let cost_w = (iw - pr.width() - 8.0).max(40.0);
        let cg = wrapped(
            ui,
            &v.cost_text,
            mono.clone(),
            cost_color,
            cost_w,
            Some(css_line_height(ui, &mono)),
        );
        let cr = Rect::from_min_size(
            pos2(top.x + iw - cg.size().x, pr.center().y - cg.size().y / 2.0),
            cg.size(),
        );
        ui.painter().galley(cr.min, cg, cost_color);
        label_node(ui, "builder-cost", cr, &format!("Cost {}", v.cost_text));
        let bottom = pr.bottom().max(cr.bottom());
        ui.advance_cursor_after_rect(Rect::from_min_max(top, pos2(top.x + iw, bottom)));
        if prim(v.run_label)
            .big()
            .full_width()
            .centered()
            .show(ui)
            .clicked()
        {
            actions.push(Action::RunToggle);
        }
    });
}

/// Conditions (`.well`, padding 12, gap 10): the heading (+ "Shake now" while a quake can
/// start), then the rows; the fold-ins add theirs at the end.
fn conditions(
    ui: &mut Ui,
    ctx: &mut Ctx,
    game: &mut Game,
    fold: &mut FoldIns,
    v: &View,
    actions: &mut Vec<Action>,
) {
    let c = &v.conditions;
    framed(ui, "Conditions", t::WELL, t::radius::WELL, |ui| {
        // heading row
        let top = ui.cursor().min;
        let w = ui.available_width();
        let title = "Conditions · change them any time";
        let shake_w = c
            .can_shake
            .then(|| ghost("Shake now").height(30.0).desired_size(ui).x);
        // the heading wraps beside "Shake now" (`justify-content: space-between; gap: 8px`)
        let title_w = shake_w.map_or(w, |sw| w - sw - 8.0);
        let lg = lbl_wrapped(ui, title, title_w);
        let head_h = if shake_w.is_some() {
            lg.size().y.max(30.0)
        } else {
            lg.size().y
        };
        let ly = top.y + (head_h - lg.size().y) / 2.0;
        ui.painter()
            .galley(pos2(top.x, ly), lg.clone(), t::TEXT_MUTED);
        label_node(
            ui,
            "builder-cond-title",
            Rect::from_min_size(pos2(top.x, ly), lg.size()),
            title,
        );
        if let Some(sw) = shake_w {
            let r = Rect::from_min_size(
                pos2(top.x + w - sw, top.y + (head_h - 30.0) / 2.0),
                vec2(sw, 30.0),
            );
            if ghost("Shake now").height(30.0).show_at(ui, r).clicked() {
                actions.push(Action::ShakeNow);
            }
        }
        ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(w, head_h)));
        let chips = |list: &[Chip<f64>]| -> Vec<ChipSpec> {
            list.iter()
                .map(|c| ChipSpec::new(c.name.clone(), c.on).dim(c.dim))
                .collect()
        };
        let winds = chips(&c.winds);
        let dirs = chips(&c.dirs);
        match chip_rows(
            ui,
            "Wind",
            &[("winds", "Wind", &winds), ("dirs", "Wind direction", &dirs)],
        ) {
            Some((0, i)) => actions.push(Action::Wind(c.winds[i].value)),
            Some((_, i)) => actions.push(Action::Dir(c.dirs[i].value)),
            None => {}
        }
        let snows = chips(&c.snows);
        if let Some((_, i)) = chip_rows(ui, "Snow", &[("snows", "Snow", &snows)]) {
            actions.push(Action::Snow(c.snows[i].value));
        }
        let quakes = chips(&c.quakes);
        if let Some((_, i)) = chip_rows(ui, "Earthquake", &[("quakes", "Earthquake", &quakes)]) {
            actions.push(Action::Quake(c.quakes[i].value));
        }
        let soils: Vec<ChipSpec> = c
            .soils
            .iter()
            .map(|s| {
                ChipSpec::new(s.name.clone(), s.on)
                    .dim(s.dim)
                    .dot(scene::soil_look(s.value).top)
            })
            .collect();
        if let Some((_, i)) = chip_rows(ui, "Soil", &[("soils", "Soil", &soils)]) {
            actions.push(Action::Soil(c.soils[i].value));
        }
        if c.show_traffic {
            let traffic: Vec<ChipSpec> = c
                .traffic
                .iter()
                .map(|s| ChipSpec::new(s.name.clone(), s.on).dim(s.dim))
                .collect();
            if let Some((_, i)) = chip_rows(ui, "Traffic", &[("traffic", "Traffic", &traffic)]) {
                actions.push(Action::Traffic(c.traffic[i].value));
            }
        }
        for f in fold.each() {
            f.conditions(ui, game, ctx);
        }
    });
}

/// A title (Space Grotesk 15) with a right-hand note on its baseline.
fn title_row(ui: &mut Ui, salt: &str, title: &str, note: &str, mono: bool) {
    let top = ui.cursor().min;
    let iw = ui.available_width();
    let tg = galley(ui, title, lab_theme::font::heading(15.0), t::TEXT);
    let font = if mono {
        lab_theme::font::mono(12.0)
    } else {
        lab_theme::font::body(12.0)
    };
    let ng = galley(ui, note, font, t::TEXT_MUTED);
    ui.painter().galley(top, tg.clone(), t::TEXT);
    let base = top.y + tg.size().y;
    let nr = Rect::from_min_size(
        pos2(top.x + iw - ng.size().x, base - ng.size().y),
        ng.size(),
    );
    ui.painter().galley(nr.min, ng, t::TEXT_MUTED);
    label_node(
        ui,
        (salt, "title"),
        Rect::from_min_size(top, tg.size()),
        title,
    );
    if !note.is_empty() {
        label_node(ui, (salt, "note"), nr, note);
    }
    ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, tg.size().y)));
}

/// The piece inspector.
fn piece(ui: &mut Ui, pi: &PieceInspector, actions: &mut Vec<Action>) {
    framed(ui, "Piece", t::WELL, t::radius::WELL, |ui| {
        title_row(ui, "builder-pi", &pi.title, &pi.len, false);
        // utilisation bar + percent
        let top = ui.cursor().min;
        let iw = ui.available_width();
        let ug = galley(ui, &pi.ut, lab_theme::font::mono(13.0), t::TEXT);
        let h = ug.size().y.max(8.0);
        let color = match pi.tone {
            BarTone::Over => t::DANGER,
            BarTone::Pulled => t::TENSION,
            BarTone::Pushed => t::COMPRESSION,
        };
        let bar_r = Rect::from_min_size(
            pos2(top.x, top.y + h / 2.0 - 4.0),
            vec2(iw - ug.size().x - 10.0, 8.0),
        );
        bar(ui.painter(), bar_r, pi.fill as f32 / 100.0, color);
        let ur = Rect::from_min_size(pos2(top.x + iw - ug.size().x, top.y), ug.size());
        ui.painter().galley(ur.min, ug, t::TEXT);
        label_node(ui, "builder-pi-ut", ur, &format!("{} of its limit", pi.ut));
        ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, h)));
        para(ui, &pi.text, 13.0, 1.5, t::TEXT_BODY);
        let specs: Vec<ChipSpec> = pi
            .types
            .iter()
            .map(|c| ChipSpec::new(c.name.clone(), c.on).dim(c.dim))
            .collect();
        if let Some(i) = chip_group(ui, "pi-types", "Change this piece", &specs) {
            actions.push(Action::SetPieceType(pi.id.clone(), pi.types[i].value));
        }
        if !pi.locked
            && ghost("Delete piece")
                .full_width()
                .centered()
                .show(ui)
                .clicked()
        {
            actions.push(Action::DeletePiece(pi.id.clone()));
        }
    });
}

/// The joint inspector.
fn joint(ui: &mut Ui, ji: &JointInspector, actions: &mut Vec<Action>) {
    framed(ui, "Joint", t::WELL, t::radius::WELL, |ui| {
        title_row(ui, "builder-ji", &ji.title, &ji.pos, true);
        let specs: Vec<ChipSpec> = ji
            .founds
            .iter()
            .map(|c| ChipSpec::new(c.name.clone(), c.on).dim(c.dim))
            .collect();
        if let Some(i) = chip_group(ui, "ji-founds", "Foundation", &specs) {
            actions.push(Action::SetFoundation(ji.id.clone(), ji.founds[i].value));
        }
        // Crate − value +
        let top = ui.cursor().min;
        let iw = ui.available_width();
        let lg = crate::widgets::text::lbl_galley(ui, "Crate", t::TEXT_MUTED);
        let cy = top.y + t::size::BUTTON / 2.0;
        ui.painter()
            .galley(pos2(top.x, text_top(cy, &lg)), lg.clone(), t::TEXT_MUTED);
        let mut x = top.x + lg.size().x + 8.0;
        let less = Rect::from_min_size(pos2(x, top.y), vec2(40.0, t::size::BUTTON));
        if ghost("−")
            .centered()
            .accessible_name("Lighter crate")
            .show_at(ui, less)
            .clicked()
        {
            actions.push(Action::CrateLess(ji.id.clone()));
        }
        x += 48.0;
        let vg = galley(ui, &ji.load, lab_theme::font::mono(14.0), t::TEXT);
        let vw = vg.size().x.max(54.0);
        let vr = Rect::from_min_size(pos2(x, top.y), vec2(vw, t::size::BUTTON));
        ui.painter().galley(
            pos2(vr.center().x - vg.size().x / 2.0, text_top(cy, &vg)),
            vg,
            t::TEXT,
        );
        label_node(ui, "builder-ji-load", vr, &format!("Crate {}", ji.load));
        x += vw + 8.0;
        let more = Rect::from_min_size(pos2(x, top.y), vec2(40.0, t::size::BUTTON));
        if ghost("+")
            .centered()
            .accessible_name("Heavier crate")
            .show_at(ui, more)
            .clicked()
        {
            actions.push(Action::CrateMore(ji.id.clone()));
        }
        ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(iw, t::size::BUTTON)));
        para(ui, &ji.note, 12.0, 1.5, t::TEXT_MUTED);
        if ji.can_delete
            && ghost("Delete joint")
                .full_width()
                .centered()
                .show(ui)
                .clicked()
        {
            actions.push(Action::DeleteJoint(ji.id.clone()));
        }
    });
}

/// "Materials · how each one pulls and pushes": a card per piece type (curve glyph, name,
/// blurb, $/m); clicking a card opens the sample drawer (LAB-1).
fn materials(ui: &mut Ui, cards: &[MaterialCard], actions: &mut Vec<Action>) {
    section(ui, "Materials", 8.0, |ui| {
        eyebrow(ui, "Materials · how each one pulls and pushes");
        for c in cards {
            if material_card(ui, c) {
                actions.push(Action::Sample(c.kind));
            }
        }
        para(
            ui,
            "Right of centre: pulled. Left: pushed. Taller means stronger.",
            11.0,
            1.25,
            t::TEXT_MUTED,
        );
    });
}

/// One Materials card (`gap: 10px; font-size: 12px; line-height: 1.4`); returns `true` when
/// clicked. AccessKit: a `Button` "<Name> · <blurb> · <cost>" ("opens the sample lab").
fn material_card(ui: &mut Ui, c: &MaterialCard) -> bool {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let font = lab_theme::font::body(12.0);
    let bold = lab_theme::font::semibold(12.0);
    let mono = lab_theme::font::mono(12.0);
    let cg = galley(ui, &c.cost, mono, t::TEXT_MUTED);
    let text_w = (w - 58.0 - 10.0 - 10.0 - cg.size().x).max(40.0);
    // "<b>Name</b> blurb" wrapped as one paragraph
    let mut job = eframe::egui::text::LayoutJob::default();
    let lh = 12.0 * 1.4;
    let fmt = |f: eframe::egui::FontId, color| eframe::egui::TextFormat {
        font_id: f,
        color,
        line_height: Some(lh),
        valign: eframe::egui::Align::Center,
        ..Default::default()
    };
    job.append(c.name, 0.0, fmt(bold, t::TEXT));
    job.append(" ", 0.0, fmt(font.clone(), t::TEXT_MUTED));
    job.append(&c.blurb, 0.0, fmt(font, t::TEXT_MUTED));
    job.wrap.max_width = text_w;
    let tg = crate::widgets::text::css_rows(ui.painter().layout_job(job), lh);
    let h = tg.size().y.max(32.0);
    let rect = Rect::from_min_size(top, vec2(w, h));
    let label = format!("{} · {} · {}", c.name, c.blurb, c.cost);
    let resp = interact(
        ui,
        Id::unique(("builder-card", c.name)),
        rect,
        true,
        Role::Button,
        &label,
        None,
    );
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(rect.expand(2.0), CornerRadius::same(6), t::SW_HOVER);
    }
    let cy = rect.center().y;
    // the glyph: 56 × 30 (+ border), axes, the curve in the swatch colour
    let g = Rect::from_min_size(pos2(top.x, cy - 16.0), vec2(58.0, 32.0));
    painter.rect(
        g,
        CornerRadius::same(4),
        t::WELL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let o = g.min + vec2(1.0, 1.0);
    let p = |q: [f64; 2]| o + vec2(q[0] as f32, q[1] as f32);
    let axes = Stroke::new(1.0, b::GLYPH_AXES);
    painter.line_segment([p([28.0, 2.0]), p([28.0, 28.0])], axes);
    painter.line_segment([p([2.0, 15.0]), p([54.0, 15.0])], axes);
    let sw = scene::look(c.kind).swatch;
    for line in &c.curve_lines {
        let pts: Vec<_> = line.iter().map(|q| p(*q)).collect();
        if pts.len() >= 2 {
            painter.add(eframe::egui::Shape::line(pts, Stroke::new(1.8, sw)));
        }
    }
    painter.galley(pos2(g.right() + 10.0, cy - tg.size().y / 2.0), tg, t::TEXT);
    painter.galley(
        pos2(rect.right() - cg.size().x, text_top(cy, &cg)),
        cg,
        t::TEXT_MUTED,
    );
    focus_ring_if(ui, &resp, rect.expand(2.0), CornerRadius::same(6));
    ui.advance_cursor_after_rect(rect);
    resp.clicked()
}
