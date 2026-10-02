//! Game state and its loop: Shape the planet -> time Running -> a new genome
//! (`Genome`) waits to be opened -> pick a `Boon` -> Shape again.

use serde::{Deserialize, Serialize};

use crate::ecology::{self, Bonus, Catch, Rule, Team, Tier};
use crate::genome::{self, Card, Morph, Odds, Pity, Rng};
use crate::planet::{self, Cycle, Habitat, Lever, Planet};
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

pub fn morph_of_bit(bit: u8) -> Morph {
    MORPHS
        .into_iter()
        .find(|&m| morph_bit(m) == bit)
        .unwrap_or(Morph::None)
}

/// The chance a genome opened with Human ends the Earth.
pub const DOOM_CHANCE: f32 = 0.2;
/// Per RAD: luck past the cap, and morph chance added (x1.5, x2, ...).
pub const RAD_LUCK: f32 = 5.0;
pub const RAD_MORPH: f32 = 0.5;

/// Most charges a growing keystone holds.
const MAX_CHARGES: u8 = 10;
/// Starfish charges: each a third of a card.
const MAX_DUPLICATE_CHARGES: u8 = 3;

/// Whether a keystone works right now.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Active,
    /// The planet (or another keystone) doesn't suit it: it gives nothing.
    Asleep(String),
    /// Awake, but its condition isn't met ("only at 35% oxygen").
    Waiting(String),
}

/// One thing a keystone adds to the next genome or to the planet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gain {
    Luck(f32),
    LuckMult(f32),
    /// Cards (the fraction is a chance).
    Cards(f32),
    Morph(f32),
    MorphMult(f32),
    Quick(f32),
    Points(u32),
    Soil(u32),
    DoubleSpecimens,
    Share(Habitat, f32),
    Pity(u32),
    MorphWindow(u32),
    Giant(f32),
    AfterRare(f32),
    AfterNew(f32),
    Feed,
    NoNewPity(u32),
    FewerBoons,
    LongerWait(f64),
    TempMin(u8),
    /// The chance a genome ends the Earth instead of opening.
    Doom(f32),
}

impl Gain {
    /// Added to the total rather than a rule change: what copies can copy.
    fn copyable(self) -> bool {
        matches!(
            self,
            Gain::Luck(_)
                | Gain::Cards(_)
                | Gain::Morph(_)
                | Gain::Quick(_)
                | Gain::Points(_)
                | Gain::Soil(_)
                | Gain::Share(..)
                | Gain::Giant(_)
                | Gain::AfterRare(_)
                | Gain::AfterNew(_)
        )
    }

    fn scaled(self, f: f32) -> Gain {
        match self {
            Gain::Luck(x) => Gain::Luck(x * f),
            Gain::Cards(x) => Gain::Cards(x * f),
            Gain::Morph(x) => Gain::Morph(x * f),
            Gain::Quick(x) => Gain::Quick(x * f),
            Gain::Share(h, x) => Gain::Share(h, x * f),
            Gain::AfterRare(x) => Gain::AfterRare(x * f),
            Gain::AfterNew(x) => Gain::AfterNew(x * f),
            g => g,
        }
    }

    /// A rough size, to find the strongest keystone to copy.
    fn worth(self) -> f32 {
        match self {
            Gain::Luck(x) => x,
            Gain::Cards(x) => x * 10.0,
            Gain::Morph(x) => x * 10.0,
            Gain::Quick(x) => x * 30.0,
            Gain::Points(n) => n as f32 * 4.0,
            Gain::Soil(n) => n as f32 * 2.0,
            Gain::Share(_, x) => x * 5.0,
            Gain::Giant(x) => x,
            Gain::AfterRare(x) => x * 0.5,
            Gain::AfterNew(x) => x * 5.0,
            _ => 0.0,
        }
    }

    /// "+3.0 Luck", for the keystone lists.
    pub fn label(self) -> String {
        match self {
            Gain::Luck(x) => format!("+{x:.1} Luck"),
            Gain::LuckMult(x) => format!("x{x:.0} Luck"),
            Gain::Cards(x) if x >= 1.0 => format!("+{x:.1} cards"),
            Gain::Cards(x) => format!("+{:.0}% card", x * 100.0),
            Gain::Morph(x) => format!("+{:.0}% morphs", x * 100.0),
            Gain::MorphMult(x) => format!("x{x:.0} morphs"),
            Gain::Quick(x) => format!("-{:.0}% wait", x * 100.0),
            Gain::Points(n) => format!("+{n} point{}", if n == 1 { "" } else { "s" }),
            Gain::Soil(n) => format!("+{n} vegetation"),
            Gain::DoubleSpecimens => "x2 duplicates".into(),
            Gain::Share(h, x) => format!("x{:.1} {}", 1.0 + x, h.name().to_lowercase()),
            Gain::Pity(n) => format!("Legendary in {n}"),
            Gain::MorphWindow(n) => format!("morph every {n}"),
            Gain::Giant(x) => format!("x{x:.0} giants"),
            Gain::AfterRare(x) => format!("+{x:.1} Luck after a Rare"),
            Gain::AfterNew(x) => format!("+{:.0}% morphs after new", x * 100.0),
            Gain::Feed => "feeds keystones".into(),
            Gain::NoNewPity(n) => format!("pity +{n} if nothing new"),
            Gain::FewerBoons => "-1 boon".into(),
            Gain::LongerWait(s) => format!("+{:.0}h wait", s / 3600.0),
            Gain::TempMin(_) => "Temperature locked".into(),
            Gain::Doom(p) => format!("{:.0}% the Earth ends", p * 100.0),
        }
    }
}

