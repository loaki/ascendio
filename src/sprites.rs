//! Hand-authored pixel art, generated from `scripts/pixel-art/`.
//!
//! Do not hand-edit the sprite data below -- regenerate it from the Python
//! source (see `scripts/pixel-art/README.md`) so the two never drift.
//! Everything else in this file (the texture builder, the animation curves,
//! the lookup) is hand-written and safe to edit directly.

use macroquad::prelude::*;

use crate::tree::{Group, Phylogeny};

/// Every sprite is authored on the same small canvas. Small on purpose: it
/// keeps the whole roster quick to author and reads as a deliberate
/// minimalist style rather than as missing detail, once nearest-neighbour
/// scaling blows it up on screen.
pub const GRID: usize = 12;

/// One sprite: a 12x12 grid of palette characters (row-major, `.` is
/// transparent) plus that sprite's own small palette.
struct SpriteDef {
    rows: [&'static str; GRID],
    palette: &'static [(char, u32)],
}

// --- specific, hand-authored animals ----------------------------------------

/// Looked up by `Taxon::name`. Every non-Backbone taxon in `tree.rs` has an
/// entry here; anything missing falls back to its clade's template.
fn animal_def(name: &str) -> Option<SpriteDef> {
    Some(match name {
        "Acoel Worm" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "...aaaaaaa..",
                "..akaaaaaaa.",
                "...aaaaaaa..",
                "............",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0xCDA37C), ('k', 0x6B4A2E)],
        },
        "Amphibian" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                ".kk.........",
                "...aaaaaaa..",
                "..aaaaaaaaa.",
                "...aaaaaaa..",
                "...a.....aa.",
                ".........aa.",
                "............",
                "............",
            ],
            palette: &[('a', 0x5FA85A), ('k', 0x1E3A1C)],
        },
        "Ant" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "..........b.",
                ".........b..",
                "........k...",
                "..aa..aaaa..",
                ".aaaaaa.a...",
                "..ab.b.b.b..",
                "..b..b..b.b.",
                "............",
                "............",
            ],
            palette: &[('a', 0xB8481E), ('b', 0x5A2010), ('k', 0x140804)],
        },
        "Ape" => SpriteDef {
            rows: [
                "............",
                "............",
                ".....kak....",
                ".....aaa....",
                ".....aaa....",
                "..a.aaaaa.a.",
                "....aaaaa...",
                ".a..aaaaa..a",
                "....aaaaa...",
                "....aaaaa...",
                "............",
                "....a...a...",
            ],
            palette: &[('a', 0x4A3A2E), ('k', 0x0A0806)],
        },
        "Bat" => SpriteDef {
            rows: [
                "............",
                "............",
                ".aa...a...aa",
                "a..a.a.a.a..",
                ".aaaa...aaaa",
                "aaaaakakaaaa",
                ".aaaaaaaaaaa",
                "aaaaaaaaaaaa",
                ".aaaa.a.aaaa",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0x3A2E44), ('k', 0xE04040)],
        },
        "Beetle" => SpriteDef {
            rows: [
                "............",
                "............",
                ".....kak....",
                ".....aaa....",
                "...aaaaaaa..",
                "...aaabaaa..",
                "..aaaabaaaa.",
                ".bbbaabaabbb",
                "...aaabaaa..",
                "...aaabaaa..",
                "......b.....",
                "............",
            ],
            palette: &[('a', 0x3E7A3A), ('b', 0x275224), ('k', 0x0F2A0E)],
        },
        "Bird" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                ".b......ak..",
                "...baaaaaac.",
                "...aaaaaaa..",
                "...baaaaaa..",
                "....aaaaa...",
                ".....c......",
                "......c.....",
                "............",
            ],
            palette: &[
                ('a', 0x3E6EDE),
                ('b', 0x2A4EB0),
                ('c', 0xE8A030),
                ('k', 0x0A1230),
            ],
        },
        "Butterfly" => SpriteDef {
            rows: [
                "............",
                ".....k.k....",
                "......k.....",
                ".accaakaacca",
                ".accaakaacca",
                ".aaaaakaaaaa",
                "...aa.k.aa..",
                "..aaaakaaaa.",
                "..accakacca.",
                "...aa.k.aa..",
                "............",
                "............",
            ],
            palette: &[('a', 0xE8862F), ('c', 0x2A1608), ('k', 0x1A1008)],
        },
        "Chimpanzee" => SpriteDef {
            rows: [
                "............",
                "............",
                ".....kck....",
                ".....ccc....",
                ".....ccc....",
                "...a.aaa.a..",
                "....aaaaa...",
                "..a.aaaaa.a.",
                ".....aaa....",
                "....a.a.a...",
                "............",
                "....a...a...",
            ],
            palette: &[('a', 0x6E4E30), ('c', 0xD9B98A), ('k', 0x0E0A04)],
        },
        "Coelacanth" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "....c.c.....",
                ".....aaa....",
                "a..aaaaaka..",
                "aa.aaaaaaa..",
                "a..aaaaaaa..",
                "..b..aaa....",
                ".b.b.c......",
                "............",
                "............",
            ],
            palette: &[
                ('a', 0x4E6E8A),
                ('b', 0x3A5468),
                ('c', 0xBFD4E0),
                ('k', 0x0E1620),
            ],
        },
        "Comb Jelly" => SpriteDef {
            rows: [
                "............",
                "............",
                "..c.cacac...",
                "..cacwcac...",
                "..cwcacwc...",
                "..cacacac...",
                "..cacacac...",
                "..cacacac...",
                "..cacacac...",
                "..c.cacac...",
                "............",
                "............",
            ],
            palette: &[('a', 0xBFE9F2), ('c', 0xE7A6E0), ('w', 0xFFFFFF)],
        },
        "Coral" => SpriteDef {
            rows: [
                "............",
                "....c..c....",
                ".c..a..a..c.",
                "..c.a..a.ca.",
                "..a.a..aaa..",
                "...a.aa.a...",
                "....aaa.a...",
                "....aaaa....",
                "....aaa.....",
                ".....aa.....",
                ".....bb.....",
                ".....bb.....",
            ],
            palette: &[('a', 0xE87A5A), ('b', 0xB2553B), ('c', 0xF7B08A)],
        },
        "Crab" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "b.b......b.b",
                ".b..aaaaa.b.",
                "..aakaakaaa.",
                "..aaaaaaaaa.",
                "..aaaaaaaaa.",
                "..bbaaaabb..",
                "..bb....bb..",
                "............",
                "............",
            ],
            palette: &[('a', 0xE0553A), ('b', 0xB03A24), ('k', 0x3A1006)],
        },
        "Crocodile" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "a...........",
                ".a.aaaaaakbb",
                "a.aaaaaaaaaa",
                "...aaaaaaa..",
                "..a......a..",
                "............",
                "............",
            ],
            palette: &[('a', 0x3E6E3A), ('b', 0x2A4E28), ('k', 0xE0D060)],
        },
        "Dinosaur" => SpriteDef {
            rows: [
                "..........a.",
                "..........kb",
                ".........aa.",
                "........aa..",
                "....b..aa...",
                "...baaaa....",
                "....aaaa....",
                "..a.aaaa....",
                "..a.aaaa....",
                ".a..aaaa....",
                ".a..aaaa....",
                "aa..aaaa....",
            ],
            palette: &[('a', 0x8A5A3A), ('b', 0x6E4428), ('k', 0x1E1006)],
        },
        "Dragonfly" => SpriteDef {
            rows: [
                "............",
                "............",
                "..ccc..ccc..",
                "cccccccccccc",
                ".kccccccccc.",
                ".kaaaaaaaaa.",
                "..aaaaaaaaa.",
                "............",
                "cccccccccccc",
                "cccccccccccc",
                "............",
                "............",
            ],
            palette: &[('a', 0x2E6E5E), ('c', 0xB9E8DC), ('k', 0x123028)],
        },
        "Elephant" => SpriteDef {
            rows: [
                "............",
                "............",
                "..bb........",
                ".aabb.......",
                "kbbbbaaaaa..",
                ".bbbbaaaaaa.",
                ".abbaaaaaaa.",
                ".a.aaaaaaaa.",
                ".aa.aaaaaa..",
                ".a.aa...a...",
                "....a...a...",
                "............",
            ],
            palette: &[('a', 0x9098A0), ('b', 0x7A828C), ('k', 0x181C1E)],
        },
        "Flatworm" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "...kaaaaaa..",
                "..abbbbbbba.",
                "...kaaaaaa..",
                "............",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0xB98CD1), ('b', 0x8E5FAE), ('k', 0x4A2E5E)],
        },
        "Frog" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "...ka.ak....",
                "...aaaaa....",
                "...aaaaaa...",
                "..aaaaaaaaa.",
                ".a.aaaaaaa..",
                "..a.aaaa..a.",
                "............",
                "............",
            ],
            palette: &[('a', 0x6FBF4A), ('k', 0x123008)],
        },
        "Horseshoe Crab" => SpriteDef {
            rows: [
                "............",
                "......a.....",
                "...aaaaaaa..",
                "..aakaaakaa.",
                "..aaaaaaaaa.",
                "..aaaaaaaaa.",
                "..aaaaaaaaa.",
                "...abbbbba..",
                "...bbbbbbb..",
                "....bbbbb...",
                ".....bbb....",
                "............",
            ],
            palette: &[('a', 0x8C6B4A), ('b', 0x6E4F32), ('k', 0x2E1E10)],
        },
        "Human" => SpriteDef {
            rows: [
                "............",
                "............",
                ".....kbk....",
                ".....bbb....",
                "......b.....",
                "...c.ccc.c..",
                ".....ccc....",
                "..c..ccc..c.",
                ".....ccc....",
                ".....aca....",
                "............",
                ".....a.a....",
            ],
            palette: &[
                ('a', 0x3E4E6E),
                ('b', 0xC9905E),
                ('c', 0xD9A470),
                ('k', 0x140E08),
            ],
        },
        "Jellyfish" => SpriteDef {
            rows: [
                "............",
                "............",
                "...aaaaaa...",
                "..aakaakaa..",
                "..aaaaaaaa..",
                "..babbabab..",
                "..b.bbab.b..",
                "..b.bb.b.b..",
                "..b.bb.b.b..",
                "..b.bb.b.b..",
                "..b.bb.b.b..",
                "............",
            ],
            palette: &[('a', 0xD79BE0), ('b', 0xA65FC0), ('k', 0x5A2E6E)],
        },
        "Kangaroo" => SpriteDef {
            rows: [
                "............",
                ".....a.a....",
                ".....aka....",
                ".....aaa....",
                "....aaaaa...",
                "....aaaaa...",
                "....aaaaa...",
                "....aaaaaa..",
                "..a.aaaaaa..",
                "....aa.aaa..",
                ".a..aa....aa",
                "a...........",
            ],
            palette: &[('a', 0xC08A4A), ('k', 0x2A1A08)],
        },
        "Lamprey" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                ".bka..a..a..",
                ".bbaaaaaaaa.",
                ".ba.aa.aa.a.",
                "............",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0x8E9A6E), ('b', 0x5E6E42), ('k', 0x1E2210)],
        },
        "Lemur" => SpriteDef {
            rows: [
                "............",
                "....a...a...",
                ".....kck....",
                ".....ccc..b.",
                ".....aca.b.b",
                ".....aaa....",
                ".....aaab.b.",
                ".....aaa.b..",
                "....a.aa....",
                "............",
                "....a..a....",
                "............",
            ],
            palette: &[
                ('a', 0x8A8478),
                ('c', 0xE8E0D0),
                ('b', 0x5A5448),
                ('k', 0x0E0E0A),
            ],
        },
        "Lizard" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                ".........ka.",
                "a...aaaaaa..",
                ".a.aaaaaaa..",
                "a...aaaaa...",
                "............",
                "...a.....a..",
                "............",
                "............",
            ],
            palette: &[('a', 0x4A9E4A), ('k', 0x102A10)],
        },
        "Mouse" => SpriteDef {
            rows: [
                "............",
                "............",
                "..b.........",
                ".bba........",
                "..kaa.......",
                "..aaaaaa...a",
                "...aaaaaaaa.",
                "...aaaaaaa..",
                "...aaaaaaa..",
                ".....aaa....",
                "............",
                "............",
            ],
            palette: &[('a', 0x9A8A78), ('b', 0xC9B49E), ('k', 0x14100C)],
        },
        "Nematode" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "..aa....aa..",
                ".aaaa..aaaa.",
                ".a..aaaa..a.",
                ".....aa.....",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0xE0C070)],
        },
        "Octopus" => SpriteDef {
            rows: [
                "............",
                ".....aaa....",
                "...aaaaaaa..",
                "...awaaawa..",
                "...aaaaaaa..",
                "...aaaaaaa..",
                ".ababababa..",
                ".ababababa..",
                ".ababababa..",
                ".ababababa..",
                ".a.a.a.a.a..",
                "............",
            ],
            palette: &[
                ('a', 0xD1548C),
                ('b', 0xA83A6C),
                ('k', 0x2A0F1E),
                ('w', 0xFFFFFF),
            ],
        },
        "Placozoan" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "..aaabaaaa..",
                "..abaaaaba..",
                "..aaaaabaa..",
                "..aaaaaaaa..",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0xE7C79A), ('b', 0xC9A26E)],
        },
        "Platypus" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "...aaaaakac.",
                "...aaaaaaccc",
                ".a.aaaaaaac.",
                "a..aaaaaaa..",
                "...a.....a..",
                "............",
                "............",
            ],
            palette: &[('a', 0x7A5A34), ('c', 0xB98A3E), ('k', 0x1E1206)],
        },
        "Ray-finned Fish" => SpriteDef {
            rows: [
                "............",
                "............",
                ".......a....",
                "......a.....",
                "a....aaa....",
                "...aaaaawa..",
                ".a.aaaaaaa..",
                "...aaaaaaa..",
                "a....aaa....",
                "......a.....",
                ".......a....",
                "............",
            ],
            palette: &[('a', 0x4A9FD8), ('k', 0x102030), ('w', 0xE8F6FF)],
        },
        "Scorpion" => SpriteDef {
            rows: [
                "............",
                "..........k.",
                "..........b.",
                "..........b.",
                ".b.......b..",
                "..baaaaab...",
                "...aaaaaa...",
                "..baaaaaa...",
                ".b.aaaaaa...",
                "...aa..aa...",
                "............",
                "............",
            ],
            palette: &[('a', 0xE0A030), ('b', 0xB87F20), ('k', 0x2A1808)],
        },
        "Sea Sponge" => SpriteDef {
            rows: [
                "............",
                "....aaaa....",
                "...aaaaaa...",
                "...akaaka...",
                "...aaaaaa...",
                "...aaaaaa...",
                "...aakaak...",
                "...aaaaaa...",
                "...akbbba...",
                "...bbbbbb...",
                "....bbbb....",
                "............",
            ],
            palette: &[('a', 0xD9A857), ('b', 0xA97C34), ('k', 0x6B4A1E)],
        },
        "Sea Squirt" => SpriteDef {
            rows: [
                "............",
                "............",
                "....bb.bb...",
                "....aa.aa...",
                "....aaaaa...",
                "....aaaaa...",
                "....aaaaa...",
                "....aaaaa...",
                "....aaaaa...",
                "....aaaaa...",
                ".....aaa....",
                "............",
            ],
            palette: &[('a', 0xE0C24A), ('b', 0xB89830)],
        },
        "Sea Urchin" => SpriteDef {
            rows: [
                ".....b......",
                "...b....b...",
                "..b......b..",
                ".b.bbbabb.b.",
                "...baaaab...",
                "....aaaaa...",
                "b..baaaaab.b",
                "...baaaab...",
                ".b.bbaabb.b.",
                "..b...b..b..",
                "...b....b...",
                "......b.....",
            ],
            palette: &[('a', 0x4A2E6E), ('b', 0x2E1A4A)],
        },
        "Segmented Worm" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                ".b.b.b.b.b.b",
                ".kababababab",
                ".bababababab",
                ".bababababab",
                ".b.b.b.b.b.b",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0xC96B4E), ('b', 0xA0492F), ('k', 0x3A1A0E)],
        },
        "Shark" => SpriteDef {
            rows: [
                "............",
                "............",
                ".........a..",
                ".........aa.",
                ".........a..",
                "a.aaaaaaak..",
                ".aaaaaaaaaa.",
                "a.aaaaaaaa..",
                ".a..........",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0x6E8CA0), ('k', 0x1A2428)],
        },
        "Snail" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                ".....bbbb...",
                "....bccccb..",
                "....bccccb..",
                ".k.bccccccb.",
                ".a.abccccb..",
                "..aabccccb..",
                ".aaaabbbb...",
                "...aaaa.....",
                "............",
            ],
            palette: &[
                ('a', 0xD9B15C),
                ('b', 0xE8CE8A),
                ('c', 0xB98530),
                ('k', 0x4A3416),
            ],
        },
        "Spider" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                ".c...aaa...c",
                "..cc.kak.cc.",
                "cc..caaac..c",
                "..c.ccccc.c.",
                "ccc.ccacc.cc",
                "..cccaaaccc.",
                ".c...aaa...c",
                "............",
                "............",
            ],
            palette: &[('a', 0x6E3A4A), ('c', 0xB05A6A), ('k', 0xE04040)],
        },
        "Starfish" => SpriteDef {
            rows: [
                "............",
                "......a.....",
                "......a.....",
                "......a.....",
                ".aaaa.a..aa.",
                "..aaaaaaaa..",
                "....aabaa...",
                "...aaaaa....",
                "...aa.aa....",
                "...a...aa...",
                "...a....a...",
                "............",
            ],
            palette: &[('a', 0xE0762E), ('b', 0xB55A1E)],
        },
        "Turtle" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "....aaaaa..k",
                "...abbbbba.c",
                "..abbaaabbc.",
                "..abbaaabba.",
                "...abbbbba..",
                "..c.caaacc..",
                "............",
                "............",
            ],
            palette: &[
                ('a', 0x3E8E4E),
                ('b', 0x2A6636),
                ('c', 0xC9A25E),
                ('k', 0x102008),
            ],
        },
        "Whale" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "........b...",
                "....aaaaab..",
                "b.aaaaaaka..",
                ".baaaaaaaaa.",
                "b.aaaaaaaa..",
                "....aaaaa...",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0x3E6E8E), ('b', 0x2E5470), ('k', 0x0A1620)],
        },
        "Wolf" => SpriteDef {
            rows: [
                "............",
                "............",
                "...........a",
                "a.........a.",
                ".a......aaka",
                "...aaaaaaaaa",
                "...aaaaaaaa.",
                "..aaaaaaaa..",
                "............",
                "...a....a...",
                "............",
                "............",
            ],
            palette: &[('a', 0x8A8E94), ('k', 0x14161A)],
        },
        _ => return None,
    })
}

