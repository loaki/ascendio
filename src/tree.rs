//! The static phylogeny the game is built on. Sources: `docs/PHYLOGENY.md`.

use crate::ecology::{self, Ecology};

/// Visual family of a taxon: picks its sprite art and pose.
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

/// `(name, clade, origin in Ma, parent index, group)`. New rows are only ever
/// appended, so saves keep their indices; an inserted node is appended too
/// and its children re-parented to it.
type Row = (&'static str, &'static str, f32, i32, Group);

use Group::*;

#[rustfmt::skip]
const ROWS: &[Row] = &[
    // --- Metazoa: the root and the non-bilaterian branches -----------------
    ("Urmetazoan",       "Metazoa",               800.0,  -1, Backbone),
    ("Sea Sponge",       "Porifera",              800.0,   0, Basal),
    ("Comb Jelly",       "Ctenophora",            780.0,   0, Basal),
    ("Placozoan",        "Placozoa",              760.0,   0, Basal),
    ("Cnidarian",        "Cnidaria",              740.0,   0, Backbone),
    ("Jellyfish",        "Medusozoa",             700.0,   4, Basal),
    ("Coral",            "Anthozoa",              700.0,   4, Basal),

    // --- Bilateria splits into protostomes and deuterostomes ---------------
    ("Bilaterian",       "Bilateria",             690.0,   0, Backbone),
    ("Acoel Worm",       "Xenacoelomorpha",       650.0,   7, Basal),
    ("Protostome",       "Protostomia",           650.0,   7, Backbone),
    ("Deuterostome",     "Deuterostomia",         640.0,   7, Backbone),
    ("Spiralian",        "Spiralia",              620.0,   9, Backbone),
    ("Ecdysozoan",       "Ecdysozoa",             610.0,   9, Backbone),

    // --- Spiralia ----------------------------------------------------------
    ("Flatworm",         "Platyhelminthes",       580.0,  11, Spiralia),
    ("Mollusc",          "Mollusca",              550.0,  11, Backbone),
    ("Snail",            "Gastropoda",            530.0,  14, Spiralia),
    ("Octopus",          "Cephalopoda",           530.0,  14, Spiralia),
    ("Segmented Worm",   "Annelida",              550.0,  11, Spiralia),

    // --- Ecdysozoa ---------------------------------------------------------
    ("Nematode",         "Nematoda",              600.0,  12, Ecdysozoa),
    ("Arthropod",        "Arthropoda",            570.0,  12, Backbone),
    ("Chelicerate",      "Chelicerata",           530.0,  19, Backbone),
    ("Horseshoe Crab",   "Xiphosura",             445.0,  20, Ecdysozoa),
    ("Scorpion",         "Scorpiones",            430.0,  20, Ecdysozoa),
    ("Spider",           "Araneae",               400.0,  20, Ecdysozoa),
    ("Pancrustacean",    "Pancrustacea",          520.0,  19, Backbone),
    ("Crab",             "Decapoda",              450.0,  24, Ecdysozoa),
    ("Insect",           "Insecta",               480.0,  24, Backbone),
    ("Dragonfly",        "Odonata",               400.0,  26, Ecdysozoa),
    ("Beetle",           "Coleoptera",            300.0,  26, Ecdysozoa),
    ("Butterfly",        "Lepidoptera",           240.0,  26, Ecdysozoa),
    ("Ant",              "Formicidae",            140.0, 139, Ecdysozoa),

    // --- Deuterostomia -----------------------------------------------------
    ("Echinoderm",       "Echinodermata",         560.0,  10, Backbone),
    ("Starfish",         "Asteroidea",            500.0,  31, Deuterostome),
    ("Sea Urchin",       "Echinoidea",            500.0,  31, Deuterostome),
    ("Chordate",         "Chordata",              550.0,  10, Backbone),
    ("Sea Squirt",       "Tunicata",              540.0,  34, Deuterostome),

    // --- Vertebrata --------------------------------------------------------
    ("Vertebrate",       "Vertebrata",            520.0,  34, Backbone),
    ("Lamprey",          "Cyclostomata",          500.0,  36, Fish),
    ("Jawed Fish",       "Gnathostomata",         470.0,  36, Backbone),
    ("Shark",            "Chondrichthyes",        450.0,  38, Fish),
    ("Ray-finned Fish",  "Actinopterygii",        430.0,  38, Fish),
    ("Lobe-finned Fish", "Sarcopterygii",         425.0,  38, Backbone),
    ("Coelacanth",       "Actinistia",            415.0,  41, Fish),

    // --- Tetrapoda ---------------------------------------------------------
    ("Tetrapod",         "Tetrapoda",             390.0,  41, Backbone),
    ("Amphibian",        "Amphibia",              350.0,  43, Tetrapod),
    ("Frog",             "Anura",                 250.0,  44, Tetrapod),
    ("Amniote",          "Amniota",               320.0,  43, Backbone),

    // --- Mammalia ----------------------------------------------------------
    ("Mammal",           "Mammalia",              180.0, 119, Backbone),
    ("Platypus",         "Monotremata",           180.0,  47, Mammal),
    ("Marsupial",        "Marsupialia",           160.0,  47, Backbone),
    ("Placental",        "Placentalia",           100.0,  47, Backbone),
    ("Afrothere",        "Afrotheria",             80.0,  50, Backbone),
    ("Mouse",            "Rodentia",               70.0,  50, Mammal),
    ("Bat",              "Chiroptera",             65.0,  50, Mammal),
    ("Carnivoran",       "Carnivora",              55.0,  50, Backbone),
    ("Cetartiodactyl",   "Cetartiodactyla",        55.0,  50, Backbone),
    ("Primate",          "Primates",               75.0,  50, Backbone),
    ("Lemur",            "Strepsirrhini",          65.0,  56, Mammal),
    ("Monkey",           "Simiiformes",            43.0,  56, Backbone),
    ("Ape",              "Hominoidea",             25.0,  58, Mammal),
    ("Chimpanzee",       "Panina",                  7.0,  59, Mammal),
    ("Human",            "Homo sapiens",            0.3, 122, Mammal),

    // --- Sauropsida --------------------------------------------------------
    ("Reptile",          "Sauropsida",            310.0,  46, Backbone),
    ("Turtle",           "Testudines",            255.0,  62, Reptile),
    ("Lizard",           "Lepidosauria",          250.0,  62, Reptile),
    ("Crocodile",        "Crocodylia",            240.0, 120, Reptile),
    ("Dinosaur",         "Dinosauria",            235.0, 120, Reptile),
    ("Bird",             "Aves",                  110.0, 121, Reptile),

    // --- Added later: appended so existing indices (and saves) stay valid --
    ("Clam",             "Bivalvia",              510.0,  14, Spiralia),
    ("Squid",            "Decapodiformes",        300.0,  16, Spiralia),
    ("Ammonite",         "Ammonoidea",            410.0,  16, Spiralia),
    ("Earthworm",        "Clitellata",            300.0,  17, Spiralia),
    ("Tardigrade",       "Tardigrada",            530.0,  12, Ecdysozoa),
    ("Trilobite",        "Trilobita",             521.0,  19, Ecdysozoa),
    ("Anomalocaris",     "Radiodonta",            515.0,  19, Ecdysozoa),
    ("Centipede",        "Myriapoda",             430.0,  19, Ecdysozoa),
    ("Shrimp",           "Caridea",               400.0,  24, Ecdysozoa),
    ("Bee",              "Apoidea",               120.0, 139, Ecdysozoa),
    ("Meganeura",        "Meganisoptera",         300.0,  27, Ecdysozoa),
    ("Dunkleosteus",     "Placodermi",            380.0,  38, Fish),
    ("Stingray",         "Batoidea",              200.0,  39, Fish),
    ("Megalodon",        "Otodus",                 23.0,  39, Fish),
    ("Seahorse",         "Hippocampus",            25.0, 124, Fish),
    ("Anglerfish",       "Lophiiformes",          130.0, 124, Fish),
    ("Tiktaalik",        "Tiktaalik",             375.0,  41, Fish),
    ("Salamander",       "Caudata",               165.0,  44, Tetrapod),
    ("Snake",            "Serpentes",             110.0,  64, Reptile),
    ("Pterosaur",        "Pterosauria",           228.0, 120, Reptile),
    ("T. rex",           "Tyrannosaurus",          68.0, 121, Reptile),
    ("Sauropod",         "Sauropoda",             200.0,  66, Reptile),
    ("Penguin",          "Spheniscidae",           60.0,  67, Reptile),
    ("Mammoth",          "Mammuthus",               5.0, 125, Mammal),
    ("Rabbit",           "Lagomorpha",             55.0,  50, Mammal),
    ("Horse",            "Equidae",                55.0, 123, Mammal),
    ("Dolphin",          "Delphinidae",            11.0, 126, Mammal),
    ("Lion",             "Felidae",                25.0,  54, Mammal),
    ("Gorilla",          "Gorilla",                 9.0,  59, Mammal),

    // --- The 151 expansion: sources in docs/PHYLOGENY.md ---------------------
    ("Nautilus",         "Nautilida",             415.0,  16, Spiralia),
    ("Fly",              "Diptera",               260.0,  26, Ecdysozoa),
    ("Clownfish",        "Amphiprion",             13.0, 124, Fish),
    ("Lungfish",         "Dipnoi",                410.0,  41, Fish),
    ("Pig",              "Suidae",                 37.0,  55, Mammal),
    ("Tuatara",          "Rhynchocephalia",       230.0,  64, Reptile),
    ("Sea Turtle",       "Chelonioidea",          120.0,  63, Reptile),
    ("Plesiosaur",       "Plesiosauria",          205.0,  62, Reptile),
    ("Ichthyosaur",      "Ichthyosauria",         250.0,  62, Reptile),
    ("Triceratops",      "Triceratops",            68.0,  66, Reptile),
    ("Velociraptor",     "Velociraptor",           75.0, 121, Reptile),
    ("Archaeopteryx",    "Archaeopteryx",         150.0, 121, Reptile),
    ("Ostrich",          "Struthio",               80.0,  67, Reptile),
    ("Parrot",           "Psittaciformes",         55.0,  67, Reptile),
    ("Owl",              "Strigiformes",           60.0,  67, Reptile),
    ("Koala",            "Phascolarctos",          35.0,  49, Mammal),
    ("Sloth",            "Folivora",               35.0,  50, Mammal),
    ("Giraffe",          "Giraffidae",             20.0,  55, Mammal),
    ("Hippo",            "Hippopotamidae",         16.0,  55, Mammal),
    ("Rhino",            "Rhinocerotidae",         16.0, 123, Mammal),
    ("Bear",             "Ursidae",                20.0,  54, Mammal),
    ("Seal",             "Pinnipedia",             25.0,  54, Mammal),
    ("Synapsid",         "Synapsida",             315.0,  46, Backbone),
    ("Archosaur",        "Archosauria",           250.0,  62, Backbone),
    ("Theropod",         "Theropoda",             230.0,  66, Backbone),
    ("Hominin",          "Hominini",                7.0,  59, Backbone),
    ("Perissodactyl",    "Perissodactyla",         60.0,  50, Backbone),
    ("Spiny-rayed Fish", "Acanthomorpha",         130.0,  40, Backbone),
    ("Elephant",         "Elephantidae",            7.0,  51, Mammal),
    ("Whale",            "Cetacea",                53.0,  55, Mammal),
    ("Wolf",             "Canidae",                40.0,  54, Mammal),
    ("Dog",              "Canis familiaris",       0.02, 127, Mammal),
    ("Kangaroo",         "Macropodidae",           15.0,  49, Mammal),
    ("Tiger",            "Panthera tigris",         3.0,  95, Mammal),
    ("Cat",              "Felis",                   6.0,  95, Mammal),
    ("Sabre-tooth",      "Smilodon",                2.5,  95, Mammal),
    ("Giant Panda",      "Ailuropoda",             19.0, 117, Mammal),
    ("Cow",              "Bovidae",                18.0,  55, Mammal),
    ("Chicken",          "Galloanserae",           67.0,  67, Reptile),
    ("Songbird",         "Passeriformes",          47.0,  67, Reptile),
    ("Dodo",             "Raphus",                 25.0,  67, Reptile),
    ("True Bug",         "Hemiptera",             310.0,  26, Ecdysozoa),
    ("Wasp",             "Hymenoptera",           280.0,  26, Ecdysozoa),
    ("Mite",             "Acari",                 410.0,  20, Ecdysozoa),
    ("Termite",          "Termitoidae",           140.0,  26, Ecdysozoa),
    ("Moss Animal",      "Bryozoa",               485.0,  11, Spiralia),
    ("Brittle Star",     "Ophiuroidea",           270.0,  31, Deuterostome),
    ("Carp",             "Ostariophysi",          150.0,  40, Fish),
    ("Dickinsonia",      "Dickinsonia",           558.0,   0, Basal),
    ("Dimetrodon",       "Dimetrodon",            295.0, 119, Mammal),
    ("Lucy",             "Australopithecus",        3.9, 122, Mammal),
    ("Neanderthal",      "Homo neanderthalensis",   0.4, 122, Mammal),
    ("Pakicetus",        "Pakicetus",              50.0, 126, Mammal),
    ("Acanthostega",     "Acanthostega",          365.0,  43, Tetrapod),
];

