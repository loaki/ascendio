//! The bottom tab bar.

use super::*;

pub fn bottom_action_rect() -> Rect {
    let (sw, bar_h) = (view::width(), render::bar_height());
    let w = sw * 0.44;
    Rect::new((sw - w) * 0.5, screen_height() - bar_h, w, bar_h)
}

pub fn bottom_tab_rect(right: bool) -> Rect {
    let (sw, bar_h) = (view::width(), render::bar_height());
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
    let (sw, bar_h) = (view::width(), render::bar_height());
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
    let (ey, eh) = (e.y + render::u() * 2.0, e.h - render::u() * 4.0);
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
                    game::plural(f.odds.cards as u32),
                    f.points,
                    game::plural(f.points.into())
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
