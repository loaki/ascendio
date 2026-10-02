//! What each taxon needs from the planet, its tier and what it does as a
//! keystone, keyed by `Taxon::name`.
//!
//! A keystone has a `Bonus` (what it adds: luck, cards, morphs...) and a
//! `Rule` (when and how much). The rules lean on the planet, so the
//! keystones a player equips decide which world they shape.

use serde::{Deserialize, Serialize};

use crate::planet::{Biome, Habitat, Planet, Ranges};
use crate::tree::Group;

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

    /// Keystone bonus strength.
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

/// What a keystone adds; scaled by tier units x level x morph, then by its
/// `Rule`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bonus {
    Luck,
    Cards,
    Morph,
    Quick,
    /// +1 adjustment point (not scaled).
    Point,
    /// +1 vegetation per cycle (not scaled).
    Soil,
    /// Duplicates count double towards specimen levels.
    DoubleSpecimens,
    /// More finds in this habitat.
    Share(Habitat),
    /// Luck, and the Legendary pity window shrinks from 40 to 30.
    LivingFossil,
    /// Morphs, and a morph is guaranteed at least once every 7 genomes.
    Oddity,
}

impl Bonus {
    pub fn describe(self) -> String {
        match self {
            Bonus::Luck => "Better rarity odds".into(),
            Bonus::Cards => "Chance of +1 card".into(),
            Bonus::Morph => "More morphs".into(),
            Bonus::Quick => "Shorter wait".into(),
            Bonus::Point => "+1 adjustment point".into(),
            Bonus::Soil => "+1 vegetation each cycle".into(),
            Bonus::DoubleSpecimens => "Duplicates count double".into(),
            Bonus::Share(h) => format!("More {} finds", h.name().to_lowercase()),
            Bonus::LivingFossil => "Luck; legendary pity 30".into(),
            Bonus::Oddity => "More morphs; one every 7 genomes".into(),
        }
    }
}

/// A planet condition some rules wait for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cond {
    /// Snowball or Cold.
    Cold,
    /// Temperate or warmer.
    Warm,
    /// Oxygen at 35%.
    MaxOxygen,
    /// Volcanism 1 or more.
    Volcanic,
    /// Vegetation 1 or less.
    Bare,
    /// The planet has forest.
    Forest,
    /// Fresh water covers at least a fifth of the planet.
    FreshWater,
    In(Biome),
}

impl Cond {
    pub fn holds(self, p: &Planet) -> bool {
        match self {
            Cond::Cold => p.temperature <= 1,
            Cond::Warm => p.temperature >= 3,
            Cond::MaxOxygen => p.oxygen >= 5,
            Cond::Volcanic => p.volcanism >= 1,
            Cond::Bare => p.vegetation <= 1,
            Cond::Forest => p.has(Habitat::Forest),
            Cond::FreshWater => p.habitat_shares()[Habitat::Fresh.index()] >= 0.2,
            Cond::In(b) => b.ranges().contains(p),
        }
    }

    pub fn describe(self) -> String {
        match self {
            Cond::Cold => "at Snowball or Cold".into(),
            Cond::Warm => "at Temperate or warmer".into(),
            Cond::MaxOxygen => "at 35% oxygen".into(),
            Cond::Volcanic => "with volcanoes".into(),
            Cond::Bare => "on bare ground".into(),
            Cond::Forest => "with forest".into(),
            Cond::FreshWater => "with 20%+ fresh water".into(),
            Cond::In(b) => format!("in the {}", b.name()),
        }
    }
}

/// Keystones that work together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    /// Cow, Pig, Chicken, Horse.
    Farm,
    /// Cnidarian, Jellyfish, Coral.
    Cnidarians,
    /// Bee, Butterfly.
    Pollinators,
    /// Every fish.
    Fish,
}

impl Team {
    pub fn includes(self, name: &str, group: Group) -> bool {
        match self {
            Team::Farm => matches!(name, "Cow" | "Pig" | "Chicken" | "Horse"),
            Team::Cnidarians => matches!(name, "Cnidarian" | "Jellyfish" | "Coral"),
            Team::Pollinators => matches!(name, "Bee" | "Butterfly"),
            Team::Fish => group == Group::Fish,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Team::Farm => "farm animal",
            Team::Cnidarians => "cnidarian",
            Team::Pollinators => "pollinator",
            Team::Fish => "fish",
        }
    }
}

