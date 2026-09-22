//! All drawing. The map view projects world-space boxes through the camera;
//! the spiral view is already in screen space by construction.

use macroquad::prelude::*;

use crate::game::Game;
use crate::layout::Layout;
use crate::spiral::{self, Bead, Frame};
use crate::sprites::{self, Sprites};
use crate::tree::Group;
use crate::upgrades::{self, Kind};
use crate::view::Camera;

/// Glyphs are rasterised once at this size and scaled, which keeps the font
/// atlas to a single entry however far the player zooms.
const FONT_PX: u16 = 48;
/// Taxon name size in the map view, in world units.
pub const LABEL_PX: f32 = 20.0;
const SUBLABEL_PX: f32 = 13.0;

const BG: Color = rgb(0x0E1116);
const PANEL: Color = rgb(0x161C24);
const PANEL_LOCKED: Color = rgb(0x11151B);
const EDGE: Color = rgb(0x2A323D);
const TEXT: Color = rgb(0xE6EAF0);
const TEXT_DIM: Color = rgb(0x97A3B5);
const LOCKED_BORDER: Color = rgb(0x3E4857);
const LOCKED_TEXT: Color = rgb(0x5A6678);
const ACCENT_OK: Color = rgb(0x5CC26B);
const ACCENT_DNA: Color = rgb(0x5BC8F5);
const ACCENT_WARN: Color = rgb(0xE8A33D);
const ACCENT_CAP: Color = rgb(0x9B7EDE);
const ACCENT_STORAGE: Color = rgb(0x4FD1C5);
const ACCENT_TAP: Color = rgb(0xE85EA0);
const ACCENT_VIGOR: Color = rgb(0xF0C34D);
const TRACK: Color = rgb(0x20272F);
/// Multiply tint for an undiscovered species' sprite: dark enough to read as
/// a silhouette tease, not a preview of its real colours.
const SILHOUETTE: Color = Color::new(0.09, 0.09, 0.11, 1.0);
const HUD_BG: Color = Color::new(0.055, 0.067, 0.086, 0.92);

const fn rgb(hex: u32) -> Color {
    Color::new(
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
        1.0,
    )
}

fn group_color(g: Group) -> Color {
    match g {
        Group::Backbone => rgb(0x8A94A3),
        Group::Basal => rgb(0x3FB8AF),
        Group::Spiralia => rgb(0x9B7EDE),
        Group::Ecdysozoa => rgb(0xE8A33D),
        Group::Deuterostome => rgb(0xE86A5E),
        Group::Fish => rgb(0x4A9FE0),
        Group::Tetrapod => rgb(0x5CC26B),
        Group::Reptile => rgb(0xE05E8A),
        Group::Mammal => rgb(0xF0C674),
    }
}

fn faded(c: Color, a: f32) -> Color {
    Color {
        a: c.a * a.clamp(0.0, 1.0),
        ..c
    }
}

// --- text -------------------------------------------------------------------

/// Beyond the largest named unit (`Qa`, 10^15), there is no more suffix to
/// promote into -- rather than ever print an untiered "1234Qa", fall back to
/// plain scientific notation, which has no ceiling. Flat, additive upgrades
/// (see `upgrades.rs`) keep every number here comfortably small in practice,
/// but nothing bounds a long-enough idle session or a very leveled species,
/// so every huge-number path needs a real answer, not a wrong-looking one.
fn scientific(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return format!("{v:.0}");
    }
    let exp = v.abs().log10().floor() as i32;
    let mantissa = v / 10f64.powi(exp);
    format!("{mantissa:.2}e{exp}")
}

/// Compact number for the HUD: 940, 1.24K, 87.3M.
/// For a rate (DNA/s): keeps two decimals below 1 so an early "0.20/s"
/// doesn't just read as a flat, alarming "0".
pub fn compact(v: f64) -> String {
    const UNITS: [&str; 6] = ["", "K", "M", "B", "T", "Qa"];
    let (mut scaled, mut unit) = (v, 0);
    while scaled >= 1000.0 && unit < UNITS.len() - 1 {
        scaled /= 1000.0;
        unit += 1;
    }
    if unit == UNITS.len() - 1 && scaled >= 1000.0 {
        return scientific(v);
    }
    if unit == 0 {
        if scaled < 1.0 {
            format!("{scaled:.2}")
        } else {
            format!("{scaled:.1}").trim_end_matches(".0").to_string()
        }
    } else if scaled < 10.0 {
        format!("{scaled:.2}{}", UNITS[unit])
    } else if scaled < 100.0 {
        format!("{scaled:.1}{}", UNITS[unit])
    } else {
        format!("{scaled:.0}{}", UNITS[unit])
    }
}

/// For a point amount (the DNA pool, its cap, an attempt's cost): whole
/// numbers only, never a decimal point, however small the value.
pub fn compact_int(v: f64) -> String {
    const UNITS: [&str; 6] = ["", "K", "M", "B", "T", "Qa"];
    let orig = v.max(0.0);
    let mut scaled = orig;
    let mut unit = 0;
    // The 999.5 threshold (not 1000) promotes a value that would round up to
    // the next tier before it does, so "1000" never appears untiered.
    while scaled >= 999.5 && unit < UNITS.len() - 1 {
        scaled /= 1000.0;
        unit += 1;
    }
    if unit == UNITS.len() - 1 && scaled >= 999.5 {
        return scientific(orig);
    }
    format!("{:.0}{}", scaled.round(), UNITS[unit])
}

/// Width of `s` when drawn at `px`, in the same units as `px`.
pub fn text_width(s: &str, px: f32) -> f32 {
    measure_text(s, None, FONT_PX, px / FONT_PX as f32).width
}

fn text(s: &str, x: f32, baseline_y: f32, px: f32, color: Color) {
    draw_text_ex(
        s,
        x,
        baseline_y,
        TextParams {
            font: None,
            font_size: FONT_PX,
            font_scale: px / FONT_PX as f32,
            color,
            ..Default::default()
        },
    );
}

fn text_centered(s: &str, cx: f32, baseline_y: f32, px: f32, color: Color) {
    text(s, cx - text_width(s, px) * 0.5, baseline_y, px, color);
}

// --- spiral view ------------------------------------------------------------

/// How long the card's evolving flash-and-punch plays, in seconds. `main`
/// counts a timer down from this; `draw_focus_card` just needs how much is
/// left, so it can ease the effect out over the same window every time.
pub const EVOLVE_ANIM_SECONDS: f32 = 0.8;

pub fn draw_spiral(
    game: &Game,
    frame: &Frame,
    t: f32,
    last: f32,
    sprites: &Sprites,
    evolve_anim: f32,
    fact: Option<&str>,
) {
    clear_background(BG);
    draw_coil(frame, t, last);
    draw_spurs(game, frame, sprites);

    // Far to near, so the focus bead lands on top.
    let mut beads: Vec<&Bead> = frame.beads.iter().collect();
    beads.sort_by(|a, b| b.d.abs().partial_cmp(&a.d.abs()).unwrap());
    for bead in beads {
        draw_bead(game, frame, bead, sprites);
    }

    draw_hud(game, "MAP");
    if let Some(f) = fact {
        draw_fact_panel(f);
    }
    // Drawn last, right above the bottom bar it now sits next to.
    draw_focus_card(game, frame.focus, frame.card, sprites, evolve_anim);
    draw_bottom_bar(game, None);
}

