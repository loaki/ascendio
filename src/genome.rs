//! Rolling a new genome: which animals it can hold, card tiers, pity, morphs
//! and the specimen fallback. Pure logic over a seedable `Rng`.

use serde::{Deserialize, Serialize};

use crate::ecology::{self, Tier};
use crate::planet::Planet;
use crate::tree::Phylogeny;

/// xorshift64*: fair and reproducible, not cryptographic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Morph {
    None,
    Giant,
    Albino,
    Melanistic,
    Amber,
}

impl Morph {
    pub fn name(self) -> &'static str {
        match self {
            Morph::None => "",
            Morph::Giant => "Giant",
            Morph::Albino => "Albino",
            Morph::Melanistic => "Melanistic",
            Morph::Amber => "Amber",
        }
    }

    /// Keystone-bonus multiplier.
    pub fn strength(self) -> f32 {
        match self {
            Morph::None => 1.0,
            Morph::Amber => 2.0,
            _ => 1.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub taxon: usize,
    pub tier: Tier,
    pub morph: Morph,
    /// A first-ever discovery (full reveal) rather than a specimen.
    pub new: bool,
    /// Why this card is a specimen, when nothing new could live on the planet.
    pub note: Option<String>,
}

/// Genomes since each tier/morph last dropped: bad luck becomes a guarantee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pity {
    pub since_rare: u32,
    pub since_epic: u32,
    pub since_legendary: u32,
    pub since_morph: u32,
    pub ever_morphed: bool,
}

const RARE_PITY: u32 = 3;
const EPIC_PITY: u32 = 10;
pub const LEGENDARY_PITY: u32 = 40;
/// A fresh player's first morph arrives within this many genomes.
const FIRST_MORPH_PITY: u32 = 14;
pub const MAX_CARDS: usize = 6;

/// Everything that bends the odds for one genome: keystones + the chosen boon.
#[derive(Clone, Debug)]
pub struct Odds {
    pub cards: usize,
    /// Percentage points moved from Common to the rare tiers (capped at 15).
    pub luck: f32,
    pub morph_mult: f32,
    /// Weight multiplier per habitat, indexed by `Habitat::index`.
    pub affinity: [f32; 6],
    /// Taxa under this node get x3 weight (the Lure boon).
    pub lure: Option<usize>,
    /// At least one card is Rare+ (Catalyst boon).
    pub catalyst: bool,
    pub legendary_pity: u32,
    /// Morph guaranteed at least every this many genomes, once the
    /// first-morph guarantee is spent.
    pub morph_window: Option<u32>,
}

impl Default for Odds {
    fn default() -> Self {
        Self {
            cards: 3,
            luck: 0.0,
            morph_mult: 1.0,
            affinity: [1.0; 6],
            lure: None,
            catalyst: false,
            legendary_pity: LEGENDARY_PITY,
            morph_window: None,
        }
    }
}

fn is_under(phy: &Phylogeny, mut taxon: usize, ancestor: usize) -> bool {
    loop {
        if taxon == ancestor {
            return true;
        }
        match phy.taxa[taxon].parent {
            Some(p) => taxon = p,
            None => return false,
        }
    }
}

/// Undiscovered taxa whose parent is discovered and whose needs `planet` meets.
pub fn eligible(phy: &Phylogeny, found: &[bool], planet: &Planet) -> Vec<usize> {
    (0..phy.len())
        .filter(|&i| !found[i])
        .filter(|&i| phy.taxa[i].parent.is_some_and(|p| found[p]))
        .filter(|&i| ecology::of(phy.taxa[i].name).needs.met_by(planet))
        .collect()
}

/// Why the planet fails the first frontier taxon (parent found, itself not).
pub fn blocked_hint(phy: &Phylogeny, found: &[bool], planet: &Planet) -> Option<String> {
    (0..phy.len())
        .filter(|&i| !found[i] && phy.taxa[i].parent.is_some_and(|p| found[p]))
        .find_map(|i| {
            let why = ecology::of(phy.taxa[i].name).needs.missing(planet)?;
            Some(format!("Something is waiting to evolve: it {why}."))
        })
}

