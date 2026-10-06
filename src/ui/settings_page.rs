//! The settings screen and shared full-screen page chrome.

use super::*;

pub struct SettingsView {
    pub notify: bool,
    pub supported: bool,
    pub allowed: bool,
    /// `Settings::brightness`, in percent.
    pub brightness: u8,
    pub language: Language,
    pub name: String,
    pub typing: Option<String>,
    /// RESET was tapped once; a second tap erases the account.
    pub reset_armed: bool,
}

pub(super) fn draw_screen_bar(title: &str, assets: &Assets, gear_lit: bool, trophy_lit: bool) {
    let (sw, u, bar_h) = (view::width(), render::u(), render::bar_height());
    draw_top_bar();
    draw_gear(assets, gear_lit);
    draw_trophy(assets, trophy_lit);
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
    text_centered(title, sw * 0.5, bar_h * 0.6, bar_h * 0.36, TEXT);
}

pub fn update_banner_rect() -> Rect {
    let (u, bar_h) = (render::u(), render::bar_height());
    Rect::new(u * 4.0, bar_h + u * 2.0, view::width() - u * 8.0, u * 12.0)
}

pub fn draw_update_banner() {
    let r = update_banner_rect();
    draw_rectangle(r.x, r.y, r.w, r.h, render::PANEL);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, render::ACCENT_OK);
    text_centered(
        "New version available - tap to update",
        r.x + r.w * 0.5,
        r.y + r.h * 0.65,
        r.h * 0.45,
        TEXT,
    );
}

