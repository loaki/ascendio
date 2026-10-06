//! Pixel art: builds the textures from the generated sprite data (`data.rs`) and animates them.

use macroquad::prelude::*;

use crate::game::genome::Morph;
use crate::game::tree::{Group, Phylogeny};

mod data;

pub use data::GRID;
use data::*;

fn rgb_of(ch: char) -> [u8; 3] {
    let hex = if ch == '*' {
        EYE
    } else {
        let i = CODES
            .find(ch)
            .unwrap_or_else(|| panic!("'{ch}' is not a palette code"));
        PALETTE[i]
    };
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
}

/// Rasterises a def, nearest-filtered. `recolor` maps (code, rgb) to the
/// drawn colour: identity, or a morph's palette shift.
fn build_texture(def: &SpriteDef, recolor: &dyn Fn(char, [u8; 3]) -> [u8; 3]) -> Texture2D {
    let mut buf = vec![0u8; GRID * GRID * 4];
    for (y, row) in def.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch != '.' {
                let [r, g, b] = recolor(ch, rgb_of(ch));
                let i = (y * GRID + x) * 4;
                buf[i..i + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }
    let tex = Texture2D::from_rgba8(GRID as u16, GRID as u16, &buf);
    tex.set_filter(FilterMode::Nearest);
    tex
}

fn luminance([r, g, b]: [u8; 3]) -> f32 {
    (r as f32 * 0.3 + g as f32 * 0.59 + b as f32 * 0.11) / 255.0
}

/// Below this, a pixel is outline ink: morphs keep it dark so the
/// silhouette still reads.
const INK: f32 = 0.14;

fn albino(ch: char, c: [u8; 3]) -> [u8; 3] {
    if ch == '*' {
        return [0xFF, 0x4A, 0x6A];
    }
    let l = luminance(c);
    if l < INK {
        return [70, 62, 72];
    }
    [
        (200.0 + l * 55.0) as u8,
        (190.0 + l * 55.0) as u8,
        (196.0 + l * 55.0) as u8,
    ]
}

fn melanistic(ch: char, c: [u8; 3]) -> [u8; 3] {
    if ch == '*' {
        return [0xE8, 0xC0, 0x40];
    }
    let l = luminance(c);
    [
        (18.0 + l * 40.0) as u8,
        (18.0 + l * 36.0) as u8,
        (24.0 + l * 44.0) as u8,
    ]
}

fn amber(ch: char, c: [u8; 3]) -> [u8; 3] {
    if ch == '*' {
        return [0x5A, 0x2A, 0x00];
    }
    let l = luminance(c);
    if l < INK {
        return [60, 30, 4];
    }
    [
        (170.0 + l * 85.0) as u8,
        (110.0 + l * 80.0) as u8,
        (20.0 + l * 40.0) as u8,
    ]
}

fn ancestor() -> SpriteDef {
    animal_def("Ancestor").expect("the root glyph exists")
}

/// A Backbone taxon borrows the art of its first descendant with its own
/// sprite; only the root keeps the ancestor glyph.
fn def_for(phy: &Phylogeny, i: usize) -> SpriteDef {
    let t = &phy.taxa[i];
    if t.group != Group::Backbone {
        return animal_def(t.name).unwrap_or_else(ancestor);
    }
    if i != Phylogeny::ROOT {
        let mut queue: std::collections::VecDeque<usize> = t.children.iter().copied().collect();
        while let Some(c) = queue.pop_front() {
            let ct = &phy.taxa[c];
            if ct.group != Group::Backbone {
                if let Some(def) = animal_def(ct.name) {
                    return def;
                }
            }
            queue.extend(ct.children.iter().copied());
        }
    }
    ancestor()
}

pub struct Sprites {
    textures: Vec<[Texture2D; 4]>,
    boons: Vec<Texture2D>,
}

pub const BOON_ART: [&str; 6] = [
    "Boon Lure",
    "Boon Lens",
    "Boon Catalyst",
    "Boon Charm",
    "Boon Tailwind",
    "Boon Tectonics",
];

impl Sprites {
    pub fn build(phy: &Phylogeny) -> Self {
        let textures = (0..phy.len())
            .map(|i| {
                let def = def_for(phy, i);
                [
                    build_texture(&def, &|_, c| c),
                    build_texture(&def, &albino),
                    build_texture(&def, &melanistic),
                    build_texture(&def, &amber),
                ]
            })
            .collect();
        let boons = BOON_ART
            .iter()
            .map(|name| build_texture(&animal_def(name).unwrap_or_else(ancestor), &|_, c| c))
            .collect();
        Self { textures, boons }
    }

    pub fn boon(&self, i: usize) -> &Texture2D {
        &self.boons[i]
    }

    pub fn get(&self, taxon: usize) -> &Texture2D {
        &self.textures[taxon][0]
    }

    pub fn get_morph(&self, taxon: usize, morph: Morph) -> &Texture2D {
        let slot = match morph {
            Morph::None | Morph::Giant => 0,
            Morph::Albino => 1,
            Morph::Melanistic => 2,
            Morph::Amber => 3,
        };
        &self.textures[taxon][slot]
    }
}

pub fn morph_scale(morph: Morph) -> f32 {
    if morph == Morph::Giant {
        1.45
    } else {
        1.0
    }
}

#[derive(Clone, Copy)]
pub struct Pose {
    pub y_off: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            y_off: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
        }
    }
}

