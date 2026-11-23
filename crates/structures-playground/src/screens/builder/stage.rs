//! The centre column (BLD-1, BLD-2, BLD-9, BLD-10, BLD-14): the hint bar, the stage (1120 ×
//! 600, aspect kept) and the legend row.
//!
//! The stage paints the engine's view ([`scene`]), lets the fold-ins paint over it, then its
//! HTML layer: the grid surface (an AccessKit button "Building grid": click to build, drag to
//! pan, wheel or pinch to zoom at the pointer), piece handles and joints as AccessKit buttons
//! with the mockup's labels (joints drag with Select), crate tags, the HUD chips, the toast, the
//! run overlay ("2 / 8 across", "Slow motion") and the result banner. During a quake the whole
//! stage shakes.

use super::bits::{hud_pill, label_node, paint_stars, stars_width};
use super::hint::{self, HintBar, HintClick, HintGroup};
use super::host::{FoldIns, HudChip};
use super::paint::{Xf, height_for};
use super::scene::{self, Anim};
use super::{Action, bits::ChipSpec, legend};
use crate::widgets::text::{galley, text_top};
use crate::widgets::{arrows_pass_through, ghost, prim};
use eframe::egui::{
    Color32, CornerRadius, CursorIcon, Id, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_engine::builder::Game;
use sp_engine::builder::view::{Cursor, HudTone, ResultView, StatusTone, View};
use sp_theme::{self as t, builder as b};

/// A core HUD tone's colour.
pub fn hud_color(tone: HudTone) -> Color32 {
    match tone {
        HudTone::Plain => t::TEXT,
        HudTone::Good => t::OK,
        HudTone::Bad => t::DANGER_INK,
        HudTone::Wind => b::HUD_WIND,
        HudTone::Snow => b::HUD_SNOW,
        HudTone::Shake => t::CAUTION,
    }
}

/// A status pill's colour.
pub fn status_color(tone: StatusTone) -> Color32 {
    match tone {
        StatusTone::Muted => t::TEXT_MUTED,
        StatusTone::Ok => t::OK,
        StatusTone::Warn => t::WARN,
        StatusTone::Danger => t::DANGER,
        StatusTone::Running => t::ACCENT,
    }
}

/// The hint bar's chip group for the tool.
fn hint_group(v: &View) -> Option<HintGroup<'static>> {
    if v.show_types {
        Some(HintGroup {
            salt: "types",
            group: "Piece to build",
            caption: None,
            chips: v
                .types
                .iter()
                .map(|c| {
                    let l = scene::look(c.kind);
                    ChipSpec::new(c.name, c.on).bar(l.swatch, c.swatch_h.max(3.0) as f32)
                })
                .collect(),
        })
    } else if v.show_founds {
        Some(HintGroup {
            salt: "founds",
            group: "Foundation type",
            caption: None,
            chips: v
                .founds
                .iter()
                .map(|c| ChipSpec::new(c.name, c.on))
                .collect(),
        })
    } else if v.show_weights {
        Some(HintGroup {
            salt: "weights",
            group: "Weight to add",
            caption: Some("Crate"),
            chips: v
                .weights
                .iter()
                .map(|c| ChipSpec::new(c.name.clone(), c.on))
                .collect(),
        })
    } else {
        None
    }
}

