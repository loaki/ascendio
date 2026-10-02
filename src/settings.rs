//! The player's settings, from the gear in the top bar.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub struct Settings {
    /// Post a notification when a new genome is ready. Off until asked for.
    #[serde(default)]
    pub notify: bool,
}

impl Settings {
    pub fn load() -> Self {
        crate::save::read_settings()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        crate::save::write_settings(&serde_json::to_string(self).unwrap_or_default());
    }
}
