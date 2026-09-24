//! Game state and its loop: Shape the planet -> time Running -> a new genome
//! (`Genome`) waits to be opened -> pick a `Boon` -> Shape again.

use serde::{Deserialize, Serialize};

use crate::ecology::{self, Bonus, Tier};
use crate::genome::{self, Card, Morph, Odds, Pity, Rng};
use crate::planet::{self, Cycle, Lever, Planet};
use crate::tree::{Group, Phylogeny, Taxon};
use crate::wait;

/// Specimens needed to go from level `n` to `n + 1` (index `n - 1`).
const LEVEL_STEPS: [u32; 4] = [1, 2, 4, 8];
pub const MAX_LEVEL: u32 = 5;
/// Adjustment points cap, whatever the keystones and boons give.
const MAX_POINTS: u8 = 8;
const BASE_KEYSTONE_SLOTS: usize = 3;
/// Keystone effects per unit of strength (tier units x level x morph).
pub const AFFINITY_PER_UNIT: f32 = 0.10;
pub const EXTRA_CARD_PER_UNIT: f32 = 0.12;
pub const MORPH_PER_UNIT: f32 = 0.08;
pub const QUICK_PER_UNIT: f32 = 0.03;
const SAVE_VERSION: u32 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Shape,
    Running,
    Genome,
    Boon,
}

/// A one-cycle bonus, picked after each genome.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Boon {
    /// Taxa under this node get x3 weight.
    Lure(usize),
    /// +1 card.
    Lens,
    /// A guaranteed Rare+ card.
    Catalyst,
    /// x4 morph chance.
    Charm,
    /// One hour off the wait (never below half of it).
    Tailwind,
    /// +2 adjustment points this shaping phase.
    Tectonics,
}

impl Boon {
    pub fn title(self) -> &'static str {
        match self {
            Boon::Lure(_) => "Lure",
            Boon::Lens => "Lens",
            Boon::Catalyst => "Catalyst",
            Boon::Charm => "Charm",
            Boon::Tailwind => "Tailwind",
            Boon::Tectonics => "Tectonics",
        }
    }

    /// Index into `sprites::BOON_ART`.
    pub fn icon(self) -> usize {
        match self {
            Boon::Lure(_) => 0,
            Boon::Lens => 1,
            Boon::Catalyst => 2,
            Boon::Charm => 3,
            Boon::Tailwind => 4,
            Boon::Tectonics => 5,
        }
    }

    pub fn describe(self, phy: &Phylogeny) -> String {
        match self {
            Boon::Lure(t) => format!("{} x3 odds", phy.taxa[t].clade),
            Boon::Lens => "+1 card in the next genome".into(),
            Boon::Catalyst => "A guaranteed Rare or better".into(),
            Boon::Charm => "x4 morph chance".into(),
            Boon::Tailwind => "1 hour less to wait".into(),
            Boon::Tectonics => "+2 adjustment points".into(),
        }
    }
}

pub fn morph_bit(m: Morph) -> u8 {
    match m {
        Morph::None => 0,
        Morph::Giant => 1,
        Morph::Albino => 2,
        Morph::Melanistic => 4,
        Morph::Amber => 8,
    }
}

pub const MORPHS: [Morph; 4] = [Morph::Giant, Morph::Albino, Morph::Melanistic, Morph::Amber];

/// What opening a genome did to one card's animal.
#[derive(Clone, Debug)]
pub struct Opened {
    pub card: Card,
    pub level_before: u32,
    pub level_after: u32,
    pub first_morph: bool,
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    version: u32,
    unlocked: Vec<bool>,
    specimens: Vec<u32>,
    morphs: Vec<u8>,
    #[serde(alias = "found_day")]
    found_ma: Vec<Option<u32>>,
    planet: Planet,
    shaped_from: Planet,
    cycle: Option<Cycle>,
    cycles_done: u32,
    /// Absent before the wait could be chosen: every cycle was 20 Ma.
    #[serde(default)]
    ma_done: Option<u32>,
    #[serde(default = "default_wait")]
    wait_hours: f32,
    #[serde(alias = "nodule")]
    genome: Option<Vec<Card>>,
    pity: Pity,
    rng: Rng,
    #[serde(alias = "patrons")]
    keystones: Vec<usize>,
    boon: Option<Boon>,
    boon_offer: Option<Vec<Boon>>,
}

