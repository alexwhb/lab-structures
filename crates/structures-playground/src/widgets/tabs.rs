//! The world tab row (NAV-2), drawn to the mockup's `<nav aria-label="Playground worlds"
//! style="display: flex; flex-wrap: wrap; align-items: center; gap: 4px 20px; padding: 0 16px;
//! border-bottom: 1px solid #242424; background: rgba(12,12,12,.85)">` with the `.tab` links
//! (Inter 500 13 px, `min-height: 44px; padding: 0 4px`, 20 px apart; the current one
//! `#F5F5F5` with a 2 px accent underline) and the screen's provenance badges (PROV-1) after
//! them, pushed right (`margin-left: auto; flex-wrap: wrap; gap: 8px; padding: 6px 0`).

use super::badge::{BadgeLook, badge_size, paint_badge};
use super::flow::{FlowItem, flow};
use super::text::{galley, text_top};
use super::{hit, interact};
use crate::router::Route;
use crate::screens::Badge;
use eframe::egui::{CornerRadius, Rect, Stroke, Ui, UiBuilder, accesskit::Role, pos2, vec2};
use lab_ui::paint::{advance_block, focus_ring_if};
use sp_theme as t;

/// Row padding (left / right).
pub const PAD_X: f32 = 16.0;
/// Gap between the tabs (and the badge group): 20 px across.
pub const GAP_X: f32 = 20.0;
/// Gap between wrapped rows.
pub const GAP_Y: f32 = 4.0;
/// A tab's horizontal padding.
const TAB_PAD: f32 = 4.0;
/// Gap between badges.
const BADGE_GAP: f32 = 8.0;
/// The badge group's vertical padding.
const BADGES_PAD_Y: f32 = 6.0;

fn tab_font() -> eframe::egui::FontId {
    lab_theme::font::medium(13.0)
}

/// Draws the world tab row at the cursor across the available width (a `TabList` named
/// "Playground worlds") with `badges` right-aligned (a `Group` named "Provenance"). Returns
/// the tab clicked (other than the current one).
pub fn world_tabs(ui: &mut Ui, route: Route, badges: &[Badge]) -> Option<Route> {
    let outer = ui.available_rect_before_wrap();
    let (left, top, width) = (outer.left(), outer.top(), outer.width());
    let inner_w = (width - 2.0 * PAD_X).max(0.0);
    let tab_ws: Vec<f32> = Route::WORLDS
        .iter()
        .map(|r| {
            galley(ui, r.tab_label().unwrap_or_default(), tab_font(), t::TEXT)
                .size()
                .x
                + 2.0 * TAB_PAD
        })
        .collect();
    let badge_items: Vec<FlowItem> = badges
        .iter()
        .map(|b| FlowItem::fixed(badge_size(ui, b)))
        .collect();
    let natural = flow(&badge_items, f32::INFINITY, vec2(BADGE_GAP, BADGE_GAP), 0.0);
    let natural_w = natural.rects.last().map_or(0.0, |r| r.right().max(0.0));
    let mut group = vec2(natural_w, natural.height + 2.0 * BADGES_PAD_Y);
    let mut items: Vec<FlowItem> = tab_ws
        .iter()
        .map(|w| FlowItem::fixed(vec2(*w, t::size::TAB_ROW)))
        .collect();
    let mut lay;
    let mut inner;
    let mut passes = 0;
    loop {
        passes += 1;
        items.truncate(tab_ws.len());
        if !badges.is_empty() {
            items.push(FlowItem::fixed(group).auto().shrink());
        }
        lay = flow(&items, inner_w, vec2(GAP_X, GAP_Y), t::size::TAB_ROW);
        let w = lay.rects.get(tab_ws.len()).map_or(0.0, |r| r.width());
        inner = flow(&badge_items, w, vec2(BADGE_GAP, BADGE_GAP), 0.0);
        let h = inner.height + 2.0 * BADGES_PAD_Y;
        if badges.is_empty() || (h - group.y).abs() < 0.5 || passes >= 3 {
            break;
        }
        group.y = h;
    }
    let row = Rect::from_min_size(pos2(left, top), vec2(width, lay.height + 1.0));
    let painter = ui.painter();
    painter.rect_filled(row, CornerRadius::ZERO, t::BAR);
    painter.hline(
        row.x_range(),
        row.bottom() - 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    let origin = vec2(left + PAD_X, top);
    let mut clicked = None;

    // ---- tabs
    let tabs_rect = lay.rects[..tab_ws.len()]
        .iter()
        .fold(Rect::NOTHING, |a, r| a.union(*r))
        .translate(origin);
    let nav = ui.new_child(
        UiBuilder::new()
            .id_salt("sp-world-tabs")
            .max_rect(row)
            .accessibility_role(Role::TabList)
            .accessibility_label("Playground worlds"),
    );
    super::set_bounds(&nav, tabs_rect);
    for (i, r) in Route::WORLDS.into_iter().enumerate() {
        let tr = lay.rects[i].translate(origin);
        let on = r == route;
        let label = r.tab_label().unwrap_or_default();
        let resp = interact(
            &nav,
            nav.scope_id().with(i),
            hit::target(tr).intersect(tr.expand2(vec2(GAP_X / 2.0, 0.0))),
            true,
            Role::Tab,
            label,
            None,
        );
        nav.ctx()
            .accesskit_node_builder(resp.id, |b| b.set_selected(on));
        let fg = if on || resp.hovered() {
            t::TEXT_STRONG
        } else {
            t::TEXT_MUTED
        };
        let g = galley(&nav, label, tab_font(), fg);
        nav.painter().galley(
            pos2(tr.left() + TAB_PAD, text_top(tr.center().y, &g)),
            g,
            fg,
        );
        if on {
            let underline = Rect::from_min_max(pos2(tr.left(), tr.bottom() - 2.0), tr.max);
            nav.painter()
                .rect_filled(underline, CornerRadius::ZERO, t::ACCENT);
        }
        focus_ring_if(
            &nav,
            &resp,
            tr.shrink2(vec2(0.0, 8.0)),
            CornerRadius::same(4),
        );
        if resp.clicked() && !on {
            clicked = Some(r);
        }
    }

    // ---- provenance badges
    if let Some(g) = lay.rects.get(tab_ws.len()) {
        let g = g.translate(origin);
        let badges_ui = ui.new_child(
            UiBuilder::new()
                .id_salt("sp-provenance")
                .max_rect(g)
                .accessibility_role(Role::Group)
                .accessibility_label("Provenance"),
        );
        super::set_bounds(&badges_ui, g);
        let at = g.min + vec2(0.0, BADGES_PAD_Y);
        for (b, r) in badges.iter().zip(&inner.rects) {
            paint_badge(
                &badges_ui,
                r.translate(at.to_vec2()),
                b,
                BadgeLook::default(),
            );
        }
    }
    advance_block(ui, row);
    clicked
}
