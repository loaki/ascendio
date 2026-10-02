//! The planet behind the spiral: a pixel-art sea-level cross-section,
//! painted per pixel into a 180-wide dithered image and scaled up with
//! nearest filtering. Levers are eased so changes are visible.

use macroquad::prelude::*;

use crate::pixel::{bayer, hash, mix, Canvas, Rgb, W};
use crate::planet::Planet;

/// Where the volcano rises, in backdrop pixels.
const VOLCANO_X: i32 = 128;
/// Waterline height as a fraction of the screen, per Land level.
const WATER: [f32; 6] = [0.14, 0.30, 0.44, 0.58, 0.72, 0.84];

fn ridge(parts: &[(f32, f32, f32)]) -> Vec<f32> {
    (0..512)
        .map(|i| {
            parts
                .iter()
                .map(|&(f, amp, ph)| {
                    (i as f32 / 512.0 * std::f32::consts::TAU * f + ph).sin() * amp
                })
                .sum()
        })
        .collect()
}

/// The levers as floats, eased toward the real planet.
#[derive(Clone, Copy)]
struct Shown {
    land: f32,
    veg: f32,
    oxygen: f32,
    temp: f32,
    volc: f32,
}

impl Shown {
    fn of(p: &Planet) -> Self {
        Self {
            land: p.land as f32,
            veg: p.vegetation as f32,
            oxygen: p.oxygen as f32,
            temp: p.temperature as f32,
            volc: p.volcanism as f32,
        }
    }
}

pub struct Backdrop {
    canvas: Canvas,
    ridges: [Vec<f32>; 3],
    shown: Option<Shown>,
}

/// Linear interpolation through `t` at fractional index `v`.
fn table(v: f32, t: &[f32]) -> f32 {
    let v = v.clamp(0.0, (t.len() - 1) as f32);
    let i = v.floor() as usize;
    let j = (i + 1).min(t.len() - 1);
    t[i] + (t[j] - t[i]) * (v - i as f32)
}

/// What the passes of one `Backdrop::paint` share.
struct Scene {
    /// Canvas height, and the scale of the 320-high design.
    h: i32,
    sc: f32,
    /// The waterline's row.
    wy: i32,
    /// Per column, the top row of each ridge, far to near.
    tops: Vec<[i32; 3]>,
    veg_level: usize,
    cold: bool,
    /// Volcanism, and the half-width of the cone at `VOLCANO_X`.
    vol: f32,
    vw: i32,
}

