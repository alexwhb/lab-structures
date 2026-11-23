//! TIME-3: "7 · Leaning tower" (`pisa`, Towers group). Soft clay; a pre-built, stone-heavy
//! tower — concrete walls and diagonals with a Floor at every level, all locked — standing on
//! three shallow footings, with its bells (a crate) hung off-centre at the top right. Left
//! alone it leans a little more every year and passes 4° within 30 years (the P-Δ feedback of
//! [`super`]). The run is the 30-year fast-forward.
//!
//! What fixes it (what engineers did at Pisa): piles (the footings can be swapped: put piles
//! under it, not just under the low side — a lone pile there tips it the other way), more
//! pieces, or crates as a counterweight on the high side. The bells stay, and the tower's own
//! pieces can't be removed. Budget [`BUDGET`]: piles under all three footings ($4.5k) earn
//! three stars; so does a counterweight (crates cost nothing).

use super::{Ending, LongTerm};
use crate::builder::data::{
    Anchor, Foundation, Group, Level, LevelKey, PieceType, RunKind, Scene, Soil, SupportRule,
    stars_for,
};
use crate::builder::design::{Design, Joint, Piece};
use crate::jsnum::fixed;

/// The tower's bays (4 m each).
pub const BAYS: usize = 2;
/// Its levels (4 m storeys).
pub const LEVELS: usize = 8;
/// Its centre line, m.
pub const CENTRE_X: f64 = 100.0;
/// The bells hung at the top right, t.
pub const BELLS_T: f64 = 8.0;
/// The budget, $k.
pub const BUDGET: f64 = 8.0;
/// The goal: under this tilt after 30 years, degrees.
pub const GOAL_TILT: f64 = 0.5;
/// The level name.
pub const NAME: &str = "7 · Leaning tower";
/// The goal sentence (SPEC TIME-3).
pub const GOAL: &str =
    "It leans more every year. Fix it without removing floors: under 0.5° after 30 years.";

/// The x of column `i` (0 = left), m.
fn col_x(i: usize) -> f64 {
    CENTRE_X - BAYS as f64 * 2.0 + i as f64 * 4.0
}

/// The id of the joint at column `i`, level `k` (level 0: the footings `J1`…`J3`).
pub fn joint_id(i: usize, k: usize) -> String {
    format!("J{}", k * (BAYS + 1) + i + 1)
}

/// The footings' joint ids, left to right (`J1`, `J2`, `J3`).
pub fn footing_ids() -> Vec<String> {
    (0..=BAYS).map(|i| joint_id(i, 0)).collect()
}

/// The top floor's joint ids, left to right.
pub fn top_ids() -> Vec<String> {
    (0..=BAYS).map(|i| joint_id(i, LEVELS)).collect()
}

/// The joint the bells hang from (top right).
pub fn bells_joint() -> String {
    joint_id(BAYS, LEVELS)
}

/// The pre-built tower: three unlocked footings on the ground (they may be swapped for piles;
/// the engine refuses to delete or drag a joint that holds a locked piece), locked joints
/// above, locked pieces (columns and diagonals in Concrete, a Floor across every level) and the
/// bells.
pub fn prebuilt() -> Design {
    let mut d = Design::default();
    for k in 0..=LEVELS {
        for i in 0..=BAYS {
            d.nodes.push(Joint {
                id: joint_id(i, k),
                x: col_x(i),
                y: k as f64 * 4.0,
                sup: (k == 0).then_some(Foundation::Footing),
                locked: k > 0,
            });
        }
    }
    d.n_n = d.nodes.len() as u32;
    let add = |d: &mut Design, kind: PieceType, a: String, b: String| {
        d.n_m += 1;
        d.members.push(Piece {
            id: format!("M{}", d.n_m),
            a,
            b,
            kind,
            locked: true,
        });
    };
    for k in 1..=LEVELS {
        for i in 0..=BAYS {
            add(
                &mut d,
                PieceType::Concrete,
                joint_id(i, k - 1),
                joint_id(i, k),
            );
        }
        for i in 0..BAYS {
            add(&mut d, PieceType::Floor, joint_id(i, k), joint_id(i + 1, k));
        }
        for i in 0..BAYS {
            let (a, b) = if (k + i) % 2 == 1 {
                (joint_id(i, k - 1), joint_id(i + 1, k))
            } else {
                (joint_id(i + 1, k - 1), joint_id(i, k))
            };
            add(&mut d, PieceType::Concrete, a, b);
        }
    }
    d.set_load(&bells_joint(), BELLS_T);
    d
}

