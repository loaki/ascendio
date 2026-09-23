# Ascendio: game design

> Status: design for the rebuild. It replaces the DNA-pool / Evolve economy
> described in the README (`game.rs`, `upgrades.rs`). The spiral, the map,
> the sprites and the tree stay.

## The pitch

You don't play the animals. You play **the planet**.

You shape the world: raise land, grow forests, change the air and the
climate. Then you let time run. Hours of real time become millions of
years. When you come back, a **new genome** has evolved. You mutate it
until it detonates, to find out what evolved while you were away, and
sometimes it is something rare.

A session lasts about 2 minutes. There are at most 4 a day. It stops you
because the planet needs time, not because the game makes you wait.

## The core loop

```
 ┌──────────────┐    ┌─────────────────┐    ┌───────────────────┐    ┌──────────────┐
 │ SHAPE        │ →  │ ACCELERATE TIME │ →  │ EXPRESS GENOME    │ →  │ STEER        │
 │ spend 3 pts  │    │ 6 h real        │    │ 3 cards, rarity,  │    │ pick 1 boon, │
 │ on 5 levers  │    │ = 20 Ma         │    │ staged reveal     │    │ set keystones│
 └──────────────┘    └─────────────────┘    └───────────────────┘    └──────┬───────┘
        ↑                                                                   │
        └───────────────────────────────────────────────────────────────────┘
```

1. **Shape.** Spend **adjustment points** (3 per cycle to start) to move the
   planet's levers one step each. The background changes as you go.
2. **Accelerate time.** One button. A real-time countdown starts, and the
   spiral and background animate the ages passing. You can close the app.
3. **Express the genome.** When the timer ends, a new genome waits. Tap
   it three times to mutate it until it detonates into 3 cards, revealed
   one at a time, best card last.
4. **Steer.** Pick 1 of 3 **boons** for the next cycle. Optionally re-equip
   your **keystones**, the discovered animals whose bonuses are active.
5. Back to 1. You can only start a new cycle once the genome is expressed, so
   progress never stacks up while you're away.

### Pacing

| Cycle | Real wait | Why |
|---|---|---|
| 1st ever | 1 min | Teach the loop in the first session |
| 2nd | 20 min | A reason to return the same day |
| Every one after | **6 h** | Up to 4 check-ins a day: morning, lunch, evening, night |

Each cycle is **20 Ma** of in-game time. The HUD shows the total let run so
far (e.g. "120 Ma", counting up while a cycle runs) and the era, which is the
age of your most recent discovery.

## The screen

There is one main screen. The **spiral** sits over a **living sea
background**, and the background *is* the planet. There is no globe.

At the start the spiral has one node, the Urmetazoan. Every discovery
winds the coil further out.

| Lever | Levels | What the background shows |
|---|---|---|
| **Land** (sea level) | 0–5 | Ridges rise out of the water at the bottom. 0 is a water world, 5 a supercontinent |
| **Vegetation** | 0–5, capped at 2 × Land | Bare rock, then moss, fern, forest, jungle on the ridges. Kelp thickens in the water |
| **Oxygen** | 0–5 (5% → 35%) | Water gets clearer, light rays get stronger, particles glow more |
| **Temperature** | 0–5 (snowball → hothouse) | Ice creeps in from the top when cold. The palette turns warm and hazy when hot |
| **Volcanism** | 0–3 | Glowing vents, ash drift and red tint |

Level meanings:

- **Oxygen:** 0 = 5%, 1 = 10%, 2 = 15%, 3 = 21% (today), 4 = 28%,
  5 = 35% (Carboniferous peak).
- **Temperature:** 0 = snowball, 1 = cold, 2 = cool, 3 = temperate,
  4 = warm, 5 = hothouse.

### Linked levers (applied at the end of each cycle)

These stop "set everything to max" from working, and teach real Earth
history. Each shows as a small preview arrow on its lever before you launch.

1. **Forests breathe.** Vegetation ≥ 3 → Oxygen +1 (up to 5).
2. **Volcanic greenhouse.** Volcanism ≥ 2 → Temperature +1 (up to 5).
3. **Ice locks up the sea.** Temperature ≤ 1 → Land +1 (glaciers lower sea
   level).
4. **Great Dying** *(designed, disabled for now: the Volcanism lever stops
   at 2)*. Volcanism 3 at launch is a deliberate **mass
   extinction** (see Radiation below). Afterwards Oxygen −2 and
   Temperature +2, so you have to repair the planet.

