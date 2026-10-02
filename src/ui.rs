//! Screen chrome and the overlay screens: the HUD, the lever panel, the
//! bottom bar, keystones, an animal's page and the boon pick. Every tappable
//! rect comes from a function here, so drawing and hit-testing agree.

use macroquad::prelude::*;

use crate::dial;
use crate::ecology::{self, Rule};
use crate::game::{self, Boon, Game, Phase, Report, Status};
use crate::genome;
use crate::genome::Morph;
use crate::planet::{self, Biome, Lever};
use crate::render::{
    self, draw_meter, faded, fit_px, rgb, text, text_centered, text_width, wrap_lines, ACCENT_DNA,
    ACCENT_OK, ACCENT_WARN, EDGE, HUD_BG, LOCKED_TEXT, PANEL, PANEL_LOCKED, SILHOUETTE, TEXT,
    TEXT_DIM, TRACK,
};
use crate::sprites::{self, Sprites};

pub const PINK: Color = rgb(0xFF7AB8);
/// Multiply tint for a keystone the planet can't support right now.
const DORMANT: Color = Color::new(0.42, 0.44, 0.5, 1.0);
pub const LIME: Color = rgb(0xC5F76A);
/// Radiation: the RAD badge and the new Earth.
pub const RAD_COLOR: Color = rgb(0xD4F542);
/// Multiply tint for a fossil: an animal found on an earlier Earth.
pub const STONE: Color = Color::new(0.58, 0.53, 0.46, 1.0);
pub const STONE_TEXT: Color = rgb(0x9A9182);
/// A keystone's drawback, or a lever it holds.
const LOCK_RED: Color = rgb(0xFF6A4A);

/// Textures the UI draws with, built once (they need a GL context).
pub struct Assets {
    pub glow: Texture2D,
    /// The settings gear, idle and lit.
    pub gear: [Texture2D; 2],
}

/// The settings gear, 16x16, lit from the top left: `l` light, `b` base,
/// `d` shadow, `k` ink.
const GEAR: [&str; 16] = [
    "......kkkk......",
    "......kllk......",
    "..kk..kllk..kk..",
    ".kllkklllbkkbbk.",
    "..klllllbbbbbk..",
    "...kllkkkkbbk...",
    "kkkllk....kbbkkk",
    "kllllk....kbdddk",
    "klllbk....kddddk",
    "kkkbbk....kddkkk",
    "...kbbkkkkddk...",
    "..kbbbbbdddddk..",
    ".kbbkkbdddkkddk.",
    "..kk..kddk..kk..",
    "......kddk......",
    "......kkkk......",
];

/// Light, base, shadow and ink: steel when idle, the DNA blue when lit.
const GEAR_INKS: [[u32; 4]; 2] = [
    [0xE6F0F6, 0xB8CAD6, 0x8CA0B2, 0x101420],
    [0xBFE8F0, 0x5BC8F5, 0x2F7F92, 0x0E242C],
];

fn gear_texture(inks: [u32; 4]) -> Texture2D {
    let mut buf = vec![0u8; 16 * 16 * 4];
    for (y, row) in GEAR.iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            let Some(i) = b"lbdk".iter().position(|&c| c == ch) else {
                continue;
            };
            let c = inks[i];
            let o = (y * 16 + x) * 4;
            buf[o..o + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
        }
    }
    let tex = Texture2D::from_rgba8(16, 16, &buf);
    tex.set_filter(FilterMode::Nearest);
    tex
}

impl Assets {
    pub fn build() -> Self {
        let n = 64usize;
        let mut buf = vec![0u8; n * n * 4];
        for y in 0..n {
            for x in 0..n {
                let d = (((x as f32 - 31.5).powi(2) + (y as f32 - 31.5).powi(2)).sqrt() / 31.5)
                    .min(1.0);
                let a = (1.0 - d).powf(2.2);
                let i = (y * n + x) * 4;
                buf[i..i + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
        let glow = Texture2D::from_rgba8(n as u16, n as u16, &buf);
        glow.set_filter(FilterMode::Linear);

        Self {
            glow,
            gear: GEAR_INKS.map(gear_texture),
        }
    }

    pub fn glow(&self, c: Vec2, r: f32, color: Color, alpha: f32) {
        draw_texture_ex(
            &self.glow,
            c.x - r,
            c.y - r,
            faded(color, alpha),
            DrawTextureParams {
                dest_size: Some(vec2(r * 2.0, r * 2.0)),
                ..Default::default()
            },
        );
    }
}

/// One design pixel: the UI sits on the backdrop's 180-wide grid.
pub fn u() -> f32 {
    screen_width() / 180.0
}

fn frame(r: Rect, fill: Color, edge: Color, w: f32) {
    draw_rectangle(r.x, r.y, r.w, r.h, fill);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, w, edge);
}

pub fn human_duration(secs: f64) -> String {
    let s = secs.round() as u64;
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{} min", s / 60)
    } else if s.is_multiple_of(3600) {
        format!("{}h", s / 3600)
    } else {
        format!("{}h{:02}", s / 3600, (s / 60) % 60)
    }
}

pub fn draw_taxon(
    sprites: &Sprites,
    game: &Game,
    taxon: usize,
    morph: Morph,
    c: Vec2,
    size: f32,
    tint: Color,
) {
    let anim = sprites::pose(game.taxon(taxon).group, taxon, get_time());
    sprites::draw_sprite(
        sprites.get_morph(taxon, morph),
        c,
        size * sprites::morph_scale(morph),
        anim,
        tint,
    );
}

/// The best morph owned, for drawing a collection entry.
pub fn best_morph(game: &Game, taxon: usize) -> Morph {
    game::MORPHS
        .iter()
        .rev()
        .copied()
        .find(|&m| game.morphs[taxon] & game::morph_bit(m) != 0)
        .unwrap_or(Morph::None)
}

// --- top HUD ------------------------------------------------------------------

/// The settings gear, at the right end of the top bar.
pub fn gear_rect() -> Rect {
    let bar_h = render::bar_height();
    let s = bar_h * 0.62;
    Rect::new(screen_width() - s - bar_h * 0.19, (bar_h - s) * 0.5, s, s)
}

/// The gear in its box; `lit` while settings are open.
fn draw_gear(assets: &Assets, lit: bool) {
    let r = gear_rect();
    if lit {
        frame(r, rgb(0x12303A), ACCENT_DNA, 1.0);
    } else {
        frame(r, PANEL, EDGE, 1.0);
    }
    // A whole number of texels per pixel keeps the pixel art crisp.
    let s = ((r.w * 0.72 / 16.0).floor().max(1.0) * 16.0).round();
    draw_texture_ex(
        &assets.gear[lit as usize],
        (r.x + (r.w - s) * 0.5).round(),
        (r.y + (r.h - s) * 0.5).round(),
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(s, s)),
            ..Default::default()
        },
    );
}

fn draw_top_bar() {
    let (sw, bar_h) = (screen_width(), render::bar_height());
    draw_rectangle(0.0, 0.0, sw, bar_h, HUD_BG);
    draw_line(0.0, bar_h, sw, bar_h, 1.0, EDGE);
}

/// The trefoil: three blades round a dot, `r` the blade radius.
pub fn draw_rad_icon(c: Vec2, r: f32, col: Color) {
    use std::f32::consts::PI;
    draw_circle(c.x, c.y, r * 0.2, col);
    for blade in 0..3 {
        let mid = -PI / 2.0 + blade as f32 * PI * 2.0 / 3.0;
        let slices = 5;
        for k in 0..slices {
            let a0 = mid - PI / 6.0 + PI / 3.0 * k as f32 / slices as f32;
            let a1 = mid - PI / 6.0 + PI / 3.0 * (k + 1) as f32 / slices as f32;
            let p = |a: f32, rr: f32| c + vec2(a.cos(), a.sin()) * rr;
            let (i0, o0, o1, i1) = (p(a0, r * 0.36), p(a0, r), p(a1, r), p(a1, r * 0.36));
            draw_triangle(i0, o0, o1, col);
            draw_triangle(i0, o1, i1, col);
        }
    }
}

/// The RAD badge, left of the gear, once Human has ended an Earth.
pub fn rad_badge_rect(game: &Game) -> Option<Rect> {
    if game.rad == 0 {
        return None;
    }
    let g = gear_rect();
    let h = g.h * 0.72;
    let w = h * 2.6;
    Some(Rect::new(g.x - w - u() * 3.0, g.y + (g.h - h) * 0.5, w, h))
}

fn draw_rad_badge(game: &Game) {
    let Some(r) = rad_badge_rect(game) else {
        return;
    };
    frame(r, rgb(0x1C2410), RAD_COLOR, 1.0);
    let icon = r.h * 0.32;
    draw_rad_icon(vec2(r.x + r.h * 0.45, r.y + r.h * 0.5), icon, RAD_COLOR);
    let label = format!("{} RAD", game.rad);
    let px = r.h * 0.5;
    text(
        &label,
        r.x + r.h * 0.85,
        r.y + r.h * 0.68,
        fit_px(&label, r.w - r.h * 0.95, px),
        RAD_COLOR,
    );
}

