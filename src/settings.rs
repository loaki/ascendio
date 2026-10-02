//! The player's settings, from the gear in the top bar.

use serde::{Deserialize, Serialize};

/// Screen brightness, in percent: below 100 dims, above brightens.
pub const BRIGHTNESS_MIN: u8 = 50;
pub const BRIGHTNESS_MAX: u8 = 150;
/// Five levels: 50, 75, 100, 125, 150.
pub const BRIGHTNESS_STEP: u8 = 25;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Settings {
    /// Post a notification when a new genome is ready. Off until asked for.
    #[serde(default)]
    pub notify: bool,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
}

fn default_brightness() -> u8 {
    100
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notify: false,
            brightness: default_brightness(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let mut s: Self = crate::save::read_settings()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        // Snapped to a level, should the levels ever change.
        let b = s.brightness.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX) - BRIGHTNESS_MIN;
        s.brightness =
            BRIGHTNESS_MIN + (b + BRIGHTNESS_STEP / 2) / BRIGHTNESS_STEP * BRIGHTNESS_STEP;
        s
    }

    pub fn save(&self) {
        crate::save::write_settings(&serde_json::to_string(self).unwrap_or_default());
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
