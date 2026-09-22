//! Game state: the DNA economy, and what Evolve does.
//!
//! The shape of the game is set by one rule: a session should run out of
//! things to do in a few minutes. So DNA accrues on a wall clock into a
//! *capped* pool, and Evolve only ever fires once that pool is full — one
//! big, satisfying roll per fill cycle, not a stream of small taps. The cap
//! is the feature; waiting for it is the pacing. That wait is tuned to be
//! **logarithmic in how much you have discovered**: a few seconds for your
//! first Evolve, gently longer as the tree fills in, never a flat wall from
//! click one -- see `refill_seconds`.

use macroquad::rand;
use serde::{Deserialize, Serialize};

use crate::tree::{Group, Phylogeny, Taxon};
use crate::upgrades::{self, Effects, Kind};

// --- production -------------------------------------------------------------

/// DNA per second from a level-1 species at depth 0.
const BASE_OUTPUT: f64 = 0.20;
/// Output multiplier per generation of depth: later species are worth more.
const OUTPUT_PER_DEPTH: f64 = 1.35;
/// Each level of each *ancestor* adds this much to a species' output. Keeps
/// the trunk of the tree worth investing in long after you have left it.
const LINEAGE_BONUS: f64 = 0.02;

// --- the pool ---------------------------------------------------------------

/// Floor the pool can never shrink below, however early the game is.
const MIN_POOL: f64 = 40.0;
/// How much the cap grows per taxon discovered, on top of that floor.
const CAP_PER_DISCOVERY: f64 = 60.0;
/// Fastest possible refill: what the very first Evolve waits for.
const MIN_REFILL_SECONDS: f64 = 5.0;
/// How steeply the refill wait grows with discovery progress.
const REFILL_GROWTH: f64 = 250.0;
/// Discoveries per "unit" of that logarithmic growth -- spreads the curve out
/// so the first several discoveries do not each spike the wait on their own.
const REFILL_SPREAD: f64 = 8.0;
/// Ignore absurd clock jumps (suspend, timezone changes, a bad system clock).
const MAX_CATCHUP_SECONDS: f64 = 7.0 * 24.0 * 3600.0;

// --- the baseline unit --------------------------------------------------

/// The reference cost Evolve's odds are measured against -- see
/// `unlock_chance`. Nothing is actually charged at this rate any more: a
/// full pool is always spent in one go, however large it is.
const ATTEMPT_BASE: f64 = 8.0;
/// Dearer per generation of depth...
const ATTEMPT_PER_DEPTH: f64 = 1.75;
/// ...and dearer again for each child already discovered.
const ATTEMPT_PER_CHILD: f64 = 0.35;

// --- odds -------------------------------------------------------------------

/// Chance that Evolve on a depth-0 taxon carries past its guaranteed first
/// find into a more advanced one, before any bonus.
const BASE_CHANCE: f32 = 0.45;
/// Each generation down the tree is this much rarer.
const DEPTH_FALLOFF: f32 = 0.88;
const MIN_CHANCE: f32 = 0.04;
const MAX_CHANCE: f32 = 0.90;
/// Every Evolve that doesn't advance builds mutation pressure on that
/// lineage, raising the odds until it breaks. Nothing is ever wasted.
const PRESSURE_PER_MISS: f32 = 0.06;
/// Chance added per natural log of (full pool / baseline unit cost). A
/// richer economy commits more DNA to a single Evolve, and that is rewarded
/// with better odds, not just a bigger number spent.
const SPEND_BONUS_SCALE: f32 = 0.16;

// --- upgrades ----------------------------------------------------------

/// DNA to buy a `Kind`'s first purchasable level (its second level overall
/// -- the first taxon to choose it already grants level 1 for free). One
/// shared price curve per `Kind`, not per taxon -- see `Game::kind_level`.
const UPGRADE_LEVEL_BASE: f64 = 50.0;
/// Dearer again for every level already bought -- an infinite sink, but a
/// steepening one.
const UPGRADE_LEVEL_PER_LEVEL: f64 = 1.6;

const PULSE_SECONDS: f32 = 0.35;
/// Taps to fill an animal's own level bar and grant it a level -- a small,
/// direct channel for player agency on a specific species, independent of
/// the Evolve gate. Logarithmic in its *current* level, the same "cheap at
/// first, never a flat wall" shape as the pool refill curve
/// (`refill_seconds`): a handful of taps for its first few levels, gently
/// more as it climbs. First guess; needs playtesting like everything else
/// here.
const TAP_LEVEL_BASE: f64 = 6.0;
const TAP_LEVEL_GROWTH: f64 = 5.0;
const TAP_LEVEL_SPREAD: f64 = 3.0;

pub struct Game {
    pub phy: Phylogeny,
    pub unlocked: Vec<bool>,
    /// 0 while undiscovered, then 1 and up. A repeat discovery is a level,
    /// raising a species' own DNA output. Unrelated to a category's upgrade
    /// strength -- see `upgrade_level`.
    pub level: Vec<u32>,
    /// How far each `Kind` has been bought, indexed by `Kind::index` -- 0
    /// until some taxon's choice has activated it (see `upgrade_kind`,
    /// `pending_choice`), 1 the instant one does (free), and climbing from
    /// there only by direct purchase (`buy_upgrade_level`), independently of
    /// `level` above. Shared across every taxon that expresses that `Kind`
    /// -- there are only ever `upgrades::NUM_KINDS` upgrades to buy, no
    /// matter how many categories offer the same one. See
    /// `Game::choose_upgrade_kind`.
    kind_level: [u32; upgrades::NUM_KINDS],
    /// Which `Kind` a `Backbone` taxon's upgrade actually is --
    /// `None` until the player picks one of 3 random candidates offered on
    /// discovery (`pending_choice`, `choose_upgrade_kind`). Deck-builder
    /// style: the taxon and its flavour (`UpgradeDef::trait_name`) are
    /// fixed, but what it mechanically does is a choice, not a constant.
    /// Drives a species' colour and inherited bonus (`nearest_category_kind`)
    /// -- `kind_level` above is what actually prices and levels it.
    pub upgrade_kind: Vec<Option<Kind>>,
    /// Consecutive failed Evolves on this lineage.
    pub pressure: Vec<u32>,
    pub dna: f64,
    /// Cached, effective (post-upgrade) DNA/s; recomputed when the tree changes.
    /// Flavour and a bonus to the pool's fill speed -- see `fill_rate` -- but
    /// not what paces it: that is `refill_seconds`, so discovering zero
    /// DNA-producing species (an opening run of categories, say) can never
    /// stall the pool the way dividing by a zero rate once did.
    rate: f64,
    /// Cached combined effect of every purchased upgrade; recomputed alongside `rate`.
    effects: Effects,
    /// Wall clock of the last tick, for accrual across a backgrounded app.
    last_tick: f64,
    /// Countdown driving the tap flash, per taxon.
    pub pulse: Vec<f32>,
    pub taps: u64,
    /// A species' own level bar: taps toward `TAP_LEVEL_THRESHOLD`, at which
    /// point it grants a level and resets -- see `tap_level`. Meaningless
    /// (stays 0) for a category, which levels through the upgrade shop.
    pub tap_progress: Vec<u32>,
    /// A category was just discovered and is waiting on the player to pick
    /// which of 3 random `Kind`s its upgrade will be -- see
    /// `choose_upgrade_kind`. The UI gates everything else while this is
    /// `Some`, so at most one of these is ever pending at a time.
    pub pending_choice: Option<PendingChoice>,
}

/// A discovered category's still-unresolved upgrade choice: `taxon` got
/// `options[i]` offered for `i` in 0..3, and `choose_upgrade_kind` expects
/// whichever one the player taps to be one of them.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct PendingChoice {
    pub taxon: usize,
    pub options: [Kind; 3],
}

/// The durable slice of `Game` that actually gets saved -- everything else
/// (`rate`, `effects`, `pulse`) is either cached/derived or purely visual,
/// and gets rebuilt fresh on `load` instead of round-tripped.
#[derive(Serialize, Deserialize)]
struct SaveData {
    unlocked: Vec<bool>,
    level: Vec<u32>,
    kind_level: [u32; upgrades::NUM_KINDS],
    upgrade_kind: Vec<Option<Kind>>,
    pressure: Vec<u32>,
    tap_progress: Vec<u32>,
    dna: f64,
    taps: u64,
    pending_choice: Option<PendingChoice>,
    saved_at: f64,
}

