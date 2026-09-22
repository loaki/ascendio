//! Upgrades: what a `Group::Backbone` taxon offers.
//!
//! A category node -- Mollusca, Arthropoda, Chordata, and the rest of the
//! internal, "common ancestor" nodes in the tree -- doesn't produce DNA
//! itself (see `game::output`). Discovering one grants its upgrade instead,
//! named after a real trait that clade is known for, active immediately at
//! level 1 -- free, no purchase needed. Buying further levels
//! (`Game::buy_upgrade_level`) is what costs DNA, and there is no ceiling on
//! how many times you can. That level is its own counter, unrelated to
//! `Game::level` (which the taxon still has, for `lineage_bonus`) -- buying
//! upgrade levels is the only thing that ever moves it. Every one of the 24
//! `Backbone` taxa has an entry here, so no category discovery is ever a
//! dead end.

use crate::tree::Phylogeny;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Multiplies total DNA/s.
    Rate,
    /// Multiplies every attempt's DNA cost (< 1).
    Cost,
    /// Adds flat percentage points to every attempt's success chance.
    Chance,
    /// Multiplies the DNA pool's cap.
    Cap,
    /// Adds banked Evolve slots: reaching a full pool while away banks a
    /// charge and restarts the count, instead of just sitting there waiting
    /// for a tap. See `Game::tick` and `Game::evolve`.
    Storage,
}

impl Kind {
    /// Magnitude per level: how much one level of this upgrade moves its
    /// stat. `Storage` doesn't fit this percentage shape -- it adds whole
    /// slots -- so `upgrades::compute` handles it as a special case instead
    /// of calling this.
    pub fn per_level(self) -> f64 {
        match self {
            Kind::Rate => 0.08,
            Kind::Cost => -0.06,
            Kind::Chance => 0.04,
            Kind::Cap => 0.10,
            Kind::Storage => 0.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Rate => "DNA",
            Kind::Cost => "cost",
            Kind::Chance => "chance",
            Kind::Cap => "DNA cap",
            Kind::Storage => "banked evolve",
        }
    }
}

pub struct UpgradeDef {
    /// The `Taxon::name` this upgrade unlocks with.
    pub taxon: &'static str,
    /// The real trait this clade is named for -- the flavour, not the taxon.
    pub trait_name: &'static str,
    pub kind: Kind,
}

/// One row per `Group::Backbone` taxon in `tree.rs`, in the same order the
/// tree table uses. `sprites.rs::animal_def`/`tests` keep the analogous
/// invariant for animals; `tests` below keeps this one honest for upgrades.
#[rustfmt::skip]
pub const UPGRADES: &[UpgradeDef] = &[
    UpgradeDef { taxon: "Urmetazoan",       trait_name: "Multicellularity",  kind: Kind::Rate },
    UpgradeDef { taxon: "Cnidarian",        trait_name: "Nerve Net",         kind: Kind::Chance },
    UpgradeDef { taxon: "Bilaterian",       trait_name: "Bilateral Symmetry",kind: Kind::Rate },
    UpgradeDef { taxon: "Protostome",       trait_name: "Protostomy",        kind: Kind::Cost },
    UpgradeDef { taxon: "Deuterostome",     trait_name: "Deuterostomy",      kind: Kind::Cost },
    UpgradeDef { taxon: "Spiralian",        trait_name: "Spiral Cleavage",   kind: Kind::Rate },
    UpgradeDef { taxon: "Ecdysozoan",       trait_name: "Molting",           kind: Kind::Chance },
    UpgradeDef { taxon: "Mollusc",          trait_name: "Shell",             kind: Kind::Cap },
    UpgradeDef { taxon: "Arthropod",        trait_name: "Exoskeleton",       kind: Kind::Cost },
    UpgradeDef { taxon: "Chelicerate",      trait_name: "Chelicerae",        kind: Kind::Rate },
    UpgradeDef { taxon: "Pancrustacean",    trait_name: "Compound Eyes",     kind: Kind::Chance },
    UpgradeDef { taxon: "Insect",           trait_name: "Flight",            kind: Kind::Rate },
    UpgradeDef { taxon: "Echinoderm",       trait_name: "Water Vascular System", kind: Kind::Cap },
    UpgradeDef { taxon: "Chordate",         trait_name: "Notochord",         kind: Kind::Cap },
    UpgradeDef { taxon: "Vertebrate",       trait_name: "Vertebral Column",  kind: Kind::Rate },
    UpgradeDef { taxon: "Jawed Fish",       trait_name: "Jaws",              kind: Kind::Chance },
    UpgradeDef { taxon: "Lobe-finned Fish", trait_name: "Paired Fins",       kind: Kind::Cost },
    UpgradeDef { taxon: "Tetrapod",         trait_name: "Limbs",             kind: Kind::Rate },
    UpgradeDef { taxon: "Amniote",          trait_name: "Amniotic Egg",      kind: Kind::Storage },
    UpgradeDef { taxon: "Mammal",           trait_name: "Endothermy",        kind: Kind::Rate },
    UpgradeDef { taxon: "Placental",        trait_name: "Placenta",          kind: Kind::Cap },
    UpgradeDef { taxon: "Primate",          trait_name: "Grasping Hands",    kind: Kind::Cost },
    UpgradeDef { taxon: "Monkey",           trait_name: "Colour Vision",     kind: Kind::Chance },
    UpgradeDef { taxon: "Reptile",          trait_name: "Keratinized Scales",kind: Kind::Cap },
];

