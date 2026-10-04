//! Ascendio -- a daily game about the animal tree of life: shape the planet,
//! let time run, evolve the genome it produced. See `docs/DESIGN.md`.

mod backdrop;
mod clock;
mod collapse;
mod dial;
mod ecology;
mod game;
mod genome;
mod genome_bg;
mod layout;
mod leaderboard;
mod net;
mod notify;
mod opening;
mod pixel;
mod planet;
mod render;
mod save;
mod settings;
mod spiral;
mod sprites;
mod tree;
mod ui;
mod view;
mod wait;

use macroquad::prelude::*;

use backdrop::Backdrop;
use collapse::Collapse;
use game::{Game, Phase};
use layout::Layout;
use leaderboard::{Leaderboard, Score};
use opening::Opening;
use planet::Lever;
use settings::Settings;
use spiral::{Frame, Nav};
use sprites::Sprites;
use ui::{Assets, KeystoneView, Menu, SettingsView};
use view::{Camera, Gesture, Input};

/// How hard you have to pinch closed before the map opens.
const PINCH_TO_MAP: f32 = 0.82;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Spiral,
    Map,
    Keystones,
    Settings,
    Leaderboard,
}

impl Mode {
    /// The full-screen pages, opened from the top bar.
    fn is_page(self) -> bool {
        matches!(self, Mode::Settings | Mode::Leaderboard)
    }
}

/// Seconds between autosaves.
const SAVE_INTERVAL: f32 = 3.0;

fn env_flag(key: &str) -> bool {
    std::env::var(key).is_ok_and(|v| v != "0")
}

fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

/// `ASCENDIO_SHOT_MODE`: what a capture shows.
#[derive(PartialEq, Clone, Copy)]
enum ShotKind {
    Map,
    Spiral,
    /// The spiral with the wait dial up.
    Dial,
    Keystones,
    Settings,
    Leaderboard,
    /// The map with the biome guide open.
    Biomes,
    /// The end of the Earth from the start (`ASCENDIO_SHOT_AFTER` picks the
    /// moment).
    Collapse,
    /// The planet alone, no spiral or UI.
    Backdrop,
}

impl ShotKind {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "map" => Self::Map,
            "spiral" => Self::Spiral,
            "dial" => Self::Dial,
            "keystones" => Self::Keystones,
            "settings" => Self::Settings,
            "leaderboard" => Self::Leaderboard,
            "biomes" => Self::Biomes,
            "collapse" => Self::Collapse,
            "backdrop" => Self::Backdrop,
            _ => return None,
        })
    }

    /// The screen it forces, if any.
    fn mode(self) -> Option<Mode> {
        match self {
            Self::Map | Self::Biomes => Some(Mode::Map),
            Self::Spiral | Self::Dial | Self::Backdrop => Some(Mode::Spiral),
            Self::Keystones => Some(Mode::Keystones),
            Self::Settings => Some(Mode::Settings),
            Self::Leaderboard => Some(Mode::Leaderboard),
            Self::Collapse => None,
        }
    }
}

/// A framebuffer capture: write `path` after `after` seconds, then exit.
struct Shot {
    path: String,
    after: f32,
    kind: Option<ShotKind>,
    /// `ASCENDIO_SHOT_EVERY=0.125`: also write every frame (`out_000.png`,
    /// ...), each this many seconds of game time, for animations.
    every: Option<f32>,
}

/// Dev aids, all read from `ASCENDIO_*` environment variables (see the
/// README). None of them exist on the web build, so it always plays for real.
struct Dev {
    /// `ASCENDIO_TIME_SCALE=3600` makes an hour pass every second.
    time_scale: f64,
    /// `ASCENDIO_AUTOPLAY`: shape, run and open cycles by itself.
    autoplay: bool,
    /// `ASCENDIO_DEV`: T ends the running cycle, R resets the game.
    keys: bool,
    /// Off for autoplay and `ASCENDIO_SCRATCH`: never touch the real save.
    persist: bool,
    /// `ASCENDIO_DEMO_TAPS=n`: start on a ready genome and tap it `n` times.
    demo_taps: Option<u32>,
    /// `ASCENDIO_SHOT=out.png`, `ASCENDIO_SHOT_AFTER` (default 5) and
    /// `ASCENDIO_SHOT_MODE=map|spiral|dial|keystones|settings|leaderboard|biomes|collapse|backdrop`.
    shot: Option<Shot>,
}

