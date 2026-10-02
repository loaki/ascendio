//! The online leaderboard (`server/`): this player's score goes up whenever
//! it changes, and the board comes down when its screen opens. Ranked by RAD,
//! then by species found on any Earth.

use serde::{Deserialize, Serialize};

use crate::genome::Rng;
use crate::net::{self, Failure, Request};

/// The deployed Worker. `ASCENDIO_LEADERBOARD_URL` at build time overrides
/// it (`http://127.0.0.1:8787` for `wrangler dev`).
const DEFAULT_URL: Option<&str> = Some("https://ascendio-scores.loaki-dev.workers.dev");

fn base_url() -> Option<&'static str> {
    option_env!("ASCENDIO_LEADERBOARD_URL").or(DEFAULT_URL)
}

/// Seconds before a failed submit is tried again.
const RETRY_SECS: f64 = 60.0;
pub const NAME_MIN: usize = 3;
pub const NAME_MAX: usize = 16;

/// Randomness for a new player ID: the clock, the frame timer's fine
/// digits and, where there is one, the OS's own randomness (std's hash
/// keys), so two first launches in the same millisecond still differ.
pub fn id_seed() -> u64 {
    let mut seed = (macroquad::miniquad::date::now() * 1000.0) as u64;
    seed ^= (macroquad::time::get_time() * 1e9) as u64 ^ 0xA5C3_17E5_9B0D_2F41;
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(seed);
        seed ^= h.finish();
    }
    seed
}

pub fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Characters the server and the pixel font both take.
pub fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-')
}

/// `raw` as a leaderboard name, or `None` if nothing usable is left.
pub fn clean_name(raw: &str) -> Option<String> {
    let kept: String = raw
        .chars()
        .filter(|&c| name_char(c))
        .take(NAME_MAX)
        .collect();
    let name = kept.trim().to_string();
    (name.len() >= NAME_MIN).then_some(name)
}

const ADJECTIVES: [&str; 20] = [
    "Swift", "Brave", "Tiny", "Mighty", "Ancient", "Silent", "Lucky", "Wild", "Bold", "Clever",
    "Sly", "Shy", "Hardy", "Primal", "Deep", "Feral", "Spiny", "Fuzzy", "Sleek", "Grand",
];
const ANIMALS: [&str; 20] = [
    "Squid", "Shark", "Raptor", "Coral", "Frog", "Gecko", "Otter", "Crab", "Wasp", "Owl", "Lemur",
    "Seal", "Snail", "Moth", "Eel", "Newt", "Bison", "Tapir", "Viper", "Dodo",
];

/// "SwiftSquid": random until the player picks a name. Names may repeat;
/// players are told apart by their ID.
pub fn random_name(rng: &mut Rng) -> String {
    let pick = |rng: &mut Rng, n: usize| (rng.next_u64() % n as u64) as usize;
    let adjective = ADJECTIVES[pick(rng, ADJECTIVES.len())];
    let animal = ANIMALS[pick(rng, ANIMALS.len())];
    format!("{adjective}{animal}")
}

/// What this player submits.
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct Score {
    pub name: String,
    pub rad: u32,
    pub species: u32,
    /// A taxon index: the last animal found.
    pub animal: usize,
}

#[derive(Serialize)]
struct Submit<'a> {
    id: &'a str,
    #[serde(flatten)]
    score: &'a Score,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    pub rank: u32,
    pub name: String,
    pub rad: u32,
    pub species: u32,
    pub animal: usize,
    /// This player.
    #[serde(default)]
    pub me: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Board {
    pub top: Vec<Entry>,
    /// This player, when outside the top.
    pub me: Option<Entry>,
    pub players: u32,
}

pub enum View {
    /// No server in this build.
    Unavailable,
    Loading,
    Ready(Board),
    Failed,
}

pub struct Leaderboard {
    /// Off for dev runs that don't play the real save.
    enabled: bool,
    /// The last score the server answered for (accepted, or refused for
    /// good): a new one goes up only when the score changes.
    sent: Option<Score>,
    submitting: Option<(Score, Request)>,
    retry_at: f64,
    fetching: Option<Request>,
    pub view: View,
    pub scroll: f32,
}

impl Leaderboard {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            sent: None,
            submitting: None,
            retry_at: 0.0,
            fetching: None,
            view: View::Unavailable,
            scroll: 0.0,
        }
    }

    /// Each frame: sends `score` when it differs from what the server has,
    /// and collects finished requests. `now` is wall-clock seconds.
    pub fn update(&mut self, id: &str, score: &Score, now: f64) {
        let done = self
            .submitting
            .as_mut()
            .and_then(|(_, req)| req.poll())
            .zip(self.submitting.as_ref().map(|(s, _)| s.clone()));
        if let Some((reply, sent)) = done {
            self.submitting = None;
            match reply {
                Ok(_) => {
                    self.sent = Some(sent);
                    // A board already loaded now has a stale row for us.
                    if !matches!(self.view, View::Unavailable) {
                        self.refresh(id);
                    }
                }
                // Refused: the same score would be refused again, so it waits
                // until the score changes.
                Err(Failure::Rejected) => self.sent = Some(sent),
                Err(Failure::Retry) => self.retry_at = now + RETRY_SECS,
            }
        }
        if let Some(req) = &mut self.fetching {
            if let Some(reply) = req.poll() {
                self.view = reply
                    .ok()
                    .and_then(|body| serde_json::from_str(&body).ok())
                    .map_or(View::Failed, View::Ready);
                self.fetching = None;
            }
        }
        let due = self.sent.as_ref() != Some(score) && now >= self.retry_at;
        if self.enabled && due && self.submitting.is_none() {
            if let Some(url) = base_url() {
                let body = serde_json::to_string(&Submit { id, score }).unwrap_or_default();
                let req = net::request("POST", &format!("{url}/score"), Some(&body));
                self.submitting = Some((score.clone(), req));
            }
        }
    }

    /// Loads the board, for its screen.
    pub fn refresh(&mut self, id: &str) {
        let Some(url) = base_url() else {
            self.view = View::Unavailable;
            return;
        };
        if self.fetching.is_none() {
            self.fetching = Some(net::request("GET", &format!("{url}/top?id={id}"), None));
        }
        // Keep showing the old board while the new one loads.
        if !matches!(self.view, View::Ready(_)) {
            self.view = View::Loading;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_keep_only_what_the_server_takes() {
        assert_eq!(clean_name("  Bold Otter  ").as_deref(), Some("Bold Otter"));
        assert_eq!(clean_name("<b>Rex</b>!").as_deref(), Some("bRexb"));
        assert_eq!(clean_name("ab"), None);
        assert_eq!(clean_name("é漢字"), None);
        assert_eq!(clean_name(&"x".repeat(40)).map(|n| n.len()), Some(NAME_MAX));
    }

    #[test]
    fn random_names_are_valid() {
        let mut rng = Rng::new(7);
        for _ in 0..500 {
            let name = random_name(&mut rng);
            assert_eq!(clean_name(&name).as_deref(), Some(name.as_str()), "{name}");
        }
    }

    #[test]
    fn a_submit_carries_the_id_and_the_score() {
        let score = Score {
            name: "SwiftSquid42".into(),
            rad: 3,
            species: 121,
            animal: 148,
        };
        let v: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&Submit {
                id: "ab",
                score: &score,
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(v["id"], "ab");
        assert_eq!(v["rad"], 3);
        assert_eq!(v["species"], 121);
        assert_eq!(v["animal"], 148);
        assert_eq!(v["name"], "SwiftSquid42");
    }
}
