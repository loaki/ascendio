# Pixel art pipeline

The sprite data in `src/sprites.rs` (everything above the "building
textures" marker) is generated from here. The texture builder, morphs,
animation and tests below the marker are hand-written Rust.

| File | What it is |
|---|---|
| `maps.py` | Every sprite as a 16x16 material map, keyed by `Taxon::name`, plus the `Ancestor` glyph and the six `Boon …` icons. |
| `palette.py` | The planet palette (from `backdrop.rs`) and the shadow/base/ink ramp for a colour. |
| `render.py` | The "rough ink" renderer: two tones, speckle, chipped edges, grit, worn outline. |
| `preview.py` | A contact sheet of sprites with their silhouettes, for checking they read. |
| `gen_rust.py` | Renders every map and writes the data half of `src/sprites.rs`. |

## Workflow

```bash
pip install pillow   # once, for the preview

# 1. Edit or add a map in maps.py.
# 2. Look at it, colour and bare silhouette side by side.
python3 scripts/pixel-art/preview.py /tmp/sheet.png "T. rex" Mammoth
# 3. Bake it into Rust.
python3 scripts/pixel-art/gen_rust.py
cargo fmt && cargo test
```

`cargo test` fails if any non-`Backbone` taxon in `src/tree.rs` has no map.

## Maps

A map is 16 strings of 16 letters plus a dict of base colours:

- `.` is empty.
- `b` body, `l` belly or pale part, `d` dark marking, `f` limbs or fins, and
  `r`/`g` accents. Each takes its colour from the dict; `render.py` snaps it
  to the palette and shades it.
- `e` eye white, `k` eye, `w` glint, `t` ivory and `m` mouth have fixed
  colours. `k` pixels become `*` in Rust, so morphs can recolour eyes.

`mirror(half)` builds a symmetric sprite from its left 8 columns.

## Style rules

- Vertebrates in side profile facing right. Flat animals (beetle, butterfly,
  starfish, trilobite) from above. Round ones (jellyfish, octopus, crab) from
  the front.
- Exaggerate the one feature a child would draw first: the shark's fin, the
  elephant's trunk, the snail's spiral, the T. rex's tiny arms.
- Check the silhouette in the preview. If the black shape alone doesn't say
  what the animal is, redraw the shape rather than adding detail.
- Sprites sharing a clade need different silhouettes (Shark and Megalodon,
  Elephant and Mammoth, Ape, Chimpanzee, Gorilla and Human).

The noise is seeded by the animal's name, so the output is stable between runs.
