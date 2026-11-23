//! FIRE-1 in the builder (owner: agent `lab`): the Fire row at the end of Conditions (Off /
//! Small / Big / Inferno, "Light it now" while running, a note on where and when it burns),
//! the fire on the stage (its bed, flames that grow with it, the haze of its reach), the glow
//! on hot pieces, and the HUD chip "Fire 650 °C". Every number comes from
//! `sp_engine::builder::fire` (the engine heats the pieces; this file only draws).

use super::bits::{ChipSpec, chip_rows, para};
use super::host::{FoldIn, HudChip};
use super::paint::{Xf, fill_any, polyline, quad};
use crate::ctx::Ctx;
use crate::widgets::ghost;
use eframe::egui::{Color32, Painter, Pos2, Ui};
use sp_engine::builder::fire::{self, FireLevel, heat};
use sp_engine::builder::{Game, Phase, PieceType, Scene, View};
use sp_theme as t;
use sp_theme::fire as tf;

/// The accessible names of the Fire chips (the visible labels are the level names; "Off" alone
/// would clash with the Earthquake row's).
pub const CHIP_NAMES: [&str; 4] = ["Fire off", "Small fire", "Big fire", "Inferno"];

/// The fold-in (stateless: the fire lives in the engine's simulation).
#[derive(Debug, Default)]
pub struct Fire;

/// The Fire level the conditions are set to.
pub fn level(game: &Game) -> FireLevel {
    FireLevel::from_temp(game.cond().fire)
}

/// Whether "Light it now" shows: running, Fire set, not burning yet.
pub fn can_light(game: &Game) -> bool {
    game.phase() == Phase::Run && game.sim().is_some_and(fire::can_light)
}

impl FoldIn for Fire {
    fn hud(&self, game: &Game, chips: &mut Vec<HudChip>) {
        if game.live()
            && let Some(sim) = game.sim()
            && let Some(v) = fire::hud_value(sim)
        {
            chips.push(HudChip::new("Fire", v, tf::HUD_FIRE));
        }
    }

    fn paint_stage(&self, game: &Game, view: &View, painter: &Painter, xf: &Xf) {
        if !game.live() {
            return;
        }
        let Some(sim) = game.sim() else {
            return;
        };
        let b = fire::blaze(sim);
        let set = level(game).is_on();
        if set || b.burning() {
            paint_fire(painter, xf, &b, sim.t, game.scene(), set);
        }
        for m in &view.members {
            let Some(c) = tf::glow(m.temp as f32) else {
                continue;
            };
            let look = sp_theme::builder::PIECES[piece_index(m.kind)];
            // decks are drawn as slabs (no line width): glow along a 7 px line
            let w = xf.l(if look.width > 0.0 { look.width } else { 7.0 });
            let (a, z) = (xf.pt(m.a), xf.pt(m.b));
            let line: Vec<Pos2> = match m.sag {
                Some(s) => quad(a, xf.pt(s), z, 12),
                None => vec![a, z],
            };
            let halo = c.gamma_multiply(0.35);
            polyline(painter, &line, w + xf.l(10.0), halo, true);
            polyline(painter, &line, (w * 0.9).max(xf.l(2.0)), c, true);
        }
    }

    fn conditions(&mut self, ui: &mut Ui, game: &mut Game, _ctx: &mut Ctx) {
        let now = level(game);
        let specs: Vec<ChipSpec> = FireLevel::ALL
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let mut c = ChipSpec::new(l.name(), *l == now).dot(tf::LEVEL_DOT[i]);
                c.name = CHIP_NAMES[i].into();
                c
            })
            .collect();
        if let Some((_, i)) = chip_rows(ui, "Fire", &[("fire", "Fire", &specs)])
            && let Some(l) = FireLevel::ALL.get(i)
        {
            game.set_fire(l.setting());
        }
        if can_light(game)
            && ghost("Light it now").height(30.0).show(ui).clicked()
            && let Some(sim) = game.sim_mut()
        {
            fire::light_now(sim);
        }
        para(
            ui,
            &fire::note(now, game.scene() == Scene::River),
            12.0,
            1.5,
            t::TEXT_MUTED,
        );
    }

    fn describe(&self, game: &Game) -> Option<String> {
        let l = level(game);
        if !l.is_on() {
            return None;
        }
        let mut s = format!(
            "Fire: {} ({} \u{b0}C)",
            l.name(),
            sp_engine::jsnum::to_string(l.temp_c())
        );
        if let Some(sim) = game.sim() {
            if let Some(v) = fire::hud_value(sim) {
                s.push_str(&format!(", burning at {v}"));
            }
            if let Some((i, temp)) = fire::hottest(sim)
                && let Some(c) = sim.c.get(i)
            {
                s.push_str(&format!(
                    ", hottest piece {} at {} \u{b0}C",
                    c.id,
                    sp_engine::jsnum::to_fixed(temp, 0)
                ));
            }
        }
        Some(s)
    }
}

