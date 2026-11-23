//! `<select>` — a 36 px drop-down (`padding: 0 8px; border: 1px solid #2E2E2E; border-radius:
//! 6px; background: #121212; font-size: 13px`) with Chrome's chevron. Click (or Enter / Space)
//! opens a [`Menu`] of the options; ↑ / ↓ step through them while it has focus. AccessKit: a
//! `ComboBox` named by its label whose value is the selected option. (From the Advanced
//! screen's Material select.)
//!
//! ```ignore
//! let r = Select::new("Material", &["Steel", "Aluminum"], cur).width(160.0).show(ui, &mut open);
//! if let Some(i) = r.picked { cur = i; }
//! ```

use super::menu::{Menu, MenuItem, menu_button_state};
use super::text::{galley, text_top};
use super::{hit, interact};
use eframe::egui::{
    CornerRadius, Key, Rect, Sense, Shape, Stroke, StrokeKind, Ui, accesskit::Role, pos2, vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_theme as t;

/// What a [`Select`] did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectResponse {
    /// The option picked this frame.
    pub picked: Option<usize>,
    /// The select's rect.
    pub rect: Rect,
}

/// A `<select>` named `name` over `options` with `selected` showing.
#[must_use = "call .show(ui, &mut open)"]
pub struct Select<'a> {
    name: &'a str,
    options: &'a [&'a str],
    selected: usize,
    width: Option<f32>,
}

impl<'a> Select<'a> {
    /// A select.
    pub fn new(name: &'a str, options: &'a [&'a str], selected: usize) -> Self {
        Self {
            name,
            options,
            selected,
            width: None,
        }
    }

    /// Its width (default: the longest option + the chevron).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }

    /// Draws it at the cursor; `open` is the menu's state (keep it in the screen).
    pub fn show(self, ui: &mut Ui, open: &mut bool) -> SelectResponse {
        let font = lab_theme::font::body(13.0);
        let natural = self
            .options
            .iter()
            .map(|o| galley(ui, *o, font.clone(), t::TEXT).size().x)
            .fold(0.0, f32::max)
            + 9.0
            + 30.0;
        let width = self.width.unwrap_or(natural);
        let (rect, _) = ui.allocate_exact_size(vec2(width, t::size::BUTTON), Sense::hover());
        let id = ui.scope_id().with(("select", self.name));
        let resp = interact(
            ui,
            id,
            hit::target(rect),
            true,
            Role::ComboBox,
            self.name,
            None,
        );
        let current = self.options.get(self.selected).copied().unwrap_or("");
        let value = current.to_string();
        ui.ctx()
            .accesskit_node_builder(id, |b| b.set_value(value.clone()));
        menu_button_state(ui, &resp, *open);
        let p = ui.painter();
        p.rect(
            rect,
            CornerRadius::same(t::radius::BUTTON),
            t::ROW,
            Stroke::new(1.0, t::BORDER_STRONG),
            StrokeKind::Inside,
        );
        let g = galley(ui, current, font, t::TEXT);
        p.galley(
            pos2(rect.left() + 9.0, text_top(rect.center().y, &g)),
            g,
            t::TEXT,
        );
        let (cx, cy) = (rect.right() - 13.0, rect.center().y);
        p.add(Shape::line(
            vec![
                pos2(cx - 4.0, cy - 2.0),
                pos2(cx, cy + 2.0),
                pos2(cx + 4.0, cy - 2.0),
            ],
            Stroke::new(1.5, t::SELECT_CHEVRON),
        ));
        focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::BUTTON));
        let mut picked = None;
        let keyboard_open = resp.has_focus()
            && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        if resp.clicked() || keyboard_open {
            *open = !*open;
        }
        if resp.has_focus() && !*open && !self.options.is_empty() {
            let (up, down) =
                ui.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)));
            if up && self.selected > 0 {
                picked = Some(self.selected - 1);
            }
            if down && self.selected + 1 < self.options.len() {
                picked = Some(self.selected + 1);
            }
        }
        if *open {
            let items: Vec<MenuItem> = self.options.iter().map(|o| MenuItem::new(o)).collect();
            let m = Menu::new(id.with("menu"), self.name, rect, &items)
                .focus_first(keyboard_open)
                .show(ui.ctx());
            if let Some(i) = m.clicked {
                picked = Some(i);
            }
            if m.close || m.clicked.is_some() {
                *open = false;
            }
        }
        SelectResponse { picked, rect }
    }
}
