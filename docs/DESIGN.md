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

| Wait | Cards | Points next | Luck | Morphs | Extra |
|---|---|---|---|---|---|
| 2 h | 1 | 1 | +0 | x1 | |
| 3 h | 1.5 | 1 | +0.75 | x1 | |
| 4 h | 2 | 2 | +3 | x1.5 | |
| 5 h | 3.5 | 2 | +6.75 | x1.5 | |
| 6 h | 5 | 3 | +12 | x2 | one Rare or better guaranteed |

Half hours in between are interpolated; a fraction is the chance of one
more card. **Points next** are the adjustment points the next shaping gets
(before keystones and boons): a long wait buys a big reshape, a short one a
small tweak. The first shaping and the ones after the tutorial cycles get 3. Keystone bonuses stack on top (6 cards at most). The tutorial
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

Cards per genome come from the wait (see Pacing), + keystone and boon
bonuses, up to 6.

### Morphs: the second rarity axis

Any card can be a morph, rolled independently. Morphs never change what
the animal is; like card editions, they change what it does as a
keystone. The rarer, the stronger:

| Morph | Chance | Look | As a keystone |
|---|---|---|---|
| Giant | 1/20 | Drawn one size up | ×1.25 its bonus |
| Albino | 1/64 | Pale palette, red eyes | ×1.5, but asleep above Cool (sunburn) |
| Melanistic | 1/64 | Near-black palette | Lives 2 temperature steps colder |
| Amber | 1/512 | Gold-preserved, prismatic frame | ×2, and keeps its charges while asleep |

The player picks which owned morph each keystone works with (the MORPH
button on the Keystones screen, while shaping). A first Giant or Amber is
picked automatically. All the morph multipliers together are capped at ×6,
or nearly every card would morph.

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
- **A keystone only works while the planet suits it.** If its habitat,
  biome or needs aren't met (a frog on a dry world, a penguin in a
  hothouse), it falls **asleep** and gives nothing. While time runs, the
  planet as launched is the one that counts. Asleep keystones are greyed
  out with the reason, and the Keystones tab gets a warning dot.
- You can swap them while shaping and while time runs (the genome is rolled
  from them when the cycle ends). They lock while a genome waits to be
  opened.

**A bonus and a rule.** Each keystone has a **bonus**, what it adds, and a
**rule**, when and how much. Most rules lean on the planet, so the
keystones you equip decide the world you shape (`src/ecology.rs`, the
taxa table below).

The bonus's strength is tier units (Common 1, Uncommon 2, Rare 3, Epic 4,
Legendary 6) × level × morph:

| Bonus | Per unit | Cap |
|---|---|---|
| **Luck** | +1% to the rare-tier rolls | +15% |
| **Cards** | +12% chance of +1 card | 3 extra cards |
| **Morphs** | +8% morph chance | ×3 |
| **Quick** | −3% real wait | −30% |
| **Point** | +1 adjustment point | 8 points |
| **Soil** | +1 vegetation each cycle | – |
| **Habitat** | +10% odds for that habitat's animals | – |

| Rule | What it does | Examples |
|---|---|---|
| **Flat** | Always on | most commons |
| **When** | ×2, or only, under a planet condition | Penguin, Wolf (×2 when cold); Ant (only at 35% O₂); Crocodile, Hippo (only with 20%+ fresh water); Lizard, Ostrich (only on bare ground); Dimetrodon (only with volcanoes) |
| **Grows** | A charge per cycle run with a condition, half the bonus each; one cycle without it empties them | Coral (warm), Bee (forest), Sauropod (hothouse), Coelacanth (the planet left unchanged) |
| **Team** | Stronger with other keystones | Cow, Pig, Chicken, Horse (+half per other farm animal); Dog (×2 with a farm animal); Clownfish (×2 with a cnidarian); Butterfly (×2 with Bee); Shark (+half per fish) |
| **Copies** | Repeats another keystone's gains | Octopus (the one above); Chimpanzee (the one below, if a mammal); Parrot (the strongest, at half) |
| **Triggers** | Acts during the reveal | Owl (after a Rare, the next card is luckier); Bat (after a new species, more morphs); Spider (a duplicate feeds a keystone a specimen); Starfish (duplicates charge +⅓ card each, up to 1); Horseshoe Crab (nothing new brings the Legendary pity 2 closer) |
| **Fragile** | ×3, then leaves after 3 cycles until found again | Dodo |
| **Extremes** | Never asleep; +2 Luck per lever at its min or max | Tardigrade |