pub struct Game {
    pub phy: Phylogeny,
    pub unlocked: Vec<bool>,
    /// 0 until discovered, then 1..=`MAX_LEVEL`, derived from `specimens`.
    pub level: Vec<u32>,
    /// Copies collected per taxon, the discovery included.
    pub specimens: Vec<u32>,
    /// `morph_bit` flags of the morphs owned per taxon.
    pub morphs: Vec<u8>,
    /// Millions of years elapsed when each taxon evolved.
    pub found_ma: Vec<Option<u32>>,
    pub planet: Planet,
    /// Points are spent as distance from this, so stepping back refunds.
    pub shaped_from: Planet,
    pub cycle: Option<Cycle>,
    pub cycles_done: u32,
    /// Millions of years of the finished cycles.
    pub ma_done: u32,
    /// The wait picked on the dial, reused as the next default.
    pub wait_hours: f32,
    pub genome: Option<Vec<Card>>,
    pub pity: Pity,
    rng: Rng,
    pub keystones: Vec<usize>,
    pub boon: Option<Boon>,
    pub boon_offer: Option<Vec<Boon>>,
}

impl Game {
    pub fn new(now: f64) -> Self {
        let phy = Phylogeny::load();
        let n = phy.len();
        let mut unlocked = vec![false; n];
        unlocked[Phylogeny::ROOT] = true;
        let mut specimens = vec![0; n];
        specimens[Phylogeny::ROOT] = 1;
        let mut found_ma = vec![None; n];
        found_ma[Phylogeny::ROOT] = Some(0);
        let mut game = Self {
            phy,
            unlocked,
            level: vec![0; n],
            specimens,
            morphs: vec![0; n],
            found_ma,
            planet: Planet::default(),
            shaped_from: Planet::default(),
            cycle: None,
            cycles_done: 0,
            ma_done: 0,
            wait_hours: wait::DEFAULT_HOURS,
            genome: None,
            pity: Pity::default(),
            rng: Rng::new((now * 1000.0) as u64 ^ 0x9E37_79B9_7F4A_7C15),
            keystones: Vec::new(),
            boon: None,
            boon_offer: None,
        };
        game.refresh_levels();
        game
    }

    pub fn save(&self) {
        crate::save::write(&self.to_json());
    }

    fn to_json(&self) -> String {
        let data = SaveData {
            version: SAVE_VERSION,
            unlocked: self.unlocked.clone(),
            specimens: self.specimens.clone(),
            morphs: self.morphs.clone(),
            found_ma: self.found_ma.clone(),
            planet: self.planet,
            shaped_from: self.shaped_from,
            cycle: self.cycle,
            cycles_done: self.cycles_done,
            ma_done: Some(self.ma_done),
            wait_hours: self.wait_hours,
            genome: self.genome.clone(),
            pity: self.pity,
            rng: self.rng.clone(),
            keystones: self.keystones.clone(),
            boon: self.boon,
            boon_offer: self.boon_offer.clone(),
        };
        serde_json::to_string(&data).unwrap_or_default()
    }

    /// `None` for a missing, old-format or mismatched save.
    pub fn load(now: f64) -> Option<Self> {
        Self::from_json(&crate::save::read()?, now)
    }

    fn from_json(json: &str, now: f64) -> Option<Self> {
        let mut d: SaveData = serde_json::from_str(json).ok()?;
        let phy = Phylogeny::load();
        let n = phy.len();
        let saved = d.unlocked.len();
        if d.version != SAVE_VERSION
            || saved > n
            || d.specimens.len() != saved
            || d.morphs.len() != saved
            || d.found_ma.len() != saved
            || d.keystones.iter().any(|&p| p >= saved)
        {
            return None;
        }
        // Taxa added since the save was written are appended to the tree.
        d.unlocked.resize(n, false);
        d.specimens.resize(n, 0);
        d.morphs.resize(n, 0);
        d.found_ma.resize(n, None);
        let mut game = Self {
            phy,
            unlocked: d.unlocked,
            level: vec![0; n],
            specimens: d.specimens,
            morphs: d.morphs,
            found_ma: d.found_ma,
            planet: d.planet,
            shaped_from: d.shaped_from,
            cycle: d.cycle,
            cycles_done: d.cycles_done,
            ma_done: d.ma_done.unwrap_or(d.cycles_done * 20),
            wait_hours: wait::snap(d.wait_hours),
            genome: d.genome,
            pity: d.pity,
            rng: d.rng,
            keystones: d.keystones,
            boon: d.boon,
            boon_offer: d.boon_offer,
        };
        game.refresh_levels();
        game.tick(now);
        Some(game)
    }