/// What the radiation gives, under the badge (tap the badge).
pub fn draw_rad_info(game: &Game) {
    let Some(b) = rad_badge_rect(game) else {
        return;
    };
    let u = u();
    let w = screen_width() * 0.62;
    let r = Rect::new(
        screen_width() - w - u * 4.0,
        b.y + b.h + u * 4.0,
        w,
        u * 30.0,
    );
    frame(r, rgb(0x0A0E08), RAD_COLOR, 1.5);
    let px = u * 6.0;
    let x = r.x + u * 4.0;
    text(
        &format!("RADIATION  ·  EARTH {}", game.rad + 1),
        x,
        r.y + u * 9.0,
        px,
        RAD_COLOR,
    );
    let perks = format!(
        "+{:.0} Luck  ·  +{:.0}% morphs",
        game::RAD_LUCK * game.rad as f32,
        game::RAD_MORPH * game.rad as f32 * 100.0
    );
    text(
        &perks,
        x,
        r.y + u * 17.0,
        fit_px(&perks, r.w - u * 8.0, px),
        LIME,
    );
    let times = if game.rad == 1 { "once" } else { "times" };
    let why = if game.rad == 1 {
        format!("Human ended the Earth {times}")
    } else {
        format!("Human ended the Earth {} {times}", game.rad)
    };
    text(
        &why,
        x,
        r.y + u * 25.0,
        fit_px(&why, r.w - u * 8.0, px * 0.85),
        TEXT_DIM,
    );
}

pub fn draw_hud(game: &Game, now: f64, assets: &Assets) {
    let bar_h = render::bar_height();
    let px = bar_h * 0.34;
    draw_top_bar();
    draw_gear(assets, false);
    draw_rad_badge(game);
    // Text stops short of the badge and the gear.
    let right = rad_badge_rect(game).map_or(gear_rect().x, |b| b.x);
    let max_w = right - px * 1.4;
    let head = format!("{} Ma  ·  {}", game.ma_elapsed(now), game.era());
    text(
        &head,
        px * 0.7,
        bar_h * 0.46,
        fit_px(&head, max_w, px),
        TEXT,
    );
    let best = game.most_advanced();
    let fossils = game.fossils();
    let sub = if fossils > 0 {
        format!(
            "{} / {} found  ·  {fossils} fossils",
            game.discovered(),
            game.phy.len()
        )
    } else {
        format!(
            "{} / {} found  ·  step {}  {}",
            game.discovered(),
            game.phy.len(),
            game.taxon(best).depth,
            game.taxon(best).name
        )
    };
    text(
        &sub,
        px * 0.7,
        bar_h * 0.84,
        fit_px(&sub, max_w, px * 0.72),
        TEXT_DIM,
    );
}

// --- the lever panel ------------------------------------------------------------

/// The lever panel, just above the bottom bar.
pub fn panel_rect() -> Rect {
    let (sw, sh) = (screen_width(), screen_height());
    let (w, h) = (sw * 0.84, sh * 0.31);
    Rect::new((sw - w) * 0.5, sh - render::bar_height() - h - 10.0, w, h)
}

fn lever_row(i: usize) -> Rect {
    let r = panel_rect();
    let head = r.h * 0.13;
    let foot = r.h * 0.22;
    let row_h = (r.h - head - foot) / 5.0;
    Rect::new(r.x, r.y + head + i as f32 * row_h, r.w, row_h)
}

pub fn lever_button(i: usize, plus: bool) -> Rect {
    let row = lever_row(i);
    let s = row.h * 0.82;
    let pad = row.h * 0.12;
    let x = if plus {
        row.x + row.w - pad - s
    } else {
        row.x + row.w - pad * 2.0 - s * 2.0
    };
    Rect::new(x, row.y + (row.h - s) * 0.5, s, s)
}

fn lever_value(game: &Game, lever: Lever) -> String {
    let p = &game.planet;
    match lever {
        Lever::Land => [
            "Water world",
            "Islands",
            "Coasts",
            "Continents",
            "Dry world",
            "Supercontinent",
        ][p.land as usize]
            .into(),
        Lever::Vegetation => [
            "Bare rock",
            "Moss",
            "Ferns",
            "Forest",
            "Dense forest",
            "Jungle",
        ][p.vegetation as usize]
            .into(),
        Lever::Oxygen => format!("{}% O2", planet::oxygen_percent(p.oxygen)),
        Lever::Temperature => planet::temperature_label(p.temperature).into(),
        Lever::Volcanism => ["Calm", "Active", "Violent"][p.volcanism as usize].into(),
    }
}

fn lever_color(lever: Lever) -> Color {
    match lever {
        Lever::Land => rgb(0xB89A6A),
        Lever::Vegetation => rgb(0x8FC878),
        Lever::Oxygen => LIME,
        Lever::Temperature => rgb(0xFFC56B),
        Lever::Volcanism => rgb(0xFF6A4A),
    }
}

/// The line under the levers: linked changes, what's missing, or what could evolve.
fn shape_hint(game: &Game) -> (String, Color) {
    if let Some((k, why)) = game
        .keystones
        .iter()
        .find_map(|&k| game.dormant_reason(k).map(|why| (k, why)))
    {
        return (
            format!("Keystone {} is dormant: {why}", game.taxon(k).name),
            ACCENT_WARN,
        );
    }
    if let Some(why) = Lever::ALL.iter().find_map(|&l| {
        game.lever_lock(l, -1)
            .or_else(|| game.lever_lock(l, 1))
            .map(|w| format!("{}: {w}", l.name()))
    }) {
        return (why, LOCK_RED);
    }
    let (now, after) = (game.planet, game.planet_after());
    let mut changes = Vec::new();
    for l in Lever::ALL {
        let (a, b) = (now.get(l), after.get(l));
        if a != b {
            changes.push(format!(
                "{} {}{}",
                l.name(),
                if b > a { "+" } else { "-" },
                a.abs_diff(b)
            ));
        }
    }
    if !changes.is_empty() {
        return (
            format!("After this cycle: {}", changes.join(", ")),
            ACCENT_WARN,
        );
    }
    let n = game.eligible_count();
    if n == 0 {
        if let Some(h) = game.blocked_hint() {
            return (h, ACCENT_WARN);
        }
        return ("Nothing new can evolve yet".into(), TEXT_DIM);
    }
    // Otherwise: what keeps the other biomes away.
    let here = Biome::of(&game.planet);
    let others: Vec<String> = Biome::ALL
        .into_iter()
        .filter(|b| !here.contains(b))
        .filter_map(|b| {
            let why = b.ranges().missing(&game.planet, 0)?;
            Some(format!("{}: {why}", b.name()))
        })
        .take(2)
        .collect();
    (others.join("  ·  "), LOCKED_TEXT)
}

/// The button that gives up a running wait, in the lever panel's corner.
pub fn cancel_wait_rect() -> Rect {
    let r = panel_rect();
    let (w, h) = (u() * 26.0, u() * 8.0);
    Rect::new(r.x + r.w - w - u() * 3.0, r.y + u() * 2.5, w, h)
}

/// `cancel_armed`: the cancel button was tapped once and asks to be confirmed.
/// The keystones the running wait was launched with and what each adds.
fn draw_launched_keystones(game: &Game, sprites: &Sprites, r: Rect, px: f32) {
    let ks = game.active_keystones();
    let y = r.y + r.h * 0.86;
    let x = r.x + r.w * 0.04;
    if ks.is_empty() {
        text("No keystones were equipped", x, y, px * 0.9, TEXT_DIM);
        return;
    }
    text(
        "KEYSTONES AT LAUNCH",
        x,
        r.y + r.h * 0.72,
        px * 0.85,
        TEXT_DIM,
    );
    let slot = r.w * 0.92 / ks.len().max(3) as f32;
    for (k, &t) in ks.iter().enumerate() {
        let sx = x + k as f32 * slot;
        let dormant = game.dormant_reason(t).is_some();
        let icon = px * 1.8;
        draw_taxon(
            sprites,
            game,
            t,
            best_morph(game, t),
            vec2(sx + icon * 0.5, y - px * 0.3),
            icon,
            if dormant { DORMANT } else { WHITE },
        );
        let (e, c) = keystone_effect(game, t);
        text(
            &e,
            sx + icon + u(),
            y,
            fit_px(&e, slot - icon - u() * 2.0, px * 0.85),
            c,
        );
    }
}

