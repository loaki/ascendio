//! Screen chrome and overlay screens; every tappable rect comes from here so drawing and hit-testing agree.

use macroquad::prelude::*;

use crate::game::ecology::{self, Bonus, Rule};
use crate::game::genome;
use crate::game::genome::Morph;
use crate::game::leaderboard::{Board, Entry, Leaderboard, View};
use crate::game::names::Language;
use crate::game::planet::{self, Biome, Lever};
use crate::game::settings;
use crate::game::{self, Boon, Game, Phase, Report, Status};
use crate::gfx::dial;
use crate::gfx::render::{
    self, draw_meter, faded, fit_px, rgb, text, text_centered, text_width, wrap_lines, ACCENT_DNA,
    ACCENT_OK, ACCENT_WARN, CYAN, EDGE, GOLD, HUD_BG, LIME, LOCKED_BORDER, LOCKED_TEXT, LOCK_RED,
    PANEL, PANEL_LOCKED, PINK, RAD_COLOR, SILHOUETTE, STONE, STONE_TEXT, TEXT, TEXT_DIM, TRACK,
};
use crate::gfx::sprites::{self, Sprites};
use crate::gfx::view;

const DEEP_BG: Color = rgb(0x05090F);
const CARD_BG: Color = rgb(0x0A1020);
const BUTTON_BG: Color = rgb(0x0A1422);
const PANEL_EDGE: Color = rgb(0x1B2C48);
const BUTTON_EDGE: Color = rgb(0x2F6B72);
const DORMANT: Color = Color::new(0.42, 0.44, 0.5, 1.0);

pub struct Assets {
    pub glow: Texture2D,
    pub gear: [Texture2D; 2],
    pub trophy: [Texture2D; 2],
}

const GEAR: [&str; 16] = [
    "......kkkk......",
    "......kllk......",
    "..kk..kllk..kk..",
    ".kllkklllbkkbbk.",
    "..klllllbbbbbk..",
    "...kllkkkkbbk...",
    "kkkllk....kbbkkk",
    "kllllk....kbdddk",
    "klllbk....kddddk",
    "kkkbbk....kddkkk",
    "...kbbkkkkddk...",
    "..kbbbbbdddddk..",
    ".kbbkkbdddkkddk.",
    "..kk..kddk..kk..",
    "......kddk......",
    "......kkkk......",
];

const GEAR_INKS: [[u32; 4]; 2] = [
    [0xE6F0F6, 0xB8CAD6, 0x8CA0B2, 0x101420],
    [0xBFE8F0, 0x5BC8F5, 0x2F7F92, 0x0E242C],
];

const TROPHY: [&str; 16] = [
    "................",
    "..kkkkkkkkkkkk..",
    "kkkllllllbbbdkkk",
    "kbklllllbbbbdkdk",
    "kbklllllbbbbdkdk",
    "kbkllllbbbbbdkdk",
    ".kkkllbbbbbbdkk.",
    "...klbbbbbbdk...",
    "....kbbbbbdk....",
    ".....kbbbdk.....",
    "......kbdk......",
    "......kbdk......",
    ".....kbbbdk.....",
    "....kllbbbdk....",
    "....kkkkkkkk....",
    "................",
];

const TROPHY_INKS: [[u32; 4]; 2] = [
    [0xFFF0B0, 0xF0B840, 0xB07A20, 0x101420],
    [0xFFFBE0, 0xFFD45A, 0xD89A30, 0x2A1A08],
];

fn icon_texture(art: &[&str; 16], inks: [u32; 4]) -> Texture2D {
    let mut buf = vec![0u8; 16 * 16 * 4];
    for (y, row) in art.iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            let Some(i) = b"lbdk".iter().position(|&c| c == ch) else {
                continue;
            };
            let c = inks[i];
            let o = (y * 16 + x) * 4;
            buf[o..o + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
        }
    }
    let tex = Texture2D::from_rgba8(16, 16, &buf);
    tex.set_filter(FilterMode::Nearest);
    tex
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

        Self {
            glow,
            gear: GEAR_INKS.map(|inks| icon_texture(&GEAR, inks)),
            trophy: TROPHY_INKS.map(|inks| icon_texture(&TROPHY, inks)),
        }
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

mod animal;
mod biomes;
mod boons;
mod bottom_bar;
mod hud;
mod keystones;
mod lever_panel;
mod settings_page;
mod wait_panel;

pub use animal::*;
pub use biomes::*;
pub use boons::*;
pub use bottom_bar::*;
pub use hud::*;
pub use keystones::*;
pub use lever_panel::*;
pub use settings_page::*;
pub use wait_panel::*;
