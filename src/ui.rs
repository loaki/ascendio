//! Screen chrome and the overlay screens: the HUD, the lever panel, the
//! bottom bar, keystones, an animal's page and the boon pick. Every tappable
//! rect comes from a function here, so drawing and hit-testing agree.

use macroquad::prelude::*;

use crate::dial;
use crate::ecology::{self, Bonus};
use crate::game::{self, Boon, Game, Phase};
use crate::genome;
use crate::genome::Morph;
use crate::planet::{self, Lever};
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

/// Textures the UI draws with, built once (they need a GL context).
pub struct Assets {
    pub glow: Texture2D,
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

        Self { glow }
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

pub fn draw_hud(game: &Game, now: f64) {
    let sw = screen_width();
    let bar_h = render::bar_height();
    let px = bar_h * 0.34;
    draw_rectangle(0.0, 0.0, sw, bar_h, HUD_BG);
    draw_line(0.0, bar_h, sw, bar_h, 1.0, EDGE);
    // The build (`make apk` sets ASCENDIO_VERSION) and the screen the game
    // actually got, so a phone screenshot says what it's running.
    let build = format!(
        "{}  {}x{}",
        option_env!("ASCENDIO_VERSION").unwrap_or("dev"),
        sw as u32,
        screen_height() as u32
    );
    let bpx = px * 0.5;
    text(
        &build,
        sw - px * 0.7 - text_width(&build, bpx),
        bar_h * 0.46,
        bpx,
        TEXT_DIM,
    );
    text(
        &format!("{} Ma  ·  {}", game.ma_elapsed(now), game.era()),
        px * 0.7,
        bar_h * 0.46,
        px,
        TEXT,
    );
    let best = game.most_advanced();
    let sub = format!(
        "{} / {} found  ·  step {}  {}",
        game.discovered(),
        game.phy.len(),
        game.taxon(best).depth,
        game.taxon(best).name
    );
    text(
        &sub,
        px * 0.7,
        bar_h * 0.84,
        fit_px(&sub, sw * 0.9, px * 0.72),
        TEXT_DIM,
    );
}

// --- the lever panel ------------------------------------------------------------

/// The lever panel, just above the bottom bar.
pub fn panel_rect() -> Rect {
    let (sw, sh) = (screen_width(), screen_height());
    let (w, h) = (sw * 0.84, sh * 0.275);
    Rect::new((sw - w) * 0.5, sh - render::bar_height() - h - 10.0, w, h)
}

fn lever_row(i: usize) -> Rect {
    let r = panel_rect();
    let head = r.h * 0.15;
    let foot = r.h * 0.14;
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
    (
        format!("{n} undiscovered species could evolve here"),
        rgb(0x6FF5E1),
    )
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
        let e = keystone_effect(game, t);
        let c = if dormant { ACCENT_WARN } else { LIME };
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
                    let ok = game.can_step(lever, if plus { 1 } else { -1 });
                    frame(
                        b,
                        if ok { rgb(0x0A1422) } else { rgb(0x070B12) },
                        if ok { rgb(0x2F6B72) } else { rgb(0x141C26) },
                        1.5,
                    );
                    text_centered(
                        label,
                        b.x + b.w * 0.5,
                        b.y + b.h * 0.72,
                        b.h * 0.7,
                        if ok { TEXT } else { LOCKED_TEXT },
                    );
                }
            }
            let (hint, col) = shape_hint(game);
            text(
                &hint,
                r.x + r.w * 0.04,
                r.y + r.h * 0.95,
                fit_px(&hint, r.w * 0.92, px * 0.95),
                col,
            );
        }
    }
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
fn keystone_effect(game: &Game, taxon: usize) -> String {
    if game.dormant_reason(taxon).is_some() {
        return "dormant".into();
    }
    let s = game.keystone_strength(taxon);
    let pct = |per: f32| format!("{:.0}%", per * s * 100.0);
    match ecology::of(game.taxon(taxon).name).bonus {
        Bonus::Affinity(h) => format!(
            "x{:.1} {}",
            1.0 + game::AFFINITY_PER_UNIT * s,
            h.name().to_lowercase()
        ),
        Bonus::Luck | Bonus::LivingFossil => format!("+{s:.1} luck"),
        Bonus::ExtraCard => format!("+{} card", pct(game::EXTRA_CARD_PER_UNIT)),
        Bonus::Morph | Bonus::Oddity => format!("+{} morph", pct(game::MORPH_PER_UNIT)),
        Bonus::Quick => format!("-{} wait", pct(game::QUICK_PER_UNIT)),
        Bonus::Point => "+1 point".into(),
        Bonus::Soil => "+1 vegetation".into(),
        Bonus::DoubleSpecimens => "x2 duplicates".into(),
        Bonus::Mind => format!("+1 pt, +{s:.1} luck"),
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
    for k in 0..genome::MAX_CARDS {
        let cx = x + k as f32 * (cw + u * 1.5);
        let (fill, edge) = if k < sure {
            (rgb(0x0A1020), col)
        } else if k == sure && chance > 0.0 {
            (rgb(0x05090F), rgb(0x5A6678))
        } else {
            (rgb(0x05090F), rgb(0x141C26))
        };
        frame(Rect::new(cx, y, cw, ch), fill, edge, 1.5);
    }
    let tx = x + genome::MAX_CARDS as f32 * (cw + u * 1.5) + u * 2.0;
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
                genome::morph_chance(f.odds.morph_mult) * 100.0
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
        let e = keystone_effect(game, t);
        let c = if dormant { ACCENT_WARN } else { LIME };
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
                format!("{} Ma  ·  {} cards", f.ma, f.odds.cards),
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
    Rect::new(r.x + r.w * 0.64, r.y + r.h * 0.46, r.w * 0.32, r.h * 0.26)
}

fn collection(game: &Game) -> Vec<usize> {
    (0..game.phy.len()).filter(|&i| game.unlocked[i]).collect()
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

    for i in 0..game.keystone_slots() {
        let r = slot_rect(i);
        match game.keystones.get(i) {
            Some(&t) => {
                let tier = ecology::of(game.taxon(t).name).tier;
                let dormant = game.dormant_reason(t);
                let col = if dormant.is_some() {
                    ACCENT_WARN
                } else {
                    render::tier_color(tier)
                };
                frame(
                    r,
                    rgb(0x0A1020),
                    faded(col, if dormant.is_some() { 0.6 } else { 1.0 }),
                    1.5,
                );
                draw_rectangle(r.x + 2.0, r.y + 2.0, r.h - 4.0, r.h - 4.0, rgb(0x050A14));
                draw_taxon(
                    sprites,
                    game,
                    t,
                    best_morph(game, t),
                    vec2(r.x + r.h * 0.5, r.y + r.h * 0.5),
                    r.h * 0.8,
                    if dormant.is_some() { DORMANT } else { WHITE },
                );
                text(
                    game.taxon(t).name,
                    r.x + r.h * 1.15,
                    r.y + r.h * 0.42,
                    r.h * 0.3,
                    if dormant.is_some() { TEXT_DIM } else { TEXT },
                );
                let (line, line_col) = match &dormant {
                    Some(why) => (format!("Dormant: {why}"), ACCENT_WARN),
                    None => (
                        format!(
                            "{}  ·  x{:.1}",
                            ecology::of(game.taxon(t).name).bonus.describe(),
                            game.keystone_strength(t)
                        ),
                        LIME,
                    ),
                };
                text(
                    &line,
                    r.x + r.h * 1.15,
                    r.y + r.h * 0.8,
                    fit_px(&line, r.w * 0.8, r.h * 0.28),
                    line_col,
                );
                let (tag, tag_col) = if dormant.is_some() {
                    ("DORMANT", ACCENT_WARN)
                } else {
                    ("ACTIVE", LIME)
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
        frame(
            r,
            if equipped {
                rgb(0x12213A)
            } else {
                rgb(0x0A1020)
            },
            if sel {
                WHITE
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
            if dormant { DORMANT } else { WHITE },
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
            let px = r.h * 0.13;
            for (i, line) in wrap_lines(&eco.bonus.describe(), col_w, px)
                .iter()
                .take(2)
                .enumerate()
            {
                let y = r.y + r.h * (0.54 + 0.15 * i as f32);
                text(line, tx, y, fit_px(line, col_w, px), LIME);
            }
            let (status, status_col) = match game.dormant_reason(t) {
                Some(why) => (format!("Dormant here: {why}"), ACCENT_WARN),
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
                r.y + r.h * 0.91,
                fit_px(&status, r.w * 0.92, r.h * 0.13),
                status_col,
            );
            let equipped = game.keystones.contains(&t);
            let can = game.keystones_editable()
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
    draw_hud(game, now);
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
            SILHOUETTE,
        );
    }
    let px = r.w * 0.045;
    let name = if found { t.name } else { "? ? ?" };
    text_centered(
        name,
        cx,
        r.y + r.h * 0.36,
        fit_px(name, r.w * 0.9, px * 1.8),
        TEXT,
    );
    let sub = format!("{}  ·  {}  ·  {}", t.clade, t.age_label(), eco.tier.name());
    text_centered(
        &sub,
        cx,
        r.y + r.h * 0.415,
        fit_px(&sub, r.w * 0.9, px),
        col,
    );

    let x = r.x + r.w * 0.07;
    let mut y = r.y + r.h * 0.50;
    text("NEEDS  (your planet right now)", x, y, px, ACCENT_WARN);
    y += px * 1.6;
    let p = &game.planet;
    let n = eco.needs;
    let hab: Vec<&str> = n.habitats.iter().map(|h| h.name()).collect();
    let rows: Vec<(String, bool)> = [
        Some((hab.join(" or "), n.habitats.iter().any(|&h| p.has(h)))),
        (n.oxygen_min > 0).then(|| {
            (
                format!("Oxygen {}%+", planet::oxygen_percent(n.oxygen_min)),
                p.oxygen >= n.oxygen_min,
            )
        }),
        (n.temp_min > 0 || n.temp_max < 5).then(|| {
            (
                format!(
                    "{} to {}",
                    planet::temperature_label(n.temp_min),
                    planet::temperature_label(n.temp_max)
                ),
                (n.temp_min..=n.temp_max).contains(&p.temperature),
            )
        }),
        (n.veg_min > 0 || n.veg_max < 5).then(|| {
            (
                format!("Vegetation {} to {}", n.veg_min, n.veg_max),
                (n.veg_min..=n.veg_max).contains(&p.vegetation),
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
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
    y += px * 1.5;
    text(&eco.bonus.describe(), x, y, px, LIME);
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
            frame(
                mr,
                rgb(0x050A14),
                if owned { rgb(0x6FF5E1) } else { rgb(0x141C26) },
                1.0,
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
        }
        y += mw + px * 1.4;
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