Nothing you discovered is ever lost. An extinction reshapes the world,
never your collection.

### Habitats (derived from the levers)

| Habitat | Exists when |
|---|---|
| **Sea** | Land ≤ 4 |
| **Reef** | Sea and Temperature ≥ 3 |
| **Shore** | 1 ≤ Land ≤ 4 |
| **Fresh water** | Land ≥ 2, Temperature ≥ 1, Vegetation ≥ 1 |
| **Land** | Land ≥ 1 and Vegetation ≥ 1 |
| **Forest** | Vegetation ≥ 3 |

## The new genome

The animal isn't dug up: it **evolves**. Each cycle ends with a new genome,
a glowing 3D double helix, and expressing it is the "pack opening". (In the
code it is still called the nodule: `nodule.rs`, `Phase::Nodule`.)

### Presentation (the "pack" moment)

1. **No tell.** The helix turns in a neutral pale light, with loose bases
   spiralling in and a charge climbing its strands. Nothing hints at the
   rarity inside, so the suspense lasts until the blast.
2. **Tap three times to mutate it.** Each tap lights up a third of its base
   pairs, with "MUTATION!", a colour-split glitch, a shake and a faster
   spin.
3. **Supernova.** On the third tap the strands spiral into a white-hot
   core, it flickers for a heartbeat, then detonates: a freeze frame (longer
   for a Legendary, with a pillar of gold light), a white flash, three
   shockwaves, speed lines and a storm of base pairs, all in the **best card's
   colour**: the first time the rarity shows. The cards are
   thrown out of the blast.
4. **Cards face down.** Each glows its tier colour, and you tap them in any
   order. The best card is always forced to go last.
5. **Staged reveal for a new species.** Silhouette, then habitat, clade and
   origin typed out, then the animal bursts into colour.
6. **First-ever discovery** gets the full animation; a duplicate is a quick
   "+1 specimen".

### What can drop: two filters, then a roll

A taxon is **eligible** when:

- **The tree allows it.** Its parent is discovered and it isn't.
- **The planet allows it.** Its habitat exists and its needs are met
  (table below).

Each card then:

1. Rolls a **tier** from the weights below, plus luck bonuses, with pity
   applied.
2. Picks a random **eligible** taxon of that tier, weighted by affinity
   boons and keystones.
3. If none exist at that tier, tries the next tier down, and so on.
4. If nothing at all is eligible, it becomes a **specimen**: a duplicate of
   an animal you own that fits the current planet. The card says why, e.g.
   "Nothing new could live here: your world has no land."

| Tier | Base weight | Colour |
|---|---|---|
| Common | 60% | grey `#8a97a3` |
| Uncommon | 25% | green `#6fdc6a` |
| Rare | 11% | blue `#5aa8ff` |
| Epic | 3.5% | purple `#c07bff` |
| Legendary | 0.5% | gold `#ffc84a` |

**Pity**, counted in genomes and reset when that tier drops:

- Rare+ at least every **3**.
- Epic+ within **10**.
- Legendary within **40**.

A pity roll only fires when an eligible taxon of that tier exists. The
counter keeps waiting otherwise.

Cards per genome are **3** base, + keystone and boon bonuses, up to 6.

### Morphs: the second rarity axis

Any card can be a morph, rolled independently. Morphs only change looks
and bonus strength, never what the animal is.

| Morph | Chance | Look | Keystone bonus |
|---|---|---|---|
| Giant | 1/20 | Drawn one size up | ×1.5 |
| Albino | 1/64 | Pale palette, red eyes | ×1.5 |
| Melanistic | 1/64 | Near-black palette | ×1.5 |
| Amber | 1/512 | Gold-preserved, prismatic frame | ×2 |

At Oxygen 5, the Giant chance for arthropods is ×3 (Meganeura: 71 cm
wingspan at about 35% O₂). Your first morph is guaranteed within your
first 14 cycles.

### Specimens (duplicates)

A duplicate is never wasted. It levels that animal from **Lv 1 to Lv 5**
(1, 2, 4, 8 specimens per level). Its keystone bonus is
× (1 + 0.25 × (level − 1)).

### Radiation genome (the "god pack")

The cycle after a Great Dying, every card is **Rare or better**, drawn
from the survivors' children. It's the payoff for choosing to wreck your
own planet.

