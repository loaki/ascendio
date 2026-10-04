//! Rolling a new genome: which animals it can hold, card tiers, morphs
//! and the specimen fallback. Pure logic over a seedable `Rng`.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::ecology::Tier;
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
    /// Every real morph, rarest last.
    pub const ALL: [Morph; 4] = [Morph::Giant, Morph::Albino, Morph::Melanistic, Morph::Amber];

    /// Its flag in a taxon's owned-morphs byte (`Game::morphs`); 0 for none.
    pub fn bit(self) -> u8 {
        match self {
            Morph::None => 0,
            Morph::Giant => 1,
            Morph::Albino => 2,
            Morph::Melanistic => 4,
            Morph::Amber => 8,
        }
    }

    /// The morph a single flag stands for, `None` for anything else.
    pub fn from_bit(bit: u8) -> Morph {
        Self::ALL
            .into_iter()
            .find(|m| m.bit() == bit)
            .unwrap_or(Morph::None)
    }

    pub fn name(self) -> &'static str {
        match self {
            Morph::None => "",
            Morph::Giant => "Giant",
            Morph::Albino => "Albino",
            Morph::Melanistic => "Melanistic",
            Morph::Amber => "Amber",
        }
    }

    /// Keystone-bonus multiplier, like a card edition. Giant and
    /// Melanistic pay another way: an adjustment point, and any
    /// temperature.
    pub fn strength(self) -> f32 {
        match self {
            Morph::None | Morph::Giant | Morph::Melanistic => 1.0,
            Morph::Albino => 1.5,
            Morph::Amber => 2.0,
        }
    }

    /// How good it is as a keystone, all else equal, for the BEST pick.
    pub fn rank(self) -> u8 {
        match self {
            Morph::None => 0,
            Morph::Melanistic => 1,
            Morph::Giant => 2,
            Morph::Albino => 3,
            Morph::Amber => 4,
        }
    }

    /// What it does to a keystone, in a line.
    pub fn effect(self) -> &'static str {
        match self {
            Morph::None => "No morph",
            Morph::Giant => "+1 point per 4h waited",
            Morph::Albino => "x1.5, asleep above Cool",
            Morph::Melanistic => "Ignores temperature",
            Morph::Amber => "x2 its bonus",
        }
    }
}

/// Above this temperature an albino keystone sunburns and sleeps.
pub const ALBINO_MAX_TEMP: u8 = 2;
/// The most a card can be a morph, however keystones, boons, the wait and
/// radiation stack: morphs stay rare.
pub const MORPH_CHANCE_MAX: f32 = 0.15;

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

/// Everything that bends the odds for one genome: the wait, keystones and
/// the chosen boon.
#[derive(Clone, Debug)]
pub struct Odds {
    pub cards: usize,
    /// Luck: points moved from Common to the rare tiers (keystones up to
    /// 20, plus the wait and radiation; 50 at most).
    pub luck: f32,
    pub morph_mult: f32,
    /// The chance each card is a species never found (else one you own).
    pub discovery: f32,
    /// At least one card is Rare+ (a 6h wait, the Catalyst boon).
    pub catalyst: bool,
    /// At least one card is Epic+ (Catalyst on a 6h wait).
    pub sure_epic: bool,
    /// Radiation's morph multiplier, applied past the usual cap.
    pub morph_rad: f32,
}