impl Backdrop {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
            ridges: [
                ridge(&[
                    (2.0, 14.0, 0.0),
                    (5.0, 8.0, 1.0),
                    (11.0, 4.0, 2.0),
                    (29.0, 1.5, 3.0),
                ]),
                ridge(&[
                    (3.0, 10.0, 2.0),
                    (7.0, 6.0, 0.5),
                    (17.0, 3.0, 1.0),
                    (41.0, 1.2, 4.0),
                ]),
                ridge(&[(2.0, 8.0, 4.0), (6.0, 5.0, 1.0), (19.0, 2.5, 3.0)]),
            ],
            shown: None,
        }
    }

    /// `t` drives ambient motion (faster while time runs); `calm` is real
    /// time, so the volcano never looks frantic.
    pub fn draw(&mut self, planet: &Planet, t: f32, calm: f32, dt: f32) {
        if !self.canvas.fits_screen() {
            *self = Self::new();
        }
        let target = Shown::of(planet);
        let s = match self.shown {
            None => target,
            Some(cur) => {
                let k = (dt * 3.0).min(1.0);
                let e = |a: f32, b: f32| a + (b - a) * k;
                Shown {
                    land: e(cur.land, target.land),
                    veg: e(cur.veg, target.veg),
                    oxygen: e(cur.oxygen, target.oxygen),
                    temp: e(cur.temp, target.temp),
                    volc: e(cur.volc, target.volc),
                }
            }
        };
        self.shown = Some(s);
        self.paint(&s, t, calm);
        self.canvas.present();
    }

    /// A dithered additive glow.
    fn glow(&mut self, cx: f32, cy: f32, r: f32, c: Rgb, strength: f32) {
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx * dx + dy * dy) as f32).sqrt() / r;
                if d > 1.0 {
                    continue;
                }
                let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                let a = (1.0 - d).powf(1.7) * strength;
                if a > bayer(x, y) * 0.9 {
                    let o = self.canvas.get(x, y);
                    let add = |o: u8, c: u8| (o as f32 + c as f32 * a * 0.6).min(255.0) as u8;
                    self.canvas
                        .put(x, y, [add(o[0], c[0]), add(o[1], c[1]), add(o[2], c[2])]);
                }
            }
        }
    }

    /// A dithered smoke disc lit from the top-left; `ember` tints its underside.
    #[allow(clippy::too_many_arguments)]
    fn puff(
        &mut self,
        cx: f32,
        cy: f32,
        r: f32,
        density: f32,
        light: Rgb,
        shade: Rgb,
        ember: bool,
    ) {
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx * dx + dy * dy) as f32).sqrt() / r;
                if d > 1.0 {
                    continue;
                }
                let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                if bayer(x, y) > density * (1.0 - d * 0.6) {
                    continue;
                }
                let mut c = if (dx + dy) < 0 { light } else { shade };
                if ember && dy > 0 {
                    c = mix(c, [200, 80, 40], 0.5);
                }
                self.canvas.put(x, y, c);
            }
        }
    }

    fn paint(&mut self, s: &Shown, t: f32, calm: f32) {
        let scene = self.scene(s);
        self.paint_ground(s, &scene, t);
        self.paint_surface(s, &scene, t);
        self.paint_trees(&scene);
        self.paint_volcano(&scene, calm);
        self.paint_sky(s, &scene, t);
        self.paint_kelp(s, &scene, t);
        self.paint_marine_snow(s, &scene, t);
        self.paint_vignette(&scene);
    }

    /// The layout every pass paints against: the waterline and the ridge
    /// tops, the volcano carved into the far one.
    fn scene(&self, s: &Shown) -> Scene {
        let h = self.canvas.h as i32;
        let sc = h as f32 / 320.0;
        let wy = (table(s.land, &WATER) * h as f32).round() as i32;
        let rise = table(s.land, &[-70.0, 5.0, 30.0, 55.0, 90.0, 120.0]) * sc;
        let bases = [26.0, 48.0, 70.0].map(|b| wy as f32 + b * sc - rise);
        let offs = [0usize, 90, 200];

        let veg_level = s.veg.round().clamp(0.0, 5.0) as usize;
        let cold = s.temp < 1.5;
        let mut tops: Vec<[i32; 3]> = (0..W)
            .map(|x| {
                [0, 1, 2]
                    .map(|k| (bases[k] - self.ridges[k][(x + offs[k]) & 511] * sc).round() as i32)
            })
            .collect();

        // The volcano is a cone in the far ridge, so it is shaded, snowed on
        // and submerged like the ground.
        let vol = s.volc.clamp(0.0, 2.0);
        let (vx, vw) = (VOLCANO_X, (30.0 * sc) as i32);
        if vol > 0.05 {
            let vh = (20.0 + s.land * 2.5) * sc * vol.min(1.0);
            let crater = 4.0 * sc;
            for dx in -vw..=vw {
                let x = vx + dx;
                if !(0..W as i32).contains(&x) {
                    continue;
                }
                let p = 1.0 - dx.abs() as f32 / vw as f32;
                let mut bump = vh * p.powf(1.4);
                if (dx.abs() as f32) < crater {
                    bump -= (crater - dx.abs() as f32) * 0.7 * vol.min(1.0);
                }
                let base = tops[x as usize][0];
                tops[x as usize][0] = base.min((base as f32 - bump).round() as i32);
            }
        }
        Scene {
            h,
            sc,
            wy,
            tops,
            veg_level,
            cold,
            vol,
            vw,
        }
    }

    /// Rock, ridges and vegetation above the water, the sea and its light below.
    fn paint_ground(&mut self, s: &Shown, scene: &Scene, t: f32) {
        let Scene {
            h,
            sc,
            wy,
            veg_level,
            cold,
            vol,
            vw,
            ..
        } = *scene;
        let tops = &scene.tops;
        let vx = VOLCANO_X;
        let sky = sky_palette(s);
        let wat = water_palette(s);
        let on_volcano = |x: i32| vol > 0.3 && (x - vx).abs() < vw;
        for y in 0..h {
            for x in 0..W as i32 {
                let b = bayer(x, y);
                let col = &tops[x as usize];
                let mut li: i32 = -1;
                let mut top = 0;
                for (k, &tp) in col.iter().enumerate() {
                    if y >= tp {
                        li = k as i32;
                        top = tp;
                    }
                }
                let c = if li >= 0 {
                    let depth_in = y - top;
                    let li = li as usize;
                    if y < wy {
                        let rock = [[90, 82, 96], [62, 56, 72], [42, 38, 50]][li];
                        let rock_d = [[74, 66, 80], [46, 42, 54], [28, 26, 34]][li];
                        let rim_h = 1 + veg_level as i32;
                        let mut c = if hash(x >> 1, y >> 1) * 0.5 + 0.25 > b {
                            rock
                        } else {
                            rock_d
                        };
                        // Basalt on the cone, bare near the top.
                        let ashen = li == 0 && on_volcano(x);
                        if ashen {
                            c = if hash(x >> 1, y >> 1) * 0.5 + 0.2 > b {
                                [70, 58, 60]
                            } else {
                                [48, 40, 44]
                            };
                        }
                        if depth_in < rim_h
                            && (veg_level > 0 || cold)
                            && !(ashen && (x - vx).abs() < vw / 2)
                        {
                            c = veg_color(veg_level, cold, depth_in < 1 || b < 0.4);
                        }
                        if s.temp < 2.5 && top < wy - (40.0 * sc) as i32 && depth_in < 3 {
                            c = [230, 240, 246];
                        }
                        c
                    } else {
                        let d = (y - wy) as f32 / (h - wy + 1) as f32;
                        let base = [[30, 58, 68], [22, 48, 58], [14, 36, 44]][li];
                        let mut c = mix(base, wat[4], (d * 0.9 + 0.1).min(0.9));
                        if depth_in < 1 {
                            c = mix(c, [79, 138, 128], 0.3);
                        }
                        c
                    }
                } else if y < wy {
                    let f = y as f32 / wy.max(1) as f32;
                    sky[((f * 3.2 + b * 0.8) as usize).min(3)]
                } else {
                    let f = (y - wy) as f32 / (h - wy).max(1) as f32;
                    let mut c = wat[((f * 4.6 + b * 0.8) as usize).min(4)];
                    let u = x as f32 + (y - wy) as f32 * 0.5;
                    let ray = (0.5 + 0.5 * (u * 0.08 + t * 0.4).sin()).powi(8)
                        * (1.0 - f)
                        * (0.3 + s.oxygen * 0.15);
                    if ray > b {
                        c = mix(c, [159, 224, 224], 0.25);
                    }
                    c
                };
                self.canvas.put(x, y, c);
            }
        }
    }

    /// The water surface, where it isn't land; ice when frozen.
    fn paint_surface(&mut self, s: &Shown, scene: &Scene, t: f32) {
        let Scene { h, wy, .. } = *scene;
        let tops = &scene.tops;
        if wy > 0 && wy < h {
            for x in 0..W as i32 {
                if tops[x as usize].iter().any(|&tp| tp <= wy) {
                    continue;
                }
                let w = ((x as f32 * 0.25 + t * 2.0).sin() * 0.8).round() as i32;
                self.canvas.put(
                    x,
                    wy + w,
                    if s.temp < 0.5 {
                        [230, 240, 246]
                    } else {
                        [191, 232, 240]
                    },
                );
                if hash(x, (t * 3.0) as i32) > 0.93 {
                    self.canvas.put(x, wy + w - 1, [255, 255, 255]);
                    self.canvas.put(x + 1, wy + w - 1, [255, 255, 255]);
                }
            }
            if s.temp < 0.5 {
                for x in 0..W as i32 {
                    if hash(x >> 3, 5) > 0.25 {
                        for dy in -2..2 {
                            self.canvas.put(x, wy + dy, [223, 238, 246]);
                        }
                    }
                }
            }
        }
    }

    /// Trees along the near ridges, once there is forest.
    fn paint_trees(&mut self, scene: &Scene) {
        let Scene {
            wy,
            veg_level,
            cold,
            ..
        } = *scene;
        let tops = &scene.tops;
        if veg_level >= 3 && !cold {
            let step = match veg_level {
                5 => 3,
                4 => 4,
                _ => 6,
            };
            for li in 1..3usize {
                let mut x = (li * 3) % 5;
                while x < W {
                    let tp = tops[x][li];
                    let hidden = (li + 1..3).any(|k| tops[x][k] <= tp);
                    if tp < wy - 1 && !hidden {
                        let th = 2 + (hash(x as i32, li as i32) * veg_level as f32) as i32;
                        for k in 0..th {
                            self.canvas.put(x as i32, tp - 1 - k, [42, 30, 20]);
                        }
                        let leaf = if li == 2 { [31, 90, 28] } else { [47, 122, 42] };
                        for dy in 0..3 {
                            for dx in -1..=1 {
                                self.canvas.put(x as i32 + dx, tp - th - 1 - dy, leaf);
                            }
                        }
                    }
                    x += step;
                }
            }
        }
    }

    /// Crater glow and smoke; lava and bombs when violent; bubbles if submerged.
    fn paint_volcano(&mut self, scene: &Scene, calm: f32) {
        let Scene { sc, wy, vol, .. } = *scene;
        let tops = &scene.tops;
        let vx = VOLCANO_X;
        if vol > 0.3 {
            let cy = tops[vx as usize][0];
            if cy < wy {
                let heat = (vol - 0.3).min(1.7);
                self.glow(
                    vx as f32,
                    cy as f32,
                    10.0 + heat * 6.0,
                    [255, 106, 40],
                    0.3 + heat * 0.25 + 0.08 * (calm * 0.9).sin(),
                );
                for dx in -3..=3 {
                    let f = 0.5 + 0.5 * (dx as f32 * 1.3 + calm * 0.8).sin();
                    self.canvas
                        .put(vx + dx, cy + 1, mix([255, 110, 40], [255, 176, 72], f));
                }
                if vol > 1.1 {
                    let len = ((vol - 1.0) * 34.0 * sc) as i32;
                    for side in [-1i32, 1] {
                        for step in 0..len {
                            let x = vx
                                + side * (3 + step * 4 / 5)
                                + ((step as f32 * 0.5 + side as f32).sin() * 1.2) as i32;
                            if !(0..W as i32).contains(&x) {
                                break;
                            }
                            let y = tops[x as usize][0] + 1;
                            if y >= wy {
                                break;
                            }
                            let f =
                                0.5 + 0.5 * (step as f32 * 0.45 - calm * 1.1 + side as f32).sin();
                            self.canvas
                                .put(x, y, mix([200, 62, 28], [255, 156, 64], f * f));
                            self.canvas.put(x, y + 1, [150, 40, 24]);
                        }
                    }
                    for i in 0..4 {
                        let age = (calm * (0.22 + hash(i, 5) * 0.08) + hash(i, 9)) % 1.0;
                        let x = vx as f32 + (hash(i, 3) - 0.5) * 50.0 * sc * age;
                        let y = cy as f32 - (70.0 * age - 80.0 * age * age) * sc;
                        if (y as i32) < wy {
                            self.canvas.put(x as i32, y as i32, [255, 190, 90]);
                            self.canvas.put(x as i32, y as i32 + 1, [220, 80, 30]);
                        }
                    }
                }
                let puffs = 18 + (heat * 10.0) as i32;
                let dark = (vol - 1.0).clamp(0.0, 1.0);
                for i in 0..puffs {
                    let age = (calm * 0.035 * (1.0 + vol * 0.3) + i as f32 / puffs as f32) % 1.0;
                    let px = vx as f32 - age.powf(1.5) * 58.0 * sc
                        + (i as f32 * 1.7 + calm * 0.25).sin() * 5.0 * age;
                    let py = cy as f32 - 3.0 - age * 105.0 * sc;
                    let r = (3.0 + age * 17.0) * sc * (0.7 + 0.3 * vol);
                    let density = (1.0 - age).powf(0.7) * (0.6 + 0.3 * vol.min(1.0));
                    let light = mix([168, 162, 164], [84, 74, 76], dark);
                    let shade = mix([116, 110, 114], [52, 44, 46], dark);
                    self.puff(px, py, r, density, light, shade, age < 0.18 && vol > 1.0);
                }
            } else {
                for i in 0..8 {
                    let age = (calm * 0.2 + i as f32 / 8.0) % 1.0;
                    let x = vx as f32 + (i as f32 * 2.1 + calm * 0.8).sin() * 3.0;
                    let y = cy as f32 - age * (cy - wy) as f32;
                    self.canvas.put(x as i32, y as i32, [191, 232, 240]);
                }
                self.glow(vx as f32, cy as f32, 8.0, [255, 106, 40], 0.3);
            }
        }
    }

    /// The sun and the clouds, while the sky shows.
    fn paint_sky(&mut self, s: &Shown, scene: &Scene, t: f32) {
        let Scene { sc, wy, cold, .. } = *scene;
        if wy > (30.0 * sc) as i32 {
            let sy = ((wy as f32 - 30.0 * sc).min(40.0 * sc)) as i32;
            let hot = s.volc >= 1.5;
            self.glow(
                142.0,
                sy as f32,
                16.0,
                if hot { [255, 138, 90] } else { [255, 240, 192] },
                0.5,
            );
            let sun = if hot {
                [255, 176, 128]
            } else {
                [255, 246, 216]
            };
            for dy in -4..=4 {
                for dx in -4..=4 {
                    if dx * dx + dy * dy <= 18 {
                        self.canvas.put(142 + dx, sy + dy, sun);
                    }
                }
            }
            for k in 0..3 {
                let cx = ((k as f32 * 70.0 + t * 4.0) % 220.0) as i32 - 20;
                let cy = ((14 + k * 11) as f32 * sc) as i32;
                if cy > wy - 10 {
                    continue;
                }
                let cloud = if cold {
                    [232, 238, 242]
                } else {
                    [226, 236, 246]
                };
                for dx in 0..20 {
                    for dy in 0..3 {
                        self.canvas.put(cx + dx, cy + dy, cloud);
                    }
                }
                for dx in 4..14 {
                    for dy in 1..3 {
                        self.canvas.put(cx + dx, cy - dy, cloud);
                    }
                }
            }
        }
    }

    /// Kelp swaying on the deep ridge.
    fn paint_kelp(&mut self, s: &Shown, scene: &Scene, t: f32) {
        let Scene {
            sc, wy, veg_level, ..
        } = *scene;
        let tops = &scene.tops;
        if s.temp >= 0.5 {
            for k in 0..(10 + veg_level * 2) {
                let x0 = hash(k as i32, 51) * W as f32;
                let sb = tops[(x0 as usize).min(W - 1)][2];
                if sb <= wy + 20 {
                    continue;
                }
                let hgt = (sb - wy - 10).min((40.0 * sc) as i32);
                let mut sy = 0;
                while sy < hgt {
                    let sway =
                        (t * 1.2 + k as f32 + sy as f32 * 0.05).sin() * 3.0 * sy as f32 / 40.0;
                    self.canvas.put((x0 + sway) as i32, sb - sy, [10, 48, 40]);
                    self.canvas
                        .put((x0 + sway) as i32 + 1, sb - sy, [10, 48, 40]);
                    sy += 2;
                }
            }
        }
    }

    /// Marine snow, brighter with more oxygen.
    fn paint_marine_snow(&mut self, s: &Shown, scene: &Scene, t: f32) {
        let Scene { h, wy, .. } = *scene;
        for k in 0..60 {
            let x = hash(k, 7) * W as f32;
            let speed = 3.0 + hash(k, 9) * 6.0;
            let y = ((hash(k, 8) * h as f32 + t * speed) % h as f32) as i32;
            if y > wy {
                let c = if s.oxygen >= 3.5 && k % 3 == 0 {
                    [197, 247, 106]
                } else {
                    [58, 110, 110]
                };
                self.canvas.put(x as i32, y, c);
            }
        }
    }

    /// Vignette, so the UI reads on top.
    fn paint_vignette(&mut self, scene: &Scene) {
        let Scene { h, .. } = *scene;
        for y in 0..h {
            for x in 0..W as i32 {
                let rx = (x as f32 - W as f32 * 0.5) / (W as f32 * 0.62);
                let ry = (y as f32 - h as f32 * 0.5) / (h as f32 * 0.6);
                let r = (rx * rx + ry * ry).sqrt();
                let v = ((r - 0.6) / 0.5).clamp(0.0, 1.0).powf(1.3) * 0.85;
                if bayer(x, y) < v {
                    self.canvas.put(x, y, [3, 5, 10]);
                }
            }
        }
    }
}

