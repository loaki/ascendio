//! The animals' names in French. The English name in `tree.rs` stays the
//! key (ecology, sprites, saves); this is only what the player reads.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

/// The language species names are shown in (the rest stays English).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Language {
    #[default]
    English,
    French,
}

static FRENCH: AtomicBool = AtomicBool::new(false);

/// Every name shown from now on is in `lang`.
pub fn set(lang: Language) {
    FRENCH.store(lang == Language::French, Ordering::Relaxed);
}

pub fn french() -> bool {
    FRENCH.load(Ordering::Relaxed)
}

/// Sorts names alphabetically whatever their accents: `Éléphant` with the E.
pub fn sort_key(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// The French name of the taxon called `english` in `tree.rs`; the English
/// one if it has none (a test makes sure every taxon has one). The font has
/// no `œ`, so `oe` stands in for it.
pub fn of(english: &'static str) -> &'static str {
    match english {
        "Urmetazoan" => "Urmétazoaire",
        "Sea Sponge" => "Éponge",
        "Comb Jelly" => "Cténophore",
        "Placozoan" => "Placozoaire",
        "Cnidarian" => "Cnidaire",
        "Jellyfish" => "Méduse",
        "Coral" => "Corail",
        "Bilaterian" => "Bilatérien",
        "Acoel Worm" => "Ver acoele",
        "Protostome" => "Protostomien",
        "Deuterostome" => "Deutérostomien",
        "Spiralian" => "Spiralien",
        "Ecdysozoan" => "Ecdysozoaire",
        "Flatworm" => "Ver plat",
        "Mollusc" => "Mollusque",
        "Snail" => "Escargot",
        "Octopus" => "Pieuvre",
        "Segmented Worm" => "Ver annelé",
        "Nematode" => "Nématode",
        "Arthropod" => "Arthropode",
        "Chelicerate" => "Chélicérate",
        "Horseshoe Crab" => "Limule",
        "Scorpion" => "Scorpion",
        "Spider" => "Araignée",
        "Pancrustacean" => "Pancrustacé",
        "Crab" => "Crabe",
        "Insect" => "Insecte",
        "Dragonfly" => "Libellule",
        "Beetle" => "Scarabée",
        "Butterfly" => "Papillon",
        "Ant" => "Fourmi",
        "Echinoderm" => "Échinoderme",
        "Starfish" => "Étoile de mer",
        "Sea Urchin" => "Oursin",
        "Chordate" => "Chordé",
        "Sea Squirt" => "Ascidie",
        "Vertebrate" => "Vertébré",
        "Lamprey" => "Lamproie",
        "Jawed Fish" => "Gnathostome",
        "Shark" => "Requin",
        "Ray-finned Fish" => "Actinoptérygien",
        "Lobe-finned Fish" => "Sarcoptérygien",
        "Coelacanth" => "Coelacanthe",
        "Tetrapod" => "Tétrapode",
        "Amphibian" => "Amphibien",
        "Frog" => "Grenouille",
        "Amniote" => "Amniote",
        "Mammal" => "Mammifère",
        "Platypus" => "Ornithorynque",
        "Marsupial" => "Marsupial",
        "Placental" => "Placentaire",
        "Afrothere" => "Afrothérien",
        "Mouse" => "Souris",
        "Bat" => "Chauve-souris",
        "Carnivoran" => "Carnivore",
        "Cetartiodactyl" => "Cétartiodactyle",
        "Primate" => "Primate",
        "Lemur" => "Lémurien",
        "Monkey" => "Singe",
        "Ape" => "Grand singe",
        "Chimpanzee" => "Chimpanzé",
        "Human" => "Humain",
        "Reptile" => "Reptile",
        "Turtle" => "Tortue",
        "Lizard" => "Lézard",
        "Crocodile" => "Crocodile",
        "Dinosaur" => "Dinosaure",
        "Bird" => "Oiseau",
        "Clam" => "Palourde",
        "Squid" => "Calmar",
        "Ammonite" => "Ammonite",
        "Earthworm" => "Ver de terre",
        "Tardigrade" => "Tardigrade",
        "Trilobite" => "Trilobite",
        "Anomalocaris" => "Anomalocaris",
        "Centipede" => "Mille-pattes",
        "Shrimp" => "Crevette",
        "Bee" => "Abeille",
        "Meganeura" => "Meganeura",
        "Dunkleosteus" => "Dunkleosteus",
        "Stingray" => "Raie",
        "Megalodon" => "Mégalodon",
        "Seahorse" => "Hippocampe",
        "Anglerfish" => "Baudroie",
        "Tiktaalik" => "Tiktaalik",
        "Salamander" => "Salamandre",
        "Snake" => "Serpent",
        "Pterosaur" => "Ptérosaure",
        "T. rex" => "T. rex",
        "Sauropod" => "Sauropode",
        "Penguin" => "Manchot",
        "Mammoth" => "Mammouth",
        "Rabbit" => "Lapin",
        "Horse" => "Cheval",
        "Dolphin" => "Dauphin",
        "Lion" => "Lion",
        "Gorilla" => "Gorille",
        "Nautilus" => "Nautile",
        "Fly" => "Mouche",
        "Clownfish" => "Poisson-clown",
        "Lungfish" => "Dipneuste",
        "Pig" => "Cochon",
        "Tuatara" => "Sphénodon",
        "Sea Turtle" => "Tortue marine",
        "Plesiosaur" => "Plésiosaure",
        "Ichthyosaur" => "Ichtyosaure",
        "Triceratops" => "Tricératops",
        "Velociraptor" => "Vélociraptor",
        "Archaeopteryx" => "Archéoptéryx",
        "Ostrich" => "Autruche",
        "Parrot" => "Perroquet",
        "Owl" => "Chouette",
        "Koala" => "Koala",
        "Sloth" => "Paresseux",
        "Giraffe" => "Girafe",
        "Hippo" => "Hippopotame",
        "Rhino" => "Rhinocéros",
        "Bear" => "Ours",
        "Seal" => "Phoque",
        "Synapsid" => "Synapside",
        "Archosaur" => "Archosaure",
        "Theropod" => "Théropode",
        "Hominin" => "Hominine",
        "Perissodactyl" => "Périssodactyle",
        "Spiny-rayed Fish" => "Acanthomorphe",
        "Elephant" => "Éléphant",
        "Whale" => "Baleine",
        "Wolf" => "Loup",
        "Dog" => "Chien",
        "Kangaroo" => "Kangourou",
        "Tiger" => "Tigre",
        "Cat" => "Chat",
        "Sabre-tooth" => "Smilodon",
        "Giant Panda" => "Panda géant",
        "Cow" => "Vache",
        "Chicken" => "Poule",
        "Songbird" => "Passereau",
        "Dodo" => "Dodo",
        "True Bug" => "Punaise",
        "Wasp" => "Guêpe",
        "Mite" => "Acarien",
        "Termite" => "Termite",
        "Moss Animal" => "Bryozoaire",
        "Brittle Star" => "Ophiure",
        "Carp" => "Carpe",
        "Dickinsonia" => "Dickinsonia",
        "Dimetrodon" => "Dimétrodon",
        "Lucy" => "Lucy",
        "Neanderthal" => "Néandertalien",
        "Pakicetus" => "Pakicetus",
        "Acanthostega" => "Acanthostega",
        _ => english,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Phylogeny;

    /// A name the same in both languages is fine only for these: proper
    /// names, genera and words French shares.
    const SAME: &[&str] = &[
        "Scorpion",
        "Amniote",
        "Marsupial",
        "Primate",
        "Reptile",
        "Crocodile",
        "Ammonite",
        "Tardigrade",
        "Trilobite",
        "Anomalocaris",
        "Meganeura",
        "Dunkleosteus",
        "Tiktaalik",
        "T. rex",
        "Lion",
        "Koala",
        "Dodo",
        "Termite",
        "Dickinsonia",
        "Lucy",
        "Pakicetus",
        "Acanthostega",
    ];

    #[test]
    fn every_taxon_has_a_french_name_the_font_can_draw() {
        let phy = Phylogeny::load();
        for t in &phy.taxa {
            let fr = of(t.name);
            assert!(
                fr != t.name || SAME.contains(&t.name),
                "{} has no French name",
                t.name
            );
            assert!(fr.chars().count() <= 16, "{fr} is too long for a label");
            // ProggyClean covers Latin-1 only.
            assert!(
                fr.chars().all(|c| (c as u32) < 0x100),
                "{fr} has a glyph the font lacks"
            );
        }
    }

    #[test]
    fn accents_sort_with_their_letter() {
        assert!(sort_key("Éléphant") < sort_key("Fourmi"));
        assert!(sort_key("Échinoderme") > sort_key("Dodo"));
    }
}
