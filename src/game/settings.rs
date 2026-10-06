//! The player's settings, from the gear in the top bar.

use serde::{Deserialize, Serialize};

use crate::game::names::Language;

/// Screen brightness, in percent: below 100 dims, above brightens.
pub const BRIGHTNESS_MIN: u8 = 50;
pub const BRIGHTNESS_MAX: u8 = 150;
/// Five levels: 50, 75, 100, 125, 150.
pub const BRIGHTNESS_STEP: u8 = 25;

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub notify: bool,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    #[serde(default)]
    pub player_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub language: Language,
    /// Set by RESET until the leaderboard has dropped the old score.
    #[serde(default)]
    pub fresh_start: bool,
}

fn default_brightness() -> u8 {
    100
}

/// `"field":"value"` in text that isn't JSON any more; `None` if absent or
/// cut off (the caller checks the value: a broken one is remade).
fn find_string(text: &str, field: &str) -> Option<String> {
    let after = &text[text.find(&format!("\"{field}\""))? + field.len() + 2..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let value = after.strip_prefix('"')?;
    Some(value[..value.find('"')?].to_string())
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notify: false,
            brightness: default_brightness(),
            player_id: String::new(),
            name: String::new(),
            language: Language::default(),
            fresh_start: false,
        }
    }
}

impl Settings {
    pub fn load(persist: bool) -> Self {
        let Some(json) = crate::platform::save::read_settings() else {
            return Self::default();
        };
        let (s, clean) = Self::parse(&json);
        if !clean && persist {
            crate::platform::save::keep_bad_settings(&json);
        }
        s
    }

    #[cfg(test)]
    fn from_json(json: &str) -> Self {
        Self::parse(json).0
    }

    fn parse(json: &str) -> (Self, bool) {
        let (mut s, clean) = match serde_json::from_str(json) {
            Ok(s) => (s, true),
            Err(_) => (Self::salvage(json), json.trim().is_empty()),
        };
        // Snapped to a level, should the levels ever change.
        let b = s.brightness.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX) - BRIGHTNESS_MIN;
        s.brightness =
            BRIGHTNESS_MIN + (b + BRIGHTNESS_STEP / 2) / BRIGHTNESS_STEP * BRIGHTNESS_STEP;
        (s, clean)
    }

    fn salvage(json: &str) -> Self {
        let mut s = Self::default();
        let Ok(serde_json::Value::Object(map)) = serde_json::from_str(json) else {
            s.player_id = find_string(json, "player_id").unwrap_or_default();
            s.name = find_string(json, "name").unwrap_or_default();
            return s;
        };
        if let Some(b) = map.get("notify").and_then(|v| v.as_bool()) {
            s.notify = b;
        }
        if let Some(b) = map.get("brightness").and_then(|v| v.as_f64()) {
            s.brightness = b.clamp(0.0, 255.0) as u8;
        }
        if let Some(id) = map.get("player_id").and_then(|v| v.as_str()) {
            s.player_id = id.to_string();
        }
        if let Some(name) = map.get("name").and_then(|v| v.as_str()) {
            s.name = name.to_string();
        }
        if let Some(lang) = map
            .get("language")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
        {
            s.language = lang;
        }
        s
    }

    pub fn save(&self) {
        crate::platform::save::write_settings(&serde_json::to_string(self).unwrap_or_default());
    }

    pub fn ensure_player(&mut self, seed: u64) -> bool {
        let mut rng = crate::game::genome::Rng::new(seed);
        let mut changed = false;
        if !crate::game::leaderboard::valid_id(&self.player_id) {
            self.player_id = format!("{:016x}{:016x}", rng.next_u64(), rng.next_u64());
            changed = true;
        }
        if crate::game::leaderboard::clean_name(&self.name).is_none() {
            self.name = crate::game::leaderboard::random_name(&mut rng);
            changed = true;
        }
        changed
    }

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
            assert!(crate::game::leaderboard::valid_id(&s.player_id), "{json:?}");
            assert!(
                crate::game::leaderboard::clean_name(&s.name).is_some(),
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
    fn broken_settings_keep_the_player() {
        let id = "0123456789abcdef0123456789abcdef";
        // A field of the wrong type, then a file cut off mid-write.
        for json in [
            format!(
                "{{\"notify\":\"yes\",\"brightness\":125,\"player_id\":\"{id}\",\"name\":\"Rex\"}}"
            ),
            format!("{{\"notify\":true,\"player_id\": \"{id}\",\"name\":\"Rex\",\"bri"),
        ] {
            let (mut s, clean) = Settings::parse(&json);
            assert!(!clean, "{json}");
            assert!(!s.ensure_player(1), "{json}: nothing to remake");
            assert_eq!(s.player_id, id);
            assert_eq!(s.name, "Rex");
        }
        assert_eq!(
            Settings::parse("{\"brightness\":\"x\",\"notify\":true}")
                .0
                .brightness,
            100
        );
        assert!(Settings::parse("").1, "nothing stored is nothing broken");
        assert!(Settings::from_json("{\"player_id\":\"ab")
            .player_id
            .is_empty());
    }

    #[test]
    fn a_broken_identity_is_remade() {
        let mut s = Settings::from_json("{\"player_id\":\"nope\",\"name\":\"x\"}");
        assert!(s.ensure_player(3));
        assert!(crate::game::leaderboard::valid_id(&s.player_id));
        assert!(crate::game::leaderboard::clean_name(&s.name).is_some());
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