/// Percent chance of each tier, Common first, after `luck`.
pub fn tier_weights(luck: f32) -> [f32; 5] {
    let luck = luck.clamp(0.0, 15.0);
    let rare_total = 11.0 + 3.5 + 0.5;
    [
        60.0 - luck,
        25.0,
        11.0 + luck * 11.0 / rare_total,
        3.5 + luck * 3.5 / rare_total,
        0.5 + luck * 0.5 / rare_total,
    ]
}

fn roll_tier(rng: &mut Rng, luck: f32) -> Tier {
    let w = tier_weights(luck);
    let mut r = rng.unit() * w.iter().sum::<f32>();
    for (i, wi) in w.iter().enumerate() {
        if r < *wi {
            return Tier::ALL[i];
        }
        r -= wi;
    }
    Tier::Common
}

fn weight(phy: &Phylogeny, taxon: usize, planet: &Planet, odds: &Odds) -> f32 {
    let eco = ecology::of(phy.taxa[taxon].name);
    let aff = eco
        .needs
        .habitats
        .iter()
        .filter(|h| planet.has(**h))
        .map(|h| odds.affinity[h.index()])
        .fold(1.0_f32, f32::max);
    let lure = match odds.lure {
        Some(root) if is_under(phy, taxon, root) => 3.0,
        _ => 1.0,
    };
    aff * lure
}

/// Picks an eligible taxon at `tier`, falling back downward, then upward.
fn pick(
    phy: &Phylogeny,
    pool: &[usize],
    tier: Tier,
    planet: &Planet,
    odds: &Odds,
    rng: &mut Rng,
) -> Option<usize> {
    let at = |t: Tier| -> Vec<usize> {
        pool.iter()
            .copied()
            .filter(|&i| ecology::of(phy.taxa[i].name).tier == t)
            .collect()
    };
    let order: Vec<Tier> = Tier::ALL
        .iter()
        .rev()
        .copied()
        .filter(|&t| t <= tier)
        .chain(Tier::ALL.iter().copied().filter(|&t| t > tier))
        .collect();
    for t in order {
        let bucket = at(t);
        if bucket.is_empty() {
            continue;
        }
        let total: f32 = bucket.iter().map(|&i| weight(phy, i, planet, odds)).sum();
        let mut r = rng.unit() * total;
        for &i in &bucket {
            r -= weight(phy, i, planet, odds);
            if r <= 0.0 {
                return Some(i);
            }
        }
        return bucket.last().copied();
    }
    None
}

/// Odds of each morph per card at `morph_mult` 1 (a giant can be boosted).
const AMBER_ODDS: f32 = 1.0 / 512.0;
const ALBINO_ODDS: f32 = 1.0 / 64.0;
const MELANISTIC_ODDS: f32 = 1.0 / 64.0;
const GIANT_ODDS: f32 = 1.0 / 20.0;

/// The chance a card is any morph, ignoring the arthropod giant boost.
pub fn morph_chance(morph_mult: f32) -> f32 {
    morph_mult.clamp(0.0, 12.0) * (AMBER_ODDS + ALBINO_ODDS + MELANISTIC_ODDS + GIANT_ODDS)
}

fn roll_morph(phy: &Phylogeny, taxon: usize, planet: &Planet, odds: &Odds, rng: &mut Rng) -> Morph {
    let m = odds.morph_mult.clamp(0.0, 12.0);
    let arthropod = phy.taxa.iter().position(|t| t.name == "Arthropod");
    let giant_boost = if planet.oxygen >= 5 && arthropod.is_some_and(|a| is_under(phy, taxon, a)) {
        3.0
    } else {
        1.0
    };
    let r = rng.unit();
    let amber = m * AMBER_ODDS;
    let albino = amber + m * ALBINO_ODDS;
    let melanistic = albino + m * MELANISTIC_ODDS;
    let giant = melanistic + m * giant_boost * GIANT_ODDS;
    if r < amber {
        Morph::Amber
    } else if r < albino {
        Morph::Albino
    } else if r < melanistic {
        Morph::Melanistic
    } else if r < giant {
        Morph::Giant
    } else {
        Morph::None
    }
}

