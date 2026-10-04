//! Camera (pan / zoom), the phone-shaped column the UI lives in, plus the
//! touch-and-mouse gesture recogniser.

use macroquad::prelude::*;

/// The widest the UI gets, as width over height: a phone held upright.
/// Wider screens (tablets, a desktop browser) get a centred column of this
/// shape, and the backdrop fills the rest.
const MAX_ASPECT: f32 = 9.0 / 16.0;

/// The width the UI is laid out in: the screen's, or the column's.
pub fn width() -> f32 {
    screen_width().min((screen_height() * MAX_ASPECT).round())
}

/// Where the column starts on the screen; 0 on a phone.
pub fn left() -> f32 {
    ((screen_width() - width()) * 0.5).round()
}

/// Draws in the column's coordinates (0 at its left edge), clipped to it.
struct Column;

impl macroquad::camera::Camera for Column {
    fn matrix(&self) -> Mat4 {
        Mat4::orthographic_rh_gl(0.0, width(), screen_height(), 0.0, -1.0, 1.0)
    }

    fn depth_enabled(&self) -> bool {
        false
    }

    fn render_pass(&self) -> Option<RenderPass> {
        None
    }

    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        // In framebuffer pixels, from the bottom.
        let dpi = screen_dpi_scale();
        Some((
            (left() * dpi).round() as i32,
            0,
            (width() * dpi).round() as i32,
            (screen_height() * dpi).round() as i32,
        ))
    }
}

/// Everything drawn from here to `end_column` lands in the column.
pub fn begin_column() {
    if left() > 0.0 {
        set_camera(&Column);
    }
}

pub fn end_column() {
    set_default_camera();
}

/// A screen position in the column's coordinates.
fn to_column(p: Vec2) -> Vec2 {
    vec2(p.x - left(), p.y)
}

/// Drag further than this and it's a pan, not a tap. Relative to the screen
/// because with `high_dpi` the framebuffer is in physical pixels.
fn tap_slop() -> f32 {
    (screen_width().min(screen_height()) * 0.025).max(10.0)
}

const MIN_ZOOM: f32 = 0.10;
const MAX_ZOOM: f32 = 2.5;

pub struct Camera {
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
        Vec2::new(width(), screen_height()) * 0.5
    }

    pub fn world_to_screen(&self, w: Vec2) -> Vec2 {
        (w - self.target) * self.zoom + Self::screen_center()
    }

    pub fn screen_to_world(&self, s: Vec2) -> Vec2 {
        (s - Self::screen_center()) / self.zoom + self.target
    }

    /// Zooms keeping `screen_anchor` still.
    pub fn zoom_at(&mut self, screen_anchor: Vec2, factor: f32) {
        let before = self.screen_to_world(screen_anchor);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_world(screen_anchor);
        self.target += before - after;
    }

    pub fn pan_screen(&mut self, delta: Vec2) {
        self.target -= delta / self.zoom;
    }

    pub fn fit(&mut self, min: Vec2, max: Vec2) {
        let size = (max - min).max(Vec2::splat(1.0));
        let pad = 1.15;
        let z = (width() / (size.x * pad)).min(screen_height() / (size.y * pad));
        self.zoom = z.clamp(MIN_ZOOM, MAX_ZOOM);
        self.target = (min + max) * 0.5;
    }

    /// Lets the camera drift a little past the tree, never off it.
    pub fn clamp_to(&mut self, min: Vec2, max: Vec2) {
        let margin = Vec2::new(width(), screen_height()) * 0.4 / self.zoom;
        let (a, b) = (min - margin, max + margin);
        self.target = self.target.clamp(a.min(b), b.max(a));
    }
}

/// Wheel travel that makes one step: X11 sends 1 per notch, a browser
/// pixels (about 100 per notch, a trackpad many small ones).
#[cfg(target_arch = "wasm32")]
const WHEEL_NOTCH: f32 = 50.0;
#[cfg(not(target_arch = "wasm32"))]
const WHEEL_NOTCH: f32 = 1.0;
/// After a step the wheel is ignored this long, so a trackpad swipe and
/// its momentum make a step or two, not a dozen.
const WHEEL_COOLDOWN: f32 = 0.25;
/// A pause this long ends a wheel gesture: what it left over is dropped.
const WHEEL_IDLE: f32 = 0.25;