pub fn draw_lever_panel(
    game: &Game,
    now: f64,
    assets: &Assets,
    sprites: &Sprites,
    cancel_armed: bool,
) {
    let r = panel_rect();
    let t = get_time() as f32;
    frame(r, faded(rgb(0x05090F), 0.88), rgb(0x1B2C48), 1.5);
    let px = r.h * 0.07;

    match game.phase() {
        Phase::Running => {
            let c = game.cycle.expect("running has a cycle");
            let left = c.remaining(now);
            let cx = r.x + r.w * 0.5;
            text_centered("TIME IS RUNNING", cx, r.y + r.h * 0.16, px, TEXT_DIM);
            text_centered(
                &planet::format_remaining(left),
                cx,
                r.y + r.h * 0.40,
                r.h * 0.2,
                LIME,
            );
            let bar = Rect::new(r.x + r.w * 0.08, r.y + r.h * 0.48, r.w * 0.84, r.h * 0.035);
            draw_meter(bar.x, bar.y, bar.w, bar.h, c.progress(now) as f32, LIME);
            let msg = format!("{} million years pass  ·  you can close the app", c.ma);
            text_centered(
                &msg,
                cx,
                r.y + r.h * 0.62,
                fit_px(&msg, r.w * 0.9, px * 0.9),
                TEXT_DIM,
            );
            let b = cancel_wait_rect();
            let (label, col) = if cancel_armed {
                ("TAP TO CONFIRM", ACCENT_WARN)
            } else {
                ("CANCEL", TEXT_DIM)
            };
            frame(b, rgb(0x0A1422), col, 1.5);
            text_centered(
                label,
                b.x + b.w * 0.5,
                b.y + b.h * 0.75,
                fit_px(label, b.w * 0.9, b.h * 0.7),
                col,
            );
            draw_launched_keystones(game, sprites, r, px);
        }
        Phase::Genome | Phase::Boon => {
            let cx = r.x + r.w * 0.5;
            let pulse = 0.5 + 0.5 * (t * 3.0).sin();
            assets.glow(
                vec2(cx, r.y + r.h * 0.45),
                r.h * 0.5,
                ACCENT_WARN,
                0.25 + pulse * 0.2,
            );
            text_centered(
                "THE AGES PRODUCED",
                cx,
                r.y + r.h * 0.34,
                px * 1.1,
                TEXT_DIM,
            );
            text_centered(
                "A NEW GENOME",
                cx,
                r.y + r.h * 0.52,
                r.h * 0.14,
                ACCENT_WARN,
            );
            text_centered(
                "tap to evolve it",
                cx,
                r.y + r.h * 0.70,
                px,
                faded(TEXT, 0.7),
            );
            if game.doom_risk > 0.0 {
                let warn = format!(
                    "HUMAN: {:.0}% THIS GENOME ENDS THE EARTH",
                    game.doom_risk * 100.0
                );
                text_centered(
                    &warn,
                    cx,
                    r.y + r.h * 0.86,
                    fit_px(&warn, r.w * 0.9, px),
                    LOCK_RED,
                );
            }
        }
        Phase::Shape => {
            text(
                "SHAPE THE PLANET",
                r.x + r.w * 0.04,
                r.y + r.h * 0.10,
                px,
                ACCENT_WARN,
            );
            let left = game.points_left();
            let max = game.max_points();
            let dot = r.h * 0.045;
            for k in 0..max {
                let x = r.x + r.w * 0.96 - (max - k) as f32 * dot * 1.6;
                draw_rectangle(
                    x,
                    r.y + r.h * 0.05,
                    dot,
                    dot,
                    if k < left { rgb(0x6FF5E1) } else { TRACK },
                );
            }
            for (i, &lever) in Lever::ALL.iter().enumerate() {
                let row = lever_row(i);
                let col = lever_color(lever);
                let rpx = row.h * 0.42;
                text(
                    lever.name(),
                    row.x + r.w * 0.04,
                    row.y + row.h * 0.64,
                    rpx,
                    TEXT,
                );
                let value = lever_value(game, lever);
                text(
                    &value,
                    row.x + r.w * 0.32,
                    row.y + row.h * 0.64,
                    fit_px(&value, r.w * 0.26, rpx * 0.9),
                    col,
                );
                // Pips: filled to the level, the reachable range dimmer.
                let max_l = game.planet.max(lever);
                let top_l = if lever == Lever::Volcanism {
                    planet::VOLCANISM_MAX
                } else {
                    planet::LEVEL_MAX
                };
                let pip_w = r.w * 0.028;
                let px0 = row.x + r.w * 0.60;
                for k in 0..top_l {
                    let c = if k < game.planet.get(lever) {
                        col
                    } else if k < max_l {
                        TRACK
                    } else {
                        rgb(0x0C141E)
                    };
                    draw_rectangle(
                        px0 + k as f32 * pip_w * 1.35,
                        row.y + row.h * 0.3,
                        pip_w,
                        row.h * 0.4,
                        c,
                    );
                }
                for (plus, label) in [(false, "-"), (true, "+")] {
                    let b = lever_button(i, plus);
                    let delta = if plus { 1 } else { -1 };
                    let ok = game.can_step(lever, delta);
                    let locked = game.lever_lock(lever, delta).is_some();
                    let (fill, edge, ink) = if locked {
                        (rgb(0x2A1512), LOCK_RED, LOCK_RED)
                    } else if ok {
                        (rgb(0x0A1422), rgb(0x2F6B72), TEXT)
                    } else {
                        (rgb(0x070B12), rgb(0x141C26), LOCKED_TEXT)
                    };
                    frame(b, fill, edge, 1.5);
                    text_centered(label, b.x + b.w * 0.5, b.y + b.h * 0.72, b.h * 0.7, ink);
                }
            }
            draw_biome_line(game, r, px);
            let (hint, col) = shape_hint(game);
            text(
                &hint,
                r.x + r.w * 0.04,
                r.y + r.h * 0.965,
                fit_px(&hint, r.w * 0.92, px * 0.9),
                col,
            );
        }
    }
}

/// The biome the planet is, and what can evolve.
fn draw_biome_line(game: &Game, r: Rect, px: f32) {
    let x = r.x + r.w * 0.04;
    let y = r.y + r.h * 0.87;
    let biomes = Biome::of(&game.planet);
    let mut cx = x;
    if biomes.is_empty() {
        text("NO BIOME", cx, y, px * 0.85, LOCKED_TEXT);
        cx += text_width("NO BIOME", px * 0.85) + px * 0.8;
    }
    for b in biomes {
        let label = b.name().to_uppercase();
        let tw = text_width(&label, px * 0.85);
        draw_rectangle(cx, y - px * 0.85, tw + px * 0.5, px * 1.05, rgb(b.color()));
        text(&label, cx + px * 0.25, y, px * 0.85, render::BG);
        cx += tw + px * 1.0;
    }
    let n = game.eligible_count();
    let msg = format!("{n} species can evolve here");
    text(
        &msg,
        cx,
        y,
        fit_px(&msg, r.x + r.w * 0.96 - cx, px * 0.85),
        rgb(0x6FF5E1),
    );
}

// --- the wait panel (with the dial) ------------------------------------------------

/// "4H", or "4H30" for a half hour.
pub fn hours_label(hours: f32) -> String {
    if hours.fract() > 0.0 {
        format!("{}H30", hours.floor())
    } else {
        format!("{hours}H")
    }
}

/// Between the dial's halo and the bottom bar.
pub fn wait_panel_rect() -> Rect {
    let r = panel_rect();
    let top = dial::center().y + dial::radius() + dial::max_half_width() + u() * 3.0;
    let bottom = r.y + r.h;
    let top = top.min(bottom - u() * 52.0);
    Rect::new(r.x, top, r.w, bottom - top)
}

pub fn wait_back_rect() -> Rect {
    let r = wait_panel_rect();
    let (w, h) = (u() * 22.0, u() * 8.0);
    Rect::new(r.x + r.w - w - u() * 3.0, r.y + u() * 2.5, w, h)
}

/// What an equipped keystone adds, short enough for the wait panel.
fn keystone_effect(game: &Game, taxon: usize) -> (String, Color) {
    match game
        .keystone_reports()
        .into_iter()
        .find(|r| r.taxon == taxon)
    {
        Some(r) => report_line(&r, true),
        None => ("-".into(), TEXT_DIM),
    }
}

/// A keystone's status or gains in a line; `short` drops the reasons.
fn report_line(r: &Report, short: bool) -> (String, Color) {
    match &r.status {
        Status::Asleep(_) if short => ("asleep".into(), ACCENT_WARN),
        Status::Asleep(why) => (format!("Asleep: {why}"), ACCENT_WARN),
        Status::Waiting(_) if short => ("waiting".into(), TEXT_DIM),
        Status::Waiting(why) => (format!("Waiting: {why}"), TEXT_DIM),
        Status::Active => {
            let mut line = r.summary();
            if let (Some(note), false) = (&r.note, short) {
                line = format!("{line}  ·  {note}");
            }
            (line, LIME)
        }
    }
}

