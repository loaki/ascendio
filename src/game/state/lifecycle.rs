//! Save loading and new-game setup.
use super::*;

impl Game {
    pub fn new(now: f64) -> Self {
        let phy = Phylogeny::load();
        let n = phy.len();
        let mut unlocked = vec![false; n];
        unlocked[Phylogeny::ROOT] = true;
        let mut specimens = vec![0; n];
        specimens[Phylogeny::ROOT] = 1;
        let mut found_ma = vec![None; n];
        found_ma[Phylogeny::ROOT] = Some(0);
        let mut game = Self {
            phy,
            unlocked,
            level: vec![0; n],
            specimens,
            morphs: vec![0; n],
            found_ma,
            planet: Planet::default(),
            shaped_from: Planet::default(),
            cycle: None,
            cycles_done: 0,
            ma_done: 0,
            wait_hours: wait::DEFAULT_HOURS,
            genome: None,
            rng: Rng::new((now * 1000.0) as u64 ^ 0x9E37_79B9_7F4A_7C15),
            keystones: Vec::new(),
            boon: None,
            boon_offer: None,
            growth: vec![0; n],
            gone: vec![false; n],
            edition: vec![0; n],
            last_launched: None,
            wait_points: planet::BASE_POINTS,
            bonus_points: 0,
            point_carry: 0.0,
            last_hours: wait::DEFAULT_HOURS,
            rad: 0,
            fossil: vec![false; n],
            doomed: false,
            doom_risk: 0.0,
            last_found: None,
            next_boons: None,
        };
        game.fossil[Phylogeny::ROOT] = true;
        game.refresh_levels();
        game
    }

    pub fn save(&self) {
        crate::platform::save::write(&self.to_json());
    }

    pub(super) fn to_json(&self) -> String {
        let data = Versioned {
            version: SAVE_VERSION,
            game: self,
        };
        serde_json::to_string(&data).unwrap_or_default()
    }

    pub fn load(now: f64) -> Option<Self> {
        Self::from_json(&crate::platform::save::read()?, now)
    }

    pub(super) fn from_json(json: &str, now: f64) -> Option<Self> {
        let header: SaveHeader = serde_json::from_str(json).ok()?;
        let mut value: serde_json::Value = serde_json::from_str(json).ok()?;
        retire_lure(&mut value);
        let mut game: Self = serde_json::from_value(value).ok()?;
        let n = game.phy.len();
        let saved = game.unlocked.len();
        if header.version != SAVE_VERSION
            || saved > n
            || game.specimens.len() != saved
            || game.morphs.len() != saved
            || game.found_ma.len() != saved
            || game.keystones.iter().any(|&p| p >= saved)
            || game.last_found.is_some_and(|t| t >= saved)
            || game
                .cycle
                .is_some_and(|c| c.launched_keystones().any(|k| k >= saved))
            || game.genome.iter().flatten().any(|c| c.taxon >= saved)
        {
            return None;
        }
        // Taxa added since the save was written are appended to the tree.
        game.unlocked.resize(n, false);
        game.specimens.resize(n, 0);
        game.morphs.resize(n, 0);
        game.found_ma.resize(n, None);
        game.growth.resize(n, 0);
        game.gone.resize(n, false);
        game.edition.resize(n, 0);
        game.fossil.resize(n, false);
        for (f, &u) in game.fossil.iter_mut().zip(&game.unlocked) {
            *f |= u;
        }
        game.level = vec![0; n];
        game.ma_done = header.ma_done.unwrap_or(game.cycles_done * 20);
        game.wait_hours = wait::snap(game.wait_hours);
        game.refresh_levels();
        // Keystone points used to count at once: this shaping keeps them.
        if header.bonus_points.is_none() {
            game.bonus_points = game.effects().points.min(MAX_POINTS.into()) as u8;
        }
        game.tick(now);
        Some(game)
    }
}