/// Draws the centre column: hint bar, stage, legend row.
pub fn main_column(
    ui: &mut Ui,
    game: &Game,
    fold: &FoldIns,
    v: &View,
    extra_hud: &[HudChip],
    anim: Anim,
    actions: &mut Vec<Action>,
) {
    let group = hint_group(v);
    let bar = HintBar {
        hint: v.hint,
        group,
        finish: v.has_pending,
        forces: v.show_forces,
    };
    match hint::show(ui, &bar) {
        Some(HintClick::Chip(i)) => {
            if v.show_types {
                if let Some(c) = v.types.get(i) {
                    actions.push(Action::Piece(c.kind));
                }
            } else if v.show_founds {
                if let Some(c) = v.founds.get(i) {
                    actions.push(Action::Found(c.kind));
                }
            } else if let Some(c) = v.weights.get(i) {
                actions.push(Action::Weight(c.value));
            }
        }
        Some(HintClick::Finish) => actions.push(Action::Finish),
        Some(HintClick::Forces) => actions.push(Action::ToggleForces),
        None => {}
    }
    let w = ui.available_width();
    let top = ui.cursor().min;
    let rect = Rect::from_min_size(top, vec2(w, height_for(w)));
    stage(ui, rect, game, fold, v, extra_hud, anim, actions);
    ui.advance_cursor_after_rect(rect);
    legend::show(ui, &v.zoom_pct, actions);
}

/// The stage in `rect`.
#[allow(clippy::too_many_arguments)]
fn stage(
    ui: &mut Ui,
    rect: Rect,
    game: &Game,
    fold: &FoldIns,
    v: &View,
    extra_hud: &[HudChip],
    anim: Anim,
    actions: &mut Vec<Action>,
) {
    let shake = if v.shaking {
        scene::shake_offset(anim.shake)
    } else {
        [0.0, 0.0]
    };
    // the SVG fills the content box inside the 1 px border
    let xf = Xf::of(rect.shrink(1.0), &v.camera, shake);
    let mut child = crate::layout::graph_panel(ui, rect, Some(b::STAGE));
    let ui = &mut child;
    let painter = ui.painter().clone();
    painter.rect_filled(rect, CornerRadius::same(t::radius::PANEL), b::STAGE);
    scene::paint(&painter, rect.shrink(1.0), &xf, v, anim);
    for f in fold.each_ref() {
        f.paint_stage(game, v, &painter, &xf);
    }
    // the scene's summary (`role="img" aria-label`)
    let img = ui.interact(rect, Id::unique("builder-stage-img"), Sense::hover());
    let label = v.stage_label.clone();
    img.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));

    // the grid surface: click to build, drag empty space to pan
    let surface = ui.interact(rect, Id::unique("builder-surface"), Sense::click_and_drag());
    let sl = v.surface_label.to_string();
    surface.widget_info(|| WidgetInfo::labeled(Role::Button, true, &sl));
    crate::widgets::reveal_on_focus(&surface);
    if surface.has_focus() {
        arrows_pass_through(ui, surface.id);
    }
    let to_stage = |p: eframe::egui::Pos2| xf.inv(p);
    if surface.drag_started()
        && let Some(o) = ui.input(|i| i.pointer.press_origin())
    {
        let (x, y) = to_stage(o);
        actions.push(Action::SurfaceDown(x, y));
    }
    if surface.clicked()
        && let Some(p) = surface.interact_pointer_pos()
    {
        let (x, y) = to_stage(p);
        actions.push(Action::ClickSurface(x, y));
    }
    // keyboard building: with the grid focused, Enter / Space clicks the stage centre (pan
    // with the arrow keys to move it) and the hover dot shows where
    if surface.clicked() && surface.interact_pointer_pos().is_none() {
        actions.push(Action::ClickSurface(560.0, 300.0));
    }
    let pointer_on_stage = ui
        .input(|i| i.pointer.latest_pos())
        .is_some_and(|p| rect.contains(p));
    if surface.has_focus() && !pointer_on_stage {
        actions.push(Action::PointerMove(560.0, 300.0));
    }
    if surface.hovered() {
        let icon = match v.cursor {
            Cursor::Crosshair => CursorIcon::Crosshair,
            Cursor::Grab => CursorIcon::Grab,
            Cursor::Default => CursorIcon::Default,
        };
        ui.ctx().set_cursor_icon(icon);
    }
    focus_ring_if(
        ui,
        &surface,
        rect.shrink(1.0),
        CornerRadius::same(t::radius::PANEL),
    );

    // pointer moves (hover dot, drags, pans) and the wheel / pinch zoom
    let pointer = ui.input(|i| i.pointer.latest_pos());
    let any_down = ui.input(|i| i.pointer.any_down());
    let dragging = game.dragging().is_some() || surface.dragged();
    match pointer.filter(|p| rect.contains(*p)) {
        Some(p) => {
            let (x, y) = to_stage(p);
            let moved = ui.input(|i| i.pointer.delta() != Vec2::ZERO);
            if moved || dragging {
                actions.push(Action::PointerMove(x, y));
            }
            // `onWheel`: one ×1.15 step per wheel event (deltaY < 0 zooms in); the page
            // doesn't scroll under the stage. ⌘ / ctrl + wheel and pinches come as zoom.
            let (steps, pinch) = ui.input_mut(|i| {
                let steps: Vec<f32> = i
                    .raw
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        eframe::egui::Event::MouseWheel {
                            delta, modifiers, ..
                        } if delta.y != 0.0 && !(modifiers.ctrl || modifiers.command) => {
                            Some(delta.y)
                        }
                        _ => None,
                    })
                    .collect();
                if !steps.is_empty() {
                    i.smooth_scroll_delta = Vec2::ZERO;
                }
                (steps, i.zoom_delta())
            });
            for d in steps {
                // egui's wheel delta is "content moves down" (positive = wheel up): deltaY = −d
                actions.push(Action::Wheel(-(d as f64), x, y));
            }
            if (pinch - 1.0).abs() > 1e-4 {
                actions.push(Action::Pinch(pinch as f64, x, y));
            }
        }
        None => {
            if dragging && let Some(p) = pointer {
                let (x, y) = to_stage(p);
                actions.push(Action::PointerMove(x, y));
            } else if v.hover.is_some() && !surface.has_focus() {
                actions.push(Action::ClearHover);
            }
        }
    }
    if (game.dragging().is_some() && !any_down) || surface.drag_stopped() {
        actions.push(Action::PointerUp);
    }

    handles(ui, &xf, v, actions);
    joints(ui, &xf, v, actions);
    tags(ui, &xf, v);
    hud(ui, rect, v, extra_hud);
    overlays(ui, rect, v, actions);
    crate::layout::graph_panel_border(ui, rect);
}