/// What the chosen wait and the keystones will bring: the rarity odds, the
/// cards and the morph chance, then each keystone's share.
pub fn draw_wait_panel(game: &Game, sprites: &Sprites) {
    let r = wait_panel_rect();
    let u = u();
    frame(r, faded(rgb(0x05090F), 0.9), rgb(0x1B2C48), 1.5);
    let f = game.forecast();
    let row = r.h / 5.4;
    let px = (row * 0.62).min(u * 6.5);
    let (x, w) = (r.x + r.w * 0.04, r.w * 0.92);
    let mut y = r.y + row * 0.85;

    text("THE AGES WILL BRING", x, y, px, ACCENT_WARN);
    let b = wait_back_rect();
    frame(b, rgb(0x0A1422), rgb(0x2F6B72), 1.5);
    text_centered("BACK", b.x + b.w * 0.5, b.y + b.h * 0.75, b.h * 0.7, TEXT);

    // Rarity odds, as one bar in the tier colours.
    y += row * 0.35;
    let weights = genome::tier_weights(f.odds.luck);
    let mut bx = x;
    draw_rectangle(x, y, w, u * 3.5, TRACK);
    for (t, &pct) in ecology::Tier::ALL.iter().zip(&weights) {
        let bw = w * pct / 100.0;
        draw_rectangle(bx, y, bw, u * 3.5, render::tier_color(*t));
        bx += bw;
    }
    y += u * 3.5 + row * 0.75;
    let cell = w / 5.0;
    for (k, (t, &pct)) in ecology::Tier::ALL.iter().zip(&weights).enumerate() {
        let label = format!("{} {:.0}%", &t.name()[..1], pct.max(0.0));
        let label = if pct < 1.0 {
            format!("{} {pct:.1}%", &t.name()[..1])
        } else {
            label
        };
        text(
            &label,
            x + k as f32 * cell,
            y,
            px * 0.9,
            render::tier_color(*t),
        );
    }

    // The cards: solid ones are sure, a dashed one is a chance.
    y += row * 0.4;
    let (cw, ch) = (u * 5.0, u * 7.0);
    let sure = f.odds.cards;
    let chance = f.cards.fract();
    let col = dial::color(game.wait_hours);
    // As many slots as cards (no cap), squeezed into the room of six.
    let area = 6.0 * (cw + u * 1.5);
    let slots = (sure + (chance > 0.0) as usize).max(1);
    let step = (area / slots as f32).min(cw + u * 1.5);
    let cw = cw.min(step - u * 0.5);
    for k in 0..slots {
        let cx = x + k as f32 * step;
        let (fill, edge) = if k < sure {
            (rgb(0x0A1020), col)
        } else if k == sure && chance > 0.0 {
            (rgb(0x05090F), rgb(0x5A6678))
        } else {
            (rgb(0x05090F), rgb(0x141C26))
        };
        frame(Rect::new(cx, y, cw, ch), fill, edge, 1.5);
    }
    let tx = x + area + u * 2.0;
    let cards = if chance > 0.0 {
        format!("{sure} cards  ·  {:.0}% for one more", chance * 100.0)
    } else {
        format!("{sure} cards")
    };
    let tw = x + w - tx;
    text(
        &cards,
        tx,
        y + ch * 0.45,
        fit_px(&cards, tw, px * 0.9),
        TEXT,
    );
    let extra = if f.odds.catalyst {
        ("one Rare or better is sure".to_string(), rgb(0xFFC56B))
    } else {
        (
            format!(
                "{:.1}% of cards are morphs",
                (genome::morph_chance(f.odds.morph_mult) * f.odds.morph_rad).min(1.0) * 100.0
            ),
            PINK,
        )
    };
    text(
        &extra.0,
        tx,
        y + ch * 1.05,
        fit_px(&extra.0, tw, px * 0.85),
        extra.1,
    );

    // Each keystone's share.
    y += ch + row * 0.75;
    if game.keystones.is_empty() {
        text("No keystones equipped", x, y, px * 0.9, TEXT_DIM);
        return;
    }
    let slot = w / game.keystones.len().max(3) as f32;
    for (k, &t) in game.keystones.iter().enumerate() {
        let sx = x + k as f32 * slot;
        let dormant = game.dormant_reason(t).is_some();
        let icon = px * 1.3;
        draw_taxon(
            sprites,
            game,
            t,
            best_morph(game, t),
            vec2(sx + icon * 0.5, y - px * 0.35),
            icon,
            if dormant { DORMANT } else { WHITE },
        );
        let (e, c) = keystone_effect(game, t);
        text(
            &e,
            sx + icon + u,
            y,
            fit_px(&e, slot - icon - u * 2.0, px * 0.85),
            c,
        );
    }
}

// --- bottom bar -------------------------------------------------------------------

pub fn bottom_action_rect() -> Rect {
    let (sw, bar_h) = (screen_width(), render::bar_height());
    let w = sw * 0.44;
    Rect::new((sw - w) * 0.5, screen_height() - bar_h, w, bar_h)
}

pub fn bottom_tab_rect(right: bool) -> Rect {
    let (sw, bar_h) = (screen_width(), render::bar_height());
    let a = bottom_action_rect();
    let y = screen_height() - bar_h;
    if right {
        Rect::new(a.x + a.w, y, sw - a.x - a.w, bar_h)
    } else {
        Rect::new(0.0, y, a.x, bar_h)
    }
}

/// `active`: which tab is lit -- `Some(false)` Keystones, `Some(true)` Map.
/// `choosing`: the wait dial is up, so the button confirms it.
pub fn draw_bottom_bar(
    game: &Game,
    now: f64,
    active: Option<bool>,
    choosing: bool,
    assets: &Assets,
) {
    let (sw, bar_h) = (screen_width(), render::bar_height());
    let y = screen_height() - bar_h;
    draw_rectangle(0.0, y, sw, bar_h, HUD_BG);
    draw_line(0.0, y, sw, y, 1.0, EDGE);
    for (right, label) in [(false, "Keystones"), (true, "Map")] {
        let r = bottom_tab_rect(right);
        let on = active == Some(right);
        if on {
            draw_rectangle(r.x, r.y, r.w, 3.0, ACCENT_DNA);
        }
        text_centered(
            label,
            r.x + r.w * 0.5,
            r.y + r.h * 0.6,
            bar_h * 0.26,
            if on { ACCENT_DNA } else { TEXT_DIM },
        );
        if !right
            && game
                .keystones
                .iter()
                .any(|&k| game.dormant_reason(k).is_some())
        {
            let w = text_width(label, bar_h * 0.26);
            draw_circle(
                r.x + r.w * 0.5 + w * 0.5 + bar_h * 0.1,
                r.y + r.h * 0.42,
                bar_h * 0.06,
                ACCENT_WARN,
            );
        }
    }

    let e = bottom_action_rect();
    let (ey, eh) = (e.y + u() * 2.0, e.h - u() * 4.0);
    let t = get_time() as f32;
    let (label, sub, col, hot) = match game.phase() {
        Phase::Shape if choosing => {
            let f = game.forecast();
            (
                format!("LET {} RUN", hours_label(game.wait_hours)),
                format!(
                    "{} Ma  ·  {} card{}  ·  {} pt{}",
                    f.ma,
                    f.odds.cards,
                    if f.odds.cards == 1 { "" } else { "s" },
                    f.points,
                    if f.points == 1 { "" } else { "s" }
                ),
                ACCENT_OK,
                true,
            )
        }
        Phase::Shape if game.wait_choosable() => (
            "LET TIME RUN".to_string(),
            "choose how long: 2h to 6h".to_string(),
            ACCENT_OK,
            true,
        ),
        Phase::Shape => (
            "LET TIME RUN".to_string(),
            format!(
                "{}  =  {} Ma",
                human_duration(game.next_cycle_seconds()),
                game.forecast().ma
            ),
            ACCENT_OK,
            true,
        ),
        Phase::Running => (
            planet::format_remaining(game.cycle.map(|c| c.remaining(now)).unwrap_or(0.0)),
            "until it evolves".to_string(),
            TEXT_DIM,
            false,
        ),
        Phase::Genome if game.doom_risk > 0.0 => (
            "EVOLVE IT".to_string(),
            format!("{:.0}% the Earth ends", game.doom_risk * 100.0),
            ACCENT_WARN,
            true,
        ),
        Phase::Genome => (
            "EVOLVE IT".to_string(),
            "a new genome is ready".to_string(),
            ACCENT_WARN,
            true,
        ),
        Phase::Boon => (
            "CHOOSE A BOON".to_string(),
            String::new(),
            ACCENT_WARN,
            true,
        ),
    };
    if hot {
        assets.glow(
            vec2(e.x + e.w * 0.5, ey + eh * 0.5),
            e.w * 0.7,
            col,
            0.18 + 0.1 * (t * 3.0).sin(),
        );
    }
    frame(
        Rect::new(e.x, ey, e.w, eh),
        if hot { faded(col, 0.16) } else { PANEL },
        col,
        2.0,
    );
    text_centered(
        &label,
        e.x + e.w * 0.5,
        ey + eh * 0.48,
        fit_px(&label, e.w * 0.9, bar_h * 0.28),
        col,
    );
    if !sub.is_empty() {
        text_centered(
            &sub,
            e.x + e.w * 0.5,
            ey + eh * 0.82,
            fit_px(&sub, e.w * 0.9, bar_h * 0.2),
            faded(col, 0.8),
        );
    }
}

