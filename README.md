# Ascendio

An incremental mobile game about the animal tree of life. You start as the
**Urmetazoan** — the last common ancestor of every animal — and every Evolve
is a roll for a descendant lineage to split off. Unlock all 68 taxa and the
whole phylogeny is yours to explore.

**Status: playable prototype.** Every taxon has hand-authored or fallback
pixel art with per-clade idle animation now; there is still no audio and no
save file. What works is the full loop — DNA accrues into a capped pool,
Evolve spends it once full, and every roll pays out — plus all three screens,
plus the art.

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

Only species produce DNA. A category node — Mollusca, Arthropoda, Chordata,
and the rest of the tree's internal "common ancestor" taxa (`Group::Backbone`
in `tree.rs`) — is not an animal, so it earns nothing per second. Discovering
one instead grants its **upgrade** immediately, at level 1, free — named
after a real trait that clade is known for (Arthropoda → Exoskeleton,
Chordate → Notochord, Mammalia → Endothermy, ...). Buying it *further* is
what costs DNA, with no ceiling on how many times. See "The upgrades tab"
below.

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

**Evolve can land on any eligible lineage** — any unlocked taxon with
something left to give, species or category alike — picked at random when you
tap it, not necessarily whatever is on screen. It plays out as an overlay on
top of whichever screen you tapped it from — Spiral, Map, or Upgrades — so
Evolve never changes what you were looking at. The focus card still shows a
lineage's odds (`"71% odds if picked"`) purely as information now; tapping
the card itself does nothing.

**The reveal is a two-part animation** (`render::draw_evolve_fx`), not just a
toast. First, DNA particles stream from the top bar down into the Evolve
button — the spend, made visible, not just a number changing. Then a panel
appears with the result: the sprite snaps in from a dark, undersized
silhouette to full size and colour with a small overshoot-bounce, a fading
glow ring behind it, and its name fading in last. A miss plays the same
sequence but never reaches full colour — it settles into a muted amber
"no reaction" instead, so a failed roll reads differently at a glance from a
find. Once fully revealed, **the card just holds there** — no timer clears
it — with a fading "tap to continue" hint; it takes a tap (anywhere, on any
screen) to dismiss, and that tap does nothing else, so it can never
accidentally also trigger something underneath.

**The whole pool is the bet, every time.** Evolve always commits everything
you have banked — that is what "how much you spent" means here, since you can
only ever act right when the pool is full. Success chance is the usual
depth/pressure/upgrade formula *plus* a bonus for how many baseline rolls your
full pool represents (`ln(pool ÷ one roll's baseline cost)`), clamped at the
same 90% ceiling as everything else. A richer economy lands its Evolves more
reliably — that is the payoff for growing the tree, not just a bigger number
on screen.

**Every Evolve pays.** There are no dud clicks:

| Outcome | What you get |
|---|---|
| New species | The lineage splits. Output and cap both go up. |
| Branch already complete | **+1 level** to a descendant — evolving Mollusca again is what levels up Octopus, and the same rule is how a purchased category upgrade keeps getting stronger for free |
| Failed roll | **+1 mutation pressure**, raising the odds until it breaks — and the whole pool you just spent starts refilling from zero |