/// What one equipped keystone does right now.
#[derive(Clone, Debug)]
pub struct Report {
    pub taxon: usize,
    pub status: Status,
    pub gains: Vec<Gain>,
    /// Live state worth showing: "4 charges", "copying Coral".
    pub note: Option<String>,
}

impl Report {
    /// The gains in a line ("+6.0 Luck  ·  +1 point").
    pub fn summary(&self) -> String {
        let parts: Vec<String> = self
            .gains
            .iter()
            .filter(|g| !matches!(g, Gain::TempMin(_)))
            .map(|g| g.label())
            .collect();
        if parts.is_empty() {
            "nothing yet".into()
        } else {
            parts.join("  ·  ")
        }
    }
}

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
    #[serde(default)]
    charges: Vec<u8>,
    #[serde(default)]
    held: Vec<u8>,
    #[serde(default)]
    gone: Vec<bool>,
    #[serde(default)]
    edition: Vec<u8>,
    #[serde(default)]
    last_launched: Option<Planet>,
    #[serde(default = "default_points")]
    wait_points: u8,
    #[serde(default)]
    rad: u32,
    #[serde(default)]
    fossil: Vec<bool>,
    #[serde(default)]
    doomed: bool,
    #[serde(default)]
    doom_risk: f32,
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
    /// Charges per taxon: cycles grown, or duplicates for the Starfish.
    pub charges: Vec<u8>,
    /// Cycles a fragile keystone has run.
    pub held: Vec<u8>,
    /// A fragile keystone that left; it can come back once found again.
    pub gone: Vec<bool>,
    /// The morph each animal works with as a keystone (`morph_bit`, 0 none).
    pub edition: Vec<u8>,
    /// The planet the last wait ran on, for the keystones that like stasis.
    pub last_launched: Option<Planet>,
    /// Adjustment points the last wait gave this shaping, before keystones.
    pub wait_points: u8,
    /// Earths Human has ended: each one boosts rarity and morphs.
    pub rad: u32,
    /// Every taxon found on any Earth. One not found on this Earth is a
    /// fossil: it keeps its level and morphs and waits to be found again.
    pub fossil: Vec<bool>,
    /// The waiting genome ends the Earth when opened (rolled with it).
    pub doomed: bool,
    /// The chance that roll had, to show while the genome waits.
    pub doom_risk: f32,
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
            charges: vec![0; n],
            held: vec![0; n],
            gone: vec![false; n],
            edition: vec![0; n],
            last_launched: None,
            wait_points: planet::BASE_POINTS,
            rad: 0,
            fossil: vec![false; n],
            doomed: false,
            doom_risk: 0.0,
        };
        game.fossil[Phylogeny::ROOT] = true;
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
            charges: self.charges.clone(),
            held: self.held.clone(),
            gone: self.gone.clone(),
            edition: self.edition.clone(),
            last_launched: self.last_launched,
            wait_points: self.wait_points,
            rad: self.rad,
            fossil: self.fossil.clone(),
            doomed: self.doomed,
            doom_risk: self.doom_risk,
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
        d.charges.resize(n, 0);
        d.held.resize(n, 0);
        d.gone.resize(n, false);
        d.edition.resize(n, 0);
        d.fossil.resize(n, false);
        for (f, &u) in d.fossil.iter_mut().zip(&d.unlocked) {
            *f |= u;
        }
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
            charges: d.charges,
            held: d.held,
            gone: d.gone,
            edition: d.edition,
            last_launched: d.last_launched,
            wait_points: d.wait_points,
            rad: d.rad,
            fossil: d.fossil,
            doomed: d.doomed,
            doom_risk: d.doom_risk,
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

    /// The morph `taxon` works with as a keystone (one it owns, or none).
    pub fn edition(&self, taxon: usize) -> Morph {
        morph_of_bit(self.edition[taxon] & self.morphs[taxon])
    }

    /// Steps `taxon`'s keystone morph to the next one it owns, then none,
    /// while shaping. Returns whether it changed.
    pub fn cycle_edition(&mut self, taxon: usize) -> bool {
        if self.phase() != Phase::Shape {
            return false;
        }
        let owned: Vec<Morph> = std::iter::once(Morph::None)
            .chain(
                MORPHS
                    .into_iter()
                    .filter(|&m| self.morphs[taxon] & morph_bit(m) != 0),
            )
            .collect();
        if owned.len() < 2 {
            return false;
        }
        let cur = self.edition(taxon);
        let i = owned.iter().position(|&m| m == cur).unwrap_or(0);
        self.edition[taxon] = morph_bit(owned[(i + 1) % owned.len()]);
        true
    }

    /// Tier units x level x the morph's multiplier.
    pub fn keystone_strength(&self, taxon: usize) -> f32 {
        let eco = ecology::of(self.phy.taxa[taxon].name);
        eco.tier.units()
            * (1.0 + 0.25 * (self.level[taxon].max(1) - 1) as f32)
            * self.edition(taxon).strength()
    }

    /// Whether keystones can be changed: while shaping and while time runs
    /// (a running wait keeps the ones it was launched with), not once the
    /// genome is ready to open.
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
        if self.keystones.len() >= self.keystone_slots() || self.gone[taxon] {
            return false;
        }
        self.keystones.push(taxon);
        true
    }

    /// The planet keystones live on: as launched while time runs.
    fn living_planet(&self) -> Planet {
        self.cycle.map_or(self.planet, |c| c.launched)
    }

    /// Asleep for reasons of its own: the planet, or its morph.
    fn own_sleep(&self, taxon: usize) -> Option<String> {
        let eco = ecology::of(self.phy.taxa[taxon].name);
        if eco.rule == Rule::Extremes {
            return None;
        }
        let p = self.living_planet();
        let morph = self.edition(taxon);
        let colder = if morph == Morph::Melanistic {
            genome::MELANISTIC_COLDER
        } else {
            0
        };
        if let Some(why) = eco.needs.missing_colder(&p, colder) {
            return Some(why);
        }
        if morph == Morph::Albino && p.temperature > genome::ALBINO_MAX_TEMP {
            return Some("albino: sunburnt above Cool".into());
        }
        None
    }

    /// Why `taxon` gives nothing right now ("too cold"), or `None` if it
    /// thrives. Asleep keystones give no bonus.
    pub fn dormant_reason(&self, taxon: usize) -> Option<String> {
        if let Some(why) = self.own_sleep(taxon) {
            return Some(why);
        }
        let name = self.phy.taxa[taxon].name;
        let tyrant = self.active_keystones().into_iter().any(|k| {
            ecology::of(self.phy.taxa[k].name).rule == Rule::Catch(Catch::Tyrant)
                && self.own_sleep(k).is_none()
        });
        if tyrant && ecology::is_herbivore(name) {
            return Some("hunted by the T. rex".into());
        }
        None
    }

    /// The keystones that count: those equipped when the running wait was
    /// launched, else the equipped ones.
    pub fn active_keystones(&self) -> Vec<usize> {
        match &self.cycle {
            Some(c) => c.launched_keystones().collect(),
            None => self.keystones.clone(),
        }
    }

    fn is_mammal(&self, taxon: usize) -> bool {
        let mut t = Some(taxon);
        while let Some(i) = t {
            if self.phy.taxa[i].name == "Mammal" {
                return true;
            }
            t = self.phy.taxa[i].parent;
        }
        false
    }

    fn in_team(&self, taxon: usize, team: Team) -> bool {
        let t = &self.phy.taxa[taxon];
        ecology::of(t.name).rule == Rule::Catch(Catch::Wildcard) || team.includes(t.name, t.group)
    }

    /// What `bonus` gives at strength `amount`.
    fn bonus_gains(bonus: Bonus, amount: f32) -> Vec<Gain> {
        if amount <= 0.0 {
            return Vec::new();
        }
        match bonus {
            Bonus::Luck => vec![Gain::Luck(amount)],
            Bonus::Cards => vec![Gain::Cards(EXTRA_CARD_PER_UNIT * amount)],
            Bonus::Morph => vec![Gain::Morph(MORPH_PER_UNIT * amount)],
            Bonus::Quick => vec![Gain::Quick(QUICK_PER_UNIT * amount)],
            Bonus::Point => vec![Gain::Points(1)],
            Bonus::Soil => vec![Gain::Soil(1)],
            Bonus::DoubleSpecimens => vec![Gain::DoubleSpecimens],
            Bonus::Share(h) => vec![Gain::Share(h, AFFINITY_PER_UNIT * amount)],
            Bonus::LivingFossil => vec![Gain::Luck(amount), Gain::Pity(30)],
            Bonus::Oddity => vec![Gain::Morph(MORPH_PER_UNIT * amount), Gain::MorphWindow(7)],
        }
    }

    /// Each active keystone's status and gains, in slot order. Copies are
    /// resolved against the others' own gains.
    pub fn keystone_reports(&self) -> Vec<Report> {
        let ks = self.active_keystones();
        let p = self.living_planet();
        let mut out: Vec<Report> = ks
            .iter()
            .map(|&k| {
                let eco = ecology::of(self.phy.taxa[k].name);
                if let Some(why) = self.dormant_reason(k) {
                    return Report {
                        taxon: k,
                        status: Status::Asleep(why),
                        gains: Vec::new(),
                        note: None,
                    };
                }
                let s = self.keystone_strength(k);
                let mult = self.edition(k).strength();
                let base = |amount: f32| Self::bonus_gains(eco.bonus, amount);
                let charges = self.charges[k];
                let others = ks.iter().filter(|&&o| o != k);
                let mut status = Status::Active;
                let mut note = None;
                let gains = match eco.rule {
                    Rule::Flat => base(s),
                    Rule::DoubleWhen(c) => base(if c.holds(&p) { 2.0 * s } else { s }),
                    Rule::OnlyWhen(c) => {
                        if c.holds(&p) {
                            base(s)
                        } else {
                            status = Status::Waiting(format!("only {}", c.describe()));
                            Vec::new()
                        }
                    }
                    Rule::Grows(_) | Rule::Stasis => {
                        note = Some(format!("{charges} charge{}", plural(charges as u32)));
                        base(0.5 * s * charges as f32)
                    }
                    Rule::PerTeam(team) => {
                        let n = others.filter(|&&o| self.in_team(o, team)).count();
                        note = Some(format!("{n} {}{}", team.name(), plural(n as u32)));
                        base(0.5 * s * n as f32)
                    }
                    Rule::DoubleWith(team) => {
                        let with = others.into_iter().any(|&o| self.in_team(o, team));
                        base(if with { 2.0 * s } else { s })
                    }
                    Rule::Fragile(n) => {
                        let left = n.saturating_sub(self.held[k]);
                        note = Some(format!("leaves in {left} cycle{}", plural(left as u32)));
                        base(3.0 * s)
                    }
                    Rule::Duplicates => {
                        note = Some(format!("{charges}/{MAX_DUPLICATE_CHARGES} charges"));
                        if charges > 0 {
                            vec![Gain::Cards(charges as f32 / 3.0 * mult)]
                        } else {
                            Vec::new()
                        }
                    }
                    Rule::Extremes => {
                        let n = Lever::ALL
                            .iter()
                            .filter(|&&l| p.get(l) == 0 || p.get(l) == p.max(l))
                            .count();
                        note = Some(format!("{n} extreme lever{}", plural(n as u32)));
                        if n > 0 {
                            vec![Gain::Luck(2.0 * n as f32 * mult)]
                        } else {
                            Vec::new()
                        }
                    }
                    Rule::CopyAbove | Rule::CopyBelowMammal | Rule::CopyStrongest => Vec::new(),
                    Rule::AfterRare => vec![Gain::AfterRare(s)],
                    Rule::AfterNew => vec![Gain::AfterNew(0.15 * s)],
                    Rule::Feed => vec![Gain::Feed],
                    Rule::NoNewPity => vec![Gain::NoNewPity(2)],
                    Rule::GiantWhen(c) => {
                        if c.holds(&p) {
                            vec![Gain::Giant(3.0)]
                        } else {
                            status = Status::Waiting(format!("only {}", c.describe()));
                            Vec::new()
                        }
                    }
                    Rule::Catch(c) => match c {
                        Catch::Tyrant => vec![Gain::Cards(2.0 * mult)],
                        Catch::Apex => vec![Gain::LuckMult(2.0), Gain::TempMin(3)],
                        Catch::Hubris => vec![
                            Gain::LuckMult(2.0),
                            Gain::Cards(2.0 * mult),
                            Gain::Doom(DOOM_CHANCE),
                        ],
                        Catch::Feathers => vec![Gain::MorphMult(3.0), Gain::FewerBoons],
                        Catch::Ancient => vec![Gain::Pity(25), Gain::LongerWait(3600.0)],
                        Catch::Wildcard => base(s),
                    },
                };
                Report {
                    taxon: k,
                    status,
                    gains,
                    note,
                }
            })
            .collect();

        // Copies take the copyable part of another keystone's own gains.
        let own: Vec<Vec<Gain>> = out.iter().map(|r| r.gains.clone()).collect();
        let copyable = |i: usize| -> bool {
            let rule = ecology::of(self.phy.taxa[ks[i]].name).rule;
            !matches!(
                rule,
                Rule::CopyAbove
                    | Rule::CopyBelowMammal
                    | Rule::CopyStrongest
                    | Rule::Catch(Catch::Wildcard)
            ) && !own[i].is_empty()
        };
        for (i, r) in out.iter_mut().enumerate() {
            if matches!(r.status, Status::Asleep(_)) {
                continue;
            }
            let rule = ecology::of(self.phy.taxa[ks[i]].name).rule;
            let target = match rule {
                Rule::CopyAbove => i.checked_sub(1).filter(|&j| copyable(j)),
                Rule::CopyBelowMammal => {
                    Some(i + 1).filter(|&j| j < ks.len() && copyable(j) && self.is_mammal(ks[j]))
                }
                Rule::CopyStrongest => {
                    (0..ks.len())
                        .filter(|&j| j != i && copyable(j))
                        .max_by(|&a, &b| {
                            let w = |j: usize| own[j].iter().map(|g| g.worth()).sum::<f32>();
                            w(a).total_cmp(&w(b))
                        })
                }
                _ => continue,
            };
            let half = if rule == Rule::CopyStrongest {
                0.5
            } else {
                1.0
            };
            match target {
                Some(j) => {
                    r.gains = own[j]
                        .iter()
                        .filter(|g| g.copyable())
                        .map(|g| g.scaled(half * self.edition(ks[i]).strength()))
                        .collect();
                    r.note = Some(format!("copying {}", self.phy.taxa[ks[j]].name));
                }
                None => r.status = Status::Waiting("nothing to copy".into()),
            }
        }
        out
    }

    /// What the keystones and current boon add up to. `extra_card`'s integer
    /// part is guaranteed cards, its fraction the chance of one more.
    pub fn effects(&self) -> Effects {
        let mut e = Effects::default();
        let mut luck_mult = 1.0;
        for g in self.keystone_reports().into_iter().flat_map(|r| r.gains) {
            match g {
                Gain::Luck(x) => e.luck += x,
                Gain::LuckMult(x) => luck_mult *= x,
                Gain::Cards(x) => e.extra_card += x,
                Gain::Morph(x) => e.morph += x,
                Gain::MorphMult(x) => e.morph_mult_keystone *= x,
                Gain::Quick(x) => e.quick += x,
                Gain::Points(n) => e.points += n,
                Gain::Soil(n) => e.soil += n,
                Gain::DoubleSpecimens => e.double_specimens = true,
                Gain::Share(h, x) => e.affinity[h.index()] += x,
                Gain::Pity(n) => e.legendary_pity = e.legendary_pity.min(n),
                Gain::MorphWindow(n) => {
                    e.morph_window = Some(e.morph_window.map_or(n, |w| w.min(n)))
                }
                Gain::Giant(x) => e.giant = e.giant.max(x),
                Gain::AfterRare(x) => e.after_rare_luck += x,
                Gain::AfterNew(x) => e.after_new_morph += x,
                Gain::Feed => e.feed = true,
                Gain::NoNewPity(n) => e.no_new_pity += n,
                Gain::FewerBoons => e.fewer_boons = true,
                Gain::LongerWait(s) => e.longer_wait += s,
                Gain::TempMin(t) => e.temp_min = e.temp_min.max(t),
                Gain::Doom(p) => e.doom = e.doom.max(p),
            }
        }
        e.luck = (e.luck * luck_mult).min(15.0);
        e.morph = e.morph.min(2.0);
        e.quick = e.quick.min(0.3);
        match self.boon {
            Some(Boon::Lens) => e.extra_card += 1.0,
            Some(Boon::Charm) => e.morph_mult_boon = 4.0,
            _ => {}
        }
        e
    }

    /// Why `lever` can't move by `delta` because of a keystone ("Megalodon
    /// holds it at Temperate or warmer"), if so.
    pub fn lever_lock(&self, lever: Lever, delta: i8) -> Option<String> {
        let v = self.planet.get(lever);
        for r in self.keystone_reports() {
            let name = self.phy.taxa[r.taxon].name;
            for g in &r.gains {
                match (*g, lever) {
                    (Gain::TempMin(t), Lever::Temperature) if delta < 0 && v <= t => {
                        return Some(format!(
                            "{name} holds it at {} or warmer",
                            planet::temperature_label(t)
                        ));
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Advances the charges and fragile timers of the keystones a finished
    /// wait ran with, on the planet it ran on.
    fn advance_keystones(&mut self, launched: Planet) {
        for k in self.active_keystones() {
            let rule = ecology::of(self.phy.taxa[k].name).rule;
            let awake = self.dormant_reason(k).is_none();
            let keeps = self.edition(k) == Morph::Amber;
            let grow = match rule {
                Rule::Grows(c) => Some(c.holds(&launched)),
                Rule::Stasis => Some(self.last_launched == Some(launched)),
                _ => None,
            };
            match grow {
                Some(true) if awake => {
                    self.charges[k] = (self.charges[k] + 1).min(MAX_CHARGES);
                }
                Some(_) if !keeps => self.charges[k] = 0,
                _ => {}
            }
            if let Rule::Fragile(n) = rule {
                if awake {
                    self.held[k] += 1;
                    if self.held[k] >= n {
                        self.held[k] = 0;
                        self.gone[k] = true;
                        self.keystones.retain(|&o| o != k);
                    }
                }
            }
        }
        self.last_launched = Some(launched);
    }

    pub fn max_points(&self) -> u8 {
        let e = self.effects();
        let tectonics = if self.boon == Some(Boon::Tectonics) {
            2
        } else {
            0
        };
        (self.wait_points + e.points as u8 + tectonics).min(MAX_POINTS)
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
        (self.phase() == Phase::Shape
            && self.lever_lock(lever, delta).is_none()
            && p.step(lever, delta)
            && self.cost(&p) <= self.max_points())
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
        let mut w = wait::bonus(hours);
        if !self.wait_choosable() {
            w.cards = wait::TUTORIAL_CARDS;
        }
        let mut affinity = [1.0; 6];
        for (a, add) in affinity.iter_mut().zip(e.affinity) {
            *a += add;
        }
        let cards = w.cards + e.extra_card;
        Forecast {
            cards,
            ma: wait::ma(hours),
            points: wait::points(hours),
            odds: Odds {
                cards: cards.floor() as usize,
                // Radiation adds past the usual cap.
                luck: (e.luck + w.luck).min(15.0) + RAD_LUCK * self.rad as f32,
                morph_mult: (1.0 + e.morph)
                    * e.morph_mult_boon
                    * e.morph_mult_keystone
                    * w.morph_mult,
                affinity,
                lure: match self.boon {
                    Some(Boon::Lure(t)) => Some(t),
                    _ => None,
                },
                catalyst: self.boon == Some(Boon::Catalyst) || w.sure_rare,
                legendary_pity: e.legendary_pity,
                morph_window: e.morph_window,
                giant_mult: e.giant,
                after_rare_luck: e.after_rare_luck,
                after_new_morph: e.after_new_morph,
                no_new_pity: e.no_new_pity,
                morph_rad: 1.0 + RAD_MORPH * self.rad as f32,
            },
        }
    }

    pub fn next_cycle_seconds(&self) -> f64 {
        let e = self.effects();
        let base =
            planet::cycle_seconds(self.cycles_done, self.next_hours(), e.quick) + e.longer_wait;
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
            keystones: {
                let mut ks = [None; planet::MAX_KEYSTONES];
                for (slot, &k) in ks.iter_mut().zip(&self.keystones) {
                    *slot = Some(k as u16);
                }
                ks
            },
        });
        true
    }

    /// Gives up the running wait: nothing is produced and the planet stays
    /// as it was launched, ready to be shaped again.
    pub fn cancel_cycle(&mut self) -> bool {
        self.cycle.take().is_some()
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
        // Decided with the genome, so reopening the app can't reroll it.
        let doom = self.effects().doom;
        self.doom_risk = doom;
        self.doomed = doom > 0.0 && self.rng.chance(doom);
        self.advance_keystones(cycle.launched);
        // The tutorial cycles keep the starting budget.
        self.wait_points = if self.wait_choosable() {
            wait::points(cycle.hours)
        } else {
            planet::BASE_POINTS
        };
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
        self.doom_risk = 0.0;
        let ma = self.ma_elapsed(now);
        let e = self.effects();
        let double = e.double_specimens;
        let starfish: Vec<usize> = self
            .keystone_reports()
            .into_iter()
            .filter(|r| {
                !matches!(r.status, Status::Asleep(_))
                    && ecology::of(self.phy.taxa[r.taxon].name).rule == Rule::Duplicates
            })
            .map(|r| r.taxon)
            .collect();
        let mut out = Vec::with_capacity(cards.len());
        for card in cards {
            let t = card.taxon;
            let level_before = self.level[t];
            if card.new && !self.unlocked[t] {
                self.unlocked[t] = true;
                // A fossil found again keeps its specimens and levels.
                self.specimens[t] = if self.fossil[t] {
                    self.specimens[t] + 1
                } else {
                    1
                };
                self.fossil[t] = true;
                self.found_ma[t] = Some(ma);
            } else {
                self.specimens[t] += if double { 2 } else { 1 };
                for &k in &starfish {
                    self.charges[k] = (self.charges[k] + 1).min(MAX_DUPLICATE_CHARGES);
                }
                // A spider keystone feeds the duplicate to a random keystone.
                if e.feed && !self.keystones.is_empty() {
                    let k = self.keystones
                        [(self.rng.next_u64() % self.keystones.len() as u64) as usize];
                    self.specimens[k] += 1;
                    self.level[k] = level_for(self.specimens[k]);
                }
            }
            self.gone[t] = false;
            let first_morph =
                card.morph != Morph::None && self.morphs[t] & morph_bit(card.morph) == 0;
            self.morphs[t] |= morph_bit(card.morph);
            // A first morph becomes the keystone's look, unless it costs
            // something (albino) or only changes reach (melanistic).
            if first_morph
                && self.edition[t] == 0
                && matches!(card.morph, Morph::Giant | Morph::Amber)
            {
                self.edition[t] = morph_bit(card.morph);
            }
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
        let n = if self.effects().fewer_boons { 2 } else { 3 };
        let mut offer = Vec::with_capacity(n);
        while offer.len() < n && !pool.is_empty() {
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

    /// Fossils: found on an earlier Earth, not yet on this one.
    pub fn is_fossil(&self, taxon: usize) -> bool {
        self.fossil[taxon] && !self.unlocked[taxon]
    }

    pub fn fossils(&self) -> usize {
        (0..self.phy.len()).filter(|&i| self.is_fossil(i)).count()
    }

    /// Human ended the Earth: +1 RAD and a new Earth from the Urmetazoan.
    /// Every animal found stays a fossil with its specimens and morphs;
    /// the planet, the spiral, Ma, keystones, pity and the genome reset.
    pub fn end_earth(&mut self) {
        let n = self.phy.len();
        self.rad += 1;
        for (f, &u) in self.fossil.iter_mut().zip(&self.unlocked) {
            *f |= u;
        }
        self.unlocked = vec![false; n];
        self.unlocked[Phylogeny::ROOT] = true;
        self.found_ma = vec![None; n];
        self.found_ma[Phylogeny::ROOT] = Some(0);
        self.planet = Planet::default();
        self.shaped_from = self.planet;
        self.cycle = None;
        self.ma_done = 0;
        self.genome = None;
        self.doomed = false;
        self.doom_risk = 0.0;
        self.pity = Pity {
            ever_morphed: self.pity.ever_morphed,
            ..Pity::default()
        };
        self.keystones.clear();
        self.boon = None;
        self.boon_offer = None;
        self.charges = vec![0; n];
        self.held = vec![0; n];
        self.gone = vec![false; n];
        self.last_launched = None;
        self.wait_points = planet::BASE_POINTS;
        self.refresh_levels();
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
    /// Adjustment points the wait gives the next shaping (before keystones).
    pub points: u8,
}

fn default_wait() -> f32 {
    wait::DEFAULT_HOURS
}

fn default_points() -> u8 {
    planet::BASE_POINTS
}

#[derive(Clone, Debug)]
pub struct Effects {
    pub affinity: [f32; 6],
    pub luck: f32,
    pub extra_card: f32,
    pub morph: f32,
    pub morph_mult_boon: f32,
    pub morph_mult_keystone: f32,
    pub quick: f32,
    pub points: u32,
    pub soil: u32,
    pub double_specimens: bool,
    pub legendary_pity: u32,
    pub morph_window: Option<u32>,
    pub giant: f32,
    pub after_rare_luck: f32,
    pub after_new_morph: f32,
    pub feed: bool,
    pub no_new_pity: u32,
    pub fewer_boons: bool,
    /// Seconds added to the wait.
    pub longer_wait: f64,
    /// The lowest Temperature a keystone allows.
    pub temp_min: u8,
    /// The chance the next genome ends the Earth (Human).
    pub doom: f32,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            affinity: [0.0; 6],
            luck: 0.0,
            extra_card: 0.0,
            morph: 0.0,
            morph_mult_boon: 1.0,
            morph_mult_keystone: 1.0,
            quick: 0.0,
            points: 0,
            soil: 0,
            double_specimens: false,
            legendary_pity: genome::LEGENDARY_PITY,
            morph_window: None,
            giant: 1.0,
            after_rare_luck: 0.0,
            after_new_morph: 0.0,
            feed: false,
            no_new_pity: 0,
            fewer_boons: false,
            longer_wait: 0.0,
            temp_min: 0,
            doom: 0.0,
        }
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
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
        assert_eq!(
            g.active_keystones(),
            vec![2, 3],
            "the launch snapshot counts"
        );
        g.skip_cycle(0.0);
        assert_eq!(g.phase(), Phase::Genome);
        assert!(!g.toggle_keystone(5), "locked once the genome is ready");
    }

    #[test]
    fn a_point_keystone_raises_the_budget() {
        let mut g = Game::new(0.0);
        let ant = equip(&mut g, "Bear");
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
            temperature: 1,
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
        assert_eq!((short.odds.cards, long.odds.cards), (1, 5));
        assert!(long.odds.luck > short.odds.luck);
        assert!(long.odds.catalyst && !short.odds.catalyst);
        assert_eq!((short.ma, long.ma), (20, 60));
    }

    #[test]
    fn the_wait_just_run_sets_the_next_budget() {
        let mut g = Game::new(0.0);
        assert_eq!(g.max_points(), planet::BASE_POINTS);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        let mut now = 0.0;
        for (hours, points) in [(2.0, 1), (4.0, 2), (6.0, 3)] {
            g.set_wait(hours);
            assert_eq!(g.forecast().points, points);
            run_cycle(&mut g, &mut now);
            g.choose_boon(usize::MAX);
            assert_eq!(g.max_points(), points, "after {hours}h");
        }
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

    fn equip(g: &mut Game, name: &str) -> usize {
        let t = g.phy.taxa.iter().position(|t| t.name == name).unwrap();
        g.unlocked[t] = true;
        g.specimens[t] = 1;
        g.refresh_levels();
        assert!(g.toggle_keystone(t), "couldn't equip {name}");
        t
    }

    fn report(g: &Game, t: usize) -> Report {
        g.keystone_reports()
            .into_iter()
            .find(|r| r.taxon == t)
            .unwrap()
    }

    fn luck(r: &Report) -> f32 {
        r.gains
            .iter()
            .map(|g| if let Gain::Luck(x) = g { *x } else { 0.0 })
            .sum()
    }

    const REEF: Planet = Planet {
        land: 1,
        vegetation: 0,
        oxygen: 3,
        temperature: 3,
        volcanism: 0,
    };

    #[test]
    fn coral_grows_while_warm_and_bleaches_in_the_cold() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        let coral = equip(&mut g, "Coral");
        assert_eq!(luck(&report(&g, coral)), 0.0);
        g.advance_keystones(REEF);
        g.advance_keystones(REEF);
        assert_eq!(g.charges[coral], 2);
        assert!(luck(&report(&g, coral)) > 0.0);
        g.advance_keystones(Planet {
            temperature: 1,
            ..REEF
        });
        assert_eq!(g.charges[coral], 0, "bleached");
    }

    #[test]
    fn amber_keeps_its_charges() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        let coral = equip(&mut g, "Coral");
        g.morphs[coral] = morph_bit(Morph::Amber);
        g.edition[coral] = morph_bit(Morph::Amber);
        g.advance_keystones(REEF);
        g.planet.temperature = 1;
        g.advance_keystones(g.planet);
        assert_eq!(g.charges[coral], 1);
    }

    #[test]
    fn the_octopus_copies_the_keystone_above() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        let coral = equip(&mut g, "Coral");
        let octo = equip(&mut g, "Octopus");
        g.charges[coral] = 4;
        let (c, o) = (report(&g, coral), report(&g, octo));
        assert!(luck(&c) > 0.0);
        assert_eq!(luck(&o), luck(&c));
        assert_eq!(o.note.as_deref(), Some("copying Coral"));
    }

    #[test]
    fn megalodon_doubles_luck_and_locks_the_cold_out() {
        let mut g = Game::new(0.0);
        g.planet = Planet { land: 1, ..REEF };
        let fossil = equip(&mut g, "Coelacanth");
        g.charges[fossil] = 3;
        let before = g.effects().luck;
        equip(&mut g, "Megalodon");
        assert_eq!(g.effects().luck, (before * 2.0).min(15.0));
        assert!(!g.can_step(Lever::Temperature, -1));
        assert!(g.lever_lock(Lever::Temperature, -1).is_some());
        assert!(g.lever_lock(Lever::Temperature, 1).is_none());
    }

    #[test]
    fn a_t_rex_puts_plant_eaters_to_sleep() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 5,
            volcanism: 1,
        };
        let saur = equip(&mut g, "Sauropod");
        assert_eq!(g.dormant_reason(saur), None);
        equip(&mut g, "T. rex");
        assert_eq!(
            g.dormant_reason(saur).as_deref(),
            Some("hunted by the T. rex")
        );
    }

    #[test]
    fn albinos_sunburn_and_melanistic_coats_live_colder() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 3,
            volcanism: 0,
        };
        let ape = equip(&mut g, "Ape");
        g.morphs[ape] = morph_bit(Morph::Albino) | morph_bit(Morph::Melanistic);
        g.edition[ape] = morph_bit(Morph::Albino);
        assert!(g.dormant_reason(ape).unwrap().contains("albino"));
        g.edition[ape] = morph_bit(Morph::Melanistic);
        g.planet.temperature = 1;
        assert_eq!(g.dormant_reason(ape), None, "two steps colder");
        g.edition[ape] = 0;
        assert_eq!(g.dormant_reason(ape).as_deref(), Some("too cold"));
    }

    #[test]
    fn editions_cycle_through_owned_morphs() {
        let mut g = Game::new(0.0);
        let t = equip(&mut g, "Urmetazoan");
        assert!(!g.cycle_edition(t), "nothing to pick");
        g.morphs[t] = morph_bit(Morph::Giant) | morph_bit(Morph::Amber);
        assert!(g.cycle_edition(t));
        assert_eq!(g.edition(t), Morph::Giant);
        g.cycle_edition(t);
        assert_eq!(g.edition(t), Morph::Amber);
        g.cycle_edition(t);
        assert_eq!(g.edition(t), Morph::None);
    }

    #[test]
    fn farm_animals_count_each_other_and_the_dog_herds_them() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 2,
            oxygen: 3,
            temperature: 3,
            volcanism: 0,
        };
        let dog = equip(&mut g, "Dog");
        let alone = luck(&report(&g, dog));
        let cow = equip(&mut g, "Cow");
        assert_eq!(luck(&report(&g, dog)), 2.0 * alone);
        assert_eq!(luck(&report(&g, cow)), 0.0, "no other farm animal");
        equip(&mut g, "Pig");
        assert!(luck(&report(&g, cow)) > 0.0);
    }

    #[test]
    fn the_dodo_leaves_after_three_cycles_until_found_again() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 5,
            oxygen: 3,
            temperature: 4,
            volcanism: 0,
        };
        let dodo = equip(&mut g, "Dodo");
        for _ in 0..3 {
            g.advance_keystones(g.planet);
        }
        assert!(!g.keystones.contains(&dodo));
        assert!(!g.toggle_keystone(dodo), "gone until found again");
    }

    /// A sea world where the Human keystone is awake for the tests.
    fn with_human() -> (Game, usize) {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 4,
            vegetation: 2,
            oxygen: 3,
            temperature: 3,
            volcanism: 0,
        };
        let human = equip(&mut g, "Human");
        (g, human)
    }

    #[test]
    fn human_doubles_luck_adds_cards_and_risks_the_earth() {
        let (g, human) = with_human();
        let r = report(&g, human);
        assert_eq!(r.status, Status::Active);
        let e = g.effects();
        assert_eq!(e.doom, DOOM_CHANCE);
        assert!(e.extra_card >= 2.0);
        assert!(r.gains.contains(&Gain::LuckMult(2.0)));
    }

    #[test]
    fn about_one_genome_in_five_is_doomed_with_human() {
        let (mut g, _) = with_human();
        g.cycles_done = planet::TUTORIAL_CYCLES;
        let mut now = 0.0;
        let mut doomed = 0;
        for _ in 0..200 {
            g.keystones = vec![g.keystones[0]];
            g.planet = with_human().0.planet;
            g.accelerate(now);
            now += g.cycle.unwrap().duration + 1.0;
            g.tick(now);
            assert_eq!(g.doom_risk, DOOM_CHANCE);
            doomed += g.doomed as u32;
            g.genome = None;
            g.doomed = false;
        }
        assert!((20..=60).contains(&doomed), "{doomed} of 200");
    }

    #[test]
    fn ending_the_earth_keeps_fossils_and_adds_rad() {
        let (mut g, human) = with_human();
        g.specimens[human] = 9;
        g.morphs[human] = morph_bit(Morph::Amber);
        g.ma_done = 500;
        g.end_earth();
        assert_eq!(g.rad, 1);
        assert_eq!(g.discovered(), 1, "back to the Urmetazoan");
        assert!(g.is_fossil(human));
        assert_eq!(g.fossils(), 1);
        assert!(g.keystones.is_empty());
        assert_eq!(g.ma_done, 0);
        assert_eq!(g.planet, Planet::default());
        assert_eq!(g.morphs[human], morph_bit(Morph::Amber), "morphs kept");
        assert!(!g.toggle_keystone(human), "a fossil must be found again");
        let odds = g.forecast().odds;
        assert!(odds.luck >= RAD_LUCK && odds.morph_rad == 1.0 + RAD_MORPH);
    }

    #[test]
    fn a_fossil_found_again_keeps_its_level() {
        let (mut g, human) = with_human();
        g.specimens[human] = 9;
        g.end_earth();
        g.genome = Some(vec![Card {
            taxon: human,
            tier: Tier::Legendary,
            morph: Morph::None,
            new: true,
            note: None,
        }]);
        g.open_genome(0.0);
        assert!(g.unlocked[human] && !g.is_fossil(human));
        assert_eq!(g.specimens[human], 10);
        assert_eq!(g.level[human], level_for(10));
    }

    #[test]
    fn cards_are_no_longer_capped() {
        let (mut g, _) = with_human();
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.set_wait(6.0);
        g.boon = Some(Boon::Lens);
        assert!(g.forecast().odds.cards >= 8, "5 + 2 + 1");
    }
}
