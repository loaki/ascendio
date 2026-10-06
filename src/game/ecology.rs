//! Per-taxon planet needs, tiers and keystone effects, keyed by `Taxon::name`.

use serde::{Deserialize, Serialize};

use crate::game::planet::{Biome, Habitat, Planet, Ranges};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Tier {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Tier {
    pub const ALL: [Tier; 5] = [
        Tier::Common,
        Tier::Uncommon,
        Tier::Rare,
        Tier::Epic,
        Tier::Legendary,
    ];

    pub fn units(self) -> f32 {
        match self {
            Tier::Common => 1.0,
            Tier::Uncommon => 2.0,
            Tier::Rare => 3.0,
            Tier::Epic => 4.0,
            Tier::Legendary => 6.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tier::Common => "Common",
            Tier::Uncommon => "Uncommon",
            Tier::Rare => "Rare",
            Tier::Epic => "Epic",
            Tier::Legendary => "Legendary",
        }
    }

    pub fn color(self) -> u32 {
        match self {
            Tier::Common => 0x8A97A3,
            Tier::Uncommon => 0x6FDC6A,
            Tier::Rare => 0x5AA8FF,
            Tier::Epic => 0xC07BFF,
            Tier::Legendary => 0xFFC84A,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bonus {
    Luck,
    Cards,
    Discovery,
    Morph,
    Quick,
    /// +1 adjustment point per 4h waited (not scaled).
    Point,
}

impl Bonus {
    pub const ALL: [Bonus; 6] = [
        Bonus::Luck,
        Bonus::Cards,
        Bonus::Discovery,
        Bonus::Morph,
        Bonus::Quick,
        Bonus::Point,
    ];

    pub fn describe(self) -> String {
        match self {
            Bonus::Luck => "Better rarity odds",
            Bonus::Cards => "Chance of +1 card",
            Bonus::Discovery => "More new species",
            Bonus::Morph => "More morphs",
            Bonus::Quick => "Shorter wait",
            Bonus::Point => "+1 adjustment point per 4h waited",
        }
        .into()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kin {
    Fish,
    Mammal,
    Bird,
}

impl Kin {
    pub fn name(self) -> &'static str {
        match self {
            Kin::Fish => "fish",
            Kin::Mammal => "mammal",
            Kin::Bird => "bird",
        }
    }

    pub fn clade(self) -> (&'static str, Option<&'static str>) {
        match self {
            Kin::Fish => ("Vertebrate", Some("Tetrapod")),
            Kin::Mammal => ("Mammal", None),
            Kin::Bird => ("Bird", None),
        }
    }
}

/// A legendary's strong effect and its drawback.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Catch {
    /// +2 cards; the keystone in the last slot falls asleep.
    Tyrant,
    /// x2 all luck; Temperature can't go below Temperate.
    Apex,
    /// Morphs x3; one fewer boon to choose from.
    Feathers,
    /// x2 luck and +2 cards; a 4h wait has 1 in 5 to end the Earth instead.
    Hubris,
    /// Every biome's bonus; the wait is 50% longer.
    AllBiomes,
}

impl Catch {
    pub fn perk(self) -> &'static str {
        match self {
            Catch::Tyrant => "+2 cards in every genome",
            Catch::Apex => "Doubles all Luck",
            Catch::Feathers => "Morphs x3",
            Catch::Hubris => "x2 Luck and +2 cards",
            Catch::AllBiomes => "Every biome's bonus at once",
        }
    }

    pub fn drawback(self) -> &'static str {
        match self {
            Catch::Tyrant => "The keystone in the last slot falls asleep",
            Catch::Apex => "Temperature can't go below Temperate",
            Catch::Feathers => "One fewer boon to choose from",
            Catch::Hubris => "A 4h wait has 1 in 5 to end the Earth",
            Catch::AllBiomes => "The wait is 50% longer",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rule {
    Flat,
    /// Copies the keystone in the slot above, if of this kin and not a
    /// Legendary.
    Copy(Kin),
    /// Never asleep; +2 luck per lever at its lowest or highest.
    Extremes,
    /// x3, but asleep after a cycle (a duplicate wakes it).
    Fragile,
    /// A charge per 40 Ma waited on a planet changed since the wait before
    /// (+20% of the bonus each); a wait on the same planet empties them.
    Changing,
    /// The opposite: charges while the planet stays exactly the same.
    Stasis,
    Catch(Catch),
}

impl Rule {
    pub fn describe(self, bonus: Bonus) -> String {
        let b = bonus.describe();
        match self {
            Rule::Flat => b,
            Rule::Copy(k) => format!(
                "Copies the {} keystone above it (not a Legendary)",
                k.name()
            ),
            Rule::Extremes => "Never asleep; +2 Luck per lever at its min or max".into(),
            Rule::Fragile => format!("{b} x3"),
            Rule::Changing => format!("{b}, growing while the planet changes every cycle"),
            Rule::Stasis => format!("{b}, growing while the planet stays the same"),
            Rule::Catch(c) => c.perk().into(),
        }
    }

    pub fn drawback(self) -> Option<&'static str> {
        match self {
            Rule::Fragile => Some("Falls asleep after one cycle"),
            Rule::Changing => Some("Morphs are halved"),
            Rule::Stasis => Some("15 points less chance of a new species"),
            Rule::Catch(c) => Some(c.drawback()),
            _ => None,
        }
    }
}

pub const BIOME_SET: usize = 3;

pub fn biome_bonus(b: Biome) -> (Bonus, f32) {
    match b {
        Biome::Primordial => (Bonus::Discovery, 0.15),
        Biome::Reef => (Bonus::Morph, 0.5),
        Biome::IceAge => (Bonus::Quick, 0.15),
        Biome::CoalSwamp => (Bonus::Cards, 1.0),
        Biome::Jungle => (Bonus::Luck, 5.0),
        Biome::Savanna => (Bonus::Point, 1.0),
        Biome::Hothouse => (Bonus::Cards, 1.0),
    }
}

pub fn biome_bonus_label(b: Biome) -> String {
    let (bonus, x) = biome_bonus(b);
    match bonus {
        Bonus::Luck => format!("+{x:.0} luck"),
        Bonus::Cards => format!("+{x:.0} card"),
        Bonus::Discovery => format!("+{:.0}% discovery", x * 100.0),
        Bonus::Morph => format!("+{:.0}% morphs", x * 100.0),
        Bonus::Quick => format!("-{:.0}% wait", x * 100.0),
        Bonus::Point => format!("+{x:.0} point per 4h"),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Needs {
    pub habitats: &'static [Habitat],
    pub ranges: Ranges,
    pub biome: Option<Biome>,
}

impl Needs {
    pub fn met_by(&self, p: &Planet) -> bool {
        self.missing(p).is_none()
    }

    pub fn missing(&self, p: &Planet) -> Option<String> {
        self.missing_with(p, false)
    }

    pub fn missing_with(&self, p: &Planet, any_temperature: bool) -> Option<String> {
        if !self.habitats.iter().any(|&h| p.has(h)) {
            let names: Vec<_> = self
                .habitats
                .iter()
                .map(|h| h.name().to_lowercase())
                .collect();
            return Some(format!("needs {}", names.join(" or ")));
        }
        let why = self.ranges.missing(p, any_temperature)?;
        Some(match self.biome {
            Some(b) => format!("{} only: {why}", b.name()),
            None => why.to_string(),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ecology {
    pub needs: Needs,
    pub tier: Tier,
    pub bonus: Bonus,
    pub rule: Rule,
}

impl Ecology {
    pub fn describe(&self) -> String {
        self.rule.describe(self.bonus)
    }
}

use Habitat::*;
use Tier::*;

const ANY: Ranges = Ranges::ANY;

const fn n(habitats: &'static [Habitat], oxygen_min: u8) -> Needs {
    let mut ranges = ANY;
    ranges.oxygen.0 = oxygen_min;
    Needs {
        habitats,
        ranges,
        biome: None,
    }
}
const fn t(mut needs: Needs, temp_min: u8, temp_max: u8) -> Needs {
    needs.ranges.temperature = (temp_min, temp_max);
    needs
}
const fn v(mut needs: Needs, veg_min: u8, veg_max: u8) -> Needs {
    needs.ranges.vegetation = (veg_min, veg_max);
    needs
}
const fn ocean(mut needs: Needs) -> Needs {
    needs.ranges.land.1 = 2;
    needs
}
const fn fur(mut needs: Needs) -> Needs {
    needs.ranges.oxygen.1 = 4;
    if needs.ranges.temperature.1 > 4 {
        needs.ranges.temperature.1 = 4;
    }
    needs
}
const fn b(biome: Biome, habitats: &'static [Habitat]) -> Needs {
    Needs {
        habitats,
        ranges: biome.ranges(),
        biome: Some(biome),
    }
}

const SEA: &[Habitat] = &[Sea];
const REEF: &[Habitat] = &[Reef];
const SHORE: &[Habitat] = &[Shore];
const FRESH: &[Habitat] = &[Fresh];
const LAND: &[Habitat] = &[Land];
const FOREST: &[Habitat] = &[Forest];
const SEA_OR_LAND: &[Habitat] = &[Sea, Land];
const SEA_OR_FRESH: &[Habitat] = &[Sea, Fresh];
const SHORE_OR_FRESH: &[Habitat] = &[Shore, Fresh];

use Biome::{CoalSwamp, Hothouse, IceAge, Jungle, Primordial, Savanna};
use Bonus as B;
use Rule as R;

#[rustfmt::skip]
const TABLE: &[(&str, Needs, Tier, Bonus, Rule)] = &[
    ("Urmetazoan",       n(SEA, 0),                         Common,    B::Luck,      R::Flat),
    ("Sea Sponge",       b(Primordial, SEA),                Common,    B::Luck,      R::Flat),
    ("Comb Jelly",       b(Primordial, SEA),                Common,    B::Discovery, R::Flat),
    ("Placozoan",        b(Primordial, SEA),                Uncommon,  B::Morph,     R::Flat),
    ("Cnidarian",        n(SEA, 1),                         Common,    B::Discovery, R::Flat),
    ("Jellyfish",        t(n(SEA, 1), 0, 2),                Common,    B::Morph,     R::Flat),
    ("Coral",            b(Biome::Reef, REEF),              Uncommon,  B::Luck,      R::Flat),
    ("Bilaterian",       n(SEA, 2),                         Common,    B::Cards,     R::Flat),
    ("Acoel Worm",       b(Primordial, SEA),                Common,    B::Quick,     R::Flat),
    ("Protostome",       n(SEA, 2),                         Common,    B::Luck,      R::Flat),
    ("Deuterostome",     n(SEA, 2),                         Uncommon,  B::Luck,      R::Flat),
    ("Spiralian",        n(SEA, 2),                         Common,    B::Discovery, R::Flat),
    ("Ecdysozoan",       n(SEA, 2),                         Common,    B::Morph,     R::Flat),
    ("Flatworm",         t(n(FRESH, 2), 1, 2),              Common,    B::Cards,     R::Flat),
    ("Mollusc",          n(SEA, 2),                         Uncommon,  B::Discovery, R::Flat),
    ("Snail",            t(n(SHORE, 2), 3, 5),              Common,    B::Quick,     R::Flat),
    ("Octopus",          b(Biome::Reef, REEF),              Epic,      B::Luck,      R::Copy(Kin::Fish)),
    ("Segmented Worm",   v(n(LAND, 2), 1, 4),               Uncommon,  B::Point,     R::Flat),
    ("Nematode",         n(SEA_OR_LAND, 1),                 Common,    B::Quick,     R::Flat),
    ("Arthropod",        n(SEA, 2),                         Uncommon,  B::Morph,     R::Flat),
    ("Chelicerate",      n(SHORE, 2),                       Uncommon,  B::Discovery, R::Flat),
    ("Horseshoe Crab",   t(n(SHORE, 2), 2, 4),              Rare,      B::Luck,      R::Flat),
    ("Scorpion",         b(CoalSwamp, LAND),                Rare,      B::Discovery, R::Flat),
    ("Spider",           b(CoalSwamp, FOREST),              Rare,      B::Cards,     R::Flat),
    ("Pancrustacean",    n(SEA, 2),                         Common,    B::Discovery, R::Flat),
    ("Crab",             t(n(SHORE, 2), 3, 5),              Uncommon,  B::Discovery, R::Flat),
    ("Insect",           v(n(LAND, 3), 2, 5),               Uncommon,  B::Discovery, R::Flat),
    ("Dragonfly",        b(CoalSwamp, FRESH),               Rare,      B::Morph,     R::Flat),
    ("Beetle",           b(CoalSwamp, FOREST),              Uncommon,  B::Cards,     R::Flat),
    ("Butterfly",        b(Jungle, FOREST),                 Rare,      B::Cards,     R::Flat),
    ("Ant",              t(n(FOREST, 0), 3, 5),             Epic,      B::Cards,     R::Flat),
    ("Echinoderm",       n(SEA, 2),                         Common,    B::Discovery, R::Flat),
    ("Starfish",         n(SHORE, 2),                       Common,    B::Cards,     R::Flat),
    ("Sea Urchin",       b(Biome::Reef, REEF),              Uncommon,  B::Discovery, R::Flat),
    ("Chordate",         n(SEA, 2),                         Uncommon,  B::Luck,      R::Flat),
    ("Sea Squirt",       t(n(SEA, 2), 0, 2),                Common,    B::Quick,     R::Flat),
    ("Vertebrate",       n(SEA, 3),                         Rare,      B::Luck,      R::Flat),
    ("Lamprey",          t(n(SEA_OR_FRESH, 2), 0, 2),       Uncommon,  B::Discovery, R::Flat),
    ("Jawed Fish",       n(SEA, 3),                         Uncommon,  B::Cards,     R::Flat),
    ("Shark",            ocean(n(SEA, 3)),                  Rare,      B::Luck,      R::Flat),
    ("Ray-finned Fish",  n(SEA_OR_FRESH, 2),                Common,    B::Cards,     R::Flat),
    ("Lobe-finned Fish", n(FRESH, 2),                       Rare,      B::Discovery, R::Flat),
    ("Coelacanth",       t(n(SEA, 2), 0, 2),                Legendary, B::Luck,      R::Stasis),
    ("Tetrapod",         v(n(SHORE_OR_FRESH, 2), 2, 5),     Epic,      B::Discovery, R::Flat),
    ("Amphibian",        v(t(n(FRESH, 0), 2, 5), 2, 5),     Uncommon,  B::Discovery, R::Flat),
    ("Frog",             b(Jungle, FRESH),                  Common,    B::Morph,     R::Flat),
    ("Amniote",          v(n(LAND, 0), 2, 5),               Rare,      B::Discovery, R::Flat),
    ("Mammal",           fur(v(t(n(LAND, 0), 1, 5), 2, 5)), Rare,      B::Luck,      R::Flat),
    ("Platypus",         t(n(FRESH, 0), 0, 2),              Legendary, B::Luck,      R::Catch(Catch::AllBiomes)),
    ("Marsupial",        fur(n(LAND, 0)),                   Uncommon,  B::Morph,     R::Flat),
    ("Placental",        fur(v(n(LAND, 0), 2, 5)),          Epic,      B::Cards,     R::Flat),
    ("Afrothere",        fur(t(n(LAND, 0), 3, 5)),          Uncommon,  B::Cards,     R::Flat),
    ("Mouse",            fur(n(LAND, 0)),                   Common,    B::Quick,     R::Flat),
    ("Bat",              fur(t(n(FOREST, 3), 2, 5)),        Rare,      B::Morph,     R::Flat),
    ("Carnivoran",       fur(n(LAND, 0)),                   Uncommon,  B::Luck,      R::Flat),
    ("Cetartiodactyl",   fur(n(LAND, 0)),                   Uncommon,  B::Discovery, R::Flat),
    ("Primate",          fur(t(n(FOREST, 0), 3, 5)),        Rare,      B::Discovery, R::Flat),
    ("Lemur",            b(Jungle, FOREST),                 Uncommon,  B::Morph,     R::Flat),
    ("Monkey",           fur(t(n(FOREST, 0), 3, 5)),        Rare,      B::Cards,     R::Flat),
    ("Ape",              fur(t(n(FOREST, 0), 3, 5)),        Epic,      B::Luck,      R::Flat),
    ("Chimpanzee",       b(Jungle, FOREST),                 Epic,      B::Luck,      R::Copy(Kin::Mammal)),
    ("Human",            b(Savanna, LAND),                  Legendary, B::Luck,      R::Catch(Catch::Hubris)),
    ("Reptile",          t(n(LAND, 0), 3, 5),               Uncommon,  B::Discovery, R::Flat),
    ("Turtle",           t(n(SHORE, 0), 3, 5),              Uncommon,  B::Cards,     R::Flat),
    ("Lizard",           v(t(n(LAND, 0), 3, 5), 0, 3),      Common,    B::Luck,      R::Flat),
    ("Crocodile",        t(n(FRESH, 0), 4, 5),              Rare,      B::Cards,     R::Flat),
    ("Dinosaur",         v(t(n(LAND, 0), 3, 5), 2, 5),      Epic,      B::Cards,     R::Flat),
    ("Bird",             n(FOREST, 3),                      Epic,      B::Luck,      R::Flat),
    ("Clam",             b(Biome::Reef, REEF),              Common,    B::Cards,     R::Flat),
    ("Squid",            ocean(t(n(SEA, 3), 0, 2)),         Uncommon,  B::Cards,     R::Flat),
    ("Ammonite",         ocean(t(n(SEA, 2), 3, 5)),         Rare,      B::Luck,      R::Flat),
    ("Earthworm",        v(n(LAND, 2), 2, 5),               Common,    B::Point,     R::Flat),
    ("Tardigrade",       n(SEA_OR_LAND, 0),                 Epic,      B::Luck,      R::Extremes),
    ("Trilobite",        b(Primordial, SEA),                Uncommon,  B::Discovery, R::Flat),
    ("Anomalocaris",     b(Primordial, SEA),                Epic,      B::Luck,      R::Flat),
    ("Centipede",        b(CoalSwamp, FOREST),              Common,    B::Quick,     R::Flat),
    ("Shrimp",           ocean(n(SEA, 2)),                  Common,    B::Cards,     R::Flat),
    ("Bee",              t(v(n(FOREST, 3), 3, 5), 2, 5),    Uncommon,  B::Luck,      R::Flat),
    ("Meganeura",        b(CoalSwamp, FRESH),               Epic,      B::Morph,     R::Flat),
    ("Dunkleosteus",     ocean(t(n(SEA, 3), 3, 5)),         Epic,      B::Luck,      R::Flat),
    ("Stingray",         ocean(t(n(SEA, 3), 3, 5)),         Uncommon,  B::Discovery, R::Flat),
    ("Megalodon",        ocean(t(n(SEA, 3), 3, 5)),         Legendary, B::Luck,      R::Catch(Catch::Apex)),
    ("Seahorse",         b(Biome::Reef, REEF),              Rare,      B::Morph,     R::Flat),
    ("Anglerfish",       ocean(t(n(SEA, 2), 0, 2)),         Rare,      B::Luck,      R::Flat),
    ("Tiktaalik",        n(SHORE_OR_FRESH, 3),              Rare,      B::Discovery, R::Flat),
    ("Salamander",       t(n(FRESH, 0), 1, 2),              Common,    B::Quick,     R::Flat),
    ("Snake",            t(n(LAND, 0), 3, 5),               Uncommon,  B::Luck,      R::Flat),
    ("Pterosaur",        b(Hothouse, SHORE),                Epic,      B::Discovery, R::Flat),
    ("T. rex",           b(Hothouse, LAND),                 Legendary, B::Cards,     R::Catch(Catch::Tyrant)),
    ("Sauropod",         b(Hothouse, FOREST),               Epic,      B::Cards,     R::Flat),
    ("Penguin",          t(n(SHORE, 0), 0, 2),              Rare,      B::Luck,      R::Flat),
    ("Mammoth",          b(IceAge, LAND),                   Epic,      B::Cards,     R::Flat),
    ("Rabbit",           fur(v(n(LAND, 0), 1, 4)),          Common,    B::Cards,     R::Flat),
    ("Horse",            v(t(n(LAND, 0), 1, 4), 1, 3),      Uncommon,  B::Luck,      R::Flat),
    ("Dolphin",          ocean(fur(t(n(SEA, 0), 3, 5))),    Rare,      B::Cards,     R::Flat),
    ("Lion",             b(Savanna, LAND),                  Rare,      B::Luck,      R::Flat),
    ("Gorilla",          b(Jungle, FOREST),                 Rare,      B::Point,     R::Flat),
    ("Nautilus",         b(Biome::Reef, REEF),              Rare,      B::Luck,      R::Flat),
    ("Fly",              n(LAND, 2),                        Common,    B::Quick,     R::Flat),
    ("Clownfish",        b(Biome::Reef, REEF),              Common,    B::Discovery, R::Flat),
    ("Lungfish",         t(n(FRESH, 2), 3, 5),              Rare,      B::Discovery, R::Flat),
    ("Pig",              fur(v(n(LAND, 0), 1, 5)),          Uncommon,  B::Luck,      R::Flat),
    ("Tuatara",          t(n(FOREST, 0), 1, 2),             Legendary, B::Discovery, R::Changing),
    ("Sea Turtle",       b(Biome::Reef, SEA),               Rare,      B::Cards,     R::Flat),
    ("Plesiosaur",       b(Hothouse, SEA),                  Epic,      B::Cards,     R::Flat),
    ("Ichthyosaur",      b(Hothouse, SEA),                  Epic,      B::Luck,      R::Flat),
    ("Triceratops",      b(Hothouse, LAND),                 Epic,      B::Point,     R::Flat),
    ("Velociraptor",     b(Hothouse, LAND),                 Rare,      B::Quick,     R::Flat),
    ("Archaeopteryx",    t(n(FOREST, 3), 3, 5),             Legendary, B::Morph,     R::Catch(Catch::Feathers)),
    ("Ostrich",          b(Savanna, LAND),                  Rare,      B::Luck,      R::Flat),
    ("Parrot",           b(Jungle, FOREST),                 Epic,      B::Luck,      R::Copy(Kin::Bird)),
    ("Owl",              t(n(FOREST, 0), 0, 2),             Uncommon,  B::Luck,      R::Flat),
    ("Koala",            fur(t(n(FOREST, 0), 3, 5)),        Uncommon,  B::Point,     R::Flat),
    ("Sloth",            b(Jungle, FOREST),                 Uncommon,  B::Cards,     R::Flat),
    ("Giraffe",          b(Savanna, LAND),                  Rare,      B::Cards,     R::Flat),
    ("Hippo",            fur(t(n(FRESH, 0), 3, 5)),         Rare,      B::Cards,     R::Flat),
    ("Rhino",            b(Savanna, LAND),                  Epic,      B::Luck,      R::Flat),
    ("Bear",             t(n(FOREST, 0), 0, 2),             Rare,      B::Point,     R::Flat),
    ("Seal",             b(IceAge, SHORE),                  Uncommon,  B::Discovery, R::Flat),
    ("Synapsid",         n(LAND, 2),                        Uncommon,  B::Luck,      R::Flat),
    ("Archosaur",        t(n(LAND, 2), 3, 5),               Uncommon,  B::Discovery, R::Flat),
    ("Theropod",         t(n(LAND, 3), 3, 5),               Rare,      B::Cards,     R::Flat),
    ("Hominin",          fur(v(t(n(LAND, 3), 2, 4), 1, 5)), Epic,      B::Luck,      R::Flat),
    ("Perissodactyl",    fur(n(LAND, 0)),                   Uncommon,  B::Quick,     R::Flat),
    ("Spiny-rayed Fish", t(n(SEA, 2), 3, 5),                Common,    B::Cards,     R::Flat),
    ("Elephant",         b(Savanna, LAND),                  Epic,      B::Point,     R::Flat),
    ("Whale",            t(n(SEA, 0), 0, 2),                Epic,      B::Discovery, R::Flat),
    ("Wolf",             t(n(LAND, 0), 0, 2),               Rare,      B::Luck,      R::Flat),
    ("Dog",              fur(n(LAND, 0)),                   Uncommon,  B::Luck,      R::Flat),
    ("Kangaroo",         b(Savanna, LAND),                  Rare,      B::Discovery, R::Flat),
    ("Tiger",            fur(t(n(FOREST, 0), 0, 2)),        Epic,      B::Luck,      R::Flat),
    ("Cat",              fur(n(LAND, 0)),                   Common,    B::Morph,     R::Flat),
    ("Sabre-tooth",      b(IceAge, LAND),                   Epic,      B::Cards,     R::Flat),
    ("Giant Panda",      v(t(n(FOREST, 0), 1, 2), 3, 5),    Epic,      B::Cards,     R::Flat),
    ("Cow",              fur(v(n(LAND, 0), 1, 4)),          Common,    B::Luck,      R::Flat),
    ("Chicken",          fur(n(LAND, 0)),                   Common,    B::Luck,      R::Flat),
    ("Songbird",         fur(n(FOREST, 0)),                 Common,    B::Quick,     R::Flat),
    ("Dodo",             b(Jungle, FOREST),                 Legendary, B::Luck,      R::Fragile),
    ("True Bug",         t(n(LAND, 2), 3, 5),               Common,    B::Quick,     R::Flat),
    ("Wasp",             v(t(n(LAND, 3), 3, 5), 1, 5),      Uncommon,  B::Luck,      R::Flat),
    ("Mite",             n(SEA_OR_LAND, 1),                 Common,    B::Cards,     R::Flat),
    ("Termite",          b(Jungle, FOREST),                 Uncommon,  B::Point,     R::Flat),
    ("Moss Animal",      b(Primordial, SEA),                Common,    B::Discovery, R::Flat),
    ("Brittle Star",     b(Primordial, SEA),                Common,    B::Quick,     R::Flat),
    ("Carp",             t(n(FRESH, 2), 1, 2),              Common,    B::Cards,     R::Flat),
    ("Dickinsonia",      b(Primordial, SEA),                Rare,      B::Morph,     R::Flat),
    ("Dimetrodon",       t(n(LAND, 2), 3, 5),               Rare,      B::Quick,     R::Flat),
    ("Lucy",             b(Savanna, LAND),                  Epic,      B::Point,     R::Flat),
    ("Neanderthal",      b(IceAge, LAND),                   Epic,      B::Luck,      R::Flat),
    ("Pakicetus",        fur(t(n(SHORE, 0), 3, 5)),         Rare,      B::Discovery, R::Flat),
    ("Acanthostega",     b(CoalSwamp, FRESH),               Rare,      B::Discovery, R::Flat),
];

/// Every taxon has one (tested), so a miss is a content bug.
pub fn of(name: &str) -> Ecology {
    let (_, needs, tier, bonus, rule) = TABLE
        .iter()
        .find(|(n, ..)| *n == name)
        .unwrap_or_else(|| panic!("no ecology for taxon {name:?}"));
    Ecology {
        needs: *needs,
        tier: *tier,
        bonus: *bonus,
        rule: *rule,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::planet::{LEVEL_MAX, VOLCANISM_MAX};
    use crate::game::tree::Phylogeny;

    fn planets() -> Vec<Planet> {
        let mut out = Vec::new();
        for land in 0..=LEVEL_MAX {
            for vegetation in 0..=(land * 2).min(LEVEL_MAX) {
                for oxygen in 0..=LEVEL_MAX {
                    for temperature in 0..=LEVEL_MAX {
                        for volcanism in 0..=VOLCANISM_MAX {
                            out.push(Planet {
                                land,
                                vegetation,
                                oxygen,
                                temperature,
                                volcanism,
                            });
                        }
                    }
                }
            }
        }
        out
    }

    #[test]
    fn only_legendaries_have_a_catch() {
        let phy = Phylogeny::load();
        let mut with: Vec<&str> = phy
            .taxa
            .iter()
            .filter(|t| t.eco.rule.drawback().is_some())
            .map(|t| t.name)
            .collect();
        with.sort();
        assert_eq!(
            with,
            [
                "Archaeopteryx",
                "Coelacanth",
                "Dodo",
                "Human",
                "Megalodon",
                "Platypus",
                "T. rex",
                "Tuatara"
            ]
        );
        let dodo = of("Dodo");
        assert_eq!(dodo.describe(), "Better rarity odds x3");
        assert!(!dodo.rule.drawback().unwrap().contains("found"));
    }

    #[test]
    fn every_taxon_has_exactly_one_ecology() {
        let phy = Phylogeny::load();
        for t in &phy.taxa {
            let hits = TABLE.iter().filter(|(n, ..)| *n == t.name).count();
            assert_eq!(hits, 1, "{} has {hits} ecology rows", t.name);
        }
        assert_eq!(
            TABLE.len(),
            phy.len(),
            "ecology rows for taxa that don't exist"
        );
    }

    #[test]
    fn the_first_children_can_live_in_the_starting_sea() {
        let phy = Phylogeny::load();
        for oxygen in [1, 2] {
            let start = Planet {
                oxygen,
                ..Planet::default()
            };
            let ok = phy.taxa[Phylogeny::ROOT]
                .children
                .iter()
                .filter(|&&c| of(phy.taxa[c].name).needs.met_by(&start))
                .count();
            assert!(ok >= 3, "the first genome would be almost empty");
        }
    }

    #[test]
    fn every_taxon_can_live_somewhere() {
        let phy = Phylogeny::load();
        let all = planets();
        for t in &phy.taxa {
            let needs = of(t.name).needs;
            assert!(
                all.iter().any(|p| needs.met_by(p)),
                "{} can never be discovered",
                t.name
            );
        }
    }

    #[test]
    fn no_planet_suits_most_animals() {
        let phy = Phylogeny::load();
        let best = planets()
            .iter()
            .map(|p| {
                phy.taxa
                    .iter()
                    .filter(|t| of(t.name).needs.met_by(p))
                    .count()
            })
            .max()
            .unwrap();
        let share = best as f32 / phy.len() as f32;
        assert!(share <= 0.6, "one planet suits {best}/{}", phy.len());
    }

    #[test]
    fn the_biomes_rule_each_other_out() {
        for p in planets() {
            let b = Biome::of(&p);
            assert!(b.len() <= 2, "{p:?} is {b:?}");
        }
    }

    #[test]
    fn every_biome_has_animals_and_a_planet() {
        let phy = Phylogeny::load();
        let all = planets();
        for biome in Biome::ALL {
            let n = phy
                .taxa
                .iter()
                .filter(|t| of(t.name).needs.biome == Some(biome))
                .count();
            assert!(n >= 4, "{} has {n} animals", biome.name());
            assert!(all.iter().any(|p| biome.ranges().contains(p)));
        }
    }

    #[test]
    fn humans_and_crocodiles_want_different_worlds_from_wolves() {
        let wolf = of("Wolf").needs.ranges.temperature;
        let croc = of("Crocodile").needs.ranges.temperature;
        assert!(wolf.1 < croc.0, "opposite climates are the point");
    }

    #[test]
    fn missing_explains_the_first_unmet_need() {
        let sea = Planet::default();
        assert_eq!(
            of("Lungfish").needs.missing(&sea).as_deref(),
            Some("needs fresh water")
        );
        let cold_swamp = Planet {
            land: 2,
            vegetation: 2,
            oxygen: 2,
            temperature: 1,
            ..Planet::default()
        };
        assert_eq!(
            of("Lungfish").needs.missing(&cold_swamp).as_deref(),
            Some("too cold")
        );
        assert_eq!(of("Mammoth").needs.missing(&cold_swamp), None, "an ice age");
        assert_eq!(
            of("Lion").needs.missing(&cold_swamp).as_deref(),
            Some("Savanna only: needs more land")
        );
    }

    #[test]
    fn a_melanistic_coat_lives_at_any_temperature() {
        let ape = of("Ape").needs;
        // Too cold, too cold, fine, too hot (fur).
        for temperature in [0, 1, 3, 5] {
            let p = Planet {
                land: 3,
                vegetation: 3,
                temperature,
                ..Planet::default()
            };
            assert_eq!(temperature == 3, ape.missing(&p).is_none(), "{temperature}");
            assert!(ape.missing_with(&p, true).is_none(), "{temperature}");
        }
        // Only the temperature: a biome's other levers still hold.
        let lion = of("Lion").needs;
        let sea = Planet::default();
        assert!(lion.missing_with(&sea, true).is_some());
    }

    #[test]
    #[ignore]
    fn print_best_planets() {
        let phy = Phylogeny::load();
        let mut res: Vec<(usize, Planet)> = planets()
            .into_iter()
            .map(|p| {
                let n = phy
                    .taxa
                    .iter()
                    .filter(|t| of(t.name).needs.met_by(&p))
                    .count();
                (n, p)
            })
            .collect();
        res.sort_by_key(|r| std::cmp::Reverse(r.0));
        for (n, p) in res.iter().take(5) {
            println!("{n}/{} {p:?} {:?}", phy.len(), Biome::of(p));
        }
    }

    #[test]
    #[ignore]
    fn print_doc_table() {
        use crate::game::planet::Lever;
        let phy = Phylogeny::load();
        for (i, t) in phy.taxa.iter().enumerate() {
            let e = of(t.name);
            let hab: Vec<&str> = e.needs.habitats.iter().map(|h| h.name()).collect();
            let needs = match e.needs.biome {
                Some(b) => b.name().to_string(),
                None => {
                    let parts: Vec<String> = Lever::ALL
                        .iter()
                        .filter_map(|&l| {
                            let (lo, hi) = e.needs.ranges.get(l);
                            let (_, top) = Ranges::ANY.get(l);
                            let k = &l.name()[..1];
                            match (lo > 0, hi < top) {
                                (false, false) => None,
                                (true, false) => Some(format!("{k}≥{lo}")),
                                (false, true) => Some(format!("{k}≤{hi}")),
                                (true, true) => Some(format!("{lo}≤{k}≤{hi}")),
                            }
                        })
                        .collect();
                    if parts.is_empty() {
                        "–".into()
                    } else {
                        parts.join(", ")
                    }
                }
            };
            let mut keystone = e.describe();
            if let Some(d) = e.rule.drawback() {
                keystone = format!("{keystone}; but {}", d.to_lowercase());
            }
            println!(
                "| {i} | {} | {} | {needs} | {} | {keystone} |",
                t.name,
                hab.join(" or "),
                &e.tier.name()[..1]
            );
        }
    }
}