/// What an Evolve produced. A real attempt always discovers something --
/// there is no "nothing happened" outcome, and no level-up either; a taxon
/// only ever gets a level by hand now (`Game::tap_level`). See `Game::evolve`.
pub enum EvolveResult {
    /// A new descendant lineage split off -- the deepest one reached, if the
    /// roll carried it past the first (guaranteed) generation. See `evolve`.
    Unlocked(usize),
    /// Nothing left to discover from this taxon directly -- either a true
    /// leaf, or its own branch is already fully found (any deeper
    /// descendants are reachable by evolving the child that has them, not
    /// this one).
    Exhausted,
    /// Called with the pool not full, or on an undiscovered taxon. The UI
    /// gates both of these already; this is the defensive fallback.
    NotReady,
}

impl Game {
    pub fn new(now: f64) -> Self {
        let phy = Phylogeny::load();
        let n = phy.len();

        let mut level = vec![0; n];
        level[Phylogeny::ROOT] = 1;
        let mut unlocked = vec![false; n];
        unlocked[Phylogeny::ROOT] = true;
        let mut upgrade_kind = vec![None; n];
        let mut kind_level = [0; upgrades::NUM_KINDS];
        if phy.taxa[Phylogeny::ROOT].group == Group::Backbone {
            // The root is never "discovered" through `evolve`, so it never
            // gets a choice moment -- it keeps the same fixed kind the
            // whole upgrade system used to have, matching its flavour
            // ("Multicellularity" -> more of everything).
            upgrade_kind[Phylogeny::ROOT] = Some(Kind::Rate);
            kind_level[Kind::Rate.index()] = 1;
        }

        let mut game = Self {
            phy,
            unlocked,
            level,
            kind_level,
            upgrade_kind,
            pressure: vec![0; n],
            dna: 0.0,
            rate: 0.0,
            effects: Effects::default(),
            last_tick: now,
            pulse: vec![0.0; n],
            taps: 0,
            tap_progress: vec![0; n],
            pending_choice: None,
        };
        game.refresh_derived();
        // Open on a full pool, so the first session starts immediately.
        game.dna = game.pool_cap();
        game
    }

    // --- persistence ----------------------------------------------------------

    /// Writes the durable parts of the state to local storage -- everything
    /// needed to reconstruct a `Game`, but not the cached derived fields
    /// (`rate`, `effects`: `refresh_derived` rebuilds them) or the purely
    /// visual `pulse`. `now` is stamped in so `load` knows how much wall
    /// clock to catch up on, the same way a running `tick` would.
    pub fn save(&self, now: f64) {
        let data = SaveData {
            unlocked: self.unlocked.clone(),
            level: self.level.clone(),
            kind_level: self.kind_level,
            upgrade_kind: self.upgrade_kind.clone(),
            pressure: self.pressure.clone(),
            tap_progress: self.tap_progress.clone(),
            dna: self.dna,
            taps: self.taps,
            pending_choice: self.pending_choice,
            saved_at: now,
        };
        if let Ok(json) = serde_json::to_string(&data) {
            crate::save::write(&json);
        }
    }

    /// Loads a previous save, catching up on elapsed wall-clock time the
    /// same way a normal `tick` would. `None` if there is nothing saved
    /// yet, it failed to parse, or the tree has grown since it was written
    /// (a content update) -- any of those, `new` is the caller's fallback
    /// rather than risking an index panic on a mismatched save.
    pub fn load(now: f64) -> Option<Self> {
        let json = crate::save::read()?;
        let data: SaveData = serde_json::from_str(&json).ok()?;
        let phy = Phylogeny::load();
        let n = phy.len();
        if data.unlocked.len() != n
            || data.level.len() != n
            || data.upgrade_kind.len() != n
            || data.pressure.len() != n
            || data.tap_progress.len() != n
        {
            return None;
        }

        let mut game = Self {
            phy,
            unlocked: data.unlocked,
            level: data.level,
            kind_level: data.kind_level,
            upgrade_kind: data.upgrade_kind,
            pressure: data.pressure,
            tap_progress: data.tap_progress,
            dna: data.dna,
            rate: 0.0,
            effects: Effects::default(),
            last_tick: data.saved_at,
            pulse: vec![0.0; n],
            taps: data.taps,
            pending_choice: data.pending_choice,
        };
        game.refresh_derived();
        game.tick(now);
        Some(game)
    }

    pub fn taxon(&self, i: usize) -> &Taxon {
        &self.phy.taxa[i]
    }

    pub fn discovered(&self) -> usize {
        self.unlocked.iter().filter(|&&u| u).count()
    }

    /// How far into the game you are, for the curves below: discoveries
    /// beyond the guaranteed root, so this is exactly 0 at the very start.
    fn progress(&self) -> f64 {
        (self.discovered() - 1) as f64
    }

    // --- economy ------------------------------------------------------------

    /// Every ancestor's levels feed into a species' power.
    fn lineage_bonus(&self, i: usize) -> f64 {
        let mut levels = 0u32;
        let mut cur = self.phy.taxa[i].parent;
        while let Some(p) = cur {
            levels += self.level[p];
            cur = self.phy.taxa[p].parent;
        }
        1.0 + LINEAGE_BONUS * levels as f64
    }

    /// How much "power" species `i` is worth, in baseline units -- exactly
    /// 1.0 for a level-1, depth-0 species with no leveled ancestors, scaling
    /// with depth, its own level and `lineage_bonus` the same way this
    /// always has. This alone used to just be DNA/s; now it is redirected
    /// through `species_bonus` into whichever stat the species' category
    /// actually represents.
    fn species_power(&self, i: usize) -> f64 {
        if !self.unlocked[i] || self.phy.taxa[i].group == Group::Backbone {
            return 0.0;
        }
        let depth = self.phy.taxa[i].depth as i32;
        OUTPUT_PER_DEPTH.powi(depth) * self.level[i] as f64 * self.lineage_bonus(i)
    }

    /// What kind of bonus species `i` gives, and how much, in that kind's
    /// own natural units -- DNA/s for `Rate` (unchanged from what `output`
    /// always was), a flat amount in every other kind's own unit otherwise.
    /// `None` for a category, an undiscovered taxon, or (in practice, never
    /// through normal play) a species with no resolved category ancestor.
    /// An animal gives whatever bonus its nearest category
    /// (`nearest_category_kind`) does now -- not DNA/s unconditionally the
    /// way it used to.
    ///
    /// `species_power` grows close to exponentially with depth, which would
    /// blow up a linear formula for one deep, leveled species -- every kind
    /// but `Rate` damps it through a log first (`ln(1 + power)`), so a
    /// species is worth more the deeper and more leveled it is without ever
    /// being worth a runaway amount, but always linearly in that damped
    /// term, same as a purchased level -- never compounding. `Rate` is
    /// deliberately left undamped: it is the exact, already-tuned DNA/s
    /// formula species have always had.
    pub fn species_bonus(&self, i: usize) -> Option<(Kind, f64)> {
        if !self.unlocked[i] || self.phy.taxa[i].group == Group::Backbone {
            return None;
        }
        let kind = self.nearest_category_kind(i)?;
        let power = self.species_power(i);
        let amount = if kind == Kind::Rate {
            BASE_OUTPUT * power
        } else {
            kind.per_level() * (1.0 + power).ln()
        };
        Some((kind, amount))
    }

    /// DNA per second from one species specifically -- 0 unless it is
    /// powered by `Kind::Rate` (see `species_bonus`). A `Backbone` taxon
    /// never produces DNA either way; its payoff is an upgrade instead.
    pub fn output(&self, i: usize) -> f64 {
        match self.species_bonus(i) {
            Some((Kind::Rate, amount)) => amount,
            _ => 0.0,
        }
    }

    /// Recomputes everything that depends on which taxa are unlocked, at what
    /// level, and which upgrades are bought: the effective DNA/s and the
    /// combined upgrade effects. Called once at startup and again after
    /// every discovery, level-up or purchase.
    fn refresh_derived(&mut self) {
        let mut effects = upgrades::compute(&self.kind_level);
        let mut dna_rate = effects.rate_bonus;
        for i in 0..self.phy.len() {
            let Some((kind, amount)) = self.species_bonus(i) else {
                continue;
            };
            match kind {
                Kind::Rate => dna_rate += amount,
                Kind::Cost => effects.cost_reduction += amount,
                Kind::Chance => effects.chance_bonus += amount as f32,
                Kind::Cap => effects.cap_bonus += amount,
                Kind::Tap => effects.tap_bonus += amount,
                Kind::Storage => effects.overflow_bonus += amount,
                Kind::Vigor => effects.bonus_start_level += amount,
            }
        }
        self.rate = dna_rate;
        self.effects = effects;
    }

