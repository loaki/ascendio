//! Upgrades: what a `Group::Backbone` taxon offers.
//!
//! A category node -- Mollusca, Arthropoda, Chordata, and the rest of the
//! internal, "common ancestor" nodes in the tree -- doesn't produce DNA
//! itself (see `game::output`). Discovering one is a deck-builder-style
//! choice instead of a fixed reward: three random `Kind`s are offered
//! (`Game::pending_choice`), and picking one is what activates that
//! `Kind` -- at level 1, free -- see `Game::choose_upgrade_kind`. The level
//! belongs to the `Kind` itself, not to the taxon that unlocked it: there
//! are only ever `NUM_KINDS` upgrades to buy, no matter how many categories
//! express the same one. Discovering a category whose flavour turns out to
//! offer an already-active `Kind` still marks that taxon with it (so its
//! species colour correctly and `nearest_category_kind` still resolves),
//! but doesn't add a second, duplicate entry to buy -- see
//! `Game::choose_upgrade_kind`. Buying further levels
//! (`Game::buy_upgrade_level`) is what costs DNA, and there is no ceiling on
//! how many times you can. Every one of the 24 `Backbone` taxa still has a
//! flavour entry here (`UPGRADES`), so no category discovery is ever a dead
//! end -- it names the real trait the choice is dressed as on the choice
//! screen, but no longer fixes what it mechanically does, and no longer
//! gets its own row in the upgrades tab.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Kind {
    /// Adds flat DNA/s.
    Rate,
    /// Lowers the DNA cost of buying every upgrade's next level.
    Cost,
    /// Adds flat percentage points to every attempt's success chance.
    Chance,
    /// Adds flat DNA to the pool's cap.
    Cap,
    /// Adds flat DNA to how far the pool can overflow past its normal cap --
    /// Evolve is still ready the moment it reaches the cap, same as ever,
    /// but time spent above that (typically while away) goes into a
    /// stronger single Evolve instead of idling at the cap wasted. See
    /// `Game::overflow_cap`.
    Storage,
    /// Adds flat fill to every tap on a species' own level bar
    /// (`Game::tap_level`).
    Tap,
    /// Adds bonus starting levels to every species newly discovered from
    /// here on -- it never touches what is already found, only what
    /// `Game::evolve` turns up next. See `Game::unlock_taxon`.
    Vigor,
}

/// Every `Kind` there is, in a fixed order -- both what a category
/// discovery draws 3 random candidates from (`Game::random_kind_options`)
/// and the fixed row order the upgrades tab lists them in.
pub const ALL_KINDS: [Kind; 7] = [
    Kind::Rate,
    Kind::Cost,
    Kind::Chance,
    Kind::Cap,
    Kind::Storage,
    Kind::Tap,
    Kind::Vigor,
];

pub const NUM_KINDS: usize = ALL_KINDS.len();

impl Kind {
    /// This `Kind`'s slot in a `[T; NUM_KINDS]` array (`Game::kind_level`,
    /// `Effects`' internals) -- `ALL_KINDS`' order.
    pub fn index(self) -> usize {
        match self {
            Kind::Rate => 0,
            Kind::Cost => 1,
            Kind::Chance => 2,
            Kind::Cap => 3,
            Kind::Storage => 4,
            Kind::Tap => 5,
            Kind::Vigor => 6,
        }
    }

    /// Magnitude per level, in this kind's own natural unit -- DNA/s for
    /// `Rate`, DNA for `Cost`/`Cap`/`Storage`, percentage points for
    /// `Chance`, whole levels for `Vigor`. Every kind is flat: level `n` is
    /// worth exactly `n × per_level`, never a compounding multiplier -- see
    /// the module doc. First-guess numbers, like every other constant here;
    /// needs playtesting.
    pub fn per_level(self) -> f64 {
        match self {
            Kind::Rate => 0.5,
            Kind::Cost => 5.0,
            Kind::Chance => 0.04,
            Kind::Cap => 25.0,
            Kind::Storage => 40.0,
            Kind::Tap => 1.0,
            Kind::Vigor => 1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Rate => "DNA",
            Kind::Cost => "upgrade cost",
            // Explicit rather than the bare "chance" it used to say --
            // there's no other kind of chance in the game to confuse it
            // with, but "evolve chance" reads unambiguously at a glance.
            Kind::Chance => "evolve chance",
            Kind::Cap => "DNA cap",
            Kind::Storage => "DNA overflow",
            Kind::Tap => "tap speed",
            Kind::Vigor => "starting level",
        }
    }