impl Dev {
    fn from_env() -> Self {
        let autoplay = env_flag("ASCENDIO_AUTOPLAY");
        let shot = std::env::var("ASCENDIO_SHOT").ok().map(|path| Shot {
            path,
            after: env_parse("ASCENDIO_SHOT_AFTER").unwrap_or(5.0),
            kind: std::env::var("ASCENDIO_SHOT_MODE")
                .ok()
                .and_then(|m| ShotKind::parse(&m)),
            every: env_parse("ASCENDIO_SHOT_EVERY").filter(|&v: &f32| v > 0.0),
        });
        Self {
            time_scale: env_parse("ASCENDIO_TIME_SCALE")
                .filter(|&v: &f64| v > 0.0)
                .unwrap_or(1.0),
            autoplay,
            keys: autoplay || env_flag("ASCENDIO_DEV"),
            persist: !autoplay && !env_flag("ASCENDIO_SCRATCH"),
            demo_taps: env_parse("ASCENDIO_DEMO_TAPS"),
            shot,
        }
    }

    /// Whether the capture asked for is `kind`.
    fn shot_is(&self, kind: ShotKind) -> bool {
        self.shot.as_ref().is_some_and(|s| s.kind == Some(kind))
    }

    /// `ASCENDIO_PLANET=land,veg,o2,temp,volc` forces the planet.
    fn planet_override() -> Option<planet::Planet> {
        let spec = std::env::var("ASCENDIO_PLANET").ok()?;
        let v: Vec<u8> = spec
            .split(',')
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        let [land, vegetation, oxygen, temperature, volcanism] = v[..] else {
            return None;
        };
        Some(planet::Planet {
            land,
            vegetation,
            oxygen,
            temperature,
            volcanism,
        })
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Ascendio".to_owned(),
        window_width: env_parse("ASCENDIO_W").unwrap_or(480),
        window_height: env_parse("ASCENDIO_H").unwrap_or(960),
        window_resizable: false,
        high_dpi: true,
        // miniquad defaults to fullscreen on Android, which hides the status bar.
        fullscreen: false,
        ..Default::default()
    }
}

/// The deepest taxon just discovered: where the spiral glides to.
fn deepest(game: &Game, opened: &[game::Opened]) -> Option<usize> {
    opened
        .iter()
        .filter(|o| o.card.new)
        .map(|o| o.card.taxon)
        .max_by_key(|&t| game.taxon(t).depth)
}

fn map_layout(game: &Game) -> Layout {
    layout::compute(game, &|s| render::text_width(s, render::LABEL_PX))
}

/// Everything the game loop keeps between frames.
struct App {
    dev: Dev,
    /// Wall-clock time the player can't wind forward (`clock.rs`), so
    /// time with the app closed still counts.
    clock: clock::Clock,
    game: Game,
    settings: Settings,
    sprites: Sprites,
    assets: Assets,
    brightness: render::Brightness,
    backdrop: Backdrop,
    nav: Nav,
    layout: Layout,
    cam: Camera,
    input: Input,
    mode: Mode,
    /// Where a page's back arrow returns to.
    page_from: Mode,
    leaderboard: Leaderboard,
    /// The name being typed, on a platform without a text box of its own.
    typing: Option<String>,
    /// A native text box for the name is open (Android answers later).
    asking_name: bool,
    keystones: KeystoneView,
    /// The animal page open over everything.
    detail: Option<usize>,
    /// The biome guide, opened from the map.
    biomes_open: bool,
    opening: Option<Opening>,
    /// Human ended the Earth: plays instead of the opening.
    collapse: Option<Collapse>,
    /// The radiation panel under the RAD badge.
    rad_info: bool,
    boon_hover: Option<usize>,
    pinch_accum: f32,
    last_phase: Phase,
    /// The wait dial is up (after tapping LET TIME RUN).
    choosing: bool,
    /// Horizontal drag, in pixels, not yet turned into dial steps.
    dial_swipe: f32,
    /// CANCEL was tapped once while time runs; a second tap gives up the wait.
    cancel_armed: bool,
    /// Asked of Android once a second while settings are open.
    notify_allowed: bool,
    allowed_timer: f32,
    /// The backdrop's clock: it hurries while time runs, without jumps.
    world_t: f32,
    elapsed: f32,
    /// Frames written by `ASCENDIO_SHOT_EVERY`.
    frame_no: u32,
    save_timer: f32,
    autoplay_timer: f32,
    demo_left: u32,
    demo_timer: f32,
}