    /// How far a `Kind` has been bought -- 0 until some taxon's choice has
    /// activated it. See `kind_level` (the field this reads).
    pub fn kind_level(&self, kind: Kind) -> u32 {
        self.kind_level[kind.index()]
    }

    /// DNA per second across the whole tree, upgrades included. Shown on the
    /// HUD as "how much your species are producing" -- see `fill_rate` for
    /// what actually paces the pool.
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// The pool's ceiling: a floor, plus a fixed amount per taxon you have
    /// found, plus whatever purchased Cap upgrades add on top. Grows with
    /// *progress*, not with DNA/s, so an opening run of DNA-free categories
    /// still grows your capacity.
    pub fn pool_cap(&self) -> f64 {
        MIN_POOL + CAP_PER_DISCOVERY * self.progress() + self.effects.cap_bonus
    }

    /// The true ceiling `dna` is clamped to -- `pool_cap` plus whatever
    /// `Kind::Storage` upgrades add on top. Evolve is still gated on
    /// `pool_full` (the *normal* cap), not this: overflow only ever lets DNA
    /// keep accruing past that point instead of idling there, typically
    /// while away, so it can go into one stronger Evolve. See `tick`.
    fn overflow_cap(&self) -> f64 {
        self.pool_cap() + self.effects.overflow_bonus
    }

    /// Seconds for a full refill cycle: logarithmic in `progress`, so the
    /// very first one is fast (`MIN_REFILL_SECONDS`) and each later one
    /// grows more slowly than the last, rather than every cycle costing the
    /// same flat wait regardless of how far the game has come.
    fn refill_seconds(&self) -> f64 {
        MIN_REFILL_SECONDS + REFILL_GROWTH * (1.0 + self.progress() / REFILL_SPREAD).ln()
    }

    /// DNA added to the pool per second. A guaranteed baseline -- the cap
    /// divided by the logarithmic refill window above -- plus actual species
    /// output on top, so discovering more/better species genuinely speeds
    /// things up without ever being the only thing keeping the pool moving.
    fn fill_rate(&self) -> f64 {
        self.pool_cap() / self.refill_seconds() + self.rate
    }

    /// How full the pool is, 0 to 1.
    pub fn pool_fraction(&self) -> f32 {
        (self.dna / self.pool_cap()).clamp(0.0, 1.0) as f32
    }

    /// Evolve only ever fires once the pool has actually topped out. See
    /// `can_evolve`.
    pub fn pool_full(&self) -> bool {
        self.dna >= self.pool_cap() - 1e-6
    }

    /// Whether Evolve has something to fire with right now.
    pub fn can_evolve(&self) -> bool {
        self.pool_full()
    }

    /// Accrues DNA against the wall clock, so time spent with the app
    /// backgrounded still counts. `now` is seconds since the epoch. Clamped
    /// to `overflow_cap`, not the normal `pool_cap` -- with no `Storage`
    /// upgrade the two are the same and the pool just clamps at the cap as
    /// ever; with one, DNA keeps accruing past the cap instead of idling
    /// there, up to the upgrade's overflow multiplier.
    pub fn tick(&mut self, now: f64) {
        let dt = (now - self.last_tick).clamp(0.0, MAX_CATCHUP_SECONDS);
        self.last_tick = now;
        self.dna = (self.dna + self.fill_rate() * dt).min(self.overflow_cap());
    }

    /// The reference cost Evolve's chance-of-advancing is measured against:
    /// dearer the deeper `i` is, and dearer again for every descendant
    /// already found. Nothing is charged at this rate directly any more --
    /// see `unlock_chance`'s spend bonus.
    fn baseline_cost(&self, i: usize) -> f64 {
        let depth = self.phy.taxa[i].depth as i32;
        let known = self.phy.taxa[i]
            .children
            .iter()
            .filter(|&&c| self.unlocked[c])
            .count();
        ATTEMPT_BASE * ATTEMPT_PER_DEPTH.powi(depth) * (1.0 + ATTEMPT_PER_CHILD * known as f64)
    }

    // --- odds ---------------------------------------------------------------

    fn base_chance(&self, i: usize) -> f32 {
        let depth = self.phy.taxa[i].depth as i32;
        (BASE_CHANCE * DEPTH_FALLOFF.powi(depth)).max(MIN_CHANCE)
    }

    /// How many baseline units the pool represents for `i`. This is "the
    /// amount of DNA you spent" that Evolve's chance-of-advancing scales
    /// with -- based on the cap at minimum, so the number on the card is
    /// never less than what a fresh top-up guarantees, but it rises with
    /// however far a `Storage` upgrade has let the pool overflow past that,
    /// so banking a stretch of overflow before spending it is a real,
    /// visible improvement rather than wasted DNA.
    fn spend_ratio(&self, i: usize) -> f32 {
        (self.dna.max(self.pool_cap()) / self.baseline_cost(i)).max(1.0) as f32
    }

    /// Chance the next Evolve on `i` carries past its guaranteed first find
    /// into a more advanced one: depth-based odds, mutation pressure, any
    /// purchased chance upgrades, and a bonus for how much DNA a full pool
    /// commits relative to a bare attempt. A richer economy reaches deeper
    /// more reliably, not just spends bigger numbers. See `Game::evolve`.
    pub fn unlock_chance(&self, i: usize) -> f32 {
        let boosted = self.base_chance(i) * (1.0 + self.pressure[i] as f32 * PRESSURE_PER_MISS);
        let spend_bonus = SPEND_BONUS_SCALE * self.spend_ratio(i).ln().max(0.0);
        (boosted + self.effects.chance_bonus + spend_bonus).min(MAX_CHANCE)
    }

    /// How far pressure and the spend bonus have carried this lineage from
    /// its base odds toward the ceiling, 0 to 1. This is the bar the player
    /// watches fill.
    pub fn pressure_fraction(&self, i: usize) -> f32 {
        let base = self.base_chance(i);
        if MAX_CHANCE <= base {
            return 1.0;
        }
        ((self.unlock_chance(i) - base) / (MAX_CHANCE - base)).clamp(0.0, 1.0)
    }

    // --- direct animal leveling ----------------------------------------------

    /// Taps to fill `i`'s bar at its *current* level -- logarithmic, so the
    /// first few levels take only a handful of taps and it grows gently
    /// from there, never a flat wall.
    fn tap_level_threshold(&self, i: usize) -> u32 {
        (TAP_LEVEL_BASE + TAP_LEVEL_GROWTH * (1.0 + self.level[i] as f64 / TAP_LEVEL_SPREAD).ln())
            .round() as u32
    }

    /// Taps `i`'s own level bar once. Animals only -- a category levels
    /// through the upgrade shop instead, not by tapping its card. Fills
    /// `tap_progress[i]` by 1 plus `effects.tap_bonus` (any `Kind::Tap`
    /// anyone has makes every tap, on every animal, worth more -- not just
    /// this one) and, once it reaches `tap_level_threshold`, grants a level
    /// and resets it. Returns whether this tap was the one that leveled it
    /// up (for a UI cue beyond the usual pulse).
    pub fn tap_level(&mut self, i: usize) -> bool {
        if !self.unlocked[i] || self.phy.taxa[i].group == Group::Backbone {
            return false;
        }
        self.pulse[i] = PULSE_SECONDS;
        self.tap_progress[i] += 1 + self.effects.tap_bonus.round().max(0.0) as u32;
        if self.tap_progress[i] >= self.tap_level_threshold(i) {
            self.tap_progress[i] = 0;
            self.level[i] += 1;
            self.refresh_derived();
            return true;
        }
        false
    }

    /// How full `i`'s tap-level bar is, 0 to 1. Meaningless (stays 0) for a
    /// category, which has no tap-level bar of its own.
    pub fn tap_progress_fraction(&self, i: usize) -> f32 {
        self.tap_progress[i] as f32 / self.tap_level_threshold(i) as f32
    }

    /// The nearest `Backbone` ancestor's chosen upgrade kind -- what a
    /// species' card shows as "powered by", and tints its accent with.
    /// `None` only if no ancestor's choice has resolved yet, which in
    /// practice does not happen: every ancestor of a discovered taxon is
    /// itself already discovered, and the choice picker gates everything
    /// else until it resolves, so an unresolved ancestor is never reachable
    /// through normal play.
    pub fn nearest_category_kind(&self, i: usize) -> Option<Kind> {
        let mut cur = self.phy.taxa[i].parent;
        while let Some(p) = cur {
            if self.phy.taxa[p].group == Group::Backbone {
                if let Some(kind) = self.upgrade_kind[p] {
                    return Some(kind);
                }
            }
            cur = self.phy.taxa[p].parent;
        }
        None
    }

