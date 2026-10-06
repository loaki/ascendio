//! An animal's detail page.

use super::*;

pub(super) fn detail_rect() -> Rect {
    let (sw, sh) = (view::width(), screen_height());
    Rect::new(sw * 0.06, sh * 0.07, sw * 0.88, sh * 0.86)
}

pub(super) fn detail_needs(game: &Game, taxon: usize) -> Vec<(String, bool)> {
    let p = &game.planet;
    let n = game.taxon(taxon).eco.needs;
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
    rows
}

pub(super) const DETAIL_HEAD: f32 = 15.0;
pub(super) const DETAIL_TILE: f32 = 4.4;

pub(super) fn detail_units(game: &Game, taxon: usize, w: f32, px: f32) -> f32 {
    let eco = game.taxon(taxon).eco;
    let found = game.unlocked[taxon];
    let mut units = DETAIL_HEAD + 1.6 + detail_needs(game, taxon).len() as f32 * 1.5 + 0.6;
    if found || game.is_fossil(taxon) {
        units += wrap_lines(&eco.describe(), w * 0.86, px).len() as f32 * 1.4;
        units += eco.rule.drawback().map_or(0.0, |_| 1.4);
    } else {
        units += 1.4;
    }
    if found {
        units += 1.6 + 1.8 + DETAIL_TILE + 2.9;
    }
    units + 2.4
}

pub fn draw_detail(game: &Game, taxon: usize, sprites: &Sprites, assets: &Assets) {
    let (sw, sh) = (view::width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, faded(render::BG, 0.82));
    let r = detail_rect();
    let t = game.taxon(taxon);
    let found = game.unlocked[taxon];
    let eco = t.eco;
    let col = if found {
        render::tier_color(eco.tier)
    } else {
        LOCKED_TEXT
    };
    frame(r, rgb(0x070D1C), col, 2.0);
    // Everything is sized by the text, the text by the width -- unless the
    // page would then run out of the frame (a short screen, many needs).
    let px = r.w * 0.045;
    let px = px.min(r.h / detail_units(game, taxon, r.w, px));
    let cx = r.x + r.w * 0.5;
    let top = r.y + px;
    let sy = top + px * 4.0;
    let size = px * 7.5;
    if found {
        assets.glow(vec2(cx, sy), size * 0.9, col, 0.3);
        draw_taxon(
            sprites,
            game,
            taxon,
            game.best_morph(taxon),
            vec2(cx, sy),
            size,
            WHITE,
        );
    } else {
        draw_taxon(
            sprites,
            game,
            taxon,
            Morph::None,
            vec2(cx, sy),
            size,
            render::taxon_tint(game, taxon),
        );
    }
    let name = if found || game.is_fossil(taxon) {
        t.label()
    } else {
        "? ? ?"
    };
    text_centered(
        name,
        cx,
        top + px * 10.0,
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
        top + px * 11.8,
        fit_px(&sub, r.w * 0.9, px),
        if game.is_fossil(taxon) {
            STONE_TEXT
        } else {
            col
        },
    );

    let x = r.x + r.w * 0.07;
    let mut y = r.y + px * DETAIL_HEAD;
    text("NEEDS  (your planet right now)", x, y, px, ACCENT_WARN);
    y += px * 1.6;
    for (label, ok) in detail_needs(game, taxon) {
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
    // Its bonus is part of the discovery: kept secret until it evolves
    // (a fossil was found on an earlier Earth, so it's known).
    if found || game.is_fossil(taxon) {
        for line in wrap_lines(&eco.describe(), r.w * 0.86, px) {
            y += px * 1.4;
            text(&line, x, y, px, LIME);
        }
        if let Some(d) = eco.rule.drawback() {
            y += px * 1.4;
            let catch = format!("But: {d}");
            text(&catch, x, y, fit_px(&catch, r.w * 0.86, px), LOCK_RED);
        }
    } else {
        y += px * 1.4;
        text("? ? ?  revealed when it evolves", x, y, px, LOCKED_TEXT);
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
        // Four tiles across the page's width, whatever their size.
        let mw = px * DETAIL_TILE;
        let gap = (r.w * 0.86 - mw * 4.0) / 3.0;
        for (k, m) in Morph::ALL.into_iter().enumerate() {
            let mr = Rect::new(x + k as f32 * (mw + gap), y, mw, mw);
            let owned = game.owns_morph(taxon, m);
            let worn = owned && game.edition(taxon) == m;
            frame(
                mr,
                rgb(0x050A14),
                if worn {
                    LIME
                } else if owned {
                    CYAN
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
            for (i, line) in wrap_lines(m.effect(), mw + gap * 0.8, px * 0.6)
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

pub(super) fn range_label(lever: Lever, lo: u8, hi: u8) -> String {
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