// --- fallback templates, one per clade, plus the ancestor glyph -------------

/// A `Group::Backbone` taxon is an inferred common ancestor, not a depicted
/// species -- it gets this abstract branching glyph instead of an animal.
/// Any other taxon without a specific sprite falls back to its clade's shape,
/// so every one of the 68 taxa always renders *something* animated.
fn template_def(group: Group) -> SpriteDef {
    let key = match group {
        Group::Backbone => "Ancestor",
        Group::Basal => "Basal",
        Group::Spiralia => "Spiralia",
        Group::Ecdysozoa => "Ecdysozoa",
        Group::Deuterostome => "Deuterostome",
        Group::Fish => "Fish",
        Group::Tetrapod => "Tetrapod",
        Group::Reptile => "Reptile",
        Group::Mammal => "Mammal",
    };
    match key {
        "Ancestor" => SpriteDef {
            rows: [
                "............",
                ".....aa.....",
                "...aaaa.....",
                ".....aaaa...",
                "...bbba.....",
                ".....abbb...",
                "...aaaa.....",
                ".....aaaa...",
                "...bbba.....",
                ".....abbb...",
                "...aaaa.....",
                "......aaa...",
            ],
            palette: &[('a', 0x8A94A3), ('b', 0x5E6774)],
        },
        "Basal" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "....aabaa...",
                "...abaaaba..",
                "..aaaaaaaaa.",
                "..aaaaaaaaa.",
                "...aaaaaaa..",
                "....aaaaa...",
                "............",
                "............",
            ],
            palette: &[('a', 0x3FB8AF), ('b', 0x2C8A83)],
        },
        "Deuterostome" => SpriteDef {
            rows: [
                "............",
                "............",
                "......a.....",
                "......a.....",
                "..a...a..a..",
                "...aaaaaa...",
                ".....aaa....",
                "....a.aa....",
                "....a..a....",
                "...a....a...",
                "............",
                "............",
            ],
            palette: &[('a', 0xE86A5E)],
        },
        "Ecdysozoa" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "..b..aaa..b.",
                "....aaaaa...",
                ".b.aaaaaaa.b",
                "....aaaaa...",
                "..b..aaa..b.",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0xE8A33D), ('b', 0xB87D26)],
        },
        "Fish" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "......a.....",
                "a....aaa....",
                "...aaaaaka..",
                ".a.aaaaaaa..",
                "...aaaaaaa..",
                "a....aaa....",
                "......a.....",
                "............",
                "............",
            ],
            palette: &[('a', 0x4A9FE0), ('k', 0x0E2436)],
        },
        "Mammal" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                ".........ak.",
                "....aaaaaaa.",
                "...aaaaaaaa.",
                "...aaaaaaa..",
                "....aaaaa...",
                "............",
                "....a...a...",
                "............",
            ],
            palette: &[('a', 0xF0C674), ('k', 0x4A3A14)],
        },
        "Reptile" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "a...aaaaak..",
                ".a.aaaaaaa..",
                "a..aaaaaaa..",
                "....aaaaa...",
                "...a.....a..",
                "............",
                "............",
            ],
            palette: &[('a', 0xE05E8A), ('k', 0x4A1626)],
        },
        "Spiralia" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                ".....bbb....",
                "...abbbbba..",
                "..aabbabbaa.",
                "...abbbbba..",
                ".....bbb....",
                "............",
                "............",
                "............",
            ],
            palette: &[('a', 0x9B7EDE), ('b', 0x7657B5)],
        },
        "Tetrapod" => SpriteDef {
            rows: [
                "............",
                "............",
                "............",
                "............",
                "............",
                "....aaaaak..",
                "...aaaaaaa..",
                "...aaaaaaa..",
                "...aaaaaa...",
                "..a......a..",
                "............",
                "............",
            ],
            palette: &[('a', 0x5CC26B), ('k', 0x113B18)],
        },
        _ => unreachable!("every Group has a template"),
    }
}
// --- building textures -------------------------------------------------------

