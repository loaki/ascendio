# Ascendio: game design

> Status: implemented, except the mass extinction and the leaderboard (see
> "Not done yet" in the README). Every number here is a first guess that
> still needs playtesting.

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
 │ spend 1-3 pts│    │ 2 h to 6 h real │    │ 1 to 6 cards,     │    │ pick 1 boon, │
 │ on 5 levers  │    │ = 20 to 60 Ma   │    │ staged reveal     │    │ set keystones│
 └──────────────┘    └─────────────────┘    └───────────────────┘    └──────┬───────┘
        ↑                                                                   │
        └───────────────────────────────────────────────────────────────────┘
```

1. **Shape.** Spend **adjustment points** (1 to 3, from the length of the
   last wait) to move the planet's levers one step each. The background changes as you go.
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
| Every one after | **2 h to 6 h**, the player's choice | Check in as often as you like, but waiting longer pays |

After the tutorial, tapping Let time run raises a **dial** all the way
round the spiral: a dithered halo with a sun on it, 2h at the top and one
step per half hour clockwise. The lever panel gives way to what the wait
will bring: the rarity odds, the cards, the morph chance and each
keystone's share. Longer waits pay more per hour, so a 6h wait always beats
two 3h ones:

| Wait | Cards | Points next | Luck | Morphs | New species | Extra |
|---|---|---|---|---|---|---|
| 2 h | 1 | 1 | +0 | x1 | 15% | |
| 3 h | 1.5 | 1 | +0.75 | x1 | 22.5% | |
| 4 h | 2 | 2 | +3 | x1.25 | 30% | |
| 5 h | 3.5 | 2 | +6.75 | x1.25 | 37.5% | |
| 6 h | 5 | 3 | +12 | x1.5 | 45% | one Rare or better guaranteed |

Half hours in between are interpolated; a fraction is the chance of one
more card. **Points next** are the adjustment points the next shaping gets
(before keystones and boons): a long wait buys a big reshape, a short one a
small tweak. The first shaping and the ones after the tutorial cycles get 3. Keystone bonuses stack on top, with no cap on cards. The tutorial
cycles pay like a 3h wait, but with 3 cards so the first genomes fill
the tree.

Each hour is **10 Ma** of in-game time. The HUD shows the total let run so
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

**Habitat share.** Each habitat covers a share of the planet: the sea
shrinks as land rises, a reef takes half of a warm shallow sea, forest
takes the vegetated part of the land. Among the animals that can drop, the
odds follow their habitat's share, so a planet that is mostly forest
mostly turns up forest animals.

### Biomes

Most rare animals belong to a **biome**, a named set of lever ranges. The
biomes rule each other out, so no planet holds them all: the best single
planet suits about half of the tree (81 of 151), against 146 before
biomes. The broad animals left (the trunk of the tree and the commons)
still split by temperature and oxygen: cold-water against warm-water,
mammals out of a hothouse or a 35% oxygen world, open-ocean swimmers off a
continent.

| Biome | Levers | Animals |
|---|---|---|
| **Primordial sea** | O ≤ 2, Land ≤ 2 | Sponge, Comb Jelly, Trilobite, Anomalocaris, Dickinsonia... |
| **Reef sea** | Land ≤ 1, T 3–4, O ≥ 3 | Coral, Octopus, Clownfish, Seahorse, Nautilus... |
| **Ice age** | T ≤ 1 | Mammoth, Sabre-tooth, Seal, Neanderthal |
| **Coal swamp** | O 5, V ≥ 4, T 3–4 | Meganeura, Dragonfly, Spider, Scorpion, Acanthostega... |
| **Jungle** | V 5, T 4, O ≤ 4 | Lemur, Gorilla, Chimpanzee, Parrot, Dodo, Frog... |
| **Savanna** | Land ≥ 4, V 1–2, T ≥ 3 | Lion, Giraffe, Elephant, Rhino, Lucy, Human... |
| **Hothouse** | T 5, Volcanism ≥ 1 | T. rex, Triceratops, Sauropod, Pterosaur, Plesiosaur... |

**Biome bonus.** Three awake keystones of the same biome give that biome's
bonus (the Platypus gives them all):

| Biome | Bonus |
|---|---|
| Primordial sea | +15% discovery |
| Reef sea | +50% morphs |
| Ice age | −15% wait |
| Coal swamp | +1 card |
| Jungle | +5 luck |
| Savanna | +1 point per 4h waited |
| Hothouse | +1 card |

The linked levers push between them: a jungle's forests raise oxygen
towards a coal swamp, volcanoes warm a planet towards a hothouse.

## The new genome

The animal isn't dug up: it **evolves**. Each cycle ends with a new genome,
a glowing 3D double helix, and expressing it is the "pack opening"
(`genome.rs` rolls it, `opening.rs` presents it).

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
4. **A row of slots.** The cards fly out of the blast into one empty slot
   each, in a row under the reveal (smaller as cards grow, two rows past
   8). The first reveal starts at once; each tap passes to the next card,
   whose slot lights up, then fills with its rarity colour and animal once
   revealed. The cards still come weakest first. After the last one, a tap
   goes straight on: the filled row was the summary.
5. **Silhouette roulette for every card.** Silhouettes of other
   animals flicker in the card's place, slowing down like a slot reel
   (gaps from 0.05 s, +22% a tick), until the real one locks ("LOCKED")
   at 1.4 s. Only then do its habitat, clade and origin type out, so they
   don't give it away, and at 1.8 s it bursts into colour.
6. **First-ever discovery** gets the clues and the full burst; a duplicate
   spins too, then shows "+1 specimen".

### What can drop: new species and owned ones

Each card:

1. Rolls a **tier** from the weights below, plus luck, with pity applied.
2. Rolls **discovery**: the chance it's a species you've never found. The
   wait sets it (**2h 15% · 4h 30% · 6h 45%**; the tutorial's genomes are
   all new), keystones, the Primordial sea and the Discovery boon add to it,
   up to 90%: some cards are always animals you own.
3. **New:** picks an **eligible** taxon of that tier, weighted by its
   habitat's share of the planet. A taxon is eligible when its parent is
   discovered and it isn't, and the planet meets its needs.
   **Owned:** picks an animal you already have that lives on the planet
   (any you own, if none does). It levels up.
4. If none exist at that tier, it tries the next tier down, then up.
5. A card due to be new when nothing new can evolve becomes an owned one,
   and says why, e.g. "Something is waiting to evolve: it needs more
   oxygen."

| Tier | Base weight | Colour |
|---|---|---|
| Common | 60% | grey `#8a97a3` |
| Uncommon | 25% | green `#6fdc6a` |
| Rare | 11% | blue `#5aa8ff` |
| Epic | 3.5% | purple `#c07bff` |
| Legendary | 0.5% | gold `#ffc84a` |