**Legendaries come with a catch:**

| Legendary | Perk | Catch |
|---|---|---|
| T. rex | +2 cards | Plant-eater keystones fall asleep |
| Megalodon | Doubles all Luck | Temperature can't go below Temperate |
| Human | +2 adjustment points | Vegetation can't go above Forest |
| Archaeopteryx | Morphs ×3 | One fewer boon to choose from |
| Tuatara | A Legendary within 25 genomes | The wait is an hour longer |
| Platypus | Counts as every team; more morphs | Can't be copied |

Coelacanth (grows with stasis) and Dodo (fragile) are the other two.
A lever a keystone holds shows a red button, and the panel says who holds it.

### Boons (short-term)

After each genome, you pick **1 of 3**, which applies to the next cycle
only:

- **Lure.** One clade (e.g. Arthropoda) gets ×3 weight.
- **Lens.** +1 card.
- **Catalyst.** Guaranteed Rare+ card.
- **Charm.** ×4 morph chance.
- **Tailwind.** −1 h wait this cycle.
- **Tectonics.** +2 adjustment points.

## The 151 taxa

Generated from the code (`cargo test print_doc_table -- --ignored
--nocapture`). **Needs** are a biome, or lever ranges on top of the
habitat: L = Land, V = Vegetation, O = Oxygen, T = Temperature.