/// One frame of digested input, in the column's coordinates.
#[derive(Default)]
pub struct Gesture {
    pub tap: Option<Vec2>,
    /// This frame's drag delta, once past the tap slop.
    pub drag: Vec2,
    pub pointer: Vec2,
    /// A drag is under way and the pointer is still down (moving or not).
    pub held: bool,
    /// True on the frame a drag ended.
    pub released: bool,
    /// Wheel steps, positive when scrolling up: whole notches, spaced out
    /// (see `WHEEL_COOLDOWN`).
    pub wheel: f32,
    /// Horizontal wheel steps (trackpads), positive when scrolling right.
    pub wheel_x: f32,
    /// The wheel's direction this frame, unspaced, for a smooth zoom.
    pub zoom_wheel: f32,
    /// This frame's pinch ratio; 1.0 means none.
    pub pinch: f32,
    pub pinch_anchor: Vec2,
}

pub struct Input {
    press_origin: Option<Vec2>,
    last_pos: Vec2,
    travel: f32,
    was_dragging: bool,
    prev_pinch: Option<f32>,
    /// Wheel travel not yet turned into steps, as (x, y).
    wheel_acc: Vec2,
    wheel_cooldown: f32,
    wheel_idle: f32,
}

impl Input {
    pub fn new() -> Self {
        Self {
            press_origin: None,
            last_pos: Vec2::ZERO,
            travel: 0.0,
            was_dragging: false,
            prev_pinch: None,
            wheel_acc: Vec2::ZERO,
            wheel_cooldown: 0.0,
            wheel_idle: 0.0,
        }
    }

    pub fn poll(&mut self, dt: f32) -> Gesture {
        let mut g = Gesture {
            pinch: 1.0,
            ..Default::default()
        };

        let fingers = touches();
        if fingers.len() >= 2 {
            self.press_origin = None;
            self.was_dragging = false;
            // Touches come in framebuffer pixels, the mouse in screen ones.
            let dpi = screen_dpi_scale();
            let (a, b) = (fingers[0].position / dpi, fingers[1].position / dpi);
            let dist = a.distance(b).max(1.0);
            g.pinch_anchor = to_column((a + b) * 0.5);
            if let Some(prev) = self.prev_pinch {
                g.pinch = dist / prev;
            }
            self.prev_pinch = Some(dist);
            return g;
        }
        self.prev_pinch = None;

        // macroquad simulates a single touch as the mouse.
        let pos = to_column(Vec2::from(mouse_position()));
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
            g.held = self.was_dragging;
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

        self.wheel(dt, pos, &mut g);
        g
    }

    /// Turns the wheel's travel into whole, spaced-out steps.
    fn wheel(&mut self, dt: f32, pos: Vec2, g: &mut Gesture) {
        // miniquad reports a scroll to the right as negative x.
        let (wheel_x, wheel_y) = mouse_wheel();
        let raw = vec2(-wheel_x, wheel_y);
        if raw.y != 0.0 {
            g.zoom_wheel = raw.y.signum();
            g.pinch_anchor = pos;
        }
        self.wheel_cooldown = (self.wheel_cooldown - dt).max(0.0);
        if raw == Vec2::ZERO {
            self.wheel_idle += dt;
            if self.wheel_idle >= WHEEL_IDLE {
                self.wheel_acc = Vec2::ZERO;
            }
            return;
        }
        self.wheel_idle = 0.0;
        if self.wheel_cooldown > 0.0 {
            return;
        }
        self.wheel_acc += raw;
        let notches = (self.wheel_acc / WHEEL_NOTCH).trunc();
        if notches == Vec2::ZERO {
            return;
        }
        // One step at most: the cooldown spaces out the rest.
        let one = |n: f32| if n == 0.0 { 0.0 } else { n.signum() };
        g.wheel = one(notches.y);
        g.wheel_x = one(notches.x);
        self.wheel_acc = Vec2::ZERO;
        self.wheel_cooldown = WHEEL_COOLDOWN;
    }
}
