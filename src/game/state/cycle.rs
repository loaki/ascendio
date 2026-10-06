//! Cycle end, genome opening and boon picking.
use super::*;

impl Game {
    pub fn tick(&mut self, now: f64) {
        // A clock set back (web) mustn't stretch the wait past its length.
        if let Some(c) = &mut self.cycle {
            c.started_at = c.started_at.min(now);
        }
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
            &mut self.rng,
        );
        self.genome = Some(rolled);
        // Decided with the genome, so reopening the app can't reroll it.
        // The chance is per 4h and compounds, so splitting a wait into
        // shorter ones never rolls it more often.
        let e = self.effects();
        let per_4h = e.doom;
        let doom = 1.0 - (1.0 - per_4h).powf(cycle.hours / wait::DEFAULT_HOURS);
        self.doom_risk = doom;
        self.doomed = doom > 0.0 && self.rng.chance(doom);
        // Likewise the boons: keystones swapped during the wait only count
        // from the next one.
        self.next_boons = Some(if e.fewer_boons { 2 } else { 3 });
        self.earn_keystone_points(cycle.hours);
        self.last_hours = cycle.hours;
        self.advance_keystones(cycle.launched, cycle.ma);
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

    pub fn skip_cycle(&mut self, now: f64) {
        if let Some(c) = &mut self.cycle {
            c.started_at = now - c.duration;
        }
        self.tick(now);
    }

    pub fn genome_tell(&self) -> Option<Tier> {
        self.genome.as_deref().map(genome::tell)
    }

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

    pub(super) fn roll_boons(&mut self) -> Vec<Boon> {
        let mut pool = vec![
            Boon::Discovery,
            Boon::Lens,
            Boon::Catalyst,
            Boon::Charm,
            Boon::Tailwind,
            Boon::Tectonics,
        ];
        let n = match self.next_boons.take() {
            Some(n) => usize::from(n),
            None if self.effects().fewer_boons => 2,
            None => 3,
        };
        let mut offer = Vec::with_capacity(n);
        while offer.len() < n && !pool.is_empty() {
            let b = pool.swap_remove((self.rng.next_u64() % pool.len() as u64) as usize);
            if !offer.contains(&b) {
                offer.push(b);
            }
        }
        offer
    }

    pub fn choose_boon(&mut self, i: usize) {
        let Some(offer) = self.boon_offer.take() else {
            return;
        };
        self.boon = offer.get(i).copied();
        self.shaped_from = self.planet;
    }

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

    /// Every animal found stays a fossil with its specimens and morphs;
    /// the planet, the spiral, Ma, keystones and the genome reset.
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
        self.keystones.clear();
        self.boon = None;
        self.boon_offer = None;
        self.next_boons = None;
        self.growth = vec![0; n];
        self.gone = vec![false; n];
        self.last_launched = None;
        self.wait_points = planet::BASE_POINTS;
        self.bonus_points = 0;
        self.point_carry = 0.0;
        self.last_hours = wait::DEFAULT_HOURS;
        self.refresh_levels();
    }

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
