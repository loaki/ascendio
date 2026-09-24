//! The genome opening's backdrop: the abyss, painted per pixel on the
//! planet's 180-wide grid with the same 4x4 dither, with the genome's light
//! (a halo, then speed lines and shockwaves at the supernova) painted in.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use crate::pixel::{bayer, hash, mix, Canvas, Rgb, W};

fn noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v) = (s(fx), s(fy));
    let a = hash(xi, yi) + (hash(xi + 1, yi) - hash(xi, yi)) * u;
    let b = hash(xi, yi + 1) + (hash(xi + 1, yi + 1) - hash(xi, yi + 1)) * u;
    a + (b - a) * v
}

/// Picks one of `ramp` for `v` in 0..1, dithered between neighbours.
fn dither(ramp: &[Rgb], v: f32, x: i32, y: i32) -> Rgb {
    let f = v.clamp(0.0, 0.999) * (ramp.len() - 1) as f32;
    let i = f.floor() as usize;
    if f - i as f32 > bayer(x, y) {
        ramp[(i + 1).min(ramp.len() - 1)]
    } else {
        ramp[i]
    }
}

/// The light on the genome this frame.
pub struct Rays {
    /// Fraction of the screen.
    pub center: Vec2,
    pub color: Rgb,
    pub alt: Rgb,
    pub halo: f32,
    /// Seconds since the supernova, while it's going off.
    pub burst: Option<f32>,
}

pub struct GenomeBg {
    canvas: Canvas,
}

impl GenomeBg {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
        }
    }

    pub fn draw(&mut self, t: f32, rays: Option<&Rays>) {
        for y in 0..self.canvas.h as i32 {
            for x in 0..W as i32 {
                let mut c = self.abyss(x, y, t);
                if let Some(r) = rays {
                    c = self.light(c, x, y, r);
                }
                self.canvas.put(x, y, c);
            }
        }
        self.marine_snow(t);
        self.canvas.present();
    }

    fn abyss(&self, x: i32, y: i32, t: f32) -> Rgb {
        const RAMP: [Rgb; 5] = [
            [2, 6, 12],
            [4, 16, 28],
            [14, 36, 44],
            [22, 48, 58],
            [30, 58, 68],
        ];
        let f = y as f32 / self.canvas.h as f32;
        let mut v = (1.0 - f).powf(1.6) * 0.75;
        let ray = noise(x as f32 * 0.07 + y as f32 * 0.035 + t * 0.08, 3.0).powi(3);
        v += ray * (1.0 - f) * 0.5;
        dither(&RAMP, v, x, y)
    }

    fn marine_snow(&mut self, t: f32) {
        for i in 0..70 {
            let x = hash(i, 7) * W as f32 + (t * 0.3 + i as f32).sin() * 2.0;
            let y = (hash(i, 9) * self.canvas.h as f32 + t * (2.0 + hash(i, 3) * 3.0))
                % self.canvas.h as f32;
            let c = if hash(i, 5) > 0.7 {
                [150, 200, 205]
            } else {
                [70, 110, 120]
            };
            self.canvas.put(x as i32, y as i32, c);
        }
    }

    /// Light on top of the water: `i` counts dither levels (1 = tinted,
    /// 2 = bright, 3 = white-hot), thresholded against the Bayer grid.
    fn light(&self, c: Rgb, x: i32, y: i32, r: &Rays) -> Rgb {
        let (cx, cy) = (r.center.x * W as f32, r.center.y * self.canvas.h as f32);
        let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        let d = (dx * dx + dy * dy).sqrt();
        let mut i = r.halo * 1.6 * (1.0 - d / (W as f32 * 0.42)).max(0.0).powi(2);
        let mut col = r.color;
        if let Some(age) = r.burst {
            // Speed lines racing outward.
            let a = dy.atan2(dx);
            let sectors = 60.0;
            let sec = ((a + PI) / TAU * sectors).floor();
            let mid = sec / sectors * TAU - PI + PI / sectors;
            if ((a - mid) * d).abs() < 0.7 {
                let h = hash(sec as i32, 3);
                let head = ((age * (0.9 + h) + h) % 1.0) * W as f32 * 0.95 + 8.0;
                let len = 5.0 + h * 12.0;
                if d < head && d > head - len {
                    i += 2.2 * (1.0 - head / W as f32).max(0.2);
                    if sec as i32 % 2 != 0 {
                        col = r.alt;
                    }
                }
            }
            // Three shockwaves.
            for k in 0..3 {
                let rk = ((age - k as f32 * 0.12) / 0.9).clamp(0.0, 1.0);
                let radius = W as f32 * 1.1 * (1.0 - (1.0 - rk).powi(3));
                let width = 1.0 + (1.0 - rk) * 2.0;
                if rk > 0.0 && rk < 1.0 && (d - radius).abs() < width {
                    i += 2.2 * (1.0 - rk);
                }
            }
        }
        let b = bayer(x, y);
        if i > 2.0 + b {
            mix(col, [244, 241, 232], 0.6)
        } else if i > 1.0 + b {
            mix(c, col, 0.85)
        } else if i > b {
            mix(c, col, 0.45)
        } else {
            c
        }
    }
}
