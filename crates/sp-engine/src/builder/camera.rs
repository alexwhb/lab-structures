//! The camera (`cam()`, `defaultView`, `toWorld`, `snap`, `zoomBy`, `fit`): world metres ↔
//! stage pixels on the 1120 × 600 stage, the snap grid and the 2.5D offset.

use super::data::Scene;
use crate::jsnum::round;

/// Stage width, px (the SVG `viewBox`).
pub const STAGE_W: f64 = 1120.0;
/// Stage height, px.
pub const STAGE_H: f64 = 600.0;
/// Zoom limits (35 %…500 %).
pub const ZOOM_MIN: f64 = 0.35;
/// The largest zoom.
pub const ZOOM_MAX: f64 = 5.0;
/// Zoom buttons' factor.
pub const ZOOM_STEP: f64 = 1.3;
/// The mouse wheel's factor per notch.
pub const WHEEL_STEP: f64 = 1.15;

/// What the camera looks at (`state.view`): the stage centre in world metres and the zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewState {
    /// World x at the stage centre, m.
    pub cx: f64,
    /// World y at the stage centre, m.
    pub cy: f64,
    /// Zoom (1 = 100 %).
    pub z: f64,
}

impl ViewState {
    /// `defaultView(scene)`: river (28, 4.4) at 100 %, open ground (100, 40) at 100 %.
    pub fn default_for(scene: Scene) -> ViewState {
        match scene {
            Scene::Flat => ViewState {
                cx: 100.0,
                cy: 40.0,
                z: 1.0,
            },
            Scene::River => ViewState {
                cx: 28.0,
                cy: 4.4,
                z: 1.0,
            },
        }
    }
}

/// The camera for a scene and view (`cam()`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The scene.
    pub scene: Scene,
    /// The view.
    pub view: ViewState,
    /// Pixels per metre (`SC` = 18 or 5 × zoom).
    pub sc: f64,
    /// Snap grid, m: 2 (river) / 4 (open ground), halved at ≥ 250 %, doubled below 60 %.
    pub grid: f64,
    /// Snapping clamps x to `xmin…xmax` and y to `ymin…ymax`, m.
    pub xmin: f64,
    /// See `xmin`.
    pub xmax: f64,
    /// See `xmin`.
    pub ymin: f64,
    /// See `xmin`.
    pub ymax: f64,
    /// The 2.5D back offset, px (`off`: right and up).
    pub off: [f64; 2],
}

impl Camera {
    /// `cam(st)`.
    pub fn new(scene: Scene, view: ViewState) -> Camera {
        let flat = scene == Scene::Flat;
        let base = if flat { 5.0 } else { 18.0 };
        let sc = base * view.z;
        let g0 = if flat { 4.0 } else { 2.0 };
        let grid = if view.z >= 2.5 {
            g0 / 2.0
        } else if view.z < 0.6 {
            g0 * 2.0
        } else {
            g0
        };
        let k = (sc / 18.0).min(1.4);
        Camera {
            scene,
            view,
            sc,
            grid,
            xmin: if flat { -40.0 } else { -20.0 },
            xmax: if flat { 240.0 } else { 76.0 },
            ymin: if flat { 0.0 } else { -12.0 },
            ymax: if flat { 200.0 } else { 40.0 },
            off: [(14.0 * k).max(4.0), -(9.0 * k).max(2.5)],
        }
    }
    /// Pixels per metre at 100 % (18 river, 5 open ground).
    pub fn base(&self) -> f64 {
        if self.scene == Scene::Flat { 5.0 } else { 18.0 }
    }
    /// World x → stage px (`X`).
    pub fn x(&self, x: f64) -> f64 {
        560.0 + (x - self.view.cx) * self.sc
    }
    /// World y → stage px (`Y`).
    pub fn y(&self, y: f64) -> f64 {
        300.0 - (y - self.view.cy) * self.sc
    }
    /// A world point → stage px.
    pub fn px(&self, p: [f64; 2]) -> [f64; 2] {
        [self.x(p[0]), self.y(p[1])]
    }
    /// Stage px → world m (`toWorld`).
    pub fn to_world(&self, px: f64, py: f64) -> [f64; 2] {
        [
            self.view.cx + (px - 560.0) / self.sc,
            self.view.cy - (py - 300.0) / self.sc,
        ]
    }
    /// Stage px → the nearest grid point, clamped to the scene (`snap`).
    pub fn snap(&self, px: f64, py: f64) -> [f64; 2] {
        let w = self.to_world(px, py);
        let x = round(w[0] / self.grid) * self.grid;
        let y = round(w[1] / self.grid) * self.grid;
        [
            self.xmin.max(self.xmax.min(x)),
            self.ymin.max(self.ymax.min(y)),
        ]
    }
    /// Whether a world point is on the stage (with a 20 px margin; handles and joints off it
    /// are not drawn).
    pub fn on_screen(&self, p: [f64; 2]) -> bool {
        let (x, y) = (self.x(p[0]), self.y(p[1]));
        x > -20.0 && x < 1140.0 && y > -20.0 && y < 620.0
    }
    /// The view after zooming by `f` about stage px (`zoomBy`; the stage centre by default).
    pub fn zoomed(&self, f: f64, px: Option<f64>, py: Option<f64>) -> ViewState {
        let v = self.view;
        let z = ZOOM_MIN.max(ZOOM_MAX.min(v.z * f));
        let px = px.unwrap_or(560.0);
        let py = py.unwrap_or(300.0);
        let wx = v.cx + (px - 560.0) / self.sc;
        let wy = v.cy - (py - 300.0) / self.sc;
        let sc2 = self.sc / v.z * z;
        ViewState {
            z,
            cx: wx - (px - 560.0) / sc2,
            cy: wy + (py - 300.0) / sc2,
        }
    }
}

/// `fit()`: frame every joint plus the ground (−12 m river / 0) and the target, with margins;
/// the scene's default view with no joints.
pub fn fit(scene: Scene, joints: &[[f64; 2]], target: Option<f64>) -> ViewState {
    if joints.is_empty() {
        return ViewState::default_for(scene);
    }
    let mut ys: Vec<f64> = joints.iter().map(|p| p[1]).collect();
    ys.push(if scene == Scene::River { -12.0 } else { 0.0 });
    ys.push(target.unwrap_or(0.0));
    let min = |v: &mut dyn Iterator<Item = f64>| v.fold(f64::INFINITY, f64::min);
    let max = |v: &mut dyn Iterator<Item = f64>| v.fold(f64::NEG_INFINITY, f64::max);
    let x0 = min(&mut joints.iter().map(|p| p[0])) - 6.0;
    let x1 = max(&mut joints.iter().map(|p| p[0])) + 6.0;
    let y0 = min(&mut ys.iter().copied()) - 4.0;
    let y1 = max(&mut ys.iter().copied()) + 6.0;
    let base = if scene == Scene::Flat { 5.0 } else { 18.0 };
    let z =
        ZOOM_MIN.max(ZOOM_MAX.min((1000.0 / ((x1 - x0) * base)).min(520.0 / ((y1 - y0) * base))));
    ViewState {
        cx: (x0 + x1) / 2.0,
        cy: (y0 + y1) / 2.0,
        z,
    }
}
