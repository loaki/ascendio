# Ascendio

A daily game about the animal tree of life. You don't play the animals:
**you shape the planet.** Raise land, grow forests, change the air and the
climate, then let time run. A few real hours become millions of years, and
when you come back a **new genome** has evolved. Mutate it until it
detonates to find out what evolved, and sometimes it's something rare.

A session is about two minutes and there are at most four a day. What stops
you is that the planet needs time, not a paywall.

The full design (every rule, number and all 151 taxa's needs) is in
[`docs/DESIGN.md`](docs/DESIGN.md). This README covers how it is built and
how to run it.

## The loop

1. **Shape.** Spend adjustment points (3 to start) on five levers: Land (sea
   level), Vegetation, Oxygen, Temperature, Volcanism. Points are measured
   against where the phase started, so stepping a lever back is free. The
   backdrop changes as you go, and the panel says what the cycle will change
   by itself (forests raise oxygen, volcanoes warm the planet, ice lowers
   the sea).
2. **Let time run.** The first cycle takes 1 minute, the second 20 minutes.
   After that, tapping `LET TIME RUN` raises a dial around the spiral
   (`src/dial.rs`): swipe or scroll right/left, or use the arrow keys, to pick 2h
   to 6h in half-hour steps, while the panel shows what the wait and your
   keystones will bring. Longer waits pay more per hour (`src/wait.rs`):
   one card per hour, luck that grows with the square of the time, morphs
   x1.5 from 4h, and at 6h morphs x2 plus a guaranteed Rare. So one 6h wait
   beats two 3h ones. Each hour is 10 Ma. The countdown runs on the wall
   clock, so it keeps going with the app closed. Nothing can be changed
   until it ends.
3. **Evolve the genome** (`src/opening.rs`). A 3D double helix gives no
   hint of what's inside. Each of three taps mutates a third of its base
   pairs; the third collapses it into a core that detonates like a supernova
   in the best card's colour, and the cards fly out face down. Flip them in any order; the best one is locked until
   last. A new species gets a full reveal (silhouette, clues, then colour),
   and a duplicate becomes a specimen that levels its animal up.
4. **Steer.** Pick 1 of 3 boons for the next cycle (Lure a clade, +1 card, a
   guaranteed Rare, x4 morphs, less wait, +2 points), and equip up to 3
   discovered animals as **keystones**. Only keystones give their bonus, and
   only while the planet suits them: an unsuited one is dormant.

What a genome can hold is filtered twice (`src/genome.rs`): the tree
(parent found, itself not) and the planet (each taxon's habitat and needs
in `src/ecology.rs`, e.g. frogs need fresh water, dragonflies need 28% O2).
Then each card rolls a rarity tier (60/25/11/3.5/0.5%, with pity at 3, 10
and 40 genomes) and a morph (giant 1/20, albino and melanistic 1/64, amber
1/512). Nothing is ever sold.

The mass extinction (volcanism 3, followed by an all-Rare "radiation"
genome) is designed in `docs/DESIGN.md` but not implemented yet.

## The screens

- **Spiral** (main): the HUD (millions of years let run so far, the era,
  your most advanced animal), the planet backdrop, the lineage coil over it,
  the lever panel (a countdown while time runs), and the bottom bar:
  `Keystones` · the action button · `Map`. The action button says
  `LET TIME RUN`, then `LET 4H RUN` once the wait dial is up (the lever
  panel becomes the rewards panel, with `BACK` to reshape), then the
  countdown, then `EVOLVE IT`.
- **Map**: the whole discovered tree as a cladogram. Tap a node for its page.
- **Animal page**: its needs checked against your planet right now, its
  keystone bonus, level, the morphs you own and how many Ma in it evolved.
- **Keystones**: the equipped slots, then your whole collection to pick from.
  Animals the planet can't support right now are dimmed; an equipped one
  shows DORMANT and why.

## Engine choice: macroquad

Picked over Bevy for this game specifically:

| | macroquad 0.4 | Bevy |
|---|---|---|
| APK size | ~5–10 MB | ~40 MB |
| Cold start on low-end Android | sub-second | 1–3 s |
| Model | immediate mode, one loop | ECS, systems, plugins |
| Cost here | must hand-roll UI and layout | ECS overhead buys nothing for a static tree |

A minimalist 2D incremental game draws a few hundred rectangles and some text.
It never needs an ECS, and on Android the size and startup difference is what
players actually feel. Bevy would be the right call if this grew into something
with physics, 3D, or a large systemic simulation.

## Run it (desktop)

```bash
cargo run --release
```

| Input | What it does |
|---|---|
| `-` / `+` on a lever | Spend or refund a point (shaping phase only) |
| Bottom action button | Let time run (raises the wait dial) · confirm the wait · (countdown) · express the genome |
| Swipe or scroll right/left, arrow keys | With the wait dial up: longer/shorter wait, 2h to 6h in half-hour steps (the spiral stays still) |
| Tap the genome / cards | Mutate it, then flip cards; tap again to move on |
| Tap the focused animal on the coil | Open its page |
| Tap a branch or ancestor on the coil | Reroute or scroll the lineage to it |
| Swipe the coil / wheel | Scroll the lineage |
| Pinch closed on the spiral, `Map` tab, or `M` | Open the map (tap `Map` again to go back) |
| `Keystones` tab | Open the keystones; tap again to return |
| `Esc` | Save and quit |

### Dev flags

```bash
# A scratch game that shapes, runs and opens cycles by itself.
ASCENDIO_AUTOPLAY=1 cargo run --release

# Make time pass 3600x faster (an hour per second) on a real save.
ASCENDIO_TIME_SCALE=3600 cargo run --release

# Dev keys: T ends the running cycle now, R resets to a fresh game.
ASCENDIO_DEV=1 cargo run --release

# Write a PNG of the framebuffer after N seconds and exit, without touching
# the real save. ASCENDIO_SHOT_MODE=map|spiral|dial|keystones|settings picks the screen.
ASCENDIO_SCRATCH=1 ASCENDIO_SHOT=shot.png ASCENDIO_SHOT_AFTER=2 cargo run --release

# Start on a ready genome and tap the opening N times (0.5 s apart), for
# capturing its stages.
ASCENDIO_SCRATCH=1 ASCENDIO_DEMO_TAPS=6 ASCENDIO_SHOT=open.png ASCENDIO_SHOT_AFTER=5 cargo run --release

# Wipe any existing save first -- see what a fresh install sees.
ASCENDIO_FRESH=1 cargo run --release

# Force the planet's five levers (land,vegetation,oxygen,temperature,volcanism).
ASCENDIO_SCRATCH=1 ASCENDIO_PLANET=3,4,4,3,0 cargo run --release
```

`ASCENDIO_W` / `ASCENDIO_H` override the window size. None of these exist on
the web build, which always plays with real timing.

## Play it on your phone (the dev loop)

The fastest way to test on a real phone is the web build — no Android SDK, no
NDK, no cable, and about 20 seconds per iteration. macroquad compiles to wasm
and miniquad forwards real touch events, so drag, tap and pinch all behave the
way they will in the APK.

```bash
rustup target add wasm32-unknown-unknown   # once
make serve
```

`make serve` builds the wasm, copies it into `web/`, prints your LAN address and
serves it. Put the phone on the same wifi and open the printed URL. Add it to
the home screen and it opens fullscreen with no browser chrome.

Each iteration after that is `make serve` again (Ctrl-C first) and a reload on
the phone. `make ip` just prints the URL. Change the port with
`make serve WEB_PORT=9000`.

### If the phone times out

**1. A firewall dropping the port.** With `ufw` set to drop incoming
traffic, dropped packets give a *timeout* rather than "connection refused",
which is the tell. Allow the port on the LAN only:

```bash
sudo ufw allow from 192.168.1.0/24 to any port 8000 proto tcp
sudo ufw status            # confirm the rule landed
```

`make serve` prints this line with your actual subnet and port filled in.

**2. The wrong address.** A machine on the LAN twice (ethernet and wifi),
or running docker or a VPN, has several addresses, and `hostname -I` lists
them in arbitrary order. `scripts/lan-ip.sh` asks the kernel instead: it
takes the source address chosen to reach the default gateway, which is by
definition on the LAN the phone shares. That is what `make ip` and
`make serve` print.

The server binds `0.0.0.0`, so either address works once the firewall allows
it, but prefer the one `make ip` gives.

What this does and does not tell you:

| | web build | APK |
|---|---|---|
| Layout, gestures, game feel | accurate | accurate |
| Touch, pinch, drag | real touch events | same |
| Frame pacing / startup time | slower than native | the real numbers |
| Binary size | ~820 KB wasm | measure separately |

So iterate on the web build, and use the APK to check performance and size.

Other causes worth checking: a wifi network with client isolation on, or the
phone silently sitting on mobile data instead of wifi.

### How the web build is wired

`web/gl.js` is miniquad 0.4.11's loader, copied verbatim out of the crate
source so it can never drift from the version the wasm was built against.
macroquad 0.4.16 sets `default = []`, so the audio feature is off and none of
the extra `mq_js_bundle.js` parts apply. Most of the wasm's JS imports
resolve against that one file; `quad-storage.js` and `sapp_jsutils.js` are
the other two, vendored the same way (from their crates' own `js/`
directories, which crates.io's published tarballs don't include) for the
save plugin -- `web/index.html` loads all three, in that order, before the
wasm. `.cargo/config.toml` adds `-C link-args=--allow-undefined` for the
wasm target: those two crates' functions are genuinely only defined in JS,
supplied at instantiation time, and `rust-lld` needs to be told that's
expected rather than treating it as a missing symbol.

`web/index.html` sets `touch-action: none` on the canvas and
`user-scalable=no` in the viewport, without which the browser keeps the drag and
pinch gestures for itself and the game never sees them.

## Build for Android

Requires Docker. **This path is not yet verified on this machine** — no Android
SDK/NDK is installed here, so the desktop build is the only one that has been
run end to end.

```bash
make apk        # -> target/android-artifacts/release/apk/ascendio.apk
make install    # adb install -r
```

`make apk` wraps the macroquad-maintained toolchain image:

```bash
docker run --rm -v "$PWD":/root/src -w /root/src \
  notfl3/cargo-apk cargo quad-apk build --release
```

Android metadata (package name, SDK levels, orientation) lives under
`[package.metadata.android]` in `Cargo.toml`. If that image proves too dated,
the fallback is [`xbuild`](https://github.com/rust-mobile/xbuild), which
targets current NDKs.

### The "genome ready" notification

Off by default; the player turns it on from the gear in the top bar. The
game is usually closed when a wait ends, so `src/notify.rs` hands the end of
the wait to Android's JobScheduler, which posts the notification on time
(give or take a minute; Doze can push it later). The Java side is spliced in
by `cargo-quad-apk` through `quad.toml`: `java/com/loaki/ascendio/GenomeJob.java`
(the job and the notification) and `android/main_activity_inject.java` (the
methods the game calls on the activity over JNI). The notification's status
bar icon is the Ancestor silhouette from `scripts/pixel-art/icon.py`. The
web and desktop builds show the setting greyed out.

## Layout

```
src/
├── main.rs      loop, screens, input wiring, dev flags
├── game.rs      the state machine: shape -> running -> genome -> boon
├── planet.rs    the five levers, linked levers, habitats, the cycle timer
├── wait.rs      the 2h-6h wait and what each length is worth
├── dial.rs      the wait dial drawn around the spiral
├── ecology.rs   every taxon's habitat, needs, rarity tier and keystone bonus
├── genome.rs    what a genome can hold: eligibility, tiers, pity, morphs
├── opening.rs   the genome's mutate-and-supernova opening (the "pack opening")
├── genome_bg.rs the abyss and supernova light behind the opening
├── backdrop.rs  the pixel-art planet cross-section behind the spiral
├── pixel.rs     the dithered low-res canvas both backdrops paint into
├── ui.rs        HUD, lever panel, bottom bar, keystones, animal page, boons, settings
├── settings.rs  the player's settings, saved apart from the game
├── notify.rs    the "genome ready" notification (Android, over JNI)
├── render.rs    text helpers, the spiral coil and the map
├── tree.rs      the 151-taxon table + Phylogeny
├── spiral.rs    the lineage spine, its branch spurs, and the coil's geometry
├── layout.rs    tidy left-to-right tree layout for the map
├── view.rs      camera (pan/zoom/fit) and gesture recognition
├── sprites.rs   pixel art: texture builder, morph recolours, animation
├── facts.rs     "did you know" trivia shown while time runs
└── save.rs      where the save data actually lives
docs/
├── DESIGN.md      the game design: rules, numbers, the 151 taxa
└── PHYLOGENY.md   sources, citations, and every simplification we made
```

The rules (`planet.rs`, `ecology.rs`, `genome.rs`, `game.rs`) are pure
logic with no drawing and a seedable RNG, and they carry most of the tests:
every taxon can live on some reachable planet, pity only fires when
something that rare can drop, a water world never yields a land animal, and
a long simulated game keeps progressing.

The coil is cheap because a logarithmic spiral is self-similar: a position
depends only on its signed distance along the spine from whatever is in
focus, so there is no world space and no camera. Which lineage it shows is
one preferred child per taxon.

## The art

Every taxon renders a 16x16 sprite in one "rough ink" style, nearest-filtered
so it stays crisp when blown up. The style is built to sit in the planet
backdrop: every colour comes from `backdrop.rs`'s own palette, half-tones use
the same speckle, and each sprite has two tones, a few chipped edge pixels
and a dark outline that breaks in places.

- **One sprite per species**: every non-`Backbone` taxon has its own art, in
  its most recognisable pose (vertebrates in side profile, flat animals from
  above, round ones from the front) with one feature exaggerated. A
  `Group::Backbone` taxon (Mollusca, Arthropoda, ...) is drawn as its first
  descendant species; only the root has its own `Ancestor` glyph.
- **Morphs** are palette shifts built at startup (`sprites::Sprites::build`):
  albino, melanistic and amber recolour every pixel by luminance, keeping the
  outline dark and treating the eye (`*`) separately. A giant is the normal
  art drawn 1.45x.

Animation is entirely procedural — `sprites::pose()` is a pure function of
`(clade, taxon id, wall-clock time)`, so nothing needs per-frame state. Each
clade moves differently: land animals bob as if stepping, fish and arthropods
flick and flutter, sessile marine life (sponges, corals, starfish) just
pulses, and the ancestor glyph breathes. A golden-angle offset per taxon id
means sprites never move in lockstep.

An **undiscovered** taxon shows its real sprite tinted near-black, a
Pokedex-style silhouette tease, on the map, on the animal page and in the
opening's reveal. The spiral's labelled chips get an icon-then-name
treatment; distant beads fall back to a tiny animated icon, and locked ones
further out are a bare dot.

The **planet backdrop** (`src/backdrop.rs`) is drawn per pixel into a
180-wide image with 4x4 Bayer dithering, then scaled up with nearest
filtering. It is a sea-level cross-section: the Land lever moves the
waterline, three ridge layers rise from the seabed, and whatever breaks the
surface gets vegetation, trees, snow or a volcano. Sky and water palettes
shift with temperature, oxygen and volcanism, and every lever eases toward
its new value, so lowering the sea is animated.

The sprites are authored as 16x16 material maps in `scripts/pixel-art/`,
rendered by a small Python pipeline and baked into `src/sprites.rs`; see
its README for the workflow.

## Tuning

The numbers live next to the rules they tune, and all are first guesses
that need playtesting:

- `wait.rs`: the wait range, `MA_PER_HOUR` and the reward curve in `bonus`.
- `planet.rs`: `cycle_seconds` (1 min, 20 min, then the chosen wait), `BASE_POINTS`, and
  the linked-lever rules in `Planet::after_cycle`.
- `ecology.rs`: `TABLE`, every taxon's needs, tier and bonus.
- `genome.rs`: the tier weights in `tier_weights`, the pity windows, and the
  morph odds in `roll_morph`.
- `game.rs`: `LEVEL_STEPS` (specimens per level), keystone slot thresholds,
  and each bonus's per-unit effect in `Game::effects`.

## Saving

`Game::save`/`Game::load` round-trip every durable field (collection,
specimens, morphs, the planet, the running cycle, the waiting genome, pity
and the RNG state, keystones, boons) through `serde_json`, using
[`quad-storage`](https://docs.rs/quad-storage): browser `localStorage` on
wasm, a local `local.data` file natively. The save carries a version (2); a
save from the old DNA-pool game fails to parse and a fresh game starts.
It autosaves every 3 seconds, on every phase change, and on `Esc`.

## Not done yet

- The mass extinction and its radiation genome are designed, not built.
- No audio or haptics yet. The genome opening especially wants sound.
- No leaderboard: the "most advanced animal" (deepest step) is shown in the
  HUD, but nothing is shared between players yet.
- Android build unverified.
- `Lion` still stands for the whole cat family (Felidae): Tiger, Cat and
  Sabre-tooth hang from it -- see `docs/PHYLOGENY.md`.

## Data

Topology and dates come from the phylogenomic literature (Laumer et al. 2019,
Irisarri et al. 2017, Misof et al. 2014, Upham et al. 2019, Prum et al. 2015,
TimeTree 5). Full citations and the list of deliberate simplifications are in
[`docs/PHYLOGENY.md`](docs/PHYLOGENY.md).