    /// A short standalone name for the upgrades tab's row, where there is no
    /// taxon identity to lean on any more -- just what the `Kind` does.
    pub fn title(self) -> &'static str {
        match self {
            Kind::Rate => "DNA Production",
            Kind::Cost => "Cheaper Upgrades",
            Kind::Chance => "Evolve Odds",
            Kind::Cap => "DNA Capacity",
            Kind::Storage => "DNA Overflow",
            Kind::Tap => "Tap Power",
            Kind::Vigor => "Head Start",
        }
    }

    /// One line explaining the mechanic -- read before anything is
    /// committed on the choice screen, and shown again, taxon-free, as the
    /// upgrades tab row's own description (see `title`).
    pub fn description(self) -> &'static str {
        match self {
            Kind::Rate => "Adds flat DNA per second.",
            Kind::Cost => "Lowers the DNA cost of buying every upgrade's next level.",
            Kind::Chance => "Adds flat percentage points to every Evolve's odds.",
            Kind::Cap => "Adds to the DNA pool's cap.",
            Kind::Storage => "Lets the DNA pool overflow further past its cap.",
            Kind::Tap => "Adds extra fill to every tap on a species' level bar.",
            Kind::Vigor => "Newly discovered animals start at a higher level.",
        }
    }
}

pub struct UpgradeDef {
    /// The `Taxon::name` this upgrade unlocks with.
    pub taxon: &'static str,
    /// The real trait this clade is named for -- the flavour, not the
    /// mechanic. What the upgrade actually *does* is chosen at discovery
    /// time (`Game::choose_upgrade_kind`), not fixed here. Shown on the
    /// choice screen only -- the upgrades tab lists `Kind`s, not taxa.
    pub trait_name: &'static str,
}

/// One row per `Group::Backbone` taxon in `tree.rs`, in the same order the
/// tree table uses. `sprites.rs::animal_def`/`tests` keep the analogous
/// invariant for animals; `tests` below keeps this one honest for upgrades.
#[rustfmt::skip]
pub const UPGRADES: &[UpgradeDef] = &[
    UpgradeDef { taxon: "Urmetazoan",       trait_name: "Multicellularity" },
    UpgradeDef { taxon: "Cnidarian",        trait_name: "Nerve Net" },
    UpgradeDef { taxon: "Bilaterian",       trait_name: "Bilateral Symmetry" },
    UpgradeDef { taxon: "Protostome",       trait_name: "Protostomy" },
    UpgradeDef { taxon: "Deuterostome",     trait_name: "Deuterostomy" },
    UpgradeDef { taxon: "Spiralian",        trait_name: "Spiral Cleavage" },
    UpgradeDef { taxon: "Ecdysozoan",       trait_name: "Molting" },
    UpgradeDef { taxon: "Mollusc",          trait_name: "Shell" },
    UpgradeDef { taxon: "Arthropod",        trait_name: "Exoskeleton" },
    UpgradeDef { taxon: "Chelicerate",      trait_name: "Chelicerae" },
    UpgradeDef { taxon: "Pancrustacean",    trait_name: "Compound Eyes" },
    UpgradeDef { taxon: "Insect",           trait_name: "Flight" },
    UpgradeDef { taxon: "Echinoderm",       trait_name: "Water Vascular System" },
    UpgradeDef { taxon: "Chordate",         trait_name: "Notochord" },
    UpgradeDef { taxon: "Vertebrate",       trait_name: "Vertebral Column" },
    UpgradeDef { taxon: "Jawed Fish",       trait_name: "Jaws" },
    UpgradeDef { taxon: "Lobe-finned Fish", trait_name: "Paired Fins" },
    UpgradeDef { taxon: "Tetrapod",         trait_name: "Limbs" },
    UpgradeDef { taxon: "Amniote",          trait_name: "Amniotic Egg" },
    UpgradeDef { taxon: "Mammal",           trait_name: "Endothermy" },
    UpgradeDef { taxon: "Placental",        trait_name: "Placenta" },
    UpgradeDef { taxon: "Primate",          trait_name: "Grasping Hands" },
    UpgradeDef { taxon: "Monkey",           trait_name: "Colour Vision" },
    UpgradeDef { taxon: "Reptile",          trait_name: "Keratinized Scales" },
];

pub fn def_for(taxon_name: &str) -> Option<&'static UpgradeDef> {
    UPGRADES.iter().find(|u| u.taxon == taxon_name)
}

/// The combined effect of every purchased `Kind`, one shared total per
/// `Kind` regardless of how many taxa express it or how many species
/// inherit it -- every field is a flat sum, never a compounding multiplier,
/// so buying levels (or discovering more species) is never a runaway
/// return: it is always worth exactly what it says.
#[derive(Clone, Copy, Default)]
pub struct Effects {
    pub rate_bonus: f64,
    pub cost_reduction: f64,
    pub chance_bonus: f32,
    pub cap_bonus: f64,
    pub overflow_bonus: f64,
    pub tap_bonus: f64,
    pub bonus_start_level: f64,
}