// --- keystones screen -----------------------------------------------------------------

#[derive(Default)]
pub struct KeystoneView {
    pub scroll: f32,
    pub selected: Option<usize>,
}

fn slot_rect(i: usize) -> Rect {
    let sw = screen_width();
    let h = u() * 21.0;
    Rect::new(
        sw * 0.04,
        render::bar_height() + u() * 19.0 + i as f32 * (h + u() * 2.0),
        sw * 0.92,
        h,
    )
}

fn grid_top(game: &Game) -> f32 {
    slot_rect(game.keystone_slots()).y + u() * 14.0
}

fn cell_size() -> f32 {
    screen_width() * 0.92 / 7.0
}

fn info_rect() -> Rect {
    let sw = screen_width();
    let h = u() * 46.0;
    Rect::new(
        sw * 0.04,
        screen_height() - render::bar_height() - h - u() * 2.0,
        sw * 0.92,
        h,
    )
}

pub fn keystone_equip_rect() -> Rect {
    let r = info_rect();
    Rect::new(r.x + r.w * 0.64, r.y + r.h * 0.1, r.w * 0.32, r.h * 0.3)
}

pub fn keystone_info_rect() -> Rect {
    let r = info_rect();
    Rect::new(r.x + r.w * 0.64, r.y + r.h * 0.46, r.w * 0.32, r.h * 0.22)
}

/// Picks the morph the selected keystone works with.
pub fn keystone_morph_rect() -> Rect {
    let r = info_rect();
    Rect::new(r.x + r.w * 0.64, r.y + r.h * 0.74, r.w * 0.32, r.h * 0.2)
}

/// A fossil's level: kept from the Earth it was found on.
fn level_for_fossil(game: &Game, t: usize) -> u32 {
    game::level_for(game.specimens[t].max(1))
}

/// A morph's tab colour and the ink for its name.
pub fn morph_colors(m: Morph) -> (Color, Color) {
    match m {
        Morph::Amber => (rgb(0xFFB048), rgb(0x0E1116)),
        Morph::Albino => (rgb(0xF2F5F8), rgb(0x0E1116)),
        Morph::Melanistic => (rgb(0x2A2F3A), TEXT),
        _ => (rgb(0x5BC8F5), rgb(0x0E1116)),
    }
}

/// A morph's name on a coloured tab, bottom-left at (`x`, `bottom`).
fn morph_badge(m: Morph, x: f32, bottom: f32, h: f32) {
    let (fill, ink) = morph_colors(m);
    let label = m.name().to_uppercase();
    let px = h * 0.8;
    let w = text_width(&label, px) + h * 0.4;
    draw_rectangle(x, bottom - h, w, h, fill);
    text(&label, x + h * 0.2, bottom - h * 0.22, px, ink);
}

/// Found animals, then the fossils waiting to be found again.
fn collection(game: &Game) -> Vec<usize> {
    let n = game.phy.len();
    (0..n)
        .filter(|&i| game.unlocked[i])
        .chain((0..n).filter(|&i| game.is_fossil(i)))
        .collect()
}

pub fn keystone_cell_at(game: &Game, view: &KeystoneView, p: Vec2) -> Option<usize> {
    let cs = cell_size();
    let top = grid_top(game) - view.scroll;
    if p.y < grid_top(game) || p.y > info_rect().y {
        return None;
    }
    let x0 = screen_width() * 0.04;
    let col = ((p.x - x0) / cs).floor();
    let row = ((p.y - top) / cs).floor();
    if !(0.0..7.0).contains(&col) || row < 0.0 {
        return None;
    }
    collection(game)
        .get(row as usize * 7 + col as usize)
        .copied()
}

pub fn keystone_slot_at(game: &Game, p: Vec2) -> Option<usize> {
    (0..game.keystone_slots())
        .find(|&i| slot_rect(i).contains(p))
        .and_then(|i| game.keystones.get(i).copied())
}

pub fn keystones_max_scroll(game: &Game) -> f32 {
    let rows = collection(game).len().div_ceil(7) as f32;
    (rows * cell_size() - (info_rect().y - grid_top(game))).max(0.0)
}

