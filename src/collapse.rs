//! The end of the Earth: Human's gamble lost. Plays instead of the genome
//! opening: the roll, a white flash that dithers away to the Earth frozen
//! under snow and ash, the spiral unwinds, then a new Earth with one more RAD.
//! It can't be skipped: a tap only closes the final black screen.
//!
//! The backdrop does the heavy lifting: `planet` hands it the frozen world.
//! Motion steps at 12 fps to stay pixel-art.

use macroquad::prelude::*;

use crate::game;
use crate::pixel::{self, bayer, hash};
use crate::planet::Planet;
use crate::render::{faded, fit_px, rgb, text_centered, TEXT, TEXT_DIM};
use crate::spiral;
use crate::ui::{self, RAD_COLOR};

/// Real seconds per second of the timeline below: above 1 plays it slower.
const PACE: f32 = 1.4;
/// Frames per real second the motion steps at.
const FPS: f32 = 12.0;

/// End of each stage, in seconds of the timeline.
const ROLL: f32 = 1.0;
/// The white flash: solid until `FLASH_HOLD` after the roll, then it
/// dithers away to the frozen Earth.
const FLASH_HOLD: f32 = 0.5;
const FLASH: f32 = 2.6;
const ASH: f32 = 4.4;
/// The old Earth is gone: `main` resets the game here.
const RESET: f32 = 5.4;
/// After a beat of black, the banner.
const FRESH: f32 = 6.0;
/// Taps close it from here.
const READY: f32 = 6.5;

const RED: Color = rgb(0xFF6A4A);
const COIL: Color = rgb(0x6FF5E1);

/// The flash's dithered white, on the backdrop's low-res grid.
struct Layer {
    img: Image,
    tex: Texture2D,
    /// The 12 fps step last painted.
    step: i32,
}

pub struct Collapse {
    t: f32,
    reset_done: bool,
    done: bool,
    layer: Option<Layer>,
}

impl Collapse {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            reset_done: false,
            done: false,
            layer: None,
        }
    }

    /// White on `cover` (0..1) of the screen, dithered like the backdrop:
    /// painted once per 12 fps step, then drawn scaled up.
    fn draw_white(&mut self, cover: f32, step: i32) {
        let u = ui::u();
        let (w, h) = (pixel::W, (screen_height() / u).ceil() as usize);
        let layer = self.layer.get_or_insert_with(|| {
            let img = Image::gen_image_color(w as u16, h as u16, Color::new(0.0, 0.0, 0.0, 0.0));
            let tex = Texture2D::from_image(&img);
            tex.set_filter(FilterMode::Nearest);
            Layer { img, tex, step: -1 }
        });
        if layer.step != step {
            layer.step = step;
            let clear = Color::new(0.0, 0.0, 0.0, 0.0);
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    let c = if bayer(x, y) < cover { WHITE } else { clear };
                    layer.img.set_pixel(x as u32, y as u32, c);
                }
            }
            layer.tex.update(&layer.img);
        }
        draw_texture_ex(
            &layer.tex,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w as f32 * u, h as f32 * u)),
                ..Default::default()
            },
        );
    }

    pub fn update(&mut self, dt: f32) {
        self.t += dt / PACE;
    }

    /// True once, when the old Earth is gone; `main` then calls
    /// `Game::end_earth` and `reset_done`.
    pub fn wants_reset(&self) -> bool {
        self.t >= RESET && !self.reset_done
    }

    pub fn reset_done(&mut self) {
        self.reset_done = true;
    }

    /// Closes it once the banner is up; it can't be skipped before.
    pub fn tap(&mut self) {
        if self.t >= READY {
            self.done = true;
        }
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    /// What the backdrop shows: the planet, then frozen under snow.
    pub fn planet(&self, p: Planet) -> Planet {
        if self.t < ROLL || self.reset_done {
            p
        } else {
            // A snowball: everything dead and frozen.
            Planet {
                temperature: 0,
                volcanism: 0,
                vegetation: 0,
                ..p
            }
        }
    }

    /// Over everything else. `rad` and `fossils` are the new Earth's.
    pub fn draw(&mut self, rad: u32, fossils: usize) {
        let (sw, sh, u) = (screen_width(), screen_height(), ui::u());
        // Stepped time: the motion moves in 12 fps frames.
        let t = (self.t * FPS * PACE).floor() / (FPS * PACE);
        let jolt = |k: i32| {
            if t < ROLL {
                vec2(
                    hash(k, (t * FPS * PACE) as i32) - 0.5,
                    hash((t * FPS * PACE) as i32, k) - 0.5,
                ) * u
                    * 4.0
            } else {
                Vec2::ZERO
            }
        };
        let mid = sh * 0.42;

        if t < ROLL {
            let flash = (0.1..0.22).contains(&t) || (0.4..0.52).contains(&t);
            if flash {
                draw_rectangle(0.0, 0.0, sw, sh, faded(rgb(0xFF3A28), 0.45));
            }
            if t >= 0.15 {
                let p = vec2(sw * 0.5, mid) + jolt(1);
                text_centered("1 IN 5", p.x, p.y, u * 22.0, RED);
            }
        } else if t < FLASH {
            // The flash: solid white, then dithering away to the frozen Earth.
            let since = t - ROLL;
            if since < FLASH_HOLD {
                draw_rectangle(0.0, 0.0, sw, sh, WHITE);
            } else {
                let cover = 1.0 - (since - FLASH_HOLD) / (FLASH - ROLL - FLASH_HOLD);
                if cover > 0.0 {
                    self.draw_white(cover, (self.t * FPS * PACE) as i32);
                }
            }
            text_centered(
                "THE EARTH IS ENDING",
                sw * 0.5,
                mid,
                fit_px("THE EARTH IS ENDING", sw * 0.9, u * 12.0),
                rgb(0x8A2A18),
            );
        } else if t < ASH {
            // Snow and ash falling on the frozen Earth, which slowly dims.
            let k = (t - FLASH) / (ASH - FLASH);
            draw_rectangle(0.0, 0.0, sw, sh, faded(rgb(0x18181C), 0.1 + 0.6 * k));
            for i in 0..70 {
                let speed = sh * (0.12 + 0.18 * hash(i, 4));
                let y = ((t - FLASH) * speed + hash(i, 5) * sh) % sh;
                let x = hash(i, 6) * sw + (t * 3.0 + i as f32).sin() * u * 2.0;
                let grey = 0.6 + 0.35 * hash(i, 7);
                draw_rectangle(
                    x.floor(),
                    y.floor(),
                    u * 1.3,
                    u * 1.3,
                    Color::new(grey, grey, grey * 1.05, 0.9),
                );
            }
            let msg = "everything found turns to stone";
            let px = fit_px(msg, sw * 0.9, u * 8.0);
            // A dark shadow keeps it readable on the snow.
            text_centered(msg, sw * 0.5 + u * 0.6, mid + u * 0.6, px, rgb(0x10141C));
            text_centered(msg, sw * 0.5, mid, px, TEXT);
        } else if t < RESET {
            let k = (t - ASH) / (RESET - ASH);
            draw_rectangle(0.0, 0.0, sw, sh, faded(BLACK, 0.8 + 0.2 * k));
            // The coil spins down to the one node it started from.
            let c = spiral::coil_center();
            let r = spiral::coil_radius() * 1.2 * (1.0 - k).max(0.05);
            let spin = -k * std::f32::consts::TAU * 1.5;
            for d in 0..16 {
                let a0 = spin + d as f32 / 16.0 * std::f32::consts::TAU;
                let a1 = a0 + std::f32::consts::TAU / 32.0;
                draw_line(
                    c.x + a0.cos() * r,
                    c.y + a0.sin() * r,
                    c.x + a1.cos() * r,
                    c.y + a1.sin() * r,
                    u * 2.0,
                    COIL,
                );
            }
        } else {
            // A black screen holds the news until the player taps on.
            draw_rectangle(0.0, 0.0, sw, sh, BLACK);
            if t >= FRESH {
                draw_banner(t - FRESH, rad, fossils, t >= READY);
            }
        }
    }
}

