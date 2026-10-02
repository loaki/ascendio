//! The planet: the five levers, the habitats they imply, and the real-time
//! cycle that turns hours into millions of years.

use serde::{Deserialize, Serialize};

/// Highest level of every lever but `Volcanism`.
pub const LEVEL_MAX: u8 = 5;
pub const VOLCANISM_MAX: u8 = 2;
/// Points to spend on the first shaping and after the tutorial cycles;
/// after that the wait just run decides (`wait::points`).
pub const BASE_POINTS: u8 = 3;
/// The first cycles run for a fixed 1 and 20 minutes, to teach the loop;
/// after them the player picks the wait on the dial.
pub const TUTORIAL_CYCLES: u32 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Lever {
    Land,
    Vegetation,
    Oxygen,
    Temperature,
    Volcanism,
}

impl Lever {
    pub const ALL: [Lever; 5] = [
        Lever::Land,
        Lever::Vegetation,
        Lever::Oxygen,
        Lever::Temperature,
        Lever::Volcanism,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Lever::Land => "Land",
            Lever::Vegetation => "Vegetation",
            Lever::Oxygen => "Oxygen",
            Lever::Temperature => "Temperature",
            Lever::Volcanism => "Volcanism",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Habitat {
    Sea,
    Reef,
    Shore,
    Fresh,
    Land,
    Forest,
}

impl Habitat {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Habitat::Sea => "Sea",
            Habitat::Reef => "Reef",
            Habitat::Shore => "Shore",
            Habitat::Fresh => "Fresh water",
            Habitat::Land => "Land",
            Habitat::Forest => "Forest",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Planet {
    pub land: u8,
    pub vegetation: u8,
    pub oxygen: u8,
    pub temperature: u8,
    pub volcanism: u8,
}

impl Default for Planet {
    /// All sea, thin air, temperate, calm.
    fn default() -> Self {
        Self {
            land: 0,
            vegetation: 0,
            oxygen: 1,
            temperature: 3,
            volcanism: 0,
        }
    }
}

impl Planet {
    pub fn get(&self, lever: Lever) -> u8 {
        match lever {
            Lever::Land => self.land,
            Lever::Vegetation => self.vegetation,
            Lever::Oxygen => self.oxygen,
            Lever::Temperature => self.temperature,
            Lever::Volcanism => self.volcanism,
        }
    }

    /// Vegetation needs ground: at most twice the land level.
    pub fn max(&self, lever: Lever) -> u8 {
        match lever {
            Lever::Volcanism => VOLCANISM_MAX,
            Lever::Vegetation => (self.land * 2).min(LEVEL_MAX),
            _ => LEVEL_MAX,
        }
    }

    fn set(&mut self, lever: Lever, v: u8) {
        let v = v.min(self.max(lever));
        match lever {
            Lever::Land => self.land = v,
            Lever::Vegetation => self.vegetation = v,
            Lever::Oxygen => self.oxygen = v,
            Lever::Temperature => self.temperature = v,
            Lever::Volcanism => self.volcanism = v,
        }
        // Sinking the land drowns whatever grew on it.
        self.vegetation = self.vegetation.min(self.max(Lever::Vegetation));
    }

    /// Moves `lever` by `delta`; returns whether it actually moved.
    pub fn step(&mut self, lever: Lever, delta: i8) -> bool {
        let cur = self.get(lever);
        let next = (cur as i16 + delta as i16).clamp(0, self.max(lever) as i16) as u8;
        if next == cur {
            return false;
        }
        self.set(lever, next);
        true
    }

    pub fn has(&self, h: Habitat) -> bool {
        match h {
            Habitat::Sea => self.land <= 4,
            Habitat::Reef => self.has(Habitat::Sea) && self.temperature >= 3,
            Habitat::Shore => (1..=4).contains(&self.land),
            Habitat::Fresh => self.land >= 2 && self.temperature >= 1 && self.vegetation >= 1,
            Habitat::Land => self.land >= 1 && self.vegetation >= 1,
            Habitat::Forest => self.vegetation >= 3,
        }
    }

    /// The linked-lever rules applied once a cycle has run. `soil` is the
    /// number of "+1 vegetation per cycle" keystones.
    pub fn after_cycle(&self, soil: u8) -> Planet {
        let mut p = *self;
        if p.vegetation >= 3 {
            p.set(Lever::Oxygen, p.oxygen + 1); // forests breathe
        }
        if p.volcanism >= 2 {
            p.set(Lever::Temperature, p.temperature + 1); // volcanic greenhouse
        }
        if p.temperature <= 1 {
            p.set(Lever::Land, p.land + 1); // ice locks up the sea
        }
        if soil > 0 {
            p.set(Lever::Vegetation, p.vegetation + soil);
        }
        p
    }

    /// How much of the planet each habitat covers, summing to 1 (indexed by
    /// `Habitat::index`). A taxon's odds follow its habitat's share, so land
    /// and vegetation decide what turns up, not only what can.
    pub fn habitat_shares(&self) -> [f32; 6] {
        use Habitat::*;
        let mut w = [0.0f32; 6];
        let land = self.land as f32;
        if self.has(Sea) {
            w[Sea.index()] = (LEVEL_MAX as f32 - land) * 2.0;
        }
        if self.has(Reef) {
            // The reef takes half of the shallow sea.
            let reef = w[Sea.index()] * 0.5;
            w[Reef.index()] = reef;
            w[Sea.index()] -= reef;
        }
        if self.has(Shore) {
            w[Shore.index()] = 1.5;
        }
        if self.has(Fresh) {
            w[Fresh.index()] = 0.5 + land.min(4.0) * 0.5;
        }
        let ground = land * 2.0;
        let forest = if self.has(Forest) {
            self.vegetation as f32 / 6.0
        } else {
            0.0
        };
        w[Forest.index()] = ground * forest;
        if self.has(Land) {
            w[Land.index()] = ground * (1.0 - forest);
        }
        let total: f32 = w.iter().sum();
        if total > 0.0 {
            for x in &mut w {
                *x /= total;
            }
        }
        w
    }
}

/// Inclusive lever ranges a planet must sit in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ranges {
    pub land: (u8, u8),
    pub vegetation: (u8, u8),
    pub oxygen: (u8, u8),
    pub temperature: (u8, u8),
    pub volcanism: (u8, u8),
}

impl Ranges {
    pub const ANY: Ranges = Ranges {
        land: (0, LEVEL_MAX),
        vegetation: (0, LEVEL_MAX),
        oxygen: (0, LEVEL_MAX),
        temperature: (0, LEVEL_MAX),
        volcanism: (0, VOLCANISM_MAX),
    };

    pub fn get(&self, lever: Lever) -> (u8, u8) {
        match lever {
            Lever::Land => self.land,
            Lever::Vegetation => self.vegetation,
            Lever::Oxygen => self.oxygen,
            Lever::Temperature => self.temperature,
            Lever::Volcanism => self.volcanism,
        }
    }

    pub fn contains(&self, p: &Planet) -> bool {
        self.missing(p, false).is_none()
    }

    /// The first lever out of range, as a short hint ("too cold").
    /// `any_temperature` skips the Temperature range (a melanistic coat).
    pub fn missing(&self, p: &Planet, any_temperature: bool) -> Option<&'static str> {
        let out = |lever: Lever| {
            if any_temperature && lever == Lever::Temperature {
                return (false, false);
            }
            let (lo, hi) = self.get(lever);
            let v = p.get(lever);
            (v < lo, v > hi)
        };
        for lever in Lever::ALL {
            let (low, high) = out(lever);
            let hint = match (lever, low, high) {
                (Lever::Land, true, _) => "needs more land",
                (Lever::Land, _, true) => "needs more sea",
                (Lever::Vegetation, true, _) => "needs more vegetation",
                (Lever::Vegetation, _, true) => "needs open ground",
                (Lever::Oxygen, true, _) => "needs more oxygen",
                (Lever::Oxygen, _, true) => "too much oxygen",
                (Lever::Temperature, true, _) => "too cold",
                (Lever::Temperature, _, true) => "too warm",
                (Lever::Volcanism, true, _) => "needs volcanoes",
                (Lever::Volcanism, _, true) => "too volcanic",
                _ => continue,
            };
            return Some(hint);
        }
        None
    }
}

/// The named worlds animals belong to. Their ranges rule each other out, so
/// no single planet holds them all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    Primordial,
    Reef,
    IceAge,
    CoalSwamp,
    Jungle,
    Savanna,
    Hothouse,
}