    pub fn taxon(&self, i: usize) -> &Taxon {
        &self.phy.taxa[i]
    }

    pub fn discovered(&self) -> usize {
        self.unlocked.iter().filter(|&&u| u).count()
    }

    pub fn phase(&self) -> Phase {
        if self.cycle.is_some() {
            Phase::Running
        } else if self.genome.is_some() {
            Phase::Genome
        } else if self.boon_offer.is_some() {
            Phase::Boon
        } else {
            Phase::Shape
        }
    }

    /// Millions of years let run so far, counting up through a running cycle.
    pub fn ma_elapsed(&self, now: f64) -> u32 {
        let running = self.cycle.map_or(0.0, |c| c.progress(now) * c.ma as f64);
        self.ma_done + running as u32
    }

    /// The deepest discovered taxon; between equally deep ones, the youngest.
    pub fn most_advanced(&self) -> usize {
        (0..self.phy.len())
            .filter(|&i| self.unlocked[i])
            .max_by(|&a, &b| {
                let (ta, tb) = (&self.phy.taxa[a], &self.phy.taxa[b]);
                ta.depth.cmp(&tb.depth).then(tb.mya.total_cmp(&ta.mya))
            })
            .unwrap_or(Phylogeny::ROOT)
    }

    /// The era of the youngest discovered taxon.
    pub fn era(&self) -> &'static str {
        let mya = (0..self.phy.len())
            .filter(|&i| self.unlocked[i])
            .map(|i| self.phy.taxa[i].mya)
            .fold(f32::INFINITY, f32::min);
        era_for(mya)
    }

    fn refresh_levels(&mut self) {
        for i in 0..self.phy.len() {
            self.level[i] = if self.unlocked[i] {
                level_for(self.specimens[i])
            } else {
                0
            };
        }
    }

    pub fn keystone_slots(&self) -> usize {
        let found = self.discovered();
        BASE_KEYSTONE_SLOTS + (found >= 20) as usize + (found >= 40) as usize
    }

    pub fn keystone_strength(&self, taxon: usize) -> f32 {
        let eco = ecology::of(self.phy.taxa[taxon].name);
        let morph = MORPHS
            .iter()
            .filter(|&&m| self.morphs[taxon] & morph_bit(m) != 0)
            .map(|m| m.strength())
            .fold(1.0, f32::max);
        eco.tier.units() * (1.0 + 0.25 * (self.level[taxon].max(1) - 1) as f32) * morph
    }

    /// Whether keystones can be changed: while shaping and while time runs
    /// (the genome is rolled from them when the cycle ends), not once it's
    /// ready to open.
    pub fn keystones_editable(&self) -> bool {
        matches!(self.phase(), Phase::Shape | Phase::Running)
    }

    /// Equips or unequips `taxon`. Returns whether anything changed.
    pub fn toggle_keystone(&mut self, taxon: usize) -> bool {
        if !self.keystones_editable() || !self.unlocked[taxon] {
            return false;
        }
        if let Some(pos) = self.keystones.iter().position(|&p| p == taxon) {
            self.keystones.remove(pos);
            return true;
        }
        if self.keystones.len() >= self.keystone_slots() {
            return false;
        }
        self.keystones.push(taxon);
        true
    }

    /// The planet keystones live on: as launched while time runs.
    fn living_planet(&self) -> Planet {
        self.cycle.map_or(self.planet, |c| c.launched)
    }

    /// Why `taxon` couldn't live on the planet right now ("too cold"), or
    /// `None` if it thrives. A dormant keystone gives no bonus.
    pub fn dormant_reason(&self, taxon: usize) -> Option<String> {
        ecology::of(self.phy.taxa[taxon].name)
            .needs
            .missing(&self.living_planet())
    }

    fn keystone_bonuses(&self) -> impl Iterator<Item = (Bonus, f32)> + '_ {
        self.keystones
            .iter()
            .filter(|&&p| self.dormant_reason(p).is_none())
            .map(|&p| {
                (
                    ecology::of(self.phy.taxa[p].name).bonus,
                    self.keystone_strength(p),
                )
            })
    }

    /// What the keystones and current boon add up to. `extra_card`'s integer
    /// part is guaranteed cards, its fraction the chance of one more.
    pub fn effects(&self) -> Effects {
        let mut e = Effects::default();
        for (bonus, s) in self.keystone_bonuses() {
            match bonus {
                Bonus::Affinity(h) => e.affinity[h.index()] += AFFINITY_PER_UNIT * s,
                Bonus::Luck => e.luck += s,
                Bonus::ExtraCard => e.extra_card += EXTRA_CARD_PER_UNIT * s,
                Bonus::Morph => e.morph += MORPH_PER_UNIT * s,
                Bonus::Quick => e.quick += QUICK_PER_UNIT * s,
                Bonus::Point => e.points += 1,
                Bonus::Soil => e.soil += 1,
                Bonus::DoubleSpecimens => e.double_specimens = true,
                Bonus::LivingFossil => {
                    e.luck += s;
                    e.legendary_pity = 30;
                }
                Bonus::Oddity => {
                    e.morph += MORPH_PER_UNIT * s;
                    e.morph_window = Some(7);
                }
                Bonus::Mind => {
                    e.points += 1;
                    e.luck += s;
                }
            }
        }
        e.luck = e.luck.min(15.0);
        e.extra_card = e.extra_card.min(3.0);
        e.morph = e.morph.min(2.0);
        e.quick = e.quick.min(0.3);
        match self.boon {
            Some(Boon::Lens) => e.extra_card += 1.0,
            Some(Boon::Charm) => e.morph_mult_boon = 4.0,
            _ => {}
        }
        e
    }

    pub fn max_points(&self) -> u8 {
        let e = self.effects();
        let tectonics = if self.boon == Some(Boon::Tectonics) {
            2
        } else {
            0
        };
        (planet::BASE_POINTS + e.points as u8 + tectonics).min(MAX_POINTS)
    }

    fn cost(&self, p: &Planet) -> u8 {
        Lever::ALL
            .iter()
            .map(|&l| p.get(l).abs_diff(self.shaped_from.get(l)))
            .sum()
    }

    pub fn points_spent(&self) -> u8 {
        self.cost(&self.planet)
    }

    pub fn points_left(&self) -> u8 {
        self.max_points().saturating_sub(self.points_spent())
    }

    /// The planet after stepping `lever`, if the phase and budget allow it.
    fn stepped(&self, lever: Lever, delta: i8) -> Option<Planet> {
        let mut p = self.planet;
        (self.phase() == Phase::Shape && p.step(lever, delta) && self.cost(&p) <= self.max_points())
            .then_some(p)
    }

    pub fn step_lever(&mut self, lever: Lever, delta: i8) -> bool {
        match self.stepped(lever, delta) {
            Some(p) => {
                self.planet = p;
                true
            }
            None => false,
        }
    }

    pub fn can_step(&self, lever: Lever, delta: i8) -> bool {
        self.stepped(lever, delta).is_some()
    }

    pub fn planet_after(&self) -> Planet {
        self.planet.after_cycle(self.effects().soil as u8)
    }

    pub fn blocked_hint(&self) -> Option<String> {
        genome::blocked_hint(&self.phy, &self.unlocked, &self.planet)
    }

    pub fn eligible_count(&self) -> usize {
        genome::eligible(&self.phy, &self.unlocked, &self.planet).len()
    }

    /// Whether the wait is picked on the dial yet (not in the tutorial).
    pub fn wait_choosable(&self) -> bool {
        self.cycles_done >= planet::TUTORIAL_CYCLES
    }

    pub fn set_wait(&mut self, hours: f32) {
        self.wait_hours = wait::snap(hours);
    }

    /// The wait the next cycle is paid for.
    pub fn next_hours(&self) -> f32 {
        if self.wait_choosable() {
            self.wait_hours
        } else {
            wait::TUTORIAL_HOURS
        }
    }

    /// What the next genome would be rolled with, from the keystones, the
    /// boon and the wait.
    pub fn forecast(&self) -> Forecast {
        self.forecast_for(self.next_hours())
    }

    fn forecast_for(&self, hours: f32) -> Forecast {
        let e = self.effects();
        let w = wait::bonus(hours);
        let mut affinity = [1.0; 6];
        for (a, add) in affinity.iter_mut().zip(e.affinity) {
            *a += add;
        }
        let cards = (w.cards + e.extra_card).min(genome::MAX_CARDS as f32);
        Forecast {
            cards,
            ma: wait::ma(hours),
            odds: Odds {
                cards: cards.floor() as usize,
                luck: (e.luck + w.luck).min(15.0),
                morph_mult: (1.0 + e.morph) * e.morph_mult_boon * w.morph_mult,
                affinity,
                lure: match self.boon {
                    Some(Boon::Lure(t)) => Some(t),
                    _ => None,
                },
                catalyst: self.boon == Some(Boon::Catalyst) || w.sure_rare,
                legendary_pity: e.legendary_pity,
                morph_window: e.morph_window,
            },
        }
    }

    pub fn next_cycle_seconds(&self) -> f64 {
        let base = planet::cycle_seconds(self.cycles_done, self.next_hours(), self.effects().quick);
        if self.boon == Some(Boon::Tailwind) {
            (base - 3600.0).max(base * 0.5)
        } else {
            base
        }
    }

    pub fn accelerate(&mut self, now: f64) -> bool {
        if self.phase() != Phase::Shape {
            return false;
        }
        let hours = self.next_hours();
        self.cycle = Some(Cycle {
            started_at: now,
            duration: self.next_cycle_seconds(),
            launched: self.planet,
            hours,
            ma: wait::ma(hours),
        });
        true
    }

    /// A finished cycle becomes a genome waiting to be opened.
    pub fn tick(&mut self, now: f64) {
        let Some(cycle) = self.cycle else { return };
        if !cycle.is_done(now) {
            return;
        }
        let soil = self.effects().soil;
        let forecast = self.forecast_for(cycle.hours);
        let mut odds = forecast.odds;
        if self.rng.chance(forecast.cards.fract()) {
            odds.cards += 1;
        }
        let rolled = genome::roll(
            &self.phy,
            &self.unlocked,
            &cycle.launched,
            &odds,
            &mut self.pity,
            &mut self.rng,
        );
        self.genome = Some(rolled);
        self.planet = cycle.launched.after_cycle(soil as u8);
        self.shaped_from = self.planet;
        self.cycle = None;
        self.cycles_done += 1;
        self.ma_done += cycle.ma;
        self.boon = None;
    }

    /// Dev aid: end the running cycle right now.
    pub fn skip_cycle(&mut self, now: f64) {
        if let Some(c) = &mut self.cycle {
            c.started_at = now - c.duration;
        }
        self.tick(now);
    }

    /// The best tier in the waiting genome.
    pub fn genome_tell(&self) -> Option<Tier> {
        self.genome.as_deref().map(genome::tell)
    }

    /// Adds the waiting genome to the collection and offers the next boons.
    /// Returns what each card did, best last.
    pub fn open_genome(&mut self, now: f64) -> Vec<Opened> {
        let Some(cards) = self.genome.take() else {
            return Vec::new();
        };
        let ma = self.ma_elapsed(now);
        let double = self.effects().double_specimens;
        let mut out = Vec::with_capacity(cards.len());
        for card in cards {
            let t = card.taxon;
            let level_before = self.level[t];
            if card.new && !self.unlocked[t] {
                self.unlocked[t] = true;
                self.specimens[t] = 1;
                self.found_ma[t] = Some(ma);
            } else {
                self.specimens[t] += if double { 2 } else { 1 };
            }
            let first_morph =
                card.morph != Morph::None && self.morphs[t] & morph_bit(card.morph) == 0;
            self.morphs[t] |= morph_bit(card.morph);
            self.level[t] = level_for(self.specimens[t]);
            out.push(Opened {
                level_after: self.level[t],
                level_before,
                first_morph,
                card,
            });
        }
        self.boon_offer = Some(self.roll_boons());
        out
    }

    fn roll_boons(&mut self) -> Vec<Boon> {
        let mut pool = vec![
            Boon::Lens,
            Boon::Catalyst,
            Boon::Charm,
            Boon::Tailwind,
            Boon::Tectonics,
        ];
        // A Lure points at a found branch with something left to find.
        let lures: Vec<usize> = (0..self.phy.len())
            .filter(|&i| {
                self.unlocked[i]
                    && self.phy.taxa[i].group == Group::Backbone
                    && i != Phylogeny::ROOT
                    && self.has_undiscovered_below(i)
            })
            .collect();
        if !lures.is_empty() {
            let t = lures[(self.rng.next_u64() % lures.len() as u64) as usize];
            pool.push(Boon::Lure(t));
            pool.push(Boon::Lure(t)); // twice as likely
        }
        let mut offer = Vec::with_capacity(3);
        while offer.len() < 3 && !pool.is_empty() {
            let b = pool.swap_remove((self.rng.next_u64() % pool.len() as u64) as usize);
            if !offer.contains(&b) {
                offer.push(b);
            }
        }
        offer
    }

    fn has_undiscovered_below(&self, i: usize) -> bool {
        self.phy.taxa[i]
            .children
            .iter()
            .any(|&c| !self.unlocked[c] || self.has_undiscovered_below(c))
    }

    /// Takes boon `i` (none if out of range) and starts shaping.
    pub fn choose_boon(&mut self, i: usize) {
        let Some(offer) = self.boon_offer.take() else {
            return;
        };
        self.boon = offer.get(i).copied();
        self.shaped_from = self.planet;
    }

    /// Specimens toward `taxon`'s next level, and the step size.
    pub fn level_progress(&self, taxon: usize) -> (u32, u32) {
        let lv = self.level[taxon].max(1);
        if lv >= MAX_LEVEL {
            return (0, 0);
        }
        let floor: u32 = LEVEL_STEPS[..(lv - 1) as usize].iter().sum::<u32>() + 1;
        let step = LEVEL_STEPS[(lv - 1) as usize];
        (self.specimens[taxon].saturating_sub(floor), step)
    }
}