/// NEW EARTH, the radiation it brings, and what waits to be found again.
fn draw_banner(age: f32, rad: u32, fossils: usize, ready: bool) {
    let (sw, sh, u) = (screen_width(), screen_height(), ui::u());
    let pop = match (age * 12.0) as i32 {
        0 => 0.6,
        1 => 0.85,
        2 => 1.08,
        _ => 1.0,
    };
    let (w, h) = (sw * 0.78 * pop, u * 62.0 * pop);
    let r = Rect::new((sw - w) * 0.5, sh * 0.3, w, h);
    draw_rectangle(r.x, r.y, r.w, r.h, rgb(0x0A0E08));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, u.max(2.0), RAD_COLOR);
    if pop < 1.0 {
        return;
    }
    let cx = r.x + r.w * 0.5;
    text_centered("NEW EARTH", cx, r.y + u * 15.0, u * 13.0, TEXT);
    let label = format!("{rad} RAD");
    let px = u * 10.0;
    let tw = crate::render::text_width(&label, px);
    let icon = u * 4.5;
    let x0 = cx - (tw + icon * 2.4) * 0.5;
    ui::draw_rad_icon(vec2(x0 + icon, r.y + u * 27.0), icon, RAD_COLOR);
    crate::render::text(&label, x0 + icon * 2.4, r.y + u * 30.5, px, RAD_COLOR);
    let perks = format!(
        "+{:.0} Luck  ·  +{:.0}% morphs",
        game::RAD_LUCK * rad as f32,
        game::RAD_MORPH * rad as f32 * 100.0
    );
    text_centered(
        &perks,
        cx,
        r.y + u * 41.0,
        fit_px(&perks, r.w * 0.9, u * 6.5),
        ui::LIME,
    );
    let f = format!("{fossils} fossils to find again");
    text_centered(&f, cx, r.y + u * 50.0, u * 5.5, TEXT_DIM);
    if ready {
        text_centered(
            "tap to continue",
            sw * 0.5,
            r.y + r.h + u * 12.0,
            u * 6.5,
            faded(TEXT, 0.6 + 0.4 * (get_time() as f32 * 3.0).sin()),
        );
    }
}
