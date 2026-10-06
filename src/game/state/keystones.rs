//! Keystone equipping, morph choice and sleep rules.
use super::*;

impl Game {
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

    pub fn ma_elapsed(&self, now: f64) -> u32 {
        let running = self.cycle.map_or(0.0, |c| c.progress(now) * c.ma as f64);
        self.ma_done + running as u32
    }

    pub fn most_advanced(&self) -> usize {
        (0..self.phy.len())
            .filter(|&i| self.unlocked[i])
            .max_by(|&a, &b| {
                let (ta, tb) = (&self.phy.taxa[a], &self.phy.taxa[b]);
                ta.depth.cmp(&tb.depth).then(tb.mya.total_cmp(&ta.mya))
            })
            .unwrap_or(Phylogeny::ROOT)
    }

    pub fn era(&self) -> &'static str {
        let mya = (0..self.phy.len())
            .filter(|&i| self.unlocked[i])
            .map(|i| self.phy.taxa[i].mya)
            .fold(f32::INFINITY, f32::min);
        era_for(mya)
    }

    pub(super) fn refresh_levels(&mut self) {
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

    pub fn edition(&self, taxon: usize) -> Morph {
        if self.edition_is_best(taxon) {
            self.best_edition(taxon)
        } else {
            Morph::from_bit(self.edition[taxon] & self.morphs[taxon])
        }
    }

    pub fn edition_is_best(&self, taxon: usize) -> bool {
        self.edition[taxon] & PICKED == 0
    }

    /// The owned morph (or none) that makes the strongest keystone on the
    /// planet it lives on: awake first, then `Morph::rank`. So an albino
    /// coat is worn only where it's cool enough, and a melanistic one where
    /// only it keeps the animal awake.
    pub(super) fn best_edition(&self, taxon: usize) -> Morph {
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

    pub fn best_morph(&self, taxon: usize) -> Morph {
        Morph::ALL
            .into_iter()
            .rev()
            .find(|&m| self.owns_morph(taxon, m))
            .unwrap_or(Morph::None)
    }

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

    pub(super) fn living_planet(&self) -> Planet {
        self.cycle.map_or(self.planet, |c| c.launched)
    }

    pub(super) fn own_sleep(&self, taxon: usize) -> Option<String> {
        self.own_sleep_with(taxon, self.edition(taxon))
    }

    pub(super) fn own_sleep_with(&self, taxon: usize, morph: Morph) -> Option<String> {
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
    pub(super) fn tyrants_prey(&self) -> Option<usize> {
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

    pub(super) fn descends(&self, taxon: usize, ancestor: &str) -> bool {
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

    pub(super) fn bonus_gains(bonus: Bonus, amount: f32) -> Vec<Gain> {
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
}