/// Idle motion per clade, a pure function of time. Phases are spread by the
/// golden angle so sprites never move in lockstep.
pub fn pose(group: Group, taxon: usize, t: f64) -> Pose {
    let off = taxon as f32 * 2.399_963;
    // The angle of a `rate` rad/s wave, wrapped in f64 so it stays smooth.
    let w = |rate: f64| crate::gfx::pixel::phase(t, rate) + off;

    match group {
        Group::Backbone => {
            let s = 1.0 + 0.05 * w(1.1).sin();
            Pose {
                scale_x: s,
                scale_y: s,
                ..Default::default()
            }
        }
        Group::Basal | Group::Deuterostome => {
            let bob = w(1.3).sin() * 0.05;
            let s = 1.0 + 0.06 * w(1.7).sin();
            Pose {
                y_off: bob,
                scale_x: s,
                scale_y: 2.0 - s,
                rotation: 0.0,
            }
        }
        Group::Ecdysozoa | Group::Fish => {
            let bob = w(3.4).sin() * 0.06;
            let rot = w(4.2).sin() * 0.09;
            Pose {
                y_off: bob,
                rotation: rot,
                ..Default::default()
            }
        }
        _ => {
            let walk = w(2.1).sin();
            let bob = walk.abs() * 0.07 - 0.02;
            Pose {
                y_off: -bob,
                scale_x: 1.0 - 0.02 * bob.abs(),
                scale_y: 1.0 + 0.02 * bob.abs(),
                rotation: walk * 0.035,
            }
        }
    }
}

pub fn draw_sprite(tex: &Texture2D, center: Vec2, size: f32, pose: Pose, tint: Color) {
    let dest = Vec2::new(size * pose.scale_x, size * pose.scale_y);
    let top_left = Vec2::new(
        center.x - dest.x * 0.5,
        center.y - dest.y * 0.5 + size * pose.y_off,
    );
    draw_texture_ex(
        tex,
        top_left.x,
        top_left.y,
        tint,
        DrawTextureParams {
            dest_size: Some(dest),
            rotation: pose.rotation,
            ..Default::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_species_has_its_own_well_formed_art() {
        let phy = Phylogeny::load();
        let names = phy
            .taxa
            .iter()
            .filter(|t| t.group != Group::Backbone)
            .map(|t| t.name)
            .chain(["Ancestor"])
            .chain(BOON_ART);
        for name in names {
            let def = animal_def(name).unwrap_or_else(|| panic!("{name} has no sprite"));
            for row in def {
                assert_eq!(row.chars().count(), GRID, "{name}: wrong row width");
                for ch in row.chars().filter(|&c| c != '.') {
                    rgb_of(ch);
                }
            }
        }
    }

    #[test]
    fn the_palette_and_codes_line_up() {
        assert_eq!(CODES.chars().count(), PALETTE.len());
        assert!(!CODES.contains(['.', '*']));
    }
}
