//! The lever panel and the running-wait summary.

use super::*;

pub fn panel_rect() -> Rect {
    let (sw, sh) = (view::width(), screen_height());
    let (w, h) = (sw * 0.84, sh * 0.31);
    Rect::new((sw - w) * 0.5, sh - render::bar_height() - h - 10.0, w, h)
}

pub(super) fn lever_row(i: usize) -> Rect {
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

pub(super) fn lever_color(lever: Lever) -> Color {
    match lever {
        Lever::Land => rgb(0xB89A6A),
        Lever::Vegetation => rgb(0x8FC878),
        Lever::Oxygen => LIME,
        Lever::Temperature => GOLD,
        Lever::Volcanism => LOCK_RED,
    }
}

pub(super) fn shape_hint(game: &Game) -> (String, Color) {
    if let Some((k, why)) = game
        .keystones
        .iter()
        .find_map(|&k| game.dormant_reason(k).map(|why| (k, why)))
    {
        return (
            format!("Keystone {} is dormant: {why}", game.taxon(k).label()),
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
            let why = b.ranges().missing(&game.planet, false)?;
            Some(format!("{}: {why}", b.name()))
        })
        .take(2)
        .collect();
    (others.join("  ·  "), LOCKED_TEXT)
}

pub fn cancel_wait_rect() -> Rect {
    let r = panel_rect();
    let (w, h) = (render::u() * 26.0, render::u() * 8.0);
    Rect::new(
        r.x + r.w - w - render::u() * 3.0,
        r.y + render::u() * 2.5,
        w,
        h,
    )
}

pub(super) fn draw_launched_keystones(game: &Game, sprites: &Sprites, r: Rect, px: f32) {
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
        r.y + r.h * 0.705,
        px * 0.85,
        TEXT_DIM,
    );
    let top = r.y + r.h * 0.725;
    let area = Rect::new(x, top, r.w * 0.92, r.y + r.h - top - render::u());
    draw_keystone_row(game, sprites, &ks, area, px);
}

pub(super) fn draw_keystone_row(
    game: &Game,
    sprites: &Sprites,
    keystones: &[usize],
    area: Rect,
    px: f32,
) {
    let u = render::u();
    let reports = game.keystone_reports();
    let slot = area.w / keystones.len().max(3) as f32;
    let line_px = px * 0.9;
    let line_h = line_px * 1.15;
    let effects: Vec<(Vec<String>, Color)> = keystones
        .iter()
        .map(|&t| {
            let (e, c) = match reports.iter().find(|r| r.taxon == t) {
                Some(r) => report_line(r, true),
                None => ("-".into(), TEXT_DIM),
            };
            let parts: Vec<&str> = e.split("  ·  ").collect();
            let lines = match parts.len() {
                0..=2 => parts.iter().map(|p| p.to_string()).collect(),
                _ => vec![parts[0].to_string(), parts[1..].join("  ·  ")],
            };
            (lines, c)
        })
        .collect();
    // The animals get whatever the tallest effect leaves; too little (a
    // short screen), and they go beside their effects instead.
    let most = effects.iter().map(|(l, _)| l.len()).max().unwrap_or(1);
    let icon = (px * 1.7).min(area.h - line_h * most as f32 - u);
    let beside = icon < px * 1.4;
    let icon = if beside {
        (px * 1.9).min(area.h - u)
    } else {
        icon
    };
    for (k, (&t, (lines, c))) in keystones.iter().zip(&effects).enumerate() {
        let x0 = area.x + k as f32 * slot;
        let cx = x0 + slot * 0.5;
        let dormant = game.dormant_reason(t).is_some();
        let at = if beside {
            vec2(x0 + u + icon * 0.5, area.y + area.h * 0.5)
        } else {
            vec2(cx, area.y + icon * 0.5)
        };
        draw_taxon(
            sprites,
            game,
            t,
            game.best_morph(t),
            at,
            icon,
            if dormant { DORMANT } else { WHITE },
        );
        if beside {
            let tx = x0 + u * 2.0 + icon;
            let top = area.y + (area.h - line_h * lines.len() as f32) * 0.5;
            for (n, line) in lines.iter().enumerate() {
                text(
                    line,
                    tx,
                    top + line_h * (n as f32 + 0.8),
                    fit_px(line, x0 + slot - u - tx, line_px),
                    *c,
                );
            }
            continue;
        }
        for (n, line) in lines.iter().enumerate() {
            text_centered(
                line,
                cx,
                area.y + icon + u * 0.5 + line_h * (n as f32 + 0.8),
                fit_px(line, slot - u * 2.0, line_px),
                *c,
            );
        }
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
    frame(r, faded(DEEP_BG, 0.88), PANEL_EDGE, 1.5);
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
            frame(b, BUTTON_BG, col, 1.5);
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
                    if k < left { CYAN } else { TRACK },
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
                let value = planet::level_label(lever, game.planet.get(lever));
                text(
                    &value,
                    row.x + r.w * 0.32,
                    row.y + row.h * 0.64,
                    fit_px(&value, r.w * 0.26, rpx * 0.9),
                    col,
                );
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
                        (BUTTON_BG, BUTTON_EDGE, TEXT)
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

pub(super) fn draw_biome_line(game: &Game, r: Rect, px: f32) {
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
        CYAN,
    );
}
