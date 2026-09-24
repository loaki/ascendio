//! What each taxon needs from the planet, its tier and its keystone bonus,
//! keyed by `Taxon::name`.

use serde::{Deserialize, Serialize};

use crate::planet::{Habitat, Planet};

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

/// A keystone's effect; scaled by tier units x level x morph.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bonus {
    Affinity(Habitat),
    Luck,
    ExtraCard,
    Morph,
    Quick,
    Point,
    /// +1 vegetation per cycle.
    Soil,
    /// Duplicates count double towards specimen levels.
    DoubleSpecimens,
    /// Luck, and the Legendary pity window shrinks from 40 to 30.
    LivingFossil,
    /// Morph, and a morph is guaranteed at least once every 7 cycles.
    Oddity,
    /// Point and Luck together.
    Mind,
}

impl Bonus {
    pub fn describe(self) -> String {
        match self {
            Bonus::Affinity(h) => format!("More {} finds", h.name().to_lowercase()),
            Bonus::Luck => "Better rarity odds".into(),
            Bonus::ExtraCard => "Chance of +1 card".into(),
            Bonus::Morph => "More morphs".into(),
            Bonus::Quick => "Shorter wait".into(),
            Bonus::Point => "+1 adjustment point".into(),
            Bonus::Soil => "+1 vegetation each cycle".into(),
            Bonus::DoubleSpecimens => "Duplicates count double".into(),
            Bonus::LivingFossil => "Luck; legendary pity 30".into(),
            Bonus::Oddity => "More morphs; one every 7 genomes".into(),
            Bonus::Mind => "+1 point and better odds".into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Needs {
    /// Any one of these habitats is enough.
    pub habitats: &'static [Habitat],
    pub oxygen_min: u8,
    pub temp_min: u8,
    pub temp_max: u8,
    pub veg_min: u8,
    pub veg_max: u8,
}

impl Needs {
    pub fn met_by(&self, p: &Planet) -> bool {
        self.habitats.iter().any(|&h| p.has(h))
            && p.oxygen >= self.oxygen_min
            && (self.temp_min..=self.temp_max).contains(&p.temperature)
            && (self.veg_min..=self.veg_max).contains(&p.vegetation)
    }

    /// The first unmet need, as a short hint ("needs fresh water").
    pub fn missing(&self, p: &Planet) -> Option<String> {
        if !self.habitats.iter().any(|&h| p.has(h)) {
            let names: Vec<_> = self
                .habitats
                .iter()
                .map(|h| h.name().to_lowercase())
                .collect();
            return Some(format!("needs {}", names.join(" or ")));
        }
        if p.oxygen < self.oxygen_min {
            return Some("needs more oxygen".into());
        }
        if p.temperature < self.temp_min {
            return Some("too cold".into());
        }
        if p.temperature > self.temp_max {
            return Some("too hot".into());
        }
        if p.vegetation < self.veg_min {
            return Some("needs more vegetation".into());
        }
        if p.vegetation > self.veg_max {
            return Some("needs open ground".into());
        }
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ecology {
    pub needs: Needs,
    pub tier: Tier,
    pub bonus: Bonus,
}

use Habitat::*;
use Tier::*;

const fn n(habitats: &'static [Habitat], oxygen_min: u8) -> Needs {
    Needs {
        habitats,
        oxygen_min,
        temp_min: 0,
        temp_max: 5,
        veg_min: 0,
        veg_max: 5,
    }
}
const fn t(mut needs: Needs, temp_min: u8, temp_max: u8) -> Needs {
    needs.temp_min = temp_min;
    needs.temp_max = temp_max;
    needs
}
const fn v(mut needs: Needs, veg_min: u8, veg_max: u8) -> Needs {
    needs.veg_min = veg_min;
    needs.veg_max = veg_max;
    needs
}

const SEA: &[Habitat] = &[Sea];
const REEF: &[Habitat] = &[Reef];
const SHORE: &[Habitat] = &[Shore];
const FRESH: &[Habitat] = &[Fresh];
const LAND: &[Habitat] = &[Land];
const FOREST: &[Habitat] = &[Forest];
const SEA_OR_LAND: &[Habitat] = &[Sea, Land];
const SEA_OR_FRESH: &[Habitat] = &[Sea, Fresh];

#[rustfmt::skip]
const TABLE: &[(&str, Needs, Tier, Bonus)] = &[
    ("Urmetazoan",       n(SEA, 0),                    Common,    Bonus::Luck),
    ("Sea Sponge",       n(SEA, 1),                    Common,    Bonus::Luck),
    ("Comb Jelly",       n(SEA, 1),                    Common,    Bonus::Affinity(Sea)),
    ("Placozoan",        n(SEA, 1),                    Uncommon,  Bonus::Morph),
    ("Cnidarian",        n(SEA, 1),                    Common,    Bonus::Affinity(Sea)),
    ("Jellyfish",        n(SEA, 1),                    Common,    Bonus::Morph),
    ("Coral",            n(REEF, 2),                   Uncommon,  Bonus::Affinity(Reef)),
    ("Bilaterian",       n(SEA, 2),                    Common,    Bonus::ExtraCard),
    ("Acoel Worm",       n(SEA, 1),                    Common,    Bonus::Quick),
    ("Protostome",       n(SEA, 2),                    Common,    Bonus::Luck),
    ("Deuterostome",     n(SEA, 2),                    Uncommon,  Bonus::Luck),
    ("Spiralian",        n(SEA, 2),                    Common,    Bonus::Affinity(Shore)),
    ("Ecdysozoan",       n(SEA, 2),                    Common,    Bonus::Morph),
    ("Flatworm",         n(FRESH, 2),                  Common,    Bonus::ExtraCard),
    ("Mollusc",          n(SEA, 2),                    Uncommon,  Bonus::Affinity(Reef)),
    ("Snail",            n(SHORE, 2),                  Common,    Bonus::Quick),
    ("Octopus",          n(REEF, 3),                   Epic,      Bonus::ExtraCard),
    ("Segmented Worm",   n(LAND, 2),                   Uncommon,  Bonus::Soil),
    ("Nematode",         n(SEA_OR_LAND, 1),            Common,    Bonus::Quick),
    ("Arthropod",        n(SEA, 2),                    Uncommon,  Bonus::Morph),
    ("Chelicerate",      n(SHORE, 2),                  Uncommon,  Bonus::Affinity(Land)),
    ("Horseshoe Crab",   n(SHORE, 2),                  Rare,      Bonus::Luck),
    ("Scorpion",         t(n(LAND, 3), 3, 5),          Rare,      Bonus::Affinity(Land)),
    ("Spider",           n(FOREST, 3),                 Rare,      Bonus::ExtraCard),
    ("Pancrustacean",    n(SEA, 2),                    Common,    Bonus::Affinity(Shore)),
    ("Crab",             n(SHORE, 2),                  Uncommon,  Bonus::Affinity(Shore)),
    ("Insect",           v(n(LAND, 3), 2, 5),          Uncommon,  Bonus::Affinity(Forest)),
    ("Dragonfly",        n(FRESH, 4),                  Rare,      Bonus::Morph),
    ("Beetle",           n(FOREST, 3),                 Uncommon,  Bonus::ExtraCard),
    ("Butterfly",        t(n(FOREST, 3), 3, 5),        Rare,      Bonus::Soil),
    ("Ant",              t(n(FOREST, 0), 3, 5),        Epic,      Bonus::Point),
    ("Echinoderm",       n(SEA, 2),                    Common,    Bonus::Affinity(Reef)),
    ("Starfish",         n(SHORE, 2),                  Common,    Bonus::DoubleSpecimens),
    ("Sea Urchin",       n(REEF, 2),                   Uncommon,  Bonus::Affinity(Reef)),
    ("Chordate",         n(SEA, 2),                    Uncommon,  Bonus::Luck),
    ("Sea Squirt",       n(SEA, 2),                    Common,    Bonus::Quick),
    ("Vertebrate",       n(SEA, 3),                    Rare,      Bonus::Luck),
    ("Lamprey",          n(SEA_OR_FRESH, 2),           Uncommon,  Bonus::Affinity(Fresh)),
    ("Jawed Fish",       n(SEA, 3),                    Uncommon,  Bonus::ExtraCard),
    ("Shark",            n(SEA, 3),                    Rare,      Bonus::Luck),
    ("Ray-finned Fish",  n(SEA_OR_FRESH, 2),           Common,    Bonus::ExtraCard),
    ("Lobe-finned Fish", n(FRESH, 2),                  Rare,      Bonus::Affinity(Fresh)),
    ("Coelacanth",       t(n(SEA, 2), 0, 3),           Legendary, Bonus::LivingFossil),
    ("Tetrapod",         v(n(&[Shore, Fresh], 2), 2, 5), Epic,    Bonus::Affinity(Land)),
    ("Amphibian",        v(t(n(FRESH, 0), 2, 5), 2, 5), Uncommon, Bonus::Affinity(Fresh)),
    ("Frog",             t(n(FRESH, 0), 3, 5),         Common,    Bonus::Morph),
    ("Amniote",          v(n(LAND, 0), 2, 5),          Rare,      Bonus::Affinity(Land)),
    ("Mammal",           v(t(n(LAND, 0), 1, 5), 2, 5), Rare,      Bonus::Luck),
    ("Platypus",         t(n(FRESH, 0), 0, 3),         Legendary, Bonus::Oddity),
    ("Marsupial",       n(LAND, 0),                     Uncommon,  Bonus::Morph),
    ("Placental",        v(n(LAND, 0), 2, 5),          Epic,      Bonus::ExtraCard),
    ("Afrothere",       n(LAND, 0),                     Uncommon,  Bonus::ExtraCard),
    ("Mouse",            n(LAND, 0),                   Common,    Bonus::Quick),
    ("Bat",              n(FOREST, 3),                 Rare,      Bonus::ExtraCard),
    ("Carnivoran",      n(LAND, 0),                     Uncommon,  Bonus::Luck),
    ("Cetartiodactyl",  n(LAND, 0),                     Uncommon,  Bonus::Affinity(Land)),
    ("Primate",          t(n(FOREST, 0), 3, 5),        Rare,      Bonus::Affinity(Forest)),
    ("Lemur",            t(n(FOREST, 0), 3, 5),        Uncommon,  Bonus::Morph),
    ("Monkey",           t(n(FOREST, 0), 3, 5),        Rare,      Bonus::ExtraCard),
    ("Ape",              t(n(FOREST, 0), 3, 5),        Epic,      Bonus::Luck),
    ("Chimpanzee",       t(n(FOREST, 0), 3, 5),        Epic,      Bonus::ExtraCard),
    ("Human",            v(t(n(LAND, 3), 2, 4), 2, 5), Legendary, Bonus::Mind),
    ("Reptile",          t(n(LAND, 0), 2, 5),          Uncommon,  Bonus::Affinity(Land)),
    ("Turtle",           t(n(SHORE, 0), 2, 5),         Uncommon,  Bonus::DoubleSpecimens),
    ("Lizard",           v(t(n(LAND, 0), 3, 5), 0, 3), Common,    Bonus::Quick),
    ("Crocodile",        t(n(FRESH, 0), 4, 5),         Rare,      Bonus::Affinity(Fresh)),
    ("Dinosaur",         v(t(n(LAND, 0), 3, 5), 3, 5), Epic,      Bonus::ExtraCard),
    ("Bird",             n(FOREST, 3),                 Epic,      Bonus::Luck),
    ("Clam",             n(REEF, 2),                   Common,    Bonus::DoubleSpecimens),
    ("Squid",            n(SEA, 3),                    Uncommon,  Bonus::ExtraCard),
    ("Ammonite",         t(n(SEA, 2), 2, 5),           Rare,      Bonus::Luck),
    ("Earthworm",        v(n(LAND, 2), 2, 5),          Common,    Bonus::Soil),
    ("Tardigrade",       n(SEA_OR_LAND, 0),            Epic,      Bonus::Oddity),
    ("Trilobite",        n(SEA, 2),                    Uncommon,  Bonus::Affinity(Sea)),
    ("Anomalocaris",     n(SEA, 2),                    Epic,      Bonus::Luck),
    ("Centipede",        n(FOREST, 3),                 Common,    Bonus::Quick),
    ("Shrimp",           n(SEA, 2),                    Common,    Bonus::ExtraCard),
    ("Bee",              t(v(n(FOREST, 3), 3, 5), 2, 5), Uncommon, Bonus::Soil),
    ("Meganeura",        n(FRESH, 5),                  Epic,      Bonus::Morph),
    ("Dunkleosteus",     n(SEA, 3),                    Epic,      Bonus::Luck),
    ("Stingray",         n(SEA, 3),                    Uncommon,  Bonus::Affinity(Sea)),
    ("Megalodon",        t(n(SEA, 3), 3, 5),           Legendary, Bonus::ExtraCard),
    ("Seahorse",         t(n(REEF, 3), 3, 5),          Rare,      Bonus::Morph),
    ("Anglerfish",       n(SEA, 2),                    Rare,      Bonus::Luck),
    ("Tiktaalik",        n(&[Shore, Fresh], 3),        Rare,      Bonus::Affinity(Shore)),
    ("Salamander",       t(n(FRESH, 0), 1, 4),         Common,    Bonus::Quick),
    ("Snake",            t(n(LAND, 0), 3, 5),          Uncommon,  Bonus::Luck),
    ("Pterosaur",        t(n(SHORE, 3), 3, 5),         Epic,      Bonus::Affinity(Shore)),
    ("T. rex",           v(t(n(LAND, 3), 3, 5), 2, 5), Legendary, Bonus::Luck),
    ("Sauropod",         v(t(n(FOREST, 3), 3, 5), 3, 5), Epic,    Bonus::DoubleSpecimens),
    ("Penguin",          t(n(SHORE, 0), 0, 2),         Rare,      Bonus::Affinity(Shore)),
    ("Mammoth",          t(n(LAND, 0), 0, 1),          Epic,      Bonus::LivingFossil),
    ("Rabbit",           v(n(LAND, 0), 1, 4),          Common,    Bonus::DoubleSpecimens),
    ("Horse",            v(t(n(LAND, 0), 1, 4), 1, 3), Uncommon,  Bonus::Quick),
    ("Dolphin",          t(n(SEA, 0), 2, 5),           Rare,      Bonus::ExtraCard),
    ("Lion",             v(t(n(LAND, 0), 3, 5), 1, 3), Rare,      Bonus::Luck),
    ("Gorilla",          t(n(FOREST, 0), 3, 5),        Rare,      Bonus::Point),
    ("Nautilus",        t(n(REEF, 2), 2, 5),            Rare,      Bonus::LivingFossil),
    ("Fly",             n(LAND, 2),                     Common,    Bonus::Quick),
    ("Clownfish",       t(n(REEF, 2), 3, 5),            Common,    Bonus::Affinity(Reef)),
    ("Lungfish",        t(n(FRESH, 2), 3, 5),           Rare,      Bonus::Affinity(Fresh)),
    ("Pig",             v(n(LAND, 0), 1, 5),            Uncommon,  Bonus::DoubleSpecimens),
    ("Tuatara",         t(n(FOREST, 0), 1, 3),          Legendary, Bonus::LivingFossil),
    ("Sea Turtle",      t(n(SEA, 0), 2, 5),             Rare,      Bonus::DoubleSpecimens),
    ("Plesiosaur",      t(n(SEA, 3), 2, 5),             Epic,      Bonus::ExtraCard),
    ("Ichthyosaur",     n(SEA, 3),                      Epic,      Bonus::Luck),
    ("Triceratops",     v(t(n(LAND, 3), 3, 5), 2, 5),   Epic,      Bonus::Point),
    ("Velociraptor",    v(t(n(LAND, 3), 3, 5), 0, 3),   Rare,      Bonus::Quick),
    ("Archaeopteryx",   t(n(FOREST, 3), 3, 5),          Legendary, Bonus::Luck),
    ("Ostrich",         v(t(n(LAND, 0), 3, 5), 0, 2),   Rare,      Bonus::Quick),
    ("Parrot",          t(n(FOREST, 0), 3, 5),          Epic,      Bonus::Point),
    ("Owl",             n(FOREST, 0),                   Uncommon,  Bonus::Luck),
    ("Koala",           t(n(FOREST, 0), 3, 5),          Uncommon,  Bonus::Soil),
    ("Sloth",           t(n(FOREST, 0), 3, 5),          Uncommon,  Bonus::DoubleSpecimens),
    ("Giraffe",         v(t(n(LAND, 0), 3, 5), 1, 3),   Rare,      Bonus::ExtraCard),
    ("Hippo",           t(n(FRESH, 0), 3, 5),           Rare,      Bonus::Affinity(Fresh)),
    ("Rhino",           v(t(n(LAND, 0), 2, 5), 1, 3),   Epic,      Bonus::Luck),
    ("Bear",            t(n(FOREST, 0), 0, 3),          Rare,      Bonus::Point),
    ("Seal",            t(n(SHORE, 0), 0, 2),           Uncommon,  Bonus::Affinity(Shore)),
    ("Synapsid",        n(LAND, 2),                     Uncommon,  Bonus::Luck),
    ("Archosaur",       n(LAND, 2),                     Uncommon,  Bonus::Affinity(Land)),
    ("Theropod",        t(n(LAND, 3), 3, 5),            Rare,      Bonus::ExtraCard),
    ("Hominin",         v(t(n(LAND, 3), 2, 4), 1, 5),   Epic,      Bonus::Luck),
    ("Perissodactyl",   n(LAND, 0),                     Uncommon,  Bonus::Quick),
    ("Spiny-rayed Fish",n(SEA, 2),                      Common,    Bonus::ExtraCard),
    ("Elephant",        v(t(n(LAND, 0), 3, 5), 3, 5),   Epic,      Bonus::Point),
    ("Whale",           n(SEA, 0),                      Epic,      Bonus::Affinity(Sea)),
    ("Wolf",            t(n(LAND, 0), 0, 3),            Rare,      Bonus::Luck),
    ("Dog",             n(LAND, 0),                     Uncommon,  Bonus::Luck),
    ("Kangaroo",        v(t(n(LAND, 0), 3, 5), 0, 3),   Rare,      Bonus::Affinity(Land)),
    ("Tiger",           t(n(FOREST, 0), 1, 5),          Epic,      Bonus::Luck),
    ("Cat",             n(LAND, 0),                     Common,    Bonus::Morph),
    ("Sabre-tooth",     t(n(LAND, 0), 0, 3),            Epic,      Bonus::ExtraCard),
    ("Giant Panda",     v(t(n(FOREST, 0), 1, 4), 3, 5), Epic,      Bonus::DoubleSpecimens),
    ("Cow",             v(n(LAND, 0), 1, 4),            Common,    Bonus::Soil),
    ("Chicken",         n(LAND, 0),                     Common,    Bonus::ExtraCard),
    ("Songbird",        n(FOREST, 0),                   Common,    Bonus::Quick),
    ("Dodo",            t(n(FOREST, 0), 3, 5),          Legendary, Bonus::Oddity),
    ("True Bug",        n(LAND, 2),                     Common,    Bonus::Quick),
    ("Wasp",            v(n(LAND, 3), 1, 5),            Uncommon,  Bonus::Luck),
    ("Mite",            n(SEA_OR_LAND, 1),              Common,    Bonus::DoubleSpecimens),
    ("Termite",         t(v(n(FOREST, 2), 2, 5), 3, 5), Uncommon,  Bonus::Soil),
    ("Moss Animal",     n(SEA, 1),                      Common,    Bonus::Affinity(Reef)),
    ("Brittle Star",    n(SEA, 1),                      Common,    Bonus::Quick),
    ("Carp",            n(FRESH, 2),                    Common,    Bonus::DoubleSpecimens),
    ("Dickinsonia",     n(SEA, 0),                      Rare,      Bonus::Oddity),
    ("Dimetrodon",      t(n(LAND, 2), 3, 5),            Rare,      Bonus::Luck),
    ("Lucy",            v(t(n(LAND, 3), 3, 5), 1, 4),   Epic,      Bonus::Point),
    ("Neanderthal",     t(n(LAND, 3), 0, 3),            Epic,      Bonus::Luck),
    ("Pakicetus",       n(SHORE, 0),                    Rare,      Bonus::Affinity(Shore)),
    ("Acanthostega",    t(n(FRESH, 2), 3, 5),           Rare,      Bonus::Affinity(Fresh)),
];

/// Every taxon has one (tested), so a miss is a content bug.
pub fn of(name: &str) -> Ecology {
    let (_, needs, tier, bonus) = TABLE
        .iter()
        .find(|(n, ..)| *n == name)
        .unwrap_or_else(|| panic!("no ecology for taxon {name:?}"));
    Ecology {
        needs: *needs,
        tier: *tier,
        bonus: *bonus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Phylogeny;

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
        let start = Planet {
            oxygen: 2,
            ..Planet::default()
        };
        let ok = phy.taxa[Phylogeny::ROOT]
            .children
            .iter()
            .filter(|&&c| of(phy.taxa[c].name).needs.met_by(&start))
            .count();
        assert!(ok >= 3, "the first genome would be almost empty");
    }

    #[test]
    fn every_taxon_can_live_somewhere() {
        let phy = Phylogeny::load();
        for t in &phy.taxa {
            let needs = of(t.name).needs;
            let mut possible = false;
            for land in 0..=5 {
                for veg in 0..=(land * 2).min(5) {
                    for oxygen in 0..=5 {
                        for temperature in 0..=5 {
                            let p = Planet {
                                land,
                                vegetation: veg,
                                oxygen,
                                temperature,
                                volcanism: 0,
                            };
                            possible |= needs.met_by(&p);
                        }
                    }
                }
            }
            assert!(possible, "{} can never be discovered", t.name);
        }
    }

    #[test]
    fn humans_and_crocodiles_want_different_worlds_from_wolves() {
        let wolf = of("Wolf").needs;
        let croc = of("Crocodile").needs;
        assert!(
            wolf.temp_max < croc.temp_min,
            "opposite climates are the point"
        );
    }

    #[test]
    fn missing_explains_the_first_unmet_need() {
        let sea = Planet::default();
        assert_eq!(
            of("Frog").needs.missing(&sea).as_deref(),
            Some("needs fresh water")
        );
        let cold_swamp = Planet {
            land: 2,
            vegetation: 2,
            temperature: 1,
            ..Planet::default()
        };
        assert_eq!(
            of("Frog").needs.missing(&cold_swamp).as_deref(),
            Some("too cold")
        );
    }
}
