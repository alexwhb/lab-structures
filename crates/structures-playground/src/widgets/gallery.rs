//! A gallery of every shared widget in its states, on the `.paper` ground. The widget tests
//! click through it and snapshot it; it's also the quickest way to eyeball a change.

use super::*;
use crate::layout::Column;
use crate::screens::Badge;
use eframe::egui::{self, Align2, Rect, Ui, vec2};
use sp_theme as t;

/// The gallery's interactive state.
#[derive(Debug, Clone)]
pub struct Gallery {
    /// Clicks per button label.
    pub clicks: std::collections::BTreeMap<String, u32>,
    /// The selected truss style (a `.seg` group).
    pub style: usize,
    /// The selected member size (a `.mini` group).
    pub size: usize,
    /// The selected strain axis (a filled `.seg` group).
    pub zoom: usize,
    /// "Exaggerate sag".
    pub sag: bool,
    /// "Show forces".
    pub forces: bool,
    /// A slider value.
    pub floors: f32,
    /// A checkbox.
    pub damper: bool,
    /// The material chip picked.
    pub material: usize,
    /// The select's option and whether its menu is open.
    pub select: usize,
    /// The select's menu is open.
    pub select_open: bool,
    /// The wrapping pick.
    pub pick: usize,
    /// Whether the drive button reads Pause.
    pub running: bool,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            clicks: Default::default(),
            style: 0,
            size: 1,
            zoom: 0,
            sag: true,
            forces: false,
            floors: 60.0,
            damper: false,
            material: 0,
            select: 0,
            select_open: false,
            pick: 0,
            running: false,
        }
    }
}

/// The influence-line-like curve the gallery plots.
pub const CURVE: [f32; 13] = [
    0.0, -180.0, -360.0, -540.0, -720.0, -880.0, -1004.0, -880.0, -720.0, -540.0, -360.0, -180.0,
    0.0,
];

impl Gallery {
    fn click(&mut self, label: &str) {
        *self.clicks.entry(label.to_string()).or_default() += 1;
    }

