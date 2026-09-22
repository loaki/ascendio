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

use crate::tree::{Group, Phylogeny, Taxon};
use crate::upgrades::{self, Effects};

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

/// Chance that Evolve on a depth-0 taxon succeeds, before any bonus.
const BASE_CHANCE: f32 = 0.45;
/// Each generation down the tree is this much rarer.
const DEPTH_FALLOFF: f32 = 0.88;
const MIN_CHANCE: f32 = 0.04;
const MAX_CHANCE: f32 = 0.90;
/// Every failed Evolve builds mutation pressure on that lineage, raising the
/// odds until it breaks. Nothing is ever wasted.
const PRESSURE_PER_MISS: f32 = 0.06;
/// Chance added per natural log of (full pool / baseline unit cost). A
/// richer economy commits more DNA to a single Evolve, and that is rewarded
/// with better odds, not just a bigger number spent.
const SPEND_BONUS_SCALE: f32 = 0.16;

// --- upgrades ----------------------------------------------------------

/// DNA to buy an upgrade's first purchasable level (its second level overall
/// -- discovering the category already grants level 1 for free).
const UPGRADE_LEVEL_BASE: f64 = 50.0;
/// Dearer per generation of depth, the same shape as everything else here.
const UPGRADE_LEVEL_PER_DEPTH: f64 = 1.5;
/// Dearer again for every level already bought -- an infinite sink, but a
/// steepening one.
const UPGRADE_LEVEL_PER_LEVEL: f64 = 1.6;

const TOAST_SECONDS: f32 = 2.5;
const PULSE_SECONDS: f32 = 0.35;

pub struct Game {
    pub phy: Phylogeny,
    pub unlocked: Vec<bool>,
    /// 0 while undiscovered, then 1 and up. A repeat discovery is a level,
    /// raising a species' own DNA output. Unrelated to a category's upgrade
    /// strength -- see `upgrade_level`.
    pub level: Vec<u32>,
    /// A `Group::Backbone` taxon's upgrade level: 0 before it is discovered,
    /// 1 the instant it is (free), and climbing from there only by direct
    /// purchase (`buy_upgrade_level`) -- infinitely, independently of
    /// `level` above. Meaningless (stays 0) for anything else.
    pub upgrade_level: Vec<u32>,
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
    pub toast: Option<Toast>,
    /// Evolves banked by the Amniote (`Kind::Storage`) upgrade: reaching a
    /// full pool -- typically while away -- banks a charge and restarts the
    /// count from 0 instead of just idling at the cap. Spending one is an
    /// alternative to a full pool, not on top of it -- see `evolve`.
    pub banked_evolves: u32,
}

pub struct Toast {
    pub text: String,
    pub remaining: f32,
}

/// What an Evolve produced.
pub enum EvolveResult {
    /// A new descendant lineage split off.
    Unlocked(usize),
    /// Every descendant was already known, so one of them got stronger.
    Leveled(usize),
    /// Rolled and lost. Pressure went up.
    Miss,
    /// This taxon has no descendants in the tree at all.
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
        let mut upgrade_level = vec![0; n];
        if phy.taxa[Phylogeny::ROOT].group == Group::Backbone {
            upgrade_level[Phylogeny::ROOT] = 1;
        }

        let mut game = Self {
            phy,
            unlocked,
            level,
            upgrade_level,
            pressure: vec![0; n],
            dna: 0.0,
            rate: 0.0,
            effects: Effects::default(),
            last_tick: now,
            pulse: vec![0.0; n],
            taps: 0,
            toast: None,
            banked_evolves: 0,
        };
        game.refresh_derived();
        // Open on a full pool, so the first session starts immediately.
        game.dna = game.pool_cap();
        game
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

    /// Every ancestor's levels feed into a species' output.
    fn lineage_bonus(&self, i: usize) -> f64 {
        let mut levels = 0u32;
        let mut cur = self.phy.taxa[i].parent;
        while let Some(p) = cur {
            levels += self.level[p];
            cur = self.phy.taxa[p].parent;
        }
        1.0 + LINEAGE_BONUS * levels as f64
    }

    /// DNA per second from one species. Only species produce DNA -- a
    /// `Backbone` taxon is a category, not an animal, and its payoff is an
    /// upgrade instead (see `upgrades.rs`).
    pub fn output(&self, i: usize) -> f64 {
        if !self.unlocked[i] || self.phy.taxa[i].group == Group::Backbone {
            return 0.0;
        }
        let depth = self.phy.taxa[i].depth as i32;
        BASE_OUTPUT * OUTPUT_PER_DEPTH.powi(depth) * self.level[i] as f64 * self.lineage_bonus(i)
    }