/// Piece handles (`.mh`, Select / Erase tools or "Show forces"): 14 px dots, or 20 px tags with
/// the force in tonnes.
fn handles(ui: &mut Ui, xf: &Xf, v: &View, actions: &mut Vec<Action>) {
    let painter = ui.painter().clone();
    for h in &v.handles {
        let c = xf.pt(h.at);
        let font = lab_theme::font::mono_medium(10.0);
        let (w, hh, radius) = if h.num {
            let g = galley(ui, &h.text, font.clone(), t::TEXT_DIM);
            ((g.size().x + 14.0).max(14.0), t::size::HANDLE_NUM, 5u8)
        } else {
            (14.0, t::size::HANDLE, 7u8)
        };
        let r = Rect::from_center_size(c, vec2(w, hh));
        let resp = ui.interact(r, Id::unique(("builder-handle", &h.id)), Sense::click());
        let label = h.label.clone();
        resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            actions.push(Action::ClickHandle(h.id.clone()));
        }
        let (border, fg) = if h.selected {
            (t::ACCENT, t::INK)
        } else if h.over {
            (t::DANGER, t::DANGER_SOFT)
        } else {
            (t::BORDER_OVERLAY, t::TEXT_DIM)
        };
        painter.rect(
            r,
            CornerRadius::same(radius),
            b::HANDLE,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        if !h.text.is_empty() {
            let g = galley(ui, &h.text, font, fg);
            painter.galley(pos2(c.x - g.size().x / 2.0, text_top(c.y, &g)), g, fg);
        }
        focus_ring_if(ui, &resp, r, CornerRadius::same(radius));
    }
}