**Pity**, counted in millions of years waited (not genomes, so short waits
never reach it sooner than long ones) and reset when that tier drops:

- Rare+ at least every **120 Ma** (3 default 4h waits).
- Epic+ within **400 Ma** (10).
- Legendary within **1600 Ma** (40).

A pity roll only fires when an animal of that tier can drop, new or owned.
The counter keeps waiting otherwise.

Cards per genome come from the wait (see Pacing), + keystone and boon
bonuses, with no cap.

### Morphs: the second rarity axis

Any card can be a morph, rolled independently. Morphs never change what
the animal is; like card editions, they change what it does as a
keystone, each in its own way:

| Morph | Chance | Look | As a keystone |
|---|---|---|---|
| Giant | 1/40 | Drawn one size up | +1 adjustment point per 4h waited |
| Albino | 1/128 | Pale palette, red eyes | ×1.5, but asleep above Cool (sunburn) |
| Melanistic | 1/128 | Near-black palette | Ignores temperature: lives at any |
| Amber | 1/1024 | Gold-preserved, prismatic frame | ×2 its bonus |

By default a keystone wears its BEST morph: of those it owns, the
strongest that keeps it awake on the planet it lives on (Amber, then an
Albino where it's Cool or colder, then Giant, then Melanistic, which is what
keeps an animal awake at a temperature it can't live at). The MORPH button on the Keystones screen
cycles BEST, NONE and each owned morph, while shaping, to pick one instead.
Morphs stay rare: however keystones, boons, the wait and radiation stack,
a card is a morph at most 15% of the time.

At Oxygen 5, the Giant chance for arthropods is ×3 (Meganeura: 71 cm
wingspan at about 35% O₂). Your first morph is guaranteed within your
first 560 Ma (14 default waits).

### Specimens (duplicates)

A duplicate is never wasted. It levels that animal from **Lv 1 to Lv 5**
(1, 2, 4, 8 specimens per level). Its keystone bonus is
× (1 + 0.25 × (level − 1)).

### Radiation genome (the "god pack")

The cycle after a Great Dying, every card is **Rare or better**, drawn
from the survivors' children. It's the payoff for choosing to wreck your
own planet.

### The end of the Earth (Human)

Human is the gamble: x2 Luck and +2 cards, but when a genome rolled with
Human opens, the Earth may end instead: **1 time in 5 for a 4h wait**. The
chance compounds with the wait (about 11% for 2h, 28% for 6h), so splitting
a wait into short ones never risks the Earth more often. The roll is made with
the genome, when the wait ends, and saved with it, so reopening the app
can't change it. While it waits, the genome panel and the EVOLVE IT button
show the risk ("20% the Earth ends").

**Nothing pays more for being split into short waits.** Pity, keystone
charges, keystone points and Human's risk all count the time waited, not
the number of genomes; the boons scale with the wait.

Tapping EVOLVE IT then plays the end of the Earth (`src/collapse.rs`,
about 9 s, it can't be skipped) instead of the cards, and the genome is
lost:

1. **The roll.** Red flashes, "1 IN 5".
2. **The flash.** The screen goes solid white, then the white dithers
   away to reveal the Earth frozen into a snowball.
3. **Snow and ash.** Snow and ash fall on the frozen world as it dims:
   "everything found turns to stone".
4. **The spiral unwinds.** The screen goes black and the coil spins down
   to its one node.
5. **New Earth.** The game restarts from the Urmetazoan with **+1 RAD**.
   There is no choice to make: the radiation is the reward.

**What resets:** the spiral (back to step 0), Ma and era, the planet,
keystones and their charges, pity, the waiting genome, the points budget.

**What stays:** every animal ever found, as a **fossil** with its
specimens (so its level) and morphs. Fossils show as grey stone with their
name on the spiral, the map and the Keystones grid, so you know what you
are hunting. A fossil must be found again before it can be a keystone;
then it comes back at its old level.

**RAD** shows as a badge in the top bar (tap it for the details) and
stacks each time Human ends an Earth. Each RAD gives **+5 Luck past the
keystones' cap of 20** and **+25% morph chance**, still within the 15%-per-card cap:

| Per card, no keystones | Earth 1 | 1 RAD | 2 RAD | 3 RAD |
|---|---|---|---|---|
| Rare or better | 15% | 20% | 25% | 30% |
| Legendary | 0.5% | 0.67% | 0.83% | 1% |
| Morph | 4.2% | 5.2% | 6.2% | 7.3% |

## Steering what you discover

### Keystones (long-term)

- You equip up to **3 discovered animals** as keystones. A 4th slot opens at
  20 discoveries, a 5th at 40.
- Only keystones give their bonus, so your collection becomes a loadout.
  Equip sea creatures to pull more sea creatures.
- **A keystone only works while the planet suits it.** If its habitat,
  biome or needs aren't met (a frog on a dry world, a penguin in a
  hothouse), it falls **asleep** and gives nothing. While time runs, the
  planet as launched is the one that counts. Asleep keystones are greyed
  out with the reason, and the Keystones tab gets a warning dot.
- You can swap them while shaping and while time runs (the genome is rolled
  from them when the cycle ends). They lock while a genome waits to be
  opened.

**A bonus and a rule.** Each keystone has a **bonus**, what it adds, and a
**rule**: almost all are flat, a few are special (`src/ecology.rs`, the
taxa table below).

The bonus's strength is tier units (Common 1, Uncommon 2, Rare 3, Epic 4,
Legendary 6) × level × morph:

| Bonus | Per unit | Cap |
|---|---|---|
| **Luck** | +1 luck (rarer tiers) | 20 from keystones and biomes (after any ×2); the wait's luck comes on top |
| **Cards** | +12% chance of +1 card | – |
| **Discovery** | +3% chance a card is a new species | 90% with the wait |
| **Morphs** | +8% morph chance | ×3 |
| **Quick** | −3% real wait | −30% |
| **Point** | +1 adjustment point per 4h waited (earned during the wait, fractions carried over) | 8 points |

| Rule | What it does | Who |
|---|---|---|
| **Flat** | Always on | everyone else |
| **Copies** | Copies the keystone in the slot above, if of its kind and not a Legendary; its slot says what it copied | Octopus (a fish), Chimpanzee (a mammal), Parrot (a bird) |
| **Extremes** | Never asleep; +2 luck per lever at its min or max | Tardigrade |
| **Fragile** | ×3 luck; but falls asleep after one cycle (a duplicate of it wakes it) | Dodo |
| **Changing** | Its discovery from the start, +20% per charge (up to 5, so x2): +1 charge per 40 Ma waited on a planet changed since the wait before; a wait on the same planet empties them; but morphs are halved | Tuatara |
| **Stasis** | Its luck from the start, +20% per charge (up to 5, so x2): +1 charge per 40 Ma waited on exactly the planet of the wait before; a changed planet empties them; but 15% fewer new species | Coelacanth |

