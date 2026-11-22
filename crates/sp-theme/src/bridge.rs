//! Bridge (Playground) stage colours — `Playground.dc.html` inline SVG. Owned by the `bridge`
//! agent: add tokens here with their CSS source.

use crate::hex;
use egui::Color32;

/// Stage background `#0C0D0F`; the snap-grid dots `#3A3D42` (r 1.3, every 36 px = 2 m).
pub const STAGE: Color32 = hex(0x0C0D0F);
pub const GRID_DOT: Color32 = hex(0x3A3D42);
/// River `#0B1A24`, its flowing dashes `#24546E`.
pub const WATER: Color32 = hex(0x0B1A24);
pub const WATER_FLOW: Color32 = hex(0x24546E);
/// Banks: top face `#2B2721`, body and riverbed `#1A1713`.
pub const BANK_TOP: Color32 = hex(0x2B2721);
pub const BANK: Color32 = hex(0x1A1713);
/// The members' back face (the 2.5D offset copy, +18, −12 px) `#34373C`.
pub const BACK_FACE: Color32 = hex(0x34373C);
/// Supports and the load cable `#8A8F98`.
pub const SUPPORT: Color32 = hex(0x8A8F98);
/// Crates: front `#7E6442` (stroke `#3E3022`), top `#9C7E55`, side `#5A4630`.
pub const CRATE: Color32 = hex(0x7E6442);
pub const CRATE_STROKE: Color32 = hex(0x3E3022);
pub const CRATE_TOP: Color32 = hex(0x9C7E55);
pub const CRATE_SIDE: Color32 = hex(0x5A4630);
/// Joint dot: fill = [`STAGE`], ring `#D4D4D4`.
pub const JOINT_RING: Color32 = hex(0xD4D4D4);
