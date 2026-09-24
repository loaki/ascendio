//! The planet: the five levers, the habitats they imply, and the real-time
//! cycle that turns hours into millions of years.

use serde::{Deserialize, Serialize};

/// Highest level of every lever but `Volcanism`.
pub const LEVEL_MAX: u8 = 5;
pub const VOLCANISM_MAX: u8 = 2;
/// Points to spend per cycle before keystones.
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
}

/// Atmospheric oxygen at an Oxygen lever level.
pub fn oxygen_percent(level: u8) -> u8 {
    [5, 10, 15, 21, 28, 35][level.min(LEVEL_MAX) as usize]
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
}

fn tutorial_hours() -> f32 {
    crate::wait::TUTORIAL_HOURS
}

/// What every cycle was worth before the wait could be chosen.
fn legacy_ma() -> u32 {
    20
}

impl Cycle {
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