**Legendary catches:**

| Legendary | Perk | Catch |
|---|---|---|
| T. rex | +2 cards | The keystone in the last slot falls asleep (the one before, if the T. rex is last) |
| Human | x2 Luck and +2 cards | A 4h wait has 1 in 5 to end the Earth (see below) |
| Platypus | Every biome's bonus | The wait is 50% longer |
| Megalodon | Doubles all Luck | Temperature can't go below Temperate (its button shows red) |
| Archaeopteryx | Morphs ×3 | One fewer boon to choose from |
| Tuatara | Discovery growing while the planet changes (above) | Morphs are halved |
| Coelacanth | Luck growing while the planet stays the same (above) | 15% fewer new species |
| Dodo | Luck ×3 (above) | Falls asleep after one cycle |


### Boons (short-term)

After each genome, you pick **1 of 3**, which applies to the next cycle
only:

- **Discovery.** +25% chance each card is a new species (up to 90%).
- **Lens.** +1 card.
- **Catalyst.** A guaranteed Rare+ card, or Epic+ on a 6h wait (which
  already guarantees a Rare).
- **Charm.** ×4 morph chance.
- **Tailwind.** 25% less to wait this cycle.
- **Tectonics.** +2 adjustment points after a 4h wait, scaled with the wait
  just run (1 for 2h, 3 for 6h).

## The 151 taxa

Generated from the code (`cargo test print_doc_table -- --ignored
--nocapture`). **Needs** are a biome, or lever ranges on top of the
habitat: L = Land, V = Vegetation, O = Oxygen, T = Temperature.

