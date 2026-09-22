# Ascendio

An incremental mobile game about the animal tree of life. You start as the
**Urmetazoan** — the last common ancestor of every animal — and every Evolve
is a roll for a descendant lineage to split off. Unlock all 68 taxa and the
whole phylogeny is yours to explore.

**Status: playable prototype.** Every taxon has hand-authored or fallback
pixel art with per-clade idle animation now; there is still no audio. What
works is the full loop — DNA accrues into a capped pool, Evolve spends it
once full, and every roll pays out — plus persistence (see "Saving" below),
plus all three screens, plus the art.

## The loop

The whole design follows from one constraint: **a session should run out of
things to do in three minutes**. Most idle games reward unlimited engagement,
so a session never ends. This one is built to end.

**DNA** accrues every second from every discovered *species* and collects in a
**capped pool**. The refill wait is **logarithmic in how much you have
discovered** (`Game::refill_seconds`): a few seconds for your very first
Evolve, gently longer as the tree fills in, never the same flat wait on click
one that it is a hundred discoveries later. The cap itself also grows with
progress, not with DNA/s — see the note on `fill_rate` below for why.

Only species can produce DNA, and even then only if that is the bonus their
category actually gives — see "A species gives its category's bonus" below.
A category node — Mollusca, Arthropoda, Chordata, and the rest of the tree's
internal "common ancestor" taxa (`Group::Backbone` in `tree.rs`) — is not an
animal, so it never earns anything itself. Discovering one instead offers a
**choice**: three random powers (see "The upgrades tab" below), and picking
one activates that power, free, at level 1 -- shared game-wide, not owned by
that one category (see "One upgrade per `Kind`" below). Buying it *further*
is what costs DNA, with no ceiling on how many times.

**The pool's fill speed is not simply "DNA/s."** Early in the game a run of
nothing but categories (which produce no DNA) must not leave the pool stuck
at zero forever — so `fill_rate` is a guaranteed, always-positive baseline
derived from the same logarithmic curve as the wait, with actual species
output added on top as a bonus. The `+X/s` on the HUD is that species output
alone, shown honestly; the pool itself usually fills a bit faster than that
number implies, especially early on.

**Evolve only fires on a full pool, and it is global.** The button sits
centred in the bottom bar — between `Upgrades` and a `Leaderboard` placeholder
that has no screen yet — not on the card, and not tied to whatever the spiral
happens to be showing. It is reachable from any of the three screens. While
the pool is charging it shows a percentage; once full, it becomes `EVOLVE`.
One big, satisfying roll per fill cycle, not a stream of small ones — this is
the mechanism that makes a session end.

**Evolve can land on any eligible lineage** — any unlocked taxon with an
undiscovered direct child, species or category alike — picked at random when
you tap it, not necessarily whatever is on screen. It plays out as an overlay
on top of whichever screen you tapped it from — Spiral, Map, or Upgrades —
so Evolve never changes what you were looking at.

