//! The static phylogeny the whole game is built on.
//!
//! Topology and divergence estimates are sourced from the phylogenomic
//! literature -- see `docs/PHYLOGENY.md` for the per-clade citations.
//! Backbone: Laumer et al. 2019 (Porifera-sister Metazoa), Irisarri et al.
//! 2017 (jawed vertebrates), Misof et al. 2014 (insects), Upham et al. 2019
//! (mammals), Prum et al. 2015 (birds).

/// Visual family of a taxon. Purely cosmetic -- drives node colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    /// Internal "last common ancestor" nodes: the trunk of the tree.
    Backbone,
    Basal,
    Spiralia,
    Ecdysozoa,
    Deuterostome,
    Fish,
    Tetrapod,
    Reptile,
    Mammal,
}

/// `(common name, clade, origin in millions of years ago, parent index, group)`
/// Parents always precede their children, which lets us build the tree in one pass.
type Row = (&'static str, &'static str, f32, i32, Group);

use Group::*;

#[rustfmt::skip]
const ROWS: &[Row] = &[
    // --- Metazoa: the root and the non-bilaterian branches -----------------
    ("Urmetazoan",       "Metazoa",          800.0, -1, Backbone),
    ("Sea Sponge",       "Porifera",         800.0,  0, Basal),
    ("Comb Jelly",       "Ctenophora",       780.0,  0, Basal),
    ("Placozoan",        "Placozoa",         760.0,  0, Basal),
    ("Cnidarian",        "Cnidaria",         740.0,  0, Backbone),
    ("Jellyfish",        "Medusozoa",        700.0,  4, Basal),
    ("Coral",            "Anthozoa",         700.0,  4, Basal),

    // --- Bilateria splits into protostomes and deuterostomes ---------------
    ("Bilaterian",       "Bilateria",        690.0,  0, Backbone),
    ("Acoel Worm",       "Xenacoelomorpha",  650.0,  7, Basal),
    ("Protostome",       "Protostomia",      650.0,  7, Backbone),
    ("Deuterostome",     "Deuterostomia",    640.0,  7, Backbone),
    ("Spiralian",        "Spiralia",         620.0,  9, Backbone),
    ("Ecdysozoan",       "Ecdysozoa",        610.0,  9, Backbone),

    // --- Spiralia ----------------------------------------------------------
    ("Flatworm",         "Platyhelminthes",  580.0, 11, Spiralia),
    ("Mollusc",          "Mollusca",         550.0, 11, Backbone),
    ("Snail",            "Gastropoda",       530.0, 14, Spiralia),
    ("Octopus",          "Cephalopoda",      530.0, 14, Spiralia),
    ("Segmented Worm",   "Annelida",         550.0, 11, Spiralia),

    // --- Ecdysozoa ---------------------------------------------------------
    ("Nematode",         "Nematoda",         600.0, 12, Ecdysozoa),
    ("Arthropod",        "Arthropoda",       570.0, 12, Backbone),
    ("Chelicerate",      "Chelicerata",      530.0, 19, Backbone),
    ("Horseshoe Crab",   "Xiphosura",        445.0, 20, Ecdysozoa),
    ("Scorpion",         "Scorpiones",       430.0, 20, Ecdysozoa),
    ("Spider",           "Araneae",          400.0, 20, Ecdysozoa),
    ("Pancrustacean",    "Pancrustacea",     520.0, 19, Backbone),
    ("Crab",             "Decapoda",         450.0, 24, Ecdysozoa),
    ("Insect",           "Insecta",          480.0, 24, Backbone),
    ("Dragonfly",        "Odonata",          400.0, 26, Ecdysozoa),
    ("Beetle",           "Coleoptera",       300.0, 26, Ecdysozoa),
    ("Butterfly",        "Lepidoptera",      240.0, 26, Ecdysozoa),
    ("Ant",              "Formicidae",       140.0, 26, Ecdysozoa),

    // --- Deuterostomia -----------------------------------------------------
    ("Echinoderm",       "Echinodermata",    560.0, 10, Backbone),
    ("Starfish",         "Asteroidea",       500.0, 31, Deuterostome),
    ("Sea Urchin",       "Echinoidea",       500.0, 31, Deuterostome),
    ("Chordate",         "Chordata",         550.0, 10, Backbone),
    ("Sea Squirt",       "Tunicata",         540.0, 34, Deuterostome),

    // --- Vertebrata --------------------------------------------------------
    ("Vertebrate",       "Vertebrata",       520.0, 34, Backbone),
    ("Lamprey",          "Cyclostomata",     500.0, 36, Fish),
    ("Jawed Fish",       "Gnathostomata",    470.0, 36, Backbone),
    ("Shark",            "Chondrichthyes",   450.0, 38, Fish),
    ("Ray-finned Fish",  "Actinopterygii",   430.0, 38, Fish),
    ("Lobe-finned Fish", "Sarcopterygii",    425.0, 38, Backbone),
    ("Coelacanth",       "Actinistia",       415.0, 41, Fish),

    // --- Tetrapoda ---------------------------------------------------------
    ("Tetrapod",         "Tetrapoda",        390.0, 41, Backbone),
    ("Amphibian",        "Amphibia",         350.0, 43, Tetrapod),
    ("Frog",             "Anura",            250.0, 44, Tetrapod),
    ("Amniote",          "Amniota",          320.0, 43, Backbone),

    // --- Mammalia ----------------------------------------------------------
    ("Mammal",           "Mammalia",         180.0, 46, Backbone),
    ("Platypus",         "Monotremata",      180.0, 47, Mammal),
    ("Kangaroo",         "Marsupialia",      160.0, 47, Mammal),
    ("Placental",        "Placentalia",      100.0, 47, Backbone),
    ("Elephant",         "Afrotheria",        80.0, 50, Mammal),
    ("Mouse",            "Rodentia",          70.0, 50, Mammal),
    ("Bat",              "Chiroptera",        65.0, 50, Mammal),
    ("Wolf",             "Carnivora",         55.0, 50, Mammal),
    ("Whale",            "Cetartiodactyla",   55.0, 50, Mammal),
    ("Primate",          "Primates",          75.0, 50, Backbone),
    ("Lemur",            "Strepsirrhini",     65.0, 56, Mammal),
    ("Monkey",           "Simiiformes",       43.0, 56, Backbone),
    ("Ape",              "Hominoidea",        25.0, 58, Mammal),
    ("Chimpanzee",       "Panina",             7.0, 59, Mammal),
    ("Human",            "Homo sapiens",       0.3, 59, Mammal),

    // --- Sauropsida --------------------------------------------------------
    ("Reptile",          "Sauropsida",       310.0, 46, Backbone),
    ("Turtle",           "Testudines",       255.0, 62, Reptile),
    ("Lizard",           "Lepidosauria",     250.0, 62, Reptile),
    ("Crocodile",        "Crocodylia",       240.0, 62, Reptile),
    ("Dinosaur",         "Dinosauria",       235.0, 62, Reptile),
    ("Bird",             "Aves",             110.0, 66, Reptile),
];