/// `PieceType` → its index in `types()` order (the theme's tables).
fn piece_index(k: PieceType) -> usize {
    PieceType::ALL.iter().position(|p| *p == k).unwrap_or(0)
}

/// The fire at its source: the bed (a burning barge on the river, a heap on open ground), the
/// haze of its reach, and three flame tongues that grow with it and flicker with sim time.
fn paint_fire(painter: &Painter, xf: &Xf, b: &fire::Blaze, t_sim: f64, scene: Scene, set: bool) {
    let s = b.source;
    let ppm = xf.ppm();
    if !(ppm.is_finite() && ppm > 0.0) {
        return;
    }
    // The reach: a dome over the fire (height counts half, so it reaches twice as high; nothing
    // below the fire heats).
    if b.reach > 0.0 {
        let pts: Vec<Pos2> = (0..=32)
            .map(|i| {
                let a = i as f64 / 32.0 * std::f64::consts::PI;
                xf.p([
                    s[0] + b.reach * a.cos(),
                    s[1] + b.reach / heat::PLUME * a.sin(),
                ])
            })
            .collect();
        fill_any(painter, &pts, tf::REACH_HAZE);
    }
    // The bed.
    let (half_w, h) = match scene {
        Scene::River => (3.0, 0.9),
        Scene::Flat => (2.2, 0.8),
    };
    let bed = [
        xf.p([s[0] - half_w, s[1]]),
        xf.p([s[0] + half_w, s[1]]),
        xf.p([s[0] + half_w * 0.8, s[1] + h]),
        xf.p([s[0] - half_w * 0.8, s[1] + h]),
    ];
    fill_any(painter, &bed, tf::FIRE_BED);
    if !set && !b.burning() {
        return;
    }
    let top = s[1] + h;
    // Embers glow on the bed even before it lights.
    for i in 0..4 {
        let x = s[0] - half_w * 0.6 + i as f64 * half_w * 0.4;
        painter.circle_filled(xf.p([x, top - 0.25]), (ppm * 0.22).max(1.0), tf::EMBER);
    }
    if !b.burning() {
        return;
    }
    // Three tongues per layer, outer to core, growing with the fire.
    let grow = b.growth;
    let size = 2.5 + b.reach * 0.45;
    let layers: [(f64, f64, Color32); 3] = [
        (1.0, 1.0, tf::FLAME_OUTER),
        (0.72, 0.7, tf::FLAME_MID),
        (0.45, 0.45, tf::FLAME_CORE),
    ];
    for (hk, wk, colour) in layers {
        for (j, dx) in [-0.55, 0.0, 0.55].iter().enumerate() {
            let flicker = 1.0 + 0.12 * (t_sim * 9.0 + j as f64 * 2.1).sin();
            let height = size * grow * hk * flicker * if j == 1 { 1.0 } else { 0.75 };
            let width = half_w * 0.6 * wk;
            let cx = s[0] + dx * half_w;
            let tip = [cx + 0.15 * (t_sim * 5.0 + j as f64).sin(), top + height];
            let left = [cx - width, top];
            let right = [cx + width, top];
            let mut pts = quad(
                xf.p(left),
                xf.p([cx - width * 0.9, top + height * 0.55]),
                xf.p(tip),
                8,
            );
            pts.extend(quad(
                xf.p(tip),
                xf.p([cx + width * 0.9, top + height * 0.55]),
                xf.p(right),
                8,
            ));
            fill_any(painter, &pts, colour);
        }
    }
}