/// A legendary's strong effect and its drawback.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Catch {
    /// +2 cards; plant-eater keystones fall asleep.
    Tyrant,
    /// Doubles all luck; Temperature can't go below Temperate.
    Apex,
    /// x2 luck and +2 cards; 1 genome in 5 ends the Earth instead.
    Hubris,
    /// Morphs x3; one fewer boon to choose from.
    Feathers,
    /// Legendary pity 25; the wait is an hour longer.
    Ancient,
    /// Counts as every team; can't be copied.
    Wildcard,
}

impl Catch {
    pub fn perk(self) -> &'static str {
        match self {
            Catch::Tyrant => "+2 cards in every genome",
            Catch::Apex => "Doubles all Luck",
            Catch::Hubris => "x2 Luck and +2 cards",
            Catch::Feathers => "Morphs x3",
            Catch::Ancient => "A Legendary within 25 genomes",
            Catch::Wildcard => "Counts as every team; more morphs",
        }
    }

    pub fn drawback(self) -> &'static str {
        match self {
            Catch::Tyrant => "Plant-eater keystones fall asleep",
            Catch::Apex => "Temperature can't go below Temperate",
            Catch::Hubris => "1 genome in 5 ends the Earth",
            Catch::Feathers => "One fewer boon to choose from",
            Catch::Ancient => "The wait is an hour longer",
            Catch::Wildcard => "Can't be copied",
        }
    }
}

/// When and how much a keystone's `Bonus` pays.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rule {
    Flat,
    /// x2 while the condition holds.
    DoubleWhen(Cond),
    /// Nothing unless the condition holds.
    OnlyWhen(Cond),
    /// A charge for each cycle run with the condition holding (half the
    /// bonus each); one without it empties them.
    Grows(Cond),
    /// A charge for each cycle run on exactly the previous cycle's planet.
    Stasis,
    /// Half the bonus for each other keystone of the team.
    PerTeam(Team),
    /// x2 with a keystone of the team.
    DoubleWith(Team),
    /// x3, then leaves its slot after this many cycles.
    Fragile(u8),
    /// A charge per duplicate (up to 3), each a third of a card.
    Duplicates,
    /// Never asleep; +2 luck per lever at its lowest or highest.
    Extremes,
    CopyAbove,
    /// Copies the keystone below it if that one is a mammal.
    CopyBelowMammal,
    /// Copies the strongest other keystone at half strength.
    CopyStrongest,
    /// After a Rare+ card, the next card is luckier.
    AfterRare,
    /// After a new species, the next cards morph more.
    AfterNew,
    /// Each duplicate gives a random keystone +1 specimen.
    Feed,
    /// A genome with nothing new brings the Legendary pity 2 closer.
    NoNewPity,
    /// Giant morphs x3 while the condition holds.
    GiantWhen(Cond),
    Catch(Catch),
}

impl Rule {
    /// The rule in a line, `bonus` being what it pays.
    pub fn describe(self, bonus: Bonus) -> String {
        let b = bonus.describe();
        match self {
            Rule::Flat => b,
            Rule::DoubleWhen(c) => format!("{b}; x2 {}", c.describe()),
            Rule::OnlyWhen(c) => format!("{b}, only {}", c.describe()),
            Rule::Grows(c) => format!("{b}, growing each cycle {}", c.describe()),
            Rule::Stasis => format!("{b}, growing while the planet stays the same"),
            Rule::PerTeam(t) => format!("{b} for each other {} keystone", t.name()),
            Rule::DoubleWith(t) => format!("{b}; x2 with a {} keystone", t.name()),
            Rule::Fragile(n) => format!("{b} x3, then it leaves after {n} cycles"),
            Rule::Duplicates => "Each duplicate: +1/3 card, up to 1".into(),
            Rule::Extremes => "Never asleep; +2 Luck per lever at its min or max".into(),
            Rule::CopyAbove => "Copies the keystone above it".into(),
            Rule::CopyBelowMammal => "Copies the keystone below it, if a mammal".into(),
            Rule::CopyStrongest => "Copies the strongest keystone at half".into(),
            Rule::AfterRare => "After a Rare or better card, the next is luckier".into(),
            Rule::AfterNew => "After a new species, the next cards morph more".into(),
            Rule::Feed => "Each duplicate feeds a keystone a specimen".into(),
            Rule::NoNewPity => "Nothing new? A Legendary comes sooner".into(),
            Rule::GiantWhen(c) => format!("Giant morphs x3 {}", c.describe()),
            Rule::Catch(c) => c.perk().into(),
        }
    }
}

