//! The wait dial: a dithered halo round the spiral for choosing how long to wait.

use std::f32::consts::{FRAC_PI_2, TAU};

use macroquad::prelude::*;

use crate::game::wait;
use crate::gfx::render::{
    self, faded, rgb, text, text_width, CYAN, GOLD, LIME, LOCKED_BORDER, TRACK,
};
use crate::gfx::spiral;
use crate::gfx::sprites::{self, Sprites};
use crate::ui::Assets;

const STEP: f32 = 40.0 * TAU / 360.0;
const TOP: f32 = -FRAC_PI_2;
const STEPS: u32 = ((wait::MAX_HOURS - wait::MIN_HOURS) / wait::STEP_HOURS) as u32;
const CHARM_ICON: usize = 3;
const CATALYST_ICON: usize = 2;

pub fn center() -> Vec2 {
    spiral::coil_center()
}

pub fn radius() -> f32 {
    spiral::coil_radius() * 1.53
}

/// Half the halo's thickness at its widest, so the panel can clear it.
pub fn max_half_width() -> f32 {
    half_width(wait::MAX_HOURS)
}

fn half_width(hours: f32) -> f32 {
    render::u() * (1.0 + (hours - wait::MIN_HOURS) * 0.9)
}

pub fn color(hours: f32) -> Color {
    if hours >= wait::SURE_RARE_HOURS {
        GOLD
    } else if hours >= wait::MORPH_HOURS {
        LIME
    } else {
        CYAN
    }
}

fn angle_of(hours: f32) -> f32 {
    TOP + (hours - wait::MIN_HOURS) / wait::STEP_HOURS * STEP
}

fn on_ring(angle: f32, r: f32) -> Vec2 {
    center() + Vec2::new(angle.cos(), angle.sin()) * r
}

pub fn step_px() -> f32 {
    render::u() * 10.0
}

pub fn draw(hours: f32, sprites: &Sprites, assets: &Assets) {
    let (c, r, u) = (center(), radius(), render::u());
    let col = color(hours);
    let at = angle_of(hours);

    draw_circle_lines(c.x, c.y, r, u * 0.8, TRACK);
    draw_halo(c, r, half_width(hours), u.round().max(1.0), faded(col, 0.7));

    let segs = ((at - TOP) / 0.04).ceil() as usize;
    for k in 0..segs {
        let (a, b) = (TOP + k as f32 * 0.04, (TOP + (k + 1) as f32 * 0.04).min(at));
        let (p, q) = (on_ring(a, r), on_ring(b, r));
        draw_line(p.x, p.y, q.x, q.y, u * 0.9, col);
    }
    for k in 0..=STEPS {
        let h = wait::MIN_HOURS + k as f32 * wait::STEP_HOURS;
        let p = on_ring(angle_of(h), r);
        let (s, pc) = if h <= hours {
            (u * 2.0, col)
        } else {
            (u * 1.4, LOCKED_BORDER)
        };
        draw_rectangle(p.x - s * 0.5, p.y - s * 0.5, s, s, pc);
    }

    milestone(
        wait::MORPH_HOURS,
        "MORPHS x1.25",
        LIME,
        CHARM_ICON,
        sprites,
        hours,
    );
    milestone(
        wait::SURE_RARE_HOURS,
        "RARE+ SURE",
        GOLD,
        CATALYST_ICON,
        sprites,
        hours,
    );

    for k in 1..=7 {
        let p = on_ring(at - k as f32 * 0.07, r);
        let s = u * (4.0 - k as f32 * 0.45).max(0.8);
        draw_rectangle(
            p.x - s * 0.5,
            p.y - s * 0.5,
            s,
            s,
            faded(col, 1.0 - k as f32 * 0.1),
        );
    }
    let sun = on_ring(at, r);
    assets.glow(sun, u * 15.0, col, 0.55);
    let s = u * 4.6;
    let white = rgb(0xFFF4D8);
    draw_rectangle(sun.x - s * 0.5, sun.y - s * 0.5, s, s, white);
    for d in [
        vec2(0.0, -1.0),
        vec2(1.0, 0.0),
        vec2(0.0, 1.0),
        vec2(-1.0, 0.0),
    ] {
        let p = sun + d * u * 4.2;
        let rs = u * 1.4;
        draw_rectangle(p.x - rs * 0.5, p.y - rs * 0.5, rs, rs, white);
    }
}

fn draw_halo(c: Vec2, r: f32, half: f32, cell: f32, col: Color) {
    let (outer, inner) = (r + half, (r - half).max(0.0));
    let y0 = ((c.y - outer) / cell).floor() as i32;
    let y1 = ((c.y + outer) / cell).ceil() as i32;
    for iy in y0..=y1 {
        let dy = (iy as f32 + 0.5) * cell - c.y;
        if dy.abs() > outer {
            continue;
        }
        let xo = (outer * outer - dy * dy).sqrt();
        let xi = if dy.abs() < inner {
            (inner * inner - dy * dy).sqrt()
        } else {
            0.0
        };
        for (from, to) in [(c.x - xo, c.x - xi), (c.x + xi, c.x + xo)] {
            let (x0, x1) = ((from / cell).floor() as i32, (to / cell).ceil() as i32);
            for ix in x0..x1 {
                if (ix + iy) % 2 == 0 {
                    draw_rectangle(ix as f32 * cell, iy as f32 * cell, cell, cell, col);
                }
            }
        }
    }
}

fn milestone(at_hours: f32, label: &str, col: Color, icon: usize, sprites: &Sprites, hours: f32) {
    let u = render::u();
    let p = on_ring(angle_of(at_hours), radius());
    let reached = hours >= at_hours;
    let ring = if reached { col } else { LOCKED_BORDER };
    draw_circle(p.x, p.y, u * 6.5, rgb(0x05090F));
    draw_circle_lines(p.x, p.y, u * 6.5, u * 0.7, ring);
    let tint = if reached { WHITE } else { faded(WHITE, 0.45) };
    sprites::draw_sprite(
        sprites.boon(icon),
        p,
        u * 9.0,
        sprites::Pose::default(),
        tint,
    );
    let right = p.x >= center().x;
    let x = if right { p.x + u * 8.5 } else { p.x - u * 8.5 };
    let align = if right { 0.0 } else { 1.0 };
    let tc = if reached { col } else { rgb(0x97A3B5) };
    ui_label(label, vec2(x, p.y + u * 2.0), u * 5.5, tc, align);
}

fn ui_label(s: &str, p: Vec2, px: f32, col: Color, align: f32) {
    let w = text_width(s, px);
    text(s, p.x - w * align, p.y, px, col);
}