/// The next genome's odds, before its card count is rolled.
#[derive(Clone, Debug)]
pub struct Forecast {
    /// Expected cards: the fraction is the chance of one more.
    pub cards: f32,
    pub odds: Odds,
    pub ma: u32,
}

fn default_wait() -> f32 {
    wait::DEFAULT_HOURS
}

#[derive(Clone, Debug)]
pub struct Effects {
    pub affinity: [f32; 6],
    pub luck: f32,
    pub extra_card: f32,
    pub morph: f32,
    pub morph_mult_boon: f32,
    pub quick: f32,
    pub points: u32,
    pub soil: u32,
    pub double_specimens: bool,
    pub legendary_pity: u32,
    pub morph_window: Option<u32>,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            affinity: [0.0; 6],
            luck: 0.0,
            extra_card: 0.0,
            morph: 0.0,
            morph_mult_boon: 1.0,
            quick: 0.0,
            points: 0,
            soil: 0,
            double_specimens: false,
            legendary_pity: genome::LEGENDARY_PITY,
            morph_window: None,
        }
    }
}

pub fn level_for(specimens: u32) -> u32 {
    let mut lv = 1;
    let mut need = 1;
    for step in LEVEL_STEPS {
        need += step;
        if specimens >= need {
            lv += 1;
        } else {
            break;
        }
    }
    lv.min(MAX_LEVEL)
}