fn specimen(phy: &Phylogeny, found: &[bool], planet: &Planet, rng: &mut Rng) -> usize {
    let living: Vec<usize> = (0..phy.len())
        .filter(|&i| found[i] && ecology::of(phy.taxa[i].name).needs.met_by(planet))
        .collect();
    let owned: Vec<usize> = (0..phy.len()).filter(|&i| found[i]).collect();
    let from = if living.is_empty() { &owned } else { &living };
    from[(rng.next_u64() % from.len() as u64) as usize]
}

/// Rolls one genome against the collection before opening. Cards come back
/// best last (reveal order) and `pity` is advanced.
pub fn roll(
    phy: &Phylogeny,
    found: &[bool],
    planet: &Planet,
    odds: &Odds,
    pity: &mut Pity,
    rng: &mut Rng,
) -> Vec<Card> {
    let mut seen = found.to_vec();
    let count = odds.cards.clamp(1, MAX_CARDS);
    let mut cards = Vec::with_capacity(count);

    for k in 0..count {
        let last = k == count - 1;
        let mut tier = roll_tier(rng, odds.luck);
        let best = cards
            .iter()
            .map(|c: &Card| c.tier)
            .max()
            .unwrap_or(Tier::Common);
        let mut floor = Tier::Common;
        if last {
            if (odds.catalyst || pity.since_rare + 1 >= RARE_PITY) && best < Tier::Rare {
                floor = floor.max(Tier::Rare);
            }
            if pity.since_epic + 1 >= EPIC_PITY && best < Tier::Epic {
                floor = floor.max(Tier::Epic);
            }
            if pity.since_legendary + 1 >= odds.legendary_pity && best < Tier::Legendary {
                floor = floor.max(Tier::Legendary);
            }
        }
        let pool = eligible(phy, &seen, planet);
        // Only lift the roll if something that rare can drop; otherwise the
        // pity keeps waiting.
        if tier < floor
            && pool
                .iter()
                .any(|&i| ecology::of(phy.taxa[i].name).tier >= floor)
        {
            tier = floor;
        }

        let card = match pick(phy, &pool, tier, planet, odds, rng) {
            Some(i) => {
                seen[i] = true;
                Card {
                    taxon: i,
                    tier: ecology::of(phy.taxa[i].name).tier,
                    morph: Morph::None,
                    new: true,
                    note: None,
                }
            }
            None => {
                let i = specimen(phy, &seen, planet, rng);
                Card {
                    taxon: i,
                    tier: ecology::of(phy.taxa[i].name).tier,
                    morph: Morph::None,
                    new: false,
                    note: blocked_hint(phy, &seen, planet)
                        .or_else(|| Some("Nothing new could evolve here.".into())),
                }
            }
        };
        cards.push(card);
    }

    for c in cards.iter_mut() {
        c.morph = roll_morph(phy, c.taxon, planet, odds, rng);
    }
    let morph_window = if pity.ever_morphed {
        odds.morph_window
    } else {
        Some(FIRST_MORPH_PITY)
    };
    if cards.iter().all(|c| c.morph == Morph::None)
        && morph_window.is_some_and(|w| pity.since_morph + 1 >= w)
    {
        if let Some(c) = cards.last_mut() {
            c.morph = Morph::Giant;
        }
    }

    // Best last: tier, then a morph, then novelty.
    cards.sort_by_key(|c| (c.tier, c.morph != Morph::None, c.new));

    let top = tell(&cards);
    let bump = |since: u32, tier: Tier| if top >= tier { 0 } else { since + 1 };
    pity.since_rare = bump(pity.since_rare, Tier::Rare);
    pity.since_epic = bump(pity.since_epic, Tier::Epic);
    pity.since_legendary = bump(pity.since_legendary, Tier::Legendary);
    if cards.iter().any(|c| c.morph != Morph::None) {
        pity.since_morph = 0;
        pity.ever_morphed = true;
    } else {
        pity.since_morph += 1;
    }
    cards
}

