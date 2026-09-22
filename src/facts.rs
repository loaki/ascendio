//! "Did you know?" trivia shown in the HUD while the pool is charging.
//!
//! Purely passive: no tap, no reward, no effect on the economy. The pacing
//! this session is built around (see `game.rs`'s module doc) comes from a
//! logarithmic wait, and anything that lets a tap shorten it would quietly
//! undo that. This is something to read instead, not a mechanic.
//!
//! Every entry is a well-established, textbook-level fact about animal
//! evolution or the phylogeny this game is built from -- general knowledge,
//! not a specific claim that would need its own citation the way the tree's
//! topology does in `docs/PHYLOGENY.md`.

#[rustfmt::skip]
pub const FACTS: &[&str] = &[
    "Sponges have no true tissues or organs -- just specialised cells.",
    "Comb jellies swim using cilia, not muscle -- the largest animals that do.",
    "A jellyfish has no brain, heart, or bones, only a simple nerve net.",
    "Placozoans are among the simplest known animals: a few thousand cells, no organs at all.",
    "Octopuses have three hearts and blue, copper-based blood.",
    "A snail's shell grows in a logarithmic spiral, the same curve this game's coil uses.",
    "Earthworms have five pairs of \"hearts\" -- aortic arches that pump their blood.",
    "Horseshoe crabs are older than trees: their lineage predates land plants.",
    "Scorpions glow under ultraviolet light, for reasons still debated.",
    "A dragonfly can see nearly 360 degrees with up to 30,000 lenses per eye.",
    "Beetles make up roughly 1 in every 4 described animal species on Earth.",
    "A butterfly tastes with its feet.",
    "Ants have no lungs -- oxygen diffuses through tiny holes in their exoskeleton.",
    "Starfish have no brain, and can regrow an entire arm -- some, a whole body from one arm.",
    "Sea urchins have no eyes, but their whole body appears to sense light.",
    "A lamprey's mouth is a ring of teeth -- it has no jaw at all.",
    "Sharks have skeletons made of cartilage, not bone.",
    "Coelacanths were thought extinct for 66 million years until one was caught in 1938.",
    "A frog breathes partly through its skin, even underwater.",
    "Turtles are the only reptiles whose shoulder blades sit inside their ribcage.",
    "Crocodiles have the strongest bite ever measured in a living animal.",
    "Birds are, technically, living dinosaurs -- descended from theropods like the ones in the ground below.",
    "The platypus is one of only five living mammal species that lays eggs.",
    "A kangaroo cannot walk backwards.",
    "An elephant's trunk has no bones -- over 40,000 muscles, and no skeleton at all.",
    "A mouse's heart beats around 600 times a minute.",
    "Bats are the only mammals capable of sustained, powered flight.",
    "A wolf pack's howl can be heard from up to 10 kilometres away.",
    "A blue whale's heart alone can weigh as much as a small car.",
    "Lemurs are found nowhere in the wild except Madagascar.",
    "Chimpanzees share about 98.8% of their DNA with humans.",
    "Humans and chimpanzees split from a common ancestor roughly 6-7 million years ago.",
    "The five mass extinctions in Earth's history each wiped out most animal species alive at the time.",
    "Every land vertebrate, including humans, descends from lobe-finned fish.",
    "\"Fish\" is not a single evolutionary group -- a shark is more distantly related to a tuna than a tuna is to a human.",
    "The word \"phylogeny\" -- Greek for \"tribe\" and \"origin\" -- means the evolutionary history of a lineage.",
    "A cladogram, like the map in this game, shows relationships, not a straight timeline.",
    "Convergent evolution is why dolphins and sharks look alike despite a 400-million-year-old split.",
    "Bilateral symmetry -- a left and right side -- appeared once and is shared by over 99% of animal species.",
    "The most recent common ancestor of all animals alive today lived roughly 600-800 million years ago.",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_enough_facts_that_rotation_does_not_repeat_too_soon() {
        assert!(FACTS.len() >= 20, "only {} facts", FACTS.len());
    }

    #[test]
    fn no_fact_is_empty_or_absurdly_long() {
        for f in FACTS {
            assert!(!f.is_empty());
            // Not a hard render limit -- draw_fact_banner wraps to two lines
            // and shrinks the font -- just a sanity bound so a runaway string
            // does not slip in unnoticed.
            assert!(f.len() < 140, "too long ({} chars): {f}", f.len());
        }
    }

    #[test]
    fn no_fact_is_duplicated() {
        for (i, a) in FACTS.iter().enumerate() {
            for b in &FACTS[i + 1..] {
                assert_ne!(a, b, "duplicate fact: {a}");
            }
        }
    }
}