Pressure is the bar across the bottom of the card. It makes "which lineage do
I spend the next full pool on?" a real decision: cash in a lineage that has
been missing (pressure's already built up its odds), or bank on a fresh one.

Every DNA amount shown — the pool, its cap, an upgrade's price — is a whole
number; only the DNA/s rate keeps decimals, since early on that is
legitimately below 1.

**Levels compound down the lineage.** A species' output is multiplied by
`1 + 0.02 × (sum of every ancestor's level)`, so investing in the trunk keeps
paying long after you have moved past it, and early species never become dead
weight.

## The two views

**The spiral** is the game. One species fills a card at the top of the screen;
below it a logarithmic coil carries that species' **lineage** — ancestors
winding tightly into the centre, descendants opening outward. Dragging scrolls
along it a generation at a time. If you happen to be here when Evolve fires,
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
cladogram, the way phylogenies are normally drawn. Pinch closed from the spiral
to open it, tap any species to route the lineage there.

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
| Tap the card | Nothing — informational only | — | — |
| Tap a branch spur | Reroute the lineage through it | — | — |
| Tap an ancestor on the coil | Scroll back to it | — | — |
| Tap a species | — | Route the lineage there | — |
| Tap a `BUY` row | — | — | Purchase that upgrade |
| Swipe | Scroll the lineage; flings with momentum | Pan | Scroll the list |
| Scroll wheel | Step one generation | Zoom | — |
| Pinch closed | Open the map | Zoom out | — |
| `MAP` / `TREE` / `BACK` button, or `M` | Switch view | Switch view | Back |
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
source so it can never drift from the version the wasm was built against. It is
the only JavaScript involved: macroquad 0.4.16 sets `default = []`, so the audio
feature is off and none of the extra `mq_js_bundle.js` parts apply. The wasm's
104 JS imports all resolve against that one file.

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
├── upgrades.rs one upgrade per Backbone taxon, and their combined effect
├── facts.rs    "did you know" trivia shown while the pool charges
└── render.rs   all drawing, for all three screens
web/
├── index.html  canvas + touch-action, for the phone dev loop
└── gl.js       miniquad's loader, vendored from the crate
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
gets the same silhouette treatment next to its "???" label — and distant
beads on the coil get one too, though at that scale it mostly reads as a
coloured dot.

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
| `UPGRADE_LEVEL_BASE` / `_PER_DEPTH` | DNA for a depth-0 upgrade's first purchasable level (its second overall), and the per-generation multiplier (50, 1.5) |
| `UPGRADE_LEVEL_PER_LEVEL` | Dearer again for every level already bought (1.6) |

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
all 24 of them — has exactly one entry, checked by a test
(`every_backbone_taxon_has_exactly_one_upgrade`), so no category discovery is
ever a dead end even though it produces no DNA:

| Kind | Effect | Per level | Example |
|---|---|---|---|
| Rate | Multiplies total DNA/s | ×1.08 | Urmetazoan → Multicellularity |
| Cost | Multiplies the Evolve baseline (cheaper spend bonus math) | ×0.94 | Protostome → Protostomy |
| Chance | Adds flat percentage points to every Evolve's odds | +4pp | Cnidarian → Nerve Net |
| Cap | Multiplies the DNA pool's cap | ×1.10 | Mollusc → Shell |
| Storage | Adds a banked Evolve slot | +1 slot | Amniote → Amniotic Egg |

**Discovering a category grants its upgrade immediately, at level 1, free.**
There is no activation purchase. What costs DNA is buying it *further* — each
row shows its current effect and level on the left, and a `BUY` button for
the next level on the right (`Game::buy_upgrade_level`), at a price that
climbs both with the taxon's depth and with every level already bought
(`UPGRADE_LEVEL_PER_LEVEL`). There is no ceiling. This level is its own
counter (`upgrade_level`), entirely independent of `level` (which the taxon
still has, for `lineage_bonus`) — buying is the only thing that ever moves
it. A level-`n` upgrade compounds `n` times (`(1 + per_level)^n` for the
multiplicative kinds). Locked rows show the real taxon's sprite as a dark
silhouette and "???", matching the tease used everywhere else undiscovered.

`Storage` is the exception -- it doesn't fit that percentage shape, so it
adds whole slots instead, one per level, and `Game::tick`/`Game::evolve`
handle it separately (see "Banked Evolves" below).

## While you wait

The whole point of the logarithmic refill is that the wait shrinks fast — but
it never hits zero, so there had to be something to do besides watch a bar.
Deliberately **not** a tap-for-DNA mechanic: that would let tapping shorten
the wait, quietly undoing the pacing fix the curve exists for. Instead,
`src/facts.rs` holds ~40 short, well-established facts about animal evolution
and the phylogeny this game is built from, shown one at a time in a bigger
panel on **the map view only** — not the spiral or the upgrades tab, since
those already have their own things on screen to look at. It never changes on
a timer: it only advances when you tap the panel, so nothing you're
mid-reading gets swapped out from under you.

## Banked Evolves

The Amniote upgrade (`Kind::Storage`) is the one exception to "Evolve only
fires on a full pool." Each level adds one banked slot. If the pool tops out
— typically while the app is backgrounded and nobody's there to tap Evolve —
instead of just idling at the cap, `Game::tick` **banks a charge and restarts
the pool from zero**, up to the slot count; further overflow past that just
waits at the cap as it always did. The bottom bar's `EVOLVE` button lights up
on a banked charge exactly as it does on a full pool, its label switching to
`EVOLVE x2` (etc.) when more than one Evolve is available at once, and
spending a banked charge leaves the live pool untouched. `Game::can_evolve()`
is the single readiness check both the button and autoplay use.

## Not done yet

- **No persistence — progress is lost on quit.** This is the next thing to
  build, and it is what "offline catch-up" is waiting on. DNA accrues against
  the wall clock (`miniquad::date::now`), so time with the app *backgrounded*
  already counts; but with nothing saved, there is no across-restart offline
  to catch up to. The accrual code is save-ready — it needs a save layer, not
  a rewrite.
- No prestige / extinction-event reset loop. Deliberately deferred until the
  three-minute session proves fun; the upgrades tab is the first half of what
  was originally scoped as "adaptations."
- Pool overflow is simply clamped, unless the Amniote upgrade is bought, in
  which case it banks up to that many Evolve charges instead — see "Banked
  Evolves" above.
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