/// The best tier in a genome.
pub fn tell(cards: &[Card]) -> Tier {
    cards.iter().map(|c| c.tier).max().unwrap_or(Tier::Common)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> (Phylogeny, Vec<bool>) {
        let phy = Phylogeny::load();
        let mut found = vec![false; phy.len()];
        found[Phylogeny::ROOT] = true;
        (phy, found)
    }

    fn idx(phy: &Phylogeny, name: &str) -> usize {
        phy.taxa.iter().position(|t| t.name == name).unwrap()
    }

    #[test]
    fn rng_is_reproducible_and_in_range() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        for _ in 0..1000 {
            let x = a.unit();
            assert_eq!(x, b.unit());
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn the_first_genome_holds_the_roots_children() {
        let (phy, found) = start();
        let planet = Planet {
            oxygen: 2,
            ..Planet::default()
        };
        let cards = roll(
            &phy,
            &found,
            &planet,
            &Odds::default(),
            &mut Pity::default(),
            &mut Rng::new(1),
        );
        assert_eq!(cards.len(), 3);
        assert!(cards.iter().all(|c| c.new));
    }

    #[test]
    fn cards_only_ever_hold_what_the_tree_and_planet_allow() {
        let (phy, mut found) = start();
        let mut rng = Rng::new(42);
        let mut pity = Pity::default();
        let planet = Planet {
            land: 3,
            vegetation: 4,
            oxygen: 4,
            temperature: 3,
            volcanism: 0,
        };
        for _ in 0..60 {
            let cards = roll(&phy, &found, &planet, &Odds::default(), &mut pity, &mut rng);
            for c in &cards {
                if c.new {
                    let t = &phy.taxa[c.taxon];
                    assert!(!found[c.taxon], "{} was already found", t.name);
                    assert!(
                        t.parent
                            .is_some_and(|p| found[p] || cards.iter().any(|o| o.taxon == p)),
                        "{} skipped its parent",
                        t.name
                    );
                    assert!(
                        ecology::of(t.name).needs.met_by(&planet),
                        "{} can't live here",
                        t.name
                    );
                }
            }
            for c in &cards {
                found[c.taxon] = true;
            }
        }
    }

    #[test]
    fn a_water_world_never_yields_land_animals() {
        let (phy, _) = start();
        let found = vec![true; phy.len()]
            .iter()
            .enumerate()
            .map(|(i, _)| phy.taxa[i].name != "Mouse")
            .collect::<Vec<_>>();
        let planet = Planet {
            oxygen: 4,
            ..Planet::default()
        };
        let cards = roll(
            &phy,
            &found,
            &planet,
            &Odds::default(),
            &mut Pity::default(),
            &mut Rng::new(3),
        );
        assert!(cards.iter().all(|c| !c.new), "a mouse can't evolve at sea");
        assert!(cards[0].note.as_deref().unwrap_or("").contains("land"));
    }

    #[test]
    fn best_card_is_revealed_last() {
        let (phy, found) = start();
        let planet = Planet {
            oxygen: 3,
            ..Planet::default()
        };
        let mut rng = Rng::new(9);
        for _ in 0..50 {
            let cards = roll(
                &phy,
                &found,
                &planet,
                &Odds {
                    cards: 5,
                    ..Odds::default()
                },
                &mut Pity::default(),
                &mut rng,
            );
            for w in cards.windows(2) {
                assert!(w[0].tier <= w[1].tier);
            }
            assert_eq!(tell(&cards), cards.last().unwrap().tier);
        }
    }

    #[test]
    fn rare_pity_fires_on_the_third_dry_genome() {
        let (phy, mut found) = start();
        // Open the deuterostome line so a Rare (Vertebrate) is reachable.
        for n in ["Bilaterian", "Deuterostome", "Chordate"] {
            found[idx(&phy, n)] = true;
        }
        let planet = Planet {
            oxygen: 3,
            ..Planet::default()
        };
        let mut pity = Pity {
            since_rare: RARE_PITY - 1,
            ..Pity::default()
        };
        let cards = roll(
            &phy,
            &found,
            &planet,
            &Odds::default(),
            &mut pity,
            &mut Rng::new(5),
        );
        assert!(tell(&cards) >= Tier::Rare);
        assert_eq!(pity.since_rare, 0);
    }

    #[test]
    fn pity_waits_when_nothing_that_rare_can_drop() {
        let (phy, found) = start();
        let planet = Planet {
            oxygen: 2,
            ..Planet::default()
        };
        let mut pity = Pity {
            since_legendary: LEGENDARY_PITY - 1,
            ..Pity::default()
        };
        let cards = roll(
            &phy,
            &found,
            &planet,
            &Odds::default(),
            &mut pity,
            &mut Rng::new(5),
        );
        assert!(tell(&cards) < Tier::Legendary);
        assert_eq!(
            pity.since_legendary, LEGENDARY_PITY,
            "the counter keeps waiting"
        );
    }

    #[test]
    fn a_first_morph_is_guaranteed() {
        let (phy, found) = start();
        let planet = Planet {
            oxygen: 2,
            ..Planet::default()
        };
        let mut pity = Pity {
            since_morph: FIRST_MORPH_PITY - 1,
            ..Pity::default()
        };
        let cards = roll(
            &phy,
            &found,
            &planet,
            &Odds {
                morph_mult: 0.0,
                ..Odds::default()
            },
            &mut pity,
            &mut Rng::new(2),
        );
        assert!(cards.iter().any(|c| c.morph != Morph::None));
        assert!(pity.ever_morphed);
    }

    #[test]
    fn tier_rates_roughly_match_the_table() {
        let mut rng = Rng::new(1234);
        let mut n = [0u32; 5];
        let trials = 200_000;
        for _ in 0..trials {
            n[roll_tier(&mut rng, 0.0).index()] += 1;
        }
        let pct = |i: usize| n[i] as f32 * 100.0 / trials as f32;
        assert!((pct(0) - 60.0).abs() < 1.0);
        assert!((pct(2) - 11.0).abs() < 0.5);
        assert!((pct(4) - 0.5).abs() < 0.1);
    }

    #[test]
    fn luck_moves_odds_from_common_to_rare_and_is_capped() {
        let w0 = tier_weights(0.0);
        let w = tier_weights(100.0);
        assert!(w[0] < w0[0] && w[4] > w0[4]);
        assert!((w.iter().sum::<f32>() - 100.0).abs() < 1e-3);
        assert_eq!(w, tier_weights(15.0));
    }

    #[test]
    fn a_lure_pulls_its_clade() {
        let (phy, mut found) = start();
        for n in [
            "Bilaterian",
            "Protostome",
            "Deuterostome",
            "Ecdysozoan",
            "Spiralian",
            "Echinoderm",
        ] {
            found[idx(&phy, n)] = true;
        }
        let planet = Planet {
            oxygen: 3,
            land: 1,
            ..Planet::default()
        };
        let ecd = idx(&phy, "Ecdysozoan");
        let (mut lured, mut plain) = (0, 0);
        for s in 0..400 {
            let odds = Odds {
                lure: Some(ecd),
                cards: 1,
                ..Odds::default()
            };
            let c = &roll(
                &phy,
                &found,
                &planet,
                &odds,
                &mut Pity::default(),
                &mut Rng::new(s + 1),
            )[0];
            lured += is_under(&phy, c.taxon, ecd) as u32;
            let c = &roll(
                &phy,
                &found,
                &planet,
                &Odds {
                    cards: 1,
                    ..Odds::default()
                },
                &mut Pity::default(),
                &mut Rng::new(s + 1),
            )[0];
            plain += is_under(&phy, c.taxon, ecd) as u32;
        }
        assert!(lured > plain, "lure {lured} vs plain {plain}");
    }
}