    /// Recomputes everything that depends on which taxa are unlocked, at what
    /// level, and which upgrades are bought: the effective DNA/s and the
    /// combined upgrade effects. Called once at startup and again after
    /// every discovery, level-up or purchase.
    fn refresh_derived(&mut self) {
        self.effects = upgrades::compute(&self.phy, &self.upgrade_level);
        let raw: f64 = (0..self.phy.len()).map(|i| self.output(i)).sum();
        self.rate = raw * self.effects.rate_mul;
    }

    /// DNA per second across the whole tree, upgrades included. Shown on the
    /// HUD as "how much your species are producing" -- see `fill_rate` for
    /// what actually paces the pool.
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// The combined effect of every purchased upgrade.
    pub fn effects(&self) -> Effects {
        self.effects
    }

    /// The pool's ceiling: a floor, plus a fixed amount per taxon you have
    /// found, times whatever purchased Cap upgrades multiply it by. Grows
    /// with *progress*, not with DNA/s, so an opening run of DNA-free
    /// categories still grows your capacity.
    pub fn pool_cap(&self) -> f64 {
        (MIN_POOL + CAP_PER_DISCOVERY * self.progress()) * self.effects.cap_mul
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

    /// Evolve only ever fires once the pool has actually topped out --
    /// or, with a banked charge in hand, without one. See `can_evolve`.
    pub fn pool_full(&self) -> bool {
        self.dna >= self.pool_cap() - 1e-6
    }

    /// Whether Evolve has something to fire with right now: a full pool, or
    /// a charge the Amniote upgrade already banked.
    pub fn can_evolve(&self) -> bool {
        self.pool_full() || self.banked_evolves > 0
    }

    /// Accrues DNA against the wall clock, so time spent with the app
    /// backgrounded still counts. `now` is seconds since the epoch. With no
    /// `Storage` upgrade the pool just clamps at the cap as ever; with one,
    /// topping out banks a charge and restarts the count instead, up to the
    /// upgrade's slot count -- so a long stretch away can bank several.
    pub fn tick(&mut self, now: f64) {
        let dt = (now - self.last_tick).clamp(0.0, MAX_CATCHUP_SECONDS);
        self.last_tick = now;
        let cap = self.pool_cap();
        let mut dna = self.dna + self.fill_rate() * dt;
        let slots = self.effects.storage_slots;
        while slots > 0 && dna >= cap && self.banked_evolves < slots {
            dna -= cap;
            self.banked_evolves += 1;
        }
        self.dna = dna.min(cap);
    }

    /// The reference cost Evolve's odds are measured against: dearer the
    /// deeper `i` is, and dearer again for every descendant already found.
    /// Nothing is charged at this rate directly any more -- see
    /// `unlock_chance`'s spend bonus.
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

    /// How many baseline units a *full* pool represents for `i`. This is
    /// "the amount of DNA you spent" that Evolve's odds scale with -- based
    /// on the cap, not the current charge, so the number on the card is a
    /// stable preview of what Evolve will do once it is actually full.
    fn spend_ratio(&self, i: usize) -> f32 {
        (self.pool_cap() / self.baseline_cost(i)).max(1.0) as f32
    }

    /// Chance the next Evolve on `i` succeeds: depth-based odds, mutation
    /// pressure, any purchased chance upgrades, and a bonus for how much DNA
    /// a full pool commits relative to a bare attempt. A richer economy
    /// lands its Evolves more reliably, not just spends bigger numbers.
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

    // --- evolve --------------------------------------------------------------

    /// Rolls the dice on `i`. Only ever does anything with a full pool --
    /// the whole pool is the bet, win or lose, which is what makes "the
    /// amount you spent" mean something: it is always everything you had.
    pub fn evolve(&mut self, i: usize) -> EvolveResult {
        if !self.unlocked[i] {
            return EvolveResult::NotReady;
        }
        if self.phy.taxa[i].children.is_empty() {
            return EvolveResult::Exhausted;
        }
        let full = self.pool_full();
        if !full && self.banked_evolves == 0 {
            return EvolveResult::NotReady;
        }

        let chance = self.unlock_chance(i);
        if full {
            self.dna = 0.0;
        } else {
            self.banked_evolves -= 1;
        }
        self.taps += 1;
        self.pulse[i] = PULSE_SECONDS;

        if rand::gen_range(0.0, 1.0) >= chance {
            self.pressure[i] += 1;
            return EvolveResult::Miss;
        }
        self.pressure[i] = 0;

        let undiscovered: Vec<usize> = self.phy.taxa[i]
            .children
            .iter()
            .copied()
            .filter(|&c| !self.unlocked[c])
            .collect();

        if !undiscovered.is_empty() {
            let child = undiscovered[rand::gen_range(0, undiscovered.len())];
            self.unlocked[child] = true;
            self.level[child] = 1;
            // A category's upgrade is free the instant it is found -- buying
            // only ever raises it beyond this starting level.
            if self.phy.taxa[child].group == Group::Backbone {
                self.upgrade_level[child] = 1;
            }
            self.pulse[child] = PULSE_SECONDS;
            self.refresh_derived();
            self.toast = Some(Toast {
                text: format!("{} ({})", self.taxon(child).name, self.taxon(child).clade),
                remaining: TOAST_SECONDS,
            });
            return EvolveResult::Unlocked(child);
        }

        // Nothing new left down this branch, so the roll reinforces instead:
        // evolving Mollusca again is what levels up Octopus.
        let kids = &self.phy.taxa[i].children;
        let child = kids[rand::gen_range(0, kids.len())];
        self.level[child] += 1;
        self.pulse[child] = PULSE_SECONDS;
        self.refresh_derived();
        self.toast = Some(Toast {
            text: format!("{}  Lv {}", self.taxon(child).name, self.level[child]),
            remaining: TOAST_SECONDS,
        });
        EvolveResult::Leveled(child)
    }

    // --- upgrade shop ---------------------------------------------------------

    /// DNA to raise `i`'s upgrade from its current level to the next. Only
    /// meaningful once it has one -- i.e. `upgrade_level[i] >= 1`.
    pub fn upgrade_level_cost(&self, i: usize) -> f64 {
        let depth = self.phy.taxa[i].depth as i32;
        // Level 1 is free (granted on discovery), so the first purchasable
        // step is 1 -> 2, at the base price for this taxon's depth.
        let steps_already_bought = self.upgrade_level[i].saturating_sub(1);
        UPGRADE_LEVEL_BASE
            * UPGRADE_LEVEL_PER_DEPTH.powi(depth)
            * UPGRADE_LEVEL_PER_LEVEL.powi(steps_already_bought as i32)
    }

    pub fn can_afford_upgrade_level(&self, i: usize) -> bool {
        self.upgrade_level[i] >= 1 && self.dna >= self.upgrade_level_cost(i)
    }

    /// Spends DNA to raise `i`'s upgrade by one level -- unbounded, and
    /// entirely independent of `level[i]`. Returns whether it went through.
    pub fn buy_upgrade_level(&mut self, i: usize) -> bool {
        if !self.can_afford_upgrade_level(i) {
            return false;
        }
        self.dna -= self.upgrade_level_cost(i);
        self.upgrade_level[i] += 1;
        self.refresh_derived();
        true
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.pulse {
            *p = (*p - dt).max(0.0);
        }
        if let Some(t) = &mut self.toast {
            t.remaining -= dt;
            if t.remaining <= 0.0 {
                self.toast = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_game() -> Game {
        Game::new(0.0)
    }

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

    /// Evolves `i` until it stops missing (pressure keeps raising the odds,
    /// so this always terminates well under the retry budget) and returns
    /// the first non-`Miss`, non-`NotReady` result.
    fn evolve_until_it_lands(game: &mut Game, i: usize) -> EvolveResult {
        for _ in 0..200 {
            fill(game);
            match game.evolve(i) {
                EvolveResult::Miss => continue,
                other => return other,
            }
        }
        panic!("never landed in 200 tries");
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
    /// independent of `rate`.
    #[test]
    fn the_pool_refills_even_at_zero_dna_per_second() {
        let mut g = new_game();
        assert_eq!(g.rate(), 0.0, "a fresh game has no species discovered yet");
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

    /// Gives `g` some `Storage` slots directly, bypassing discovery/purchase
    /// -- these tests are about the banking mechanic, not how you get there.
    fn grant_storage_slots(game: &mut Game, slots: u32) {
        let amniote = game
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Amniote")
            .unwrap();
        game.upgrade_level[amniote] = slots;
        game.refresh_derived();
    }

    #[test]
    fn topping_out_with_storage_banks_a_charge_and_restarts_the_pool() {
        let mut g = new_game();
        grant_storage_slots(&mut g, 1);
        g.dna = 0.0;

        // Enough to reach the cap and a bit past it, not enough for a second.
        g.tick(g.refill_seconds() * 1.5);
        assert_eq!(g.banked_evolves, 1);
        assert!(
            g.dna < g.pool_cap(),
            "overflow past the cap should have gone into the bank, not sat in the pool"
        );
    }

    #[test]
    fn a_long_absence_banks_up_to_the_slot_count_and_no_further() {
        let mut g = new_game();
        grant_storage_slots(&mut g, 2);
        g.dna = 0.0;

        g.tick(g.refill_seconds() * 50.0);
        assert_eq!(g.banked_evolves, 2, "should never exceed the slot count");
        assert!(
            (g.dna - g.pool_cap()).abs() < 1e-6,
            "should sit at the cap once banked out"
        );
    }

    #[test]
    fn no_storage_upgrade_behaves_exactly_as_before() {
        let mut g = new_game();
        g.dna = 0.0;
        g.tick(g.refill_seconds() * 50.0);
        assert_eq!(g.banked_evolves, 0);
        assert!(g.pool_full(), "should just clamp at the cap as ever");
    }

    #[test]
    fn a_banked_charge_lets_evolve_fire_without_a_full_pool() {
        let mut g = new_game();
        grant_storage_slots(&mut g, 1);
        g.banked_evolves = 1;
        g.dna = 0.0;
        assert!(!g.pool_full());
        assert!(g.can_evolve());

        let before = g.dna;
        assert!(!matches!(g.evolve(Phylogeny::ROOT), EvolveResult::NotReady));
        assert_eq!(g.banked_evolves, 0, "the charge should be spent");
        assert_eq!(
            g.dna, before,
            "spending a banked charge must not touch the live pool"
        );
    }

    #[test]
    fn with_no_full_pool_and_no_banked_charge_evolve_is_not_ready() {
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

    /// The rule that makes every Evolve pay: once a branch is fully
    /// discovered, a success reinforces a descendant instead of being wasted.
    #[test]
    fn a_success_on_a_fully_discovered_branch_levels_a_descendant() {
        let mut g = new_game();

        let mollusc = g.phy.taxa.iter().position(|t| t.name == "Mollusc").unwrap();
        g.unlocked[mollusc] = true;
        g.level[mollusc] = 1;
        for &c in &g.phy.taxa[mollusc].children.clone() {
            g.unlocked[c] = true;
            g.level[c] = 1;
        }
        g.refresh_derived();

        let before: u32 = g.phy.taxa[mollusc]
            .children
            .iter()
            .map(|&c| g.level[c])
            .sum();

        assert!(matches!(
            evolve_until_it_lands(&mut g, mollusc),
            EvolveResult::Leveled(_)
        ));
        let after: u32 = g.phy.taxa[mollusc]
            .children
            .iter()
            .map(|&c| g.level[c])
            .sum();
        assert_eq!(after, before + 1);
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
    fn discovering_a_category_grants_its_upgrade_at_level_one_for_free() {
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
                g.upgrade_level[child] = 0;
                g.refresh_derived();
            }
        }

        assert_eq!(
            g.upgrade_level[cnidarian], 1,
            "level 1 should be free on discovery"
        );
        assert!(
            g.effects().chance_bonus > 0.0,
            "the free level should already be active"
        );
    }

    #[test]
    fn buying_a_level_needs_the_category_discovered_and_dna_in_hand_and_never_caps() {
        let mut g = new_game();
        let arthropod = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Arthropod")
            .unwrap();

        // Not discovered: no upgrade to buy, regardless of DNA.
        fill(&mut g);
        assert!(!g.can_afford_upgrade_level(arthropod));
        assert!(!g.buy_upgrade_level(arthropod));

        // Discovered: level 1 is already free.
        g.unlocked[arthropod] = true;
        g.level[arthropod] = 1;
        g.upgrade_level[arthropod] = 1;
        g.refresh_derived();

        // Poor: refused.
        g.dna = 0.0;
        assert!(!g.can_afford_upgrade_level(arthropod));

        // Rich: buys repeatedly, with no ceiling, each costing more than the last.
        let mut last_cost = 0.0;
        for expected_level in 2..=6 {
            fill(&mut g);
            let cost = g.upgrade_level_cost(arthropod);
            assert!(
                cost > last_cost,
                "level {expected_level} should cost more than the last"
            );
            last_cost = cost;
            assert!(g.buy_upgrade_level(arthropod));
            assert_eq!(g.upgrade_level[arthropod], expected_level);
        }
    }

    #[test]
    fn a_species_has_no_upgrade_to_buy() {
        let mut g = new_game();
        fill(&mut g);
        let sponge = g
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Sea Sponge")
            .unwrap();
        g.unlocked[sponge] = true;
        assert!(!g.can_afford_upgrade_level(sponge));
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
        g.upgrade_level[arthropod] = 1;
        g.refresh_derived();

        // Buy several upgrade levels without ever touching `level`.
        for _ in 0..4 {
            fill(&mut g);
            g.buy_upgrade_level(arthropod);
        }
        assert_eq!(g.upgrade_level[arthropod], 5);
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
}
