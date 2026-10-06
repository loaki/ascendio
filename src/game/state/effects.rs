//! Keystone effects: statuses, copies, biome bonuses and totals.
use super::*;

impl Game {
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
                        let mut gains = base(s * (1.0 + CHARGE_BONUS * grown));
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
                    r.note = Some(format!("copying {}", self.phy.taxa[ks[j]].label()));
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

    pub fn active_biomes(&self) -> Vec<Biome> {
        self.biomes_from(&self.keystone_reports())
    }

    pub(super) fn biomes_from(&self, reports: &[Report]) -> Vec<Biome> {
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
}
