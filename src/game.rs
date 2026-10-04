//! Game state and its loop: Shape the planet -> time Running -> a new genome
//! (`Genome`) waits to be opened -> pick a `Boon` -> Shape again.

use serde::{Deserialize, Serialize};

use crate::ecology::{self, Bonus, Catch, Kin, Rule, Tier};
use crate::genome::{self, Card, Morph, Odds, Pity, Rng};
use crate::planet::{self, Biome, Cycle, Lever, Planet};
use crate::tree::{Phylogeny, Taxon};
use crate::wait;

/// Specimens needed to go from level `n` to `n + 1` (index `n - 1`).
const LEVEL_STEPS: [u32; 4] = [1, 2, 4, 8];
pub const MAX_LEVEL: u32 = 5;
/// Adjustment points cap, whatever the keystones and boons give.
const MAX_POINTS: u8 = 8;
const BASE_KEYSTONE_SLOTS: usize = 3;
/// Keystone effects per unit of strength (tier units x level x morph).
pub const EXTRA_CARD_PER_UNIT: f32 = 0.12;
pub const DISCOVERY_PER_UNIT: f32 = 0.03;
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
    /// +25% chance each card is a new species.
    Discovery,
    /// +1 card.
    Lens,
    /// A guaranteed Rare+ card (Epic+ on a 6h wait).
    Catalyst,
    /// x4 morph chance.
    Charm,
    /// 25% off the wait.
    Tailwind,
    /// Adjustment points this shaping, scaled with the wait just run.
    Tectonics,
}

impl Boon {
    pub fn title(self) -> &'static str {
        match self {
            Boon::Discovery => "Discovery",
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
            // The angler's lure: a light into the unknown.
            Boon::Discovery => 0,
            Boon::Lens => 1,
            Boon::Catalyst => 2,
            Boon::Charm => 3,
            Boon::Tailwind => 4,
            Boon::Tectonics => 5,
        }
    }

    /// What it does, for the boon pick (right after a wait).
    pub fn describe(self, game: &Game) -> String {
        match self {
            Boon::Discovery => format!(
                "+{:.0}% new species in the next genome",
                DISCOVERY_BOON * 100.0
            ),
            Boon::Lens => "+1 card in the next genome".into(),
            Boon::Catalyst => "A sure Rare, or a sure Epic on a 6h wait".into(),
            Boon::Charm => "x4 morph chance".into(),
            Boon::Tailwind => "25% less to wait".into(),
            Boon::Tectonics => {
                let n = game.tectonics_points();
                format!("+{n} adjustment point{}", plural(n.into()))
            }
        }
    }
}

/// The chance a 4h wait with Human ends the Earth (it compounds per hour).
pub const DOOM_CHANCE: f32 = 0.2;
/// The most luck keystones and biomes give together, after any x2: the
/// wait's luck comes on top, so it never crowds them out.
pub const KEYSTONE_LUCK_MAX: f32 = 20.0;
/// Per RAD: luck past the cap, and morph chance added (x1.25, x1.5, ...).
pub const RAD_LUCK: f32 = 5.0;
pub const RAD_MORPH: f32 = 0.25;

/// `Game::edition`: the player chose this morph rather than the best one.
const PICKED: u8 = 0x80;

/// Ma of growth per charge of the Tuatara: a charge per default 4h wait, so
/// short waits grow no faster than long ones.
pub const CHARGE_MA: u16 = 40;
/// Most charges a growing keystone holds...
const MAX_CHARGES: u16 = 5;
const MAX_GROWTH: u16 = MAX_CHARGES * CHARGE_MA;
/// ...each worth this much of its base bonus: x2 when full.
const CHARGE_BONUS: f32 = 0.2;
/// Tailwind's cut of the wait.
const TAILWIND: f64 = 0.25;
/// Tectonics' points for a default 4h wait, scaled with the wait just run.
const TECTONICS_POINTS: f32 = 2.0;
/// The most a card can be a new species once the tutorial is over: some
/// are always animals already owned.
pub const DISCOVERY_MAX: f32 = 0.9;
/// The Discovery boon's chance of a new species, added per card.
const DISCOVERY_BOON: f32 = 0.25;
/// The Tuatara's catch: its morphs multiplier.
const TUATARA_MORPHS: f32 = 0.5;
/// The Coelacanth's catch: the chance of a new species it takes away.
const COELACANTH_DISCOVERY: f32 = 0.15;
/// The Platypus's longer wait.
const PLATYPUS_WAIT: f32 = 0.5;

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
    /// Added to the chance a card is a new species.
    Discovery(f32),
    Morph(f32),
    Quick(f32),
    /// Adjustment points per 4h waited.
    Points(u32),
    /// The wait runs this much longer (0.25: +25%).
    SlowerWait(f32),
    MorphMult(f32),
    FewerBoons,
    /// The lowest Temperature the planet may be shaped to.
    TempMin(u8),
    /// The chance a 4h wait ends the Earth instead of opening.
    Doom(f32),
}

impl Gain {
    /// Added to the total rather than a rule change: what copies can copy.
    fn copyable(self) -> bool {
        matches!(
            self,
            Gain::Luck(_)
                | Gain::Cards(_)
                | Gain::Discovery(_)
                | Gain::Morph(_)
                | Gain::Quick(_)
                | Gain::Points(_)
        )
    }

    fn scaled(self, f: f32) -> Gain {
        match self {
            Gain::Luck(x) => Gain::Luck(x * f),
            Gain::Cards(x) => Gain::Cards(x * f),
            Gain::Discovery(x) => Gain::Discovery(x * f),
            Gain::Morph(x) => Gain::Morph(x * f),
            Gain::Quick(x) => Gain::Quick(x * f),
            g => g,
        }
    }

    /// "+3.0 Luck", for the keystone lists.
    pub fn label(self) -> String {
        match self {
            Gain::Luck(x) => format!("+{x:.1} Luck"),
            Gain::LuckMult(x) => format!("x{x:.0} Luck"),
            Gain::Cards(x) if x >= 1.0 => format!("+{x:.1} cards"),
            Gain::Cards(x) => format!("+{:.0}% card", x * 100.0),
            Gain::Discovery(x) if x < 0.0 => format!("{:.0}% discovery", x * 100.0),
            Gain::Discovery(x) => format!("+{:.0}% discovery", x * 100.0),
            Gain::Morph(x) => format!("+{:.0}% morphs", x * 100.0),
            Gain::Quick(x) => format!("-{:.0}% wait", x * 100.0),
            Gain::Points(n) => format!("+{n} point{} per 4h", plural(n)),
            Gain::SlowerWait(x) => format!("+{:.0}% wait", x * 100.0),
            Gain::MorphMult(x) if x < 1.0 => format!("x{x:.1} morphs"),
            Gain::MorphMult(x) => format!("x{x:.0} morphs"),
            Gain::FewerBoons => "-1 boon".into(),
            Gain::TempMin(_) => "Temperature locked".into(),
            Gain::Doom(p) => format!("{:.0}% per 4h the Earth ends", p * 100.0),
        }
    }
}

