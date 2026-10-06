//! The keystones screen: slots, collection grid, sort and filter.

use super::*;

#[derive(Default)]
pub struct KeystoneView {
    pub scroll: f32,
    pub selected: Option<usize>,
    pub sort: Sort,
    pub filter: Filter,
    pub menu: Option<Menu>,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Sort {
    #[default]
    Tree,
    Bonus,
    Rarity,
    Level,
    Age,
    Name,
    Newest,
}

impl Sort {
    pub const ALL: [Sort; 7] = [
        Sort::Tree,
        Sort::Bonus,
        Sort::Rarity,
        Sort::Level,
        Sort::Age,
        Sort::Name,
        Sort::Newest,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Sort::Tree => "Tree",
            Sort::Bonus => "Bonus",
            Sort::Rarity => "Rarity",
            Sort::Level => "Level",
            Sort::Age => "Age",
            Sort::Name => "Name",
            Sort::Newest => "Newest",
        }
    }

    fn apply(self, game: &Game, taxa: &mut [usize]) {
        let level = |t: usize| {
            if game.is_fossil(t) {
                level_for_fossil(game, t)
            } else {
                game.level[t]
            }
        };
        let size = |t: usize| {
            let eco = game.taxon(t).eco;
            let triple = if eco.rule == Rule::Fragile { 3.0 } else { 1.0 };
            game.keystone_strength(t) * triple
        };
        let kind = |t: usize| {
            let b = game.taxon(t).eco.bonus;
            Bonus::ALL.iter().position(|&k| k == b).unwrap_or(0)
        };
        match self {
            Sort::Tree => {}
            Sort::Bonus => {
                taxa.sort_by(|&a, &b| kind(a).cmp(&kind(b)).then(size(b).total_cmp(&size(a))))
            }
            Sort::Rarity => taxa.sort_by_key(|&t| std::cmp::Reverse(game.taxon(t).eco.tier)),
            Sort::Level => taxa.sort_by_key(|&t| std::cmp::Reverse(level(t))),
            Sort::Age => taxa.sort_by(|&a, &b| game.taxon(b).mya.total_cmp(&game.taxon(a).mya)),
            Sort::Name => {
                taxa.sort_by_key(|&t| crate::game::names::sort_key(game.taxon(t).label()))
            }
            Sort::Newest => taxa.sort_by_key(|&t| std::cmp::Reverse(game.found_ma[t])),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Filter {
    #[default]
    All,
    Active,
    Luck,
    Cards,
    Discovery,
    Morphs,
    Wait,
    Points,
    Fossils,
}

impl Filter {
    pub const ALL: [Filter; 9] = [
        Filter::All,
        Filter::Active,
        Filter::Luck,
        Filter::Cards,
        Filter::Discovery,
        Filter::Morphs,
        Filter::Wait,
        Filter::Points,
        Filter::Fossils,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Luck => "Luck",
            Filter::Cards => "Cards",
            Filter::Discovery => "Discovery",
            Filter::Morphs => "Morphs",
            Filter::Wait => "Shorter wait",
            Filter::Points => "Points",
            Filter::Fossils => "Fossils",
        }
    }

    fn keeps(self, game: &Game, t: usize) -> bool {
        let bonus = game.taxon(t).eco.bonus;
        match self {
            Filter::All => true,
            Filter::Active => !game.is_fossil(t) && game.dormant_reason(t).is_none(),
            Filter::Luck => bonus == Bonus::Luck,
            Filter::Cards => bonus == Bonus::Cards,
            Filter::Discovery => bonus == Bonus::Discovery,
            Filter::Morphs => bonus == Bonus::Morph,
            Filter::Wait => bonus == Bonus::Quick,
            Filter::Points => bonus == Bonus::Point,
            Filter::Fossils => game.is_fossil(t),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Menu {
    Sort,
    Filter,
}

impl Menu {
    fn len(self) -> usize {
        match self {
            Menu::Sort => Sort::ALL.len(),
            Menu::Filter => Filter::ALL.len(),
        }
    }

    fn item(self, i: usize) -> &'static str {
        match self {
            Menu::Sort => Sort::ALL[i].label(),
            Menu::Filter => Filter::ALL[i].label(),
        }
    }

    fn chosen(self, view: &KeystoneView) -> usize {
        match self {
            Menu::Sort => Sort::ALL.iter().position(|&s| s == view.sort),
            Menu::Filter => Filter::ALL.iter().position(|&f| f == view.filter),
        }
        .unwrap_or(0)
    }
}

pub fn keystone_menu_button_rect(game: &Game, menu: Menu) -> Rect {
    let (sw, u) = (view::width(), render::u());
    let gap = u * 3.0;
    let w = (sw * 0.92 - gap) * 0.5;
    let x = sw * 0.04 + if menu == Menu::Filter { w + gap } else { 0.0 };
    Rect::new(
        x,
        slot_rect(game.keystone_slots()).y + u * 12.0,
        w,
        u * 10.0,
    )
}

pub(super) fn keystone_menu_item_rect(game: &Game, menu: Menu, i: usize) -> Rect {
    let (b, u) = (keystone_menu_button_rect(game, menu), render::u());
    let h = u * 10.0;
    let below = b.y + b.h + u;
    let top = if below + h * menu.len() as f32 > screen_height() - render::bar_height() - u {
        b.y - u - h * menu.len() as f32
    } else {
        below
    };
    Rect::new(b.x, top + i as f32 * h, b.w, h)
}

pub fn keystone_menu_item_at(game: &Game, view: &KeystoneView, p: Vec2) -> Option<usize> {
    let menu = view.menu?;
    (0..menu.len()).find(|&i| keystone_menu_item_rect(game, menu, i).contains(p))
}

pub fn choose_keystone_menu(view: &mut KeystoneView, menu: Menu, i: usize) {
    match menu {
        Menu::Sort => view.sort = Sort::ALL[i],
        Menu::Filter => view.filter = Filter::ALL[i],
    }
    view.menu = None;
    view.scroll = 0.0;
}

pub(super) fn draw_keystone_menus(game: &Game, view: &KeystoneView) {
    let u = render::u();
    for (menu, title) in [(Menu::Sort, "SORT"), (Menu::Filter, "FILTER")] {
        let b = keystone_menu_button_rect(game, menu);
        let open = view.menu == Some(menu);
        let lit = open || (menu == Menu::Filter && view.filter != Filter::All);
        frame(b, BUTTON_BG, if lit { CYAN } else { BUTTON_EDGE }, 1.5);
        let px = b.h * 0.55;
        let y = b.y + b.h * 0.7;
        text(title, b.x + u * 3.0, y, px, TEXT_DIM);
        let vx = b.x + u * 4.5 + text_width(title, px);
        let value = menu.item(menu.chosen(view)).to_uppercase();
        let room = b.x + b.w - u * 9.0 - vx;
        text(&value, vx, y, fit_px(&value, room, px), CYAN);
        let (ax, ay, s) = (b.x + b.w - u * 5.0, b.y + b.h * 0.5, u * 2.0);
        let (tip, base) = if open { (-s, s * 0.6) } else { (s, -s * 0.6) };
        draw_triangle(
            vec2(ax - s, ay + base),
            vec2(ax + s, ay + base),
            vec2(ax, ay + tip),
            TEXT_DIM,
        );
    }
}

pub(super) fn draw_keystone_menu_list(game: &Game, view: &KeystoneView) {
    let Some(menu) = view.menu else { return };
    let u = render::u();
    let first = keystone_menu_item_rect(game, menu, 0);
    let all = Rect::new(first.x, first.y, first.w, first.h * menu.len() as f32);
    frame(all, DEEP_BG, CYAN, 1.5);
    let chosen = menu.chosen(view);
    for i in 0..menu.len() {
        let r = keystone_menu_item_rect(game, menu, i);
        if i == chosen {
            draw_rectangle(r.x + 1.5, r.y, r.w - 3.0, r.h, faded(CYAN, 0.15));
        } else if i > 0 {
            draw_line(r.x + u * 2.0, r.y, r.x + r.w - u * 2.0, r.y, 1.0, EDGE);
        }
        let label = menu.item(i);
        let col = if i == chosen { CYAN } else { TEXT };
        text(label, r.x + u * 4.0, r.y + r.h * 0.68, r.h * 0.55, col);
    }
}

pub(super) fn slot_rect(i: usize) -> Rect {
    let sw = view::width();
    let h = render::u() * 21.0;
    Rect::new(
        sw * 0.04,
        render::bar_height() + render::u() * 19.0 + i as f32 * (h + render::u() * 2.0),
        sw * 0.92,
        h,
    )
}

pub(super) fn grid_top(game: &Game) -> f32 {
    let b = keystone_menu_button_rect(game, Menu::Sort);
    b.y + b.h + render::u() * 3.0
}

pub(super) fn cell_size() -> f32 {
    view::width() * 0.92 / 7.0
}

pub(super) fn info_rect() -> Rect {
    let sw = view::width();
    let h = render::u() * 46.0;
    Rect::new(
        sw * 0.04,
        screen_height() - render::bar_height() - h - render::u() * 2.0,
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
    Rect::new(r.x + r.w * 0.64, r.y + r.h * 0.46, r.w * 0.32, r.h * 0.22)
}

pub fn keystone_morph_rect() -> Rect {
    let r = info_rect();
    Rect::new(r.x + r.w * 0.64, r.y + r.h * 0.74, r.w * 0.32, r.h * 0.2)
}

pub(super) fn level_for_fossil(game: &Game, t: usize) -> u32 {
    game::level_for(game.specimens[t].max(1))
}

pub fn morph_colors(m: Morph) -> (Color, Color) {
    match m {
        Morph::Amber => (rgb(0xFFB048), rgb(0x0E1116)),
        Morph::Albino => (rgb(0xF2F5F8), rgb(0x0E1116)),
        Morph::Melanistic => (rgb(0x2A2F3A), TEXT),
        _ => (rgb(0x5BC8F5), rgb(0x0E1116)),
    }
}

pub(super) fn morph_badge(m: Morph, x: f32, bottom: f32, h: f32) {
    let (fill, ink) = morph_colors(m);
    let label = m.name().to_uppercase();
    let px = h * 0.8;
    let w = text_width(&label, px) + h * 0.4;
    draw_rectangle(x, bottom - h, w, h, fill);
    text(&label, x + h * 0.2, bottom - h * 0.22, px, ink);
}

pub(super) fn collection(game: &Game, view: &KeystoneView) -> Vec<usize> {
    let n = game.phy.len();
    let keep = |i: usize| view.filter.keeps(game, i);
    let mut found: Vec<usize> = (0..n).filter(|&i| game.unlocked[i] && keep(i)).collect();
    let mut fossils: Vec<usize> = (0..n).filter(|&i| game.is_fossil(i) && keep(i)).collect();
    view.sort.apply(game, &mut found);
    view.sort.apply(game, &mut fossils);
    found.extend(fossils);
    found
}

pub fn keystone_cell_at(game: &Game, view: &KeystoneView, p: Vec2) -> Option<usize> {
    let cs = cell_size();
    let top = grid_top(game) - view.scroll;
    if p.y < grid_top(game) || p.y > info_rect().y {
        return None;
    }
    let x0 = view::width() * 0.04;
    let col = ((p.x - x0) / cs).floor();
    let row = ((p.y - top) / cs).floor();
    if !(0.0..7.0).contains(&col) || row < 0.0 {
        return None;
    }
    collection(game, view)
        .get(row as usize * 7 + col as usize)
        .copied()
}

pub fn keystone_slot_at(game: &Game, p: Vec2) -> Option<usize> {
    (0..game.keystone_slots())
        .find(|&i| slot_rect(i).contains(p))
        .and_then(|i| game.keystones.get(i).copied())
}

pub fn keystones_max_scroll(game: &Game, view: &KeystoneView) -> f32 {
    let rows = collection(game, view).len().div_ceil(7) as f32;
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
    let sw = view::width();
    let top = render::bar_height();
    let px = render::u() * 7.5;
    text(
        "KEYSTONES",
        sw * 0.04,
        top + render::u() * 9.0,
        px * 1.2,
        ACCENT_WARN,
    );
    let biomes = game.active_biomes();
    let (note, note_col) = if !game.keystones_editable() {
        (
            "Locked until you open the waiting genome".to_string(),
            TEXT_DIM,
        )
    } else if biomes.is_empty() {
        (
            format!(
                "{} keystones of one biome give its bonus",
                ecology::BIOME_SET
            ),
            TEXT_DIM,
        )
    } else {
        let on: Vec<String> = biomes
            .iter()
            .map(|&b| format!("{} {}", b.name(), ecology::biome_bonus_label(b)))
            .collect();
        (on.join("  ·  "), LIME)
    };
    text(
        &note,
        sw * 0.04,
        top + render::u() * 15.5,
        fit_px(&note, sw * 0.92, px * 0.8),
        note_col,
    );

    let reports = game.keystone_reports();
    for i in 0..game.keystone_slots() {
        let r = slot_rect(i);
        match game.keystones.get(i) {
            Some(&t) => {
                let tier = game.taxon(t).eco.tier;
                let rep = reports.iter().find(|x| x.taxon == t);
                let asleep = rep.is_none_or(|x| matches!(x.status, Status::Asleep(_)));
                let col = if asleep {
                    ACCENT_WARN
                } else {
                    render::tier_color(tier)
                };
                frame(r, CARD_BG, faded(col, if asleep { 0.6 } else { 1.0 }), 1.5);
                draw_rectangle(r.x + 2.0, r.y + 2.0, r.h - 4.0, r.h - 4.0, rgb(0x050A14));
                let morph = game.edition(t);
                draw_taxon(
                    sprites,
                    game,
                    t,
                    morph,
                    vec2(r.x + r.h * 0.5, r.y + r.h * 0.5),
                    r.h * 0.8,
                    if asleep { DORMANT } else { WHITE },
                );
                if morph != Morph::None {
                    morph_badge(morph, r.x + 2.0, r.y + r.h - 2.0, r.h * 0.22);
                }
                text(
                    game.taxon(t).label(),
                    r.x + r.h * 1.15,
                    r.y + r.h * 0.42,
                    fit_px(game.taxon(t).label(), r.w * 0.5, r.h * 0.3),
                    if asleep { TEXT_DIM } else { TEXT },
                );
                let (line, line_col) = match rep {
                    Some(x) => report_line(x, false),
                    None => (String::new(), TEXT_DIM),
                };
                text(
                    &line,
                    r.x + r.h * 1.15,
                    r.y + r.h * 0.8,
                    fit_px(&line, r.w - r.h * 1.3, r.h * 0.28),
                    line_col,
                );
                let (tag, tag_col) = match rep.map(|x| &x.status) {
                    Some(Status::Active) => ("ACTIVE", LIME),
                    Some(Status::Waiting(_)) => ("WAITING", TEXT_DIM),
                    _ => ("ASLEEP", ACCENT_WARN),
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
                frame(r, rgb(0x070B12), PANEL_EDGE, 1.0);
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
    let cs = cell_size();
    let (gtop, gbot) = (grid_top(game), info_rect().y);
    let x0 = sw * 0.04;
    let shown = collection(game, view);
    if shown.is_empty() {
        text_centered(
            "No animal matches this filter",
            sw * 0.5,
            gtop + cs * 0.6,
            px * 0.85,
            LOCKED_TEXT,
        );
    }
    for (k, &t) in shown.iter().enumerate() {
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
        let tier = game.taxon(t).eco.tier;
        let dormant = game.dormant_reason(t).is_some();
        let fossil = game.is_fossil(t);
        frame(
            r,
            if equipped {
                rgb(0x12213A)
            } else if fossil {
                rgb(0x14120F)
            } else {
                CARD_BG
            },
            if sel {
                WHITE
            } else if fossil {
                rgb(0x4A443C)
            } else if equipped && dormant {
                ACCENT_WARN
            } else if equipped {
                CYAN
            } else {
                faded(render::tier_color(tier), if dormant { 0.25 } else { 0.6 })
            },
            if sel { 2.5 } else { 1.0 },
        );
        draw_taxon(
            sprites,
            game,
            t,
            game.best_morph(t),
            vec2(r.x + r.w * 0.5, r.y + r.h * 0.5),
            r.w * 0.7,
            if fossil {
                STONE
            } else if dormant {
                DORMANT
            } else {
                WHITE
            },
        );
    }
    let band = slot_rect(game.keystone_slots()).y;
    draw_rectangle(0.0, band, sw, gtop - band, render::BG);
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
            keystone_menu_button_rect(game, Menu::Sort).y - render::u() * 3.0,
            px * 0.8,
            LOCKED_TEXT,
        );
    }

    draw_keystone_menus(game, view);
    draw_rectangle(
        0.0,
        gbot - render::u(),
        sw,
        screen_height() - gbot + render::u(),
        render::BG,
    );

    let r = info_rect();
    frame(r, rgb(0x070D1C), BUTTON_EDGE, 1.5);
    match view.selected {
        Some(t) => {
            let eco = game.taxon(t).eco;
            let col = render::tier_color(eco.tier);
            let icon = vec2(r.x + r.h * 0.45, r.y + r.h * 0.42);
            assets.glow(icon, r.h * 0.45, col, 0.25);
            draw_taxon(
                sprites,
                game,
                t,
                game.best_morph(t),
                icon,
                r.h * 0.55,
                WHITE,
            );
            let tx = r.x + r.h * 0.9;
            let col_w = keystone_equip_rect().x - tx - r.w * 0.02;
            text(
                game.taxon(t).label(),
                tx,
                r.y + r.h * 0.2,
                fit_px(game.taxon(t).label(), col_w, r.h * 0.16),
                TEXT,
            );
            text(eco.tier.name(), tx, r.y + r.h * 0.36, r.h * 0.13, col);
            let px = r.h * 0.12;
            let mut lines: Vec<(String, Color)> = wrap_lines(&eco.describe(), col_w, px)
                .into_iter()
                .map(|l| (l, LIME))
                .collect();
            if let Some(d) = eco.rule.drawback() {
                lines.push((format!("But: {d}"), LOCK_RED));
            }
            for (i, (line, c)) in lines.iter().take(3).enumerate() {
                let y = r.y + r.h * (0.52 + 0.13 * i as f32);
                text(line, tx, y, fit_px(line, col_w, px), *c);
            }
            let (status, status_col) = match game.dormant_reason(t) {
                _ if game.is_fossil(t) => (
                    format!(
                        "Fossil  ·  Lv {}  ·  find it again to equip",
                        level_for_fossil(game, t)
                    ),
                    STONE_TEXT,
                ),
                Some(why) => (format!("Asleep here: {why}"), ACCENT_WARN),
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
                r.y + r.h * 0.93,
                fit_px(
                    &status,
                    keystone_morph_rect().x - sx - r.w * 0.02,
                    r.h * 0.12,
                ),
                status_col,
            );
            if game.morphs[t] != 0 {
                let b = keystone_morph_rect();
                let can = game.phase() == Phase::Shape;
                let m = game.edition(t);
                frame(b, PANEL, if can { CYAN } else { PANEL_EDGE }, 1.0);
                let name = if m == Morph::None {
                    "NONE".to_string()
                } else {
                    m.name().to_uppercase()
                };
                let label = if game.edition_is_best(t) {
                    format!("MORPH: BEST ({name})")
                } else {
                    format!("MORPH: {name}")
                };
                text_centered(
                    &label,
                    b.x + b.w * 0.5,
                    b.y + b.h * 0.7,
                    fit_px(&label, b.w * 0.9, b.h * 0.5),
                    if can { TEXT } else { LOCKED_TEXT },
                );
            }
            let equipped = game.keystones.contains(&t);
            let can = game.keystones_editable()
                && game.unlocked[t]
                && (equipped || game.keystones.len() < game.keystone_slots());
            let b = keystone_equip_rect();
            frame(
                b,
                if can {
                    faded(ACCENT_OK, 0.15)
                } else {
                    PANEL_LOCKED
                },
                if can { ACCENT_OK } else { PANEL_EDGE },
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
    draw_keystone_menu_list(game, view);
    draw_hud(game, now, assets);
}
