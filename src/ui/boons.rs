//! The boon pick.

use super::*;

pub fn boon_rect(i: usize) -> Rect {
    let (sw, sh) = (view::width(), screen_height());
    let (w, h) = (sw * 0.84, sh * 0.15);
    Rect::new((sw - w) * 0.5, sh * 0.26 + i as f32 * (h + sh * 0.03), w, h)
}

pub fn boon_at(game: &Game, p: Vec2) -> Option<usize> {
    let n = game.boon_offer.as_ref().map_or(0, |o| o.len());
    (0..n).find(|&i| boon_rect(i).contains(p))
}

pub(super) fn boon_color(b: Boon) -> Color {
    match b {
        Boon::Discovery => rgb(0x5AA8FF),
        Boon::Lens => LIME,
        Boon::Catalyst => rgb(0xC07BFF),
        Boon::Charm => PINK,
        Boon::Tailwind => CYAN,
        Boon::Tectonics => rgb(0xB89A6A),
    }
}

pub fn draw_boons(game: &Game, sprites: &Sprites, assets: &Assets, hover: Option<usize>) {
    let (sw, sh) = (view::width(), screen_height());
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
            -render::u() * 2.0
        } else {
            (t * 2.0 + i as f32).sin() * render::u() * 0.6
        };
        let r = Rect::new(r.x, r.y + lift, r.w, r.h);
        assets.glow(vec2(r.x + r.h * 0.5, r.y + r.h * 0.5), r.h * 0.7, col, 0.3);
        frame(r, CARD_BG, col, 2.0);
        let ic = vec2(r.x + r.h * 0.5, r.y + r.h * 0.5);
        let bob = sprites::Pose {
            y_off: (t * 2.0 + i as f32).sin() * 0.03,
            ..Default::default()
        };
        sprites::draw_sprite(sprites.boon(b.icon()), ic, r.h * 0.72, bob, WHITE);
        text(
            b.title(),
            r.x + r.h * 1.05,
            r.y + r.h * 0.42,
            r.h * 0.24,
            TEXT,
        );
        let d = b.describe(game);
        text(
            &d,
            r.x + r.h * 1.05,
            r.y + r.h * 0.74,
            fit_px(&d, r.w - r.h * 1.2, r.h * 0.17),
            col,
        );
    }
}
