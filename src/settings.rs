//! The player's settings, from the gear in the top bar.

use serde::{Deserialize, Serialize};

/// Screen brightness, in percent: below 100 dims, above brightens.
pub const BRIGHTNESS_MIN: u8 = 50;
pub const BRIGHTNESS_MAX: u8 = 150;
/// Five levels: 50, 75, 100, 125, 150.
pub const BRIGHTNESS_STEP: u8 = 25;

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    /// Post a notification when a new genome is ready. Off until asked for.
    #[serde(default)]
    pub notify: bool,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    /// The leaderboard's key for this player: random, made on first launch.
    #[serde(default)]
    pub player_id: String,
    /// The name on the leaderboard: random until the player picks one.
    #[serde(default)]
    pub name: String,
}

fn default_brightness() -> u8 {
    100
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notify: false,
            brightness: default_brightness(),
            player_id: String::new(),
            name: String::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        crate::save::read_settings().map_or_else(Self::default, |json| Self::from_json(&json))
    }

    /// Settings as any build wrote them; what an older one didn't know
    /// takes its default, and anything unreadable starts over.
    fn from_json(json: &str) -> Self {
        let mut s: Self = serde_json::from_str(json).unwrap_or_default();
        // Snapped to a level, should the levels ever change.
        let b = s.brightness.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX) - BRIGHTNESS_MIN;
        s.brightness =
            BRIGHTNESS_MIN + (b + BRIGHTNESS_STEP / 2) / BRIGHTNESS_STEP * BRIGHTNESS_STEP;
        s
    }

    pub fn save(&self) {
        crate::save::write_settings(&serde_json::to_string(self).unwrap_or_default());
    }

    /// Makes the leaderboard identity on first launch. Returns whether it
    /// had to (the settings then need saving).
    pub fn ensure_player(&mut self, seed: u64) -> bool {
        let mut rng = crate::genome::Rng::new(seed);
        let mut changed = false;
        if !crate::leaderboard::valid_id(&self.player_id) {
            self.player_id = format!("{:016x}{:016x}", rng.next_u64(), rng.next_u64());
            changed = true;
        }
        if crate::leaderboard::clean_name(&self.name).is_none() {
            self.name = crate::leaderboard::random_name(&mut rng);
            changed = true;
        }
        changed
    }

    /// One step brighter (`up`) or dimmer. Returns whether it changed.
    pub fn step_brightness(&mut self, up: bool) -> bool {
        let next = if up {
            self.brightness.saturating_add(BRIGHTNESS_STEP)
        } else {
            self.brightness.saturating_sub(BRIGHTNESS_STEP)
        }
        .clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX);
        let changed = next != self.brightness;
        self.brightness = next;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What each release wrote: nothing before the gear existed, then the
    /// notification alone, then the brightness too.
    const OLDER: [&str; 4] = [
        "",
        "{\"notify\":true}",
        "{\"notify\":false,\"brightness\":125}",
        // The brightness had 10% steps for a while.
        "{\"notify\":true,\"brightness\":110}",
    ];

    #[test]
    fn settings_from_older_releases_load_and_get_a_player() {
        for json in OLDER {
            let mut s = Settings::from_json(json);
            assert!(s.ensure_player(42), "{json:?}: a first launch makes one");
            assert!(crate::leaderboard::valid_id(&s.player_id), "{json:?}");
            assert!(
                crate::leaderboard::clean_name(&s.name).is_some(),
                "{json:?}"
            );
            assert_eq!(
                s.brightness % BRIGHTNESS_STEP,
                0,
                "{json:?}: snapped to a level"
            );
        }
        assert!(
            Settings::from_json(OLDER[1]).notify,
            "kept across the update"
        );
        assert_eq!(Settings::from_json(OLDER[2]).brightness, 125);
        assert_eq!(Settings::from_json(OLDER[3]).brightness, 100);
    }

    #[test]
    fn the_player_survives_a_relaunch() {
        let mut s = Settings::from_json("{\"notify\":true}");
        s.ensure_player(7);
        let mut again = Settings::from_json(&serde_json::to_string(&s).unwrap());
        assert!(!again.ensure_player(99), "nothing to make the second time");
        assert_eq!(again.player_id, s.player_id);
        assert_eq!(again.name, s.name);
    }

    #[test]
    fn a_broken_identity_is_remade() {
        let mut s = Settings::from_json("{\"player_id\":\"nope\",\"name\":\"x\"}");
        assert!(s.ensure_player(3));
        assert!(crate::leaderboard::valid_id(&s.player_id));
        assert!(crate::leaderboard::clean_name(&s.name).is_some());
    }

    #[test]
    fn different_seeds_make_different_players() {
        // From 1: the RNG takes a seed of 0 as 1.
        let ids: std::collections::HashSet<String> = (1..=200u64)
            .map(|seed| {
                let mut s = Settings::default();
                s.ensure_player(seed);
                s.player_id
            })
            .collect();
        assert_eq!(ids.len(), 200);
    }
}