pub fn draw_keystones(
    game: &Game,
    now: f64,
    view: &KeystoneView,
    sprites: &Sprites,
    assets: &Assets,
) {
    clear_background(render::BG);
    let sw = screen_width();
    let top = render::bar_height();
    let px = u() * 7.5;
    text(
        "KEYSTONES",
        sw * 0.04,
        top + u() * 9.0,
        px * 1.2,
        ACCENT_WARN,
    );
    let note = if game.keystones_editable() {
        "A keystone only works while the planet suits it"
    } else {
        "Locked until you open the waiting genome"
    };
    text(
        note,
        sw * 0.04,
        top + u() * 15.5,
        fit_px(note, sw * 0.92, px * 0.8),
        TEXT_DIM,
    );

    let reports = game.keystone_reports();
    for i in 0..game.keystone_slots() {
        let r = slot_rect(i);
        match game.keystones.get(i) {
            Some(&t) => {
                let tier = ecology::of(game.taxon(t).name).tier;
                let rep = reports.iter().find(|x| x.taxon == t);
                let asleep = rep.is_none_or(|x| matches!(x.status, Status::Asleep(_)));
                let col = if asleep {
                    ACCENT_WARN
                } else {
                    render::tier_color(tier)
                };
                frame(
                    r,
                    rgb(0x0A1020),
                    faded(col, if asleep { 0.6 } else { 1.0 }),
                    1.5,
                );
                draw_rectangle(r.x + 2.0, r.y + 2.0, r.h - 4.0, r.h - 4.0, rgb(0x050A14));
                let morph = game.edition(t);
                draw_taxon(
                    sprites,
                    game,
                    t,
                    morph,
                    vec2(r.x + r.h * 0.5, r.y + r.h * 0.5),
                    r.h * 0.8,
                    if asleep { DORMANT } else { WHITE },
                );
                if morph != Morph::None {
                    morph_badge(morph, r.x + 2.0, r.y + r.h - 2.0, r.h * 0.22);
                }
                text(
                    game.taxon(t).name,
                    r.x + r.h * 1.15,
                    r.y + r.h * 0.42,
                    fit_px(game.taxon(t).name, r.w * 0.5, r.h * 0.3),
                    if asleep { TEXT_DIM } else { TEXT },
                );
                let (line, line_col) = match rep {
                    Some(x) => report_line(x, false),
                    None => (String::new(), TEXT_DIM),
                };
                text(
                    &line,
                    r.x + r.h * 1.15,
                    r.y + r.h * 0.8,
                    fit_px(&line, r.w - r.h * 1.3, r.h * 0.28),
                    line_col,
                );
                let (tag, tag_col) = match rep.map(|x| &x.status) {
                    Some(Status::Active) => ("ACTIVE", LIME),
                    Some(Status::Waiting(_)) => ("WAITING", TEXT_DIM),
                    _ => ("ASLEEP", ACCENT_WARN),
                };
                text(
                    tag,
                    r.x + r.w - r.h * 0.2 - text_width(tag, r.h * 0.26),
                    r.y + r.h * 0.42,
                    r.h * 0.26,
                    tag_col,
                );
            }
            None => {
                frame(r, rgb(0x070B12), rgb(0x1B2C48), 1.0);
                text_centered(
                    "empty slot  ·  pick an animal below",
                    r.x + r.w * 0.5,
                    r.y + r.h * 0.62,
                    r.h * 0.26,
                    LOCKED_TEXT,
                );
            }
        }
    }
    let next = match game.keystone_slots() {
        3 => Some(20),
        4 => Some(40),
        _ => None,
    };
    if let Some(n) = next {
        let msg = format!("Another slot at {n} finds ({}/{n})", game.discovered());
        text(
            &msg,
            sw * 0.04,
            grid_top(game) - u() * 4.0,
            px * 0.8,
            LOCKED_TEXT,
        );
    }

    let cs = cell_size();
    let (gtop, gbot) = (grid_top(game), info_rect().y);
    let x0 = sw * 0.04;
    for (k, &t) in collection(game).iter().enumerate() {
        let (col, row) = (k % 7, k / 7);
        let r = Rect::new(
            x0 + col as f32 * cs + 1.0,
            gtop + row as f32 * cs - view.scroll + 1.0,
            cs - 2.0,
            cs - 2.0,
        );
        if r.y + r.h < gtop || r.y > gbot {
            continue;
        }
        let equipped = game.keystones.contains(&t);
        let sel = view.selected == Some(t);
        let tier = ecology::of(game.taxon(t).name).tier;
        let dormant = game.dormant_reason(t).is_some();
        let fossil = game.is_fossil(t);
        frame(
            r,
            if equipped {
                rgb(0x12213A)
            } else if fossil {
                rgb(0x14120F)
            } else {
                rgb(0x0A1020)
            },
            if sel {
                WHITE
            } else if fossil {
                rgb(0x4A443C)
            } else if equipped && dormant {
                ACCENT_WARN
            } else if equipped {
                rgb(0x6FF5E1)
            } else {
                faded(render::tier_color(tier), if dormant { 0.25 } else { 0.6 })
            },
            if sel { 2.5 } else { 1.0 },
        );
        draw_taxon(
            sprites,
            game,
            t,
            best_morph(game, t),
            vec2(r.x + r.w * 0.5, r.y + r.h * 0.5),
            r.w * 0.7,
            if fossil {
                STONE
            } else if dormant {
                DORMANT
            } else {
                WHITE
            },
        );
    }
    // The info panel's own background hides the scrolled overflow.
    draw_rectangle(
        0.0,
        gbot - u(),
        sw,
        screen_height() - gbot + u(),
        render::BG,
    );

    let r = info_rect();
    frame(r, rgb(0x070D1C), rgb(0x2F6B72), 1.5);
    match view.selected {
        Some(t) => {
            let eco = ecology::of(game.taxon(t).name);
            let col = render::tier_color(eco.tier);
            let icon = vec2(r.x + r.h * 0.45, r.y + r.h * 0.42);
            assets.glow(icon, r.h * 0.45, col, 0.25);
            draw_taxon(
                sprites,
                game,
                t,
                best_morph(game, t),
                icon,
                r.h * 0.55,
                WHITE,
            );
            // The left column, up to the buttons.
            let tx = r.x + r.h * 0.9;
            let col_w = keystone_equip_rect().x - tx - r.w * 0.02;
            text(
                game.taxon(t).name,
                tx,
                r.y + r.h * 0.2,
                fit_px(game.taxon(t).name, col_w, r.h * 0.16),
                TEXT,
            );
            text(eco.tier.name(), tx, r.y + r.h * 0.36, r.h * 0.13, col);
            let px = r.h * 0.12;
            let mut lines: Vec<(String, Color)> = wrap_lines(&eco.describe(), col_w, px)
                .into_iter()
                .map(|l| (l, LIME))
                .collect();
            if let Rule::Catch(c) = eco.rule {
                lines.push((format!("But: {}", c.drawback()), rgb(0xFF6A4A)));
            }
            for (i, (line, c)) in lines.iter().take(3).enumerate() {
                let y = r.y + r.h * (0.52 + 0.13 * i as f32);
                text(line, tx, y, fit_px(line, col_w, px), *c);
            }
            let (status, status_col) = match game.dormant_reason(t) {
                _ if game.is_fossil(t) => (
                    format!(
                        "Fossil  ·  Lv {}  ·  find it again to equip",
                        level_for_fossil(game, t)
                    ),
                    STONE_TEXT,
                ),
                Some(why) => (format!("Asleep here: {why}"), ACCENT_WARN),
                None => (
                    format!(
                        "Thrives here  ·  Lv {}  ·  x{:.1}",
                        game.level[t],
                        game.keystone_strength(t)
                    ),
                    TEXT_DIM,
                ),
            };
            let sx = r.x + r.w * 0.04;
            text(
                &status,
                sx,
                r.y + r.h * 0.93,
                fit_px(
                    &status,
                    keystone_morph_rect().x - sx - r.w * 0.02,
                    r.h * 0.12,
                ),
                status_col,
            );
            // The morph it works with, when it owns one to pick.
            if game.morphs[t] != 0 {
                let b = keystone_morph_rect();
                let can = game.phase() == Phase::Shape;
                let m = game.edition(t);
                frame(
                    b,
                    PANEL,
                    if can { rgb(0x6FF5E1) } else { rgb(0x1B2C48) },
                    1.0,
                );
                let label = if m == Morph::None {
                    "MORPH: NONE".to_string()
                } else {
                    format!("MORPH: {}", m.name().to_uppercase())
                };
                text_centered(
                    &label,
                    b.x + b.w * 0.5,
                    b.y + b.h * 0.7,
                    fit_px(&label, b.w * 0.9, b.h * 0.5),
                    if can { TEXT } else { LOCKED_TEXT },
                );
            }
            let equipped = game.keystones.contains(&t);
            let can = game.keystones_editable()
                && game.unlocked[t]
                && (equipped || game.keystones.len() < game.keystone_slots());
            let b = keystone_equip_rect();
            frame(
                b,
                if can {
                    faded(ACCENT_OK, 0.15)
                } else {
                    PANEL_LOCKED
                },
                if can { ACCENT_OK } else { rgb(0x1B2C48) },
                1.5,
            );
            let label = if equipped { "REMOVE" } else { "EQUIP" };
            text_centered(
                label,
                b.x + b.w * 0.5,
                b.y + b.h * 0.68,
                b.h * 0.45,
                if can { ACCENT_OK } else { LOCKED_TEXT },
            );
            let b = keystone_info_rect();
            frame(b, PANEL, TEXT_DIM, 1.0);
            text_centered(
                "DETAILS",
                b.x + b.w * 0.5,
                b.y + b.h * 0.7,
                b.h * 0.45,
                TEXT_DIM,
            );
        }
        None => {
            text_centered(
                "Tap an animal to see its bonus",
                r.x + r.w * 0.5,
                r.y + r.h * 0.55,
                r.h * 0.12,
                TEXT_DIM,
            );
        }
    }
    draw_hud(game, now, assets);
}

// --- an animal's page ------------------------------------------------------------------

fn detail_rect() -> Rect {
    let (sw, sh) = (screen_width(), screen_height());
    Rect::new(sw * 0.06, sh * 0.12, sw * 0.88, sh * 0.76)
}