impl App {
    fn new(dev: Dev) -> Self {
        if env_flag("ASCENDIO_FRESH") {
            save::clear();
        }
        // A sped-up dev clock never becomes the real one.
        let mut clock = clock::Clock::load(dev.persist && dev.time_scale == 1.0);
        let now = clock.now(dev.time_scale);
        let mut game = if dev.persist {
            Game::load(now).unwrap_or_else(|| Game::new(now))
        } else {
            Game::new(now)
        };
        if let Some(p) = Dev::planet_override() {
            game.planet = p;
            game.shaped_from = p;
        }
        let mut settings = if dev.persist {
            Settings::load()
        } else {
            Settings::default()
        };
        if settings.ensure_player(leaderboard::id_seed()) && dev.persist {
            settings.save();
        }
        notify::sync(&game, settings.notify, now, dev.time_scale);
        if dev.demo_taps.is_some() {
            game.step_lever(Lever::Oxygen, 1);
            game.accelerate(now);
            game.skip_cycle(now);
        }
        let mut nav = Nav::new(&game);
        nav.go_to(&game, game.most_advanced());
        let mut app = Self {
            layout: map_layout(&game),
            sprites: Sprites::build(&game.phy),
            assets: Assets::build(),
            brightness: render::Brightness::new(),
            backdrop: Backdrop::new(),
            nav,
            cam: Camera::new(),
            input: Input::new(),
            mode: Mode::Spiral,
            page_from: Mode::Spiral,
            // Only a real game, played in real time, goes on the board.
            leaderboard: Leaderboard::new(
                dev.persist
                    && dev.time_scale == 1.0
                    && dev.shot.is_none()
                    && dev.demo_taps.is_none(),
            ),
            typing: None,
            asking_name: false,
            keystones: KeystoneView::default(),
            detail: None,
            biomes_open: false,
            opening: None,
            collapse: dev.shot_is(ShotKind::Collapse).then(Collapse::new),
            rad_info: false,
            boon_hover: None,
            pinch_accum: 1.0,
            last_phase: game.phase(),
            choosing: dev.shot_is(ShotKind::Dial),
            dial_swipe: 0.0,
            cancel_armed: false,
            notify_allowed: false,
            allowed_timer: 0.0,
            world_t: 0.0,
            elapsed: 0.0,
            frame_no: 0,
            save_timer: 0.0,
            autoplay_timer: 0.0,
            demo_left: dev.demo_taps.unwrap_or(0),
            demo_timer: 0.8,
            game,
            settings,
            clock,
            dev,
        };
        // A capture of the board needs it loaded by then.
        if app.dev.shot_is(ShotKind::Leaderboard) {
            app.open_page(Mode::Leaderboard);
        }
        app
    }

    fn save(&self) {
        if self.dev.persist {
            self.game.save();
        }
    }

    fn sync_notify(&self, now: f64) {
        notify::sync(&self.game, self.settings.notify, now, self.dev.time_scale);
    }

    fn settings_view(&self) -> SettingsView {
        SettingsView {
            notify: self.settings.notify,
            supported: notify::supported(),
            allowed: self.notify_allowed,
            brightness: self.settings.brightness,
            name: self.settings.name.clone(),
            typing: self.typing.clone(),
        }
    }

    fn spiral_frame(&self) -> Frame {
        Frame::build(&self.game, &self.nav, &|s, px| render::text_width(s, px))
    }

    /// Back to the spiral with nothing open, for a game that just changed
    /// wholesale (a reset, a new Earth).
    fn reset_views(&mut self) {
        self.nav = Nav::new(&self.game);
        self.layout = map_layout(&self.game);
        self.mode = Mode::Spiral;
        self.detail = None;
        self.opening = None;
        self.choosing = false;
    }

    /// The map, fitted to the tree, from the spiral; otherwise the spiral.
    fn toggle_map(&mut self) {
        self.mode = match self.mode {
            Mode::Spiral => {
                self.cam.fit(self.layout.min, self.layout.max);
                Mode::Map
            }
            _ => Mode::Spiral,
        };
    }

    /// EVOLVE IT: the genome opens, unless Human's gamble ends the Earth.
    fn evolve(&mut self, now: f64) {
        if self.game.doomed {
            self.collapse = Some(Collapse::new());
        } else {
            self.opening = Some(Opening::new(&self.game, (now * 1000.0) as u32));
        }
    }

