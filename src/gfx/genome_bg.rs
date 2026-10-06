//! The genome opening's backdrop: the abyss on the planet's dithered grid, with the genome's light painted in.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use crate::gfx::pixel::{bayer, hash, mix, Canvas, Rgb, W};
use crate::gfx::render::ease_out_cubic;

fn noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v) = (s(fx), s(fy));
    let a = hash(xi, yi) + (hash(xi + 1, yi) - hash(xi, yi)) * u;
    let b = hash(xi, yi + 1) + (hash(xi + 1, yi + 1) - hash(xi, yi + 1)) * u;
    a + (b - a) * v
}

fn dither(ramp: &[Rgb], v: f32, x: i32, y: i32) -> Rgb {
    let f = v.clamp(0.0, 0.999) * (ramp.len() - 1) as f32;
    let i = f.floor() as usize;
    if f - i as f32 > bayer(x, y) {
        ramp[(i + 1).min(ramp.len() - 1)]
    } else {
        ramp[i]
    }
}

pub struct Rays {
    pub center: Vec2,
    pub color: Rgb,
    pub alt: Rgb,
    pub halo: f32,
    pub burst: Option<f32>,
}

pub struct GenomeBg {
    canvas: Canvas,
    /// Per pixel, its angle around the light's centre (in canvas pixels),
    /// for the speed lines: kept while the centre and the canvas stay put.
    angles: Vec<f32>,
    angles_for: Option<Vec2>,
}

impl GenomeBg {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::in_column(),
            angles: Vec::new(),
            angles_for: None,
        }
    }

    pub fn draw(&mut self, t: f32, rays: Option<&Rays>) {
        if !self.canvas.fits_screen() {
            *self = Self::new();
        }
        if let Some(r) = rays.filter(|r| r.burst.is_some()) {
            self.cache_angles(r);
        }
        for y in 0..self.canvas.h as i32 {
            let f = y as f32 / self.canvas.h as f32;
            let base = (1.0 - f).powf(1.6) * 0.75;
            for x in 0..W as i32 {
                let mut c = self.abyss(x, y, f, base, t);
                if let Some(r) = rays {
                    c = self.light(c, x, y, r);
                }
                self.canvas.put(x, y, c);
            }
        }
        self.marine_snow(t);
        self.canvas.present();
    }

    fn center_of(&self, r: &Rays) -> Vec2 {
        vec2(r.center.x * W as f32, r.center.y * self.canvas.h as f32)
    }

    fn cache_angles(&mut self, r: &Rays) {
        let c = self.center_of(r);
        if self.angles_for == Some(c) {
            return;
        }
        self.angles = (0..self.canvas.h as i32)
            .flat_map(|y| {
                (0..W as i32).map(move |x| (y as f32 + 0.5 - c.y).atan2(x as f32 + 0.5 - c.x))
            })
            .collect();
        self.angles_for = Some(c);
    }

    fn abyss(&self, x: i32, y: i32, f: f32, base: f32, t: f32) -> Rgb {
        const RAMP: [Rgb; 5] = [
            [2, 6, 12],
            [4, 16, 28],
            [14, 36, 44],
            [22, 48, 58],
            [30, 58, 68],
        ];
        let mut v = base;
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

    fn light(&self, c: Rgb, x: i32, y: i32, r: &Rays) -> Rgb {
        let c0 = self.center_of(r);
        let (dx, dy) = (x as f32 + 0.5 - c0.x, y as f32 + 0.5 - c0.y);
        let d = (dx * dx + dy * dy).sqrt();
        let mut i = r.halo * 1.6 * (1.0 - d / (W as f32 * 0.42)).max(0.0).powi(2);
        let mut col = r.color;
        if let Some(age) = r.burst {
            let a = self.angles[y as usize * W + x as usize];
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
            for k in 0..3 {
                let rk = ((age - k as f32 * 0.12) / 0.9).clamp(0.0, 1.0);
                let radius = W as f32 * 1.1 * ease_out_cubic(rk);
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
