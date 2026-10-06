//! Top-bar buttons and the radiation badge.

use super::*;

pub fn gear_rect() -> Rect {
    let bar_h = render::bar_height();
    let s = bar_h * 0.62;
    Rect::new(view::width() - s - bar_h * 0.19, (bar_h - s) * 0.5, s, s)
}

pub(super) fn draw_gear(assets: &Assets, lit: bool) {
    draw_icon_button(gear_rect(), &assets.gear, lit, ACCENT_DNA);
}

pub fn trophy_rect() -> Rect {
    let g = gear_rect();
    Rect::new(g.x - g.w - render::u() * 2.0, g.y, g.w, g.h)
}

pub(super) fn draw_trophy(assets: &Assets, lit: bool) {
    draw_icon_button(trophy_rect(), &assets.trophy, lit, GOLD);
}

pub(super) fn draw_icon_button(r: Rect, icon: &[Texture2D; 2], lit: bool, accent: Color) {
    if lit {
        frame(r, faded(accent, 0.18), accent, 1.0);
    } else {
        frame(r, PANEL, EDGE, 1.0);
    }
    // A whole number of texels per pixel keeps the pixel art crisp.
    let s = ((r.w * 0.72 / 16.0).floor().max(1.0) * 16.0).round();
    draw_texture_ex(
        &icon[lit as usize],
        (r.x + (r.w - s) * 0.5).round(),
        (r.y + (r.h - s) * 0.5).round(),
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(s, s)),
            ..Default::default()
        },
    );
}

pub(super) fn draw_top_bar() {
    let (sw, bar_h) = (view::width(), render::bar_height());
    draw_rectangle(0.0, 0.0, sw, bar_h, HUD_BG);
    draw_line(0.0, bar_h, sw, bar_h, 1.0, EDGE);
}

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

pub fn rad_badge_rect(game: &Game) -> Option<Rect> {
    if game.rad == 0 {
        return None;
    }
    let g = trophy_rect();
    let h = g.h * 0.72;
    let w = h * 2.6;
    Some(Rect::new(
        g.x - w - render::u() * 3.0,
        g.y + (g.h - h) * 0.5,
        w,
        h,
    ))
}

pub(super) fn draw_rad_badge(game: &Game) {
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

pub fn draw_rad_info(game: &Game) {
    let Some(b) = rad_badge_rect(game) else {
        return;
    };
    let u = render::u();
    let w = view::width() * 0.62;
    let r = Rect::new(
        view::width() - w - u * 4.0,
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
    draw_trophy(assets, false);
    draw_rad_badge(game);
    let right = rad_badge_rect(game).map_or(trophy_rect().x, |b| b.x);
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
            game.taxon(best).label()
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