pub fn draw_detail(game: &Game, taxon: usize, sprites: &Sprites, assets: &Assets) {
    let (sw, sh) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, faded(render::BG, 0.82));
    let r = detail_rect();
    let t = game.taxon(taxon);
    let found = game.unlocked[taxon];
    let eco = ecology::of(t.name);
    let col = if found {
        render::tier_color(eco.tier)
    } else {
        LOCKED_TEXT
    };
    frame(r, rgb(0x070D1C), col, 2.0);
    let cx = r.x + r.w * 0.5;
    let sy = r.y + r.h * 0.17;
    if found {
        assets.glow(vec2(cx, sy), r.w * 0.3, col, 0.3);
        draw_taxon(
            sprites,
            game,
            taxon,
            best_morph(game, taxon),
            vec2(cx, sy),
            r.w * 0.34,
            WHITE,
        );
    } else {
        draw_taxon(
            sprites,
            game,
            taxon,
            Morph::None,
            vec2(cx, sy),
            r.w * 0.34,
            if game.is_fossil(taxon) {
                STONE
            } else {
                SILHOUETTE
            },
        );
    }
    let px = r.w * 0.045;
    let name = if found || game.is_fossil(taxon) {
        t.name
    } else {
        "? ? ?"
    };
    text_centered(
        name,
        cx,
        r.y + r.h * 0.36,
        fit_px(name, r.w * 0.9, px * 1.8),
        TEXT,
    );
    let sub = if game.is_fossil(taxon) {
        format!("FOSSIL  ·  {}  ·  {}", t.clade, eco.tier.name())
    } else {
        format!("{}  ·  {}  ·  {}", t.clade, t.age_label(), eco.tier.name())
    };
    text_centered(
        &sub,
        cx,
        r.y + r.h * 0.415,
        fit_px(&sub, r.w * 0.9, px),
        if game.is_fossil(taxon) {
            STONE_TEXT
        } else {
            col
        },
    );

    let x = r.x + r.w * 0.07;
    let mut y = r.y + r.h * 0.50;
    text("NEEDS  (your planet right now)", x, y, px, ACCENT_WARN);
    y += px * 1.6;
    let p = &game.planet;
    let n = eco.needs;
    let mut rows: Vec<(String, bool)> = Vec::new();
    if let Some(b) = n.biome {
        rows.push((format!("Biome: {}", b.name()), b.ranges().contains(p)));
    }
    let hab: Vec<&str> = n.habitats.iter().map(|h| h.name()).collect();
    rows.push((hab.join(" or "), n.habitats.iter().any(|&h| p.has(h))));
    for lever in Lever::ALL {
        let (lo, hi) = n.ranges.get(lever);
        let in_biome = n.biome.is_some_and(|b| b.ranges().get(lever) == (lo, hi));
        if in_biome || (lo, hi) == planet::Ranges::ANY.get(lever) {
            continue;
        }
        rows.push((
            range_label(lever, lo, hi),
            (lo..=hi).contains(&p.get(lever)),
        ));
    }
    for (label, ok) in rows {
        text(&label, x, y, px, TEXT);
        let v = if ok { "YES" } else { "NO" };
        text(
            v,
            r.x + r.w * 0.93 - text_width(v, px),
            y,
            px,
            if ok { ACCENT_OK } else { rgb(0xFF5A66) },
        );
        y += px * 1.5;
    }
    y += px * 0.6;
    text("AS A KEYSTONE", x, y, px, ACCENT_WARN);
    for line in wrap_lines(&eco.describe(), r.w * 0.86, px) {
        y += px * 1.4;
        text(&line, x, y, px, LIME);
    }
    if let Rule::Catch(c) = eco.rule {
        y += px * 1.4;
        let catch = format!("But: {}", c.drawback());
        text(&catch, x, y, fit_px(&catch, r.w * 0.86, px), rgb(0xFF6A4A));
    }
    if found {
        y += px * 1.6;
        let (have, step) = game.level_progress(taxon);
        text(&format!("Level {}", game.level[taxon]), x, y, px, TEXT);
        if step > 0 {
            draw_meter(
                x + r.w * 0.3,
                y - px * 0.6,
                r.w * 0.4,
                px * 0.5,
                have as f32 / step as f32,
                LIME,
            );
            text(&format!("{have}/{step}"), r.x + r.w * 0.78, y, px, TEXT_DIM);
        } else {
            text("MAX", r.x + r.w * 0.78, y, px, LIME);
        }
        y += px * 1.8;
        let mw = r.w * 0.2;
        for (k, &m) in game::MORPHS.iter().enumerate() {
            let mr = Rect::new(x + k as f32 * (mw + r.w * 0.02), y, mw, mw);
            let owned = game.morphs[taxon] & game::morph_bit(m) != 0;
            let worn = owned && game.edition(taxon) == m;
            frame(
                mr,
                rgb(0x050A14),
                if worn {
                    LIME
                } else if owned {
                    rgb(0x6FF5E1)
                } else {
                    rgb(0x141C26)
                },
                if worn { 2.0 } else { 1.0 },
            );
            draw_taxon(
                sprites,
                game,
                taxon,
                m,
                vec2(mr.x + mw * 0.5, mr.y + mw * 0.45),
                mw * 0.5,
                if owned { WHITE } else { SILHOUETTE },
            );
            text_centered(
                m.name(),
                mr.x + mw * 0.5,
                mr.y + mw * 0.93,
                fit_px(m.name(), mw * 0.9, px * 0.7),
                if owned { TEXT } else { LOCKED_TEXT },
            );
            for (i, line) in wrap_lines(m.effect(), mw, px * 0.6)
                .iter()
                .take(2)
                .enumerate()
            {
                text_centered(
                    line,
                    mr.x + mw * 0.5,
                    mr.y + mw + px * (0.9 + 0.75 * i as f32),
                    px * 0.6,
                    if owned { TEXT_DIM } else { LOCKED_TEXT },
                );
            }
        }
        y += mw + px * 2.9;
        if let Some(ma) = game.found_ma[taxon] {
            text(&format!("Evolved after {ma} Ma"), x, y, px * 0.9, TEXT_DIM);
        }
    }
    text_centered(
        "tap to close",
        cx,
        r.y + r.h - px * 0.8,
        px * 0.8,
        faded(TEXT_DIM, 0.7),
    );
}

/// "Oxygen 21%+", "Cold to Temperate", "Land 4+".
fn range_label(lever: Lever, lo: u8, hi: u8) -> String {
    let top = planet::Ranges::ANY.get(lever).1;
    let fmt = |v: u8| match lever {
        Lever::Oxygen => format!("{}%", planet::oxygen_percent(v)),
        Lever::Temperature => planet::temperature_label(v).to_string(),
        _ => v.to_string(),
    };
    let name = match lever {
        Lever::Temperature => "",
        l => l.name(),
    };
    let span = if hi >= top {
        format!("{}+", fmt(lo))
    } else if lo == 0 && lever != Lever::Temperature {
        format!("up to {}", fmt(hi))
    } else if lo == hi {
        fmt(lo)
    } else {
        format!("{} to {}", fmt(lo), fmt(hi))
    };
    format!("{name} {span}").trim().to_string()
}

// --- the biome guide (from the map) ---------------------------------------------------

/// Under the top bar, at the right: opens the biome guide.
pub fn biomes_button_rect() -> Rect {
    let u = u();
    let (w, h) = (u * 40.0, u * 16.0);
    Rect::new(
        screen_width() - w - u * 4.0,
        render::bar_height() + u * 4.0,
        w,
        h,
    )
}

pub fn draw_biomes_button() {
    let b = biomes_button_rect();
    frame(b, faded(PANEL, 0.92), rgb(0x6FF5E1), 1.5);
    text_centered(
        "BIOMES",
        b.x + b.w * 0.5,
        b.y + b.h * 0.68,
        b.h * 0.6,
        rgb(0x6FF5E1),
    );
}

/// A biome's lever ranges in a line ("Land 4+  ·  Vegetation 1 to 2").
fn biome_recipe(b: Biome) -> String {
    let r = b.ranges();
    Lever::ALL
        .iter()
        .filter(|&&l| r.get(l) != planet::Ranges::ANY.get(l))
        .map(|&l| {
            let (lo, hi) = r.get(l);
            let label = range_label(l, lo, hi);
            // Temperature labels carry no lever name.
            if l == Lever::Temperature {
                format!("Temp {label}")
            } else {
                label
            }
        })
        .collect::<Vec<_>>()
        .join("  ·  ")
}

/// Every biome: its levers, whether the planet is it (or why not), and its
/// animals, found ones in colour.
pub fn draw_biomes(game: &Game, sprites: &Sprites) {
    let (sw, sh, u) = (screen_width(), screen_height(), u());
    draw_rectangle(0.0, 0.0, sw, sh, faded(render::BG, 0.9));
    let r = Rect::new(sw * 0.04, sh * 0.07, sw * 0.92, sh * 0.86);
    frame(r, rgb(0x070D1C), rgb(0x6FF5E1), 2.0);
    let px = u * 7.0;
    let x = r.x + u * 5.0;
    let w = r.w - u * 10.0;
    text("BIOMES", x, r.y + u * 11.0, px * 1.3, ACCENT_WARN);
    let note = "Each rules the others out. Animals of a biome live only there.";
    text(
        note,
        x,
        r.y + u * 18.0,
        fit_px(note, w, px * 0.75),
        TEXT_DIM,
    );

    let top = r.y + u * 24.0;
    let row_h = (r.y + r.h - u * 10.0 - top) / Biome::ALL.len() as f32;
    let p = &game.planet;
    for (i, b) in Biome::ALL.into_iter().enumerate() {
        let y = top + i as f32 * row_h;
        let col = rgb(b.color());
        if i > 0 {
            draw_line(x, y, x + w, y, 1.0, EDGE);
        }
        let name_y = y + row_h * 0.27;
        text(&b.name().to_uppercase(), x, name_y, px, col);
        let members: Vec<usize> = (0..game.phy.len())
            .filter(|&t| ecology::of(game.taxon(t).name).needs.biome == Some(b))
            .collect();
        let found = members.iter().filter(|&&t| game.unlocked[t]).count();
        let count = format!("{found}/{}", members.len());
        text(
            &count,
            x + w - text_width(&count, px * 0.85),
            name_y,
            px * 0.85,
            TEXT_DIM,
        );
        let recipe = biome_recipe(b);
        text(
            &recipe,
            x,
            y + row_h * 0.48,
            fit_px(&recipe, w, px * 0.75),
            TEXT,
        );
        let (status, status_col) = match b.ranges().missing(p, 0) {
            None => ("Your planet is this biome".to_string(), ACCENT_OK),
            Some(why) => (format!("Your planet: {why}"), LOCKED_TEXT),
        };
        text(
            &status,
            x,
            y + row_h * 0.68,
            fit_px(&status, w, px * 0.7),
            status_col,
        );
        // The animals, found ones in colour, the rest as silhouettes.
        let icon = (row_h * 0.26).min(u * 10.0);
        let iy = y + row_h * 0.84;
        for (k, &t) in members.iter().enumerate() {
            let cx = x + icon * 0.5 + k as f32 * icon * 1.1;
            if cx > x + w {
                break;
            }
            let seen = game.unlocked[t];
            draw_taxon(
                sprites,
                game,
                t,
                if seen {
                    best_morph(game, t)
                } else {
                    Morph::None
                },
                vec2(cx, iy),
                icon,
                if seen { WHITE } else { SILHOUETTE },
            );
        }
    }
    text_centered(
        "tap to close",
        r.x + r.w * 0.5,
        r.y + r.h - u * 3.5,
        px * 0.75,
        faded(TEXT_DIM, 0.7),
    );
}

