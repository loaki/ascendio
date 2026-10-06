//! Online leaderboard client: score submit, board fetch, player ID and name.

use serde::{Deserialize, Serialize};

use crate::game::genome::Rng;
use crate::platform::net::{self, Failure, Request};

/// The deployed Worker. `ASCENDIO_LEADERBOARD_URL` at build time overrides
/// it (`http://127.0.0.1:8787` for `wrangler dev`).
const DEFAULT_URL: Option<&str> = Some("https://ascendio-scores.loaki-dev.workers.dev");

fn base_url() -> Option<&'static str> {
    option_env!("ASCENDIO_LEADERBOARD_URL").or(DEFAULT_URL)
}

/// Seconds before a failed submit is tried again.
const RETRY_SECS: f64 = 60.0;
/// When the server cut a score down to its per-hour rate: the wait before
/// sending it again, so a long-time player keeps climbing.
const CATCH_UP_SECS: f64 = 1800.0;
pub const NAME_MIN: usize = 3;
pub const NAME_MAX: usize = 16;

/// Randomness for a new player ID: the clock, the frame timer's fine
/// digits and, where there is one, the OS's own randomness (std's hash
/// keys), so two first launches in the same millisecond still differ.
/// The web has no OS randomness here: there it also mixes in where the
/// allocator put a fresh block and how long a little busy work took, which
/// varies with the page, the browser and the machine.
pub fn id_seed() -> u64 {
    let mut seed = mix(
        0xA5C3_17E5_9B0D_2F41,
        macroquad::miniquad::date::now().to_bits(),
    );
    seed = mix(seed, macroquad::time::get_time().to_bits());
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(seed);
        seed ^= h.finish();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let block = Box::new(seed);
        seed = mix(seed, &*block as *const u64 as u64);
        // Spin until the clock ticks, twice: how often it got round
        // between ticks is noise from the machine.
        for _ in 0..2 {
            let start = macroquad::miniquad::date::now();
            let mut spins = 0u64;
            while macroquad::miniquad::date::now() == start && spins < 1 << 18 {
                spins += 1;
            }
            seed = mix(seed, spins);
        }
    }
    seed
}

/// splitmix64's finalizer over `seed ^ v`: every bit of `v` reaches all 64.
fn mix(seed: u64, v: u64) -> u64 {
    let mut z = (seed ^ v).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-')
}

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

#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct Score {
    pub name: String,
    pub rad: u32,
    pub species: u32,
    pub animal: usize,
    /// The player started over: the server drops their old row.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reset: bool,
}

#[derive(Deserialize, Default)]
struct Stored {
    rad: Option<u32>,
    species: Option<u32>,
}

impl Stored {
    /// Whether the server kept less than was sent.
    fn cut(&self, sent: &Score) -> bool {
        self.rad.is_some_and(|r| r < sent.rad) || self.species.is_some_and(|n| n < sent.species)
    }
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
    enabled: bool,
    /// The last score the server answered for (accepted, or refused for
    /// good): a new one goes up only when the score changes.
    sent: Option<Score>,
    submitting: Option<(Score, Request)>,
    retry_at: f64,
    /// How long `retry_at` was set ahead, to spot a clock set back.
    retry_wait: f64,
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
            retry_wait: 0.0,
            fetching: None,
            view: View::Unavailable,
            scroll: 0.0,
        }
    }

    /// Each frame: sends `score` when it differs from what the server has,
    /// and collects finished requests. `now` is wall-clock seconds; a clock
    /// set back never holds a retry up for longer than it was meant to wait.
    /// True once the server has accepted a score with `reset` set.
    pub fn update(&mut self, id: &str, score: &Score, now: f64) -> bool {
        let mut reset_done = false;
        let done = self
            .submitting
            .as_mut()
            .and_then(|(_, req)| req.poll())
            .zip(self.submitting.as_ref().map(|(s, _)| s.clone()));
        if let Some((reply, sent)) = done {
            self.submitting = None;
            match reply {
                Ok(body) => {
                    let stored: Stored = serde_json::from_str(&body).unwrap_or_default();
                    if stored.cut(&sent) {
                        // Cut to the per-hour rate: the rest goes up later.
                        self.sent = None;
                        self.retry_at = now + CATCH_UP_SECS;
                        self.retry_wait = CATCH_UP_SECS;
                    } else {
                        reset_done = sent.reset;
                        self.sent = Some(Score {
                            reset: false,
                            ..sent
                        });
                    }
                    // A board already loaded now has a stale row for us.
                    if !matches!(self.view, View::Unavailable) {
                        self.refresh(id);
                    }
                }
                // Refused: the same score would be refused again, so it waits
                // until the score changes.
                Err(Failure::Rejected) => self.sent = Some(sent),
                Err(Failure::Retry) => {
                    self.retry_at = now + RETRY_SECS;
                    self.retry_wait = RETRY_SECS;
                }
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
        if self.retry_at - now > self.retry_wait {
            self.retry_at = now;
        }
        let due = self.sent.as_ref() != Some(score) && now >= self.retry_at;
        if self.enabled && due && self.submitting.is_none() {
            if let Some(url) = base_url() {
                let body = serde_json::to_string(&Submit { id, score }).unwrap_or_default();
                let req = net::request("POST", &format!("{url}/score"), Some(&body));
                self.submitting = Some((score.clone(), req));
            }
        }
        reset_done
    }

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
            reset: false,
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

    #[test]
    fn a_score_cut_by_the_server_is_sent_again() {
        let sent = Score {
            name: "SwiftSquid42".into(),
            rad: 2,
            species: 121,
            animal: 0,
            reset: false,
        };
        let stored = |body: &str| serde_json::from_str::<Stored>(body).unwrap();
        assert!(stored(r#"{"rank":3,"rad":2,"species":40}"#).cut(&sent));
        assert!(!stored(r#"{"rank":3,"rad":2,"species":121}"#).cut(&sent));
        // An older server only answers the rank.
        assert!(!stored(r#"{"rank":3}"#).cut(&sent));
    }
}