/// What one equipped keystone does right now.
#[derive(Clone, Debug)]
pub struct Report {
    pub taxon: usize,
    pub status: Status,
    pub gains: Vec<Gain>,
    /// Live state worth showing: "4 charges", "copying Shark".
    pub note: Option<String>,
    /// A copier's gains are another keystone's (this one).
    pub copied_from: Option<usize>,
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
        } else if self.copied_from.is_some() {
            format!("{}  copied", parts.join("  ·  "))
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

/// What a save is checked against before the game itself is read.
#[derive(Deserialize)]
struct SaveHeader {
    version: u32,
    /// Absent before the wait could be chosen: every cycle was 20 Ma.
    #[serde(default)]
    ma_done: Option<u32>,
    /// Absent before keystone points were earned by waiting.
    #[serde(default)]
    bonus_points: Option<u8>,
}

/// The game as saved: its own fields plus the format version.
#[derive(Serialize)]
struct Versioned<'a> {
    version: u32,
    #[serde(flatten)]
    game: &'a Game,
}

/// Saved as is, but for the tree (static) and the levels (derived).
/// The aliases and defaults read saves from older builds.
#[derive(Serialize, Deserialize)]
pub struct Game {
    #[serde(skip, default = "Phylogeny::load")]
    pub phy: Phylogeny,
    pub unlocked: Vec<bool>,
    /// 0 until discovered, then 1..=`MAX_LEVEL`, derived from `specimens`.
    #[serde(skip)]
    pub level: Vec<u32>,
    /// Copies collected per taxon, the discovery included.
    pub specimens: Vec<u32>,
    /// `Morph::bit` flags of the morphs owned per taxon.
    pub morphs: Vec<u8>,
    /// Millions of years elapsed when each taxon evolved.
    #[serde(alias = "found_day")]
    pub found_ma: Vec<Option<u32>>,
    pub planet: Planet,
    /// Points are spent as distance from this, so stepping back refunds.
    pub shaped_from: Planet,
    pub cycle: Option<Cycle>,
    pub cycles_done: u32,
    /// Millions of years of the finished cycles (see `SaveHeader`).
    #[serde(default)]
    pub ma_done: u32,
    /// The wait picked on the dial, reused as the next default.
    #[serde(default = "default_wait")]
    pub wait_hours: f32,
    #[serde(alias = "nodule")]
    pub genome: Option<Vec<Card>>,
    pub pity: Pity,
    rng: Rng,
    #[serde(alias = "patrons")]
    pub keystones: Vec<usize>,
    pub boon: Option<Boon>,
    pub boon_offer: Option<Vec<Boon>>,
    /// Ma a growing keystone has grown (a charge per `CHARGE_MA`).
    #[serde(default)]
    pub growth: Vec<u16>,
    /// A fragile keystone (the Dodo) worn out: asleep until a duplicate of
    /// it is opened.
    #[serde(default)]
    pub gone: Vec<bool>,
    /// The morph each animal works with as a keystone: `PICKED` and its
    /// `Morph::bit` (none: 0) once chosen on the MORPH button; without
    /// `PICKED`, the best one. Saves from before `PICKED` all read as best.
    #[serde(default)]
    pub edition: Vec<u8>,
    /// The planet the last wait ran on, for the keystones that like stasis.
    #[serde(default)]
    pub last_launched: Option<Planet>,
    /// Adjustment points the last wait gave this shaping, before keystones.
    #[serde(default = "default_points")]
    pub wait_points: u8,
    /// Points the keystones earned during the last wait (`Gain::Points`
    /// per 4h), and the fraction carried to the next.
    #[serde(default)]
    pub bonus_points: u8,
    #[serde(default)]
    pub point_carry: f32,
    /// The wait just run, in hours: what Tectonics scales with.
    #[serde(default = "default_wait")]
    pub last_hours: f32,
    /// Earths Human has ended: each one boosts rarity and morphs.
    #[serde(default)]
    pub rad: u32,
    /// Every taxon found on any Earth. One not found on this Earth is a
    /// fossil: it keeps its level and morphs and waits to be found again.
    #[serde(default)]
    pub fossil: Vec<bool>,
    /// The waiting genome ends the Earth when opened (rolled with it).
    #[serde(default)]
    pub doomed: bool,
    /// The chance that roll had, to show while the genome waits.
    #[serde(default)]
    pub doom_risk: f32,
    /// The last animal discovered, for the leaderboard.
    #[serde(default)]
    pub last_found: Option<usize>,
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
            growth: vec![0; n],
            gone: vec![false; n],
            edition: vec![0; n],
            last_launched: None,
            wait_points: planet::BASE_POINTS,
            bonus_points: 0,
            point_carry: 0.0,
            last_hours: wait::DEFAULT_HOURS,
            rad: 0,
            fossil: vec![false; n],
            doomed: false,
            doom_risk: 0.0,
            last_found: None,
        };
        game.fossil[Phylogeny::ROOT] = true;
        game.refresh_levels();
        game
    }

    pub fn save(&self) {
        crate::save::write(&self.to_json());
    }

    fn to_json(&self) -> String {
        let data = Versioned {
            version: SAVE_VERSION,
            game: self,
        };
        serde_json::to_string(&data).unwrap_or_default()
    }

    /// `None` for a missing, old-format or mismatched save.
    pub fn load(now: f64) -> Option<Self> {
        Self::from_json(&crate::save::read()?, now)
    }

    fn from_json(json: &str, now: f64) -> Option<Self> {
        let header: SaveHeader = serde_json::from_str(json).ok()?;
        let mut value: serde_json::Value = serde_json::from_str(json).ok()?;
        retire_lure(&mut value);
        let mut game: Self = serde_json::from_value(value).ok()?;
        let n = game.phy.len();
        let saved = game.unlocked.len();
        if header.version != SAVE_VERSION
            || saved > n
            || game.specimens.len() != saved
            || game.morphs.len() != saved
            || game.found_ma.len() != saved
            || game.keystones.iter().any(|&p| p >= saved)
            || game.last_found.is_some_and(|t| t >= saved)
        {
            return None;
        }
        // Taxa added since the save was written are appended to the tree.
        game.unlocked.resize(n, false);
        game.specimens.resize(n, 0);
        game.morphs.resize(n, 0);
        game.found_ma.resize(n, None);
        game.growth.resize(n, 0);
        game.gone.resize(n, false);
        game.edition.resize(n, 0);
        game.fossil.resize(n, false);
        for (f, &u) in game.fossil.iter_mut().zip(&game.unlocked) {
            *f |= u;
        }
        game.level = vec![0; n];
        game.ma_done = header.ma_done.unwrap_or(game.cycles_done * 20);
        game.wait_hours = wait::snap(game.wait_hours);
        game.pity.migrate();
        game.refresh_levels();
        // Keystone points used to count at once: this shaping keeps them.
        if header.bonus_points.is_none() {
            game.bonus_points = game.effects().points.min(MAX_POINTS.into()) as u8;
        }
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

    /// The morph `taxon` works with as a keystone: the one picked on the
    /// MORPH button, or by default its best (`best_edition`).
    pub fn edition(&self, taxon: usize) -> Morph {
        if self.edition_is_best(taxon) {
            self.best_edition(taxon)
        } else {
            Morph::from_bit(self.edition[taxon] & self.morphs[taxon])
        }
    }

    /// No morph picked: the keystone wears its best one.
    pub fn edition_is_best(&self, taxon: usize) -> bool {
        self.edition[taxon] & PICKED == 0
    }

    /// The owned morph (or none) that makes the strongest keystone on the
    /// planet it lives on: awake first, then `Morph::rank`. So an albino
    /// coat is worn only where it's cool enough, and a melanistic one where
    /// only it keeps the animal awake.
    fn best_edition(&self, taxon: usize) -> Morph {
        std::iter::once(Morph::None)
            .chain(
                Morph::ALL
                    .into_iter()
                    .filter(|&m| self.owns_morph(taxon, m)),
            )
            .max_by(|&a, &b| {
                let awake = |m| self.own_sleep_with(taxon, m).is_none();
                awake(a).cmp(&awake(b)).then(a.rank().cmp(&b.rank()))
            })
            .unwrap_or(Morph::None)
    }

    pub fn owns_morph(&self, taxon: usize, m: Morph) -> bool {
        self.morphs[taxon] & m.bit() != 0
    }

    /// The best morph owned, for drawing a collection entry.
    pub fn best_morph(&self, taxon: usize) -> Morph {
        Morph::ALL
            .into_iter()
            .rev()
            .find(|&m| self.owns_morph(taxon, m))
            .unwrap_or(Morph::None)
    }

    /// Steps `taxon`'s keystone morph while shaping: best, then none, then
    /// each morph it owns, then back to best. Returns whether it changed.
    pub fn cycle_edition(&mut self, taxon: usize) -> bool {
        if self.phase() != Phase::Shape || self.morphs[taxon] == 0 {
            return false;
        }
        let states: Vec<u8> = [0, PICKED]
            .into_iter()
            .chain(
                Morph::ALL
                    .into_iter()
                    .filter(|&m| self.owns_morph(taxon, m))
                    .map(|m| PICKED | m.bit()),
            )
            .collect();
        let cur = if self.edition_is_best(taxon) {
            0
        } else {
            PICKED | self.edition(taxon).bit()
        };
        let i = states.iter().position(|&s| s == cur).unwrap_or(0);
        self.edition[taxon] = states[(i + 1) % states.len()];
        true
    }

    /// Tier units x level x the morph's multiplier.
    pub fn keystone_strength(&self, taxon: usize) -> f32 {
        let eco = self.phy.taxa[taxon].eco;
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
        self.own_sleep_with(taxon, self.edition(taxon))
    }

    /// As `own_sleep`, wearing `morph`.
    fn own_sleep_with(&self, taxon: usize, morph: Morph) -> Option<String> {
        let eco = self.phy.taxa[taxon].eco;
        if eco.rule == Rule::Fragile && self.gone[taxon] {
            return Some("worn out".into());
        }
        if eco.rule == Rule::Extremes {
            return None;
        }
        let p = self.living_planet();
        if let Some(why) = eco.needs.missing_with(&p, morph == Morph::Melanistic) {
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
        if self.tyrants_prey() == Some(taxon) {
            return Some("eaten by the T. rex".into());
        }
        None
    }

    /// The keystone an awake T. rex puts to sleep: the one in the last slot,
    /// or the one before if the T. rex is last.
    fn tyrants_prey(&self) -> Option<usize> {
        let ks = self.active_keystones();
        let rex = ks.iter().copied().find(|&k| {
            self.phy.taxa[k].eco.rule == Rule::Catch(Catch::Tyrant) && self.own_sleep(k).is_none()
        })?;
        ks.iter().rev().copied().find(|&k| k != rex)
    }

    /// The keystones that count: those equipped when the running wait was
    /// launched, else the equipped ones.
    pub fn active_keystones(&self) -> Vec<usize> {
        match &self.cycle {
            Some(c) => c.launched_keystones().collect(),
            None => self.keystones.clone(),
        }
    }

    /// Whether `taxon` descends from the taxon named `ancestor`.
    fn descends(&self, taxon: usize, ancestor: &str) -> bool {
        let mut t = Some(taxon);
        while let Some(i) = t {
            if self.phy.taxa[i].name == ancestor {
                return true;
            }
            t = self.phy.taxa[i].parent;
        }
        false
    }

    pub fn is_kin(&self, taxon: usize, kin: Kin) -> bool {
        let (clade, not_under) = kin.clade();
        self.descends(taxon, clade) && !not_under.is_some_and(|n| self.descends(taxon, n))
    }

    /// What `bonus` gives at strength `amount`.
    fn bonus_gains(bonus: Bonus, amount: f32) -> Vec<Gain> {
        if amount <= 0.0 {
            return Vec::new();
        }
        vec![match bonus {
            Bonus::Luck => Gain::Luck(amount),
            Bonus::Cards => Gain::Cards(EXTRA_CARD_PER_UNIT * amount),
            Bonus::Discovery => Gain::Discovery(DISCOVERY_PER_UNIT * amount),
            Bonus::Morph => Gain::Morph(MORPH_PER_UNIT * amount),
            Bonus::Quick => Gain::Quick(QUICK_PER_UNIT * amount),
            Bonus::Point => Gain::Points(1),
        }]
    }

    /// Each active keystone's status and gains, in slot order. Copies are
    /// resolved against the others' own gains.
    pub fn keystone_reports(&self) -> Vec<Report> {
        let ks = self.active_keystones();
        let p = self.living_planet();
        let mut out: Vec<Report> = ks
            .iter()
            .map(|&k| {
                let eco = self.phy.taxa[k].eco;
                let mut report = Report {
                    taxon: k,
                    status: Status::Active,
                    gains: Vec::new(),
                    note: None,
                    copied_from: None,
                };
                if let Some(why) = self.dormant_reason(k) {
                    report.status = Status::Asleep(why);
                    return report;
                }
                let s = self.keystone_strength(k);
                let mult = self.edition(k).strength();
                let base = |amount: f32| Self::bonus_gains(eco.bonus, amount);
                report.gains = match eco.rule {
                    Rule::Flat => base(s),
                    // Resolved below, against the others.
                    Rule::Copy(_) => Vec::new(),
                    Rule::Extremes => {
                        let n = Lever::ALL
                            .iter()
                            .filter(|&&l| p.get(l) == 0 || p.get(l) == p.max(l))
                            .count();
                        report.note = Some(format!("{n} extreme lever{}", plural(n as u32)));
                        if n > 0 {
                            vec![Gain::Luck(2.0 * n as f32 * mult)]
                        } else {
                            Vec::new()
                        }
                    }
                    Rule::Fragile => {
                        report.note = Some("asleep after this cycle".into());
                        base(3.0 * s)
                    }
                    Rule::Changing | Rule::Stasis => {
                        // Saves from when 10 charges fit are held to the new most.
                        let grown =
                            f32::from(self.growth[k].min(MAX_GROWTH)) / f32::from(CHARGE_MA);
                        let count = if grown.fract() == 0.0 {
                            format!("{grown:.0}")
                        } else {
                            format!("{grown:.1}")
                        };
                        report.note = Some(format!("{count}/{MAX_CHARGES} charges"));
                        // Its base bonus from the start, growing to x2.
                        let mut gains = base(s * (1.0 + CHARGE_BONUS * grown));
                        // Their catches: fewer morphs, fewer new species.
                        gains.push(match eco.rule {
                            Rule::Changing => Gain::MorphMult(TUATARA_MORPHS),
                            _ => Gain::Discovery(-COELACANTH_DISCOVERY),
                        });
                        gains
                    }
                    Rule::Catch(Catch::Tyrant) => vec![Gain::Cards(2.0 * mult)],
                    Rule::Catch(Catch::Apex) => vec![Gain::LuckMult(2.0), Gain::TempMin(3)],
                    Rule::Catch(Catch::Feathers) => vec![Gain::MorphMult(3.0), Gain::FewerBoons],
                    Rule::Catch(Catch::Hubris) => vec![
                        Gain::LuckMult(2.0),
                        Gain::Cards(2.0 * mult),
                        Gain::Doom(DOOM_CHANCE),
                    ],
                    // Its biome bonuses are added with the others' (`effects`).
                    Rule::Catch(Catch::AllBiomes) => vec![Gain::SlowerWait(PLATYPUS_WAIT)],
                };
                report
            })
            .collect();

        // A copier takes the copyable gains of the keystone above it, if
        // that one is of its kin and not a Legendary.
        let own: Vec<Vec<Gain>> = out.iter().map(|r| r.gains.clone()).collect();
        for (i, r) in out.iter_mut().enumerate() {
            let Rule::Copy(kin) = self.phy.taxa[ks[i]].eco.rule else {
                continue;
            };
            if matches!(r.status, Status::Asleep(_)) {
                continue;
            }
            let target = i.checked_sub(1).filter(|&j| {
                let t = ks[j];
                self.is_kin(t, kin)
                    && self.phy.taxa[t].eco.tier != Tier::Legendary
                    && !own[j].is_empty()
            });
            match target {
                Some(j) => {
                    let f = self.edition(ks[i]).strength();
                    r.gains = own[j]
                        .iter()
                        .filter(|g| g.copyable())
                        .map(|g| g.scaled(f))
                        .collect();
                    r.note = Some(format!("copying {}", self.phy.taxa[ks[j]].name));
                    r.copied_from = Some(ks[j]);
                }
                None => {
                    r.status = Status::Waiting(format!("needs a {} keystone above it", kin.name()))
                }
            }
        }
        // A giant reshapes its world: a point while it's awake, added after
        // the copies so that none of them copies it.
        for (r, &k) in out.iter_mut().zip(&ks) {
            if self.edition(k) == Morph::Giant && !matches!(r.status, Status::Asleep(_)) {
                r.gains.push(Gain::Points(1));
            }
        }
        out
    }

    /// The biomes whose bonus is on: `ecology::BIOME_SET` awake keystones
    /// of the biome, or every biome with an awake Platypus.
    pub fn active_biomes(&self) -> Vec<Biome> {
        self.biomes_from(&self.keystone_reports())
    }

    fn biomes_from(&self, reports: &[Report]) -> Vec<Biome> {
        let awake = || {
            reports
                .iter()
                .filter(|r| !matches!(r.status, Status::Asleep(_)))
                .map(|r| self.phy.taxa[r.taxon].eco)
        };
        if awake().any(|e| e.rule == Rule::Catch(Catch::AllBiomes)) {
            return Biome::ALL.to_vec();
        }
        Biome::ALL
            .into_iter()
            .filter(|&b| awake().filter(|e| e.needs.biome == Some(b)).count() >= ecology::BIOME_SET)
            .collect()
    }

    /// What the keystones, their biomes and the current boon add up to.
    /// `extra_card`'s integer part is guaranteed cards, its fraction the
    /// chance of one more.
    pub fn effects(&self) -> Effects {
        let mut e = Effects::default();
        let mut luck_mult = 1.0;
        let reports = self.keystone_reports();
        let biomes = self.biomes_from(&reports);
        let biome_gains = biomes.into_iter().map(|b| match ecology::biome_bonus(b) {
            (Bonus::Luck, x) => Gain::Luck(x),
            (Bonus::Cards, x) => Gain::Cards(x),
            (Bonus::Discovery, x) => Gain::Discovery(x),
            (Bonus::Morph, x) => Gain::Morph(x),
            (Bonus::Quick, x) => Gain::Quick(x),
            (Bonus::Point, x) => Gain::Points(x as u32),
        });
        for g in reports.into_iter().flat_map(|r| r.gains).chain(biome_gains) {
            match g {
                Gain::Luck(x) => e.luck += x,
                Gain::LuckMult(x) => luck_mult *= x,
                Gain::Cards(x) => e.extra_card += x,
                Gain::Discovery(x) => e.discovery += x,
                Gain::Morph(x) => e.morph += x,
                Gain::Quick(x) => e.quick += x,
                Gain::Points(n) => e.points += n,
                Gain::SlowerWait(x) => e.slower += x,
                Gain::MorphMult(x) => e.morph_mult_keystone *= x,
                Gain::FewerBoons => e.fewer_boons = true,
                Gain::TempMin(t) => e.temp_min = e.temp_min.max(t),
                Gain::Doom(p) => e.doom = e.doom.max(p),
            }
        }
        e.luck = (e.luck * luck_mult).min(KEYSTONE_LUCK_MAX);
        e.morph = e.morph.min(2.0);
        e.quick = e.quick.min(0.3);
        if self.boon == Some(Boon::Charm) {
            e.morph_mult_boon = 4.0;
        }
        e
    }

    /// Why `lever` can't move by `delta` because of a keystone ("Megalodon
    /// holds it at Temperate or warmer"), if so.
    pub fn lever_lock(&self, lever: Lever, delta: i8) -> Option<String> {
        if lever != Lever::Temperature || delta >= 0 {
            return None;
        }
        let v = self.planet.get(lever);
        self.keystone_reports().into_iter().find_map(|r| {
            r.gains.iter().find_map(|g| match *g {
                Gain::TempMin(t) if v <= t => Some(format!(
                    "{} holds it at {} or warmer",
                    self.phy.taxa[r.taxon].name,
                    planet::temperature_label(t)
                )),
                _ => None,
            })
        })
    }

    /// Advances what the keystones a finished wait ran with keep track of,
    /// on the planet it ran on: the Tuatara's and the Coelacanth's charges,
    /// the Dodo wearing out.
    fn advance_keystones(&mut self, launched: Planet, ma: u32) {
        for k in self.active_keystones() {
            let rule = self.phy.taxa[k].eco.rule;
            let awake = self.dormant_reason(k).is_none();
            match rule {
                Rule::Changing | Rule::Stasis => {
                    let changed = self.last_launched != Some(launched);
                    // The Tuatara grows on change, the Coelacanth on stasis.
                    let grows = changed == (rule == Rule::Changing);
                    if grows && awake {
                        let ma = u16::try_from(ma).unwrap_or(MAX_GROWTH);
                        self.growth[k] = self.growth[k].saturating_add(ma).min(MAX_GROWTH);
                    } else {
                        self.growth[k] = 0;
                    }
                }
                Rule::Fragile if awake => self.gone[k] = true,
                _ => {}
            }
        }
        self.last_launched = Some(launched);
    }

    /// This shaping's budget: what the last wait gave, what the keystones
    /// earned during it, and Tectonics.
    pub fn max_points(&self) -> u8 {
        let tectonics = if self.boon == Some(Boon::Tectonics) {
            self.tectonics_points()
        } else {
            0
        };
        (self.wait_points + self.bonus_points + tectonics).min(MAX_POINTS)
    }

    /// What Tectonics gives after the last wait: 2 for a 4h one.
    pub fn tectonics_points(&self) -> u8 {
        (TECTONICS_POINTS * self.last_hours / wait::DEFAULT_HOURS).round() as u8
    }

    /// The keystones' points for a wait of `hours`: 1 per 4h per point, the
    /// fraction carried to the next wait.
    fn earn_keystone_points(&mut self, hours: f32) {
        let earned = self.effects().points as f32 * hours / wait::DEFAULT_HOURS + self.point_carry;
        self.bonus_points = (earned.floor() as u8).min(MAX_POINTS);
        self.point_carry = earned.fract();
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
        self.planet.after_cycle()
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
        let mut cards = w.cards + e.extra_card;
        if self.boon == Some(Boon::Lens) {
            cards += 1.0;
        }
        Forecast {
            cards,
            ma: wait::ma(hours),
            points: wait::points(hours),
            odds: Odds {
                cards: cards.floor() as usize,
                // The wait's and radiation's luck come on top of the
                // keystones' cap.
                luck: e.luck + w.luck + RAD_LUCK * self.rad as f32,
                morph_mult: (1.0 + e.morph)
                    * e.morph_mult_boon
                    * e.morph_mult_keystone
                    * w.morph_mult,
                // The tutorial's genomes only find new species.
                discovery: if self.wait_choosable() {
                    let boon = if self.boon == Some(Boon::Discovery) {
                        DISCOVERY_BOON
                    } else {
                        0.0
                    };
                    (w.discovery + e.discovery + boon).clamp(0.0, DISCOVERY_MAX)
                } else {
                    1.0
                },
                // Catalyst lifts the sure card a step: Rare, or Epic on a
                // wait that already guarantees a Rare.
                catalyst: self.boon == Some(Boon::Catalyst) || w.sure_rare,
                sure_epic: self.boon == Some(Boon::Catalyst) && w.sure_rare,
                ma: wait::ma(hours),
                morph_rad: 1.0 + RAD_MORPH * self.rad as f32,
            },
        }
    }

    pub fn next_cycle_seconds(&self) -> f64 {
        let e = self.effects();
        let base = planet::cycle_seconds(self.cycles_done, self.next_hours(), e.quick)
            * (1.0 + f64::from(e.slower));
        if self.boon == Some(Boon::Tailwind) {
            base * (1.0 - TAILWIND)
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
        // The chance is per 4h and compounds, so splitting a wait into
        // shorter ones never rolls it more often.
        let per_4h = self.effects().doom;
        let doom = 1.0 - (1.0 - per_4h).powf(cycle.hours / wait::DEFAULT_HOURS);
        self.doom_risk = doom;
        self.doomed = doom > 0.0 && self.rng.chance(doom);
        self.earn_keystone_points(cycle.hours);
        self.last_hours = cycle.hours;
        self.advance_keystones(cycle.launched, cycle.ma);
        // The tutorial cycles keep the starting budget.
        self.wait_points = if self.wait_choosable() {
            wait::points(cycle.hours)
        } else {
            planet::BASE_POINTS
        };
        self.planet = cycle.launched.after_cycle();
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
                self.last_found = Some(t);
            } else {
                self.specimens[t] += 1;
            }
            // Found again: a worn-out Dodo wakes up.
            self.gone[t] = false;
            let first_morph = card.morph != Morph::None && !self.owns_morph(t, card.morph);
            self.morphs[t] |= card.morph.bit();
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
            Boon::Discovery,
            Boon::Lens,
            Boon::Catalyst,
            Boon::Charm,
            Boon::Tailwind,
            Boon::Tectonics,
        ];
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

    /// Every species found, on this Earth or an earlier one: the
    /// leaderboard's count, which an ended Earth never lowers.
    pub fn species_ever(&self) -> usize {
        self.fossil.iter().filter(|&&f| f).count()
    }

    /// What the leaderboard shows for this player: the last animal found,
    /// or the most advanced one for a save from before it was kept.
    pub fn showcase(&self) -> usize {
        self.last_found.unwrap_or_else(|| self.most_advanced())
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
        self.growth = vec![0; n];
        self.gone = vec![false; n];
        self.last_launched = None;
        self.wait_points = planet::BASE_POINTS;
        self.bonus_points = 0;
        self.point_carry = 0.0;
        self.last_hours = wait::DEFAULT_HOURS;
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
    pub luck: f32,
    pub extra_card: f32,
    /// Added to the wait's chance that a card is a new species.
    pub discovery: f32,
    pub morph: f32,
    pub morph_mult_boon: f32,
    pub quick: f32,
    /// Adjustment points per 4h waited.
    pub points: u32,
    /// How much longer the wait runs (Platypus).
    pub slower: f32,
    /// Morphs multiplied (Archaeopteryx up, Tuatara down).
    pub morph_mult_keystone: f32,
    /// One fewer boon offered (Archaeopteryx).
    pub fewer_boons: bool,
    /// The lowest Temperature allowed (Megalodon).
    pub temp_min: u8,
    /// The chance a 4h wait ends the Earth (Human).
    pub doom: f32,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            luck: 0.0,
            extra_card: 0.0,
            discovery: 0.0,
            morph: 0.0,
            morph_mult_boon: 1.0,
            quick: 0.0,
            points: 0,
            slower: 0.0,
            morph_mult_keystone: 1.0,
            fewer_boons: false,
            temp_min: 0,
            doom: 0.0,
        }
    }
}