pub(super) fn draw_version_footer() {
    let (sw, sh, bar_h) = (view::width(), screen_height(), render::bar_height());
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

pub fn settings_back_rect() -> Rect {
    let g = gear_rect();
    Rect::new(view::width() - g.x - g.w, g.y, g.w, g.h)
}

pub(super) fn settings_panel_rect() -> Rect {
    let (l, u) = (player_panel_rect(), render::u());
    Rect::new(l.x, l.y + l.h + u * 6.0, l.w, u * 46.0)
}

pub(super) fn display_panel_rect() -> Rect {
    let (n, u) = (settings_panel_rect(), render::u());
    Rect::new(n.x, n.y + n.h + u * 6.0, n.w, u * 50.0)
}

pub(super) fn reset_panel_rect() -> Rect {
    let (d, u) = (display_panel_rect(), render::u());
    Rect::new(d.x, d.y + d.h + u * 6.0, d.w, u * 34.0)
}

pub fn reset_button_rect() -> Rect {
    let (p, u) = (reset_panel_rect(), render::u());
    let (w, h) = (u * 34.0, u * 13.0);
    Rect::new(p.x + p.w - u * 5.0 - w, p.y + u * 16.0, w, h)
}

pub(super) fn draw_reset_panel(view: &SettingsView) {
    let (p, u) = (reset_panel_rect(), render::u());
    frame(p, PANEL, EDGE, 1.0);
    let x = p.x + u * 6.0;
    let px = u * 7.5;
    text("ACCOUNT", x, p.y + u * 10.0, px, ACCENT_WARN);
    let b = reset_button_rect();
    let (label, tone) = if view.reset_armed {
        ("CONFIRM", ACCENT_WARN)
    } else {
        ("RESET", TEXT)
    };
    let sub = if view.reset_armed {
        "Tap CONFIRM to erase everything."
    } else {
        "Erase progress and start over."
    };
    let sub_px = px * 0.8;
    text(
        sub,
        x,
        b.y + b.h * 0.66,
        fit_px(sub, b.x - u * 4.0 - x, sub_px),
        if view.reset_armed {
            ACCENT_WARN
        } else {
            TEXT_DIM
        },
    );
    frame(
        b,
        BUTTON_BG,
        if view.reset_armed {
            ACCENT_WARN
        } else {
            BUTTON_EDGE
        },
        1.5,
    );
    text_centered(
        label,
        b.x + b.w * 0.5,
        b.y + b.h * 0.68,
        fit_px(label, b.w * 0.85, b.h * 0.5),
        tone,
    );
}

pub fn brightness_button_rect(plus: bool) -> Rect {
    let (p, u) = (display_panel_rect(), render::u());
    let s = u * 13.0;
    let x = p.x + p.w - u * 5.0 - s - if plus { 0.0 } else { s + u * 3.0 };
    Rect::new(x, p.y + u * 16.0, s, s)
}

pub fn language_button_rect(lang: Language) -> Rect {
    let (p, u) = (display_panel_rect(), render::u());
    let (w, h) = (u * 16.0, u * 13.0);
    let fr = lang == Language::French;
    let x = p.x + p.w - u * 5.0 - w - if fr { 0.0 } else { w + u * 3.0 };
    Rect::new(x, p.y + u * 32.0, w, h)
}

pub(super) fn player_panel_rect() -> Rect {
    let (sw, u) = (view::width(), render::u());
    let y = render::bar_height() + u * 8.0;
    Rect::new(sw * 0.04, y, sw * 0.92, u * 34.0)
}

pub fn name_button_rect() -> Rect {
    let (p, u) = (player_panel_rect(), render::u());
    let (w, h) = (u * 28.0, u * 13.0);
    Rect::new(p.x + p.w - u * 5.0 - w, p.y + u * 16.0, w, h)
}

pub(super) fn draw_player_panel(view: &SettingsView) {
    let (p, u) = (player_panel_rect(), render::u());
    frame(p, PANEL, EDGE, 1.0);
    let x = p.x + u * 6.0;
    let px = u * 7.5;
    text("LEADERBOARD", x, p.y + u * 10.0, px, ACCENT_WARN);
    let edit = name_button_rect();
    let y = edit.y + edit.h * 0.66;
    text("Name", x, y, px, TEXT);
    let nx = x + text_width("Name", px) + u * 5.0;
    let room = edit.x - u * 4.0 - nx;
    match &view.typing {
        Some(typed) => {
            let caret = if (get_time() * 2.0) as i64 % 2 == 0 {
                "_"
            } else {
                " "
            };
            let shown = format!("{typed}{caret}");
            text(&shown, nx, y, fit_px(&shown, room, px * 0.9), TEXT);
        }
        None => text(&view.name, nx, y, fit_px(&view.name, room, px * 0.9), CYAN),
    }
    let hot = view.typing.is_some();
    let label = if hot { "SAVE" } else { "EDIT" };
    frame(edit, BUTTON_BG, if hot { CYAN } else { BUTTON_EDGE }, 1.5);
    text_centered(
        label,
        edit.x + edit.w * 0.5,
        edit.y + edit.h * 0.68,
        fit_px(label, edit.w * 0.85, edit.h * 0.5),
        if hot { CYAN } else { TEXT },
    );
}

pub(super) fn draw_display_panel(view: &SettingsView) {
    let (p, u) = (display_panel_rect(), render::u());
    frame(p, PANEL, EDGE, 1.0);
    let x = p.x + u * 6.0;
    let px = u * 7.5;
    text("DISPLAY", x, p.y + u * 10.0, px, ACCENT_WARN);

    let minus = brightness_button_rect(false);
    let y = minus.y + minus.h * 0.66;
    text("Brightness", x, y, px, TEXT);
    let value = format!("{}%", view.brightness);
    text(
        &value,
        x + text_width("Brightness", px) + u * 5.0,
        y,
        px * 0.9,
        CYAN,
    );
    let levels =
        (settings::BRIGHTNESS_MAX - settings::BRIGHTNESS_MIN) / settings::BRIGHTNESS_STEP + 1;
    let lit = (view.brightness - settings::BRIGHTNESS_MIN) / settings::BRIGHTNESS_STEP + 1;
    let pip_w = view::width() * 0.0235;
    let pip_h = pip_w * 1.4;
    let x0 = minus.x - u * 5.0 - (levels as f32 * 1.35 - 0.35) * pip_w;
    for k in 0..levels {
        let c = if k < lit { CYAN } else { TRACK };
        draw_rectangle(
            x0 + k as f32 * pip_w * 1.35,
            minus.y + (minus.h - pip_h) * 0.5,
            pip_w,
            pip_h,
            c,
        );
    }
    for (plus, label) in [(false, "-"), (true, "+")] {
        let b = brightness_button_rect(plus);
        let ok = if plus {
            view.brightness < settings::BRIGHTNESS_MAX
        } else {
            view.brightness > settings::BRIGHTNESS_MIN
        };
        let (fill, edge, ink) = if ok {
            (BUTTON_BG, BUTTON_EDGE, TEXT)
        } else {
            (PANEL_LOCKED, EDGE, LOCKED_TEXT)
        };
        frame(b, fill, edge, 1.5);
        text_centered(label, b.x + b.w * 0.5, b.y + b.h * 0.72, b.h * 0.7, ink);
    }

    let en = language_button_rect(Language::English);
    let label = "Species names";
    text(
        label,
        x,
        en.y + en.h * 0.66,
        fit_px(label, en.x - u * 4.0 - x, px),
        TEXT,
    );
    for (lang, tag) in [(Language::English, "EN"), (Language::French, "FR")] {
        let b = language_button_rect(lang);
        let on = view.language == lang;
        frame(b, BUTTON_BG, if on { CYAN } else { BUTTON_EDGE }, 1.5);
        text_centered(
            tag,
            b.x + b.w * 0.5,
            b.y + b.h * 0.68,
            b.h * 0.5,
            if on { CYAN } else { TEXT_DIM },
        );
    }
}

pub fn notify_row_rect() -> Rect {
    let (p, u) = (settings_panel_rect(), render::u());
    Rect::new(p.x, p.y + u * 16.0, p.w, u * 28.0)
}

pub fn notify_checkbox_rect() -> Rect {
    let (row, u) = (notify_row_rect(), render::u());
    let s = u * 11.0;
    Rect::new(row.x + row.w - u * 6.0 - s, row.y + u * 2.0, s, s)
}

pub fn notify_blocked(v: &SettingsView) -> bool {
    v.supported && v.notify && !v.allowed
}

pub fn draw_settings(view: &SettingsView, assets: &Assets) {
    clear_background(render::BG);
    let u = render::u();

    draw_screen_bar("SETTINGS", assets, true, false);

    let p = settings_panel_rect();
    frame(p, PANEL, EDGE, 1.0);
    let x = p.x + u * 6.0;
    let px = u * 7.5;
    text("NOTIFICATIONS", x, p.y + u * 10.0, px, ACCENT_WARN);

    let row = notify_row_rect();
    let bx = notify_checkbox_rect();
    let s = bx.w;
    let (edge, fill) = match (view.supported, view.notify) {
        (false, _) => (EDGE, PANEL_LOCKED),
        (true, true) => (ACCENT_OK, ACCENT_OK),
        (true, false) => (LOCKED_BORDER, PANEL_LOCKED),
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

    let tx = x;
    let label_c = if view.supported { TEXT } else { LOCKED_TEXT };
    text("Genome ready", tx, row.y + u * 9.0, px, label_c);
    let (sub, sub_c) = if notify_blocked(view) {
        (
            "Blocked by Android: tap to allow notifications.",
            ACCENT_WARN,
        )
    } else {
        (
            "Send a notification when a new genome is ready to evolve.",
            TEXT_DIM,
        )
    };
    let sub_px = px * 0.8;
    for (i, line) in wrap_lines(sub, bx.x - u * 6.0 - tx, sub_px)
        .iter()
        .enumerate()
    {
        text(
            line,
            tx,
            row.y + u * 16.0 + i as f32 * u * 6.5,
            sub_px,
            sub_c,
        );
    }

    draw_display_panel(view);
    draw_reset_panel(view);

    draw_player_panel(view);
    draw_version_footer();
}

pub(super) fn board_rows_rect() -> Rect {
    let (sw, sh, u, bar_h) = (
        view::width(),
        screen_height(),
        render::u(),
        render::bar_height(),
    );
    let top = bar_h + u * 16.0;
    Rect::new(sw * 0.04, top, sw * 0.92, sh - bar_h - u * 4.0 - top)
}

pub(super) fn board_row_h() -> f32 {
    render::u() * 18.0
}

/// This player's row, wherever it is: pinned under the list so the rank is
/// always in view.
pub(super) fn board_me(board: &Board) -> Option<&Entry> {
    board
        .me
        .as_ref()
        .or_else(|| board.top.iter().find(|e| e.me))
}

pub(super) fn board_scroll_rect(board: &Board) -> Rect {
    let mut r = board_rows_rect();
    if board_me(board).is_some() {
        r.h -= board_row_h() + render::u() * 6.0;
    }
    r
}

pub fn leaderboard_max_scroll(lb: &Leaderboard) -> f32 {
    let View::Ready(board) = &lb.view else {
        return 0.0;
    };
    let rows = board.top.len() as f32 * board_row_h();
    (rows - board_scroll_rect(board).h).max(0.0)
}

pub(super) fn rank_color(rank: u32) -> Color {
    match rank {
        1 => GOLD,
        2 => rgb(0xC8D2DC),
        3 => rgb(0xD08A50),
        _ => TEXT_DIM,
    }
}

pub(super) fn draw_board_row(game: &Game, sprites: &Sprites, e: &Entry, r: Rect) {
    let u = render::u();
    frame(
        Rect::new(r.x, r.y + u * 0.5, r.w, r.h - u),
        if e.me { faded(CYAN, 0.12) } else { CARD_BG },
        if e.me { CYAN } else { PANEL_EDGE },
        if e.me { 1.5 } else { 1.0 },
    );
    let cy = r.y + r.h * 0.5;
    let base = cy + u * 2.6;
    let px = u * 7.0;
    let rank = e.rank.to_string();
    let rank_px = fit_px(&rank, u * 15.0, px * 1.1);
    text(
        &rank,
        r.x + u * 18.0 - text_width(&rank, rank_px),
        base,
        rank_px,
        rank_color(e.rank),
    );
    let icon = r.h * 0.78;
    let ix = r.x + u * 22.0 + icon * 0.5;
    if e.animal < game.phy.len() {
        draw_taxon(
            sprites,
            game,
            e.animal,
            Morph::None,
            vec2(ix, cy),
            icon,
            WHITE,
        );
    } else {
        text_centered("?", ix, base, px, LOCKED_TEXT);
    }
    let right = r.x + r.w - u * 4.0;
    let species = format!("{} species", e.species);
    let spx = px * 0.85;
    let species_x = right - text_width(&species, spx);
    text(&species, species_x, base, spx, LIME);
    let rad = format!("{} RAD", e.rad);
    let rad_x = species_x - u * 6.0 - text_width(&rad, spx);
    text(
        &rad,
        rad_x,
        base,
        spx,
        if e.rad > 0 { RAD_COLOR } else { LOCKED_TEXT },
    );
    let nx = ix + icon * 0.5 + u * 4.0;
    let name_px = fit_px(&e.name, rad_x - u * 4.0 - nx, px);
    text(&e.name, nx, base, name_px, if e.me { CYAN } else { TEXT });
}

pub fn draw_leaderboard(game: &Game, lb: &Leaderboard, sprites: &Sprites, assets: &Assets) {
    clear_background(render::BG);
    draw_screen_bar("LEADERBOARD", assets, false, true);
    let (sw, u) = (view::width(), render::u());
    let rows = board_rows_rect();
    let px = u * 6.0;
    let mid = rows.y + rows.h * 0.4;
    let message = |line: &str, sub: &str| {
        text_centered(
            line,
            sw * 0.5,
            mid,
            fit_px(line, sw * 0.9, px * 1.2),
            TEXT_DIM,
        );
        text_centered(sub, sw * 0.5, mid + u * 10.0, px, LOCKED_TEXT);
    };
    let board = match &lb.view {
        View::Unavailable => {
            message("The leaderboard isn't set up", "in this build");
            return;
        }
        View::Loading => {
            message("Loading...", "");
            return;
        }
        View::Failed => {
            message("Can't reach the leaderboard", "tap to try again");
            return;
        }
        View::Ready(board) => board,
    };
    let count = format!(
        "{} player{}  ·  ranked by RAD, then species",
        board.players,
        game::plural(board.players)
    );
    text(
        &count,
        rows.x,
        rows.y - u * 6.0,
        fit_px(&count, rows.w, px * 0.9),
        TEXT_DIM,
    );
    if board.top.is_empty() {
        message("No one yet", "be the first");
        return;
    }
    let area = board_scroll_rect(board);
    let h = board_row_h();
    for (k, e) in board.top.iter().enumerate() {
        let y = area.y + k as f32 * h - lb.scroll;
        if y + h < area.y || y > area.y + area.h {
            continue;
        }
        draw_board_row(game, sprites, e, Rect::new(area.x, y, area.w, h));
    }
    let bar_h = render::bar_height();
    draw_rectangle(0.0, bar_h, sw, area.y - bar_h, render::BG);
    text(
        &count,
        rows.x,
        rows.y - u * 6.0,
        fit_px(&count, rows.w, px * 0.9),
        TEXT_DIM,
    );
    let below = area.y + area.h;
    draw_rectangle(0.0, below, sw, screen_height() - below, render::BG);
    if let Some(me) = board_me(board) {
        let y = below + u * 6.0;
        text("YOU", area.x, y - u * 1.5, px * 0.85, CYAN);
        draw_board_row(game, sprites, me, Rect::new(area.x, y, area.w, h));
    }
    draw_screen_bar("LEADERBOARD", assets, false, true);
    draw_version_footer();
}
