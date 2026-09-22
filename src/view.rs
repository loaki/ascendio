//! Camera (pan / zoom) plus the touch-and-mouse gesture recogniser.

use macroquad::prelude::*;

/// Drag further than this and the gesture is a pan, not a tap.
///
/// Relative to the screen, not a fixed pixel count: with `high_dpi` the
/// framebuffer is in physical pixels, so a constant 12 would be ~4 CSS pixels
/// on a 3x phone -- far below a finger's natural wobble, and taps would be
/// swallowed as drags. Android's own touch slop is about 8dp.
fn tap_slop() -> f32 {
    (screen_width().min(screen_height()) * 0.025).max(10.0)
}

const MIN_ZOOM: f32 = 0.10;
const MAX_ZOOM: f32 = 2.5;

pub struct Camera {
    /// World point the view is centred on.
    pub target: Vec2,
    pub zoom: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            target: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    fn screen_center() -> Vec2 {
        Vec2::new(screen_width(), screen_height()) * 0.5
    }

    pub fn world_to_screen(&self, w: Vec2) -> Vec2 {
        (w - self.target) * self.zoom + Self::screen_center()
    }

    pub fn screen_to_world(&self, s: Vec2) -> Vec2 {
        (s - Self::screen_center()) / self.zoom + self.target
    }

    /// Zoom about a fixed screen point, so pinching keeps that point still.
    pub fn zoom_at(&mut self, screen_anchor: Vec2, factor: f32) {
        let before = self.screen_to_world(screen_anchor);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_world(screen_anchor);
        self.target += before - after;
    }

    pub fn pan_screen(&mut self, delta: Vec2) {
        self.target -= delta / self.zoom;
    }

    /// Frames the whole discovered tree.
    pub fn fit(&mut self, min: Vec2, max: Vec2) {
        let size = (max - min).max(Vec2::splat(1.0));
        let pad = 1.15;
        let z = (screen_width() / (size.x * pad)).min(screen_height() / (size.y * pad));
        self.zoom = z.clamp(MIN_ZOOM, MAX_ZOOM);
        self.target = (min + max) * 0.5;
    }

    /// Keeps the tree reachable: the camera may drift a little past the tree's
    /// bounding box, but never far enough to lose it off-screen.
    pub fn clamp_to(&mut self, min: Vec2, max: Vec2) {
        let margin = Vec2::new(screen_width(), screen_height()) * 0.4 / self.zoom;
        let (a, b) = (min - margin, max + margin);
        self.target = self.target.clamp(a.min(b), b.max(a));
    }
}

/// One frame's worth of digested input. Both view modes read the same struct
/// and interpret it differently: the map pans and zooms, the spiral scrolls.
#[derive(Default)]
pub struct Gesture {
    /// Screen position of a completed tap, if any.
    pub tap: Option<Vec2>,
    /// Drag delta for this frame, once the gesture has passed the tap slop.
    pub drag: Vec2,
    /// Where the pointer is now. The spiral needs the absolute position, not
    /// just the delta, because it converts the drag into a rotation.
    pub pointer: Vec2,
    /// True on the frame a drag ended, which is when the spiral snaps.
    pub released: bool,
    /// Wheel notches, positive when scrolling up.
    pub wheel: f32,
    /// Two-finger pinch for this frame. 1.0 means no pinch.
    pub pinch: f32,
    pub pinch_anchor: Vec2,
}

pub struct Input {
    press_origin: Option<Vec2>,
    last_pos: Vec2,
    travel: f32,
    was_dragging: bool,
    prev_pinch: Option<f32>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            press_origin: None,
            last_pos: Vec2::ZERO,
            travel: 0.0,
            was_dragging: false,
            prev_pinch: None,
        }
    }

    pub fn poll(&mut self) -> Gesture {
        let mut g = Gesture {
            pinch: 1.0,
            ..Default::default()
        };

        let fingers = touches();
        if fingers.len() >= 2 {
            // Two-finger pinch. Cancel any single-touch gesture in progress.
            self.press_origin = None;
            self.was_dragging = false;
            let (a, b) = (fingers[0].position, fingers[1].position);
            let dist = a.distance(b).max(1.0);
            g.pinch_anchor = (a + b) * 0.5;
            if let Some(prev) = self.prev_pinch {
                g.pinch = dist / prev;
            }
            self.prev_pinch = Some(dist);
            return g;
        }
        self.prev_pinch = None;

        // A single touch arrives as mouse input; macroquad simulates it by default.
        let pos = Vec2::from(mouse_position());
        g.pointer = pos;

        if is_mouse_button_pressed(MouseButton::Left) {
            self.press_origin = Some(pos);
            self.last_pos = pos;
            self.travel = 0.0;
        }

        if is_mouse_button_down(MouseButton::Left) && self.press_origin.is_some() {
            let delta = pos - self.last_pos;
            self.travel += delta.length();
            self.last_pos = pos;
            if self.travel > tap_slop() {
                g.drag = delta;
                self.was_dragging = true;
            }
        }

        if is_mouse_button_released(MouseButton::Left) {
            if let Some(origin) = self.press_origin.take() {
                if self.travel <= tap_slop() {
                    g.tap = Some(origin);
                }
            }
            g.released = self.was_dragging;
            self.was_dragging = false;
        }

        let (_, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            g.wheel = wheel_y.signum();
            g.pinch_anchor = pos;
        }

        g
    }
}