impl Default for Odds {
    fn default() -> Self {
        Self {
            cards: 3,
            luck: 0.0,
            morph_mult: 1.0,
            discovery: 1.0,
            catalyst: false,
            sure_epic: false,
            morph_rad: 1.0,
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
        .filter(|&i| phy.taxa[i].eco.needs.met_by(planet))
        .collect()
}

/// Why the planet fails the first frontier taxon (parent found, itself not).
pub fn blocked_hint(phy: &Phylogeny, found: &[bool], planet: &Planet) -> Option<String> {
    (0..phy.len())
        .filter(|&i| !found[i] && phy.taxa[i].parent.is_some_and(|p| found[p]))
        .find_map(|i| {
            let why = phy.taxa[i].eco.needs.missing(planet)?;
            Some(format!("Something is waiting to evolve: it {why}."))
        })
}

/// Percent chance of each tier, Common first, after `luck`.
pub fn tier_weights(luck: f32) -> [f32; 5] {
    // Common never drops below 10%, whatever the luck.
    let luck = luck.clamp(0.0, 50.0);
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

/// How likely `taxon` is against the others of its tier: its best habitat's
/// share of the planet.
fn weight(phy: &Phylogeny, taxon: usize, planet: &Planet) -> f32 {
    let eco = phy.taxa[taxon].eco;
    let shares = planet.habitat_shares();
    let aff = eco
        .needs
        .habitats
        .iter()
        .filter(|h| planet.has(**h))
        .map(|h| shares[h.index()].max(0.01))
        .fold(0.01_f32, f32::max);
    aff
}

/// Picks a taxon of `pool` at `tier`, falling back downward, then upward.
fn pick(
    phy: &Phylogeny,
    pool: &[usize],
    tier: Tier,
    planet: &Planet,
    rng: &mut Rng,
) -> Option<usize> {
    let at = |t: Tier| -> Vec<usize> {
        pool.iter()
            .copied()
            .filter(|&i| phy.taxa[i].eco.tier == t)
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
        let total: f32 = bucket.iter().map(|&i| weight(phy, i, planet)).sum();
        let mut r = rng.unit() * total;
        for &i in &bucket {
            r -= weight(phy, i, planet);
            if r <= 0.0 {
                return Some(i);
            }
        }
        return bucket.last().copied();
    }
    None
}

/// Odds of each morph per card at `morph_mult` 1 (a giant can be boosted).
const AMBER_ODDS: f32 = 1.0 / 1024.0;
const ALBINO_ODDS: f32 = 1.0 / 128.0;
const MELANISTIC_ODDS: f32 = 1.0 / 128.0;
const GIANT_ODDS: f32 = 1.0 / 40.0;

/// The chance a card is any morph (ignoring the arthropod giant boost),
/// capped at `MORPH_CHANCE_MAX`.
pub fn morph_chance(morph_mult: f32, morph_rad: f32) -> f32 {
    let m = morph_mult.max(0.0) * morph_rad;
    (m * (AMBER_ODDS + ALBINO_ODDS + MELANISTIC_ODDS + GIANT_ODDS)).min(MORPH_CHANCE_MAX)
}

fn roll_morph(phy: &Phylogeny, taxon: usize, planet: &Planet, odds: &Odds, rng: &mut Rng) -> Morph {
    let m = odds.morph_mult.max(0.0) * odds.morph_rad;
    let arthropod = phy.taxa.iter().position(|t| t.name == "Arthropod");
    let giant_boost = if planet.oxygen >= 5 && arthropod.is_some_and(|a| is_under(phy, taxon, a)) {
        3.0
    } else {
        1.0
    };
    // Every morph scaled down together past the cap, so their mix holds.
    let total = m * (AMBER_ODDS + ALBINO_ODDS + MELANISTIC_ODDS + giant_boost * GIANT_ODDS);
    let m = if total > MORPH_CHANCE_MAX {
        m * MORPH_CHANCE_MAX / total
    } else {
        m
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

/// The animals a duplicate card can be: those owned that live on the
/// planet, or any owned if none does.
fn owned_pool(phy: &Phylogeny, found: &[bool], planet: &Planet) -> Vec<usize> {
    let living: Vec<usize> = (0..phy.len())
        .filter(|&i| found[i] && phy.taxa[i].eco.needs.met_by(planet))
        .collect();
    if living.is_empty() {
        (0..phy.len()).filter(|&i| found[i]).collect()
    } else {
        living
    }
}

/// The animals of `pool` at least `floor` rare: the whole pool, as is, when
/// there is no floor.
fn reaching<'a>(phy: &Phylogeny, pool: &'a [usize], floor: Tier) -> Cow<'a, [usize]> {
    if floor == Tier::Common {
        return Cow::Borrowed(pool);
    }
    pool.iter()
        .copied()
        .filter(|&i| phy.taxa[i].eco.tier >= floor)
        .collect()
}

/// Rolls one genome against the collection before opening. Each card rolls
/// a rarity, then whether it's a new species (`Odds::discovery`) or one
/// already owned, which levels it up. Cards come back best last (reveal
/// order).
pub fn roll(
    phy: &Phylogeny,
    found: &[bool],
    planet: &Planet,
    odds: &Odds,
    rng: &mut Rng,
) -> Vec<Card> {
    let mut seen = found.to_vec();
    let owned = owned_pool(phy, found, planet);
    let count = odds.cards.max(1);
    let mut cards = Vec::with_capacity(count);

    for k in 0..count {
        let last = k == count - 1;
        let mut tier = roll_tier(rng, odds.luck);
        let best = cards
            .iter()
            .map(|c: &Card| c.tier)
            .max()
            .unwrap_or(Tier::Common);
        let fresh = eligible(phy, &seen, planet);
        let can_drop = |t: Tier| {
            fresh
                .iter()
                .chain(&owned)
                .any(|&i| phy.taxa[i].eco.tier >= t)
        };
        // The last card makes good a sure Rare (a 6h wait, Catalyst) or a
        // sure Epic (Catalyst on a 6h wait) the others didn't bring, as far
        // as something that rare can drop, new or owned: an Epic out of
        // reach still keeps the Rare.
        let mut floor = Tier::Common;
        if last {
            if odds.sure_epic && best < Tier::Epic && can_drop(Tier::Epic) {
                floor = Tier::Epic;
            } else if (odds.catalyst || odds.sure_epic) && best < Tier::Rare && can_drop(Tier::Rare)
            {
                floor = Tier::Rare;
            }
        }
        tier = tier.max(floor);

        let wants_new = rng.chance(odds.discovery);
        // A guaranteed floor only picks what reaches it, from the other pool
        // when the one the card was due to draw from has nothing that rare.
        let (fresh_reach, owned_reach) =
            (reaching(phy, &fresh, floor), reaching(phy, &owned, floor));
        let take_new = if floor > Tier::Common {
            if wants_new {
                !fresh_reach.is_empty()
            } else {
                owned_reach.is_empty()
            }
        } else {
            wants_new
        };
        let new = if take_new {
            pick(phy, &fresh_reach, tier, planet, rng)
        } else {
            None
        };
        let card = match new {
            Some(i) => {
                seen[i] = true;
                Card {
                    taxon: i,
                    tier: phy.taxa[i].eco.tier,
                    morph: Morph::None,
                    new: true,
                    note: None,
                }
            }
            None => {
                let i = pick(phy, &owned_reach, tier, planet, rng).unwrap_or(Phylogeny::ROOT);
                // A new species was due but nothing new could live here.
                let note = (wants_new && fresh.is_empty()).then(|| {
                    blocked_hint(phy, &seen, planet)
                        .unwrap_or_else(|| "Nothing new could evolve here.".into())
                });
                Card {
                    taxon: i,
                    tier: phy.taxa[i].eco.tier,
                    morph: Morph::None,
                    new: false,
                    note,
                }
            }
        };
        cards.push(card);
    }

    for c in cards.iter_mut() {
        c.morph = roll_morph(phy, c.taxon, planet, odds, rng);
    }
    // Best last: tier, then a morph, then novelty.
    cards.sort_by_key(|c| (c.tier, c.morph != Morph::None, c.new));

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
        let cards = roll(&phy, &found, &planet, &Odds::default(), &mut Rng::new(1));
        assert_eq!(cards.len(), 3);
        assert!(cards.iter().all(|c| c.new));
    }

    #[test]
    fn cards_only_ever_hold_what_the_tree_and_planet_allow() {
        let (phy, mut found) = start();
        let mut rng = Rng::new(42);
        let planet = Planet {
            land: 3,
            vegetation: 4,
            oxygen: 4,
            temperature: 3,
            volcanism: 0,
        };
        for _ in 0..60 {
            let cards = roll(&phy, &found, &planet, &Odds::default(), &mut rng);
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
                    assert!(t.eco.needs.met_by(&planet), "{} can't live here", t.name);
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
        let cards = roll(&phy, &found, &planet, &Odds::default(), &mut Rng::new(3));
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
                &mut rng,
            );
            for w in cards.windows(2) {
                assert!(w[0].tier <= w[1].tier);
            }
            assert_eq!(tell(&cards), cards.last().unwrap().tier);
        }
    }

    #[test]
    fn discovery_decides_between_new_species_and_owned_ones() {
        let (phy, found) = start();
        let planet = Planet {
            oxygen: 2,
            ..Planet::default()
        };
        let draw = |discovery| {
            let odds = Odds {
                cards: 40,
                discovery,
                ..Odds::default()
            };
            roll(&phy, &found, &planet, &odds, &mut Rng::new(9))
        };
        let owned = draw(0.0);
        assert!(
            owned.iter().all(|c| !c.new && found[c.taxon]),
            "all duplicates"
        );
        assert!(
            owned.iter().all(|c| c.note.is_none()),
            "none was due to be new"
        );
        let fresh = draw(1.0);
        assert!(fresh.iter().any(|c| c.new));
        // Once nothing new is left, a card due to be new says why it isn't.
        assert!(fresh.iter().filter(|c| !c.new).all(|c| c.note.is_some()));
    }

    /// The root and Dickinsonia owned in a primordial sea: the only Rare
    /// that can drop is the owned Dickinsonia, and no Epic at all.
    fn only_an_owned_rare() -> (Phylogeny, Vec<bool>, Planet) {
        let (phy, mut found) = start();
        found[idx(&phy, "Dickinsonia")] = true;
        let planet = Planet::default();
        assert!(
            eligible(&phy, &found, &planet)
                .iter()
                .all(|&i| phy.taxa[i].eco.tier < Tier::Rare),
            "nothing new is Rare here"
        );
        (phy, found, planet)
    }

    #[test]
    fn a_sure_rare_draws_from_the_pool_that_has_one() {
        let (phy, found, planet) = only_an_owned_rare();
        let mut rng = Rng::new(5);
        for discovery in [1.0, 0.5, 0.0] {
            let odds = Odds {
                discovery,
                catalyst: true,
                ..Odds::default()
            };
            for _ in 0..50 {
                let cards = roll(&phy, &found, &planet, &odds, &mut rng);
                assert!(tell(&cards) >= Tier::Rare, "{cards:?}");
            }
        }
    }

    #[test]
    fn a_sure_epic_out_of_reach_still_keeps_the_rare() {
        let (phy, found, planet) = only_an_owned_rare();
        let mut rng = Rng::new(6);
        let odds = Odds {
            sure_epic: true,
            ..Odds::default()
        };
        for _ in 0..50 {
            let cards = roll(&phy, &found, &planet, &odds, &mut rng);
            assert!(tell(&cards) >= Tier::Rare, "{cards:?}");
        }
    }

    #[test]
    fn morphs_stay_rare_whatever_stacks() {
        assert!(morph_chance(1.0, 1.0) < 0.05, "a plain card");
        assert_eq!(morph_chance(100.0, 3.0), MORPH_CHANCE_MAX);
        // Rolled: even maxed out with arthropods at 35% oxygen, the cap holds.
        let (phy, mut found) = start();
        for n in ["Bilaterian", "Protostome", "Ecdysozoan", "Arthropod"] {
            found[idx(&phy, n)] = true;
        }
        let planet = Planet {
            oxygen: 5,
            land: 1,
            ..Planet::default()
        };
        let odds = Odds {
            cards: 2000,
            morph_mult: 100.0,
            morph_rad: 3.0,
            discovery: 0.0,
            ..Odds::default()
        };
        let cards = roll(&phy, &found, &planet, &odds, &mut Rng::new(4));
        let rate = cards.iter().filter(|c| c.morph != Morph::None).count() as f32 / 2000.0;
        assert!(rate < MORPH_CHANCE_MAX + 0.03, "{rate}");
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
        assert_eq!(w, tier_weights(50.0), "radiation's ceiling");
        assert!(w[0] >= 10.0, "Common never vanishes");
    }
}