fn sky_palette(s: &Shown) -> [Rgb; 4] {
    let (mut top, mut hor): (Rgb, Rgb) = ([10, 26, 58], [126, 176, 214]);
    let cold = (1.8 - s.temp).clamp(0.0, 1.0);
    top = mix(top, [26, 38, 56], cold);
    hor = mix(hor, [200, 214, 224], cold);
    let hot = (s.temp - 3.2).clamp(0.0, 1.0);
    top = mix(top, [42, 42, 64], hot);
    hor = mix(hor, [224, 168, 120], hot);
    let thin = (1.5 - s.oxygen).clamp(0.0, 1.0);
    hor = mix(hor, [200, 144, 112], thin * 0.5);
    top = mix(top, [58, 32, 32], thin * 0.4);
    let ash = (s.volc - 1.0).clamp(0.0, 1.0);
    hor = mix(hor, [192, 80, 58], ash * 0.45);
    top = mix(top, [42, 16, 16], ash * 0.5);
    [top, mix(top, hor, 0.35), mix(top, hor, 0.65), hor]
}

fn water_palette(s: &Shown) -> [Rgb; 5] {
    let mut surf: Rgb = [47, 127, 146];
    surf = mix(surf, [90, 138, 160], (1.8 - s.temp).clamp(0.0, 1.0));
    surf = mix(surf, [42, 138, 120], (s.temp - 3.2).clamp(0.0, 1.0));
    surf = mix(surf, [58, 90, 80], (1.5 - s.oxygen).clamp(0.0, 1.0) * 0.5);
    surf = mix(surf, [106, 64, 48], (s.volc - 1.0).clamp(0.0, 1.0) * 0.35);
    let deep: Rgb = [4, 16, 28];
    [
        surf,
        mix(surf, deep, 0.3),
        mix(surf, deep, 0.55),
        mix(surf, deep, 0.78),
        deep,
    ]
}

fn veg_color(level: usize, cold: bool, light: bool) -> Rgb {
    if cold {
        return if light {
            [223, 238, 246]
        } else {
            [184, 202, 214]
        };
    }
    const V: [(Rgb, Rgb); 6] = [
        ([106, 96, 112], [85, 76, 92]),
        ([106, 122, 74], [79, 95, 56]),
        ([79, 138, 58], [58, 106, 44]),
        ([58, 122, 48], [42, 90, 36]),
        ([47, 122, 42], [32, 90, 30]),
        ([42, 122, 38], [26, 85, 24]),
    ];
    let (a, b) = V[level.min(5)];
    if light {
        a
    } else {
        b
    }
}