fn draw_coil(frame: &Frame, t: f32, last: f32) {
    for pair in spiral::curve(frame.center, frame.k, t, last).windows(2) {
        let (a, da) = pair[0];
        let (b, _) = pair[1];
        // Thicker near the focus: the lineage reads as coming toward you.
        let w = (frame.k * 0.022 * spiral::scale_at(da)).max(1.0);
        draw_line(a.x, a.y, b.x, b.y, w, faded(EDGE, spiral::fade_at(da)));
    }
}

/// The accent colour for any rectangle that represents `taxon` -- the card,
/// a map node, a spiral chip/bead, all of it. A discovered species is
/// tinted with the colour of whatever kind its nearest resolved category
/// ancestor gives it (`kind_color`), not its clade colour any more, so the
/// colour alone shows what it's powering. A category, or anything still
/// locked, keeps the clade colour it always had.
fn accent_of(game: &Game, taxon: usize) -> Color {
    if !game.unlocked[taxon] {
        return LOCKED_BORDER;
    }
    if game.taxon(taxon).group != Group::Backbone {
        if let Some(kind) = game.nearest_category_kind(taxon) {
            return kind_color(kind);
        }
    }
    group_color(game.taxon(taxon).group)
}

/// A label box on the coil. Callers own the geometry; this just paints it.
struct Chip<'a> {
    pos: Vec2,
    half: Vec2,
    label: &'a str,
    px: f32,
    accent: Color,
    unlocked: bool,
    fade: f32,
    border: f32,
    taxon: usize,
}

/// The chip icon's diameter as a fraction of the box height -- must match
/// the `* 1.3` reservation in `spiral::Frame::build`'s `chip` closure.
const CHIP_ICON: f32 = 0.72;

/// A horizontal fill bar, for the DNA pool and for mutation pressure.
fn draw_meter(x: f32, y: f32, w: f32, h: f32, fraction: f32, fill: Color) {
    draw_rectangle(x, y, w, h, TRACK);
    let f = fraction.clamp(0.0, 1.0);
    if f > 0.0 {
        draw_rectangle(x, y, w * f, h, fill);
    }
}

fn draw_chip(game: &Game, sprites: &Sprites, c: Chip) {
    let (x, y) = (c.pos.x - c.half.x, c.pos.y - c.half.y);
    let (w, h) = (c.half.x * 2.0, c.half.y * 2.0);
    draw_rectangle(
        x,
        y,
        w,
        h,
        faded(if c.unlocked { PANEL } else { PANEL_LOCKED }, c.fade),
    );
    draw_rectangle_lines(x, y, w, h, c.border, faded(c.accent, c.fade));

    // A small icon to the left, same convention as the map and focus card --
    // locked chips get the same dark silhouette tease.
    let icon = h * CHIP_ICON;
    let icon_center = Vec2::new(x + h * 0.14 + icon * 0.5, c.pos.y);
    let group = game.taxon(c.taxon).group;
    let anim = sprites::pose(group, c.taxon, get_time());
    let tint = faded(if c.unlocked { WHITE } else { SILHOUETTE }, c.fade);
    sprites::draw_sprite(sprites.get(c.taxon), icon_center, icon, anim, tint);

    let text_x0 = x + h * 0.14 + icon + h * 0.10;
    let text_w = (x + w - h * 0.12 - text_x0).max(1.0);
    text(
        c.label,
        text_x0,
        c.pos.y + c.px * 0.34,
        fit_px(c.label, text_w, c.px),
        faded(if c.unlocked { TEXT } else { LOCKED_TEXT }, c.fade),
    );
}

/// The branches leaving the lineage. These are what make the coil a tree.
fn draw_spurs(game: &Game, frame: &Frame, sprites: &Sprites) {
    let px = spiral::label_px(frame.k);

    for spur in &frame.spurs {
        let fade = spiral::fade_at(spur.d);
        let accent = accent_of(game, spur.taxon);
        let w = (frame.k * 0.014 * spiral::scale_at(spur.d)).max(1.0);
        draw_line(
            spur.from.x,
            spur.from.y,
            spur.pos.x,
            spur.pos.y,
            w,
            faded(accent, fade * 0.5),
        );

        match &spur.label {
            Some(label) => draw_chip(
                game,
                sprites,
                Chip {
                    pos: spur.pos,
                    half: spur.half,
                    label,
                    px: px * spiral::scale_at(spur.d) * 0.92,
                    accent,
                    unlocked: game.unlocked[spur.taxon],
                    fade,
                    border: 1.5,
                    taxon: spur.taxon,
                },
            ),
            None => {
                let r = spiral::bead_radius(frame.k, spiral::scale_at(spur.d) * 0.7);
                draw_circle(spur.pos.x, spur.pos.y, r, faded(accent, fade * 0.7));
            }
        }
    }
}

fn draw_bead(game: &Game, frame: &Frame, bead: &Bead, sprites: &Sprites) {
    let fade = spiral::fade_at(bead.d);
    let accent = accent_of(game, bead.taxon);
    let is_focus = bead.d.abs() < 0.5;

    let Some(label) = &bead.label else {
        // Unlocked and far from the focus: a tiny animated icon rather than a
        // flat dot. Still locked: keep the plain dot, so the coil does not
        // spoil undiscovered shapes at a glance from across the tree.
        if game.unlocked[bead.taxon] {
            let group = game.taxon(bead.taxon).group;
            let size = spiral::bead_radius(frame.k, bead.scale) * 3.4;
            let anim = sprites::pose(group, bead.taxon, get_time());
            sprites::draw_sprite(
                sprites.get(bead.taxon),
                bead.pos,
                size,
                anim,
                faded(WHITE, fade),
            );
        } else {
            let r = spiral::bead_radius(frame.k, bead.scale);
            draw_circle(bead.pos.x, bead.pos.y, r, faded(accent, fade * 0.85));
        }
        return;
    };

    draw_chip(
        game,
        sprites,
        Chip {
            pos: bead.pos,
            half: bead.half,
            label,
            px: spiral::label_px(frame.k) * bead.scale,
            accent,
            unlocked: game.unlocked[bead.taxon],
            fade,
            border: if is_focus { 3.0 } else { 1.5 },
            taxon: bead.taxon,
        },
    );
}

/// Shrinks `px` until `s` fits in `max_w`.
fn fit_px(s: &str, max_w: f32, px: f32) -> f32 {
    let w = text_width(s, px);
    if w <= max_w {
        px
    } else {
        px * max_w / w
    }
}