impl Biome {
    pub const ALL: [Biome; 7] = [
        Biome::Primordial,
        Biome::Reef,
        Biome::IceAge,
        Biome::CoalSwamp,
        Biome::Jungle,
        Biome::Savanna,
        Biome::Hothouse,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Biome::Primordial => "Primordial sea",
            Biome::Reef => "Reef sea",
            Biome::IceAge => "Ice age",
            Biome::CoalSwamp => "Coal swamp",
            Biome::Jungle => "Jungle",
            Biome::Savanna => "Savanna",
            Biome::Hothouse => "Hothouse",
        }
    }

    pub fn color(self) -> u32 {
        match self {
            Biome::Primordial => 0x97A3B5,
            Biome::Reef => 0x6FF5E1,
            Biome::IceAge => 0xBFE8F0,
            Biome::CoalSwamp => 0xC5F76A,
            Biome::Jungle => 0x8FC878,
            Biome::Savanna => 0xE0C27A,
            Biome::Hothouse => 0xFF8A5A,
        }
    }

    pub const fn ranges(self) -> Ranges {
        let r = Ranges::ANY;
        match self {
            Biome::Primordial => Ranges {
                oxygen: (0, 2),
                land: (0, 2),
                ..r
            },
            Biome::Reef => Ranges {
                land: (0, 1),
                temperature: (3, 4),
                oxygen: (3, LEVEL_MAX),
                ..r
            },
            Biome::IceAge => Ranges {
                temperature: (0, 1),
                ..r
            },
            Biome::CoalSwamp => Ranges {
                oxygen: (5, 5),
                vegetation: (4, 5),
                temperature: (3, 4),
                ..r
            },
            Biome::Jungle => Ranges {
                vegetation: (5, 5),
                temperature: (4, 4),
                oxygen: (0, 4),
                ..r
            },
            Biome::Savanna => Ranges {
                land: (4, LEVEL_MAX),
                vegetation: (1, 2),
                temperature: (3, LEVEL_MAX),
                ..r
            },
            Biome::Hothouse => Ranges {
                temperature: (5, 5),
                volcanism: (1, VOLCANISM_MAX),
                ..r
            },
        }
    }

    /// The biomes `p` is, in `ALL` order.
    pub fn of(p: &Planet) -> Vec<Biome> {
        Biome::ALL
            .into_iter()
            .filter(|b| b.ranges().contains(p))
            .collect()
    }
}