/// `kind_level[kind.index()]` is 0 until some taxon's choice has activated
/// that `Kind` (`Game::choose_upgrade_kind`), 1 the instant it does (free),
/// and climbs from there only by direct purchase (`Game::buy_upgrade_level`)
/// -- shared across every taxon that happens to express that `Kind`, not
/// per-taxon: picking an already-active `Kind` again elsewhere doesn't add
/// a second counter.
pub fn compute(kind_level: &[u32; NUM_KINDS]) -> Effects {
    let mut e = Effects::default();
    for kind in ALL_KINDS {
        let n = kind_level[kind.index()];
        if n == 0 {
            continue;
        }
        let amount = kind.per_level() * n as f64;
        match kind {
            Kind::Rate => e.rate_bonus += amount,
            Kind::Cost => e.cost_reduction += amount,
            Kind::Chance => e.chance_bonus += amount as f32,
            Kind::Cap => e.cap_bonus += amount,
            Kind::Storage => e.overflow_bonus += amount,
            Kind::Tap => e.tap_bonus += amount,
            Kind::Vigor => e.bonus_start_level += amount,
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Group, Phylogeny};

    #[test]
    fn every_backbone_taxon_has_exactly_one_upgrade() {
        let phy = Phylogeny::load();
        for t in &phy.taxa {
            let found = UPGRADES.iter().filter(|u| u.taxon == t.name).count();
            if t.group == Group::Backbone {
                assert_eq!(found, 1, "{} (Backbone) has {found} upgrade defs", t.name);
            } else {
                assert_eq!(
                    found, 0,
                    "{} is not Backbone but has an upgrade def",
                    t.name
                );
            }
        }
    }

    #[test]
    fn every_kind_has_a_distinct_index() {
        let mut seen = [false; NUM_KINDS];
        for kind in ALL_KINDS {
            let i = kind.index();
            assert!(!seen[i], "index {i} used by more than one Kind");
            seen[i] = true;
        }
    }

    #[test]
    fn no_upgrade_effect_starts_active() {
        let e = compute(&[0; NUM_KINDS]);
        assert_eq!(e.rate_bonus, 0.0);
        assert_eq!(e.cost_reduction, 0.0);
        assert_eq!(e.chance_bonus, 0.0);
        assert_eq!(e.cap_bonus, 0.0);
        assert_eq!(e.overflow_bonus, 0.0);
        assert_eq!(e.tap_bonus, 0.0);
        assert_eq!(e.bonus_start_level, 0.0);
    }

    /// Undiscovered (level 0) is the only state with no effect -- level 1 is
    /// the free grant a resolved choice gives, and already counts.
    #[test]
    fn level_one_already_has_an_effect() {
        let mut kind_level = [0; NUM_KINDS];
        assert_eq!(
            compute(&kind_level).cost_reduction,
            0.0,
            "level 0: no effect"
        );
        kind_level[Kind::Cost.index()] = 1;
        assert!(
            compute(&kind_level).cost_reduction > 0.0,
            "level 1: already active"
        );
    }

    /// Every kind adds linearly with level -- no compounding, so it can
    /// never grow faster than the level count itself.
    #[test]
    fn every_kind_adds_linearly_with_level() {
        for kind in ALL_KINDS {
            let mut kind_level = [0; NUM_KINDS];
            kind_level[kind.index()] = 1;
            let one = compute(&kind_level);
            kind_level[kind.index()] = 3;
            let three = compute(&kind_level);
            let (a, b) = match kind {
                Kind::Rate => (one.rate_bonus, three.rate_bonus),
                Kind::Cost => (one.cost_reduction, three.cost_reduction),
                Kind::Chance => (one.chance_bonus as f64, three.chance_bonus as f64),
                Kind::Cap => (one.cap_bonus, three.cap_bonus),
                Kind::Storage => (one.overflow_bonus, three.overflow_bonus),
                Kind::Tap => (one.tap_bonus, three.tap_bonus),
                Kind::Vigor => (one.bonus_start_level, three.bonus_start_level),
            };
            assert!(
                (b - 3.0 * a).abs() < 1e-9,
                "{kind:?}: level 3 should be exactly 3x level 1 ({a} -> {b})"
            );
        }
    }

    /// Two different taxa expressing the same `Kind` add into one shared
    /// total -- `compute` only ever sees `kind_level`, never which or how
    /// many taxa are behind it, so this is really just confirming the
    /// caller-visible contract `Game::choose_upgrade_kind` relies on.
    #[test]
    fn multiple_taxa_sharing_a_kind_add_into_one_shared_level() {
        let mut kind_level = [0; NUM_KINDS];
        kind_level[Kind::Rate.index()] = 1;
        let one_taxon = compute(&kind_level).rate_bonus;

        kind_level[Kind::Rate.index()] = 2;
        let two_taxa = compute(&kind_level).rate_bonus;

        assert!(two_taxa > one_taxon);
    }
}