/// What a taxon's meta row shows on the right -- a species gives whatever
/// bonus its nearest category kind is (`Game::species_bonus`), not DNA/s
/// unconditionally any more. A category (or, in principle, a species with no
/// resolved ancestor kind) falls through to the old "+DNA/s" shape, which is
/// always 0 for a category -- categories never produced DNA either way.
fn species_earn_label(game: &Game, taxon: usize) -> String {
    match game.species_bonus(taxon) {
        Some((Kind::Rate, amount)) => format!("+{}/s", compact(amount)),
        Some((Kind::Cost, amount)) => format!("-{} cost", compact(amount)),
        Some((Kind::Chance, amount)) => format!("+{:.0}% chance", amount * 100.0),
        Some((Kind::Cap, amount)) => format!("+{} cap", compact(amount)),
        Some((Kind::Tap, amount)) => format!("+{} tap", compact(amount)),
        Some((Kind::Storage, amount)) => format!("+{} overflow", compact(amount)),
        Some((Kind::Vigor, amount)) => {
            let n = amount.round() as u32;
            format!("+{n} start lvl{}", if n == 1 { "" } else { "s" })
        }
        None => format!("+{}/s", compact(game.output(taxon))),
    }
}

/// Draws the species/category info card at `card`, for `taxon`. Used both
/// for the spiral's focus (where `evolve_anim` punches it on a fresh
/// Evolve) and the map's tap-to-inspect overlay (where nothing evolved this
/// specific taxon, so `evolve_anim` is always 0 there).
fn draw_focus_card(game: &Game, taxon: usize, card: Rect, sprites: &Sprites, evolve_anim: f32) {
    let c = card;
    let unlocked = game.unlocked[taxon];
    let accent = accent_of(game, taxon);
    let pulse = game.pulse[taxon];
    let inner = c.w * 0.88;

    // 0 at the start of the flash, 1 by the end -- everything below eases
    // out over this same window, so the flash, the punch and the border all
    // settle together instead of drifting apart.
    let evolving = evolve_anim > 0.0;
    let t = 1.0 - (evolve_anim / EVOLVE_ANIM_SECONDS).clamp(0.0, 1.0);

    draw_rectangle(
        c.x,
        c.y,
        c.w,
        c.h,
        if unlocked { PANEL } else { PANEL_LOCKED },
    );
    draw_rectangle(c.x, c.y, c.h * 0.045, c.h, accent);
    draw_rectangle_lines(
        c.x,
        c.y,
        c.w,
        c.h,
        2.5 + pulse * 10.0,
        if pulse > 0.0 { WHITE } else { accent },
    );

    // The sprite is the hero of the card: a big animated icon across the top
    // band, with the name/stats stack underneath. Evolving adds a quick
    // punch of scale on top of its usual idle motion.
    let cx = c.x + c.w * 0.5;
    let sprite_center = Vec2::new(cx, c.y + c.h * 0.27);
    let sprite_size = c.h * 0.42;
    let mut anim = sprites::pose(game.taxon(taxon).group, taxon, get_time());
    if evolving {
        let punch = 1.0 + 0.5 * (std::f32::consts::PI * t).sin();
        anim.scale_x *= punch;
        anim.scale_y *= punch;
    }

    if !unlocked {
        // The real shape, as a dark tease -- Pokedex-style -- rather than
        // nothing at all: the mystery is what it looks like, not whether it
        // exists.
        sprites::draw_sprite(
            sprites.get(taxon),
            sprite_center,
            sprite_size,
            anim,
            SILHOUETTE,
        );
        text_centered("? ? ?", cx, c.y + c.h * 0.62, c.h * 0.145, LOCKED_TEXT);
        text_centered(
            "undiscovered",
            cx,
            c.y + c.h * 0.755,
            c.h * 0.09,
            LOCKED_TEXT,
        );
        return;
    }

    sprites::draw_sprite(sprites.get(taxon), sprite_center, sprite_size, anim, WHITE);
    if evolving {
        // A flash of light at the moment of commitment, fading as it settles.
        draw_rectangle(c.x, c.y, c.w, c.h, faded(WHITE, 0.35 * (1.0 - t)));
    }

    let taxon_ref = game.taxon(taxon);
    let name_px = fit_px(taxon_ref.name, inner, c.h * 0.155);
    text_centered(taxon_ref.name, cx, c.y + c.h * 0.585, name_px, TEXT);
    text_centered(
        taxon_ref.clade,
        cx,
        c.y + c.h * 0.685,
        fit_px(taxon_ref.clade, inner, c.h * 0.085),
        accent,
    );
    // Level on the left, what it earns on the right, its age between them.
    // A category has no level of its own worth showing here -- its own
    // progression is the upgrade shop's level, not this one (see
    // `Game::upgrade_level`), and showing both would just read as two
    // different numbers competing for the same word.
    let meta_px = c.h * 0.08;
    let pad = c.h * 0.14;
    let meta_y = c.y + c.h * 0.80;
    if taxon_ref.group != Group::Backbone {
        text(
            &format!("Lv {}", game.level[taxon]),
            c.x + pad,
            meta_y,
            meta_px,
            TEXT,
        );
    }
    text_centered(&taxon_ref.age_label(), cx, meta_y, meta_px, TEXT_DIM);
    // What this taxon actually gives -- a species no longer means DNA/s
    // unconditionally, it means whatever its nearest category's kind is.
    let earns = species_earn_label(game, taxon);
    let earns_px = fit_px(&earns, inner * 0.42, meta_px);
    text(
        &earns,
        c.x + c.w - pad - text_width(&earns, earns_px),
        meta_y,
        earns_px,
        accent,
    );

    // A category's card stops here: its payoff is the choice it already
    // made on discovery, not a per-roll percentage -- that reads as a
    // distraction, not useful information, once the choice is resolved.
    if taxon_ref.group == Group::Backbone {
        if taxon_ref.children.is_empty() {
            text_centered(
                "LINEAGE COMPLETE",
                cx,
                c.y + c.h * 0.935,
                c.h * 0.075,
                TEXT_DIM,
            );
        }
        return;
    }

    // A species keeps the advanced-find odds preview it always had, if
    // there is still a branch under it to roll on -- Evolve always
    // discovers *something* from here now, this is only about how much
    // further it might reach in the same roll (see `Game::evolve`).
    if !taxon_ref.children.is_empty() {
        draw_meter(
            c.x + pad,
            c.y + c.h * 0.855,
            c.w - pad * 2.0,
            c.h * 0.03,
            game.pressure_fraction(taxon),
            ACCENT_WARN,
        );
        let caption = format!(
            "{:.0}% chance of a more advanced find",
            game.unlock_chance(taxon) * 100.0
        );
        text_centered(
            &caption,
            cx,
            c.y + c.h * 0.905,
            fit_px(&caption, inner, c.h * 0.06),
            TEXT_DIM,
        );
    }

    // ...and, either way, its own tap-level bar: tapping the card fills it
    // (`Game::tap_level`), and it levels up once full -- a direct, always-
    // available channel independent of the Evolve gate, so a leaf species
    // with nothing left to discover still has something to do here. The
    // hint sits right on top of the bar it explains, not up near the name.
    let hint = "tap to level up";
    text_centered(
        hint,
        cx,
        c.y + c.h * 0.925,
        fit_px(hint, inner, c.h * 0.05),
        TEXT_DIM,
    );
    draw_meter(
        c.x + pad,
        c.y + c.h * 0.945,
        c.w - pad * 2.0,
        c.h * 0.035,
        game.tap_progress_fraction(taxon),
        accent,
    );
}