/// Plant eaters: asleep while a T. rex is a keystone.
pub fn is_herbivore(name: &str) -> bool {
    matches!(
        name,
        "Sauropod"
            | "Triceratops"
            | "Mammoth"
            | "Elephant"
            | "Giraffe"
            | "Horse"
            | "Cow"
            | "Rabbit"
            | "Koala"
            | "Sloth"
            | "Giant Panda"
            | "Rhino"
            | "Hippo"
            | "Kangaroo"
    )
}

#[derive(Clone, Copy, Debug)]
pub struct Needs {
    /// Any one of these habitats is enough.
    pub habitats: &'static [Habitat],
    pub ranges: Ranges,
    /// The biome the animal belongs to, if any; its ranges are in `ranges`.
    pub biome: Option<Biome>,
}

impl Needs {
    pub fn met_by(&self, p: &Planet) -> bool {
        self.missing(p).is_none()
    }

    /// The first unmet need, as a short hint ("needs fresh water").
    pub fn missing(&self, p: &Planet) -> Option<String> {
        self.missing_colder(p, 0)
    }

    /// As `missing`, living `colder` temperature steps below the usual.
    pub fn missing_colder(&self, p: &Planet, colder: u8) -> Option<String> {
        if !self.habitats.iter().any(|&h| p.has(h)) {
            let names: Vec<_> = self
                .habitats
                .iter()
                .map(|h| h.name().to_lowercase())
                .collect();
            return Some(format!("needs {}", names.join(" or ")));
        }
        let why = self.ranges.missing(p, colder)?;
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

/// Lives in `habitats` with at least `oxygen_min`, anywhere else.
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
/// Open-ocean swimmers: no more land than Coasts.
const fn ocean(mut needs: Needs) -> Needs {
    needs.ranges.land.1 = 2;
    needs
}
/// Mammals: no 35% oxygen (wildfires) and no Hothouse.
const fn fur(mut needs: Needs) -> Needs {
    needs.ranges.oxygen.1 = 4;
    if needs.ranges.temperature.1 > 4 {
        needs.ranges.temperature.1 = 4;
    }
    needs
}
/// Lives in `habitats`, only in `biome`.
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
    ("Urmetazoan",       n(SEA, 0),                         Common,    B::Luck,            R::Flat),
    ("Sea Sponge",       b(Primordial, SEA),                Common,    B::Luck,            R::Flat),
    ("Comb Jelly",       b(Primordial, SEA),                Common,    B::Share(Sea),      R::Flat),
    ("Placozoan",        b(Primordial, SEA),                Uncommon,  B::Morph,           R::Flat),
    ("Cnidarian",        n(SEA, 1),                         Common,    B::Share(Sea),      R::Flat),
    ("Jellyfish",        t(n(SEA, 1), 0, 2),                Common,    B::Morph,           R::Flat),
    ("Coral",            b(Biome::Reef, REEF),              Uncommon,  B::Luck,            R::Grows(Cond::Warm)),
    ("Bilaterian",       n(SEA, 2),                         Common,    B::Cards,           R::Flat),
    ("Acoel Worm",       b(Primordial, SEA),                Common,    B::Quick,           R::Flat),
    ("Protostome",       n(SEA, 2),                         Common,    B::Luck,            R::Flat),
    ("Deuterostome",     n(SEA, 2),                         Uncommon,  B::Luck,            R::Flat),
    ("Spiralian",        n(SEA, 2),                         Common,    B::Share(Shore),    R::Flat),
    ("Ecdysozoan",       n(SEA, 2),                         Common,    B::Morph,           R::Flat),
    ("Flatworm",         t(n(FRESH, 2), 1, 2),              Common,    B::Cards,           R::Flat),
    ("Mollusc",          n(SEA, 2),                         Uncommon,  B::Share(Reef),     R::Flat),
    ("Snail",            t(n(SHORE, 2), 3, 5),              Common,    B::Quick,           R::Flat),
    ("Octopus",          b(Biome::Reef, REEF),              Epic,      B::Luck,            R::CopyAbove),
    ("Segmented Worm",   v(n(LAND, 2), 1, 4),               Uncommon,  B::Soil,            R::Flat),
    ("Nematode",         n(SEA_OR_LAND, 1),                 Common,    B::Quick,           R::Flat),
    ("Arthropod",        n(SEA, 2),                         Uncommon,  B::Morph,           R::Flat),
    ("Chelicerate",      n(SHORE, 2),                       Uncommon,  B::Share(Land),     R::Flat),
    ("Horseshoe Crab",   t(n(SHORE, 2), 2, 4),              Rare,      B::Luck,            R::NoNewPity),
    ("Scorpion",         b(CoalSwamp, LAND),                Rare,      B::Share(Land),     R::Flat),
    ("Spider",           b(CoalSwamp, FOREST),              Rare,      B::Cards,           R::Feed),
    ("Pancrustacean",    n(SEA, 2),                         Common,    B::Share(Shore),    R::Flat),
    ("Crab",             t(n(SHORE, 2), 3, 5),              Uncommon,  B::Share(Shore),    R::Flat),
    ("Insect",           v(n(LAND, 3), 2, 5),               Uncommon,  B::Share(Forest),   R::Flat),
    ("Dragonfly",        b(CoalSwamp, FRESH),               Rare,      B::Morph,           R::GiantWhen(Cond::MaxOxygen)),
    ("Beetle",           b(CoalSwamp, FOREST),              Uncommon,  B::Cards,           R::Flat),
    ("Butterfly",        b(Jungle, FOREST),                 Rare,      B::Cards,           R::DoubleWith(Team::Pollinators)),
    ("Ant",              t(n(FOREST, 0), 3, 5),             Epic,      B::Cards,           R::OnlyWhen(Cond::MaxOxygen)),
    ("Echinoderm",       n(SEA, 2),                         Common,    B::Share(Reef),     R::Flat),
    ("Starfish",         n(SHORE, 2),                       Common,    B::Cards,           R::Duplicates),
    ("Sea Urchin",       b(Biome::Reef, REEF),              Uncommon,  B::Share(Reef),     R::Flat),
    ("Chordate",         n(SEA, 2),                         Uncommon,  B::Luck,            R::Flat),
    ("Sea Squirt",       t(n(SEA, 2), 0, 2),                Common,    B::Quick,           R::Flat),
    ("Vertebrate",       n(SEA, 3),                         Rare,      B::Luck,            R::Flat),
    ("Lamprey",          t(n(SEA_OR_FRESH, 2), 0, 2),       Uncommon,  B::Share(Fresh),    R::Flat),
    ("Jawed Fish",       n(SEA, 3),                         Uncommon,  B::Cards,           R::Flat),
    ("Shark",            ocean(n(SEA, 3)),                  Rare,      B::Luck,            R::PerTeam(Team::Fish)),
    ("Ray-finned Fish",  n(SEA_OR_FRESH, 2),                Common,    B::Cards,           R::Flat),
    ("Lobe-finned Fish", n(FRESH, 2),                       Rare,      B::Share(Fresh),    R::Flat),
    ("Coelacanth",       t(n(SEA, 2), 0, 2),                Legendary, B::Luck,            R::Stasis),
    ("Tetrapod",         v(n(SHORE_OR_FRESH, 2), 2, 5),     Epic,      B::Share(Land),     R::Flat),
    ("Amphibian",        v(t(n(FRESH, 0), 2, 5), 2, 5),     Uncommon,  B::Share(Fresh),    R::Flat),
    ("Frog",             b(Jungle, FRESH),                  Common,    B::Morph,           R::Flat),
    ("Amniote",          v(n(LAND, 0), 2, 5),               Rare,      B::Share(Land),     R::Flat),
    ("Mammal",           fur(v(t(n(LAND, 0), 1, 5), 2, 5)), Rare,      B::Luck,            R::Flat),
    ("Platypus",         t(n(FRESH, 0), 0, 2),              Legendary, B::Oddity,          R::Catch(Catch::Wildcard)),
    ("Marsupial",        fur(n(LAND, 0)),                   Uncommon,  B::Morph,           R::Flat),
    ("Placental",        fur(v(n(LAND, 0), 2, 5)),          Epic,      B::Cards,           R::Flat),
    ("Afrothere",        fur(t(n(LAND, 0), 3, 5)),          Uncommon,  B::Cards,           R::Flat),
    ("Mouse",            fur(n(LAND, 0)),                   Common,    B::Quick,           R::Flat),
    ("Bat",              fur(t(n(FOREST, 3), 2, 5)),        Rare,      B::Morph,           R::AfterNew),
    ("Carnivoran",       fur(n(LAND, 0)),                   Uncommon,  B::Luck,            R::Flat),
    ("Cetartiodactyl",   fur(n(LAND, 0)),                   Uncommon,  B::Share(Land),     R::Flat),
    ("Primate",          fur(t(n(FOREST, 0), 3, 5)),        Rare,      B::Share(Forest),   R::Flat),
    ("Lemur",            b(Jungle, FOREST),                 Uncommon,  B::Morph,           R::Flat),
    ("Monkey",           fur(t(n(FOREST, 0), 3, 5)),        Rare,      B::Cards,           R::Flat),
    ("Ape",              fur(t(n(FOREST, 0), 3, 5)),        Epic,      B::Luck,            R::Flat),
    ("Chimpanzee",       b(Jungle, FOREST),                 Epic,      B::Luck,            R::CopyBelowMammal),
    ("Human",            b(Savanna, LAND),                  Legendary, B::Luck,            R::Catch(Catch::Hubris)),
    ("Reptile",          t(n(LAND, 0), 3, 5),               Uncommon,  B::Share(Land),     R::Flat),
    ("Turtle",           t(n(SHORE, 0), 3, 5),              Uncommon,  B::DoubleSpecimens, R::Flat),
    ("Lizard",           v(t(n(LAND, 0), 3, 5), 0, 3),      Common,    B::Luck,            R::OnlyWhen(Cond::Bare)),
    ("Crocodile",        t(n(FRESH, 0), 4, 5),              Rare,      B::Cards,           R::OnlyWhen(Cond::FreshWater)),
    ("Dinosaur",         v(t(n(LAND, 0), 3, 5), 2, 5),      Epic,      B::Cards,           R::Flat),
    ("Bird",             n(FOREST, 3),                      Epic,      B::Luck,            R::Flat),
    ("Clam",             b(Biome::Reef, REEF),              Common,    B::DoubleSpecimens, R::Flat),
    ("Squid",            ocean(t(n(SEA, 3), 0, 2)),         Uncommon,  B::Cards,           R::Flat),
    ("Ammonite",         ocean(t(n(SEA, 2), 3, 5)),         Rare,      B::Luck,            R::Flat),
    ("Earthworm",        v(n(LAND, 2), 2, 5),               Common,    B::Soil,            R::Flat),
    ("Tardigrade",       n(SEA_OR_LAND, 0),                 Epic,      B::Luck,            R::Extremes),
    ("Trilobite",        b(Primordial, SEA),                Uncommon,  B::Share(Sea),      R::Flat),
    ("Anomalocaris",     b(Primordial, SEA),                Epic,      B::Luck,            R::Flat),
    ("Centipede",        b(CoalSwamp, FOREST),              Common,    B::Quick,           R::Flat),
    ("Shrimp",           ocean(n(SEA, 2)),                  Common,    B::Cards,           R::Flat),
    ("Bee",              t(v(n(FOREST, 3), 3, 5), 2, 5),    Uncommon,  B::Luck,            R::Grows(Cond::Forest)),
    ("Meganeura",        b(CoalSwamp, FRESH),               Epic,      B::Morph,           R::GiantWhen(Cond::MaxOxygen)),
    ("Dunkleosteus",     ocean(t(n(SEA, 3), 3, 5)),         Epic,      B::Luck,            R::Flat),
    ("Stingray",         ocean(t(n(SEA, 3), 3, 5)),         Uncommon,  B::Share(Sea),      R::Flat),
    ("Megalodon",        ocean(t(n(SEA, 3), 3, 5)),         Legendary, B::Luck,            R::Catch(Catch::Apex)),
    ("Seahorse",         b(Biome::Reef, REEF),              Rare,      B::Morph,           R::Flat),
    ("Anglerfish",       ocean(t(n(SEA, 2), 0, 2)),         Rare,      B::Luck,            R::Flat),
    ("Tiktaalik",        n(SHORE_OR_FRESH, 3),              Rare,      B::Share(Shore),    R::Flat),
    ("Salamander",       t(n(FRESH, 0), 1, 2),              Common,    B::Quick,           R::Flat),
    ("Snake",            t(n(LAND, 0), 3, 5),               Uncommon,  B::Luck,            R::Flat),
    ("Pterosaur",        b(Hothouse, SHORE),                Epic,      B::Share(Shore),    R::Flat),
    ("T. rex",           b(Hothouse, LAND),                 Legendary, B::Cards,           R::Catch(Catch::Tyrant)),
    ("Sauropod",         b(Hothouse, FOREST),               Epic,      B::Cards,           R::Grows(Cond::In(Hothouse))),
    ("Penguin",          t(n(SHORE, 0), 0, 2),              Rare,      B::Luck,            R::DoubleWhen(Cond::Cold)),
    ("Mammoth",          b(IceAge, LAND),                   Epic,      B::Cards,           R::Flat),
    ("Rabbit",           fur(v(n(LAND, 0), 1, 4)),          Common,    B::DoubleSpecimens, R::Flat),
    ("Horse",            v(t(n(LAND, 0), 1, 4), 1, 3),      Uncommon,  B::Luck,            R::PerTeam(Team::Farm)),
    ("Dolphin",          ocean(fur(t(n(SEA, 0), 3, 5))),    Rare,      B::Cards,           R::Flat),
    ("Lion",             b(Savanna, LAND),                  Rare,      B::Luck,            R::Flat),
    ("Gorilla",          b(Jungle, FOREST),                 Rare,      B::Point,           R::Flat),
    ("Nautilus",         b(Biome::Reef, REEF),              Rare,      B::LivingFossil,    R::Flat),
    ("Fly",              n(LAND, 2),                        Common,    B::Quick,           R::Flat),
    ("Clownfish",        b(Biome::Reef, REEF),              Common,    B::Share(Reef),     R::DoubleWith(Team::Cnidarians)),
    ("Lungfish",         t(n(FRESH, 2), 3, 5),              Rare,      B::Share(Fresh),    R::Flat),
    ("Pig",              fur(v(n(LAND, 0), 1, 5)),          Uncommon,  B::Luck,            R::PerTeam(Team::Farm)),
    ("Tuatara",          t(n(FOREST, 0), 1, 2),             Legendary, B::Luck,            R::Catch(Catch::Ancient)),
    ("Sea Turtle",       b(Biome::Reef, SEA),               Rare,      B::DoubleSpecimens, R::Flat),
    ("Plesiosaur",       b(Hothouse, SEA),                  Epic,      B::Cards,           R::Flat),
    ("Ichthyosaur",      b(Hothouse, SEA),                  Epic,      B::Luck,            R::Flat),
    ("Triceratops",      b(Hothouse, LAND),                 Epic,      B::Point,           R::Flat),
    ("Velociraptor",     b(Hothouse, LAND),                 Rare,      B::Quick,           R::Flat),
    ("Archaeopteryx",    t(n(FOREST, 3), 3, 5),             Legendary, B::Morph,           R::Catch(Catch::Feathers)),
    ("Ostrich",          b(Savanna, LAND),                  Rare,      B::Luck,            R::OnlyWhen(Cond::Bare)),
    ("Parrot",           b(Jungle, FOREST),                 Epic,      B::Luck,            R::CopyStrongest),
    ("Owl",              t(n(FOREST, 0), 0, 2),             Uncommon,  B::Luck,            R::AfterRare),
    ("Koala",            fur(t(n(FOREST, 0), 3, 5)),        Uncommon,  B::Soil,            R::Flat),
    ("Sloth",            b(Jungle, FOREST),                 Uncommon,  B::DoubleSpecimens, R::Flat),
    ("Giraffe",          b(Savanna, LAND),                  Rare,      B::Cards,           R::Flat),
    ("Hippo",            fur(t(n(FRESH, 0), 3, 5)),         Rare,      B::Cards,           R::OnlyWhen(Cond::FreshWater)),
    ("Rhino",            b(Savanna, LAND),                  Epic,      B::Luck,            R::Flat),
    ("Bear",             t(n(FOREST, 0), 0, 2),             Rare,      B::Point,           R::Flat),
    ("Seal",             b(IceAge, SHORE),                  Uncommon,  B::Share(Shore),    R::Flat),
    ("Synapsid",         n(LAND, 2),                        Uncommon,  B::Luck,            R::Flat),
    ("Archosaur",        t(n(LAND, 2), 3, 5),               Uncommon,  B::Share(Land),     R::Flat),
    ("Theropod",         t(n(LAND, 3), 3, 5),               Rare,      B::Cards,           R::Flat),
    ("Hominin",          fur(v(t(n(LAND, 3), 2, 4), 1, 5)), Epic,      B::Luck,            R::Flat),
    ("Perissodactyl",    fur(n(LAND, 0)),                   Uncommon,  B::Quick,           R::Flat),
    ("Spiny-rayed Fish", t(n(SEA, 2), 3, 5),                Common,    B::Cards,           R::Flat),
    ("Elephant",         b(Savanna, LAND),                  Epic,      B::Point,           R::Flat),
    ("Whale",            t(n(SEA, 0), 0, 2),                Epic,      B::Share(Sea),      R::Flat),
    ("Wolf",             t(n(LAND, 0), 0, 2),               Rare,      B::Luck,            R::DoubleWhen(Cond::Cold)),
    ("Dog",              fur(n(LAND, 0)),                   Uncommon,  B::Luck,            R::DoubleWith(Team::Farm)),
    ("Kangaroo",         b(Savanna, LAND),                  Rare,      B::Share(Land),     R::Flat),
    ("Tiger",            fur(t(n(FOREST, 0), 0, 2)),        Epic,      B::Luck,            R::Flat),
    ("Cat",              fur(n(LAND, 0)),                   Common,    B::Morph,           R::Flat),
    ("Sabre-tooth",      b(IceAge, LAND),                   Epic,      B::Cards,           R::Flat),
    ("Giant Panda",      v(t(n(FOREST, 0), 1, 2), 3, 5),    Epic,      B::DoubleSpecimens, R::Flat),
    ("Cow",              fur(v(n(LAND, 0), 1, 4)),          Common,    B::Luck,            R::PerTeam(Team::Farm)),
    ("Chicken",          fur(n(LAND, 0)),                   Common,    B::Luck,            R::PerTeam(Team::Farm)),
    ("Songbird",         fur(n(FOREST, 0)),                 Common,    B::Quick,           R::Flat),
    ("Dodo",             b(Jungle, FOREST),                 Legendary, B::Luck,            R::Fragile(3)),
    ("True Bug",         t(n(LAND, 2), 3, 5),               Common,    B::Quick,           R::Flat),
    ("Wasp",             v(t(n(LAND, 3), 3, 5), 1, 5),      Uncommon,  B::Luck,            R::Flat),
    ("Mite",             n(SEA_OR_LAND, 1),                 Common,    B::DoubleSpecimens, R::Flat),
    ("Termite",          b(Jungle, FOREST),                 Uncommon,  B::Soil,            R::Flat),
    ("Moss Animal",      b(Primordial, SEA),                Common,    B::Share(Sea),      R::Flat),
    ("Brittle Star",     b(Primordial, SEA),                Common,    B::Quick,           R::Flat),
    ("Carp",             t(n(FRESH, 2), 1, 2),              Common,    B::DoubleSpecimens, R::Flat),
    ("Dickinsonia",      b(Primordial, SEA),                Rare,      B::Oddity,          R::Flat),
    ("Dimetrodon",       t(n(LAND, 2), 3, 5),               Rare,      B::Quick,           R::OnlyWhen(Cond::Volcanic)),
    ("Lucy",             b(Savanna, LAND),                  Epic,      B::Point,           R::Flat),
    ("Neanderthal",      b(IceAge, LAND),                   Epic,      B::Luck,            R::Flat),
    ("Pakicetus",        fur(t(n(SHORE, 0), 3, 5)),         Rare,      B::Share(Shore),    R::Flat),
    ("Acanthostega",     b(CoalSwamp, FRESH),               Rare,      B::Share(Fresh),    R::Flat),
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
    use crate::planet::{LEVEL_MAX, VOLCANISM_MAX};
    use crate::tree::Phylogeny;

    /// Every planet the levers can make.
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

    /// The point of biomes: no planet holds most of the tree.
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
    fn a_melanistic_coat_lives_colder() {
        let cool = Planet {
            land: 3,
            vegetation: 3,
            temperature: 1,
            ..Planet::default()
        };
        let ape = of("Ape").needs;
        assert!(ape.missing(&cool).is_some());
        assert!(ape.missing_colder(&cool, 2).is_none());
    }

    /// `cargo test print_best_planets -- --ignored --nocapture`: the planets
    /// that suit the most animals, for balancing the table.
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

    /// `cargo test print_doc_table -- --ignored --nocapture`: the taxa
    /// table in docs/DESIGN.md.
    #[test]
    #[ignore]
    fn print_doc_table() {
        use crate::planet::Lever;
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
            if let Rule::Catch(c) = e.rule {
                keystone = format!("{keystone}; but {}", c.drawback().to_lowercase());
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