pub struct Taxon {
    pub name: &'static str,
    pub clade: &'static str,
    pub mya: f32,
    pub parent: Option<usize>,
    pub group: Group,
    pub children: Vec<usize>,
    pub depth: u8,
    /// Its needs, tier and keystone effect (`ecology::of`), looked up once.
    pub eco: Ecology,
}

impl Taxon {
    /// `"800 Ma"` / `"0.3 Ma"` / `"20,000 years"`.
    pub fn age_label(&self) -> String {
        if self.mya < 0.1 {
            let thousands = (self.mya * 1000.0).round() as u32;
            format!("{thousands},000 years")
        } else if self.mya < 1.0 {
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
        let mut taxa: Vec<Taxon> = ROWS
            .iter()
            .map(|&(name, clade, mya, parent, group)| Taxon {
                name,
                clade,
                mya,
                parent: usize::try_from(parent).ok(),
                group,
                children: Vec::new(),
                depth: 0,
                eco: ecology::of(name),
            })
            .collect();
        for i in 0..taxa.len() {
            if let Some(p) = taxa[i].parent {
                taxa[p].children.push(i);
            }
        }
        // Depth from the root down: a node inserted later (appended so saves
        // keep their indices) can be the parent of an earlier row.
        let mut stack = vec![(Self::ROOT, 0u8)];
        while let Some((i, depth)) = stack.pop() {
            taxa[i].depth = depth;
            stack.extend(taxa[i].children.iter().map(|&c| (c, depth + 1)));
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
    fn ages_read_in_the_right_unit() {
        let phy = Phylogeny::load();
        let age = |name: &str| {
            phy.taxa
                .iter()
                .find(|t| t.name == name)
                .unwrap()
                .age_label()
        };
        assert_eq!(age("Urmetazoan"), "800 Ma");
        assert_eq!(age("Human"), "0.3 Ma");
        assert_eq!(age("Dog"), "20,000 years");
    }

    #[test]
    fn depth_counts_generations_from_the_root() {
        let phy = Phylogeny::load();
        for t in &phy.taxa {
            match t.parent {
                Some(p) => assert_eq!(t.depth, phy.taxa[p].depth + 1, "{}", t.name),
                None => assert_eq!(t.depth, 0),
            }
        }
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