| # | Taxon | Habitat | Needs | Tier | Keystone |
|---|---|---|---|---|---|
| 0 | Urmetazoan | Sea | – | C | Better rarity odds |
| 1 | Sea Sponge | Sea | Primordial sea | C | Better rarity odds |
| 2 | Comb Jelly | Sea | Primordial sea | C | More new species |
| 3 | Placozoan | Sea | Primordial sea | U | More morphs |
| 4 | Cnidarian | Sea | O≥1 | C | More new species |
| 5 | Jellyfish | Sea | O≥1, T≤2 | C | More morphs |
| 6 | Coral | Reef | Reef sea | U | Better rarity odds |
| 7 | Bilaterian | Sea | O≥2 | C | Chance of +1 card |
| 8 | Acoel Worm | Sea | Primordial sea | C | Shorter wait |
| 9 | Protostome | Sea | O≥2 | C | Better rarity odds |
| 10 | Deuterostome | Sea | O≥2 | U | Better rarity odds |
| 11 | Spiralian | Sea | O≥2 | C | More new species |
| 12 | Ecdysozoan | Sea | O≥2 | C | More morphs |
| 13 | Flatworm | Fresh water | O≥2, 1≤T≤2 | C | Chance of +1 card |
| 14 | Mollusc | Sea | O≥2 | U | More new species |
| 15 | Snail | Shore | O≥2, T≥3 | C | Shorter wait |
| 16 | Octopus | Reef | Reef sea | E | Copies the fish keystone above it (not a Legendary) |
| 17 | Segmented Worm | Land | 1≤V≤4, O≥2 | U | +1 adjustment point per 4h waited |
| 18 | Nematode | Sea or Land | O≥1 | C | Shorter wait |
| 19 | Arthropod | Sea | O≥2 | U | More morphs |
| 20 | Chelicerate | Shore | O≥2 | U | More new species |
| 21 | Horseshoe Crab | Shore | O≥2, 2≤T≤4 | R | Better rarity odds |
| 22 | Scorpion | Land | Coal swamp | R | More new species |
| 23 | Spider | Forest | Coal swamp | R | Chance of +1 card |
| 24 | Pancrustacean | Sea | O≥2 | C | More new species |
| 25 | Crab | Shore | O≥2, T≥3 | U | More new species |
| 26 | Insect | Land | V≥2, O≥3 | U | More new species |
| 27 | Dragonfly | Fresh water | Coal swamp | R | More morphs |
| 28 | Beetle | Forest | Coal swamp | U | Chance of +1 card |
| 29 | Butterfly | Forest | Jungle | R | Chance of +1 card |
| 30 | Ant | Forest | T≥3 | E | Chance of +1 card |
| 31 | Echinoderm | Sea | O≥2 | C | More new species |
| 32 | Starfish | Shore | O≥2 | C | Chance of +1 card |
| 33 | Sea Urchin | Reef | Reef sea | U | More new species |
| 34 | Chordate | Sea | O≥2 | U | Better rarity odds |
| 35 | Sea Squirt | Sea | O≥2, T≤2 | C | Shorter wait |
| 36 | Vertebrate | Sea | O≥3 | R | Better rarity odds |
| 37 | Lamprey | Sea or Fresh water | O≥2, T≤2 | U | More new species |
| 38 | Jawed Fish | Sea | O≥3 | U | Chance of +1 card |
| 39 | Shark | Sea | L≤2, O≥3 | R | Better rarity odds |
| 40 | Ray-finned Fish | Sea or Fresh water | O≥2 | C | Chance of +1 card |
| 41 | Lobe-finned Fish | Fresh water | O≥2 | R | More new species |
| 42 | Coelacanth | Sea | O≥2, T≤2 | L | Better rarity odds, growing while the planet stays the same; but 15% fewer new species |
| 43 | Tetrapod | Shore or Fresh water | V≥2, O≥2 | E | More new species |
| 44 | Amphibian | Fresh water | V≥2, T≥2 | U | More new species |
| 45 | Frog | Fresh water | Jungle | C | More morphs |
| 46 | Amniote | Land | V≥2 | R | More new species |
| 47 | Mammal | Land | V≥2, O≤4, 1≤T≤4 | R | Better rarity odds |
| 48 | Platypus | Fresh water | T≤2 | L | Every biome's bonus at once; but the wait is 50% longer |
| 49 | Marsupial | Land | O≤4, T≤4 | U | More morphs |
| 50 | Placental | Land | V≥2, O≤4, T≤4 | E | Chance of +1 card |
| 51 | Afrothere | Land | O≤4, 3≤T≤4 | U | Chance of +1 card |
| 52 | Mouse | Land | O≤4, T≤4 | C | Shorter wait |
| 53 | Bat | Forest | 3≤O≤4, 2≤T≤4 | R | More morphs |
| 54 | Carnivoran | Land | O≤4, T≤4 | U | Better rarity odds |
| 55 | Cetartiodactyl | Land | O≤4, T≤4 | U | More new species |
| 56 | Primate | Forest | O≤4, 3≤T≤4 | R | More new species |
| 57 | Lemur | Forest | Jungle | U | More morphs |
| 58 | Monkey | Forest | O≤4, 3≤T≤4 | R | Chance of +1 card |
| 59 | Ape | Forest | O≤4, 3≤T≤4 | E | Better rarity odds |
| 60 | Chimpanzee | Forest | Jungle | E | Copies the mammal keystone above it (not a Legendary) |
| 61 | Human | Land | Savanna | L | x2 Luck and +2 cards; but a 4h wait has 1 in 5 to end the earth |
| 62 | Reptile | Land | T≥3 | U | More new species |
| 63 | Turtle | Shore | T≥3 | U | Chance of +1 card |
| 64 | Lizard | Land | V≤3, T≥3 | C | Better rarity odds |
| 65 | Crocodile | Fresh water | T≥4 | R | Chance of +1 card |
| 66 | Dinosaur | Land | V≥2, T≥3 | E | Chance of +1 card |
| 67 | Bird | Forest | O≥3 | E | Better rarity odds |
| 68 | Clam | Reef | Reef sea | C | Chance of +1 card |
| 69 | Squid | Sea | L≤2, O≥3, T≤2 | U | Chance of +1 card |
| 70 | Ammonite | Sea | L≤2, O≥2, T≥3 | R | Better rarity odds |
| 71 | Earthworm | Land | V≥2, O≥2 | C | +1 adjustment point per 4h waited |
| 72 | Tardigrade | Sea or Land | – | E | Never asleep; +2 Luck per lever at its min or max |
| 73 | Trilobite | Sea | Primordial sea | U | More new species |
| 74 | Anomalocaris | Sea | Primordial sea | E | Better rarity odds |
| 75 | Centipede | Forest | Coal swamp | C | Shorter wait |
| 76 | Shrimp | Sea | L≤2, O≥2 | C | Chance of +1 card |
| 77 | Bee | Forest | V≥3, O≥3, T≥2 | U | Better rarity odds |
| 78 | Meganeura | Fresh water | Coal swamp | E | More morphs |
| 79 | Dunkleosteus | Sea | L≤2, O≥3, T≥3 | E | Better rarity odds |
| 80 | Stingray | Sea | L≤2, O≥3, T≥3 | U | More new species |
| 81 | Megalodon | Sea | L≤2, O≥3, T≥3 | L | Doubles all Luck; but temperature can't go below temperate |
| 82 | Seahorse | Reef | Reef sea | R | More morphs |
| 83 | Anglerfish | Sea | L≤2, O≥2, T≤2 | R | Better rarity odds |
| 84 | Tiktaalik | Shore or Fresh water | O≥3 | R | More new species |
| 85 | Salamander | Fresh water | 1≤T≤2 | C | Shorter wait |
| 86 | Snake | Land | T≥3 | U | Better rarity odds |
| 87 | Pterosaur | Shore | Hothouse | E | More new species |
| 88 | T. rex | Land | Hothouse | L | +2 cards in every genome; but the keystone in the last slot falls asleep |
| 89 | Sauropod | Forest | Hothouse | E | Chance of +1 card |
| 90 | Penguin | Shore | T≤2 | R | Better rarity odds |
| 91 | Mammoth | Land | Ice age | E | Chance of +1 card |
| 92 | Rabbit | Land | 1≤V≤4, O≤4, T≤4 | C | Chance of +1 card |
| 93 | Horse | Land | 1≤V≤3, 1≤T≤4 | U | Better rarity odds |
| 94 | Dolphin | Sea | L≤2, O≤4, 3≤T≤4 | R | Chance of +1 card |
| 95 | Lion | Land | Savanna | R | Better rarity odds |
| 96 | Gorilla | Forest | Jungle | R | +1 adjustment point per 4h waited |
| 97 | Nautilus | Reef | Reef sea | R | Better rarity odds |
| 98 | Fly | Land | O≥2 | C | Shorter wait |
| 99 | Clownfish | Reef | Reef sea | C | More new species |
| 100 | Lungfish | Fresh water | O≥2, T≥3 | R | More new species |
| 101 | Pig | Land | V≥1, O≤4, T≤4 | U | Better rarity odds |
| 102 | Tuatara | Forest | 1≤T≤2 | L | More new species, growing while the planet changes every cycle; but morphs are halved |
| 103 | Sea Turtle | Sea | Reef sea | R | Chance of +1 card |
| 104 | Plesiosaur | Sea | Hothouse | E | Chance of +1 card |
| 105 | Ichthyosaur | Sea | Hothouse | E | Better rarity odds |
| 106 | Triceratops | Land | Hothouse | E | +1 adjustment point per 4h waited |
| 107 | Velociraptor | Land | Hothouse | R | Shorter wait |
| 108 | Archaeopteryx | Forest | O≥3, T≥3 | L | Morphs x3; but one fewer boon to choose from |
| 109 | Ostrich | Land | Savanna | R | Better rarity odds |
| 110 | Parrot | Forest | Jungle | E | Copies the bird keystone above it (not a Legendary) |
| 111 | Owl | Forest | T≤2 | U | Better rarity odds |
| 112 | Koala | Forest | O≤4, 3≤T≤4 | U | +1 adjustment point per 4h waited |
| 113 | Sloth | Forest | Jungle | U | Chance of +1 card |
| 114 | Giraffe | Land | Savanna | R | Chance of +1 card |
| 115 | Hippo | Fresh water | O≤4, 3≤T≤4 | R | Chance of +1 card |
| 116 | Rhino | Land | Savanna | E | Better rarity odds |
| 117 | Bear | Forest | T≤2 | R | +1 adjustment point per 4h waited |
| 118 | Seal | Shore | Ice age | U | More new species |
| 119 | Synapsid | Land | O≥2 | U | Better rarity odds |
| 120 | Archosaur | Land | O≥2, T≥3 | U | More new species |
| 121 | Theropod | Land | O≥3, T≥3 | R | Chance of +1 card |
| 122 | Hominin | Land | V≥1, 3≤O≤4, 2≤T≤4 | E | Better rarity odds |
| 123 | Perissodactyl | Land | O≤4, T≤4 | U | Shorter wait |
| 124 | Spiny-rayed Fish | Sea | O≥2, T≥3 | C | Chance of +1 card |
| 125 | Elephant | Land | Savanna | E | +1 adjustment point per 4h waited |
| 126 | Whale | Sea | T≤2 | E | More new species |
| 127 | Wolf | Land | T≤2 | R | Better rarity odds |
| 128 | Dog | Land | O≤4, T≤4 | U | Better rarity odds |
| 129 | Kangaroo | Land | Savanna | R | More new species |
| 130 | Tiger | Forest | O≤4, T≤2 | E | Better rarity odds |
| 131 | Cat | Land | O≤4, T≤4 | C | More morphs |
| 132 | Sabre-tooth | Land | Ice age | E | Chance of +1 card |
| 133 | Giant Panda | Forest | V≥3, 1≤T≤2 | E | Chance of +1 card |
| 134 | Cow | Land | 1≤V≤4, O≤4, T≤4 | C | Better rarity odds |
| 135 | Chicken | Land | O≤4, T≤4 | C | Better rarity odds |
| 136 | Songbird | Forest | O≤4, T≤4 | C | Shorter wait |
| 137 | Dodo | Forest | Jungle | L | Better rarity odds x3; but falls asleep after one cycle |
| 138 | True Bug | Land | O≥2, T≥3 | C | Shorter wait |
| 139 | Wasp | Land | V≥1, O≥3, T≥3 | U | Better rarity odds |
| 140 | Mite | Sea or Land | O≥1 | C | Chance of +1 card |
| 141 | Termite | Forest | Jungle | U | +1 adjustment point per 4h waited |
| 142 | Moss Animal | Sea | Primordial sea | C | More new species |
| 143 | Brittle Star | Sea | Primordial sea | C | Shorter wait |
| 144 | Carp | Fresh water | O≥2, 1≤T≤2 | C | Chance of +1 card |
| 145 | Dickinsonia | Sea | Primordial sea | R | More morphs |
| 146 | Dimetrodon | Land | O≥2, T≥3 | R | Shorter wait |
| 147 | Lucy | Land | Savanna | E | +1 adjustment point per 4h waited |
| 148 | Neanderthal | Land | Ice age | E | Better rarity odds |
| 149 | Pakicetus | Shore | O≤4, 3≤T≤4 | R | More new species |
| 150 | Acanthostega | Fresh water | Coal swamp | R | More new species |

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
- **Opposite temperatures:** Wolf and Coelacanth want T≤2, while
  Crocodile wants T≥4. You can't have both at once, so every planet is a
  choice of who shows up.
- **The long road to Human** is 15 branch discoveries through forests,
  then out onto the Savanna. Humans are Legendary.

## Leaderboard

Players are ranked by **most advanced animal**: the depth of the deepest
discovered taxon. Human is step 16. Ties break by collection size, then by
who got there first. A secondary board ranks by morphs owned.

## Fairness

- No real-money purchases of cycles, genomes or odds, ever.
- Pity timers and specimen levels mean every genome moves you forward.
- The session cap is the cycle timer. Quick bonuses are capped at −30%.