/// Joints (`.joint`: 30 px buttons with an 11 px dot): square-ish on a foundation, glowing
/// while a chain waits on them, haloed when selected, red when sagging or overloaded.
fn joints(ui: &mut Ui, xf: &Xf, v: &View, actions: &mut Vec<Action>) {
    let painter = ui.painter().clone();
    for j in &v.joints {
        let c = xf.pt(j.at);
        let r = Rect::from_center_size(c, Vec2::splat(t::size::JOINT_HIT));
        let resp = ui.interact(
            r,
            Id::unique(("builder-joint", &j.id)),
            Sense::click_and_drag(),
        );
        let label = j.label.clone();
        resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
        crate::widgets::reveal_on_focus(&resp);
        if resp.has_focus() {
            arrows_pass_through(ui, resp.id);
        }
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if resp.drag_started() {
            actions.push(Action::JointDown(j.id.clone()));
        }
        if resp.clicked() {
            actions.push(Action::ClickJoint(j.id.clone()));
        }
        let hovered = resp.hovered();
        let d = t::size::JOINT_DOT * if hovered { 1.25 } else { 1.0 };
        let dot = Rect::from_center_size(c, Vec2::splat(d));
        let radius = if j.founded { 3.0 * d / 11.0 } else { d / 2.0 };
        if j.pending {
            painter.rect_filled(
                dot.expand(5.0),
                CornerRadius::same((radius + 5.0) as u8),
                b::JOINT_PENDING_GLOW,
            );
        } else if j.bad {
            painter.rect_filled(
                dot.expand(4.0),
                CornerRadius::same((radius + 4.0) as u8),
                b::JOINT_BAD_HALO,
            );
        } else if j.selected {
            painter.rect_filled(
                dot.expand(4.0),
                CornerRadius::same((radius + 4.0) as u8),
                b::JOINT_SELECTED_HALO,
            );
        }
        let (fill, ring) = if j.pending {
            (t::ACCENT, t::INK)
        } else if j.bad {
            (b::STAGE, t::DANGER)
        } else if hovered {
            (b::STAGE, t::INK)
        } else if j.founded {
            (b::STAGE, t::ACCENT)
        } else {
            (b::STAGE, b::JOINT_RING)
        };
        painter.rect(
            dot,
            CornerRadius::same(radius.round() as u8),
            fill,
            Stroke::new(2.0, ring),
            StrokeKind::Inside,
        );
        focus_ring_if(ui, &resp, r.shrink(4.0), CornerRadius::same(12));
    }
}

/// Crate tags (`.ltag`, `pointer-events: none`): "5 t" centred under each crate.
fn tags(ui: &Ui, xf: &Xf, v: &View) {
    let painter = ui.painter();
    for c in &v.crates {
        let g = galley(ui, &c.tag, lab_theme::font::mono_medium(11.0), t::LOAD_TAG);
        let a = xf.pt(c.tag_at);
        let r = Rect::from_min_size(
            pos2(a.x - g.size().x / 2.0 - 6.0, a.y),
            vec2(g.size().x + 12.0, g.size().y + 2.0),
        );
        painter.rect_filled(r, CornerRadius::same(4), b::TAG);
        painter.galley(pos2(r.left() + 6.0, r.top() + 1.0), g, t::LOAD_TAG);
    }
}