    // --- evolve --------------------------------------------------------------

    /// Unlocks `child`: level 1 for a category, or 1 plus whatever `Vigor`
    /// upgrades are in effect for a species (`Game::effects().bonus_start_level`,
    /// rounded down) -- a free upgrade choice offered if it is a category,
    /// pulsed and reflected in the cached derived state. Shared by every
    /// path that discovers something, so there is exactly one place that
    /// does it. Only ever looks at *already* discovered `Vigor` upgrades --
    /// `child` itself cannot be the source of its own head start.
    fn unlock_taxon(&mut self, child: usize) {
        self.unlocked[child] = true;
        self.level[child] = if self.phy.taxa[child].group == Group::Backbone {
            1
        } else {
            1 + self.effects.bonus_start_level.max(0.0) as u32
        };
        // A category's upgrade is a deck-builder-style choice now, not an
        // automatic grant -- offer 3 random kinds and wait for
        // `choose_upgrade_kind` before it does anything.
        if self.phy.taxa[child].group == Group::Backbone {
            self.pending_choice = Some(PendingChoice {
                taxon: child,
                options: Self::random_kind_options(),
            });
        }
        self.pulse[child] = PULSE_SECONDS;
        self.refresh_derived();
    }

    /// Every undiscovered direct child of `i` -- what `evolve` picks the
    /// guaranteed first discovery from, and what a target needs at least
    /// one of to be worth calling Evolve on at all (see `pick_evolve_target`
    /// in `main.rs`).
    fn undiscovered_children(&self, i: usize) -> Vec<usize> {
        self.phy.taxa[i]
            .children
            .iter()
            .copied()
            .filter(|&c| !self.unlocked[c])
            .collect()
    }

    /// Rolls the dice on `i`. Only ever does anything with a full pool --
    /// the whole pool is the bet, which is what makes "the amount you
    /// spent" mean something: it is always everything you had, overflow
    /// included.
    ///
    /// Always discovers something -- there is no more miss, and no more
    /// levelling a descendant either; an animal only ever gets a level by
    /// hand now (`Game::tap_level`). The first new taxon (a random
    /// undiscovered direct child of `i`) is unconditional. What
    /// `unlock_chance` governs instead is how much *further* the discovery
    /// reaches: after that guaranteed first find, each additional
    /// generation gets its own roll at continuing deeper, so a higher
    /// chance is a real shot at a more advanced animal in one Evolve, not a
    /// binary success/fail any more. Pressure resets when it does carry
    /// past the first generation, and climbs when it doesn't -- the same
    /// shape as before, just measuring "how advanced" instead of
    /// "succeeded at all."
    pub fn evolve(&mut self, i: usize) -> EvolveResult {
        if !self.unlocked[i] {
            return EvolveResult::NotReady;
        }
        let first = self.undiscovered_children(i);
        if first.is_empty() {
            return EvolveResult::Exhausted;
        }
        if !self.pool_full() {
            return EvolveResult::NotReady;
        }

        let chance = self.unlock_chance(i);
        self.dna = 0.0;
        self.taps += 1;
        self.pulse[i] = PULSE_SECONDS;

        let mut cur = first[rand::gen_range(0, first.len())];
        self.unlock_taxon(cur);

        let mut advanced = false;
        loop {
            // A freshly discovered category needs its choice resolved
            // before anything else happens -- including continuing deeper.
            if self.pending_choice.is_some() {
                break;
            }
            if rand::gen_range(0.0, 1.0) >= chance {
                break;
            }
            let deeper = self.undiscovered_children(cur);
            if deeper.is_empty() {
                break;
            }
            cur = deeper[rand::gen_range(0, deeper.len())];
            self.unlock_taxon(cur);
            advanced = true;
        }

        self.pressure[i] = if advanced { 0 } else { self.pressure[i] + 1 };
        EvolveResult::Unlocked(cur)
    }

    /// 3 distinct `Kind`s, drawn uniformly at random from all of them --
    /// what a freshly discovered category offers. A partial Fisher-Yates:
    /// only needs the first 3 slots shuffled, not the whole array.
    fn random_kind_options() -> [Kind; 3] {
        let mut all = upgrades::ALL_KINDS;
        for i in 0..3 {
            let j = i + rand::gen_range(0, all.len() - i);
            all.swap(i, j);
        }
        [all[0], all[1], all[2]]
    }

    /// Resolves the pending choice a category discovery left open: `kind`
    /// must be one of the 3 offered for `taxon`, or this does nothing and
    /// returns `false`. Always marks `taxon` with `kind` (so its species
    /// colour and inherit from it correctly), but only activates `kind`
    /// itself -- at level 1, free -- the *first* time any taxon picks it;
    /// picking an already-active `Kind` again elsewhere is still a valid,
    /// meaningful choice for that taxon's own flavour/colour, it just
    /// doesn't add a second, duplicate entry to the upgrades tab or bump
    /// its level for free.
    pub fn choose_upgrade_kind(&mut self, taxon: usize, kind: Kind) -> bool {
        let Some(pending) = self.pending_choice else {
            return false;
        };
        if pending.taxon != taxon || !pending.options.contains(&kind) {
            return false;
        }
        self.upgrade_kind[taxon] = Some(kind);
        if self.kind_level[kind.index()] == 0 {
            self.kind_level[kind.index()] = 1;
        }
        self.pending_choice = None;
        self.refresh_derived();
        true
    }

    // --- upgrade shop ---------------------------------------------------------

    /// DNA to raise `kind`'s level to the next. Only meaningful once it has
    /// one -- i.e. `kind_level(kind) >= 1`. Every purchased/inherited
    /// `Kind::Cost` (`effects.cost_reduction`) knocks flat DNA off this,
    /// floored at 10% of the undiscounted price so it can never reach zero.
    pub fn upgrade_level_cost(&self, kind: Kind) -> f64 {
        // Level 1 is free (granted on the first taxon to choose it), so the
        // first purchasable step is 1 -> 2.
        let steps_already_bought = self.kind_level(kind).saturating_sub(1);
        let base = UPGRADE_LEVEL_BASE * UPGRADE_LEVEL_PER_LEVEL.powi(steps_already_bought as i32);
        (base - self.effects.cost_reduction).max(base * 0.1)
    }

    /// Whether `kind`'s next level is affordable with the DNA on hand.
    pub fn can_afford_upgrade_level(&self, kind: Kind) -> bool {
        if self.kind_level(kind) == 0 {
            return false;
        }
        self.dna >= self.upgrade_level_cost(kind)
    }