## Steering what you discover

### Keystones (long-term)

- You equip up to **3 discovered animals** as keystones. A 4th slot opens at
  20 discoveries, a 5th at 40.
- Only keystones give their bonus, so your collection becomes a loadout.
  Equip sea creatures to pull more sea creatures.
- **A keystone only works while the planet suits it.** If its habitat or
  needs aren't met (a frog on a dry world, a penguin in a hothouse), it goes
  **dormant** and gives nothing. While time runs, the planet as launched is
  the one that counts. Dormant keystones are greyed out with the reason, the
  Keystones tab gets a warning dot, and the lever panel says which one
  your last lever change put to sleep.
- You can swap them while shaping and while time runs (the genome is rolled
  from them when the cycle ends). They lock while a genome waits to be
  opened.

**Bonus kinds.** Magnitude scales by tier: Common 1, Uncommon 2, Rare 3,
Epic 4, Legendary 6 units.

| Kind | Per unit | Cap |
|---|---|---|
| **Affinity (habitat)** | +10% weight for taxa of that habitat | none |
| **Luck** | +1% to the rare-tier rolls | +15% |
| **Extra card** | +12% chance of +1 card | 3 extra cards |
| **Morph** | +8% morph chance | ×3 |
| **Quick** | −3% real wait | −30% |
| **Point** | +1 adjustment point (Epic+ only) | 6 points |
| **Planet** | A passive lever effect, e.g. worms +1 vegetation per cycle | – |

### Boons (short-term)

After each genome, you pick **1 of 3**, which applies to the next cycle
only:

- **Lure.** One clade (e.g. Arthropoda) gets ×3 weight.
- **Lens.** +1 card.
- **Catalyst.** Guaranteed Rare+ card.
- **Charm.** ×4 morph chance.
- **Tailwind.** −1 h wait this cycle.
- **Tectonics.** +2 adjustment points.

## The 97 taxa

Legend:

- **Needs** are extra conditions on top of the habitat. O = Oxygen level,
  T = Temperature level, V = Vegetation level.
- **Branch** marks a category (`Group::Backbone`). Discovering it opens the
  pool of its children. Its keystone bonus is the clade's signature.