pub fn era_for(mya: f32) -> &'static str {
    match mya {
        m if m >= 541.0 => "Ediacaran",
        m if m >= 485.0 => "Cambrian",
        m if m >= 444.0 => "Ordovician",
        m if m >= 419.0 => "Silurian",
        m if m >= 359.0 => "Devonian",
        m if m >= 299.0 => "Carboniferous",
        m if m >= 252.0 => "Permian",
        m if m >= 201.0 => "Triassic",
        m if m >= 145.0 => "Jurassic",
        m if m >= 66.0 => "Cretaceous",
        m if m >= 23.0 => "Paleogene",
        m if m >= 2.6 => "Neogene",
        _ => "Quaternary",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_cycle(g: &mut Game, now: &mut f64) -> Vec<Opened> {
        assert!(g.accelerate(*now));
        *now += g.cycle.unwrap().duration + 1.0;
        g.tick(*now);
        assert_eq!(g.phase(), Phase::Genome);
        let opened = g.open_genome(*now);
        assert_eq!(g.phase(), Phase::Boon);
        g.choose_boon(0);
        assert_eq!(g.phase(), Phase::Shape);
        opened
    }

    #[test]
    fn a_new_game_starts_shaping_with_one_animal() {
        let g = Game::new(0.0);
        assert_eq!(g.phase(), Phase::Shape);
        assert_eq!(g.discovered(), 1);
        assert_eq!(g.level[Phylogeny::ROOT], 1);
        assert_eq!(g.points_left(), planet::BASE_POINTS);
    }

    #[test]
    fn points_are_spent_by_distance_and_refunded_by_stepping_back() {
        let mut g = Game::new(0.0);
        assert!(g.step_lever(Lever::Oxygen, 1));
        assert!(g.step_lever(Lever::Oxygen, 1));
        assert_eq!(g.points_left(), 1);
        assert!(g.step_lever(Lever::Oxygen, -1));
        assert_eq!(g.points_left(), 2, "stepping back refunds");
        assert!(g.step_lever(Lever::Land, 1));
        assert!(g.step_lever(Lever::Temperature, 1));
        assert!(!g.step_lever(Lever::Temperature, 1), "out of points");
    }

    #[test]
    fn nothing_changes_while_time_runs() {
        let mut g = Game::new(0.0);
        assert!(g.accelerate(0.0));
        assert_eq!(g.phase(), Phase::Running);
        assert!(!g.step_lever(Lever::Oxygen, 1));
        assert!(!g.accelerate(1.0));
        g.tick(30.0);
        assert_eq!(g.phase(), Phase::Running, "the first cycle lasts a minute");
    }

    #[test]
    fn the_first_cycle_discovers_the_roots_children() {
        let mut g = Game::new(0.0);
        g.step_lever(Lever::Oxygen, 1);
        let mut now = 0.0;
        let opened = run_cycle(&mut g, &mut now);
        assert_eq!(opened.len(), 3);
        assert!(opened.iter().all(|o| o.card.new));
        assert_eq!(g.discovered(), 4);
        assert!(g.boon_offer.is_none());
    }

    #[test]
    fn a_long_game_progresses_and_levels_up_duplicates() {
        let mut g = Game::new(0.0);
        let mut now = 0.0;
        // A friendly world that hosts most of the tree.
        let target = Planet {
            land: 2,
            vegetation: 3,
            oxygen: 4,
            temperature: 3,
            volcanism: 0,
        };
        for _ in 0..80 {
            for l in Lever::ALL {
                while g.planet.get(l) < target.get(l) && g.step_lever(l, 1) {}
                while g.planet.get(l) > target.get(l) && g.step_lever(l, -1) {}
            }
            run_cycle(&mut g, &mut now);
        }
        assert!(g.discovered() > 25, "only {} found", g.discovered());
        assert!(
            (0..g.phy.len()).any(|i| g.level[i] >= 2),
            "duplicates should level"
        );
    }

    #[test]
    fn keystones_are_limited_and_editable_until_the_genome_is_ready() {
        let mut g = Game::new(0.0);
        for i in 1..6 {
            g.unlocked[i] = true;
            g.specimens[i] = 1;
        }
        g.refresh_levels();
        for i in 1..5 {
            g.toggle_keystone(i);
        }
        assert_eq!(g.keystones.len(), 3);
        assert!(g.toggle_keystone(1), "unequip");
        g.accelerate(0.0);
        assert!(g.toggle_keystone(5), "still editable while time runs");
        g.skip_cycle(0.0);
        assert_eq!(g.phase(), Phase::Genome);
        assert!(!g.toggle_keystone(5), "locked once the genome is ready");
    }

    #[test]
    fn a_point_keystone_raises_the_budget() {
        let mut g = Game::new(0.0);
        let ant = g.phy.taxa.iter().position(|t| t.name == "Ant").unwrap();
        g.unlocked[ant] = true;
        g.specimens[ant] = 1;
        g.refresh_levels();
        g.toggle_keystone(ant);
        assert!(g.dormant_reason(ant).is_some(), "no forest on a sea world");
        assert_eq!(
            g.max_points(),
            planet::BASE_POINTS,
            "a dormant keystone gives nothing"
        );
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 3,
            volcanism: 0,
        };
        assert_eq!(g.dormant_reason(ant), None);
        assert_eq!(g.max_points(), planet::BASE_POINTS + 1);
    }

    #[test]
    fn levels_follow_the_specimen_steps() {
        assert_eq!(level_for(1), 1);
        assert_eq!(level_for(2), 2);
        assert_eq!(level_for(4), 3);
        assert_eq!(level_for(8), 4);
        assert_eq!(level_for(16), 5);
        assert_eq!(level_for(999), MAX_LEVEL);
    }

    #[test]
    fn save_round_trips() {
        let mut g = Game::new(10.0);
        g.step_lever(Lever::Oxygen, 1);
        g.accelerate(10.0);
        let back = Game::from_json(&g.to_json(), 20.0).expect("save should load");
        assert_eq!(back.phase(), Phase::Running);
        assert_eq!(back.planet, g.planet);
        assert!(
            Game::from_json("{\"dna\": 5}", 0.0).is_none(),
            "old saves are discarded"
        );
    }

    #[test]
    fn the_most_advanced_prefers_the_youngest_of_equal_depth() {
        let mut g = Game::new(0.0);
        let find = |g: &Game, name| g.phy.taxa.iter().position(|t| t.name == name).unwrap();
        let (neanderthal, human) = (find(&g, "Neanderthal"), find(&g, "Human"));
        assert_eq!(g.taxon(neanderthal).depth, g.taxon(human).depth);
        g.unlocked[neanderthal] = true;
        assert_eq!(g.most_advanced(), neanderthal);
        g.unlocked[human] = true;
        assert_eq!(g.most_advanced(), human);
    }

    #[test]
    fn a_waiting_genome_saved_under_its_old_name_still_loads() {
        let mut g = Game::new(0.0);
        g.accelerate(0.0);
        g.skip_cycle(0.0);
        let json = g.to_json().replace("\"genome\":", "\"nodule\":");
        let back = Game::from_json(&json, 0.0).expect("older save should load");
        assert_eq!(back.phase(), Phase::Genome);
        assert_eq!(back.genome, g.genome);
    }

    #[test]
    fn a_save_from_a_smaller_tree_still_loads() {
        let g = Game::new(0.0);
        let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
        for key in ["unlocked", "specimens", "morphs", "found_ma"] {
            v[key].as_array_mut().unwrap().truncate(10);
        }
        let back = Game::from_json(&v.to_string(), 0.0).expect("older save should load");
        assert_eq!(back.unlocked.len(), back.phy.len());
        assert_eq!(back.discovered(), 1);
    }

    #[test]
    fn tailwind_shortens_the_wait_but_never_below_half() {
        let mut g = Game::new(0.0);
        g.cycles_done = 5;
        g.set_wait(6.0);
        g.boon = Some(Boon::Tailwind);
        assert_eq!(g.next_cycle_seconds(), 6.0 * 3600.0 - 3600.0);
        g.cycles_done = 0;
        assert_eq!(g.next_cycle_seconds(), 30.0);
    }

    #[test]
    fn the_wait_is_chosen_only_after_the_tutorial() {
        let mut g = Game::new(0.0);
        g.set_wait(6.0);
        assert!(!g.wait_choosable());
        assert_eq!(g.next_hours(), wait::TUTORIAL_HOURS);
        assert_eq!(g.next_cycle_seconds(), 60.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        assert_eq!(g.next_hours(), 6.0);
        assert_eq!(g.next_cycle_seconds(), 6.0 * 3600.0);
    }

    #[test]
    fn a_long_wait_pays_in_cards_luck_and_a_sure_rare() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.set_wait(2.0);
        let short = g.forecast();
        g.set_wait(6.0);
        let long = g.forecast();
        assert_eq!((short.odds.cards, long.odds.cards), (2, 6));
        assert!(long.odds.luck > short.odds.luck);
        assert!(long.odds.catalyst && !short.odds.catalyst);
        assert_eq!((short.ma, long.ma), (20, 60));
    }

    #[test]
    fn millions_of_years_add_up_per_cycle() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.set_wait(4.5);
        g.accelerate(0.0);
        assert_eq!(g.ma_elapsed(2.25 * 3600.0), 22);
        g.skip_cycle(0.0);
        assert_eq!(g.ma_done, 45);
    }

    #[test]
    fn a_save_from_before_the_dial_keeps_its_millions_of_years() {
        let mut g = Game::new(0.0);
        g.cycles_done = 4;
        let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
        let obj = v.as_object_mut().unwrap();
        obj.remove("ma_done");
        obj.remove("wait_hours");
        let back = Game::from_json(&v.to_string(), 0.0).expect("older save should load");
        assert_eq!(back.ma_done, 80);
        assert_eq!(back.wait_hours, wait::DEFAULT_HOURS);
    }

    #[test]
    fn eras_read_from_ages() {
        assert_eq!(era_for(800.0), "Ediacaran");
        assert_eq!(era_for(400.0), "Devonian");
        assert_eq!(era_for(0.3), "Quaternary");
    }
}