**Evolve always discovers something — win or lose the roll, and never a
level-up.** The first new taxon (a random undiscovered direct child of
whichever lineage got picked) is unconditional; nothing is rolled for it.
What `unlock_chance` governs instead is how much *further* that discovery
reaches: past the guaranteed first find, each additional generation gets its
own roll at continuing deeper, so a higher chance is a real shot at landing
on a more advanced animal in one Evolve, not a binary success/fail any more.
`Game::evolve` walks a chain — first child, then (if the roll keeps
succeeding) its child, and so on — unlocking every taxon along the way, so a
lucky roll can discover several generations at once. It stops the moment a
roll comes up short, a category needs its choice made first (see "The
upgrades tab" below), or there is nothing left to advance into. The card
still shows the odds of that extra reach (`"71% chance of a more advanced
find"`) purely as information; tapping the card itself doesn't evolve
anything — it fills that species' own tap-level bar instead (see "Levelling
an animal by hand" below). A category's card leaves the odds line off — its
payoff is a choice, not a percentage (see "The upgrades tab").

A roll that doesn't carry past the first generation still adds **+1
mutation pressure**, raising the odds on that lineage until it breaks — the
"Chance" upgrades still work exactly the same way, just biasing *how far*
now instead of *whether*. A roll that does carry past the first generation
resets pressure to 0.

**The reveal is a two-part animation** (`render::draw_evolve_fx`), not just a
toast. First, DNA particles stream from the top bar down into the Evolve
button — the spend, made visible, not just a number changing. Then a panel
appears with the result, labelled **`DISCOVERED`**: the sprite snaps in from
a dark, undersized silhouette to full size and colour with a small
overshoot-bounce, a fading glow ring behind it, and its name fading in last —
the *deepest* taxon reached, if the roll cascaded past the first one (the
earlier ones along the chain are still unlocked, just not individually
celebrated here). Once fully revealed, **the card just holds there** — no
timer clears it — with a fading "tap to continue" hint; it takes a tap
(anywhere, on any screen) to dismiss, and that tap does nothing else, so it
can never accidentally also trigger something underneath. Discovering a
category chains straight into its choice screen once this is dismissed —
see "The upgrades tab" below.

**The whole pool is the bet, every time.** Evolve always commits everything
you have banked — that is what "how much you spent" means here, since you can
only ever act right when the pool is full. The chance of reaching further is
the usual depth/pressure/upgrade formula *plus* a bonus for how many baseline
rolls your full pool represents (`ln(pool ÷ one roll's baseline cost)`),
clamped at the same 90% ceiling as everything else. A richer economy reaches
deeper more reliably — that is the payoff for growing the tree, not just a
bigger number on screen.

Every DNA amount shown — the pool, its cap, an upgrade's price — is a whole
number; only the DNA/s rate keeps decimals, since early on that is
legitimately below 1.

**Levels compound down the lineage.** A species' *power* — how much of
whatever it gives (see below) it's worth — is multiplied by
`1 + 0.02 × (sum of every ancestor's level)`, so investing in the trunk keeps
paying long after you have moved past it, and early species never become
dead weight. A category never levels up at all any more, and nor does Evolve
ever level a species either — the only way anything gains a level past 1 is
by hand (see "Levelling an animal by hand" below). A category's card doesn't
show a `Lv` number at all: its own visible progression is the upgrade level
instead (see "The upgrades tab" below).

**A species gives its category's bonus, not DNA/s unconditionally.**
`Game::species_bonus` inherits `nearest_category_kind`, the same lookup the
card's colour comes from, and redirects that species' power into whichever
stat it names: a `Rate`-powered species gives DNA/s exactly as it always
has; a `Cost`-powered one instead discounts upgrade prices; `Chance` adds
Evolve odds; `Cap` grows the pool; `Tap` speeds up tap-levelling (see below);
`Storage` raises how far the DNA pool can overflow past its cap; `Vigor`
raises the starting level newly discovered species get (see "A default level
on discovery" below). Every one of those (`Rate` aside) is flat, not a
percentage — see "The upgrades tab" below for why. A species' power grows
close to exponentially with depth, level and lineage, which would blow up a
linear formula for one deep, leveled animal — every kind but `Rate` damps it
through a log first (`ln(1 + power)`) before applying that flat amount, so it
is worth more the deeper and more leveled it gets without ever being worth a
runaway amount. The card's right-hand meta number
(`render::species_earn_label`) shows whichever of these is actually true for
that species — `+X/s`, `-X cost`, `+X% chance`, `+X cap`, `+X tap`,
`+X overflow`, `+N start lvl(s)` — not always "+X/s" any more, and every
number is rendered through `render::compact`, which falls back to plain
scientific notation (`1.23e18`) past the largest named unit (`Qa`, 10^15)
rather than ever print an untiered, wrong-looking number.

**Levelling an animal by hand.** Tapping a species' card — the same card
Evolve's roll targets, informational until now — fills a small bar of its
own (`Game::tap_level`) and grants it a level once full, independent of the
Evolve gate entirely: free, always available, no DNA spent. How many taps
that takes is logarithmic in the species' *current* level
(`Game::tap_level_threshold`) — a handful for its first few levels, gently
more as it climbs, the same "cheap at first, never a flat wall" shape as the
pool refill curve, rather than one flat number forever. It only works on an
unlocked species, never a category (whose card isn't a button this way —
see "The upgrades tab"), and it's what gives a leaf species — one with no
descendants left to discover, so Evolve has nothing left to do with it at
all — something to do besides sit there once found. A small "tap to level up"
label sits directly on top of the bar it explains (not up near the name),
and any `Kind::Tap` anyone has (category or species) adds flat extra fill to
*every* tap, on every animal (`effects.tap_bonus`). Both bars
can show on the same card: the pressure/odds one near the top of the bottom
section (species with a branch still to roll on), and the tap-level one
beneath it (every species, always).

## The two views

**The spiral** is the game. A logarithmic coil fills most of the screen,
carrying the focused species' **lineage** — ancestors winding tightly into the
centre, descendants opening outward — with a "did you know" trivia line
(`facts.rs`) sitting just under the top HUD bar, and the focused species' own
card pinned to the bottom, just above the bottom bar. Dragging scrolls along
the coil a generation at a time. If you happen to be here when Evolve fires,
the card itself also punches with scale and a flash of light on top of the
main reveal overlay — a bonus if you're looking, not something you have to be
here for.

The coil is a *spine*: a single root-to-tip lineage where every taxon really is
the child of the one before it. That constraint matters. An earlier version
walked the whole tree depth-first, which put Cnidaria directly before
Protostomia on the coil and read as though one descended from the other — they
are sister groups, both children of the root. A test now enforces the
invariant: consecutive taxa on the coil are always a real parent-child edge.

Scrolling is a **rotation about the coil's centre**, not an up/down drag. It
has to be: both neighbours of the focus sit *below* it on screen — the coil
leaves the focus heading down in both directions — so vertical drag is
ambiguous and reads as inverted half the time. Turning the coil is direct
manipulation: whatever is under your finger stays under your finger, so
swiping left brings descendants in and swiping right winds back toward the
root. Releasing mid-swipe flings, with friction, and snaps to a generation.

Branching is what the coil shows instead. Every child a taxon has that the
spine does *not* continue through sprouts outward as a **spur**. The focused
species' spurs are labelled and tappable — tapping one reroutes the spine
through that branch — and spurs further along the coil stay as dots, so you can
see where the tree forks without the labels colliding.

**The map** is the overview: the whole discovered tree as a left-to-right
cladogram, the way phylogenies are normally drawn. Pinch closed from the
spiral to open it. Tapping a taxon opens its card right there, on the map --
the same card the spiral shows, just pinned in place instead of jumping you
to the spiral; tapping empty space closes it again. Every node's sublabel
shows its level too (`"Lv 3  ·  740 Ma  32%"`, `render::sublabel`) for a
species -- a category still leaves its own level off, same as its card. The
top-right button is always labelled with where it goes (`MAP` from the
spiral, `SPIRAL` from the map), rather than naming the screen you're already
on. An Evolve pins its card open here too, exactly like the spiral --
whichever view you're on when you tap `EVOLVE`, it shows you what just
happened.

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

Controls:

| Input | Spiral | Map | Upgrades |
|---|---|---|---|
| Bottom bar's `EVOLVE` (pool full) | Roll on a random eligible lineage, from any screen | | |
| Bottom bar's `EVOLVE` (charging) | Nothing — shows a percentage | | |
| Tap the card (a species) | Fill its tap-level bar; levels it up once full | — | — |
| Tap the card (a category) | Nothing — its progression is the upgrade shop | — | — |
| Tap a branch spur | Reroute the lineage through it | — | — |
| Tap an ancestor on the coil | Scroll back to it | — | — |
| Tap a species | — | Open its card, right there | — |
| Tap the map's open card | — | Same as the spiral card, above | — |
| Tap empty space on the map | — | Close whatever card is open | — |
| Tap a `BUY` row | — | — | Purchase that upgrade |
| Swipe | Scroll the lineage; flings with momentum | Pan | Scroll the list |
| Scroll wheel | Step one generation | Zoom | — |
| Pinch closed | Open the map | Zoom out | — |
| `MAP` / `SPIRAL` / `BACK` button, or `M` | Switch view | Switch view | Back |
| Bottom bar's `Upgrades` tab, or `U` | Open it | Open it | Back |
| Bottom bar's `Leaderboard` tab | Placeholder — no screen yet | | |
| `F` | — | Fit the whole tree on screen | — |
| `R` | Reset to a fresh game | | |
| `Esc` | Quit | | |

### Dev flags

```bash
# Auto-tap random unlocked taxa -- fills the tree for demos and screenshots.
ASCENDIO_AUTOPLAY=1 cargo run --release

# Play for 30s, then write a PNG of the game's own framebuffer and exit.
ASCENDIO_AUTOPLAY=1 ASCENDIO_SHOT=shot.png ASCENDIO_SHOT_AFTER=30 cargo run --release

# Run the economy 250x faster, so hours of accrual happen in seconds.
ASCENDIO_TIME_SCALE=250 ASCENDIO_AUTOPLAY=1 cargo run --release

# ASCENDIO_SHOT_MODE=map|spiral forces a view for the capture; unset uses
# whatever is live. ASCENDIO_W / ASCENDIO_H override the window size.
ASCENDIO_W=1080 ASCENDIO_H=2400 ASCENDIO_SHOT_MODE=map ASCENDIO_SHOT=map.png \
  ASCENDIO_AUTOPLAY=1 cargo run --release

# Wipe any existing save first -- see what a fresh install actually sees.
ASCENDIO_FRESH=1 cargo run --release
```

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

Two things bite on this machine, and both were hit the first time:

**1. `ufw` is enabled with `DEFAULT_INPUT_POLICY="DROP"`.** Dropped packets give
a *timeout* rather than "connection refused", which is the tell. Allow the port
on the LAN only:

```bash
sudo ufw allow from 192.168.1.0/24 to any port 8082 proto tcp
sudo ufw status            # confirm the rule landed
```

`make serve` prints this line with your actual subnet and port filled in.

**2. This machine is on the LAN twice** — a USB ethernet dongle
(`enx…`, 192.168.1.58) and wifi (`wlp0s20f3`, 192.168.1.60). `hostname -I`
returns them in arbitrary order, along with every docker bridge and the VPN
tunnel, so it is not a reliable way to pick one. `scripts/lan-ip.sh` asks the
kernel instead — it takes the source address chosen to reach the default
gateway, which is by definition on the LAN the phone shares. That is what
`make ip` and `make serve` print.

The server binds `0.0.0.0`, so either address works once the firewall allows
it, but prefer the one `make ip` gives.

What this does and does not tell you:

| | web build | APK |
|---|---|---|
| Layout, gestures, game feel | accurate | accurate |
| Touch, pinch, drag | real touch events | same |
| Frame pacing / startup time | slower than native | the real numbers |
| Binary size | 528 KB wasm | measure separately |

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

## Layout

```
src/
├── main.rs     loop, mode switching, input wiring, dev flags
├── tree.rs     the 68-taxon table + Phylogeny
├── game.rs     unlock state, tap rolls, pity counter
├── spiral.rs   the lineage spine, its branch spurs, and the coil's geometry
├── layout.rs   tidy left-to-right tree layout for the map
├── view.rs     camera (pan/zoom/fit) and gesture recognition
├── sprites.rs  pixel art: texture builder, per-clade animation, lookup
├── upgrades.rs the 6 upgrade Kinds, and their combined effect
├── facts.rs    "did you know" trivia shown while the pool charges
├── save.rs     where the save data actually lives (see "Saving")
└── render.rs   all drawing, for all three screens
web/
├── index.html        canvas + touch-action, for the phone dev loop
├── gl.js              miniquad's loader, vendored from the crate
├── quad-storage.js    the save plugin's JS half, vendored likewise
└── sapp_jsutils.js    a dependency of quad-storage.js, also vendored
scripts/
├── lan-ip.sh   the LAN address a phone can reach, via the default route
└── pixel-art/  the sprite authoring pipeline (see its own README)
docs/
└── PHYLOGENY.md   sources, citations, and every simplification we made
```

The coil is cheap because a logarithmic spiral is self-similar: a position
depends only on its signed distance along the spine from whatever is in focus,
so there is no world space and no camera — scrolling just changes one float.
Position comes from the spiral, but *size* peaks at the focus and falls off
both ways, so the species you are looking at is always the largest thing on
screen. Spurs reuse the same polar formula, pushed 1.9x further out — well
inside the next winding, which is 3.3x out.

Which lineage the coil shows is one `preferred child` per taxon. Rerouting to a
branch walks up from it setting those, then rebuilds the spine from the root.

The map layout is two passes: measure each subtree's height bottom-up, then
place nodes left-to-right inside the space that measurement reserved. Both the
thread and the map only recompute when something new is unlocked.

## The art

Every one of the 68 taxa renders a 12x12 pixel sprite, nearest-filtered so it
stays crisp when blown up. Two tiers:

- **44 hand-authored animals** (`scripts/pixel-art/animals.py`), one per
  non-`Backbone` taxon — Octopus, Shark, Elephant, Human, and so on.
- **9 clade fallback shapes plus an `Ancestor` glyph** (`templates.py`), so a
  taxon without a specific sprite still renders something recognisable
  instead of nothing. Every `Group::Backbone` taxon (Mollusca, Arthropoda,
  Chordata, ...) gets the `Ancestor` glyph rather than an animal, on purpose:
  those are inferred common ancestors, not depicted species.

Animation is entirely procedural — `sprites::pose()` is a pure function of
`(clade, taxon id, wall-clock time)`, so nothing needs per-frame state. Each
clade moves differently: land animals bob as if stepping, fish and arthropods
flick and flutter, sessile marine life (sponges, corals, starfish) just
pulses, and the ancestor glyph breathes. A golden-angle offset per taxon id
means 68 sprites never move in lockstep.

An **undiscovered** species on the focus card shows its real sprite, tinted
near-black — a Pokedex-style silhouette tease, not a blank "? ? ?". The map
draws a small icon beside every node's name, unlocked or not — a locked node
gets the same silhouette treatment next to its "???" label. The spiral's own
labelled chips (`render::draw_chip`) — every spur and every nearby bead along
the coil, not just the big focus card — get the same icon-then-name
treatment; distant, unlabelled beads fall back to a tiny animated icon with
no box at all, and locked ones further out are still a bare dot, so the coil
does not spoil an undiscovered shape at a glance from across the tree.

The Python pipeline that produced this (grid primitives, a contact-sheet
preview, and the generator that writes `src/sprites.rs`'s data) lives in
`scripts/pixel-art/`, with its own README on the authoring workflow. It is
a first pass, not a final one — some sprites (the primates especially) are
harder to tell apart than the clearer ones (Octopus, Crab, Whale).

## Tuning the game feel

All in `src/game.rs`, grouped by what they control. **These numbers are first
guesses and need playtesting** — especially the refill curve.

| Constant | Meaning |
|---|---|
| `BASE_OUTPUT` | DNA/s from a level-1 depth-0 species (0.20) |
| `OUTPUT_PER_DEPTH` | Output multiplier per generation (1.35) |
| `LINEAGE_BONUS` | Output added per ancestor level (0.02) |
| `MIN_POOL` | Floor the pool cap can never shrink below (40) |
| `CAP_PER_DISCOVERY` | Cap added per taxon found, beyond that floor (60) |
| `MIN_REFILL_SECONDS` | The very first refill's wait (5) |
| `REFILL_GROWTH` | How steeply the wait grows with progress (250) |
| `REFILL_SPREAD` | Discoveries per log-unit of that growth (8) |
| `ATTEMPT_BASE` | The baseline unit Evolve's spend bonus is measured against (8) |
| `ATTEMPT_PER_DEPTH` | Baseline multiplier per generation (1.75) |
| `ATTEMPT_PER_CHILD` | Baseline multiplier per child already found (0.35) |
| `BASE_CHANCE` | Success chance at depth 0, before any bonus (0.45) |
| `DEPTH_FALLOFF` | Per-generation rarity multiplier (0.88) |
| `MIN_CHANCE` / `MAX_CHANCE` | Floor and ceiling (0.04 / 0.90) |
| `PRESSURE_PER_MISS` | Odds added per failed Evolve (0.06) |
| `SPEND_BONUS_SCALE` | Odds added per `ln(pool ÷ baseline)` (0.16) |
| `UPGRADE_LEVEL_BASE` | DNA for a `Kind`'s first purchasable level (its second overall -- the first taxon to choose it already grants level 1 free) (50) |
| `UPGRADE_LEVEL_PER_LEVEL` | Dearer again for every level already bought -- one shared price curve per `Kind`, not per taxon (1.6) |
| `TAP_LEVEL_BASE` / `_GROWTH` / `_SPREAD` | Taps to fill a species' own level bar at its current level, logarithmic (`base + growth × ln(1 + level ÷ spread)`) (6, 5, 3) |

With `MIN_REFILL_SECONDS=5`, `REFILL_GROWTH=250`, `REFILL_SPREAD=8`, the
curve runs roughly: 5s at the very start, ~35s after one discovery, ~2 min
after five, ~5 min after twenty, under 10 min even with the whole tree found
— nothing like the flat, equal-every-time wait a constant multiplier of DNA/s
gives.

The baseline unit scales faster than DNA output (1.75 vs 1.35 per
generation), which is what makes a deep lineage's Evolve odds climb more
slowly than a shallow one's for the same pool size — depth is still real
friction, it just shows up in the spend bonus instead of in a per-tap price.

## The upgrades tab

The bottom bar's one tab (`src/upgrades.rs`). Every `Group::Backbone` taxon —
all 24 of them — still has exactly one flavour entry (`UPGRADES`, checked by
`every_backbone_taxon_has_exactly_one_upgrade`), so no category discovery is
ever a dead end even though it produces no DNA. But what a category's
upgrade *does* is a choice among only 7 possible `Kind`s, and **the tab lists
`Kind`s, not taxa** — 7 rows, always, never 24:

| Kind | Effect | Per level |
|---|---|---|
| Rate | Adds flat DNA/s | +0.5/s |
| Cost | Lowers the DNA cost of buying every upgrade's next level | -5 DNA |
| Chance ("evolve chance") | Adds flat percentage points to every Evolve's odds | +4pp |
| Cap | Adds flat DNA to the pool's cap | +25 |
| Storage ("DNA overflow") | Adds flat DNA to how far the pool can overflow past its cap | +40 |
| Tap | Adds flat extra fill to every tap on a species' level bar | +1 |
| Vigor ("starting level") | Adds bonus starting levels to newly discovered species | +1 level |

**Every kind is flat, deliberately.** Level `n` of any of them is worth
exactly `n × per_level` — never a compounding multiplier the way an early
version of this had it (`(1 + per_level)^n`, which made "+8% DNA" balloon
into a runaway number a few dozen levels in). A flat amount can never grow
faster than the level count itself, so buying up any one `Kind` forever is
always worth exactly what it says, both for a category's own purchased
levels (`upgrades::compute`) and for what a species of that kind
contributes (`Game::species_bonus`, damped through `ln(1 + power)` first so
one very deep, very leveled species still can't outrun everything else —
see "A species gives its category's bonus" above).

**Discovering a category is a deck-builder-style choice, not an automatic
grant.** Once its reveal card is dismissed, `render::draw_kind_picker`
offers 3 of the 7 `Kind`s above, drawn at random and distinct
(`Game::random_kind_options`); tapping one calls `Game::choose_upgrade_kind`.
Every other tap is gated until a choice is made — same as the reveal before
it. The taxon's real trait (`UpgradeDef::trait_name` — Arthropoda's
Exoskeleton, Chordate's Notochord, and so on) is still shown on *that*
choice screen, since it's about one specific just-discovered taxon — but it
is flavour, not a fixed mechanic: two playthroughs can have Arthropoda's
Exoskeleton doing completely different things, depending which of its 3
offered powers got picked, and several different taxa can end up expressing
the very same `Kind`. The root (`Urmetazoan`) is the one exception — it
starts already discovered, never passes through a choice moment, and keeps
a fixed `Rate` kind to match its "Multicellularity" flavour.

**One upgrade per `Kind`, not one per taxon.** `Game::kind_level` is a
single shared counter per `Kind` (`[u32; 7]`), not a per-taxon one —
picking a `Kind` for the first time anywhere activates it at level 1, free,
same as before; picking an *already-active* `Kind` again for a different
category still marks that taxon with it (so its own species colour and
`nearest_category_kind` resolve correctly), but does not grant a second free
level or add a second row to the tab. There are only ever 7 upgrades to buy,
full stop, however many of the 24 categories you end up discovering.
`Game::upgrade_kind[taxon]` (which `Kind` a taxon expresses, for colour and
inheritance) and `Game::kind_level(kind)` (how far that `Kind` has actually
been bought, shared) are deliberately two different things now.

**The upgrades tab shows what a `Kind` does, not which taxon it came
from.** Each row is titled with a short standalone name
(`Kind::title` — "DNA Production", "Cheaper Upgrades", "Evolve Odds", and so
on) and `Kind::description` underneath, in place of a taxon's flavour text —
there is no single taxon to credit any more, several categories can share
one `Kind`, so naming one would be arbitrary. A colour swatch stands in for
the old taxon sprite. What costs DNA is buying a `Kind` *further* — the row
shows its current effect and level on the left, and a `BUY` button for the
next level on the right (`Game::buy_upgrade_level`), at a price that climbs
with every level already bought (`UPGRADE_LEVEL_PER_LEVEL`) and falls with
every `Cost` kind anyone has (`effects.cost_reduction`, floored at 10% of the
undiscounted price so it can never reach zero — see "The upgrade shop gets
cheaper" below). There is no ceiling. Locked rows (never yet chosen by any
category) show "???", matching the tease used everywhere else undiscovered.

**The upgrade shop gets cheaper.** `Kind::Cost` used to be computed
(`effects.cost_reduction`) but never actually consumed anywhere — a real
bug, not a design choice, from an earlier pass. It now knocks flat DNA off
`Game::upgrade_level_cost` directly: every `Cost` kind anyone has, category
or species, discounts the price of buying *every* upgrade's next level, not
just its own.

**A default level on discovery.** `Kind::Vigor` doesn't touch anything
already found — it changes what `Game::unlock_taxon` hands a *newly*
discovered species: instead of the flat level 1 every species has always
opened at, it starts at `1 + effects.bonus_start_level` (rounded down), where
that total is whatever every `Vigor` upgrade anyone holds — category or
species — currently adds up to. Categories are unaffected either way; they
always open at level 1 regardless of who holds `Vigor`, since a category's
own progression is its upgrade level, not `Game::level`. Buying `Vigor`
further only ever changes the *next* discovery, never retroactively
re-levels what's already on the tree.

**Colour is the only thing that shows what a species inherits.**
`Game::nearest_category_kind` walks up from a species to the closest
`Backbone` ancestor with a resolved choice, and `render::accent_of` tints
*every* rectangle that represents it — the card, its border, its map node,
its spiral chip or bead — with that kind's colour (`kind_color`) instead of
the usual clade colour. No caption spells it out any more; the colour (and,
for the card, the actual number on its meta row — see "A species gives its
category's bonus" above) is the indicator. Every discovered species
resolves to *some* kind this way, even one with nothing but the root above
it — the root always has a fixed kind of its own (see above), so the walk
never comes up empty. A category keeps its clade colour; only a species'
colour moves.

## While you wait

The whole point of the logarithmic refill is that the wait shrinks fast — but
it never hits zero, so there had to be something to do besides watch a bar.
Deliberately **not** a tap-for-DNA mechanic: that would let tapping shorten
the wait, quietly undoing the pacing fix the curve exists for. Instead,
`src/facts.rs` holds ~40 short, well-established facts about animal evolution
and the phylogeny this game is built from, shown one at a time low on screen,
right above the bottom bar, on **both the spiral and the map** — not the
upgrades tab, which already has its own list to read. It only shows up
**while the pool is charging** (`render::should_show_fact`) — once Evolve is
ready there's something better to look at — and a tap **dismisses** it rather
than cycling to another; the next Evolve starting a fresh charge cycle is
what brings it back, with a new fact picked at random.

## DNA cap overflow

`Kind::Storage` is the one thing that lets `dna` climb past `Game::pool_cap`
at all. `Game::tick` clamps to `Game::overflow_cap` (`pool_cap() +
effects.overflow_bonus` — flat, like every other kind) instead of the plain
cap, so with no `Storage` upgrade the two are identical and the pool still
just clamps at the cap as ever. With one — typically accruing while the app
is backgrounded and
nobody's there to tap Evolve — DNA keeps climbing past the normal cap
instead of idling there. `Game::pool_full` (the single readiness check
`can_evolve`, the bottom bar's `EVOLVE` button, and autoplay all use) is
still measured against the *normal* cap, so Evolve is ready the moment it's
reached, same as ever — overflow only changes what happens if you let it
keep charging instead of spending right away. That extra banked amount is
not wasted: `Game::spend_ratio` measures "how much you spent" against
`dna.max(pool_cap())` rather than a bare `pool_cap()`, so a fuller-than-full
pool is a real, visible improvement to the next Evolve's odds of reaching
further, not just a bigger number sitting unused. No separate badge for it
either — the top HUD's `140 / 200 DNA` line simply shows the first number
outgrowing the second.

## Saving

`src/save.rs` + `Game::save`/`Game::load` (`src/game.rs`). Every durable
field (`unlocked`, `level`, `kind_level`, `upgrade_kind`, `pressure`,
`tap_progress`, `dna`, `taps`, `pending_choice`)
round-trips through
`serde_json` into one JSON blob under the key `"ascendio_save_v1"`, read and
written through [`quad-storage`](https://docs.rs/quad-storage) -- browser
`localStorage` on wasm, a local `local.data` file natively, one API either
way, so `save.rs` has no platform branching of its own. Cached/derived
fields (`rate`, `effects`) and purely visual ones (`pulse`) are rebuilt or
reset on load instead, not round-tripped.

Autosaves every 3 seconds (`main.rs`'s `SAVE_INTERVAL`), and immediately on
`R` (reset, so a reset is durable even if the app is closed right after) and
on quitting via `Esc`. `Game::load` stamps a `saved_at` wall-clock time into
the save and replays it through the normal `tick` catch-up math on the next
launch, exactly like a long-backgrounded session would -- offline progress
was already accounted for by the accrual code; saving is what lets it
survive a restart to be caught up *to*.

Dev tooling never touches the real save: `ASCENDIO_AUTOPLAY=1` runs against
a scratch game (so demo/screenshot runs stay repeatable and never leak into
a real save), and `ASCENDIO_FRESH=1` wipes any existing save before deciding
whether to load one, for testing what a new install sees without hunting
down and deleting `local.data`/clearing `localStorage` by hand.

## Not done yet

- No prestige / extinction-event reset loop. Deliberately deferred until the
  three-minute session proves fun; the upgrades tab's choice-of-3 mechanic is
  the first half of what was originally scoped as "adaptations."
- Pool overflow is simply clamped, unless someone has picked up `Kind::Storage`
  along the way, in which case it keeps accruing past the cap instead — see
  "DNA cap overflow" above.
- No audio, no haptics.
- No feedback for tapping `EVOLVE` while charging, `Leaderboard`, or a
  locked/unaffordable `BUY` row — all just inert. A shake or a muted click
  would make "not yet" read as clearly as "yes" already does.
- `Leaderboard` is a bottom-bar placeholder only — no screen, no data, no
  `Mode` behind it yet.
- The primate sprites (Lemur/Ape/Chimpanzee/Human) read as similar standing
  figures at 12x12 — the hardest body plan for this resolution and the
  weakest part of the current art pass.
- Only the focused species' branches are labelled; the rest are dots.
- The map at full-tree zoom is small on a 480px-wide screen; you have to pinch
  in to read labels.
- Android build unverified (see above).
- `Elephant`/`Whale`/`Wolf` are clade nodes wearing animal names — see
  `docs/PHYLOGENY.md`.

## Data

Topology and dates come from the phylogenomic literature (Laumer et al. 2019,
Irisarri et al. 2017, Misof et al. 2014, Upham et al. 2019, Prum et al. 2015,
TimeTree 5). Full citations and the list of deliberate simplifications are in
[`docs/PHYLOGENY.md`](docs/PHYLOGENY.md).
