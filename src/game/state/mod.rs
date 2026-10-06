//! Game state and its loop: shape, wait, open a genome, pick a boon.

use serde::{Deserialize, Serialize};

use crate::game::ecology::{self, Bonus, Catch, Kin, Rule, Tier};
use crate::game::genome::{self, Card, Morph, Odds, Rng};
use crate::game::planet::{self, Biome, Cycle, Lever, Planet};
use crate::game::tree::{Phylogeny, Taxon};
use crate::game::wait;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Boon {
    Discovery,
    Lens,
    /// A guaranteed Rare+ card (Epic+ on a 6h wait).
    Catalyst,
    Charm,
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

    pub fn icon(self) -> usize {
        match self {
            Boon::Discovery => 0,
            Boon::Lens => 1,
            Boon::Catalyst => 2,
            Boon::Charm => 3,
            Boon::Tailwind => 4,
            Boon::Tectonics => 5,
        }
    }

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

pub const DOOM_CHANCE: f32 = 0.2;
/// The most luck keystones and biomes give together, after any x2: the
/// wait's luck comes on top, so it never crowds them out.
pub const KEYSTONE_LUCK_MAX: f32 = 20.0;
pub const RAD_LUCK: f32 = 5.0;
pub const RAD_MORPH: f32 = 0.25;

const PICKED: u8 = 0x80;

/// Ma of growth per charge of the Tuatara: a charge per default 4h wait, so
/// short waits grow no faster than long ones.
pub const CHARGE_MA: u16 = 40;
/// Most charges a growing keystone holds...
const MAX_CHARGES: u16 = 5;
const MAX_GROWTH: u16 = MAX_CHARGES * CHARGE_MA;
/// ...each worth this much of its base bonus: x2 when full.
const CHARGE_BONUS: f32 = 0.2;
const TAILWIND: f64 = 0.25;
const TECTONICS_POINTS: f32 = 2.0;
/// The most a card can be a new species once the tutorial is over: some
/// are always animals already owned.
pub const DISCOVERY_MAX: f32 = 0.9;
const DISCOVERY_BOON: f32 = 0.25;
const TUATARA_MORPHS: f32 = 0.5;
const COELACANTH_DISCOVERY: f32 = 0.15;
const PLATYPUS_WAIT: f32 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Active,
    Asleep(String),
    Waiting(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gain {
    Luck(f32),
    LuckMult(f32),
    Cards(f32),
    Discovery(f32),
    Morph(f32),
    Quick(f32),
    Points(u32),
    SlowerWait(f32),
    MorphMult(f32),
    FewerBoons,
    TempMin(u8),
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

#[derive(Clone, Debug)]
pub struct Report {
    pub taxon: usize,
    pub status: Status,
    pub gains: Vec<Gain>,
    pub note: Option<String>,
    /// A copier's gains are another keystone's (this one).
    pub copied_from: Option<usize>,
}

impl Report {
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

#[derive(Clone, Debug)]
pub struct Opened {
    pub card: Card,
    pub level_before: u32,
    pub level_after: u32,
    pub first_morph: bool,
}

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
    pub specimens: Vec<u32>,
    pub morphs: Vec<u8>,
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
    #[serde(default = "default_wait")]
    pub wait_hours: f32,
    #[serde(alias = "nodule")]
    pub genome: Option<Vec<Card>>,
    rng: Rng,
    #[serde(alias = "patrons")]
    pub keystones: Vec<usize>,
    pub boon: Option<Boon>,
    pub boon_offer: Option<Vec<Boon>>,
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
    #[serde(default)]
    pub last_launched: Option<Planet>,
    #[serde(default = "default_points")]
    pub wait_points: u8,
    /// Points the keystones earned during the last wait (`Gain::Points`
    /// per 4h), and the fraction carried to the next.
    #[serde(default)]
    pub bonus_points: u8,
    #[serde(default)]
    pub point_carry: f32,
    #[serde(default = "default_wait")]
    pub last_hours: f32,
    #[serde(default)]
    pub rad: u32,
    /// Every taxon found on any Earth. One not found on this Earth is a
    /// fossil: it keeps its level and morphs and waits to be found again.
    #[serde(default)]
    pub fossil: Vec<bool>,
    /// The waiting genome ends the Earth when opened (rolled with it).
    #[serde(default)]
    pub doomed: bool,
    #[serde(default)]
    pub doom_risk: f32,
    #[serde(default)]
    pub last_found: Option<usize>,
    /// Boons the waiting genome offers, decided by the keystones its wait
    /// ran with. Absent in older saves: the equipped ones decide.
    #[serde(default)]
    pub next_boons: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Forecast {
    /// Expected cards: the fraction is the chance of one more.
    pub cards: f32,
    pub odds: Odds,
    pub ma: u32,
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
    pub discovery: f32,
    pub morph: f32,
    pub morph_mult_boon: f32,
    pub quick: f32,
    pub points: u32,
    pub slower: f32,
    pub morph_mult_keystone: f32,
    pub fewer_boons: bool,
    pub temp_min: u8,
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

mod cycle;
mod effects;
mod keystones;
mod levers;
mod lifecycle;
#[cfg(test)]
mod tests;