// --- map view ---------------------------------------------------------------

pub fn draw_map(
    game: &Game,
    layout: &Layout,
    cam: &Camera,
    sprites: &Sprites,
    fact: Option<&str>,
    card_focus: Option<usize>,
) {
    clear_background(BG);
    draw_edges(game, layout, cam);
    draw_nodes(game, layout, cam, sprites);
    draw_hud(game, "SPIRAL");
    if let Some(f) = fact {
        draw_fact_panel(f);
    }
    // Tapping a node shows its card right here rather than jumping to the
    // spiral -- nothing evolved this specific taxon just by looking at it,
    // so unlike the spiral's own card, this one never punches on an Evolve.
    // `main` keeps `card_focus` in sync with whatever just evolved, so
    // looking at the map when Evolve fires shows what happened here too.
    // Drawn last, right above the bottom bar it now sits next to.
    if let Some(taxon) = card_focus {
        draw_focus_card(game, taxon, spiral::card_rect(), sprites, 0.0);
    }
    draw_bottom_bar(game, None);
}

fn draw_edges(game: &Game, layout: &Layout, cam: &Camera) {
    let thickness = (1.5 * cam.zoom).max(1.0);

    for i in 0..game.phy.len() {
        let (Some(child_box), Some(parent)) = (layout.get(i), game.taxon(i).parent) else {
            continue;
        };
        let Some(parent_box) = layout.get(parent) else {
            continue;
        };

        let from = cam.world_to_screen(parent_box.right());
        let to = cam.world_to_screen(child_box.left());
        let mid_x = (from.x + to.x) * 0.5;

        // Orthogonal elbow: across, down, across.
        draw_line(from.x, from.y, mid_x, from.y, thickness, EDGE);
        draw_line(mid_x, from.y, mid_x, to.y, thickness, EDGE);
        draw_line(mid_x, to.y, to.x, to.y, thickness, EDGE);
    }
}

fn draw_nodes(game: &Game, layout: &Layout, cam: &Camera, sprites: &Sprites) {
    let (sw, sh) = (screen_width(), screen_height());

    for i in 0..game.phy.len() {
        let Some(b) = layout.get(i) else { continue };

        let center = cam.world_to_screen(b.center);
        let (w, h) = (b.w * cam.zoom, b.h * cam.zoom);
        let (x, y) = (center.x - w * 0.5, center.y - h * 0.5);

        // Cheap cull -- the full tree is far wider than any phone screen.
        if x + w < 0.0 || x > sw || y + h < 0.0 || y > sh {
            continue;
        }

        let accent = accent_of(game, i);

        if game.unlocked[i] {
            draw_rectangle(x, y, w, h, PANEL);
            draw_rectangle_lines(x, y, w, h, 1.5 * cam.zoom.max(0.5), accent);
            // A thicker left edge reads as a clade stripe without extra geometry.
            draw_rectangle(x, y, 3.0 * cam.zoom, h, accent);

            // A small icon to the left, so the map is not text-only either.
            let icon = h * 0.82;
            let icon_center = Vec2::new(x + h * 0.20 + icon * 0.5, center.y);
            let anim = sprites::pose(game.taxon(i).group, i, get_time());
            sprites::draw_sprite(sprites.get(i), icon_center, icon, anim, WHITE);

            let text_x0 = x + h * 0.20 + icon + h * 0.12;
            let name_px = LABEL_PX * cam.zoom;
            let name_w = (x + w - h * 0.15 - text_x0).max(1.0);
            let name = game.taxon(i).name;
            text(
                name,
                text_x0,
                center.y + name_px * 0.05,
                fit_px(name, name_w, name_px),
                TEXT,
            );
            let sub_text = sublabel(game, i);
            let sub_px = SUBLABEL_PX * cam.zoom;
            text(
                &sub_text,
                text_x0,
                center.y + name_px * 0.4 + sub_px,
                fit_px(&sub_text, name_w, sub_px),
                TEXT_DIM,
            );
        } else {
            draw_rectangle(x, y, w, h, PANEL_LOCKED);
            draw_rectangle_lines(x, y, w, h, 1.0 * cam.zoom.max(0.5), LOCKED_BORDER);

            // The real shape, as a dark tease -- same convention as the
            // focus card and the upgrade rows -- rather than text alone.
            let icon = h * 0.82;
            let icon_center = Vec2::new(x + h * 0.20 + icon * 0.5, center.y);
            let anim = sprites::pose(game.taxon(i).group, i, get_time());
            sprites::draw_sprite(sprites.get(i), icon_center, icon, anim, SILHOUETTE);

            let text_x0 = x + h * 0.20 + icon + h * 0.12;
            let text_w = (x + w - h * 0.15 - text_x0).max(1.0);
            text_centered(
                "? ? ?",
                text_x0 + text_w * 0.5,
                center.y + LABEL_PX * cam.zoom * 0.35,
                fit_px("? ? ?", text_w, LABEL_PX * cam.zoom),
                LOCKED_TEXT,
            );
        }
    }
}

/// `"740 Ma"`, or `"740 Ma  32%"` while the lineage still has something to
/// give, prefixed with `"Lv N  ·  "` for a species -- a category leaves its
/// own level off here too, same as the focus card (see `draw_focus_card`).
fn sublabel(game: &Game, i: usize) -> String {
    let age = game.taxon(i).age_label();
    let has_locked_kids = game.taxon(i).children.iter().any(|&c| !game.unlocked[c]);
    let base = if has_locked_kids {
        format!("{age}  {:.0}%", game.unlock_chance(i) * 100.0)
    } else {
        age
    };
    if game.taxon(i).group == Group::Backbone {
        base
    } else {
        format!("Lv {}  ·  {base}", game.level[i])
    }
}

// --- shared chrome ----------------------------------------------------------

/// The mode toggle in the top bar. Main hit-tests this before anything else.
pub fn mode_button() -> Rect {
    let sw = screen_width();
    let bar_h = bar_height();
    let w = bar_h * 1.9;
    Rect::new(sw - w - bar_h * 0.25, bar_h * 0.18, w, bar_h * 0.64)
}

pub fn bar_height() -> f32 {
    // Two lines of text, so taller than a plain title bar.
    (screen_height() * 0.085).clamp(56.0, 92.0)
}

