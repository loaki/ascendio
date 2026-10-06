//! The wait-choice panel.

use super::*;

pub fn hours_label(hours: f32) -> String {
    if hours.fract() > 0.0 {
        format!("{}H30", hours.floor())
    } else {
        format!("{hours}H")
    }
}

pub fn wait_panel_rect() -> Rect {
    let r = panel_rect();
    let top = dial::center().y + dial::radius() + dial::max_half_width() + render::u() * 3.0;
    let bottom = r.y + r.h;
    let top = top.min(bottom - render::u() * 52.0);
    Rect::new(r.x, top, r.w, bottom - top)
}

pub fn wait_back_rect() -> Rect {
    let r = wait_panel_rect();
    let (w, h) = (render::u() * 22.0, render::u() * 8.0);
    Rect::new(
        r.x + r.w - w - render::u() * 3.0,
        r.y + render::u() * 2.5,
        w,
        h,
    )
}

/// A keystone's status or gains in a line; `short` drops the reasons.
pub(super) fn report_line(r: &Report, short: bool) -> (String, Color) {
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

pub fn draw_wait_panel(game: &Game, sprites: &Sprites) {
    let r = wait_panel_rect();
    let u = render::u();
    frame(r, faded(DEEP_BG, 0.9), PANEL_EDGE, 1.5);
    let f = game.forecast();
    let row = r.h / 5.4;
    let px = (row * 0.62).min(u * 6.5);
    let (x, w) = (r.x + r.w * 0.04, r.w * 0.92);
    let mut y = r.y + row * 0.85;

    text("THE AGES WILL BRING", x, y, px, ACCENT_WARN);
    let b = wait_back_rect();
    frame(b, BUTTON_BG, BUTTON_EDGE, 1.5);
    text_centered("BACK", b.x + b.w * 0.5, b.y + b.h * 0.75, b.h * 0.7, TEXT);

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
        let initial = &t.name()[..1];
        let label = if pct < 1.0 {
            format!("{initial} {pct:.1}%")
        } else {
            format!("{initial} {pct:.0}%")
        };
        text(
            &label,
            x + k as f32 * cell,
            y,
            px * 0.9,
            render::tier_color(*t),
        );
    }

    y += row * 0.4;
    let (cw, ch) = (u * 5.0, u * 7.0);
    let sure = f.odds.cards;
    let chance = f.cards.fract();
    let col = dial::color(game.wait_hours);
    let area = 6.0 * (cw + u * 1.5);
    let slots = (sure + (chance > 0.0) as usize).max(1);
    let step = (area / slots as f32).min(cw + u * 1.5);
    let cw = cw.min(step - u * 0.5);
    for k in 0..slots {
        let cx = x + k as f32 * step;
        let (fill, edge) = if k < sure {
            (CARD_BG, col)
        } else if k == sure && chance > 0.0 {
            (DEEP_BG, rgb(0x5A6678))
        } else {
            (DEEP_BG, rgb(0x141C26))
        };
        frame(Rect::new(cx, y, cw, ch), fill, edge, 1.5);
    }
    let tx = x + area + u * 2.0;
    let new = format!("{:.0}% new", f.odds.discovery * 100.0);
    let cards = if chance > 0.0 {
        format!(
            "{sure} cards  ·  {:.0}% for one more  ·  {new}",
            chance * 100.0
        )
    } else {
        format!("{sure} cards  ·  {new}")
    };
    let tw = x + w - tx;
    text(
        &cards,
        tx,
        y + ch * 0.45,
        fit_px(&cards, tw, px * 0.9),
        TEXT,
    );
    let extra = if f.odds.sure_epic {
        ("one Epic or better is sure".to_string(), GOLD)
    } else if f.odds.catalyst {
        ("one Rare or better is sure".to_string(), GOLD)
    } else {
        (
            format!(
                "{:.1}% of cards are morphs",
                genome::morph_chance(f.odds.morph_mult, f.odds.morph_rad) * 100.0
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

    let top = y + ch + u * 2.5;
    y += ch + row * 0.75;
    if game.keystones.is_empty() {
        text("No keystones equipped", x, y, px * 0.9, TEXT_DIM);
        return;
    }
    let area = Rect::new(x, top, w, r.y + r.h - top - u * 1.5);
    draw_keystone_row(game, sprites, &game.keystones, area, px);
}