/// The level's table entry (the engine's [`crate::builder::data::level`] uses it for
/// [`LevelKey::Pisa`]).
pub fn level() -> Level {
    Level {
        key: LevelKey::Pisa,
        group: Group::Towers,
        name: NAME.into(),
        goal: GOAL.into(),
        scene: Some(Scene::Flat),
        soil: Soil::Soft,
        bed: None,
        wind: 0.0,
        snow: 0.0,
        quake: 0.0,
        fire: 0.0,
        budget: Some(BUDGET),
        target: None,
        comfort: None,
        sup: SupportRule::Any,
        traffic: vec![],
        anchors: (0..=BAYS)
            .map(|i| Anchor {
                x: col_x(i),
                y: 0.0,
                found: Foundation::Footing,
            })
            .collect(),
        prebuilt: Some(prebuilt()),
        run: RunKind::Years,
    }
}

/// What the player's fix cost, $k: the design's cost without the level's own (locked) pieces,
/// which cost nothing — like a level's locked foundations (requested of `Design::cost`; the
/// same number once it skips locked pieces).
pub fn fix_cost(d: &Design) -> f64 {
    let mut c = 0.0;
    for m in d.members.iter().filter(|m| !m.locked) {
        c += d.length(m) * m.kind.spec().cost;
    }
    for n in d.nodes.iter().filter(|n| !n.locked) {
        if let Some(f) = n.sup {
            c += f.cost();
        }
    }
    c / 1000.0
}

/// The result of a Leaning tower run (its 30-year fast-forward).
#[derive(Debug, Clone, PartialEq)]
pub struct PisaResult {
    /// Under 0.5° after 30 years, still standing.
    pub pass: bool,
    /// Stars (0 when failed, practice or over budget).
    pub stars: u8,
    /// The banner title.
    pub title: String,
    /// What's happening: the result line, why, and the stars sentence.
    pub explain: String,
    /// The tilt after 30 years (or when it fell), degrees.
    pub tilt: f64,
}

/// Scores a Leaning tower fast-forward: passes when it stood 30 years under [`GOAL_TILT`];
/// stars by cost against the budget as every level (none for a practice run).
pub fn result(lt: &LongTerm, cost: f64, practice: bool, budget: f64) -> PisaResult {
    let tilt = lt.last().map_or(0.0, |f| f.tilt.abs());
    let pass = matches!(lt.ending, Ending::Stood) && lt.is_tower() && tilt < GOAL_TILT;
    let stars = if pass && !practice {
        stars_for(cost, budget)
    } else {
        0
    };
    let over = cost > budget;
    let title = match lt.ending {
        Ending::Toppled(_) => "It toppled".to_string(),
        Ending::CameDown(_) => "It came down".to_string(),
        _ if pass && over => "Straight, but over budget".to_string(),
        _ if pass => "Straight enough!".to_string(),
        _ => "Still leaning".to_string(),
    };
    let pr = if practice {
        " This was a practice run: the conditions were easier than the level asks, so no stars."
    } else {
        ""
    };
    let tail = if pass && !practice {
        if over {
            " Over budget though, so no stars.".to_string()
        } else if stars < 3 {
            " Under budget. Trim it for more stars.".to_string()
        } else {
            " Three stars. Nicely done.".to_string()
        }
    } else if !pass && matches!(lt.ending, Ending::Stood) {
        format!(" The goal is under {}°.", fixed(GOAL_TILT, 1))
    } else {
        String::new()
    };
    let explain = format!("{} {}{tail}{pr}", lt.result_line(), lt.sentence());
    PisaResult {
        pass,
        stars,
        title,
        explain,
        tilt,
    }
}