| # | Taxon | Habitat | Needs | Tier | Keystone |
|---|---|---|---|---|---|
| 0 | Urmetazoan | Sea | – | C | Better rarity odds |
| 1 | Sea Sponge | Sea | Primordial sea | C | Better rarity odds |
| 2 | Comb Jelly | Sea | Primordial sea | C | More sea finds |
| 3 | Placozoan | Sea | Primordial sea | U | More morphs |
| 4 | Cnidarian | Sea | O≥1 | C | More sea finds |
| 5 | Jellyfish | Sea | O≥1, T≤2 | C | More morphs |
| 6 | Coral | Reef | Reef sea | U | Better rarity odds, growing each cycle at Temperate or warmer |
| 7 | Bilaterian | Sea | O≥2 | C | Chance of +1 card |
| 8 | Acoel Worm | Sea | Primordial sea | C | Shorter wait |
| 9 | Protostome | Sea | O≥2 | C | Better rarity odds |
| 10 | Deuterostome | Sea | O≥2 | U | Better rarity odds |
| 11 | Spiralian | Sea | O≥2 | C | More shore finds |
| 12 | Ecdysozoan | Sea | O≥2 | C | More morphs |
| 13 | Flatworm | Fresh water | O≥2, 1≤T≤2 | C | Chance of +1 card |
| 14 | Mollusc | Sea | O≥2 | U | More reef finds |
| 15 | Snail | Shore | O≥2, T≥3 | C | Shorter wait |
| 16 | Octopus | Reef | Reef sea | E | Copies the keystone above it |
| 17 | Segmented Worm | Land | 1≤V≤4, O≥2 | U | +1 vegetation each cycle |
| 18 | Nematode | Sea or Land | O≥1 | C | Shorter wait |
| 19 | Arthropod | Sea | O≥2 | U | More morphs |
| 20 | Chelicerate | Shore | O≥2 | U | More land finds |
| 21 | Horseshoe Crab | Shore | O≥2, 2≤T≤4 | R | Nothing new? A Legendary comes sooner |
| 22 | Scorpion | Land | Coal swamp | R | More land finds |
| 23 | Spider | Forest | Coal swamp | R | Each duplicate feeds a keystone a specimen |
| 24 | Pancrustacean | Sea | O≥2 | C | More shore finds |
| 25 | Crab | Shore | O≥2, T≥3 | U | More shore finds |
| 26 | Insect | Land | V≥2, O≥3 | U | More forest finds |
| 27 | Dragonfly | Fresh water | Coal swamp | R | Giant morphs x3 at 35% oxygen |
| 28 | Beetle | Forest | Coal swamp | U | Chance of +1 card |
| 29 | Butterfly | Forest | Jungle | R | Chance of +1 card; x2 with a pollinator keystone |
| 30 | Ant | Forest | T≥3 | E | Chance of +1 card, only at 35% oxygen |
| 31 | Echinoderm | Sea | O≥2 | C | More reef finds |
| 32 | Starfish | Shore | O≥2 | C | Each duplicate: +1/3 card, up to 1 |
| 33 | Sea Urchin | Reef | Reef sea | U | More reef finds |
| 34 | Chordate | Sea | O≥2 | U | Better rarity odds |
| 35 | Sea Squirt | Sea | O≥2, T≤2 | C | Shorter wait |
| 36 | Vertebrate | Sea | O≥3 | R | Better rarity odds |
| 37 | Lamprey | Sea or Fresh water | O≥2, T≤2 | U | More fresh water finds |
| 38 | Jawed Fish | Sea | O≥3 | U | Chance of +1 card |
| 39 | Shark | Sea | L≤2, O≥3 | R | Better rarity odds for each other fish keystone |
| 40 | Ray-finned Fish | Sea or Fresh water | O≥2 | C | Chance of +1 card |
| 41 | Lobe-finned Fish | Fresh water | O≥2 | R | More fresh water finds |
| 42 | Coelacanth | Sea | O≥2, T≤2 | L | Better rarity odds, growing while the planet stays the same |
| 43 | Tetrapod | Shore or Fresh water | V≥2, O≥2 | E | More land finds |
| 44 | Amphibian | Fresh water | V≥2, T≥2 | U | More fresh water finds |
| 45 | Frog | Fresh water | Jungle | C | More morphs |
| 46 | Amniote | Land | V≥2 | R | More land finds |
| 47 | Mammal | Land | V≥2, O≤4, 1≤T≤4 | R | Better rarity odds |
| 48 | Platypus | Fresh water | T≤2 | L | Counts as every team; more morphs; but can't be copied |
| 49 | Marsupial | Land | O≤4, T≤4 | U | More morphs |
| 50 | Placental | Land | V≥2, O≤4, T≤4 | E | Chance of +1 card |
| 51 | Afrothere | Land | O≤4, 3≤T≤4 | U | Chance of +1 card |
| 52 | Mouse | Land | O≤4, T≤4 | C | Shorter wait |
| 53 | Bat | Forest | 3≤O≤4, 2≤T≤4 | R | After a new species, the next cards morph more |
| 54 | Carnivoran | Land | O≤4, T≤4 | U | Better rarity odds |
| 55 | Cetartiodactyl | Land | O≤4, T≤4 | U | More land finds |
| 56 | Primate | Forest | O≤4, 3≤T≤4 | R | More forest finds |
| 57 | Lemur | Forest | Jungle | U | More morphs |
| 58 | Monkey | Forest | O≤4, 3≤T≤4 | R | Chance of +1 card |
| 59 | Ape | Forest | O≤4, 3≤T≤4 | E | Better rarity odds |
| 60 | Chimpanzee | Forest | Jungle | E | Copies the keystone below it, if a mammal |
| 61 | Human | Land | Savanna | L | +2 adjustment points; but vegetation can't go above forest |
| 62 | Reptile | Land | T≥3 | U | More land finds |
| 63 | Turtle | Shore | T≥3 | U | Duplicates count double |
| 64 | Lizard | Land | V≤3, T≥3 | C | Better rarity odds, only on bare ground |
| 65 | Crocodile | Fresh water | T≥4 | R | Chance of +1 card, only with 20%+ fresh water |
| 66 | Dinosaur | Land | V≥2, T≥3 | E | Chance of +1 card |
| 67 | Bird | Forest | O≥3 | E | Better rarity odds |
| 68 | Clam | Reef | Reef sea | C | Duplicates count double |
| 69 | Squid | Sea | L≤2, O≥3, T≤2 | U | Chance of +1 card |
| 70 | Ammonite | Sea | L≤2, O≥2, T≥3 | R | Better rarity odds |
| 71 | Earthworm | Land | V≥2, O≥2 | C | +1 vegetation each cycle |
| 72 | Tardigrade | Sea or Land | – | E | Never asleep; +2 Luck per lever at its min or max |
| 73 | Trilobite | Sea | Primordial sea | U | More sea finds |
| 74 | Anomalocaris | Sea | Primordial sea | E | Better rarity odds |
| 75 | Centipede | Forest | Coal swamp | C | Shorter wait |
| 76 | Shrimp | Sea | L≤2, O≥2 | C | Chance of +1 card |
| 77 | Bee | Forest | V≥3, O≥3, T≥2 | U | Better rarity odds, growing each cycle with forest |
| 78 | Meganeura | Fresh water | Coal swamp | E | Giant morphs x3 at 35% oxygen |
| 79 | Dunkleosteus | Sea | L≤2, O≥3, T≥3 | E | Better rarity odds |
| 80 | Stingray | Sea | L≤2, O≥3, T≥3 | U | More sea finds |
| 81 | Megalodon | Sea | L≤2, O≥3, T≥3 | L | Doubles all Luck; but temperature can't go below temperate |
| 82 | Seahorse | Reef | Reef sea | R | More morphs |
| 83 | Anglerfish | Sea | L≤2, O≥2, T≤2 | R | Better rarity odds |
| 84 | Tiktaalik | Shore or Fresh water | O≥3 | R | More shore finds |
| 85 | Salamander | Fresh water | 1≤T≤2 | C | Shorter wait |
| 86 | Snake | Land | T≥3 | U | Better rarity odds |
| 87 | Pterosaur | Shore | Hothouse | E | More shore finds |
| 88 | T. rex | Land | Hothouse | L | +2 cards in every genome; but plant-eater keystones fall asleep |
| 89 | Sauropod | Forest | Hothouse | E | Chance of +1 card, growing each cycle in the Hothouse |
| 90 | Penguin | Shore | T≤2 | R | Better rarity odds; x2 at Snowball or Cold |
| 91 | Mammoth | Land | Ice age | E | Chance of +1 card |
| 92 | Rabbit | Land | 1≤V≤4, O≤4, T≤4 | C | Duplicates count double |
| 93 | Horse | Land | 1≤V≤3, 1≤T≤4 | U | Better rarity odds for each other farm animal keystone |
| 94 | Dolphin | Sea | L≤2, O≤4, 3≤T≤4 | R | Chance of +1 card |
| 95 | Lion | Land | Savanna | R | Better rarity odds |
| 96 | Gorilla | Forest | Jungle | R | +1 adjustment point |
| 97 | Nautilus | Reef | Reef sea | R | Luck; legendary pity 30 |
| 98 | Fly | Land | O≥2 | C | Shorter wait |
| 99 | Clownfish | Reef | Reef sea | C | More reef finds; x2 with a cnidarian keystone |
| 100 | Lungfish | Fresh water | O≥2, T≥3 | R | More fresh water finds |
| 101 | Pig | Land | V≥1, O≤4, T≤4 | U | Better rarity odds for each other farm animal keystone |
| 102 | Tuatara | Forest | 1≤T≤2 | L | A Legendary within 25 genomes; but the wait is an hour longer |
| 103 | Sea Turtle | Sea | Reef sea | R | Duplicates count double |
| 104 | Plesiosaur | Sea | Hothouse | E | Chance of +1 card |
| 105 | Ichthyosaur | Sea | Hothouse | E | Better rarity odds |
| 106 | Triceratops | Land | Hothouse | E | +1 adjustment point |
| 107 | Velociraptor | Land | Hothouse | R | Shorter wait |
| 108 | Archaeopteryx | Forest | O≥3, T≥3 | L | Morphs x3; but one fewer boon to choose from |
| 109 | Ostrich | Land | Savanna | R | Better rarity odds, only on bare ground |
| 110 | Parrot | Forest | Jungle | E | Copies the strongest keystone at half |
| 111 | Owl | Forest | T≤2 | U | After a Rare or better card, the next is luckier |
| 112 | Koala | Forest | O≤4, 3≤T≤4 | U | +1 vegetation each cycle |
| 113 | Sloth | Forest | Jungle | U | Duplicates count double |
| 114 | Giraffe | Land | Savanna | R | Chance of +1 card |
| 115 | Hippo | Fresh water | O≤4, 3≤T≤4 | R | Chance of +1 card, only with 20%+ fresh water |
| 116 | Rhino | Land | Savanna | E | Better rarity odds |
| 117 | Bear | Forest | T≤2 | R | +1 adjustment point |
| 118 | Seal | Shore | Ice age | U | More shore finds |
| 119 | Synapsid | Land | O≥2 | U | Better rarity odds |
| 120 | Archosaur | Land | O≥2, T≥3 | U | More land finds |
| 121 | Theropod | Land | O≥3, T≥3 | R | Chance of +1 card |
| 122 | Hominin | Land | V≥1, 3≤O≤4, 2≤T≤4 | E | Better rarity odds |
| 123 | Perissodactyl | Land | O≤4, T≤4 | U | Shorter wait |
| 124 | Spiny-rayed Fish | Sea | O≥2, T≥3 | C | Chance of +1 card |
| 125 | Elephant | Land | Savanna | E | +1 adjustment point |
| 126 | Whale | Sea | T≤2 | E | More sea finds |
| 127 | Wolf | Land | T≤2 | R | Better rarity odds; x2 at Snowball or Cold |
| 128 | Dog | Land | O≤4, T≤4 | U | Better rarity odds; x2 with a farm animal keystone |
| 129 | Kangaroo | Land | Savanna | R | More land finds |
| 130 | Tiger | Forest | O≤4, T≤2 | E | Better rarity odds |
| 131 | Cat | Land | O≤4, T≤4 | C | More morphs |
| 132 | Sabre-tooth | Land | Ice age | E | Chance of +1 card |
| 133 | Giant Panda | Forest | V≥3, 1≤T≤2 | E | Duplicates count double |
| 134 | Cow | Land | 1≤V≤4, O≤4, T≤4 | C | Better rarity odds for each other farm animal keystone |
| 135 | Chicken | Land | O≤4, T≤4 | C | Better rarity odds for each other farm animal keystone |
| 136 | Songbird | Forest | O≤4, T≤4 | C | Shorter wait |
| 137 | Dodo | Forest | Jungle | L | Better rarity odds x3, then it leaves after 3 cycles |
| 138 | True Bug | Land | O≥2, T≥3 | C | Shorter wait |
| 139 | Wasp | Land | V≥1, O≥3, T≥3 | U | Better rarity odds |
| 140 | Mite | Sea or Land | O≥1 | C | Duplicates count double |
| 141 | Termite | Forest | Jungle | U | +1 vegetation each cycle |
| 142 | Moss Animal | Sea | Primordial sea | C | More sea finds |
| 143 | Brittle Star | Sea | Primordial sea | C | Shorter wait |
| 144 | Carp | Fresh water | O≥2, 1≤T≤2 | C | Duplicates count double |
| 145 | Dickinsonia | Sea | Primordial sea | R | More morphs; one every 7 genomes |
| 146 | Dimetrodon | Land | O≥2, T≥3 | R | Shorter wait, only with volcanoes |
| 147 | Lucy | Land | Savanna | E | +1 adjustment point |
| 148 | Neanderthal | Land | Ice age | E | Better rarity odds |
| 149 | Pakicetus | Shore | O≤4, 3≤T≤4 | R | More shore finds |
| 150 | Acanthostega | Fresh water | Coal swamp | R | More fresh water finds |

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