    /// Spends DNA to raise `kind`'s level by one -- unbounded. Returns
    /// whether it went through.
    pub fn buy_upgrade_level(&mut self, kind: Kind) -> bool {
        if !self.can_afford_upgrade_level(kind) {
            return false;
        }
        self.dna -= self.upgrade_level_cost(kind);
        self.kind_level[kind.index()] += 1;
        self.refresh_derived();
        true
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.pulse {
            *p = (*p - dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn new_game() -> Game {
        Game::new(0.0)
    }

    /// `crate::save` is one global (a file natively, `localStorage` on wasm)
    /// -- cargo runs tests in parallel by default, so anything that touches
    /// it needs to serialize against every other test that does, or they
    /// race on the same underlying storage.
    static SAVE_LOCK: Mutex<()> = Mutex::new(());

    /// Tops the pool up far past its cap, so `pool_full` holds regardless of
    /// how big the cap itself is.
    fn fill(game: &mut Game) {
        game.dna = f64::MAX / 4.0;
    }

    /// Unlocks a real, DNA-producing species (Backbone taxa produce none),
    /// for tests that need the economy actually running.
    fn unlock_an_animal(game: &mut Game) {
        let animal = game
            .phy
            .taxa
            .iter()
            .position(|t| t.group != Group::Backbone)
            .expect("the tree has at least one non-Backbone taxon");
        game.unlocked[animal] = true;
        game.level[animal] = 1;
        game.refresh_derived();
    }

    #[test]
    fn only_the_root_starts_unlocked_and_it_starts_at_level_one() {
        let g = new_game();
        assert_eq!(g.discovered(), 1);
        assert!(g.unlocked[Phylogeny::ROOT]);
        assert_eq!(g.level[Phylogeny::ROOT], 1);
    }

    #[test]
    fn the_game_opens_on_a_full_pool() {
        let g = new_game();
        assert!((g.dna - g.pool_cap()).abs() < 1e-9);
        assert!(g.pool_fraction() > 0.99);
        assert!(g.pool_full());
    }

    /// The bug this whole pass fixed: a rate of exactly 0 (an opening run of
    /// nothing but categories, which produce no DNA) must never stop the
    /// pool from refilling. `fill_rate` guarantees a positive floor
    /// independent of `rate`. Forced to exactly 0 directly, since the root's
    /// free `Rate` kind now gives every fresh game a small flat rate of its
    /// own (see `refresh_derived`) -- this test is about the guaranteed
    /// floor underneath that, not about a fresh game's actual rate.
    #[test]
    fn the_pool_refills_even_at_zero_dna_per_second() {
        let mut g = new_game();
        g.rate = 0.0;
        g.dna = 0.0;

        g.tick(10.0);
        assert!(
            g.dna > 0.0,
            "pool never accrued with rate == 0 -- the old softlock"
        );
    }

    #[test]
    fn the_first_refill_is_fast_and_later_ones_take_longer() {
        let mut g = new_game();
        g.dna = 0.0;
        g.tick(MIN_REFILL_SECONDS + 1.0);
        assert!(g.pool_full(), "the very first refill should be quick");

        // Discovering taxa raises both the cap and the wait -- confirm the
        // *wait* actually grows, not just the cap (which would trivially
        // make `pool_full` harder to satisfy without proving the curve).
        let before = g.refill_seconds();
        for i in 0..20.min(g.phy.len()) {
            g.unlocked[i] = true;
            g.level[i] = g.level[i].max(1);
        }
        assert!(
            g.refill_seconds() > before,
            "refill wait should grow with progress"
        );
    }

    #[test]
    fn dna_accrues_on_the_wall_clock_and_stops_at_the_cap() {
        let mut g = new_game();
        unlock_an_animal(&mut g);
        g.dna = 0.0;

        g.tick(1.0);
        let after_1s = g.dna;
        assert!(after_1s > 0.0, "nothing accrued");
        assert!(after_1s < g.pool_cap());
        assert!(!g.pool_full());

        // A week away must not overflow the pool.
        g.tick(1.0 + 7.0 * 24.0 * 3600.0);
        assert!(
            (g.dna - g.pool_cap()).abs() < 1e-6,
            "pool overflowed to {}",
            g.dna
        );
        assert!(g.pool_full());
    }

    #[test]
    fn a_backwards_clock_does_not_drain_the_pool() {
        let mut g = new_game();
        g.tick(1000.0);
        let before = g.dna;
        g.tick(0.0);
        assert!(g.dna >= before - 1e-9, "went backwards");
    }

    /// Gives `g` a `Storage` upgrade directly, bypassing discovery/purchase
    /// -- these tests are about the overflow mechanic, not how you get there.
    fn grant_storage_level(game: &mut Game, level: u32) {
        let bilaterian = game
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Bilaterian")
            .unwrap();
        game.upgrade_kind[bilaterian] = Some(Kind::Storage);
        game.kind_level[Kind::Storage.index()] = level;
        game.refresh_derived();
    }

    #[test]
    fn topping_out_with_storage_keeps_accruing_past_the_normal_cap() {
        let mut g = new_game();
        let normal_cap = g.pool_cap();
        grant_storage_level(&mut g, 1);
        g.dna = 0.0;

        // Enough to reach the normal cap and well past it.
        g.tick(g.refill_seconds() * 3.0);
        assert!(
            g.dna > normal_cap,
            "should have kept accruing past the normal cap, got {}",
            g.dna
        );
        assert!(g.pool_full(), "still ready to Evolve once past the cap");
    }

    #[test]
    fn storage_overflow_never_exceeds_its_multiplier() {
        let mut g = new_game();
        grant_storage_level(&mut g, 2);
        g.dna = 0.0;

        g.tick(g.refill_seconds() * 50.0);
        assert!(
            g.dna <= g.pool_cap() + g.effects.overflow_bonus + 1e-6,
            "should clamp at the overflow cap, got {}",
            g.dna
        );
    }

    #[test]
    fn no_storage_upgrade_behaves_exactly_as_before() {
        let mut g = new_game();
        g.dna = 0.0;
        g.tick(g.refill_seconds() * 50.0);
        assert!(
            (g.dna - g.pool_cap()).abs() < 1e-6,
            "should just clamp at the cap as ever"
        );
        assert!(g.pool_full());
    }

    #[test]
    fn overflowed_dna_improves_the_chance_preview() {
        let mut g = new_game();
        grant_storage_level(&mut g, 3);
        g.dna = 0.0;
        g.tick(g.refill_seconds() * 50.0);
        let overflowed_chance = g.unlock_chance(Phylogeny::ROOT);

        g.dna = g.pool_cap();
        let baseline_chance = g.unlock_chance(Phylogeny::ROOT);

        assert!(
            overflowed_chance >= baseline_chance,
            "banked overflow should never make the odds worse ({overflowed_chance} < {baseline_chance})"
        );
    }

    #[test]
    fn with_no_full_pool_evolve_is_not_ready() {
        let mut g = new_game();
        g.dna = 0.0;
        assert!(!g.can_evolve());
        assert!(matches!(g.evolve(Phylogeny::ROOT), EvolveResult::NotReady));
    }

    #[test]
    fn evolve_refuses_a_pool_that_is_not_full() {
        let mut g = new_game();
        g.dna = g.pool_cap() * 0.5;
        assert!(!g.pool_full());
        assert!(matches!(g.evolve(Phylogeny::ROOT), EvolveResult::NotReady));
        assert_eq!(g.taps, 0, "a refused evolve must not count");
    }

    #[test]
    fn evolve_spends_the_whole_pool_win_or_lose() {
        let mut g = new_game();
        fill(&mut g);
        g.evolve(Phylogeny::ROOT);
        assert_eq!(g.dna, 0.0, "evolve must commit everything, not a fraction");
        assert_eq!(g.taps, 1);
    }

    #[test]
    fn a_bigger_pool_relative_to_the_baseline_raises_the_odds() {
        let g = new_game();
        let cheap = g.unlock_chance(Phylogeny::ROOT);

        // A deeper taxon has a dearer baseline, so the same pool buys a
        // smaller multiple of it -- a smaller spend bonus, lower odds.
        // Deliberately not unlocking it: unlocking anything would grow
        // `discovered()` and so `pool_cap()` too (see `progress`), which
        // would confound the comparison this test is actually making.
        // `unlock_chance` itself never checks `unlocked`.
        let deeper = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Bilaterian")
            .unwrap();
        let expensive = g.unlock_chance(deeper);
        assert!(cheap >= expensive, "{cheap} should be >= {expensive}");
    }

    #[test]
    fn a_failed_evolve_builds_pressure_and_raises_the_odds() {
        let mut g = new_game();
        let base = g.unlock_chance(Phylogeny::ROOT);

        // Drive misses directly; the roll itself is random.
        g.pressure[Phylogeny::ROOT] = 5;
        assert!(g.unlock_chance(Phylogeny::ROOT) > base);
        assert!(g.pressure_fraction(Phylogeny::ROOT) > 0.0);

        g.pressure[Phylogeny::ROOT] = 100_000;
        assert!(g.unlock_chance(Phylogeny::ROOT) <= MAX_CHANCE);
        assert!((g.pressure_fraction(Phylogeny::ROOT) - 1.0).abs() < 1e-5);
    }

    /// Evolve never levels anything any more -- once a branch is fully
    /// discovered, there is nothing left for it to give directly, so it
    /// reports `Exhausted` (the deeper descendants through it are reachable
    /// by evolving the child that actually has them, not this taxon).
    #[test]
    fn evolving_a_fully_discovered_branch_reports_exhausted() {
        let mut g = new_game();

        let mollusc = g.phy.taxa.iter().position(|t| t.name == "Mollusc").unwrap();
        g.unlocked[mollusc] = true;
        g.level[mollusc] = 1;
        for &c in &g.phy.taxa[mollusc].children.clone() {
            g.unlocked[c] = true;
            g.level[c] = 1;
        }
        g.refresh_derived();

        let before: Vec<u32> = g.phy.taxa[mollusc]
            .children
            .iter()
            .map(|&c| g.level[c])
            .collect();

        fill(&mut g);
        let dna_before = g.dna;
        assert!(matches!(g.evolve(mollusc), EvolveResult::Exhausted));
        let after: Vec<u32> = g.phy.taxa[mollusc]
            .children
            .iter()
            .map(|&c| g.level[c])
            .collect();
        assert_eq!(after, before, "nothing should have leveled up");
        assert_eq!(
            g.dna, dna_before,
            "an Exhausted attempt must not spend the pool"
        );
    }

    /// The core promise of this change: Evolve always discovers something,
    /// deterministically -- the first taxon is a guaranteed, unconditional
    /// find, never a roll. On a fresh game the root is the only candidate
    /// with undiscovered children, so this is also fully deterministic:
    /// no RNG-dependent retry loop needed to observe it.
    #[test]
    fn evolve_always_discovers_something_never_a_level() {
        let mut g = new_game();
        fill(&mut g);
        let result = g.evolve(Phylogeny::ROOT);
        assert!(
            matches!(result, EvolveResult::Unlocked(_)),
            "the very first Evolve on a fresh game must discover something"
        );
        assert_eq!(
            g.level[Phylogeny::ROOT],
            1,
            "the root must never gain a level"
        );
    }

    /// A `Vigor` upgrade should give a freshly discovered species a starting
    /// level above 1 -- and only from the moment it is bought onward; it
    /// must never retroactively touch anything already discovered.
    #[test]
    fn vigor_raises_a_newly_discovered_species_starting_level() {
        let mut g = new_game();
        let already_here = g
            .phy
            .taxa
            .iter()
            .position(|t| t.group != Group::Backbone)
            .expect("the tree has at least one non-Backbone taxon");
        g.unlocked[already_here] = true;
        g.level[already_here] = 1;

        g.upgrade_kind[Phylogeny::ROOT] = Some(Kind::Vigor);
        g.kind_level[Kind::Vigor.index()] = 3;
        g.refresh_derived();

        let acoel = (0..g.phy.len())
            .find(|&i| !g.unlocked[i] && g.phy.taxa[i].group != Group::Backbone)
            .expect("the tree should have an undiscovered species");
        g.unlock_taxon(acoel);

        assert_eq!(
            g.level[already_here], 1,
            "buying Vigor must not retroactively level up what is already discovered"
        );
        assert!(
            g.level[acoel] > 1,
            "a freshly discovered species should start above level 1, got {}",
            g.level[acoel]
        );
    }

    /// Categories still always start at level 1 -- `Vigor` is a species-only
    /// head start, not a loophole back into leveling a category on discovery.
    #[test]
    fn vigor_never_raises_a_freshly_discovered_categorys_starting_level() {
        let mut g = new_game();
        g.upgrade_kind[Phylogeny::ROOT] = Some(Kind::Vigor);
        g.kind_level[Kind::Vigor.index()] = 5;
        g.refresh_derived();

        let category = g
            .phy
            .taxa
            .iter()
            .position(|t| t.group == Group::Backbone && t.parent == Some(Phylogeny::ROOT))
            .expect("the root should have a Backbone child");
        assert!(!g.unlocked[category]);
        g.unlock_taxon(category);
        assert_eq!(g.level[category], 1);
    }

    /// The new core mechanic: a maxed-out chance is a real shot at reaching
    /// more than one generation deep in a single Evolve, not just a binary
    /// success/fail. Needs a taxon `start` with an undiscovered child `y`
    /// that is itself non-`Backbone` (a category would stop the cascade
    /// with a choice before it could go any further) and has children of
    /// its own for a second hop to land on -- searched for at runtime
    /// rather than hard-coding a name that might drift.
    #[test]
    fn a_maxed_chance_can_reach_more_than_one_generation() {
        let phy = Phylogeny::load();
        let start = (0..phy.len())
            .find(|&i| {
                phy.taxa[i].children.iter().any(|&y| {
                    phy.taxa[y].group != Group::Backbone && !phy.taxa[y].children.is_empty()
                })
            })
            .expect("the tree should have a taxon with a species child that itself has children");

        let mut reached_two_deep = false;
        for _ in 0..200 {
            let mut g = new_game();
            g.unlocked[start] = true;
            g.level[start] = 1;
            g.pressure[start] = 1_000_000; // clamps unlock_chance at MAX_CHANCE
            g.refresh_derived();
            fill(&mut g);

            let before = g.discovered();
            g.evolve(start);
            if g.discovered() - before > 1 {
                reached_two_deep = true;
                break;
            }
        }
        assert!(
            reached_two_deep,
            "a maxed chance never once reached a second generation in 200 tries"
        );
    }

    /// Evolve only ever sets a level on a taxon it just discovered (always
    /// to 1) -- it never touches the level of anything already unlocked,
    /// category or species. Levelling an existing taxon is `tap_level`'s
    /// job alone now.
    #[test]
    fn evolve_never_changes_an_existing_taxons_level() {
        let mut g = new_game();
        unlock_an_animal(&mut g);
        for _ in 0..50 {
            let Some(target) = pick_any_frontier(&g) else {
                break;
            };
            // A cascading Evolve can discover more than one taxon in a
            // single call -- capture *every* already-unlocked taxon's level
            // beforehand, not just the one `target` itself.
            let was_unlocked = g.unlocked.clone();
            let before_level = g.level.clone();
            fill(&mut g);
            g.evolve(target);
            for i in 0..g.phy.len() {
                if was_unlocked[i] {
                    assert_eq!(
                        g.level[i], before_level[i],
                        "evolve changed the level of an already-discovered taxon {i}"
                    );
                }
            }
        }
    }

    /// Test-only helper mirroring `main.rs`'s `pick_evolve_target`: any
    /// unlocked taxon with at least one undiscovered direct child.
    fn pick_any_frontier(g: &Game) -> Option<usize> {
        (0..g.phy.len())
            .find(|&i| g.unlocked[i] && g.phy.taxa[i].children.iter().any(|&c| !g.unlocked[c]))
    }

    /// A full playthrough -- the same stress pattern as
    /// `unlocked_and_level_never_disagree` above -- must never leave a
    /// category above level 1, however many Evolves land on it along the
    /// way. This is the direct regression guard for the animal-only rule.
    #[test]
    fn categories_never_gain_a_level_through_play() {
        let mut g = new_game();
        for _ in 0..3000 {
            for i in 0..g.phy.len() {
                if g.unlocked[i] {
                    fill(&mut g);
                    g.evolve(i);
                }
            }
            if g.discovered() == g.phy.len() {
                break;
            }
        }
        for i in 0..g.phy.len() {
            if g.phy.taxa[i].group == Group::Backbone {
                assert_eq!(g.level[i], 1, "category {i} gained a level");
            }
        }
    }

    #[test]
    fn tap_level_fills_the_bar_and_grants_a_level() {
        let mut g = new_game();
        unlock_an_animal(&mut g);
        let animal = g
            .phy
            .taxa
            .iter()
            .position(|t| t.group != Group::Backbone)
            .unwrap();
        let level_before = g.level[animal];
        let threshold = g.tap_level_threshold(animal);

        for _ in 0..(threshold - 1) {
            assert!(
                !g.tap_level(animal),
                "should not level up before the bar fills"
            );
        }
        assert!(g.tap_progress_fraction(animal) > 0.0);
        assert!(
            g.tap_level(animal),
            "the threshold-th tap should level it up"
        );
        assert_eq!(g.level[animal], level_before + 1);
        assert_eq!(g.tap_progress[animal], 0, "the bar should reset");
    }

    #[test]
    fn tap_level_refuses_a_category() {
        let mut g = new_game();
        assert!(g.unlocked[Phylogeny::ROOT]);
        assert_eq!(g.phy.taxa[Phylogeny::ROOT].group, Group::Backbone);
        let level_before = g.level[Phylogeny::ROOT];

        assert!(!g.tap_level(Phylogeny::ROOT));
        assert_eq!(g.level[Phylogeny::ROOT], level_before);
        assert_eq!(g.tap_progress[Phylogeny::ROOT], 0);
    }

    #[test]
    fn nearest_category_kind_finds_the_resolved_ancestor() {
        let mut g = new_game();
        // The root always has a resolved kind (fixed, no choice moment).
        let root_kind = g.upgrade_kind[Phylogeny::ROOT].unwrap();

        let child = g.phy.taxa[Phylogeny::ROOT].children[0];
        g.unlocked[child] = true;
        g.level[child] = 1;
        g.refresh_derived();

        assert_eq!(g.nearest_category_kind(child), Some(root_kind));
    }

    #[test]
    fn levels_and_ancestor_levels_both_raise_output() {
        let mut g = new_game();
        let child = g.phy.taxa[Phylogeny::ROOT].children[0];
        g.unlocked[child] = true;
        g.level[child] = 1;
        g.refresh_derived();

        let base = g.output(child);
        g.level[child] = 2;
        assert!(g.output(child) > base, "own level did nothing");

        let own = g.output(child);
        g.level[Phylogeny::ROOT] += 10;
        assert!(g.output(child) > own, "ancestor level did nothing");
    }

    #[test]
    fn discovering_a_species_raises_the_rate_and_the_cap() {
        let mut g = new_game();
        let rate = g.rate();
        let cap = g.pool_cap();

        let child = g.phy.taxa[Phylogeny::ROOT].children[0];
        g.unlocked[child] = true;
        g.level[child] = 1;
        g.refresh_derived();

        assert!(g.rate() > rate);
        assert!(g.pool_cap() > cap, "cap should grow with progress alone");
    }

    /// The point of this whole redesign: a species is no longer an
    /// unconditional DNA/s producer -- it gives whatever bonus its nearest
    /// category ancestor does.
    #[test]
    fn a_species_gives_the_bonus_its_category_gives() {
        let mut g = new_game();
        let cnidarian = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Cnidarian")
            .unwrap();
        g.unlocked[cnidarian] = true;
        g.level[cnidarian] = 1;
        g.upgrade_kind[cnidarian] = Some(Kind::Chance);
        g.kind_level[Kind::Chance.index()] = 1;

        let jellyfish = g.phy.taxa[cnidarian].children[0];
        g.unlocked[jellyfish] = true;
        g.level[jellyfish] = 1;
        g.refresh_derived();

        assert_eq!(
            g.output(jellyfish),
            0.0,
            "a Chance-powered species should not produce DNA"
        );
        let (kind, amount) = g.species_bonus(jellyfish).unwrap();
        assert_eq!(kind, Kind::Chance);
        assert!(amount > 0.0, "it should still give a real Chance bonus");
        assert!(
            g.effects.chance_bonus > 0.0,
            "that bonus should be reflected in the combined effects"
        );
    }

    #[test]
    fn a_rate_powered_species_still_produces_dna_like_before() {
        let mut g = new_game();
        // The root is always Rate-kind, so any of its species descendants
        // inherit Rate by default -- the one case that must not have moved.
        let child = g.phy.taxa[Phylogeny::ROOT].children[0];
        g.unlocked[child] = true;
        g.level[child] = 1;
        g.refresh_derived();

        assert_eq!(g.nearest_category_kind(child), Some(Kind::Rate));
        assert!(g.output(child) > 0.0);
    }

    /// The bug report this fixed: a `Storage`-powered species always gave 0
    /// overflow contribution, no matter its level. It should behave like
    /// every other damped kind -- more the more it is leveled up, never 0
    /// once discovered.
    #[test]
    fn a_storage_powered_species_raises_the_overflow_multiplier() {
        let mut g = new_game();
        let bilaterian = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Bilaterian")
            .unwrap();
        g.unlocked[bilaterian] = true;
        g.level[bilaterian] = 1;
        g.upgrade_kind[bilaterian] = Some(Kind::Storage);
        g.kind_level[Kind::Storage.index()] = 1;

        let acoel = g.phy.taxa[bilaterian].children[0];
        g.unlocked[acoel] = true;
        g.level[acoel] = 1;
        g.refresh_derived();

        let Some((Kind::Storage, level_1)) = g.species_bonus(acoel) else {
            panic!("expected a Storage bonus");
        };
        assert!(
            level_1 > 0.0,
            "level 1 should already be worth something, not 0"
        );
        let overflow_at_1 = g.effects.overflow_bonus;

        g.level[acoel] = 32;
        g.refresh_derived();
        let Some((Kind::Storage, level_32)) = g.species_bonus(acoel) else {
            panic!("expected a Storage bonus");
        };
        assert!(
            level_32 > level_1,
            "level 32 should be worth more than level 1, not 0 ({level_32} <= {level_1})"
        );
        assert!(g.effects.overflow_bonus > overflow_at_1);
    }

    #[test]
    fn a_tap_kind_speeds_up_tap_leveling() {
        let mut g = new_game();
        unlock_an_animal(&mut g);
        let animal = g
            .phy
            .taxa
            .iter()
            .position(|t| t.group != Group::Backbone)
            .unwrap();

        g.upgrade_kind[Phylogeny::ROOT] = Some(Kind::Tap);
        g.kind_level[Kind::Tap.index()] = 5;
        g.refresh_derived();
        assert!(
            g.effects.tap_bonus > 0.0,
            "a Tap kind should raise tap_bonus"
        );

        // A big enough bonus can even fill the bar in one tap -- either
        // outcome proves the bonus took effect; only a flat, no-bonus fill
        // of exactly 1 would not.
        let leveled = g.tap_level(animal);
        assert!(
            leveled || g.tap_progress[animal] > 1,
            "one tap should fill more than 1 unit of progress once tap_bonus is above 0, got {} (leveled: {leveled})",
            g.tap_progress[animal]
        );
    }

    /// The bug report this fixed: `Kind::Cost` was computed but never
    /// actually consumed anywhere. Now it discounts every upgrade's price.
    #[test]
    fn a_cost_kind_lowers_every_upgrades_price() {
        let mut g = new_game();
        g.kind_level[Kind::Chance.index()] = 1;
        g.refresh_derived();
        let price_before = g.upgrade_level_cost(Kind::Chance);

        g.kind_level[Kind::Cost.index()] = 3;
        g.refresh_derived();

        assert!(
            g.upgrade_level_cost(Kind::Chance) < price_before,
            "a Cost upgrade anywhere should discount every upgrade's price"
        );
    }

    #[test]
    fn a_leaf_lineage_reports_exhausted_and_costs_nothing() {
        let mut g = new_game();
        fill(&mut g);
        let human = g.phy.taxa.iter().position(|t| t.name == "Human").unwrap();
        g.unlocked[human] = true;
        g.level[human] = 1;

        let before = g.dna;
        assert!(matches!(g.evolve(human), EvolveResult::Exhausted));
        assert_eq!(g.dna, before, "an impossible evolve charged anyway");
    }

    // --- upgrade shop ---------------------------------------------------------

    #[test]
    fn discovering_a_category_offers_a_choice_that_activates_its_upgrade() {
        let mut g = new_game();
        fill(&mut g);

        // Force the roll toward Cnidarian, a Backbone child of the root.
        let cnidarian = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Cnidarian")
            .unwrap();
        loop {
            fill(&mut g);
            if let EvolveResult::Unlocked(child) = g.evolve(Phylogeny::ROOT) {
                if child == cnidarian {
                    break;
                }
                // Landed on a sibling instead: put it back and try again.
                g.unlocked[child] = false;
                g.level[child] = 0;
                g.refresh_derived();
            }
        }

        // Discovered, but the choice hasn't resolved yet.
        assert_eq!(g.upgrade_kind[cnidarian], None, "not active until chosen");
        let pending = g
            .pending_choice
            .expect("a category discovery should offer a choice");
        assert_eq!(pending.taxon, cnidarian);
        assert_ne!(pending.options[0], pending.options[1]);
        assert_ne!(pending.options[0], pending.options[2]);
        assert_ne!(pending.options[1], pending.options[2]);

        let chosen = pending.options[1];
        assert!(g.choose_upgrade_kind(cnidarian, chosen));
        assert_eq!(
            g.kind_level(chosen),
            1,
            "level 1 should be free on choosing"
        );
        assert_eq!(g.upgrade_kind[cnidarian], Some(chosen));
        assert!(g.pending_choice.is_none(), "a resolved choice should clear");
    }

    /// Two different categories choosing the same `Kind` must not stack a
    /// second free level -- there is only ever one shared counter per
    /// `Kind`, not one per taxon. See `Game::choose_upgrade_kind`.
    #[test]
    fn choosing_an_already_active_kind_again_grants_no_extra_level() {
        let mut g = new_game();
        let cnidarian = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Cnidarian")
            .unwrap();
        g.unlocked[cnidarian] = true;
        g.level[cnidarian] = 1;
        g.pending_choice = Some(PendingChoice {
            taxon: cnidarian,
            options: [Kind::Rate, Kind::Cost, Kind::Chance],
        });

        // Root is always Rate at level 1 already.
        assert_eq!(g.kind_level(Kind::Rate), 1);
        assert!(g.choose_upgrade_kind(cnidarian, Kind::Rate));
        assert_eq!(
            g.kind_level(Kind::Rate),
            1,
            "picking an already-active kind again must not bump its level"
        );
        assert_eq!(
            g.upgrade_kind[cnidarian],
            Some(Kind::Rate),
            "the taxon itself is still marked with it, for colour/inheritance"
        );
    }

    #[test]
    fn choosing_a_kind_not_offered_is_refused() {
        let mut g = new_game();
        fill(&mut g);
        let cnidarian = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Cnidarian")
            .unwrap();
        loop {
            fill(&mut g);
            if let EvolveResult::Unlocked(child) = g.evolve(Phylogeny::ROOT) {
                if child == cnidarian {
                    break;
                }
                g.unlocked[child] = false;
                g.level[child] = 0;
                g.refresh_derived();
            }
        }
        let pending = g.pending_choice.unwrap();
        let not_offered = upgrades::ALL_KINDS
            .into_iter()
            .find(|k| !pending.options.contains(k))
            .expect("more kinds than are ever offered -- at least one is left out");

        assert!(!g.choose_upgrade_kind(cnidarian, not_offered));
        assert_eq!(
            g.upgrade_kind[cnidarian], None,
            "refused choice must not apply"
        );
        assert!(
            g.pending_choice.is_some(),
            "refused choice must stay pending"
        );
    }

    #[test]
    fn buying_a_level_needs_the_kind_active_and_dna_in_hand_and_never_caps() {
        let mut g = new_game();

        // Never chosen: no upgrade to buy, regardless of DNA.
        fill(&mut g);
        assert!(!g.can_afford_upgrade_level(Kind::Tap));
        assert!(!g.buy_upgrade_level(Kind::Tap));

        // Activated: level 1 is already free.
        g.kind_level[Kind::Tap.index()] = 1;
        g.refresh_derived();

        // Poor: refused.
        g.dna = 0.0;
        assert!(!g.can_afford_upgrade_level(Kind::Tap));

        // Rich: buys repeatedly, with no ceiling, each costing more than the last.
        let mut last_cost = 0.0;
        for expected_level in 2..=6 {
            fill(&mut g);
            let cost = g.upgrade_level_cost(Kind::Tap);
            assert!(
                cost > last_cost,
                "level {expected_level} should cost more than the last"
            );
            last_cost = cost;
            assert!(g.buy_upgrade_level(Kind::Tap));
            assert_eq!(g.kind_level(Kind::Tap), expected_level);
        }
    }

    #[test]
    fn buying_an_upgrade_level_only_ever_spends_liquid_dna() {
        let mut g = new_game();
        g.kind_level[Kind::Chance.index()] = 1;
        g.refresh_derived();

        // Poor: refused, however full the pool cap theoretically is.
        g.dna = 0.0;
        assert!(!g.can_afford_upgrade_level(Kind::Chance));
        assert!(!g.buy_upgrade_level(Kind::Chance));

        let cost = g.upgrade_level_cost(Kind::Chance);
        g.dna = cost;
        assert!(g.can_afford_upgrade_level(Kind::Chance));
        assert!(g.buy_upgrade_level(Kind::Chance));
        assert_eq!(g.kind_level(Kind::Chance), 2);
        assert!((g.dna).abs() < 1e-6, "the exact cost should be spent");
    }

    /// A `Kind` no taxon has ever chosen yet has nothing to buy -- regardless
    /// of how much DNA is on hand.
    #[test]
    fn an_unchosen_kind_has_no_upgrade_to_buy() {
        let mut g = new_game();
        fill(&mut g);
        assert!(!g.can_afford_upgrade_level(Kind::Tap));
    }

    #[test]
    fn upgrade_level_is_independent_of_the_taxons_own_level() {
        let mut g = new_game();
        let arthropod = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Arthropod")
            .unwrap();
        g.unlocked[arthropod] = true;
        g.level[arthropod] = 1;
        g.kind_level[Kind::Chance.index()] = 1;
        g.refresh_derived();

        // Buy several upgrade levels without ever touching `level`.
        for _ in 0..4 {
            fill(&mut g);
            g.buy_upgrade_level(Kind::Chance);
        }
        assert_eq!(g.kind_level(Kind::Chance), 5);
        assert_eq!(g.level[arthropod], 1, "level should not have moved");
    }

    #[test]
    fn unlocked_and_level_never_disagree() {
        let mut g = new_game();
        for _ in 0..3000 {
            for i in 0..g.phy.len() {
                if g.unlocked[i] {
                    fill(&mut g);
                    g.evolve(i);
                }
            }
            if g.discovered() == g.phy.len() {
                break;
            }
        }
        assert_eq!(g.discovered(), g.phy.len(), "never finished the tree");
        for i in 0..g.phy.len() {
            assert_eq!(g.unlocked[i], g.level[i] > 0, "mismatch at {i}");
        }
    }

    // --- persistence ------------------------------------------------------

    #[test]
    fn a_fresh_install_has_nothing_to_load() {
        let _guard = SAVE_LOCK.lock().unwrap();
        crate::save::clear();
        assert!(Game::load(0.0).is_none());
    }

    #[test]
    fn save_then_load_round_trips_the_durable_state() {
        let _guard = SAVE_LOCK.lock().unwrap();
        let mut g = new_game();
        let child = g.phy.taxa[Phylogeny::ROOT].children[0];
        g.unlocked[child] = true;
        g.level[child] = 3;
        g.kind_level[Kind::Rate.index()] = 2;
        g.pressure[child] = 4;
        g.tap_progress[child] = 5;
        // Within `pool_cap()` for this much progress -- `load`'s catch-up
        // tick clamps to the cap same as any other tick, so a dna value
        // that would not fit here is a broken fixture, not a real save.
        g.dna = g.pool_cap() * 0.5;
        g.taps = 7;

        g.save(1000.0);
        let loaded = Game::load(1000.0).expect("just-written save should load");

        assert_eq!(loaded.unlocked, g.unlocked);
        assert_eq!(loaded.level, g.level);
        assert_eq!(loaded.kind_level, g.kind_level);
        assert_eq!(loaded.pressure, g.pressure);
        assert_eq!(loaded.tap_progress, g.tap_progress);
        assert_eq!(loaded.dna, g.dna);
        assert_eq!(loaded.taps, g.taps);

        crate::save::clear();
    }

    /// `rate`/`effects` are cached, not saved -- `load` must rebuild them
    /// from the taxa it did restore, not leave a species producing nothing.
    #[test]
    fn loading_rebuilds_the_cached_derived_fields() {
        let _guard = SAVE_LOCK.lock().unwrap();
        let mut g = new_game();
        unlock_an_animal(&mut g);
        assert!(g.rate() > 0.0);

        g.save(1000.0);
        let loaded = Game::load(1000.0).expect("just-written save should load");
        assert_eq!(loaded.rate(), g.rate());

        crate::save::clear();
    }

    /// Loading catches up on elapsed wall-clock time exactly like a running
    /// `tick` would -- the pool should not just resume frozen where it left off.
    #[test]
    fn loading_catches_up_on_elapsed_wall_clock() {
        let _guard = SAVE_LOCK.lock().unwrap();
        let mut g = new_game();
        g.dna = 0.0;
        g.save(1000.0);

        let loaded = Game::load(1000.0 + 5.0).expect("just-written save should load");
        assert!(
            loaded.dna > 0.0,
            "loading should accrue elapsed time, not resume frozen"
        );

        crate::save::clear();
    }

    /// A save from a tree shape the game no longer has (a content update
    /// added/removed taxa) must not load into a state that panics on the
    /// first index -- `new` is the fallback caller uses instead.
    #[test]
    fn a_save_with_the_wrong_taxon_count_is_rejected() {
        let _guard = SAVE_LOCK.lock().unwrap();
        let bogus = SaveData {
            unlocked: vec![true; 3],
            level: vec![1; 3],
            kind_level: [0; upgrades::NUM_KINDS],
            upgrade_kind: vec![None; 3],
            pressure: vec![0; 3],
            tap_progress: vec![0; 3],
            dna: 0.0,
            taps: 0,
            pending_choice: None,
            saved_at: 0.0,
        };
        crate::save::write(&serde_json::to_string(&bogus).unwrap());

        assert!(Game::load(0.0).is_none());
        crate::save::clear();
    }
}