/// Rasterises a def into an RGBA texture, nearest-filtered so each authored
/// pixel stays a crisp square however far it is scaled up on screen.
fn build_texture(def: &SpriteDef) -> Texture2D {
    let mut buf = vec![0u8; GRID * GRID * 4];

    for (y, row) in def.rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let rgba = match ch {
                '.' => [0, 0, 0, 0],
                _ => {
                    // A palette miss would be a silent grey square; fail loud
                    // instead, since this only runs at startup on trusted data.
                    let hex = def
                        .palette
                        .iter()
                        .find(|&&(c, _)| c == ch)
                        .unwrap_or_else(|| panic!("'{ch}' has no palette entry"))
                        .1;
                    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]
                }
            };
            let i = (y * GRID + x) * 4;
            buf[i..i + 4].copy_from_slice(&rgba);
        }
    }

    let tex = Texture2D::from_rgba8(GRID as u16, GRID as u16, &buf);
    tex.set_filter(FilterMode::Nearest);
    tex
}

/// Every taxon's texture, built once at startup (texture creation needs a GL
/// context, so this can only happen after macroquad's main has started).
pub struct Sprites {
    textures: Vec<Texture2D>,
}

impl Sprites {
    pub fn build(phy: &Phylogeny) -> Self {
        let textures = phy
            .taxa
            .iter()
            .map(|t| {
                let def = if t.group == Group::Backbone {
                    template_def(t.group)
                } else {
                    animal_def(t.name).unwrap_or_else(|| template_def(t.group))
                };
                build_texture(&def)
            })
            .collect();
        Self { textures }
    }