    /// One frame: input, then the clock-driven updates, then drawing.
    /// Returns false to quit.
    fn frame(&mut self) -> bool {
        // Recording frames: a fixed step, however slow the export.
        let dt = self
            .dev
            .shot
            .as_ref()
            .and_then(|s| s.every)
            .unwrap_or_else(get_frame_time);
        self.elapsed += dt;
        let now = self.clock.now(self.dev.time_scale);
        self.game.tick(now);

        if self.game.phase() != self.last_phase {
            self.last_phase = self.game.phase();
            self.choosing &= self.last_phase == Phase::Shape;
            self.cancel_armed = false;
            self.save();
            self.sync_notify(now);
        }

        if self.typing.is_some() {
            self.type_name();
        } else if is_key_pressed(KeyCode::Escape) {
            self.save();
            self.clock.save();
            return false;
        }
        if self.asking_name {
            self.ask_name();
        }
        let score = Score {
            name: self.settings.name.clone(),
            rad: self.game.rad,
            species: self.game.species_ever() as u32,
            animal: self.game.showcase(),
        };
        self.leaderboard.update(
            &self.settings.player_id,
            &score,
            macroquad::miniquad::date::now(),
        );
        self.keys(now);

        let gesture = self.input.poll();
        let mut tap = gesture.tap;
        self.demo(dt, &mut tap);

        // Leaving the spiral drops the time selection: back to LET TIME RUN.
        if self.mode != Mode::Spiral {
            self.choosing = false;
        }
        if self.mode != Mode::Keystones {
            self.keystones.menu = None;
        }
        self.dial_keys();
        let panel = if self.choosing {
            ui::wait_panel_rect()
        } else {
            ui::panel_rect()
        };

        self.overlay_taps(&mut tap, &gesture, now);
        self.cancel_tap(&mut tap);
        self.page_taps(&mut tap, now);
        self.bottom_bar_taps(&mut tap, panel, now);
        match self.mode {
            Mode::Spiral => self.spiral_input(&gesture, tap, panel, dt),
            Mode::Map => self.map_input(&gesture, tap),
            Mode::Settings => {
                self.allowed_timer -= dt;
                if self.allowed_timer <= 0.0 {
                    self.allowed_timer = 1.0;
                    self.notify_allowed = notify::allowed();
                }
            }
            Mode::Keystones => self.keystones_input(&gesture, tap),
            Mode::Leaderboard => {
                if gesture.drag != Vec2::ZERO {
                    self.leaderboard.scroll = (self.leaderboard.scroll - gesture.drag.y)
                        .clamp(0.0, ui::leaderboard_max_scroll(&self.leaderboard));
                }
            }
        }

        if self.dev.autoplay {
            self.autoplay_timer -= dt;
            if self.autoplay_timer <= 0.0 {
                self.autoplay_timer = 0.15;
                run_autoplay(&mut self.game, now);
                self.layout = map_layout(&self.game);
                self.nav.rebuild(&self.game);
            }
        }
        self.save_timer -= dt;
        if self.save_timer <= 0.0 {
            self.save_timer = SAVE_INTERVAL;
            self.save();
            self.clock.save();
        }
        self.animate(dt);

        let capture = self
            .dev
            .shot
            .as_ref()
            .is_some_and(|s| self.elapsed >= s.after);
        if capture {
            if let Some(forced) = self.dev.shot.as_ref().and_then(|s| s.kind?.mode()) {
                self.mode = forced;
            }
            if self.mode == Mode::Map {
                self.cam.fit(self.layout.min, self.layout.max);
            }
            self.biomes_open |= self.dev.shot_is(ShotKind::Biomes);
        }

        self.draw(dt, now);
        self.export(capture)
    }

    /// Dev keys (R resets the game, T ends the running cycle) and M for the map.
    fn keys(&mut self, now: f64) {
        if self.dev.keys && is_key_pressed(KeyCode::R) {
            self.game = Game::new(now);
            self.reset_views();
        }
        if self.dev.keys && is_key_pressed(KeyCode::T) {
            self.game.skip_cycle(now);
        }
        if is_key_pressed(KeyCode::M) && matches!(self.mode, Mode::Spiral | Mode::Map) {
            self.toggle_map();
        }
    }