pub fn def_for(taxon_name: &str) -> Option<&'static UpgradeDef> {
    UPGRADES.iter().find(|u| u.taxon == taxon_name)
}

/// Every upgrade paired with the taxon id it unlocks on, in `UPGRADES`
/// order (which itself follows the tree). For the Upgrades screen: it needs
/// to know *which* taxon each row is about, not just its name.
pub fn rows(phy: &Phylogeny) -> Vec<(usize, &'static UpgradeDef)> {
    UPGRADES
        .iter()
        .map(|def| {
            let taxon = phy
                .taxa
                .iter()
                .position(|t| t.name == def.taxon)
                .unwrap_or_else(|| panic!("upgrade for unknown taxon '{}'", def.taxon));
            (taxon, def)
        })
        .collect()
}

/// The combined effect of every discovered category's upgrade. A level-`n`
/// upgrade compounds `n` times, so buying it up is never a diminishing
/// return the way a flat bonus would be. `storage_slots` is the exception --
/// it adds, level `n` of a `Storage` upgrade is worth exactly `n` slots.
#[derive(Clone, Copy)]
pub struct Effects {
    pub rate_mul: f64,
    pub cost_mul: f64,
    pub chance_bonus: f32,
    pub cap_mul: f64,
    pub storage_slots: u32,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            rate_mul: 1.0,
            cost_mul: 1.0,
            chance_bonus: 0.0,
            cap_mul: 1.0,
            storage_slots: 0,
        }
    }
}

/// `upgrade_level[i]` is 0 for anything not (yet) discovered, and at least 1
/// for every discovered `Backbone` taxon -- `Game::evolve` grants that first
/// level for free the instant it unlocks one.
pub fn compute(phy: &Phylogeny, upgrade_level: &[u32]) -> Effects {
    let mut e = Effects::default();
    for (i, taxon) in phy.taxa.iter().enumerate() {
        if upgrade_level[i] == 0 {
            continue;
        }
        let Some(def) = def_for(taxon.name) else {
            continue;
        };
        let n = upgrade_level[i] as i32;
        match def.kind {
            Kind::Rate => e.rate_mul *= (1.0 + def.kind.per_level()).powi(n),
            Kind::Cost => e.cost_mul *= (1.0 + def.kind.per_level()).powi(n),
            Kind::Chance => e.chance_bonus += def.kind.per_level() as f32 * n as f32,
            Kind::Cap => e.cap_mul *= (1.0 + def.kind.per_level()).powi(n),
            Kind::Storage => e.storage_slots += upgrade_level[i],
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Group;

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
    fn no_upgrade_effect_starts_active() {
        let phy = Phylogeny::load();
        let upgrade_level = vec![0; phy.len()];
        let e = compute(&phy, &upgrade_level);
        assert_eq!(e.rate_mul, 1.0);
        assert_eq!(e.cost_mul, 1.0);
        assert_eq!(e.chance_bonus, 0.0);
        assert_eq!(e.cap_mul, 1.0);
    }

    /// Undiscovered (level 0) is the only state with no effect -- level 1 is
    /// the free grant `Game::evolve` gives on discovery, and already counts.
    #[test]
    fn level_one_already_has_an_effect() {
        let phy = Phylogeny::load();
        let arthropod = phy.taxa.iter().position(|t| t.name == "Arthropod").unwrap();
        let mut upgrade_level = vec![0; phy.len()];

        assert_eq!(
            compute(&phy, &upgrade_level).cost_mul,
            1.0,
            "level 0: no effect"
        );
        upgrade_level[arthropod] = 1;
        assert!(
            compute(&phy, &upgrade_level).cost_mul < 1.0,
            "level 1: already active"
        );
    }

    #[test]
    fn storage_slots_add_rather_than_compound() {
        let phy = Phylogeny::load();
        let amniote = phy.taxa.iter().position(|t| t.name == "Amniote").unwrap();
        assert_eq!(def_for("Amniote").unwrap().kind, Kind::Storage);

        let mut upgrade_level = vec![0; phy.len()];
        assert_eq!(compute(&phy, &upgrade_level).storage_slots, 0);

        upgrade_level[amniote] = 1;
        assert_eq!(compute(&phy, &upgrade_level).storage_slots, 1);

        upgrade_level[amniote] = 4;
        assert_eq!(compute(&phy, &upgrade_level).storage_slots, 4);
    }

    #[test]
    fn buying_further_levels_compounds_the_upgrade() {
        let phy = Phylogeny::load();
        let arthropod = phy.taxa.iter().position(|t| t.name == "Arthropod").unwrap();
        let mut upgrade_level = vec![0; phy.len()];
        upgrade_level[arthropod] = 1;

        let one = compute(&phy, &upgrade_level).cost_mul;
        upgrade_level[arthropod] = 3;
        let three = compute(&phy, &upgrade_level).cost_mul;

        assert!(
            three < one,
            "levels 1->3 should discount cost further ({one} -> {three})"
        );
    }
}