/// Atmospheric oxygen at an Oxygen lever level.
pub fn oxygen_percent(level: u8) -> u8 {
    [5, 10, 15, 21, 28, 35][level.min(LEVEL_MAX) as usize]
}

/// What a lever's level is called in the lever panel ("Coasts", "21% O2").
pub fn level_label(lever: Lever, level: u8) -> String {
    let i = level.min(LEVEL_MAX) as usize;
    match lever {
        Lever::Land => [
            "Water world",
            "Islands",
            "Coasts",
            "Continents",
            "Dry world",
            "Supercontinent",
        ][i]
            .into(),
        Lever::Vegetation => [
            "Bare rock",
            "Moss",
            "Ferns",
            "Forest",
            "Dense forest",
            "Jungle",
        ][i]
            .into(),
        Lever::Oxygen => format!("{}% O2", oxygen_percent(level)),
        Lever::Temperature => temperature_label(level).into(),
        Lever::Volcanism => ["Calm", "Active", "Violent"][i.min(VOLCANISM_MAX as usize)].into(),
    }
}

pub fn temperature_label(level: u8) -> &'static str {
    ["Snowball", "Cold", "Cool", "Temperate", "Warm", "Hothouse"][level.min(LEVEL_MAX) as usize]
}

/// Wall-clock seconds, so a cycle keeps running while the app is closed.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Cycle {
    pub started_at: f64,
    pub duration: f64,
    /// The planet the genome is rolled against.
    pub launched: Planet,
    /// The wait the genome is paid for (`wait::bonus`), whatever boons and
    /// keystones took off `duration`.
    #[serde(default = "tutorial_hours")]
    pub hours: f32,
    /// Millions of years this cycle adds.
    #[serde(default = "legacy_ma")]
    pub ma: u32,
    /// The keystones equipped when the wait was launched; only these count,
    /// whatever is equipped while time runs.
    #[serde(default)]
    pub keystones: [Option<u16>; MAX_KEYSTONES],
}

/// The most keystone slots there ever are.
pub const MAX_KEYSTONES: usize = 5;

fn tutorial_hours() -> f32 {
    crate::wait::TUTORIAL_HOURS
}

/// What every cycle was worth before the wait could be chosen.
fn legacy_ma() -> u32 {
    20
}

impl Cycle {
    pub fn launched_keystones(&self) -> impl Iterator<Item = usize> + '_ {
        self.keystones.iter().flatten().map(|&k| k as usize)
    }

    pub fn remaining(&self, now: f64) -> f64 {
        (self.started_at + self.duration - now).max(0.0)
    }

    pub fn is_done(&self, now: f64) -> bool {
        self.remaining(now) <= 0.0
    }

    pub fn progress(&self, now: f64) -> f64 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        (1.0 - self.remaining(now) / self.duration).clamp(0.0, 1.0)
    }
}