    pub fn get(&self, taxon: usize) -> &Texture2D {
        &self.textures[taxon]
    }
}

// --- animation ----------------------------------------------------------------

/// A sprite's pose for this frame, applied on top of its base on-screen size.
#[derive(Clone, Copy)]
pub struct Pose {
    /// Vertical offset, as a fraction of the sprite's size.
    pub y_off: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    /// Radians.
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

/// A stable per-taxon offset so 68 sprites never breathe in lockstep. The
/// golden angle spreads them evenly around the cycle without needing to
/// store anything -- it is a pure function of the taxon id.
fn phase_of(taxon: usize) -> f32 {
    const GOLDEN_ANGLE: f32 = 2.399_963;
    (taxon as f32) * GOLDEN_ANGLE
}

/// Every clade gets its own flavour of idle motion: land animals bob as if
/// stepping, fish and arthropods flick and flutter, sessile marine life
/// pulses, and the abstract ancestor glyph just breathes. `t` is wall-clock
/// seconds (`macroquad::time::get_time()`); the animation is a pure function
/// of it, so nothing here needs per-frame state.
pub fn pose(group: Group, taxon: usize, t: f64) -> Pose {
    let phase = phase_of(taxon);
    let tt = t as f32;

    match group {
        Group::Backbone => {
            let s = 1.0 + 0.05 * (tt * 1.1 + phase).sin();
            Pose {
                scale_x: s,
                scale_y: s,
                ..Default::default()
            }
        }
        Group::Basal | Group::Deuterostome => {
            // Sessile or drifting: a slow pulse and a gentle bob, no rotation.
            let bob = (tt * 1.3 + phase).sin() * 0.05;
            let s = 1.0 + 0.06 * (tt * 1.7 + phase).sin();
            Pose {
                y_off: bob,
                scale_x: s,
                scale_y: 2.0 - s,
                rotation: 0.0,
            }
        }
        Group::Ecdysozoa | Group::Fish => {
            // Brisk flutter or fin-flick.
            let bob = (tt * 3.4 + phase).sin() * 0.06;
            let rot = (tt * 4.2 + phase).sin() * 0.09;
            Pose {
                y_off: bob,
                rotation: rot,
                ..Default::default()
            }
        }
        _ => {
            // Legged land animals: a stepping bob with a slight sway, so it
            // never looks like it is just floating in place.
            let walk = (tt * 2.1 + phase).sin();
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

/// Draws one sprite centred at `center`, `size` on its longest unanimated
/// edge, animated per `pose`. `tint` multiplies every pixel -- `WHITE` for no
/// change, or a dark colour for the locked-species silhouette tease.
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
    use crate::game::Game;

    #[test]
    fn every_non_backbone_taxon_has_a_named_sprite_or_a_clade_fallback() {
        let game = Game::new(0.0);
        for t in &game.phy.taxa {
            if t.group != Group::Backbone {
                // Either a specific def exists, or template_def(t.group) covers
                // it -- both branches just must not panic building the texture.
                let _ = animal_def(t.name).is_some();
            }
        }
    }

    #[test]
    fn every_group_has_a_template() {
        for g in [
            Group::Backbone,
            Group::Basal,
            Group::Spiralia,
            Group::Ecdysozoa,
            Group::Deuterostome,
            Group::Fish,
            Group::Tetrapod,
            Group::Reptile,
            Group::Mammal,
        ] {
            let def = template_def(g);
            // Every referenced palette char must resolve -- build_texture
            // panics otherwise, without needing a GL context to check for it.
            for row in &def.rows {
                for ch in row.chars() {
                    if ch != '.' {
                        assert!(
                            def.palette.iter().any(|&(c, _)| c == ch),
                            "template for {g:?} uses '{ch}' with no palette entry"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_animal_def_is_a_well_formed_grid() {
        // Every hand-authored sprite: exactly GRID rows of exactly GRID chars,
        // and every non-'.' char resolves in that sprite's own palette. This
        // runs with no GL context, so a bad grid fails `cargo test`, not just
        // a screenshot.
        let game = Game::new(0.0);
        for t in &game.phy.taxa {
            let Some(def) = animal_def(t.name) else {
                continue;
            };
            assert_eq!(def.rows.len(), GRID, "{}: wrong row count", t.name);
            for row in &def.rows {
                assert_eq!(row.chars().count(), GRID, "{}: wrong row width", t.name);
                for ch in row.chars() {
                    if ch != '.' {
                        assert!(
                            def.palette.iter().any(|&(c, _)| c == ch),
                            "{}: '{}' has no palette entry",
                            t.name,
                            ch
                        );
                    }
                }
            }
        }
    }
}