pub struct Taxon {
    pub name: &'static str,
    pub clade: &'static str,
    pub mya: f32,
    pub parent: Option<usize>,
    pub group: Group,
    pub children: Vec<usize>,
    pub depth: u8,
}

impl Taxon {
    /// `"800 Ma"` / `"0.3 Ma"` -- the subtitle under each unlocked node.
    pub fn age_label(&self) -> String {
        if self.mya < 1.0 {
            format!("{:.1} Ma", self.mya)
        } else {
            format!("{} Ma", self.mya as i32)
        }
    }
}

pub struct Phylogeny {
    pub taxa: Vec<Taxon>,
}

impl Phylogeny {
    pub fn load() -> Self {
        let mut taxa: Vec<Taxon> = Vec::with_capacity(ROWS.len());

        for &(name, clade, mya, parent, group) in ROWS {
            let parent = if parent < 0 {
                None
            } else {
                Some(parent as usize)
            };
            // Rows are ordered parents-first, so the parent's depth is already known.
            let depth = parent.map_or(0, |p| taxa[p].depth + 1);
            taxa.push(Taxon {
                name,
                clade,
                mya,
                parent,
                group,
                children: Vec::new(),
                depth,
            });
        }

        for i in 0..taxa.len() {
            if let Some(p) = taxa[i].parent {
                taxa[p].children.push(i);
            }
        }

        Self { taxa }
    }

    pub const ROOT: usize = 0;

    pub fn len(&self) -> usize {
        self.taxa.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_taxon_is_reachable_from_the_root() {
        let phy = Phylogeny::load();
        let mut seen = vec![false; phy.len()];
        let mut stack = vec![Phylogeny::ROOT];
        while let Some(i) = stack.pop() {
            seen[i] = true;
            stack.extend(phy.taxa[i].children.iter().copied());
        }
        assert!(seen.iter().all(|&s| s), "tree has orphaned taxa");
    }

    #[test]
    fn children_are_never_older_than_their_parent() {
        let phy = Phylogeny::load();
        for t in &phy.taxa {
            if let Some(p) = t.parent {
                assert!(
                    t.mya <= phy.taxa[p].mya,
                    "{} predates {}",
                    t.name,
                    phy.taxa[p].name
                );
            }
        }
    }
}
