//! The tools rail (BR-3): `<nav aria-label="Tools">` with Select, Member, Joint, Support,
//! Weight, Erase (pressed = the active tool), a divider, then Undo and Clear.

use super::Action;
use super::bits::tool_button;
use crate::widgets::Icon;
use eframe::egui::{Id, Rect, Ui, pos2, vec2};
use sp_engine::bridge::Tool;
use sp_theme as t;

fn icon(tool: Tool) -> Icon {
    match tool {
        Tool::Select => Icon::Select,
        Tool::Member => Icon::Member,
        Tool::Joint => Icon::Joint,
        Tool::Support => Icon::Support,
        Tool::Weight => Icon::Weight,
        Tool::Erase => Icon::Erase,
    }
}

/// Draws the rail into `ui` (the column's content box) and returns the clicked action.
pub fn show(ui: &mut Ui, active: Tool, actions: &mut Vec<Action>) {
    let left = ui.max_rect().left();
    let mut y = ui.max_rect().top();
    let w = t::size::TOOL_W;
    let x = left + (ui.max_rect().width() - w) / 2.0;
    for tool in Tool::ALL {
        let rect = Rect::from_min_size(pos2(x, y), vec2(w, t::size::TOOL_H));
        let id = Id::unique(("bridge-tool", tool.name()));
        if tool_button(
            ui,
            id,
            rect,
            icon(tool),
            tool.name(),
            tool.tip(),
            Some(active == tool),
        )
        .clicked()
        {
            actions.push(Action::Tool(tool));
        }
        y += t::size::TOOL_H + 6.0;
    }
    // `<div style="height: 1px; background: #242424; margin: 6px 0">`
    let line_y = y + 6.0;
    ui.painter().hline(
        left..=left + ui.max_rect().width(),
        line_y + 0.5,
        eframe::egui::Stroke::new(1.0, t::BORDER),
    );
    y = line_y + 1.0 + 6.0 + 6.0;
    for (label, ic, tip, action) in [
        ("Undo", Icon::Undo, "Undo the last change", Action::Undo),
        (
            "Clear",
            Icon::Clear,
            "Start this level again",
            Action::Clear,
        ),
    ] {
        let rect = Rect::from_min_size(pos2(x, y), vec2(w, t::size::TOOL_H));
        if tool_button(
            ui,
            Id::unique(("bridge-tool", label)),
            rect,
            ic,
            label,
            tip,
            None,
        )
        .clicked()
        {
            actions.push(action);
        }
        y += t::size::TOOL_H + 6.0;
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(
        ui.max_rect().min,
        pos2(ui.max_rect().right(), y - 6.0),
    ));
}