    /// Arrow keys step the wait dial while it's up.
    fn dial_keys(&mut self) {
        let dial_live = self.choosing
            && self.mode == Mode::Spiral
            && self.opening.is_none()
            && self.detail.is_none();
        if dial_live {
            let step = wait::STEP_HOURS;
            if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Right) {
                self.game.set_wait(self.game.wait_hours + step);
            }
            if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Left) {
                self.game.set_wait(self.game.wait_hours - step);
            }
        }
    }

    /// `ASCENDIO_DEMO_TAPS`: opens the ready genome and taps through it.
    fn demo(&mut self, dt: f32, tap: &mut Option<Vec2>) {
        if self.dev.demo_taps.is_none() {
            return;
        }
        if self.opening.is_none() && self.game.phase() == Phase::Genome {
            self.opening = Some(Opening::new(&self.game, 7));
        }
        self.demo_timer -= dt;
        if self.demo_left > 0 && self.demo_timer <= 0.0 {
            self.demo_timer = 0.5;
            self.demo_left -= 1;
            *tap = Some(vec2(screen_width() * 0.5, screen_height() * 0.5));
        }
    }

    /// Overlays eat taps first: the end of the Earth, the RAD panel, the
    /// opening, the biome guide, an animal's page, the boon pick.
    fn overlay_taps(&mut self, tap: &mut Option<Vec2>, gesture: &Gesture, now: f64) {
        if let Some(c) = &mut self.collapse {
            if tap.take().is_some() {
                c.tap();
            }
        } else if self.rad_info {
            if tap.take().is_some() {
                self.rad_info = false;
            }
        } else if tap.is_some_and(|p| {
            matches!(self.mode, Mode::Spiral | Mode::Map | Mode::Keystones)
                && ui::rad_badge_rect(&self.game).is_some_and(|b| b.contains(p))
        }) {
            self.rad_info = true;
            *tap = None;
        } else if let Some(op) = &mut self.opening {
            if tap.take().is_some() {
                let results = op.wants_results().then(|| self.game.open_genome(now));
                let discovered = results.as_ref().and_then(|r| deepest(&self.game, r));
                op.tap(results);
                if let Some(new) = discovered {
                    self.layout = map_layout(&self.game);
                    self.nav.go_to(&self.game, new);
                    self.save();
                }
            }
            if self.opening.as_ref().is_some_and(Opening::is_done) {
                self.opening = None;
            }
        } else if self.biomes_open {
            if tap.take().is_some() {
                self.biomes_open = false;
            }
        } else if self.detail.is_some() {
            if tap.take().is_some() {
                self.detail = None;
            }
        } else if self.game.phase() == Phase::Boon {
            self.boon_hover = (0..3).find(|&i| ui::boon_rect(i).contains(gesture.pointer));
            if let Some(p) = tap.take() {
                if let Some(i) = (0..3).find(|&i| ui::boon_rect(i).contains(p)) {
                    self.game.choose_boon(i);
                    self.save();
                }
            }
        }
    }

    /// CANCEL asks once, then gives up the running wait; any other tap
    /// disarms it.
    fn cancel_tap(&mut self, tap: &mut Option<Vec2>) {
        let Some(p) = *tap else { return };
        if self.game.phase() == Phase::Running
            && self.mode == Mode::Spiral
            && ui::cancel_wait_rect().contains(p)
        {
            if self.cancel_armed {
                self.game.cancel_cycle();
                self.cancel_armed = false;
                self.save();
            } else {
                self.cancel_armed = true;
            }
            *tap = None;
        } else {
            self.cancel_armed = false;
        }
    }

    /// The gear opens settings; on the settings screen, every tap stays there.
    /// Opens `page` from the top bar; from the other page, it swaps, and
    /// back still returns to the screen the first one was opened from.
    fn open_page(&mut self, page: Mode) {
        if !self.mode.is_page() {
            self.page_from = self.mode;
        }
        self.mode = page;
        match page {
            Mode::Settings => self.allowed_timer = 0.0,
            Mode::Leaderboard => {
                self.leaderboard.scroll = 0.0;
                self.leaderboard.refresh(&self.settings.player_id);
            }
            _ => {}
        }
    }

    /// The trophy and the gear open their pages; on a page, every tap stays
    /// there (no bottom bar).
    fn page_taps(&mut self, tap: &mut Option<Vec2>, now: f64) {
        let Some(p) = *tap else { return };
        let page = [
            (ui::gear_rect(), Mode::Settings),
            (ui::trophy_rect(), Mode::Leaderboard),
        ]
        .into_iter()
        .find(|(r, _)| r.contains(p))
        .map(|(_, m)| m);
        if !self.mode.is_page() {
            if let Some(page) = page {
                self.open_page(page);
                *tap = None;
            }
            return;
        }
        *tap = None;
        // A name being typed is saved by any tap away from it.
        if self.typing.is_some() {
            self.commit_name();
            if ui::name_button_rect().contains(p) {
                return;
            }
        }
        if ui::settings_back_rect().contains(p) || page == Some(self.mode) {
            self.mode = self.page_from;
        } else if let Some(page) = page {
            self.open_page(page);
        } else if self.mode == Mode::Leaderboard {
            if matches!(self.leaderboard.view, leaderboard::View::Failed) {
                self.leaderboard.refresh(&self.settings.player_id);
            }
        } else {
            self.settings_page_tap(p, now);
        }
    }

    fn settings_page_tap(&mut self, p: Vec2, now: f64) {
        if ui::name_button_rect().contains(p) {
            if net::has_text_box() {
                self.asking_name = true;
                self.ask_name();
            } else {
                self.typing = Some(self.settings.name.clone());
            }
        } else if notify::supported() && ui::notify_row_rect().contains(p) {
            // Turned on but blocked by Android: the text leads to its
            // settings, the checkbox still turns it off.
            if ui::notify_blocked(&self.settings_view()) && !ui::notify_checkbox_rect().contains(p)
            {
                notify::open_settings();
            } else {
                self.settings.notify = !self.settings.notify;
                if self.settings.notify {
                    notify::ask();
                }
                self.allowed_timer = 0.0;
                self.sync_notify(now);
                if self.dev.persist {
                    self.settings.save();
                }
            }
        } else if let Some(plus) = [false, true]
            .into_iter()
            .find(|&plus| ui::brightness_button_rect(plus).contains(p))
        {
            if self.settings.step_brightness(plus) && self.dev.persist {
                self.settings.save();
            }
        }
    }

    /// Takes `raw` as the leaderboard name, if anything usable is left.
    fn set_name(&mut self, raw: &str) {
        if let Some(name) = leaderboard::clean_name(raw) {
            self.settings.name = name;
            if self.dev.persist {
                self.settings.save();
            }
        }
    }

    fn commit_name(&mut self) {
        if let Some(typed) = self.typing.take() {
            self.set_name(&typed);
        }
    }

    /// The platform's text box: answers at once on the web, later on Android.
    fn ask_name(&mut self) {
        let current = self.settings.name.clone();
        if let Some(answer) = net::ask_text("Your name on the leaderboard", &current) {
            self.asking_name = false;
            if let Some(name) = answer {
                self.set_name(&name);
            }
        }
    }

    /// Typing the name on the game's own keyboard: Enter saves, Escape drops.
    fn type_name(&mut self) {
        let Some(typed) = &mut self.typing else {
            return;
        };
        while let Some(c) = get_char_pressed() {
            if leaderboard::name_char(c) && typed.chars().count() < leaderboard::NAME_MAX {
                typed.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            typed.pop();
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            self.commit_name();
        } else if is_key_pressed(KeyCode::Escape) {
            self.typing = None;
        }
    }

    /// The tabs, the action button, and the lever panel's own taps.
    fn bottom_bar_taps(&mut self, tap: &mut Option<Vec2>, panel: Rect, now: f64) {
        let Some(p) = *tap else { return };
        *tap = None;
        if ui::bottom_tab_rect(false).contains(p) {
            self.mode = if self.mode == Mode::Keystones {
                Mode::Spiral
            } else {
                Mode::Keystones
            };
        } else if ui::bottom_tab_rect(true).contains(p) {
            let was_map = self.mode == Mode::Map;
            self.mode = Mode::Spiral;
            if !was_map {
                self.toggle_map();
            }
        } else if ui::bottom_action_rect().contains(p) {
            match self.game.phase() {
                Phase::Shape if self.choosing => {
                    self.game.accelerate(now);
                    self.choosing = false;
                    self.mode = Mode::Spiral;
                }
                Phase::Shape if self.game.wait_choosable() => {
                    // The dial lives on the spiral screen.
                    self.mode = Mode::Spiral;
                    self.choosing = true;
                }
                Phase::Shape => {
                    self.game.accelerate(now);
                    self.mode = Mode::Spiral;
                }
                Phase::Genome => {
                    self.evolve(now);
                    self.mode = Mode::Spiral;
                }
                _ => {}
            }
        } else if self.mode == Mode::Spiral && self.choosing && panel.contains(p) {
            if ui::wait_back_rect().contains(p) {
                self.choosing = false;
            }
        } else if self.mode == Mode::Spiral
            && self.game.phase() == Phase::Genome
            && ui::panel_rect().contains(p)
        {
            self.evolve(now);
        } else {
            *tap = Some(p);
        }
    }

    fn spiral_input(&mut self, gesture: &Gesture, tap: Option<Vec2>, panel: Rect, dt: f32) {
        let dragging = gesture.drag != Vec2::ZERO && !panel.contains(gesture.pointer);
        if dragging && self.choosing {
            // With the dial up, a swipe sets it, not the spiral: right waits
            // longer, left shorter.
            self.dial_swipe += gesture.drag.x;
            let step = dial::step_px();
            while self.dial_swipe.abs() >= step {
                let dir = self.dial_swipe.signum();
                self.game
                    .set_wait(self.game.wait_hours + dir * wait::STEP_HOURS);
                self.dial_swipe -= dir * step;
            }
        }
        if gesture.released {
            self.dial_swipe = 0.0;
        }
        if dragging && !self.choosing {
            self.nav.drag_around(
                spiral::coil_center(),
                gesture.pointer - gesture.drag,
                gesture.pointer,
                spiral::coil_radius(),
                dt,
            );
        }
        if gesture.released {
            self.nav.release();
        }
        if self.choosing && (gesture.wheel != 0.0 || gesture.wheel_x != 0.0) {
            let notches = gesture.wheel_x + gesture.wheel;
            self.game
                .set_wait(self.game.wait_hours + notches.signum() * wait::STEP_HOURS);
        } else if gesture.wheel != 0.0 {
            self.nav.step(-gesture.wheel);
        }
        if gesture.pinch != 1.0 {
            self.pinch_accum *= gesture.pinch;
            if self.pinch_accum < PINCH_TO_MAP {
                self.toggle_map();
                self.pinch_accum = 1.0;
            }
        } else {
            self.pinch_accum = 1.0;
        }
        if let Some(p) = tap {
            // The levers are hidden while the dial is up.
            let lever_hit = Lever::ALL
                .iter()
                .enumerate()
                .filter(|_| !self.choosing)
                .find_map(|(i, &l)| {
                    [(false, -1i8), (true, 1i8)]
                        .into_iter()
                        .find(|&(plus, _)| ui::lever_button(i, plus).contains(p))
                        .map(|(_, d)| (l, d))
                });
            if let Some((lever, delta)) = lever_hit {
                self.game.step_lever(lever, delta);
            } else if !panel.contains(p) {
                let frame = self.spiral_frame();
                match frame.hit(p) {
                    Some(i) if i == frame.focus => self.detail = Some(i),
                    Some(i) => self.nav.go_to(&self.game, i),
                    None => {}
                }
            }
        }
        if gesture.drag == Vec2::ZERO {
            self.nav.settle(dt);
        }
    }

    fn map_input(&mut self, gesture: &Gesture, tap: Option<Vec2>) {
        if gesture.pinch != 1.0 {
            self.cam.zoom_at(gesture.pinch_anchor, gesture.pinch);
        }
        if gesture.wheel != 0.0 {
            let f = if gesture.wheel > 0.0 { 1.1 } else { 1.0 / 1.1 };
            self.cam.zoom_at(gesture.pinch_anchor, f);
        }
        if gesture.drag != Vec2::ZERO {
            self.cam.pan_screen(gesture.drag);
        }
        if let Some(p) = tap {
            if ui::biomes_button_rect().contains(p) {
                self.biomes_open = true;
            } else if let Some(i) = self.layout.hit(self.cam.screen_to_world(p)) {
                self.detail = Some(i);
            }
        }
        self.cam.clamp_to(self.layout.min, self.layout.max);
    }

    fn keystones_input(&mut self, gesture: &Gesture, tap: Option<Vec2>) {
        let game = &mut self.game;
        let view = &mut self.keystones;
        if gesture.drag != Vec2::ZERO && view.menu.is_none() {
            view.scroll =
                (view.scroll - gesture.drag.y).clamp(0.0, ui::keystones_max_scroll(game, view));
        }
        let Some(p) = tap else { return };
        // An open dropdown takes the tap: a pick, or anywhere else to close it.
        if let Some(menu) = view.menu {
            match ui::keystone_menu_item_at(game, view, p) {
                Some(i) => ui::choose_keystone_menu(view, menu, i),
                None => view.menu = None,
            }
            return;
        }
        if let Some(menu) = [Menu::Sort, Menu::Filter]
            .into_iter()
            .find(|&m| ui::keystone_menu_button_rect(game, m).contains(p))
        {
            view.menu = Some(menu);
            return;
        }
        let changed = match view.selected {
            Some(sel) if game.morphs[sel] != 0 && ui::keystone_morph_rect().contains(p) => {
                game.cycle_edition(sel)
            }
            Some(sel) if ui::keystone_equip_rect().contains(p) => {
                game.toggle_keystone(sel);
                true
            }
            Some(sel) if ui::keystone_info_rect().contains(p) => {
                self.detail = Some(sel);
                false
            }
            _ => {
                if let Some(t) =
                    ui::keystone_slot_at(game, p).or_else(|| ui::keystone_cell_at(game, view, p))
                {
                    view.selected = Some(t);
                }
                false
            }
        };
        if changed {
            self.save();
        }
    }

    /// The opening and the end of the Earth play on.
    fn animate(&mut self, dt: f32) {
        if let Some(op) = &mut self.opening {
            op.update(dt);
        }
        let Some(c) = &mut self.collapse else { return };
        c.update(dt);
        if c.wants_reset() {
            c.reset_done();
            self.game.end_earth();
            self.reset_views();
            self.save();
        }
        if self.collapse.as_ref().is_some_and(Collapse::is_done) {
            self.collapse = None;
        }
    }

    fn draw(&mut self, dt: f32, now: f64) {
        let game = &self.game;
        let t = get_time() as f32;
        match self.mode {
            Mode::Spiral => {
                let shown = game.cycle.map_or(game.planet, |c| c.launched);
                let shown = self.collapse.as_ref().map_or(shown, |c| c.planet(shown));
                let speed = if game.phase() == Phase::Running {
                    2.5
                } else {
                    1.0
                };
                self.world_t += dt * speed;
                self.backdrop.draw(&shown, self.world_t, t, dt);
                if !self.dev.shot_is(ShotKind::Backdrop) {
                    if self.choosing {
                        dial::draw(game.wait_hours, &self.sprites, &self.assets);
                    }
                    let frame = self.spiral_frame();
                    render::draw_spiral(game, &frame, self.nav.t, self.nav.last(), &self.sprites);
                    ui::draw_hud(game, now, &self.assets);
                    // The end of the Earth gets the whole screen.
                    if self.collapse.is_none() {
                        if self.choosing {
                            ui::draw_wait_panel(game, &self.sprites);
                        } else {
                            ui::draw_lever_panel(
                                game,
                                now,
                                &self.assets,
                                &self.sprites,
                                self.cancel_armed,
                            );
                        }
                        ui::draw_bottom_bar(game, now, None, self.choosing, &self.assets);
                    }
                }
            }
            Mode::Map => {
                render::draw_map(game, &self.layout, &self.cam, &self.sprites);
                ui::draw_hud(game, now, &self.assets);
                ui::draw_biomes_button();
                ui::draw_bottom_bar(game, now, Some(true), self.choosing, &self.assets);
            }
            Mode::Keystones => {
                ui::draw_keystones(game, now, &self.keystones, &self.sprites, &self.assets);
                ui::draw_bottom_bar(game, now, Some(false), self.choosing, &self.assets);
            }
            Mode::Settings => ui::draw_settings(&self.settings_view(), &self.assets),
            Mode::Leaderboard => {
                ui::draw_leaderboard(game, &self.leaderboard, &self.sprites, &self.assets)
            }
        }
        if game.phase() == Phase::Boon && self.opening.is_none() {
            ui::draw_boons(game, &self.sprites, &self.assets, self.boon_hover);
        }
        // The guide belongs to the map: leaving it closes the guide.
        self.biomes_open &= self.mode == Mode::Map;
        if self.biomes_open {
            ui::draw_biomes(game, &self.sprites);
        }
        if let Some(i) = self.detail {
            ui::draw_detail(game, i, &self.sprites, &self.assets);
        }
        if let Some(op) = &mut self.opening {
            op.draw(game, &self.sprites, &self.assets);
        }
        if self.rad_info {
            ui::draw_rad_info(game);
        }
        if let Some(c) = &mut self.collapse {
            c.draw(game.rad, game.fossils());
        }
        self.brightness.apply(self.settings.brightness);
    }

    /// Writes the `ASCENDIO_SHOT*` captures. Returns false once the final
    /// one is written.
    fn export(&mut self, capture: bool) -> bool {
        let Some(shot) = &self.dev.shot else {
            return true;
        };
        if shot.every.is_some() {
            let stem = shot.path.trim_end_matches(".png");
            get_screen_data().export_png(&format!("{stem}_{:03}.png", self.frame_no));
            self.frame_no += 1;
        }
        if !capture {
            return true;
        }
        get_screen_data().export_png(&shot.path);
        println!(
            "wrote {} ({}x{})",
            shot.path,
            screen_width(),
            screen_height()
        );
        false
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new(Dev::from_env());
    while app.frame() {
        next_frame().await;
    }
}

/// Dev autoplay: shape at random, let time run instantly, open, pick.
fn run_autoplay(game: &mut Game, now: f64) {
    match game.phase() {
        Phase::Shape => {
            for _ in 0..4 {
                let lever = Lever::ALL[macroquad::rand::gen_range(0, Lever::ALL.len())];
                let d = if macroquad::rand::gen_range(0, 3) == 0 {
                    -1
                } else {
                    1
                };
                game.step_lever(lever, d);
            }
            let found: Vec<usize> = (0..game.phy.len()).filter(|&i| game.unlocked[i]).collect();
            while game.keystones.len() < game.keystone_slots().min(found.len()) {
                let t = found[macroquad::rand::gen_range(0, found.len())];
                if !game.toggle_keystone(t) {
                    break;
                }
            }
            game.accelerate(now);
        }
        Phase::Running => game.skip_cycle(now),
        Phase::Genome if game.doomed => game.end_earth(),
        Phase::Genome => {
            game.open_genome(now);
        }
        Phase::Boon => game.choose_boon(0),
    }
}