| # | Taxon | Habitat | Needs | Tier | Keystone bonus |
|---|---|---|---|---|---|
| 0 | Urmetazoan | Sea | – | start | Luck (+1) |
| 1 | Sea Sponge | Sea | O≥1 | C | Luck |
| 2 | Comb Jelly | Sea | O≥1 | C | Affinity: Sea |
| 3 | Placozoan | Sea | O≥1 | U | Morph |
| 4 | Cnidarian | Sea | O≥1 | C | Branch · Affinity: Sea |
| 5 | Jellyfish | Sea | O≥1 | C | Morph |
| 6 | Coral | Reef | O≥2 | U | Affinity: Reef |
| 7 | Bilaterian | Sea | O≥2 | C | Branch · Extra card |
| 8 | Acoel Worm | Sea | O≥1 | C | Quick |
| 9 | Protostome | Sea | O≥2 | C | Branch · Luck |
| 10 | Deuterostome | Sea | O≥2 | U | Branch · Luck |
| 11 | Spiralian | Sea | O≥2 | C | Branch · Affinity: Shore |
| 12 | Ecdysozoan | Sea | O≥2 | C | Branch · Morph |
| 13 | Flatworm | Fresh | O≥2 | C | Extra card |
| 14 | Mollusc | Sea | O≥2 | U | Branch · Affinity: Reef |
| 15 | Snail | Shore | O≥2 | C | Quick |
| 16 | Octopus | Reef | O≥3 | E | Extra card |
| 17 | Segmented Worm | Land | O≥2 | U | Planet: +1 Vegetation/cycle (soil) |
| 18 | Nematode | Sea or Land | O≥1 | C | Quick |
| 19 | Arthropod | Sea | O≥2 | U | Branch · Morph |
| 20 | Chelicerate | Shore | O≥2 | U | Branch · Affinity: Land |
| 21 | Horseshoe Crab | Shore | O≥2 | R | Luck (living fossil) |
| 22 | Scorpion | Land | O≥3, T≥3 | R | Affinity: Land |
| 23 | Spider | Forest | O≥3 | R | Extra card |
| 24 | Pancrustacean | Sea | O≥2 | C | Branch · Affinity: Shore |
| 25 | Crab | Shore | O≥2 | U | Affinity: Shore |
| 26 | Insect | Land | O≥3, V≥2 | U | Branch · Affinity: Forest |
| 27 | Dragonfly | Fresh | O≥4 | R | Morph (Giant ×3 at O5) |
| 28 | Beetle | Forest | O≥3 | U | Extra card |
| 29 | Butterfly | Forest | O≥3, T≥3 | R | Planet: +1 Vegetation/cycle (pollination) |
| 30 | Ant | Forest | T≥3 | E | Point |
| 31 | Echinoderm | Sea | O≥2 | C | Branch · Affinity: Reef |
| 32 | Starfish | Shore | O≥2 | C | Specimens ×2 (regeneration) |
| 33 | Sea Urchin | Reef | O≥2 | U | Affinity: Reef |
| 34 | Chordate | Sea | O≥2 | U | Branch · Luck |
| 35 | Sea Squirt | Sea | O≥2 | C | Quick |
| 36 | Vertebrate | Sea | O≥3 | R | Branch · Luck |
| 37 | Lamprey | Sea or Fresh | O≥2 | U | Affinity: Fresh |
| 38 | Jawed Fish | Sea | O≥3 | U | Branch · Extra card |
| 39 | Shark | Sea | O≥3 | R | Luck |
| 40 | Ray-finned Fish | Sea or Fresh | O≥2 | C | Extra card |
| 41 | Lobe-finned Fish | Fresh | O≥2 | R | Branch · Affinity: Fresh |
| 42 | Coelacanth | Sea | O≥2, T≤3 | L | Luck + Legendary pity 40 → 30 |
| 43 | Tetrapod | Shore + Fresh | O≥2, V≥2 | E | Branch · Affinity: Land |
| 44 | Amphibian | Fresh | T≥2, V≥2 | U | Branch · Affinity: Fresh |
| 45 | Frog | Fresh | T≥3 | C | Morph |
| 46 | Amniote | Land | V≥2 | R | Branch · Affinity: Land |
| 47 | Mammal | Land | T≥1, V≥2 | R | Branch · Luck |
| 48 | Platypus | Fresh | T≤3 | L | Morph + first morph guaranteed each week |
| 49 | Kangaroo | Land | T≥3, V≤3 | R | Affinity: Land |
| 50 | Placental | Land | V≥2 | E | Branch · Extra card |
| 51 | Elephant | Land | T≥3, V≥3 | E | Point |
| 52 | Mouse | Land | V≥1 | C | Quick |
| 53 | Bat | Forest | O≥3 | R | Extra card |
| 54 | Wolf | Land | T≤3 | R | Luck |
| 55 | Whale | Sea | – | E | Affinity: Sea |
| 56 | Primate | Forest | T≥3 | R | Branch · Affinity: Forest |
| 57 | Lemur | Forest | T≥3 | U | Morph |
| 58 | Monkey | Forest | T≥3 | R | Branch · Extra card |
| 59 | Ape | Forest | T≥3 | E | Branch · Luck |
| 60 | Chimpanzee | Forest | T≥3 | E | Extra card |
| 61 | Human | Land | O≥3, 2≤T≤4, V≥2 | L | Point + Luck |
| 62 | Reptile | Land | T≥2 | U | Branch · Affinity: Land |
| 63 | Turtle | Shore | T≥2 | U | Specimens ×2 |
| 64 | Lizard | Land | T≥3, V≤3 | C | Quick |
| 65 | Crocodile | Fresh | T≥4 | R | Affinity: Fresh |
| 66 | Dinosaur | Land | T≥3, V≥3 | E | Extra card |
| 67 | Bird | Forest | O≥3 | E | Luck |
| 68 | Clam | Reef | O≥2 | C | Specimens ×2 |
| 69 | Squid | Sea | O≥3 | U | Extra card |
| 70 | Ammonite | Sea | O≥2, T≥2 | R | Luck |
| 71 | Earthworm | Land | O≥2, V≥2 | C | Soil |
| 72 | Tardigrade | Sea or Land | – | E | Oddity |
| 73 | Trilobite | Sea | O≥2 | U | Affinity: Sea |
| 74 | Anomalocaris | Sea | O≥2 | E | Luck |
| 75 | Centipede | Forest | O≥3 | C | Quick |
| 76 | Shrimp | Sea | O≥2 | C | Extra card |
| 77 | Bee | Forest | O≥3, T≥2, V≥3 | U | Soil |
| 78 | Meganeura | Fresh | O≥5 | E | Morph |
| 79 | Dunkleosteus | Sea | O≥3 | E | Luck |
| 80 | Stingray | Sea | O≥3 | U | Affinity: Sea |
| 81 | Megalodon | Sea | O≥3, T≥3 | L | Extra card |
| 82 | Seahorse | Reef | O≥3, T≥3 | R | Morph |
| 83 | Anglerfish | Sea | O≥2 | R | Luck |
| 84 | Tiktaalik | Shore or Fresh | O≥3 | R | Affinity: Shore |
| 85 | Salamander | Fresh | 1≤T≤4 | C | Quick |
| 86 | Snake | Land | T≥3 | U | Luck |
| 87 | Pterosaur | Shore | O≥3, T≥3 | E | Affinity: Shore |
| 88 | T. rex | Land | O≥3, T≥3, V≥2 | L | Luck |
| 89 | Sauropod | Forest | O≥3, T≥3, V≥3 | E | Specimens ×2 |
| 90 | Penguin | Shore | T≤2 | R | Affinity: Shore |
| 91 | Mammoth | Land | T≤1 | E | Living fossil |
| 92 | Rabbit | Land | 1≤V≤4 | C | Specimens ×2 |
| 93 | Horse | Land | 1≤T≤4, 1≤V≤3 | U | Quick |
| 94 | Dolphin | Sea | T≥2 | R | Extra card |
| 95 | Lion | Land | T≥3, 1≤V≤3 | R | Luck |
| 96 | Gorilla | Forest | T≥3 | R | Point |

