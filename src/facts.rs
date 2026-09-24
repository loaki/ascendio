//! "Did you know?" trivia shown in the lever panel while time runs.
//! Textbook-level facts only; the tree's own sources are in `docs/PHYLOGENY.md`.

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
    "A nautilus floats by adjusting the gas in its chambered shell.",
    "A fly's hind wings have shrunk into halteres, tiny balancing organs.",
    "Every clownfish is born male; the dominant one of a group turns female.",
    "Lungfish are our closest living fish relatives and can survive years dried in mud.",
    "Pigs can't sweat, so they wallow in mud to keep cool.",
    "The tuatara is not a lizard but the last survivor of a Triassic order.",
    "Female sea turtles return to lay eggs on the beach where they hatched.",
    "Plesiosaurs flew underwater on four flippers and gave birth to live young.",
    "Ichthyosaurs were dolphin-shaped reptiles with some of the largest eyes ever.",
    "Triceratops lived alongside T. rex at the very end of the Cretaceous.",
    "Velociraptor was turkey-sized and feathered: its arm bones carry quill knobs.",
    "Archaeopteryx, the first \"bird\", was found in 1861, two years after Darwin's Origin.",
    "An ostrich lays the largest egg of any living bird, and its eye is bigger than its brain.",
    "Parrots' closest relatives are falcons, and some parrots live more than 80 years.",
    "Serrated flight feathers make owls almost silent in flight.",
    "Koalas eat almost nothing but eucalyptus leaves and sleep up to 20 hours a day.",
    "Green algae grow in a sloth's fur; extinct ground sloths were elephant-sized.",
    "A giraffe's neck has seven vertebrae, the same number as yours.",
    "The hippo's closest living relatives are whales and dolphins.",
    "A rhino's horn is keratin, the same stuff as your fingernails.",
    "The giant panda is a true bear, the first branch of the bear family.",
    "Seals, sea lions and walruses descend from land carnivores related to bears.",
    "Mammals are synapsids: our line split from the reptiles' about 320 million years ago.",
    "Crocodiles are closer kin to birds than to lizards: both are archosaurs.",
    "Birds are theropod dinosaurs, the same group as T. rex.",
    "Our lineage split from the chimpanzees' about 7 million years ago.",
    "Horses, rhinos and tapirs are the odd-toed hoofed mammals.",
    "Spiny-rayed fishes make up about a third of all vertebrate species.",
    "Elephants can hear through their feet, picking up rumbles in the ground.",
    "The blue whale is the largest animal that has ever lived.",
    "Wolves hunt in family packs led by a breeding pair.",
    "Dogs were the first domesticated animals, tamed from wolves over 15,000 years ago.",
    "A newborn kangaroo is the size of a jellybean and crawls into its mother's pouch.",
    "No two tigers share the same stripes, and the stripes are on their skin too.",
    "House cats descend from the African wildcat, tamed about 10,000 years ago.",
    "Smilodon's sabre teeth grew up to 28 cm long.",
    "A giant panda spends up to 14 hours a day eating bamboo.",
    "A cow's stomach has four chambers to break down grass.",
    "There are more chickens on Earth than any other bird.",
    "More than half of all bird species are perching birds.",
    "The dodo was a giant flightless pigeon, gone by the late 1600s.",
    "Aphids, cicadas and shield bugs are true bugs: all of them sip through a beak.",
    "Ants and bees both evolved from wasps.",
    "Mites and ticks are arachnids, and some mites live in human skin.",
    "Termites are social cockroaches, and a queen can live for decades.",
    "A moss animal colony is thousands of tiny clones, each with a crown of tentacles.",
    "Brittle stars walk by snaking their arms, and can drop one to escape.",
    "Carp, catfish and piranhas are cousins: two-thirds of all freshwater fish.",
    "Traces of cholesterol prove that Dickinsonia, 558 million years old, was an animal.",
    "Dimetrodon was not a dinosaur but a distant relative of mammals.",
    "Lucy walked upright 3.2 million years ago with a brain the size of a chimp's.",
    "Most people alive today carry a little Neanderthal DNA.",
    "Pakicetus, an early whale, walked on four legs and waded in rivers.",
    "Acanthostega had eight fingers per hand, and legs it used only in water.",
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
