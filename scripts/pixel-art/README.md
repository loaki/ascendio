# Pixel art authoring pipeline

`src/sprites.rs`'s sprite *data* (the two functions `animal_def` and
`template_def`) is generated from the Python here, not hand-written. The
*code* around it in that file -- the texture builder, the per-clade animation
curves, the `Sprites` registry, the tests -- is hand-written Rust and the
generator never touches it.

## Why Python, not Rust, for the data

Authoring a 12x12 pixel grid is trial and error: draw an ellipse here, nudge a
leg there, look at it, adjust. That loop wants a scripting language and a
quick raster preview, not a recompile. Python + Pillow gives both; the Rust
side only needs the finished grids.

## Files

| File | What it is |
|---|---|
| `helpers.py` | Grid primitives: `ellipse`, `rect`, `hline`/`vline`, `mirror_right`, etc. |
| `animals.py` | The 44 hand-authored animals, one function each, keyed by `Taxon::name`. |
| `templates.py` | The 9 clade fallback shapes (one per `Group`) plus the `Ancestor` glyph. |
| `preview.py` | Renders every def to one contact-sheet PNG, for eyeballing before you commit. |
| `gen_rust.py` | Regenerates `src/sprites.rs`'s data half from `animals.py` + `templates.py`. |

## Workflow

```bash
# 1. Edit or add a sprite in animals.py (or templates.py for a clade fallback).
#    A function per animal, using the helpers in helpers.py, ending in
#    add("Name", g, {'a': 0xRRGGBB, ...}).

# 2. Look at it before committing to Rust.
pip install pillow   # once
python3 scripts/pixel-art/preview.py /tmp/sheet.png
# open /tmp/sheet.png

# 3. Regenerate the Rust data and rebuild.
python3 scripts/pixel-art/gen_rust.py
cargo fmt
cargo test    # validates every grid is 12x12 with a complete palette
```

## Adding a new animal

Every non-`Backbone` taxon in `src/tree.rs` should have an entry in
`animals.py` keyed by its exact `Taxon::name`. If one is missing, that species
silently falls back to its clade's template from `templates.py` -- correct,
just generic. `cargo test` doesn't fail on a missing animal (the fallback is
intentional, not a bug), so check the contact sheet against `tree.rs`'s
`ROWS` table if you want to know what's still using a template.

## Format

12x12, row-major, one character per pixel. `.` is transparent; every other
character must be a key in that sprite's own `palette` dict (`cargo test`
catches a mismatch: `every_animal_def_is_a_well_formed_grid`). Each sprite
owns its palette rather than sharing a global one, so a sprite is fully
self-contained in its `add(...)` call.

## Why 12x12

Small enough that a whole animal is a few minutes of primitive calls, not an
afternoon of pixel-pushing -- 44 of them needed to happen once, and more will
get added over time. It also matches the game's stated minimalist style:
nearest-neighbour upscaling turns the low resolution into a deliberate chunky
look rather than reading as missing detail. If a future pass wants more
fidelity, `GRID` in `src/sprites.rs` and the canvas size in `helpers.py` are
the only two places size is assumed.