### Some intended journeys

- **The first days** stay in the sea. Sponges, jellies and worms drop
  almost every cycle, which teaches the reveal.
- **The first real decision** comes with Coral and Octopus. They need a
  reef, so you must warm the planet to T≥3.
- **Getting out of the water:** Lobe-finned Fish needs fresh water, which
  needs land and vegetation. Then Tetrapod needs shore, fresh water and
  vegetation together. It's the first multi-lever puzzle.
- **The Carboniferous gambit:** push Oxygen to 5 through forests. Giant
  dragonflies and spiders appear, and Giant morphs of arthropods triple.
- **Opposite temperatures:** Wolf and Coelacanth want T≤3, while
  Crocodile wants T≥4. You can't have both at once, so every planet is a
  choice of who shows up.
- **The long road to Human** is 13 branch discoveries. It needs forests,
  a warm but not hot climate, and at least Oxygen 3. Humans are Legendary.

## Leaderboard

Players are ranked by **most advanced animal**: the depth of the deepest
discovered taxon. Human is step 14. Ties break by collection size, then by
who got there first. A secondary board ranks by morphs owned.

## Fairness

- No real-money purchases of cycles, genomes or odds, ever.
- Pity timers and specimen levels mean every genome moves you forward.
- The session cap is the cycle timer. Quick bonuses are capped at −30%.

## Rebuild plan (code)

| Keep | Change | Replace |
|---|---|---|
| `tree.rs`, `sprites.rs`, `spiral.rs`, `layout.rs`, `view.rs`, `save.rs`, `facts.rs` | `render.rs`: new sea-planet background and new overlays. `main.rs`: new modes | `game.rs` becomes `planet.rs` (levers, habitats, cycle timer), `nodule.rs` (roll, pity, morphs, specimens) and `collection.rs` (discovered, levels, keystones, boons). `upgrades.rs` is removed |

The new `Game` keeps the fields the views already read (`phy`, `unlocked`,
`taxon`), so the spiral and map work unchanged from the first commit.

Suggested order:

1. `planet.rs`: the levers, the linked levers and habitats, the cycle timer
   with wall-clock catch-up, and its tests.
2. `nodule.rs`: eligibility, tier roll, pity, morphs and specimens. This is
   pure logic, heavily tested with seeded RNG.
3. A new `Game` wiring it together, plus the save format (v2, which
   discards old saves).
4. Render: the planet background driven by the levers (dithered sea,
   ridges, vegetation, ice, vents).
5. Screens: the lever drawer, the Accelerate button and countdown, the
   genome opening, the boon pick, and the keystones screen.
6. Juice: shake and hit-stop by tier, glow colours, sound.
