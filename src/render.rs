//! Shared drawing helpers, the spiral and the map.

use macroquad::prelude::*;

use crate::ecology;
use crate::game::Game;
use crate::layout::Layout;
use crate::spiral::{self, Bead, Frame};
use crate::sprites::{self, Sprites};
use crate::view::Camera;

/// Glyphs are rasterised once at this size and scaled: one atlas entry.
const FONT_PX: u16 = 48;
/// Taxon name size in the map view, in world units.
pub const LABEL_PX: f32 = 20.0;
const SUBLABEL_PX: f32 = 13.0;

pub const BG: Color = rgb(0x0E1116);
pub const PANEL: Color = rgb(0x161C24);
pub const PANEL_LOCKED: Color = rgb(0x11151B);
pub const EDGE: Color = rgb(0x2A323D);
pub const TEXT: Color = rgb(0xE6EAF0);
pub const TEXT_DIM: Color = rgb(0x97A3B5);
const LOCKED_BORDER: Color = rgb(0x3E4857);
pub const LOCKED_TEXT: Color = rgb(0x5A6678);
pub const ACCENT_OK: Color = rgb(0x5CC26B);
pub const ACCENT_DNA: Color = rgb(0x5BC8F5);
pub const ACCENT_WARN: Color = rgb(0xE8A33D);
pub const TRACK: Color = rgb(0x20272F);
/// Multiply tint for an undiscovered species' sprite.
pub const SILHOUETTE: Color = Color::new(0.09, 0.09, 0.11, 1.0);
pub const HUD_BG: Color = Color::new(0.055, 0.067, 0.086, 0.92);

pub const fn rgb(hex: u32) -> Color {
    Color::new(
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
        1.0,
    )
}

pub fn faded(c: Color, a: f32) -> Color {
    Color {
        a: c.a * a.clamp(0.0, 1.0),
        ..c
    }
}

// --- text -------------------------------------------------------------------

pub fn text_width(s: &str, px: f32) -> f32 {
    measure_text(s, None, FONT_PX, px / FONT_PX as f32).width
}

pub fn text(s: &str, x: f32, baseline_y: f32, px: f32, color: Color) {
    draw_text_ex(
        s,
        x,
        baseline_y,
        TextParams {
            font: None,
            font_size: FONT_PX,
            font_scale: px / FONT_PX as f32,
            color,
            ..Default::default()
        },
    );
}

pub fn text_centered(s: &str, cx: f32, baseline_y: f32, px: f32, color: Color) {
    text(s, cx - text_width(s, px) * 0.5, baseline_y, px, color);
}

// --- spiral view ------------------------------------------------------------

pub fn draw_spiral(game: &Game, frame: &Frame, t: f32, last: f32, sprites: &Sprites) {
    draw_coil(frame, t, last);
    draw_spurs(game, frame, sprites);

    // Far to near, so the focus bead lands on top.
    let mut beads: Vec<&Bead> = frame.beads.iter().collect();
    beads.sort_by(|a, b| b.d.abs().partial_cmp(&a.d.abs()).unwrap());
    for bead in beads {
        draw_bead(game, frame, bead, sprites);
    }
}

fn draw_coil(frame: &Frame, t: f32, last: f32) {
    for pair in spiral::curve(frame.center, frame.k, t, last).windows(2) {
        let (a, da) = pair[0];
        let (b, _) = pair[1];
        let w = (frame.k * 0.022 * spiral::scale_at(da)).max(1.0);
        draw_line(a.x, a.y, b.x, b.y, w, faded(EDGE, spiral::fade_at(da)));
    }
}

/// A taxon's tier colour once discovered, the locked border before.
fn accent_of(game: &Game, taxon: usize) -> Color {
    if !game.unlocked[taxon] {
        return LOCKED_BORDER;
    }
    tier_color(ecology::of(game.taxon(taxon).name).tier)
}

pub fn tier_color(tier: ecology::Tier) -> Color {
    rgb(tier.color())
}

/// A label box on the coil.
struct Chip<'a> {
    pos: Vec2,
    half: Vec2,
    label: &'a str,
    px: f32,
    accent: Color,
    unlocked: bool,
    fade: f32,
    border: f32,
    taxon: usize,
}