/// The HUD (top right, `max-width: 60%`, wrapping right-aligned lines, gap 6): the core's
/// chips, then the fold-ins'.
fn hud(ui: &Ui, rect: Rect, v: &View, extra: &[HudChip]) {
    let chips: Vec<(String, String, Color32)> = v
        .hud
        .iter()
        .map(|c| (c.key.trim().to_string(), c.value.clone(), hud_color(c.tone)))
        .chain(
            extra
                .iter()
                .map(|c| (c.key.trim().to_string(), c.value.clone(), c.color)),
        )
        .collect();
    if chips.is_empty() {
        return;
    }
    let sizes: Vec<Vec2> = chips
        .iter()
        .map(|(k, val, c)| hud_pill(ui, None, k, val, *c).0)
        .collect();
    let max_w = rect.width() * 0.6;
    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut lw = 0.0;
    for (i, s) in sizes.iter().enumerate() {
        let cur = lines.last_mut().expect("a line");
        let need = if cur.is_empty() { s.x } else { lw + 6.0 + s.x };
        if need > max_w && !cur.is_empty() {
            lines.push(vec![i]);
            lw = s.x;
        } else {
            cur.push(i);
            lw = need;
        }
    }
    let mut y = rect.top() + 12.0;
    for l in &lines {
        let w: f32 = l.iter().map(|&i| sizes[i].x).sum::<f32>() + 6.0 * (l.len() - 1) as f32;
        let mut x = rect.right() - 12.0 - w;
        let h = l.iter().map(|&i| sizes[i].y).fold(0.0, f32::max);
        for &i in l {
            let (k, val, c) = &chips[i];
            hud_pill(ui, Some(pos2(x, y)), k, val, *c);
            x += sizes[i].x + 6.0;
        }
        y += h + 6.0;
    }
}

/// The toast (top centre), the run overlay (bottom centre) and the result banner (top centre).
fn overlays(ui: &mut Ui, rect: Rect, v: &View, actions: &mut Vec<Action>) {
    let painter = ui.painter().clone();
    let cx = rect.center().x;
    if let Some(r) = &v.result {
        result_banner(ui, rect, r, actions);
    }
    if let Some(text) = &v.toast {
        let g = galley(ui, text, lab_theme::font::body(13.0), t::TEXT);
        let w = (g.size().x + 26.0).min(rect.width() - 8.0);
        let h = g.size().y + 14.0;
        let r = Rect::from_min_size(pos2(cx - w / 2.0, rect.top() + 14.0), vec2(w, h));
        painter.rect(
            r,
            CornerRadius::same(8),
            t::OVERLAY,
            Stroke::new(1.0, t::BORDER_OVERLAY),
            StrokeKind::Inside,
        );
        painter.galley(pos2(r.left() + 13.0, r.top() + 7.0), g, t::TEXT);
        label_node(ui, "builder-toast", r, text);
    }
    if v.running {
        // `bottom: 12px; padding: 8px 14px; gap: 12px; border-radius: 10px; font-size: 13px`
        let g = galley(ui, &v.run_info, lab_theme::font::mono(13.0), t::TEXT);
        let slow = ghost(v.slow_label).height(30.0);
        let ss = slow.desired_size(ui);
        let inner = vec2(g.size().x + 12.0 + ss.x, g.size().y.max(ss.y));
        let size = inner + vec2(30.0, 18.0);
        let r = Rect::from_min_size(pos2(cx - size.x / 2.0, rect.bottom() - 12.0 - size.y), size);
        painter.rect(
            r,
            CornerRadius::same(10),
            t::OVERLAY,
            Stroke::new(1.0, t::BORDER_OVERLAY),
            StrokeKind::Inside,
        );
        let cy = r.center().y;
        let tr = Rect::from_min_size(pos2(r.left() + 15.0, cy - g.size().y / 2.0), g.size());
        painter.galley(pos2(tr.left(), text_top(cy, &g)), g, t::TEXT);
        label_node(ui, "builder-run-info", tr, &v.run_info);
        let br = Rect::from_min_size(pos2(tr.right() + 12.0, cy - ss.y / 2.0), ss);
        if ghost(v.slow_label).height(30.0).show_at(ui, br).clicked() {
            actions.push(Action::ToggleSlow);
        }
    }
}