/// The Lure boon (`{"Lure": taxon}`) became Discovery: a save holding one,
/// picked or offered, reads it as that.
fn retire_lure(save: &mut serde_json::Value) {
    let fix = |b: &mut serde_json::Value| {
        if b.get("Lure").is_some() {
            *b = "Discovery".into();
        }
    };
    if let Some(b) = save.get_mut("boon") {
        fix(b);
    }
    if let Some(offer) = save.get_mut("boon_offer").and_then(|o| o.as_array_mut()) {
        offer.iter_mut().for_each(fix);
        // Two Lures offered (they came in pairs) become one Discovery.
        let mut seen = Vec::new();
        offer.retain(|b| {
            let new = !seen.contains(b);
            seen.push(b.clone());
            new
        });
    }
}

/// "s", unless `n` is one.
pub fn plural(n: u32) -> &'static str {
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
    fn a_point_keystone_earns_a_point_per_4h_waited() {
        let mut g = Game::new(0.0);
        let bear = equip(&mut g, "Bear");
        assert!(g.dormant_reason(bear).is_some(), "no forest on a sea world");
        g.earn_keystone_points(4.0);
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
        assert_eq!(g.dormant_reason(bear), None);
        assert_eq!(g.max_points(), planet::BASE_POINTS, "earned by waiting");
        g.earn_keystone_points(4.0);
        assert_eq!(g.max_points(), planet::BASE_POINTS + 1);
    }

    #[test]
    fn short_waits_earn_keystone_points_no_faster_than_long_ones() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 1,
            volcanism: 0,
        };
        equip(&mut g, "Bear");
        let mut short = 0;
        for _ in 0..3 {
            g.earn_keystone_points(2.0);
            short += g.bonus_points;
        }
        g.point_carry = 0.0;
        g.earn_keystone_points(6.0);
        // 3 x 2h: 0 + 1 + 0 (the halves carry over); 6h: 1 and a half carried.
        assert_eq!((short, g.bonus_points), (1, 1));
        assert_eq!(g.point_carry, 0.5);
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
    fn tailwind_takes_a_quarter_off_any_wait() {
        let mut g = Game::new(0.0);
        g.cycles_done = 5;
        g.boon = Some(Boon::Tailwind);
        for hours in [2.0, 6.0] {
            g.set_wait(hours);
            assert_eq!(g.next_cycle_seconds(), hours as f64 * 3600.0 * 0.75);
        }
        g.cycles_done = 0;
        assert_eq!(g.next_cycle_seconds(), 45.0);
    }

    #[test]
    fn lens_adds_a_card_and_catalyst_scales_with_the_wait() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.set_wait(4.0);
        let plain = g.forecast().cards;
        g.boon = Some(Boon::Lens);
        assert_eq!(g.forecast().cards, plain + 1.0);
        g.boon = Some(Boon::Catalyst);
        g.set_wait(2.0);
        let short = g.forecast().odds;
        assert!(short.catalyst && !short.sure_epic, "a sure Rare");
        g.set_wait(6.0);
        assert!(
            g.forecast().odds.sure_epic,
            "a 6h wait already has its Rare"
        );
    }

    #[test]
    fn discovery_never_passes_ninety_percent() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.set_wait(6.0);
        g.boon = Some(Boon::Discovery);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 2,
            volcanism: 0,
        };
        let tua = equip(&mut g, "Tuatara");
        g.growth[tua] = 10 * CHARGE_MA;
        assert_eq!(g.forecast().odds.discovery, DISCOVERY_MAX);
    }

    #[test]
    fn the_discovery_boon_adds_a_quarter() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.boon = Some(Boon::Discovery);
        for (hours, d) in [(4.0, 0.55), (6.0, 0.7)] {
            g.set_wait(hours);
            assert!((g.forecast().odds.discovery - d).abs() < 1e-6, "{hours}h");
        }
    }

    #[test]
    fn a_save_holding_a_lure_reads_it_as_discovery() {
        let mut g = Game::new(0.0);
        g.boon = Some(Boon::Lens);
        g.boon_offer = Some(vec![Boon::Charm, Boon::Lens]);
        let json = g
            .to_json()
            .replace("\"boon\":\"Lens\"", "\"boon\":{\"Lure\":4}")
            .replace(
                "\"boon_offer\":[\"Charm\",\"Lens\"]",
                "\"boon_offer\":[{\"Lure\":7},\"Charm\",{\"Lure\":7}]",
            );
        assert!(json.contains("Lure"), "{json}");
        let back = Game::from_json(&json, 0.0).expect("an old save still loads");
        assert_eq!(back.boon, Some(Boon::Discovery));
        assert_eq!(back.boon_offer, Some(vec![Boon::Discovery, Boon::Charm]));
    }

    #[test]
    fn tectonics_scales_with_the_wait_just_run() {
        let mut g = Game::new(0.0);
        for (hours, points) in [(2.0, 1), (4.0, 2), (6.0, 3)] {
            g.last_hours = hours;
            assert_eq!(g.tectonics_points(), points, "{hours}h");
        }
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
    fn a_fresh_game_has_a_score_for_the_leaderboard() {
        let g = Game::new(0.0);
        assert_eq!(g.species_ever(), 1, "the Urmetazoan");
        assert_eq!(g.showcase(), Phylogeny::ROOT);
        assert_eq!(g.rad, 0);
    }

    /// Every field the save gained after its format was fixed (they all
    /// have a default): a save from the earliest release has none of them.
    const ADDED_SINCE: [&str; 17] = [
        "ma_done",
        "wait_hours",
        "charges",
        "held",
        "gone",
        "edition",
        "last_launched",
        "wait_points",
        "rad",
        "fossil",
        "doomed",
        "doom_risk",
        "last_found",
        "growth",
        "bonus_points",
        "point_carry",
        "last_hours",
    ];

    #[test]
    fn a_save_from_the_first_release_loads_with_a_leaderboard_score() {
        let mut g = Game::new(0.0);
        let mut now = 0.0;
        for _ in 0..6 {
            run_cycle(&mut g, &mut now);
            g.choose_boon(0);
        }
        let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
        for key in ADDED_SINCE {
            v.as_object_mut().unwrap().remove(key);
        }
        let back = Game::from_json(&v.to_string(), now).expect("an old save loads");
        assert_eq!(back.discovered(), g.discovered());
        // No fossil list yet: everything found counts.
        assert_eq!(back.species_ever(), g.discovered());
        // No last find kept yet: the most advanced animal stands in.
        assert_eq!(back.last_found, None);
        assert_eq!(back.showcase(), back.most_advanced());
    }

    #[test]
    fn the_leaderboard_animal_is_the_last_one_found() {
        let mut g = Game::new(0.0);
        let mut now = 0.0;
        let opened = run_cycle(&mut g, &mut now);
        let last_new = opened
            .iter()
            .rev()
            .find(|o| o.card.new)
            .map(|o| o.card.taxon);
        assert!(
            last_new.is_some(),
            "the first genome always finds something"
        );
        assert!(g.last_found.is_some());
        assert!(g.unlocked[g.showcase()]);
        let back = Game::from_json(&g.to_json(), now).unwrap();
        assert_eq!(back.last_found, g.last_found, "kept in the save");
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
    fn an_old_save_keeps_this_shapings_keystone_points() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        let coral = equip(&mut g, "Coral");
        g.morphs[coral] = Morph::Giant.bit();
        let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
        let o = v.as_object_mut().unwrap();
        for key in ["growth", "bonus_points", "point_carry", "last_hours"] {
            o.remove(key);
        }
        let back = Game::from_json(&v.to_string(), 0.0).unwrap();
        assert_eq!(
            back.max_points(),
            planet::BASE_POINTS + 1,
            "the giant's point"
        );
    }

    #[test]
    fn amber_doubles_the_bonus() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        let coral = equip(&mut g, "Coral");
        let plain = luck(&report(&g, coral));
        assert!(plain > 0.0);
        g.morphs[coral] = Morph::Amber.bit();
        assert_eq!(luck(&report(&g, coral)), plain * 2.0);
    }

    #[test]
    fn albinos_sunburn_and_melanistic_coats_ignore_temperature() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 3,
            volcanism: 0,
        };
        let ape = equip(&mut g, "Ape");
        g.morphs[ape] = Morph::Albino.bit() | Morph::Melanistic.bit();
        g.edition[ape] = PICKED | Morph::Albino.bit();
        assert!(g.dormant_reason(ape).unwrap().contains("albino"));
        g.edition[ape] = PICKED | Morph::Melanistic.bit();
        g.planet.temperature = 0;
        assert_eq!(g.dormant_reason(ape), None, "a Snowball ape");
        g.edition[ape] = PICKED;
        assert_eq!(g.dormant_reason(ape).as_deref(), Some("too cold"));
    }

    #[test]
    fn the_best_morph_is_the_strongest_that_stays_awake() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 2,
            volcanism: 0,
        };
        let ape = equip(&mut g, "Ape");
        assert_eq!(g.edition(ape), Morph::None, "nothing owned");
        g.morphs[ape] = Morph::Giant.bit() | Morph::Albino.bit() | Morph::Melanistic.bit();
        assert!(g.edition_is_best(ape));
        g.planet.temperature = 3;
        assert_eq!(g.edition(ape), Morph::Giant, "too warm for an albino");
        g.planet.temperature = 1;
        assert_eq!(g.edition(ape), Morph::Melanistic, "only it lives this cold");
        assert_eq!(g.dormant_reason(ape), None);
        g.morphs[ape] |= Morph::Amber.bit();
        g.planet.temperature = 3;
        assert_eq!(g.edition(ape), Morph::Amber);
        // A bear lives at Cool, where an albino coat is fine and strongest.
        let bear = equip(&mut g, "Bear");
        g.morphs[bear] = Morph::Giant.bit() | Morph::Albino.bit();
        g.planet.temperature = 2;
        assert_eq!(g.dormant_reason(bear), None);
        assert_eq!(g.edition(bear), Morph::Albino);
    }

    #[test]
    fn a_giant_keystone_gives_an_adjustment_point_no_copier_takes() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        let clown = equip(&mut g, "Clownfish");
        let base = g.max_points();
        g.morphs[clown] = Morph::Giant.bit();
        assert_eq!(g.edition(clown), Morph::Giant);
        g.earn_keystone_points(4.0);
        assert_eq!(g.max_points(), base + 1);
        let octo = equip(&mut g, "Octopus");
        let copied = report(&g, octo);
        assert_eq!(copied.copied_from, Some(clown));
        assert!(!copied.gains.contains(&Gain::Points(1)), "{copied:?}");
        g.earn_keystone_points(4.0);
        assert_eq!(g.max_points(), base + 1, "the octopus adds no point");
        // Asleep, a giant gives nothing either.
        g.planet.temperature = 0;
        assert!(g.dormant_reason(clown).is_some());
        assert!(!report(&g, clown).gains.contains(&Gain::Points(1)));
    }

    fn find(g: &Game, name: &str) -> usize {
        g.phy.taxa.iter().position(|t| t.name == name).unwrap()
    }

    #[test]
    fn copiers_copy_their_kin_above_but_never_a_legendary() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        let clown = equip(&mut g, "Clownfish");
        let octo = equip(&mut g, "Octopus");
        let (c, o) = (report(&g, clown), report(&g, octo));
        assert!(!c.gains.is_empty());
        assert_eq!(o.gains, c.gains);
        assert_eq!(o.copied_from, Some(clown));
        assert!(o.summary().ends_with("copied"), "{}", o.summary());
        // Not a fish above: nothing to copy.
        let coral = find(&g, "Coral");
        g.unlocked[coral] = true;
        g.keystones = vec![coral, octo];
        assert!(matches!(report(&g, octo).status, Status::Waiting(_)));
        // A legendary fish isn't copied either.
        let mega = find(&g, "Megalodon");
        g.unlocked[mega] = true;
        g.keystones = vec![mega, octo];
        assert!(g.dormant_reason(mega).is_none());
        assert!(matches!(report(&g, octo).status, Status::Waiting(_)));
    }

    #[test]
    fn the_kins_are_fish_mammals_and_birds() {
        let g = Game::new(0.0);
        let kin = |name, k| g.is_kin(find(&g, name), k);
        assert!(kin("Shark", Kin::Fish) && kin("Coelacanth", Kin::Fish));
        assert!(!kin("Frog", Kin::Fish), "a tetrapod");
        assert!(kin("Lion", Kin::Mammal) && !kin("Lizard", Kin::Mammal));
        assert!(kin("Owl", Kin::Bird) && kin("Dodo", Kin::Bird) && !kin("Bat", Kin::Bird));
    }

    #[test]
    fn the_t_rex_eats_the_keystone_in_the_last_slot() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 5,
            volcanism: 1,
        };
        let first = equip(&mut g, "Tardigrade");
        let rex = equip(&mut g, "T. rex");
        assert_eq!(g.dormant_reason(rex), None);
        // The T. rex is last: the one before it is eaten.
        assert_eq!(
            g.dormant_reason(first).as_deref(),
            Some("eaten by the T. rex")
        );
        g.keystones = vec![rex, first];
        assert_eq!(
            g.dormant_reason(first).as_deref(),
            Some("eaten by the T. rex")
        );
        assert!(report(&g, rex).gains.contains(&Gain::Cards(2.0)));
    }

    #[test]
    fn the_dodo_triples_luck_then_sleeps_until_found_again() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 5,
            oxygen: 3,
            temperature: 4,
            volcanism: 0,
        };
        let dodo = equip(&mut g, "Dodo");
        let s = g.keystone_strength(dodo);
        assert_eq!(luck(&report(&g, dodo)), 3.0 * s);
        g.advance_keystones(g.planet, genome::GENOME_MA);
        assert!(g.dormant_reason(dodo).unwrap().contains("worn out"));
        g.genome = Some(vec![Card {
            taxon: dodo,
            tier: Tier::Legendary,
            morph: Morph::None,
            new: false,
            note: None,
        }]);
        g.open_genome(0.0);
        assert_eq!(g.dormant_reason(dodo), None, "a duplicate wakes it");
    }

    #[test]
    fn the_tuatara_grows_while_the_planet_keeps_changing() {
        let mut g = Game::new(0.0);
        let cool = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 2,
            volcanism: 0,
        };
        g.planet = cool;
        let tua = equip(&mut g, "Tuatara");
        g.advance_keystones(cool, genome::GENOME_MA);
        assert_eq!(g.growth[tua], CHARGE_MA);
        g.advance_keystones(cool, genome::GENOME_MA);
        assert_eq!(g.growth[tua], 0, "the same planet twice");
        g.advance_keystones(Planet { oxygen: 4, ..cool }, genome::GENOME_MA);
        assert_eq!(g.growth[tua], CHARGE_MA);
        assert!(report(&g, tua)
            .gains
            .iter()
            .any(|x| matches!(x, Gain::Discovery(_))));
    }

    #[test]
    fn the_coelacanth_grows_while_the_planet_stays_the_same() {
        let mut g = Game::new(0.0);
        let deep = Planet {
            land: 1,
            vegetation: 0,
            oxygen: 2,
            temperature: 1,
            volcanism: 0,
        };
        g.planet = deep;
        let fish = equip(&mut g, "Coelacanth");
        assert_eq!(g.dormant_reason(fish), None);
        let s = g.keystone_strength(fish);
        assert_eq!(luck(&report(&g, fish)), s, "its base luck from the start");
        g.advance_keystones(deep, genome::GENOME_MA);
        assert_eq!(g.growth[fish], 0, "nothing to compare the first wait to");
        g.advance_keystones(deep, genome::GENOME_MA);
        g.advance_keystones(deep, genome::GENOME_MA);
        assert_eq!(g.growth[fish], 2 * CHARGE_MA);
        assert!(
            (luck(&report(&g, fish)) - s * 1.4).abs() < 1e-4,
            "+20% a charge"
        );
        // Full at 5 charges: twice its base, no more.
        for _ in 0..10 {
            g.advance_keystones(deep, genome::GENOME_MA);
        }
        assert_eq!(g.growth[fish], 5 * CHARGE_MA);
        assert!((luck(&report(&g, fish)) - s * 2.0).abs() < 1e-4);
        assert!(
            (g.effects().discovery + COELACANTH_DISCOVERY).abs() < 1e-6,
            "its catch: fewer new species"
        );
        assert_eq!(g.effects().morph_mult_keystone, 1.0, "not the Tuatara's");
        g.advance_keystones(Planet { oxygen: 3, ..deep }, genome::GENOME_MA);
        assert_eq!(g.growth[fish], 0, "the planet changed");
    }

    #[test]
    fn the_platypus_has_every_biome_bonus_and_a_longer_wait() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.planet = Planet {
            land: 2,
            vegetation: 1,
            oxygen: 3,
            temperature: 1,
            volcanism: 0,
        };
        let plain = g.next_cycle_seconds();
        let platypus = equip(&mut g, "Platypus");
        assert_eq!(g.dormant_reason(platypus), None);
        assert_eq!(g.active_biomes(), Biome::ALL.to_vec());
        // 50% longer, less the Ice age's 15% it brings along.
        let expected = plain * 1.5 * 0.85;
        assert!((g.next_cycle_seconds() - expected).abs() < 1e-3);
    }

    #[test]
    fn megalodon_doubles_luck_and_locks_the_cold_out() {
        let mut g = Game::new(0.0);
        g.planet = Planet { land: 1, ..REEF };
        equip(&mut g, "Coral");
        let before = g.effects().luck;
        equip(&mut g, "Megalodon");
        assert_eq!(g.effects().luck, (before * 2.0).min(KEYSTONE_LUCK_MAX));
        assert!(!g.can_step(Lever::Temperature, -1));
        assert!(g.lever_lock(Lever::Temperature, -1).is_some());
        assert!(g.lever_lock(Lever::Temperature, 1).is_none());
    }

    #[test]
    fn the_wait_never_crowds_out_keystone_luck() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.set_wait(6.0);
        g.planet = Planet { land: 1, ..REEF };
        let wait = g.forecast().odds.luck;
        assert_eq!(wait, 12.0, "a 6h wait alone");
        equip(&mut g, "Coral");
        let one = g.effects().luck;
        assert!(one > 0.0);
        assert_eq!(g.forecast().odds.luck, wait + one, "all of it counts");
        // The Megalodon's x2 counts too, up to the keystones' cap.
        equip(&mut g, "Megalodon");
        assert_eq!(
            g.forecast().odds.luck,
            wait + (one * 2.0).min(KEYSTONE_LUCK_MAX)
        );
        assert!(g.forecast().odds.luck > 15.0, "past the old cap of 15");
    }

    #[test]
    fn archaeopteryx_triples_morphs_and_takes_a_boon() {
        let mut g = Game::new(0.0);
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 3,
            volcanism: 0,
        };
        let plain = g.forecast().odds.morph_mult;
        let bird = equip(&mut g, "Archaeopteryx");
        assert_eq!(g.dormant_reason(bird), None);
        assert_eq!(g.forecast().odds.morph_mult, plain * 3.0);
        assert!(g.effects().fewer_boons);
        assert_eq!(g.roll_boons().len(), 2);
    }

    #[test]
    fn the_tuatara_halves_morphs() {
        let mut g = Game::new(0.0);
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 2,
            volcanism: 0,
        };
        let (wait, morphs) = (g.next_cycle_seconds(), g.forecast().odds.morph_mult);
        equip(&mut g, "Tuatara");
        assert_eq!(g.next_cycle_seconds(), wait, "no longer wait");
        assert_eq!(g.forecast().odds.morph_mult, morphs * 0.5);
    }

    #[test]
    fn three_keystones_of_a_biome_give_its_bonus() {
        let mut g = Game::new(0.0);
        g.planet = REEF;
        equip(&mut g, "Coral");
        equip(&mut g, "Clownfish");
        let morph = g.effects().morph;
        assert!(g.active_biomes().is_empty(), "two isn't enough");
        equip(&mut g, "Seahorse");
        assert_eq!(g.active_biomes(), vec![Biome::Reef]);
        let seahorse = g.effects().morph;
        assert!(seahorse >= morph + 0.5, "{morph} -> {seahorse}");
    }

    #[test]
    fn discovery_grows_with_the_wait_and_keystones() {
        let mut g = Game::new(0.0);
        assert_eq!(g.forecast().odds.discovery, 1.0, "the tutorial only finds");
        g.cycles_done = planet::TUTORIAL_CYCLES;
        for (hours, d) in [(2.0, 0.15), (4.0, 0.3), (6.0, 0.45)] {
            g.set_wait(hours);
            assert!((g.forecast().odds.discovery - d).abs() < 1e-6, "{hours}h");
        }
        g.planet = Planet {
            land: 3,
            vegetation: 3,
            oxygen: 3,
            temperature: 2,
            volcanism: 0,
        };
        g.set_wait(4.0);
        // Its charges grow it: give the Tuatara a full one.
        let tua = equip(&mut g, "Tuatara");
        g.growth[tua] = 2 * CHARGE_MA;
        assert!(g.forecast().odds.discovery > 0.3);
    }

    #[test]
    fn a_save_from_before_picking_wears_the_best_morph() {
        let mut g = Game::new(0.0);
        let t = equip(&mut g, "Urmetazoan");
        g.morphs[t] = Morph::Giant.bit() | Morph::Amber.bit();
        // What older builds stored: a bare morph bit, or nothing.
        for old in [0, Morph::Giant.bit()] {
            g.edition[t] = old;
            assert_eq!(g.edition(t), Morph::Amber, "{old}");
        }
    }

    #[test]
    fn editions_cycle_through_owned_morphs() {
        let mut g = Game::new(0.0);
        let t = equip(&mut g, "Urmetazoan");
        assert!(!g.cycle_edition(t), "nothing to pick");
        g.morphs[t] = Morph::Giant.bit() | Morph::Amber.bit();
        assert_eq!(g.edition(t), Morph::Amber, "best by default");
        let mut seen = Vec::new();
        for _ in 0..4 {
            assert!(g.cycle_edition(t));
            seen.push((g.edition_is_best(t), g.edition(t)));
        }
        assert_eq!(
            seen,
            [
                (false, Morph::None),
                (false, Morph::Giant),
                (false, Morph::Amber),
                (true, Morph::Amber),
            ]
        );
    }

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
    fn about_one_4h_wait_in_five_is_doomed_with_human() {
        let (mut g, _) = with_human();
        g.cycles_done = planet::TUTORIAL_CYCLES;
        g.set_wait(wait::DEFAULT_HOURS);
        let mut now = 0.0;
        let mut doomed = 0;
        for _ in 0..200 {
            g.keystones = vec![g.keystones[0]];
            g.planet = with_human().0.planet;
            g.accelerate(now);
            now += g.cycle.unwrap().duration + 1.0;
            g.tick(now);
            assert!((g.doom_risk - DOOM_CHANCE).abs() < 1e-6);
            doomed += g.doomed as u32;
            g.genome = None;
            g.doomed = false;
        }
        assert!((20..=60).contains(&doomed), "{doomed} of 200");
    }

    #[test]
    fn the_doom_compounds_so_short_waits_risk_the_same() {
        let (mut g, _) = with_human();
        g.cycles_done = planet::TUTORIAL_CYCLES;
        let mut risk = |hours: f32| {
            g.set_wait(hours);
            g.accelerate(0.0);
            g.skip_cycle(0.0);
            g.genome = None;
            g.doomed = false;
            g.planet = with_human().0.planet;
            g.doom_risk
        };
        let (short, long) = (risk(2.0), risk(6.0));
        let three_short = 1.0 - (1.0 - short).powi(3);
        assert!((three_short - long).abs() < 1e-5, "{three_short} vs {long}");
    }

    #[test]
    fn ending_the_earth_keeps_fossils_and_adds_rad() {
        let (mut g, human) = with_human();
        g.specimens[human] = 9;
        g.morphs[human] = Morph::Amber.bit();
        g.ma_done = 500;
        g.end_earth();
        assert_eq!(g.rad, 1);
        assert_eq!(g.discovered(), 1, "back to the Urmetazoan");
        assert!(g.is_fossil(human));
        assert_eq!(g.fossils(), 1);
        assert!(g.keystones.is_empty());
        assert_eq!(g.ma_done, 0);
        assert_eq!(g.planet, Planet::default());
        assert_eq!(g.morphs[human], Morph::Amber.bit(), "morphs kept");
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