// --- the boon pick ----------------------------------------------------------------------

pub fn boon_rect(i: usize) -> Rect {
    let (sw, sh) = (screen_width(), screen_height());
    let (w, h) = (sw * 0.84, sh * 0.15);
    Rect::new((sw - w) * 0.5, sh * 0.26 + i as f32 * (h + sh * 0.03), w, h)
}

fn boon_color(b: Boon) -> Color {
    match b {
        Boon::Lure(_) => rgb(0x5AA8FF),
        Boon::Lens => LIME,
        Boon::Catalyst => rgb(0xC07BFF),
        Boon::Charm => PINK,
        Boon::Tailwind => rgb(0x6FF5E1),
        Boon::Tectonics => rgb(0xB89A6A),
    }
}

pub fn draw_boons(game: &Game, sprites: &Sprites, assets: &Assets, hover: Option<usize>) {
    let (sw, sh) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, faded(render::BG, 0.9));
    let t = get_time() as f32;
    text_centered("PICK A BOON", sw * 0.5, sh * 0.16, sh * 0.04, ACCENT_WARN);
    text_centered(
        "it lasts for the next cycle only",
        sw * 0.5,
        sh * 0.2,
        sh * 0.02,
        TEXT_DIM,
    );
    let Some(offer) = &game.boon_offer else {
        return;
    };
    for (i, &b) in offer.iter().enumerate() {
        let r = boon_rect(i);
        let col = boon_color(b);
        let lift = if hover == Some(i) {
            -u() * 2.0
        } else {
            (t * 2.0 + i as f32).sin() * u() * 0.6
        };
        let r = Rect::new(r.x, r.y + lift, r.w, r.h);
        assets.glow(vec2(r.x + r.h * 0.5, r.y + r.h * 0.5), r.h * 0.7, col, 0.3);
        frame(r, rgb(0x0A1020), col, 2.0);
        let ic = vec2(r.x + r.h * 0.5, r.y + r.h * 0.5);
        let bob = sprites::Pose {
            y_off: (t * 2.0 + i as f32).sin() * 0.03,
            ..Default::default()
        };
        sprites::draw_sprite(sprites.boon(b.icon()), ic, r.h * 0.72, bob, WHITE);
        if let Boon::Lure(taxon) = b {
            let at = ic + vec2(r.h * 0.26, r.h * 0.24);
            draw_taxon(sprites, game, taxon, Morph::None, at, r.h * 0.4, WHITE);
        }
        text(
            b.title(),
            r.x + r.h * 1.05,
            r.y + r.h * 0.42,
            r.h * 0.24,
            TEXT,
        );
        let d = b.describe(&game.phy);
        text(
            &d,
            r.x + r.h * 1.05,
            r.y + r.h * 0.74,
            fit_px(&d, r.w - r.h * 1.2, r.h * 0.17),
            col,
        );
    }
}

// --- settings ---------------------------------------------------------------------------

/// What the settings screen shows besides the settings themselves.
pub struct SettingsView {
    pub notify: bool,
    /// Notifications can be posted here (the Android app).
    pub supported: bool,
    /// Android lets the app post them.
    pub allowed: bool,
}

/// The back arrow, at the left end of the top bar.
pub fn settings_back_rect() -> Rect {
    let g = gear_rect();
    Rect::new(screen_width() - g.x - g.w, g.y, g.w, g.h)
}

fn settings_panel_rect() -> Rect {
    let (sw, u) = (screen_width(), u());
    let y = render::bar_height() + u * 8.0;
    Rect::new(sw * 0.04, y, sw * 0.92, u * 63.0)
}

/// The notification row: the checkbox and its labels, all one tap target.
pub fn notify_row_rect() -> Rect {
    let (p, u) = (settings_panel_rect(), u());
    Rect::new(p.x, p.y + u * 16.0, p.w, u * 28.0)
}

/// The line under the row: what the setting will do, or why it can't.
pub fn notify_status_rect() -> Rect {
    let (p, u) = (settings_panel_rect(), u());
    let row = notify_row_rect();
    Rect::new(
        p.x,
        row.y + row.h + u * 2.0,
        p.w,
        p.y + p.h - row.y - row.h - u * 2.0,
    )
}

/// Blocked by Android: the status line opens the system settings.
pub fn notify_blocked(v: &SettingsView) -> bool {
    v.supported && v.notify && !v.allowed
}

pub fn draw_settings(view: &SettingsView, assets: &Assets) {
    clear_background(render::BG);
    let (sw, sh, u) = (screen_width(), screen_height(), u());
    let bar_h = render::bar_height();

    draw_top_bar();
    draw_gear(assets, true);
    // A chunky pixel chevron for "back".
    let b = settings_back_rect();
    let c = vec2(b.x + b.w * 0.5, b.y + b.h * 0.5);
    let (arm, w) = (b.h * 0.2, (u * 1.5).max(2.0));
    draw_line(
        c.x + arm * 0.5,
        c.y - arm,
        c.x - arm * 0.5,
        c.y,
        w,
        TEXT_DIM,
    );
    draw_line(
        c.x - arm * 0.5,
        c.y,
        c.x + arm * 0.5,
        c.y + arm,
        w,
        TEXT_DIM,
    );
    text_centered("SETTINGS", sw * 0.5, bar_h * 0.6, bar_h * 0.36, TEXT);

    let p = settings_panel_rect();
    frame(p, PANEL, EDGE, 1.0);
    let x = p.x + u * 6.0;
    let px = u * 7.5;
    text("NOTIFICATIONS", x, p.y + u * 10.0, px, ACCENT_WARN);

    // The checkbox: a pixel tick on green when on.
    let row = notify_row_rect();
    let s = u * 11.0;
    let bx = Rect::new(x, row.y + u * 2.0, s, s);
    let (edge, fill) = match (view.supported, view.notify) {
        (false, _) => (EDGE, PANEL_LOCKED),
        (true, true) => (ACCENT_OK, ACCENT_OK),
        (true, false) => (rgb(0x3E4857), PANEL_LOCKED),
    };
    frame(bx, fill, edge, (u * 0.9).max(2.0));
    if view.notify {
        let d = s / 11.0;
        for (cx, cy) in [(2, 5), (3, 6), (4, 7), (5, 6), (6, 5), (7, 4), (8, 3)] {
            draw_rectangle(
                bx.x + cx as f32 * d,
                bx.y + cy as f32 * d,
                d * 2.0,
                d * 2.0,
                render::BG,
            );
        }
    }

    let tx = bx.x + s + u * 6.0;
    let label_c = if view.supported { TEXT } else { LOCKED_TEXT };
    text("Genome ready", tx, row.y + u * 9.0, px, label_c);
    let sub = "Send a notification when a new genome is ready to evolve.";
    let sub_px = px * 0.8;
    for (i, line) in wrap_lines(sub, p.x + p.w - u * 5.0 - tx, sub_px)
        .iter()
        .enumerate()
    {
        text(
            line,
            tx,
            row.y + u * 16.0 + i as f32 * u * 6.5,
            sub_px,
            TEXT_DIM,
        );
    }

    let st = notify_status_rect();
    // A dashed rule between the row and its status.
    let mut dx = x;
    while dx < p.x + p.w - u * 6.0 {
        draw_rectangle(dx, st.y, u * 2.0, 1.0, EDGE);
        dx += u * 4.0;
    }
    let (status, col) = if !view.supported {
        ("Only in the Android app", LOCKED_TEXT)
    } else if notify_blocked(view) {
        ("Blocked by Android: tap here to allow", ACCENT_WARN)
    } else if view.notify {
        ("ON  ·  one per wait, when time has run", ACCENT_OK)
    } else {
        ("OFF  ·  you'll see it when you open the game", LOCKED_TEXT)
    };
    text(
        status,
        x,
        st.y + st.h * 0.62,
        fit_px(status, p.w - u * 12.0, px * 0.8),
        col,
    );

    // The build, in the footer: `make apk` sets ASCENDIO_VERSION.
    let foot = sh - bar_h;
    draw_line(0.0, foot, sw, foot, 1.0, EDGE);
    text_centered(
        "ASCENDIO",
        sw * 0.5,
        foot + bar_h * 0.42,
        bar_h * 0.26,
        TEXT_DIM,
    );
    text_centered(
        option_env!("ASCENDIO_VERSION").unwrap_or("dev"),
        sw * 0.5,
        foot + bar_h * 0.72,
        bar_h * 0.22,
        LOCKED_TEXT,
    );
}
