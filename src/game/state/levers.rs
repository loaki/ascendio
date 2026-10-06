//! Lever stepping, adjustment-point budget and genome odds.
use super::*;

impl Game {
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
                    self.phy.taxa[r.taxon].label(),
                    planet::temperature_label(t)
                )),
                _ => None,
            })
        })
    }

    /// Advances what the keystones a finished wait ran with keep track of,
    /// on the planet it ran on: the Tuatara's and the Coelacanth's charges,
    /// the Dodo wearing out. A growing keystone that sat the wait out loses
    /// its charges, as an asleep one does, so they can't be banked by
    /// unequipping it for a wait that would empty them.
    pub(super) fn advance_keystones(&mut self, launched: Planet, ma: u32) {
        let ran = self.active_keystones();
        for k in 0..self.phy.len() {
            if !ran.contains(&k)
                && matches!(self.phy.taxa[k].eco.rule, Rule::Changing | Rule::Stasis)
            {
                self.growth[k] = 0;
            }
        }
        for k in ran {
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

    pub fn max_points(&self) -> u8 {
        let tectonics = if self.boon == Some(Boon::Tectonics) {
            self.tectonics_points()
        } else {
            0
        };
        (self.wait_points + self.bonus_points + tectonics).min(MAX_POINTS)
    }

    pub fn tectonics_points(&self) -> u8 {
        (TECTONICS_POINTS * self.last_hours / wait::DEFAULT_HOURS).round() as u8
    }

    pub(super) fn earn_keystone_points(&mut self, hours: f32) {
        let earned = self.effects().points as f32 * hours / wait::DEFAULT_HOURS + self.point_carry;
        self.bonus_points = (earned.floor() as u8).min(MAX_POINTS);
        self.point_carry = earned.fract();
    }

    pub(super) fn cost(&self, p: &Planet) -> u8 {
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

    pub(super) fn stepped(&self, lever: Lever, delta: i8) -> Option<Planet> {
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

    pub fn wait_choosable(&self) -> bool {
        self.cycles_done >= planet::TUTORIAL_CYCLES
    }

    pub fn set_wait(&mut self, hours: f32) {
        self.wait_hours = wait::snap(hours);
    }

    pub fn next_hours(&self) -> f32 {
        if self.wait_choosable() {
            self.wait_hours
        } else {
            wait::TUTORIAL_HOURS
        }
    }

    pub fn forecast(&self) -> Forecast {
        self.forecast_for(self.next_hours())
    }

    pub(super) fn forecast_for(&self, hours: f32) -> Forecast {
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

    pub fn cancel_cycle(&mut self) -> bool {
        self.cycle.take().is_some()
    }
}