fn draw_hud(game: &Game, button_label: &str) {
    let sw = screen_width();
    let bar_h = bar_height();
    let px = bar_h * 0.34;

    draw_rectangle(0.0, 0.0, sw, bar_h, HUD_BG);
    draw_line(0.0, bar_h, sw, bar_h, 1.0, EDGE);

    // Line 1: the pool. The whole session is about spending this. A
    // `Storage` upgrade can push `dna` past `pool_cap` (see
    // `Game::overflow_cap`) -- that shows up right here as the first number
    // outgrowing the second, no separate badge needed.
    let pool = format!(
        "{} / {} DNA",
        compact_int(game.dna),
        compact_int(game.pool_cap())
    );
    text(&pool, px * 0.7, bar_h * 0.46, px, TEXT);

    // Line 2: where it comes from, and how far through the tree you are.
    let sub = format!(
        "+{}/s   ·   {} / {} species",
        compact(game.rate()),
        game.discovered(),
        game.phy.len()
    );
    text(&sub, px * 0.7, bar_h * 0.84, hud_subline_px(), TEXT_DIM);

    // The pool again, as a bar along the bottom edge of the HUD.
    draw_meter(0.0, bar_h - 3.0, sw, 3.0, game.pool_fraction(), ACCENT_DNA);

    let b = mode_button();
    draw_rectangle(b.x, b.y, b.w, b.h, PANEL);
    draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.5, TEXT_DIM);
    text_centered(
        button_label,
        b.x + b.w * 0.5,
        b.y + b.h * 0.68,
        px * 0.85,
        TEXT,
    );
}

// --- "did you know" panel (spiral + map, while charging) ---------------------
//
// A companion for the wait, not a permanent fixture: it only shows while the
// pool is charging (there is nothing else to do until Evolve is ready), on
// both of the screens you'd actually be browsing on -- not the upgrades tab,
// which already has its own list to read. Sits right under the top bar, out
// of the way of the coil/card below it -- see `spiral::card_rect`, which
// lives at the opposite (bottom) end of the screen now. A tap dismisses it
// outright rather than cycling through facts; the next charge cycle picks a
// fresh one.

/// Screen-space rect of the fact panel. The one source of truth for both
/// drawing it and hit-testing a tap on it.
pub fn fact_panel_rect() -> Rect {
    let sw = screen_width();
    let h = (screen_height() * 0.13).clamp(100.0, 190.0);
    Rect::new(sw * 0.06, bar_height() + 10.0, sw * 0.88, h)
}

/// Whether there is anything for the fact panel to show right now: only
/// while the pool is still charging -- once Evolve is ready, that is what
/// deserves the attention instead. `main` combines this with whether the
/// player already dismissed this charge cycle's fact.
pub fn should_show_fact(game: &Game) -> bool {
    !game.can_evolve()
}

/// The size of the top bar's "+X/s · Y/Z species" line -- the fact panel's
/// body text matches it rather than sizing off its own box, per request.
fn hud_subline_px() -> f32 {
    bar_height() * 0.34 * 0.72
}

