#!/usr/bin/env python3
"""Regenerates the data half of `src/sprites.rs` from the Python sprite defs.

    python3 scripts/pixel-art/gen_rust.py

Only touches `animal_def` and `template_def` -- everything below the
"building textures" marker in sprites.rs (the texture builder, the animation
curves, the Sprites registry, the tests) is hand-written Rust and this script
never rewrites it. Run `cargo fmt` afterward.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from animals import ANIMALS
from templates import TEMPLATES

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT_PATH = os.path.join(REPO_ROOT, "src", "sprites.rs")
MARKER = "// --- building textures "


def emit_def(name, rows, pal, indent="    "):
    lines = [f'{indent}"{name}" => SpriteDef {{', f"{indent}    rows: ["]
    lines += [f'{indent}        "{r}",' for r in rows]
    lines.append(f"{indent}    ],")
    pal_items = ", ".join(f"('{c}', 0x{h:06X})" for c, h in pal.items())
    lines.append(f"{indent}    palette: &[{pal_items}],")
    lines.append(f"{indent}}},")
    return "\n".join(lines)


def main():
    out = []
    out.append("//! Hand-authored pixel art, generated from `scripts/pixel-art/`.")
    out.append("//!")
    out.append("//! Do not hand-edit the sprite data below -- regenerate it from the Python")
    out.append("//! source (see `scripts/pixel-art/README.md`) so the two never drift.")
    out.append("//! Everything else in this file (the texture builder, the animation curves,")
    out.append("//! the lookup) is hand-written and safe to edit directly.")
    out.append("")
    out.append("use macroquad::prelude::*;")
    out.append("")
    out.append("use crate::tree::{Group, Phylogeny};")
    out.append("")
    out.append("/// Every sprite is authored on the same small canvas. Small on purpose: it")
    out.append("/// keeps the whole roster quick to author and reads as a deliberate")
    out.append("/// minimalist style rather than as missing detail, once nearest-neighbour")
    out.append("/// scaling blows it up on screen.")
    out.append("pub const GRID: usize = 12;")
    out.append("")
    out.append("/// One sprite: a 12x12 grid of palette characters (row-major, `.` is")
    out.append("/// transparent) plus that sprite's own small palette.")
    out.append("struct SpriteDef {")
    out.append("    rows: [&'static str; GRID],")
    out.append("    palette: &'static [(char, u32)],")
    out.append("}")
    out.append("")
    out.append("// --- specific, hand-authored animals ----------------------------------------")
    out.append("")
    out.append("/// Looked up by `Taxon::name`. Every non-Backbone taxon in `tree.rs` has an")
    out.append("/// entry here; anything missing falls back to its clade's template.")
    out.append("fn animal_def(name: &str) -> Option<SpriteDef> {")
    out.append("    Some(match name {")
    for name in sorted(ANIMALS.keys()):
        rows, pal = ANIMALS[name]
        out.append(emit_def(name, rows, pal, indent="        "))
    out.append("        _ => return None,")
    out.append("    })")
    out.append("}")
    out.append("")
    out.append("// --- fallback templates, one per clade, plus the ancestor glyph -------------")
    out.append("")
    out.append("/// A `Group::Backbone` taxon is an inferred common ancestor, not a depicted")
    out.append("/// species -- it gets this abstract branching glyph instead of an animal.")
    out.append("/// Any other taxon without a specific sprite falls back to its clade's shape,")
    out.append("/// so every one of the 68 taxa always renders *something* animated.")
    out.append("fn template_def(group: Group) -> SpriteDef {")
    out.append("    let key = match group {")
    out.append('        Group::Backbone => "Ancestor",')
    out.append('        Group::Basal => "Basal",')
    out.append('        Group::Spiralia => "Spiralia",')
    out.append('        Group::Ecdysozoa => "Ecdysozoa",')
    out.append('        Group::Deuterostome => "Deuterostome",')
    out.append('        Group::Fish => "Fish",')
    out.append('        Group::Tetrapod => "Tetrapod",')
    out.append('        Group::Reptile => "Reptile",')
    out.append('        Group::Mammal => "Mammal",')
    out.append("    };")
    out.append("    match key {")
    for name in sorted(TEMPLATES.keys()):
        rows, pal = TEMPLATES[name]
        out.append(emit_def(name, rows, pal, indent="        "))
    out.append('        _ => unreachable!("every Group has a template"),')
    out.append("    }")
    out.append("}")
    out.append("")

    # Everything from the marker onward in the existing file is hand-written
    # Rust (texture builder, animation, Sprites, tests): keep it verbatim.
    with open(OUT_PATH) as f:
        existing = f.read()
    idx = existing.index(MARKER)
    hand_written = existing[idx:]

    with open(OUT_PATH, "w") as f:
        f.write("\n".join(out) + hand_written)
    print(f"regenerated {OUT_PATH}")


if __name__ == "__main__":
    main()