/// Icon diameter over box height; must match `spiral::Frame::build`'s `* 1.3`.
const CHIP_ICON: f32 = 0.72;

pub fn draw_meter(x: f32, y: f32, w: f32, h: f32, fraction: f32, fill: Color) {
    draw_rectangle(x, y, w, h, TRACK);
    let f = fraction.clamp(0.0, 1.0);
    if f > 0.0 {
        draw_rectangle(x, y, w * f, h, fill);
    }
}

fn draw_chip(game: &Game, sprites: &Sprites, c: Chip) {
    let (x, y) = (c.pos.x - c.half.x, c.pos.y - c.half.y);
    let (w, h) = (c.half.x * 2.0, c.half.y * 2.0);
    draw_rectangle(
        x,
        y,
        w,
        h,
        faded(if c.unlocked { PANEL } else { PANEL_LOCKED }, c.fade),
    );
    draw_rectangle_lines(x, y, w, h, c.border, faded(c.accent, c.fade));

    let icon = h * CHIP_ICON;
    let icon_center = Vec2::new(x + h * 0.14 + icon * 0.5, c.pos.y);
    let group = game.taxon(c.taxon).group;
    let anim = sprites::pose(group, c.taxon, get_time());
    let tint = faded(if c.unlocked { WHITE } else { SILHOUETTE }, c.fade);
    sprites::draw_sprite(sprites.get(c.taxon), icon_center, icon, anim, tint);

    let text_x0 = x + h * 0.14 + icon + h * 0.10;
    let text_w = (x + w - h * 0.12 - text_x0).max(1.0);
    text(
        c.label,
        text_x0,
        c.pos.y + c.px * 0.34,
        fit_px(c.label, text_w, c.px),
        faded(if c.unlocked { TEXT } else { LOCKED_TEXT }, c.fade),
    );
}

fn draw_spurs(game: &Game, frame: &Frame, sprites: &Sprites) {
    let px = spiral::label_px(frame.k);

    for spur in &frame.spurs {
        let fade = spiral::fade_at(spur.d);
        let accent = accent_of(game, spur.taxon);
        let w = (frame.k * 0.014 * spiral::scale_at(spur.d)).max(1.0);
        draw_line(
            spur.from.x,
            spur.from.y,
            spur.pos.x,
            spur.pos.y,
            w,
            faded(accent, fade * 0.5),
        );

        match &spur.label {
            Some(label) => draw_chip(
                game,
                sprites,
                Chip {
                    pos: spur.pos,
                    half: spur.half,
                    label,
                    px: px * spiral::scale_at(spur.d) * 0.92,
                    accent,
                    unlocked: game.unlocked[spur.taxon],
                    fade,
                    border: 1.5,
                    taxon: spur.taxon,
                },
            ),
            None => {
                let r = spiral::bead_radius(frame.k, spiral::scale_at(spur.d) * 0.7);
                draw_circle(spur.pos.x, spur.pos.y, r, faded(accent, fade * 0.7));
            }
        }
    }
}

fn draw_bead(game: &Game, frame: &Frame, bead: &Bead, sprites: &Sprites) {
    let fade = spiral::fade_at(bead.d);
    let accent = accent_of(game, bead.taxon);
    let is_focus = bead.d.abs() < 0.5;

    let Some(label) = &bead.label else {
        // Far from the focus: a tiny icon, or a plain dot so locked shapes stay secret.
        if game.unlocked[bead.taxon] {
            let group = game.taxon(bead.taxon).group;
            let size = spiral::bead_radius(frame.k, bead.scale) * 3.4;
            let anim = sprites::pose(group, bead.taxon, get_time());
            sprites::draw_sprite(
                sprites.get(bead.taxon),
                bead.pos,
                size,
                anim,
                faded(WHITE, fade),
            );
        } else {
            let r = spiral::bead_radius(frame.k, bead.scale);
            draw_circle(bead.pos.x, bead.pos.y, r, faded(accent, fade * 0.85));
        }
        return;
    };

    draw_chip(
        game,
        sprites,
        Chip {
            pos: bead.pos,
            half: bead.half,
            label,
            px: spiral::label_px(frame.k) * bead.scale,
            accent,
            unlocked: game.unlocked[bead.taxon],
            fade,
            border: if is_focus { 3.0 } else { 1.5 },
            taxon: bead.taxon,
        },
    );
}

