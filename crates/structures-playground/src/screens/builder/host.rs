//! Where the fold-ins plug into the builder (SPEC "Fold-ins": CMF, TIME, LAB, FIRE).
//!
//! The screen owns this file; the fold-in UIs live in `builder/{comfort,time,lab,fire}.rs`
//! (agents `time` and `lab`). Each defines one `Default` struct implementing [`FoldIn`]; every
//! method has a default, so a fold-in implements only what it needs. The screen holds one of
//! each in [`FoldIns`] and calls them in the order comfort, time, lab, fire:
//!
//! | hook | when | who uses it |
//! |---|---|---|
//! | [`FoldIn::tick`] | once a frame before drawing | fire heats pieces, the 30-year animation, the sample pull |
//! | [`FoldIn::adjust_view`] | after `Game::view()`, before painting | TIME-1 settled positions / breaks per year |
//! | [`FoldIn::hud`] | the stage's HUD chips, after the core's | "Fire 650 °C", "Year 12.0", "Tilt 1.8°", "Settled 214 mm" |
//! | [`FoldIn::paint_stage`] | over the structure, under joints and handles | flames, glowing pieces |
//! | [`FoldIn::conditions`] | the end of the Conditions panel | FIRE-1's Fire row + "Light it now" |
//! | [`FoldIn::results`] | the aside's result slot under the goal panel | CMF-1 meter, "Compare shapes", "Fast-forward 30 years" |
//! | [`FoldIn::open_sample`] | a Materials card was clicked | LAB-1 |
//! | [`FoldIn::drawer`] | instead of the aside while open | LAB-1's sample drawer |
//! | [`FoldIn::escape`] | Esc, before it ends a chain | close the drawer |
//! | [`FoldIn::drive`] / [`FoldIn::drive_button`] | the header drive / goal Run press | TIME-3 (the Leaning tower's run is the fast-forward) |
//! | [`FoldIn::reset`] | back to building, Reset, Clear, a level or scene picked, a netlist loaded | drop run-only state |
//! | [`FoldIn::busy`] | every frame | keeps frames coming while a fold-in animates |
//! | [`FoldIn::describe`] | MCP `describe_screen` | extra words for the summary |

use super::bits;
use super::paint::Xf;
use crate::ctx::Ctx;
use crate::screens::Drive;
use eframe::egui::{Color32, Painter, Ui};
use sp_engine::builder::{Game, PieceType, View};
use sp_theme as t;

/// A chip in the stage's HUD (top right): `<key> <value>`, the value in `color` (from
/// `sp_theme`).
#[derive(Debug, Clone, PartialEq)]
pub struct HudChip {
    /// The muted key ("Fire").
    pub key: String,
    /// The value ("650 °C").
    pub value: String,
    /// The value's colour.
    pub color: Color32,
}

impl HudChip {
    /// A chip.
    pub fn new(key: impl Into<String>, value: impl Into<String>, color: Color32) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            color,
        }
    }
}

/// A fold-in's hooks into the builder. See the module docs for when each runs.
#[allow(unused_variables)]
pub trait FoldIn {
    /// Once a frame before anything is drawn (time passes). Ask for frames with
    /// `ctx.wake_in(1.0 / 30.0)` while something moves (PLAT-3).
    fn tick(&mut self, game: &mut Game, ctx: &mut Ctx) {}

    /// Adjust what the stage draws this frame (TIME-1: settled joint positions, broken pieces,
    /// bands for the year shown).
    fn adjust_view(&self, game: &Game, view: &mut View) {}

    /// Extra HUD chips after the core's.
    fn hud(&self, game: &Game, chips: &mut Vec<HudChip>) {}

    /// Paint on the stage over the structure (under joints, handles and overlays). `xf` maps
    /// world metres to points (`xf.p([x, y])`, `xf.ppm()`, `xf.l(stage px)`).
    fn paint_stage(&self, game: &Game, view: &View, painter: &Painter, xf: &Xf) {}

    /// Rows at the end of the Conditions panel (draw at the cursor, allocate what you draw;
    /// [`bits::chip_row`] gives the look).
    fn conditions(&mut self, ui: &mut Ui, game: &mut Game, ctx: &mut Ctx) {}

    /// Sections in the aside's result slot right under the goal panel. Draw at the cursor
    /// and allocate what you draw; draw nothing when there is nothing to show ([`card`] gives
    /// the `.well` look).
    fn results(&mut self, ui: &mut Ui, game: &mut Game, ctx: &mut Ctx) {}

    /// A Materials card was clicked: open the sample drawer for `piece`. `true` if handled.
    fn open_sample(&mut self, piece: PieceType) -> bool {
        false
    }

    /// While open, draw the drawer in the aside column instead of the aside and return
    /// `true`.
    fn drawer(&mut self, ui: &mut Ui, game: &mut Game, ctx: &mut Ctx) -> bool {
        false
    }

    /// Esc was pressed: `true` if consumed (Esc then doesn't end the member chain).
    fn escape(&mut self) -> bool {
        false
    }

    /// The drive button or the goal panel's Run button was pressed: `true` to take it.
    fn drive(&mut self, game: &mut Game, ctx: &mut Ctx) -> bool {
        false
    }

    /// The header's drive button while this fold-in drives (`None`: the core's).
    fn drive_button(&self, game: &Game) -> Option<Drive> {
        None
    }

    /// Back to building, Reset, Clear, a level or scene picked, a netlist loaded: drop
    /// run-only state.
    fn reset(&mut self, game: &Game) {}

    /// Something this fold-in animates is moving.
    fn busy(&self) -> bool {
        false
    }

    /// Extra words for the one-line summary (MCP `describe_screen`).
    fn describe(&self, game: &Game) -> Option<String> {
        None
    }
}

/// The four fold-ins, called in this order.
#[derive(Debug, Default)]
pub struct FoldIns {
    /// CMF-1 / CMF-2 (agent `time`).
    pub comfort: super::comfort::Comfort,
    /// TIME-1 / TIME-3 (agent `time`).
    pub time: super::time::Time,
    /// LAB-1 (agent `lab`).
    pub lab: super::lab::Lab,
    /// FIRE-1 (agent `lab`).
    pub fire: super::fire::Fire,
}

impl FoldIns {
    /// Each fold-in, mutably, in calling order.
    pub fn each(&mut self) -> [&mut dyn FoldIn; 4] {
        [
            &mut self.comfort,
            &mut self.time,
            &mut self.lab,
            &mut self.fire,
        ]
    }

    /// Each fold-in, read-only, in calling order.
    pub fn each_ref(&self) -> [&dyn FoldIn; 4] {
        [&self.comfort, &self.time, &self.lab, &self.fire]
    }
}

/// A `.well` block (padding 12, gap 10, radius 8) labelled `label` for assistive tech.
pub fn card(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    bits::framed(ui, label, t::WELL, t::radius::WELL, add);
}

/// A Conditions-style row: the caption over wrapping chips (see [`bits::chip_row`]).
pub use bits::{ChipSpec, chip_row, chip_rows, label_node, para};