    /// Draws the gallery into `ui` (meant for a 1440 × 900 window).
    pub fn ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        lab_ui::paint::paint_grid(ui.painter(), full, full.min, &t::PAPER);
        let cols = [
            Column::aside("Buttons", 360.0).flex(1.0, 1.0, 360.0),
            Column::aside("Readouts", 360.0).flex(1.0, 1.0, 360.0),
            Column::aside("Icons", 300.0).flex(1.0, 1.0, 300.0),
        ];
        let lines = crate::layout::resolve(full.width(), &cols);
        for p in lines.iter().flatten() {
            let rect = Rect::from_min_size(full.min + vec2(p.x, 0.0), vec2(p.width, full.height()))
                .shrink(16.0);
            let mut col = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("gallery", p.index))
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            col.spacing_mut().item_spacing = vec2(10.0, 12.0);
            match p.index {
                0 => self.buttons(&mut col),
                1 => self.readouts(&mut col),
                _ => self.icons(&mut col),
            }
        }
    }

    fn buttons(&mut self, ui: &mut Ui) {
        lbl(ui, "Buttons");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (label, icon) = if self.running {
                ("Pause", Icon::Pause)
            } else {
                ("Test", Icon::Play)
            };
            if prim(label).icon(icon).show(ui).clicked() {
                self.running = !self.running;
                self.click("Test");
            }
            if ghost("Reset").icon(Icon::Restart).show(ui).clicked() {
                self.click("Reset");
            }
            if ghost("Share").show(ui).clicked() {
                self.click("Share");
            }
            ghost("Disabled").enabled(false).show(ui);
        });
        if prim("Test it: add the load")
            .icon(Icon::Play)
            .big()
            .show(ui)
            .clicked()
        {
            self.click("Test it: add the load");
        }
        lbl(ui, "Segmented");
        const MODES: [SegItem<'static>; 2] = [
            SegItem::new("Playground").icon(Icon::Truss),
            SegItem::new("Advanced").icon(Icon::Chart),
        ];
        let _ = SegGroup::new("Mode", &MODES, Some(0))
            .style(SegStyle::Header)
            .links()
            .navigation()
            .show(ui);
        const STYLES: [SegItem<'static>; 3] = [
            SegItem::new("Pratt"),
            SegItem::new("Howe"),
            SegItem::new("Warren"),
        ];
        if let Some(i) = SegGroup::new("Truss style", &STYLES, Some(self.style))
            .style(SegStyle::Header)
            .show(ui)
            .clicked
        {
            self.style = i;
        }
        const ZOOMS: [SegItem<'static>; 2] = [SegItem::new("0–25 %"), SegItem::new("0–2 %")];
        if let Some(i) = SegGroup::new("Strain axis range", &ZOOMS, Some(self.zoom))
            .style(SegStyle::Header)
            .fill()
            .show(ui)
            .clicked
        {
            self.zoom = i;
        }
        const PICKS: [SegItem<'static>; 4] = [
            SegItem::new("Top chord T2–T3"),
            SegItem::new("Bottom chord B2–B3"),
            SegItem::new("End post B0–T1"),
            SegItem::new("Vertical B2–T2"),
        ];
        if let Some(i) = SegGroup::new("Member to plot", &PICKS, Some(self.pick))
            .style(SegStyle::Header)
            .wrap()
            .show(ui)
            .clicked
        {
            self.pick = i;
        }
        let r = Select::new(
            "Material",
            &["Steel", "Aluminum", "Glulam", "Carbon fiber"],
            self.select,
        )
        .width(160.0)
        .show(ui, &mut self.select_open);
        if let Some(i) = r.picked {
            self.select = i;
        }
        const SIZES: [SegItem<'static>; 3] = [
            SegItem::new("Thin"),
            SegItem::new("Medium"),
            SegItem::new("Thick"),
        ];
        if let Some(i) = SegGroup::new("Member size", &SIZES, Some(self.size))
            .show(ui)
            .clicked
        {
            self.size = i;
        }
        lbl(ui, "Toggles");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if Toggle::new("Exaggerate sag", self.sag).show(ui).clicked() {
                self.sag = !self.sag;
            }
            if Toggle::new("Show forces", self.forces).show(ui).clicked() {
                self.forces = !self.forces;
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (i, (name, dot)) in [("Steel", t::STEEL.fill), ("Timber", t::TIMBER.fill)]
                .into_iter()
                .enumerate()
            {
                if Toggle::new(name, self.material == i)
                    .dot(dot)
                    .show(ui)
                    .clicked()
                {
                    self.material = i;
                }
            }
            Toggle::new("Roller", false).dimmed(true).show(ui);
        });
        if Checkbox::new("Tuned mass damper at the top", self.damper)
            .show(ui)
            .clicked()
        {
            self.damper = !self.damper;
        }
        let text = format!("{:.0} floors", self.floors);
        Slider::new("Floors", &mut self.floors, 5.0..=120.0)
            .step(1.0)
            .value_text(text)
            .show(ui);
    }

    fn readouts(&mut self, ui: &mut Ui) {
        lbl(ui, "Readouts");
        ui.columns(2, |c| {
            StatTile::new("Top sway", "928 mm").show(&mut c[0]);
            StatTile::new("Safety factor", "2.4")
                .color(t::OK)
                .show(&mut c[1]);
        });
        Callout::happening(
            "The bottom chord is stretched and the top chord is squeezed: the truss works like a deep beam.",
        )
        .show(ui);
        lbl(ui, "Badges");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            badge(ui, &Badge::exact("Solver: linear statics, exact"));
            badge(ui, &Badge::simplified("Materials: representative"));
            badge(ui, &Badge::illustrative("Game costs"));
            badge(ui, &Badge::plain("example: 48 m truss"));
            badge_with(
                ui,
                &Badge::plain("Educational. Not for real structural design."),
                BadgeLook::DANGER,
            );
        });
        Plot::series(&CURVE)
            .height(110.0)
            .playhead(Some(6))
            .label("Force in the top chord as the truck crosses")
            .show(ui);
        Card::well().show(ui, |ui| {
            CodeBlock::new(
                "* river-crossing.str\n.node B0 0 0 fix=xy\n.member M1 B0 B1 chord\n.static linear",
            )
            .show(ui);
        });
    }

    fn icons(&mut self, ui: &mut Ui) {
        lbl(ui, "Icons");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for icon in Icon::ALL {
                let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 6, t::CONTROL);
                let s = icon.css_size();
                icon.paint(
                    ui.painter(),
                    Rect::from_center_size(rect.center(), s),
                    t::TEXT_DIM,
                );
            }
        });
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 40.0), egui::Sense::hover());
        Chip::new("top 928 mm · ").part("H/259", t::ACCENT).show_at(
            ui,
            rect.left_top(),
            Align2::LEFT_TOP,
        );
    }
}