/// Shrinks `px` until `s` fits in `max_w`.
pub fn fit_px(s: &str, max_w: f32, px: f32) -> f32 {
    let w = text_width(s, px);
    if w <= max_w {
        px
    } else {
        px * max_w / w
    }
}

// --- map view ---------------------------------------------------------------

pub fn draw_map(game: &Game, layout: &Layout, cam: &Camera, sprites: &Sprites) {
    clear_background(BG);
    draw_edges(game, layout, cam);
    draw_nodes(game, layout, cam, sprites);
}

fn draw_edges(game: &Game, layout: &Layout, cam: &Camera) {
    let thickness = (1.5 * cam.zoom).max(1.0);

    for i in 0..game.phy.len() {
        let (Some(child_box), Some(parent)) = (layout.get(i), game.taxon(i).parent) else {
            continue;
        };
        let Some(parent_box) = layout.get(parent) else {
            continue;
        };

        let from = cam.world_to_screen(parent_box.right());
        let to = cam.world_to_screen(child_box.left());
        let mid_x = (from.x + to.x) * 0.5;

        draw_line(from.x, from.y, mid_x, from.y, thickness, EDGE);
        draw_line(mid_x, from.y, mid_x, to.y, thickness, EDGE);
        draw_line(mid_x, to.y, to.x, to.y, thickness, EDGE);
    }
}

fn draw_nodes(game: &Game, layout: &Layout, cam: &Camera, sprites: &Sprites) {
    let (sw, sh) = (screen_width(), screen_height());
    for i in 0..game.phy.len() {
        let Some(b) = layout.get(i) else { continue };
        let center = cam.world_to_screen(b.center);
        let (w, h) = (b.w * cam.zoom, b.h * cam.zoom);
        let (x, y) = (center.x - w * 0.5, center.y - h * 0.5);
        if x + w < 0.0 || x > sw || y + h < 0.0 || y > sh {
            continue;
        }
        let found = game.unlocked[i];
        let accent = accent_of(game, i);
        if found {
            draw_rectangle(x, y, w, h, PANEL);
            draw_rectangle_lines(x, y, w, h, 1.5 * cam.zoom.max(0.5), accent);
            draw_rectangle(x, y, 3.0 * cam.zoom, h, accent);
        } else {
            draw_rectangle(x, y, w, h, PANEL_LOCKED);
            draw_rectangle_lines(x, y, w, h, cam.zoom.max(0.5), accent);
        }

        let icon = h * 0.82;
        let icon_center = Vec2::new(x + h * 0.20 + icon * 0.5, center.y);
        let anim = sprites::pose(game.taxon(i).group, i, get_time());
        let tint = if found { WHITE } else { SILHOUETTE };
        sprites::draw_sprite(sprites.get(i), icon_center, icon, anim, tint);

        let text_x0 = x + h * 0.20 + icon + h * 0.12;
        let text_w = (x + w - h * 0.15 - text_x0).max(1.0);
        let name_px = LABEL_PX * cam.zoom;
        if found {
            let name = game.taxon(i).name;
            text(
                name,
                text_x0,
                center.y + name_px * 0.05,
                fit_px(name, text_w, name_px),
                TEXT,
            );
            let sub = format!(
                "Lv {}  ·  {}",
                game.level[i].max(1),
                game.taxon(i).age_label()
            );
            let sub_px = SUBLABEL_PX * cam.zoom;
            text(
                &sub,
                text_x0,
                center.y + name_px * 0.4 + sub_px,
                fit_px(&sub, text_w, sub_px),
                TEXT_DIM,
            );
        } else {
            text_centered(
                "? ? ?",
                text_x0 + text_w * 0.5,
                center.y + name_px * 0.35,
                fit_px("? ? ?", text_w, name_px),
                LOCKED_TEXT,
            );
        }
    }
}

// --- shared chrome ----------------------------------------------------------

pub fn bar_height() -> f32 {
    (screen_height() * 0.085).clamp(56.0, 92.0)
}

/// Word-wraps `s` into lines that fit `max_w` at `px`.
pub fn wrap_lines(s: &str, max_w: f32, px: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in s.split(' ') {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if text_width(&candidate, px) > max_w && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