/// The result banner: title (Space Grotesk 15, green / red), stars or "practice", "Replay in
/// slow motion", "Back to building"; wrapping centred lines (`gap: 8px 12px`, `max-width:
/// 92%`).
fn result_banner(ui: &mut Ui, rect: Rect, r: &ResultView, actions: &mut Vec<Action>) {
    let painter = ui.painter().clone();
    let cx = rect.center().x;
    let color = if r.pass { t::OK } else { t::DANGER };
    let tg = galley(ui, &r.title, lab_theme::font::heading(15.0), color);
    let star_n = r.stars.chars().filter(|c| *c == '★').count() as u8;
    let painted_stars = r.stars.contains('★') || r.stars.contains('☆');
    let sg = (!painted_stars && !r.stars.is_empty())
        .then(|| galley(ui, &r.stars, lab_theme::font::mono(14.0), t::ACCENT));
    let rs = ghost("Replay in slow motion").desired_size(ui);
    let bs = prim("Back to building").desired_size(ui);
    #[derive(Clone, Copy, PartialEq)]
    enum It {
        Title,
        Stars,
        Replay,
        Back,
    }
    let mut items: Vec<(It, Vec2)> = vec![(It::Title, tg.size())];
    if painted_stars {
        items.push((It::Stars, vec2(stars_width(14.0), 14.0)));
    } else if let Some(g) = &sg {
        items.push((It::Stars, g.size()));
    }
    items.push((It::Replay, rs));
    items.push((It::Back, bs));
    let max_w = rect.width() * 0.92 - 30.0;
    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut lw = 0.0;
    for (i, (_, s)) in items.iter().enumerate() {
        let cur = lines.last_mut().expect("a line");
        let need = if cur.is_empty() { s.x } else { lw + 12.0 + s.x };
        if need > max_w && !cur.is_empty() {
            lines.push(vec![i]);
            lw = s.x;
        } else {
            cur.push(i);
            lw = need;
        }
    }
    let line_w = |l: &Vec<usize>| {
        l.iter().map(|&i| items[i].1.x).sum::<f32>() + 12.0 * (l.len().saturating_sub(1)) as f32
    };
    let line_h = |l: &Vec<usize>| l.iter().map(|&i| items[i].1.y).fold(0.0, f32::max);
    let inner_w = lines.iter().map(line_w).fold(0.0, f32::max);
    let inner_h: f32 =
        lines.iter().map(line_h).sum::<f32>() + 8.0 * (lines.len().saturating_sub(1)) as f32;
    let banner = Rect::from_min_size(
        pos2(cx - (inner_w + 30.0) / 2.0, rect.top() + 14.0),
        vec2(inner_w + 30.0, inner_h + 22.0),
    );
    painter.rect(
        banner,
        CornerRadius::same(12),
        b::BANNER,
        Stroke::new(1.0, if r.pass { t::OK_EDGE } else { t::DANGER_EDGE }),
        StrokeKind::Inside,
    );
    let mut y = banner.top() + 11.0;
    for l in &lines {
        let lh = line_h(l);
        let mut x = cx - line_w(l) / 2.0;
        for &i in l {
            let (it, s) = items[i];
            let cell = Rect::from_min_size(pos2(x, y + (lh - s.y) / 2.0), s);
            match it {
                It::Title => {
                    painter.galley(cell.min, tg.clone(), color);
                    label_node(ui, "builder-result", cell, &r.title);
                }
                It::Stars => {
                    if painted_stars {
                        paint_stars(&painter, cell.left(), cell.center().y, star_n, 14.0);
                    } else if let Some(g) = &sg {
                        painter.galley(cell.min, g.clone(), t::ACCENT);
                    }
                    label_node(ui, "builder-stars", cell, &r.stars);
                }
                It::Replay => {
                    if ghost("Replay in slow motion").show_at(ui, cell).clicked() {
                        actions.push(Action::Replay);
                    }
                }
                It::Back => {
                    if prim("Back to building").show_at(ui, cell).clicked() {
                        actions.push(Action::BackToBuild);
                    }
                }
            }
            x += s.x + 12.0;
        }
        y += lh + 8.0;
    }
}
