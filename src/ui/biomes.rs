//! The biome guide.

use super::*;

pub fn biomes_button_rect() -> Rect {
    let u = render::u();
    let (w, h) = (u * 40.0, u * 16.0);
    Rect::new(
        view::width() - w - u * 4.0,
        render::bar_height() + u * 4.0,
        w,
        h,
    )
}

pub fn draw_biomes_button() {
    let b = biomes_button_rect();
    frame(b, faded(PANEL, 0.92), CYAN, 1.5);
    text_centered("BIOMES", b.x + b.w * 0.5, b.y + b.h * 0.68, b.h * 0.6, CYAN);
}

pub(super) fn biome_recipe(b: Biome) -> String {
    let r = b.ranges();
    Lever::ALL
        .iter()
        .filter(|&&l| r.get(l) != planet::Ranges::ANY.get(l))
        .map(|&l| {
            let (lo, hi) = r.get(l);
            let label = range_label(l, lo, hi);
            if l == Lever::Temperature {
                format!("Temp {label}")
            } else {
                label
            }
        })
        .collect::<Vec<_>>()
        .join("  ·  ")
}

pub fn draw_biomes(game: &Game, sprites: &Sprites) {
    let (sw, sh, u) = (view::width(), screen_height(), render::u());
    draw_rectangle(0.0, 0.0, sw, sh, faded(render::BG, 0.9));
    let r = Rect::new(sw * 0.04, sh * 0.07, sw * 0.92, sh * 0.86);
    frame(r, rgb(0x070D1C), CYAN, 2.0);
    let px = u * 7.0;
    let x = r.x + u * 5.0;
    let w = r.w - u * 10.0;
    text("BIOMES", x, r.y + u * 11.0, px * 1.3, ACCENT_WARN);
    let note = format!(
        "Animals of a biome live only there. {} keystones of a biome give its bonus.",
        ecology::BIOME_SET
    );
    text(
        &note,
        x,
        r.y + u * 18.0,
        fit_px(&note, w, px * 0.75),
        TEXT_DIM,
    );

    let top = r.y + u * 24.0;
    let active = game.active_biomes();
    let row_h = (r.y + r.h - u * 10.0 - top) / Biome::ALL.len() as f32;
    let p = &game.planet;
    for (i, b) in Biome::ALL.into_iter().enumerate() {
        let y = top + i as f32 * row_h;
        let col = rgb(b.color());
        if i > 0 {
            draw_line(x, y, x + w, y, 1.0, EDGE);
        }
        let name_y = y + row_h * 0.27;
        let name = b.name().to_uppercase();
        text(&name, x, name_y, px, col);
        let on = active.contains(&b);
        let bonus = if on {
            format!("{}  ON", ecology::biome_bonus_label(b))
        } else {
            ecology::biome_bonus_label(b)
        };
        let bx = x + text_width(&name, px) + u * 4.0;
        text(
            &bonus,
            bx,
            name_y,
            px * 0.8,
            if on { LIME } else { TEXT_DIM },
        );
        let members: Vec<usize> = (0..game.phy.len())
            .filter(|&t| game.taxon(t).eco.needs.biome == Some(b))
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
        let (status, status_col) = match b.ranges().missing(p, false) {
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
                    game.best_morph(t)
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