/// Breaks `s` into lines that each fit within `max_w` at `px`, wrapping at
/// word boundaries -- as many lines as it takes, unlike a single `fit_px`
/// shrink, which would otherwise crush a long fact down to an illegible size.
fn wrap_lines(s: &str, max_w: f32, px: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in s.split(' ') {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if text_width(&candidate, px) > max_w && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn draw_fact_panel(fact: &str) {
    let r = fact_panel_rect();
    draw_rectangle(r.x, r.y, r.w, r.h, faded(PANEL, 0.95));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, faded(ACCENT_DNA, 0.7));

    let label_px = (r.h * 0.11).clamp(14.0, 20.0);
    text_centered(
        "DID YOU KNOW?",
        r.x + r.w * 0.5,
        r.y + label_px * 1.6,
        label_px,
        ACCENT_DNA,
    );

    // Shrinks the font only as far as it takes to fit the box, rather than
    // committing to one size and hoping -- a long fact gets more, smaller
    // lines before it ever gets illegibly tiny ones. Starts at the same size
    // as the top bar's "+X/s · Y/Z species" line, per request, not its own
    // box-derived size.
    let max_w = r.w * 0.88;
    let header_h = label_px * 2.8;
    let hint_h = label_px * 1.6;
    let mut body_px = hud_subline_px();
    let mut lines = wrap_lines(fact, max_w, body_px);
    while lines.len() as f32 * body_px * 1.35 > r.h - header_h - hint_h && body_px > 12.0 {
        body_px -= 1.0;
        lines = wrap_lines(fact, max_w, body_px);
    }

    let line_h = body_px * 1.35;
    let total_h = lines.len() as f32 * line_h;
    let start_y =
        r.y + header_h + ((r.h - header_h - hint_h - total_h).max(0.0)) * 0.5 + body_px * 0.8;
    for (i, line) in lines.iter().enumerate() {
        text_centered(
            line,
            r.x + r.w * 0.5,
            start_y + i as f32 * line_h,
            body_px,
            TEXT_DIM,
        );
    }

    let hint_px = (label_px * 0.62).max(11.0);
    text_centered(
        "tap to dismiss",
        r.x + r.w * 0.5,
        r.y + r.h - hint_px * 0.7,
        hint_px,
        faded(TEXT_DIM, 0.6),
    );
}

// --- bottom bar ---------------------------------------------------------------

/// Side tab labels, flanking the Evolve button in the middle. `Leaderboard`
/// is a placeholder for a screen that does not exist yet -- drawn dim and
/// untappable, per `BOTTOM_TABS_ENABLED`, rather than left out and re-added
/// later once the layout is already familiar.
pub const BOTTOM_TABS: &[&str] = &["Upgrades", "Leaderboard"];
const BOTTOM_TABS_ENABLED: &[bool] = &[true, false];

/// Screen-space rect of the centred Evolve button, between the side tabs.
pub fn bottom_evolve_rect() -> Rect {
    let sw = screen_width();
    let bar_h = bar_height();
    let w = (sw * 0.36).min(220.0);
    Rect::new((sw - w) * 0.5, screen_height() - bar_h, w, bar_h)
}

/// Screen-space rect of side tab `i`: 0 is everything left of Evolve, the
/// rest split whatever room is left on the right.
pub fn bottom_tab_rect(i: usize) -> Rect {
    let sw = screen_width();
    let bar_h = bar_height();
    let y = screen_height() - bar_h;
    let evolve = bottom_evolve_rect();

    if i == 0 {
        return Rect::new(0.0, y, evolve.x, bar_h);
    }
    let right_n = (BOTTOM_TABS.len() - 1) as f32;
    let right_w = (sw - evolve.x - evolve.w) / right_n;
    Rect::new(
        evolve.x + evolve.w + (i - 1) as f32 * right_w,
        y,
        right_w,
        bar_h,
    )
}

/// Persistent bottom navigation, drawn on every screen: the Upgrades tab,
/// Evolve centred between it and whatever comes next, and that placeholder
/// itself. `active` is the tab index for the screen currently showing, or
/// `None` on the spiral/map (they are not tabs -- the top-right button
/// handles those).
fn draw_bottom_bar(game: &Game, active: Option<usize>) {
    let sw = screen_width();
    let bar_h = bar_height();
    let y = screen_height() - bar_h;

    draw_rectangle(0.0, y, sw, bar_h, HUD_BG);
    draw_line(0.0, y, sw, y, 1.0, EDGE);

    for (i, label) in BOTTOM_TABS.iter().enumerate() {
        let r = bottom_tab_rect(i);
        let enabled = BOTTOM_TABS_ENABLED[i];
        let is_active = active == Some(i);
        let color = if !enabled {
            faded(TEXT_DIM, 0.45)
        } else if is_active {
            ACCENT_DNA
        } else {
            TEXT_DIM
        };
        if is_active {
            draw_rectangle(r.x, r.y, r.w, 3.0, ACCENT_DNA);
        }
        text_centered(label, r.x + r.w * 0.5, r.y + r.h * 0.6, bar_h * 0.26, color);
    }

    // Evolve: the one action the whole session is paced around, so it gets
    // the centre slot and a state that reads at a glance -- a percentage
    // while charging, the word itself the moment it means something. How
    // many charges that covers lives next to the DNA total in the HUD
    // instead (see `draw_hud`), not crowded onto this button.
    let e = bottom_evolve_rect();
    let ready = game.can_evolve();
    let (ey, eh) = (e.y + 4.0, e.h - 8.0);
    draw_rectangle(
        e.x,
        ey,
        e.w,
        eh,
        if ready { faded(ACCENT_OK, 0.16) } else { PANEL },
    );
    draw_rectangle_lines(
        e.x,
        ey,
        e.w,
        eh,
        2.0,
        if ready { ACCENT_OK } else { TEXT_DIM },
    );
    let label = if ready {
        "EVOLVE".to_string()
    } else {
        format!("{:.0}%", game.pool_fraction() * 100.0)
    };
    text_centered(
        &label,
        e.x + e.w * 0.5,
        e.y + e.h * 0.62,
        bar_h * 0.30,
        if ready { ACCENT_OK } else { TEXT_DIM },
    );
}

// --- upgrades screen ---------------------------------------------------------

const UPGRADE_ROW_H: f32 = 84.0;
const UPGRADE_LIST_TOP_PAD: f32 = 66.0;

/// Screen-space rect of row `row_i`, at the given scroll offset. The one
/// source of truth for that row's geometry -- both `draw_upgrade_row` and
/// `main`'s tap hit-testing call this, so they can never drift apart.
pub fn upgrade_row_rect(row_i: usize, scroll: f32) -> Rect {
    let sw = screen_width();
    let pad = sw * 0.05;
    let row_y = bar_height() + UPGRADE_LIST_TOP_PAD + row_i as f32 * UPGRADE_ROW_H - scroll;
    Rect::new(pad, row_y + 6.0, sw - pad * 2.0, UPGRADE_ROW_H - 10.0)
}

fn kind_color(kind: Kind) -> Color {
    match kind {
        Kind::Rate => ACCENT_OK,
        Kind::Cost => ACCENT_DNA,
        Kind::Chance => ACCENT_WARN,
        Kind::Cap => ACCENT_CAP,
        Kind::Storage => ACCENT_STORAGE,
        Kind::Tap => ACCENT_TAP,
        Kind::Vigor => ACCENT_VIGOR,
    }
}

/// `kind`'s own total contribution at level `lv` -- not the game's combined
/// total, just this one row's effect, for the number printed on it. Every
/// kind is flat and linear in its level (see `upgrades.rs`), so this is
/// nothing more than `per_level × lv`; kept as its own function only so the
/// row-drawing code doesn't repeat the multiplication inline.
fn kind_effect_amount(kind: Kind, lv: u32) -> f64 {
    kind.per_level() * lv as f64
}

/// `kind`'s effect amount, formatted in that kind's own natural unit --
/// `Chance` in percentage points, `Vigor` in whole levels, everything else
/// as a plain (possibly huge) DNA-flavoured number via `compact`.
fn kind_effect_label(kind: Kind, amount: f64) -> String {
    match kind {
        Kind::Rate => format!("+{}/s DNA", compact(amount)),
        Kind::Cost => format!("-{} cost", compact(amount)),
        Kind::Chance => format!("+{:.0}% chance", amount * 100.0),
        Kind::Cap => format!("+{} cap", compact(amount)),
        Kind::Storage => format!("+{} overflow", compact(amount)),
        Kind::Tap => format!("+{} tap fill", compact(amount)),
        Kind::Vigor => {
            let n = amount.round() as u32;
            format!("+{n} start level{}", if n == 1 { "" } else { "s" })
        }
    }
}

/// Total scrollable content height, so `main` can clamp the scroll offset
/// without needing to draw anything first.
pub fn upgrades_content_height() -> f32 {
    UPGRADE_LIST_TOP_PAD + upgrades::NUM_KINDS as f32 * UPGRADE_ROW_H
}

pub fn draw_upgrades(game: &Game, scroll: f32) {
    clear_background(BG);

    let top = bar_height();
    let bottom = screen_height() - bar_height();
    let sw = screen_width();

    let unlocked_n = upgrades::ALL_KINDS
        .iter()
        .filter(|&&k| game.kind_level(k) > 0)
        .count();
    text(
        &format!("UPGRADES   {unlocked_n} / {} unlocked", upgrades::NUM_KINDS),
        sw * 0.06,
        top + UPGRADE_LIST_TOP_PAD * 0.42,
        UPGRADE_LIST_TOP_PAD * 0.30,
        TEXT,
    );

    // What every purchased kind adds up to right now -- only the ones
    // actually active, so this stays short early on.
    let summary = upgrades::ALL_KINDS
        .iter()
        .filter(|&&k| game.kind_level(k) > 0)
        .map(|&k| kind_effect_label(k, kind_effect_amount(k, game.kind_level(k))))
        .collect::<Vec<_>>()
        .join("   ·   ");
    let summary = if summary.is_empty() {
        "discover a category to unlock its first upgrade".to_string()
    } else {
        summary
    };
    let summary_px = fit_px(&summary, sw * 0.86, UPGRADE_LIST_TOP_PAD * 0.24);
    text(
        &summary,
        sw * 0.06,
        top + UPGRADE_LIST_TOP_PAD * 0.78,
        summary_px,
        TEXT_DIM,
    );

    for (row_i, &kind) in upgrades::ALL_KINDS.iter().enumerate() {
        let r = upgrade_row_rect(row_i, scroll);
        if r.y + r.h < top || r.y > bottom {
            continue; // off-screen -- cheap cull, the list is longer than one page.
        }
        draw_upgrade_row(game, kind, r);
    }

    // Drawn last: the top and bottom bars are opaque, so they cleanly cover
    // whatever scrolled underneath them.
    draw_hud(game, "BACK");
    draw_bottom_bar(game, Some(0));
}

/// One row: a single `Kind`, not a taxon -- see the module doc in
/// `upgrades.rs`. Locked (never yet chosen by any category) shows "???";
/// unlocked shows `Kind::title`/`description` (no taxon identity any more --
/// several categories can share a `Kind`, so naming one would be arbitrary)
/// and a `BUY` button for its next level.
fn draw_upgrade_row(game: &Game, kind: Kind, r: Rect) {
    let sw = screen_width();
    let lv = game.kind_level(kind);
    let unlocked = lv > 0;
    let (x, y, w, h) = (r.x, r.y, r.w, r.h);
    let accent = if unlocked {
        kind_color(kind)
    } else {
        LOCKED_BORDER
    };

    draw_rectangle(x, y, w, h, if unlocked { PANEL } else { PANEL_LOCKED });
    draw_rectangle(x, y, 3.0, h, accent);
    draw_rectangle_lines(
        x,
        y,
        w,
        h,
        1.5,
        faded(accent, if unlocked { 0.8 } else { 0.4 }),
    );

    let swatch_size = h * 0.56;
    let swatch_center = Vec2::new(x + h * 0.5, y + h * 0.5);
    let swatch_color = if unlocked { accent } else { SILHOUETTE };
    draw_rectangle(
        swatch_center.x - swatch_size * 0.5,
        swatch_center.y - swatch_size * 0.5,
        swatch_size,
        swatch_size,
        faded(swatch_color, if unlocked { 0.9 } else { 1.0 }),
    );

    let text_x = x + h * 1.05;

    if !unlocked {
        text("???", text_x, y + h * 0.58, h * 0.30, LOCKED_TEXT);
        return;
    }

    // Right side: buying the next level. The first taxon to choose this
    // kind already granted level 1 for free -- this is the only thing DNA
    // ever buys here, and there is no ceiling on how many times.
    let next_cost = game.upgrade_level_cost(kind);
    let affordable = game.can_afford_upgrade_level(kind);
    let (bw, bh) = (sw * 0.27, (h * 0.56).max(38.0));
    let (bx, by) = (x + w - h * 0.12 - bw, y + h * 0.5 - bh * 0.5);
    let btn_color = if affordable { ACCENT_OK } else { TEXT_DIM };
    draw_rectangle_lines(bx, by, bw, bh, 2.0, btn_color);
    if affordable {
        draw_rectangle(bx, by, bw, bh, faded(ACCENT_OK, 0.12));
    }
    let buy_label = format!("Lv {}", lv + 1);
    let buy_px = fit_px(&buy_label, bw * 0.88, (bh * 0.32).max(14.0));
    text_centered(&buy_label, bx + bw * 0.5, by + bh * 0.44, buy_px, btn_color);
    let cost_label = format!("{} DNA", compact_int(next_cost));
    let cost_px = fit_px(&cost_label, bw * 0.88, (bh * 0.24).max(13.0));
    text_centered(
        &cost_label,
        bx + bw * 0.5,
        by + bh * 0.80,
        cost_px,
        faded(btn_color, 0.85),
    );

    // Left side: just what this kind does -- no taxon identity any more,
    // several categories can share one (see the module doc in `upgrades.rs`).
    let text_w = (bx - h * 0.10 - text_x).max(1.0);
    let title = kind.title();
    let title_px = fit_px(title, text_w, h * 0.22);
    text(title, text_x, y + h * 0.32, title_px, TEXT);
    let subtitle = kind.description();
    // Below ~14px this bitmap font stops being reliably legible -- letters
    // like a/o and U/Y start reading as each other rather than just looking
    // small. `fit_px` only ever shrinks further to make width, so the floor
    // has to be baked into the size passed in, not applied after.
    let subtitle_px = fit_px(subtitle, text_w, (h * 0.15).max(14.0));
    text(subtitle, text_x, y + h * 0.55, subtitle_px, TEXT_DIM);

    let effect = format!(
        "{}  ·  Lv {lv}",
        kind_effect_label(kind, kind_effect_amount(kind, lv))
    );
    let effect_px = fit_px(&effect, text_w, (h * 0.15).max(14.0));
    text(&effect, text_x, y + h * 0.80, effect_px, accent);
}

// --- evolve reveal: particles + summon ---------------------------------------
//
// Plays on top of whichever screen is active when Evolve fires, so leaving
// the current view is never required to see what happened. Two phases:
// DNA draining from the top bar into the Evolve button (the spend, made
// visible), then a slower reveal of whatever it produced.

/// Phase 1: the pool draining into the Evolve button.
pub const EVOLVE_FX_PARTICLE_SECONDS: f32 = 0.5;
/// Phase 2: summoning the result.
pub const EVOLVE_FX_SUMMON_SECONDS: f32 = 1.3;
/// Total lifetime of the effect; `main` counts a timer down from this.
pub const EVOLVE_FX_SECONDS: f32 = EVOLVE_FX_PARTICLE_SECONDS + EVOLVE_FX_SUMMON_SECONDS;

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// A small overshoot-then-settle curve -- what makes a "summon" read as a
/// snap into place rather than just a linear grow.
fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

/// Cheap, seeded pseudo-random in [0, 1) -- deterministic per index, so
/// particles don't need their own RNG state, just their slot number.
fn pseudo_random(seed: u32) -> f32 {
    let x = (seed as f32) * 12.9898;
    (x.sin() * 43758.547).fract().abs()
}

/// `elapsed` is seconds since Evolve fired, `taxon` is what it produced --
/// every Evolve discovers something now, so there is no failed-roll or
/// level-up case to distinguish here any more, only the one taxon it found.
/// Holds on the fully-revealed frame past `EVOLVE_FX_SECONDS` rather than
/// ending itself -- `main` only clears it once the player taps.
pub fn draw_evolve_fx(game: &Game, sprites: &Sprites, taxon: usize, elapsed: f32) {
    if elapsed < EVOLVE_FX_PARTICLE_SECONDS {
        draw_dna_particles(elapsed / EVOLVE_FX_PARTICLE_SECONDS);
        return;
    }
    let t = ((elapsed - EVOLVE_FX_PARTICLE_SECONDS) / EVOLVE_FX_SUMMON_SECONDS).clamp(0.0, 1.0);
    let held = (elapsed - EVOLVE_FX_SECONDS).max(0.0);
    draw_summon(game, sprites, taxon, t, held);
}

/// DNA streaming out of the top bar and converging on the Evolve button --
/// the spend, made visible, rather than a number just changing.
fn draw_dna_particles(progress: f32) {
    let sw = screen_width();
    let target = {
        let e = bottom_evolve_rect();
        Vec2::new(e.x + e.w * 0.5, e.y + e.h * 0.5)
    };
    let source_y = bar_height() - 2.0;

    const N: u32 = 16;
    for i in 0..N {
        // Staggered starts across the first half of the phase, so it reads
        // as a stream, not one rigid rank of dots moving together.
        let delay = (i as f32 / N as f32) * 0.5;
        let local = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
        if local <= 0.0 {
            continue;
        }
        let rx = pseudo_random(i * 7 + 1);
        let start = Vec2::new(rx * sw, source_y);
        let pos = start.lerp(target, ease_out_cubic(local));
        // Fades in fast, then out as it arrives.
        let alpha = (local / 0.15).min(1.0) * (1.0 - local * 0.7);
        let r = 2.5 + pseudo_random(i * 13 + 5) * 2.5;
        draw_circle(pos.x, pos.y, r, faded(ACCENT_DNA, alpha));
    }
}

/// The result condensing into view: a glow ring, the sprite snapping in from
/// a dark, undersized silhouette to full size and colour, then its name.
/// `held` is how long it has sat fully revealed, waiting on a tap -- 0 while
/// the reveal itself is still playing.
fn draw_summon(game: &Game, sprites: &Sprites, taxon: usize, t: f32, held: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    let (w, h) = (sw * 0.74, sh * 0.30);
    let (cx, cy) = (sw * 0.5, sh * 0.42);
    let panel_t = (t / 0.2).min(1.0);

    draw_rectangle(
        cx - w * 0.5,
        cy - h * 0.5,
        w,
        h,
        faded(PANEL, ease_out_cubic(panel_t) * 0.97),
    );
    let border = accent_of(game, taxon);
    draw_rectangle_lines(
        cx - w * 0.5,
        cy - h * 0.5,
        w,
        h,
        2.5,
        faded(border, panel_t),
    );

    // The reveal itself: silhouette and undersized -> full colour and size,
    // with a touch of overshoot so it lands rather than just stops.
    let reveal = ((t - 0.08) / 0.55).clamp(0.0, 1.0);
    let scale = 0.35 + 0.65 * ease_out_back(reveal);
    let sprite_size = h * 0.44 * scale;

    // An expanding, fading ring behind the sprite -- energy arriving.
    if reveal < 1.0 {
        let ring_r = sprite_size * (0.6 + reveal * 1.1);
        draw_circle_lines(
            cx,
            cy - h * 0.08,
            ring_r,
            2.0,
            faded(border, (1.0 - reveal) * 0.8),
        );
    }

    let k = reveal;
    let tint = Color::new(
        SILHOUETTE.r + (1.0 - SILHOUETTE.r) * k,
        SILHOUETTE.g + (1.0 - SILHOUETTE.g) * k,
        SILHOUETTE.b + (1.0 - SILHOUETTE.b) * k,
        1.0,
    );
    let anim = sprites::pose(game.taxon(taxon).group, taxon, get_time());
    sprites::draw_sprite(
        sprites.get(taxon),
        Vec2::new(cx, cy - h * 0.08),
        sprite_size,
        anim,
        tint,
    );

    // Name, fading in once the reveal has mostly landed.
    let text_t = ((t - 0.55) / 0.35).clamp(0.0, 1.0);
    if text_t <= 0.0 {
        return;
    }
    let outcome_px = h * 0.06;
    text_centered(
        "DISCOVERED",
        cx,
        cy + h * 0.25,
        fit_px("DISCOVERED", w * 0.85, outcome_px),
        faded(border, text_t),
    );
    let label = format!("{}  ·  {}", game.taxon(taxon).name, game.taxon(taxon).clade);
    let label_px = fit_px(&label, w * 0.85, h * 0.11);
    text_centered(&label, cx, cy + h * 0.36, label_px, faded(TEXT, text_t));

    // The card holds here until tapped -- this is what tells the player a
    // tap is what moves on, rather than another timer to wait out.
    let hint_t = (held / 0.4).clamp(0.0, 1.0);
    if hint_t > 0.0 {
        let hint_px = (label_px * 0.55).max(11.0);
        text_centered(
            "tap to continue",
            cx,
            cy + h * 0.46,
            hint_px,
            faded(TEXT_DIM, hint_t * 0.8),
        );
    }
}

// --- upgrade-kind picker: the deck-builder choice on a category discovery ---
//
// Discovering a category no longer grants a fixed power. Three random
// `Kind`s are offered and the player picks one -- `Game::choose_upgrade_kind`
// is what actually activates it. Mode-independent, like the evolve reveal,
// and drawn after it so the sequence reads as reveal-then-choose; `main`
// gates every other tap while `Game::pending_choice` is `Some`.

const KIND_CARD_H: f32 = 118.0;
const KIND_CARD_GAP: f32 = 14.0;

/// Screen-space rect of choice card `i` (0..3) -- the one source of truth
/// for both drawing it and hit-testing a tap on it.
pub fn kind_choice_rect(i: usize) -> Rect {
    let (sw, sh) = (screen_width(), screen_height());
    let w = (sw * 0.86).min(420.0);
    let total_h = KIND_CARD_H * 3.0 + KIND_CARD_GAP * 2.0;
    let top = sh * 0.5 - total_h * 0.5 + sh * 0.05;
    Rect::new(
        (sw - w) * 0.5,
        top + i as f32 * (KIND_CARD_H + KIND_CARD_GAP),
        w,
        KIND_CARD_H,
    )
}

pub fn draw_kind_picker(game: &Game, sprites: &Sprites, taxon: usize, options: [Kind; 3]) {
    let (sw, sh) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, faded(BG, 0.95));

    let taxon_ref = game.taxon(taxon);
    let def = upgrades::def_for(taxon_ref.name);
    let top = sh * 0.5 - (KIND_CARD_H * 3.0 + KIND_CARD_GAP * 2.0) * 0.5 + sh * 0.05;

    let icon_center = Vec2::new(sw * 0.5, top - 92.0);
    let anim = sprites::pose(taxon_ref.group, taxon, get_time());
    sprites::draw_sprite(sprites.get(taxon), icon_center, 64.0, anim, WHITE);

    text_centered("CHOOSE ITS POWER", sw * 0.5, top - 54.0, 18.0, ACCENT_DNA);
    let title = match def {
        Some(d) => format!("{}  ·  {}", taxon_ref.name, d.trait_name),
        None => taxon_ref.name.to_string(),
    };
    text_centered(
        &title,
        sw * 0.5,
        top - 26.0,
        fit_px(&title, sw * 0.84, 22.0),
        TEXT,
    );

    for (i, &kind) in options.iter().enumerate() {
        let r = kind_choice_rect(i);
        let accent = kind_color(kind);

        draw_rectangle(r.x, r.y, r.w, r.h, PANEL);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, accent);
        draw_rectangle(r.x, r.y, 4.0, r.h, accent);

        let pad = 18.0;
        let label = kind.label();
        text(label, r.x + pad, r.y + r.h * 0.36, 22.0, TEXT);

        // Descriptions run long enough to wrap rather than shrink to fit --
        // `wrap_lines` (also used by the fact panel) breaks at word
        // boundaries instead of crushing the font down to illegible.
        let desc_px = 14.0;
        let desc_w = r.w - pad * 2.0;
        for (j, line) in wrap_lines(kind.description(), desc_w, desc_px)
            .iter()
            .enumerate()
        {
            text(
                line,
                r.x + pad,
                r.y + r.h * 0.60 + j as f32 * desc_px * 1.3,
                desc_px,
                TEXT_DIM,
            );
        }
    }
}