/// Real seconds for the `n`th cycle: the tutorial ones are short, then the
/// chosen `hours`. `quick` (the keystones' wait cut) is capped at 30%.
pub fn cycle_seconds(n: u32, hours: f32, quick: f32) -> f64 {
    let base = match n {
        0 => 60.0,
        n if n < TUTORIAL_CYCLES => 20.0 * 60.0,
        _ => hours as f64 * 3600.0,
    };
    base * (1.0 - quick.clamp(0.0, 0.3) as f64)
}

pub fn format_remaining(secs: f64) -> String {
    let s = secs.ceil().max(0.0) as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starting_world_is_all_sea() {
        let p = Planet::default();
        assert!(p.has(Habitat::Sea));
        for h in [
            Habitat::Shore,
            Habitat::Fresh,
            Habitat::Land,
            Habitat::Forest,
        ] {
            assert!(!p.has(h), "{h:?} should not exist on a water world");
        }
    }

    #[test]
    fn vegetation_is_capped_by_land_and_drowns_with_it() {
        let mut p = Planet::default();
        assert!(!p.step(Lever::Vegetation, 1), "nothing grows without land");
        p.step(Lever::Land, 1);
        p.step(Lever::Vegetation, 1);
        p.step(Lever::Vegetation, 1);
        assert_eq!(p.vegetation, 2);
        assert!(
            !p.step(Lever::Vegetation, 1),
            "land 1 holds at most vegetation 2"
        );
        p.step(Lever::Land, -1);
        assert_eq!(p.vegetation, 0, "sinking the land drowns the forest");
    }

    #[test]
    fn step_reports_only_real_moves() {
        let mut p = Planet::default();
        assert!(!p.step(Lever::Land, -1));
        assert!(p.step(Lever::Land, 1));
        p.volcanism = VOLCANISM_MAX;
        assert!(!p.step(Lever::Volcanism, 1));
    }

    #[test]
    fn a_supercontinent_has_no_sea() {
        let p = Planet {
            land: 5,
            ..Planet::default()
        };
        assert!(!p.has(Habitat::Sea));
        assert!(!p.has(Habitat::Reef));
        assert!(!p.has(Habitat::Shore));
    }

    #[test]
    fn linked_levers_apply_after_a_cycle() {
        let forest = Planet {
            land: 3,
            vegetation: 4,
            oxygen: 3,
            ..Planet::default()
        };
        assert_eq!(forest.after_cycle(0).oxygen, 4, "forests breathe");

        let vents = Planet {
            volcanism: 2,
            temperature: 3,
            ..Planet::default()
        };
        assert_eq!(vents.after_cycle(0).temperature, 4, "volcanic greenhouse");

        let ice = Planet {
            temperature: 1,
            land: 1,
            ..Planet::default()
        };
        assert_eq!(ice.after_cycle(0).land, 2, "ice locks up the sea");
    }

    #[test]
    fn soil_keystones_grow_vegetation_within_the_land_cap() {
        let p = Planet {
            land: 1,
            vegetation: 1,
            ..Planet::default()
        };
        assert_eq!(p.after_cycle(3).vegetation, 2);
    }

    #[test]
    fn tutorial_cycles_are_short_then_the_chosen_wait_applies() {
        assert_eq!(cycle_seconds(0, 6.0, 0.0), 60.0);
        assert_eq!(cycle_seconds(1, 6.0, 0.0), 1200.0);
        assert_eq!(cycle_seconds(7, 6.0, 0.0), 21600.0);
        assert_eq!(cycle_seconds(7, 2.5, 0.0), 9000.0);
        assert!(
            (cycle_seconds(7, 6.0, 0.9) - 21600.0 * 0.7).abs() < 0.01,
            "quick is capped at 30%"
        );
    }

    #[test]
    fn a_cycle_counts_down_on_the_wall_clock() {
        let c = Cycle {
            started_at: 100.0,
            duration: 50.0,
            launched: Planet::default(),
            hours: 3.0,
            ma: 30,
            keystones: Default::default(),
        };
        assert_eq!(c.remaining(120.0), 30.0);
        assert!(!c.is_done(149.0));
        assert!(c.is_done(151.0));
        assert!((c.progress(125.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn countdown_formats() {
        assert_eq!(format_remaining(59.2), "01:00");
        assert_eq!(format_remaining(3725.0), "1:02:05");
    }
}
