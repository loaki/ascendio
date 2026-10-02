//! Ascendio -- a daily game about the animal tree of life: shape the planet,
//! let time run, evolve the genome it produced. See `docs/DESIGN.md`.

mod backdrop;
mod dial;
mod ecology;
mod game;
mod genome;
mod genome_bg;
mod layout;
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
use game::{Game, Phase};
use layout::Layout;
use opening::Opening;
use planet::Lever;
use settings::Settings;
use spiral::{Frame, Nav};
use sprites::Sprites;
use ui::{Assets, KeystoneView, SettingsView};
use view::{Camera, Input};

/// How hard you have to pinch closed before the map opens.
const PINCH_TO_MAP: f32 = 0.82;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Spiral,
    Map,
    Keystones,
    Settings,
}

/// Seconds between autosaves.
const SAVE_INTERVAL: f32 = 3.0;

fn env_flag(key: &str) -> bool {
    std::env::var(key).is_ok_and(|v| v != "0")
}

fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

/// A framebuffer capture: write `path` after `after` seconds, then exit.
struct Shot {
    path: String,
    after: f32,
    mode: Option<Mode>,
    /// `ASCENDIO_SHOT_MODE=dial`: the spiral with the wait dial up.
    dial: bool,
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
    /// `ASCENDIO_SHOT_MODE=map|spiral|dial|keystones|settings`.
    shot: Option<Shot>,
}

impl Dev {
    fn from_env() -> Self {
        let autoplay = env_flag("ASCENDIO_AUTOPLAY");
        let shot_mode = std::env::var("ASCENDIO_SHOT_MODE").ok();
        let shot = std::env::var("ASCENDIO_SHOT").ok().map(|path| Shot {
            path,
            after: env_parse("ASCENDIO_SHOT_AFTER").unwrap_or(5.0),
            mode: match shot_mode.as_deref() {
                Some("map") => Some(Mode::Map),
                Some("spiral" | "dial") => Some(Mode::Spiral),
                Some("keystones") => Some(Mode::Keystones),
                Some("settings") => Some(Mode::Settings),
                _ => None,
            },
            dial: shot_mode.as_deref() == Some("dial"),
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

#[macroquad::main(window_conf)]
async fn main() {
    let dev = Dev::from_env();
    let save = |g: &Game| {
        if dev.persist {
            g.save();
        }
    };
    if env_flag("ASCENDIO_FRESH") {
        save::clear();
    }
    // Wall clock, so time with the app closed still counts.
    let now = || macroquad::miniquad::date::now() * dev.time_scale;
    let mut game = if dev.persist {
        Game::load(now()).unwrap_or_else(|| Game::new(now()))
    } else {
        Game::new(now())
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
    notify::sync(&game, settings.notify, now(), dev.time_scale);
    // Asked of Android once a second while settings are open.
    let mut notify_allowed = false;
    let mut allowed_timer = 0.0f32;
    // Where the back arrow returns to.
    let mut settings_from = Mode::Spiral;
    let demo_taps = dev.demo_taps;
    if demo_taps.is_some() {
        game.step_lever(Lever::Oxygen, 1);
        game.accelerate(now());
        game.skip_cycle(now());
    }
    let mut demo_left = demo_taps.unwrap_or(0);
    let mut demo_timer = 0.8f32;
    let mut nav = Nav::new(&game);
    nav.go_to(&game, game.most_advanced());
    let mut layout = map_layout(&game);
    let sprites = Sprites::build(&game.phy);
    let assets = Assets::build();
    let mut backdrop = Backdrop::new();
    let mut world_t = 0.0f32;

    let mut cam = Camera::new();
    let mut input = Input::new();
    let mut mode = Mode::Spiral;
    let mut keystones = KeystoneView::default();
    let mut detail: Option<usize> = None;
    let mut opening: Option<Opening> = None;
    let mut boon_hover: Option<usize> = None;
    let mut pinch_accum = 1.0f32;
    let mut last_phase = game.phase();
    // The wait dial is up (after tapping LET TIME RUN).
    let mut choosing = dev.shot.as_ref().is_some_and(|s| s.dial);
    // Horizontal drag, in pixels, not yet turned into dial steps.
    let mut dial_swipe = 0.0f32;
    // CANCEL was tapped once while time runs; a second tap gives up the wait.
    let mut cancel_armed = false;

    let mut elapsed = 0.0f32;
    let mut save_timer = 0.0f32;
    let mut autoplay_timer = 0.0f32;

    loop {
        let dt = get_frame_time();
        elapsed += dt;
        let t_now = now();
        game.tick(t_now);

        if game.phase() != last_phase {
            last_phase = game.phase();
            choosing &= last_phase == Phase::Shape;
            cancel_armed = false;
            save(&game);
            notify::sync(&game, settings.notify, t_now, dev.time_scale);
        }

        if is_key_pressed(KeyCode::Escape) {
            save(&game);
            break;
        }
        if dev.keys && is_key_pressed(KeyCode::R) {
            game = Game::new(t_now);
            nav = Nav::new(&game);
            layout = map_layout(&game);
            mode = Mode::Spiral;
            detail = None;
            opening = None;
            choosing = false;
        }
        if dev.keys && is_key_pressed(KeyCode::T) {
            game.skip_cycle(t_now);
        }
        if is_key_pressed(KeyCode::M) && matches!(mode, Mode::Spiral | Mode::Map) {
            mode = toggle(mode, &mut cam, &layout);
        }

        let gesture = input.poll();
        let measure = |s: &str, px: f32| render::text_width(s, px);
        let mut tap = gesture.tap;
        if demo_taps.is_some() {
            if opening.is_none() && game.phase() == Phase::Genome {
                opening = Some(Opening::new(&game, 7));
            }
            demo_timer -= dt;
            if demo_left > 0 && demo_timer <= 0.0 {
                demo_timer = 0.5;
                demo_left -= 1;
                tap = Some(vec2(screen_width() * 0.5, screen_height() * 0.5));
            }
        }

        // Leaving the spiral drops the time selection: back to LET TIME RUN.
        if mode != Mode::Spiral {
            choosing = false;
        }
        let dial_live = choosing && mode == Mode::Spiral && opening.is_none() && detail.is_none();
        if dial_live {
            let step = wait::STEP_HOURS;
            if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Right) {
                game.set_wait(game.wait_hours + step);
            }
            if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Left) {
                game.set_wait(game.wait_hours - step);
            }
        }
        let panel = if choosing {
            ui::wait_panel_rect()
        } else {
            ui::panel_rect()
        };

        // Overlays eat taps first.
        if let Some(op) = &mut opening {
            if let Some(p) = tap.take() {
                let results = op.wants_results().then(|| game.open_genome(t_now));
                let discovered = results.as_ref().and_then(|r| deepest(&game, r));
                op.tap(p, results);
                if let Some(new) = discovered {
                    layout = map_layout(&game);
                    nav.go_to(&game, new);
                    save(&game);
                }
            }
            if op.is_done() {
                opening = None;
            }
        } else if detail.is_some() {
            if tap.take().is_some() {
                detail = None;
            }
        } else if game.phase() == Phase::Boon {
            boon_hover = (0..3).find(|&i| ui::boon_rect(i).contains(gesture.pointer));
            if let Some(p) = tap.take() {
                if let Some(i) = (0..3).find(|&i| ui::boon_rect(i).contains(p)) {
                    game.choose_boon(i);
                    save(&game);
                }
            }
        }

        if let Some(p) = tap {
            if game.phase() == Phase::Running
                && mode == Mode::Spiral
                && ui::cancel_wait_rect().contains(p)
            {
                if cancel_armed {
                    game.cancel_cycle();
                    cancel_armed = false;
                    save(&game);
                } else {
                    cancel_armed = true;
                }
                tap = None;
            } else if cancel_armed {
                cancel_armed = false;
            }
        }
        if let Some(p) = tap {
            if mode == Mode::Settings {
                // Full screen, no bottom bar: nothing below sees the tap.
                tap = None;
                if ui::settings_back_rect().contains(p) || ui::gear_rect().contains(p) {
                    mode = settings_from;
                } else if notify::supported() && ui::notify_row_rect().contains(p) {
                    settings.notify = !settings.notify;
                    if settings.notify {
                        notify::ask();
                    }
                    allowed_timer = 0.0;
                    notify::sync(&game, settings.notify, t_now, dev.time_scale);
                    if dev.persist {
                        settings.save();
                    }
                } else if ui::notify_status_rect().contains(p) && settings.notify && !notify_allowed
                {
                    notify::open_settings();
                }
            } else if ui::gear_rect().contains(p) {
                settings_from = mode;
                mode = Mode::Settings;
                allowed_timer = 0.0;
                tap = None;
            }
        }
        if let Some(p) = tap {
            if ui::bottom_tab_rect(false).contains(p) {
                mode = if mode == Mode::Keystones {
                    Mode::Spiral
                } else {
                    Mode::Keystones
                };
                tap = None;
            } else if ui::bottom_tab_rect(true).contains(p) {
                mode = if mode == Mode::Map {
                    Mode::Spiral
                } else {
                    toggle(Mode::Spiral, &mut cam, &layout)
                };
                tap = None;
            } else if ui::bottom_action_rect().contains(p) {
                match game.phase() {
                    Phase::Shape if choosing => {
                        game.accelerate(t_now);
                        choosing = false;
                        mode = Mode::Spiral;
                    }
                    Phase::Shape if game.wait_choosable() => {
                        // The dial lives on the spiral screen.
                        mode = Mode::Spiral;
                        choosing = true;
                    }
                    Phase::Shape => {
                        game.accelerate(t_now);
                        mode = Mode::Spiral;
                    }
                    Phase::Genome => {
                        opening = Some(Opening::new(&game, (t_now * 1000.0) as u32));
                    }
                    _ => {}
                }
                tap = None;
            } else if mode == Mode::Spiral && choosing && panel.contains(p) {
                if ui::wait_back_rect().contains(p) {
                    choosing = false;
                }
                tap = None;
            } else if mode == Mode::Spiral
                && game.phase() == Phase::Genome
                && ui::panel_rect().contains(p)
            {
                opening = Some(Opening::new(&game, (t_now * 1000.0) as u32));
                tap = None;
            }
        }

        match mode {
            Mode::Spiral => {
                let dragging = gesture.drag != Vec2::ZERO && !panel.contains(gesture.pointer);
                if dragging && choosing {
                    // With the dial up, a swipe sets it, not the spiral:
                    // right waits longer, left shorter.
                    dial_swipe += gesture.drag.x;
                    let step = dial::step_px();
                    while dial_swipe.abs() >= step {
                        let dir = dial_swipe.signum();
                        game.set_wait(game.wait_hours + dir * wait::STEP_HOURS);
                        dial_swipe -= dir * step;
                    }
                }
                if gesture.released {
                    dial_swipe = 0.0;
                }
                if dragging && !choosing {
                    nav.drag_around(
                        spiral::coil_center(),
                        gesture.pointer - gesture.drag,
                        gesture.pointer,
                        spiral::coil_radius(),
                        dt,
                    );
                }
                if gesture.released {
                    nav.release();
                }
                if choosing && (gesture.wheel != 0.0 || gesture.wheel_x != 0.0) {
                    let notches = gesture.wheel_x + gesture.wheel;
                    game.set_wait(game.wait_hours + notches.signum() * wait::STEP_HOURS);
                } else if gesture.wheel != 0.0 {
                    nav.step(-gesture.wheel);
                }
                if gesture.pinch != 1.0 {
                    pinch_accum *= gesture.pinch;
                    if pinch_accum < PINCH_TO_MAP {
                        mode = toggle(mode, &mut cam, &layout);
                        pinch_accum = 1.0;
                    }
                } else {
                    pinch_accum = 1.0;
                }
                if let Some(p) = tap {
                    // The levers are hidden while the dial is up.
                    let lever_hit = Lever::ALL
                        .iter()
                        .enumerate()
                        .filter(|_| !choosing)
                        .find_map(|(i, &l)| {
                            [(false, -1i8), (true, 1i8)]
                                .into_iter()
                                .find(|&(plus, _)| ui::lever_button(i, plus).contains(p))
                                .map(|(_, d)| (l, d))
                        });
                    if let Some((lever, delta)) = lever_hit {
                        game.step_lever(lever, delta);
                    } else if !panel.contains(p) {
                        let frame = Frame::build(&game, &nav, &measure);
                        match frame.hit(p) {
                            Some(i) if i == frame.focus => detail = Some(i),
                            Some(i) => nav.go_to(&game, i),
                            None => {}
                        }
                    }
                }
                if gesture.drag == Vec2::ZERO {
                    nav.settle(dt);
                }
            }
            Mode::Map => {
                if gesture.pinch != 1.0 {
                    cam.zoom_at(gesture.pinch_anchor, gesture.pinch);
                }
                if gesture.wheel != 0.0 {
                    let f = if gesture.wheel > 0.0 { 1.1 } else { 1.0 / 1.1 };
                    cam.zoom_at(gesture.pinch_anchor, f);
                }
                if gesture.drag != Vec2::ZERO {
                    cam.pan_screen(gesture.drag);
                }
                if let Some(p) = tap {
                    if let Some(i) = layout.hit(cam.screen_to_world(p)) {
                        detail = Some(i);
                    }
                }
                cam.clamp_to(layout.min, layout.max);
            }
            Mode::Settings => {
                allowed_timer -= dt;
                if allowed_timer <= 0.0 {
                    allowed_timer = 1.0;
                    notify_allowed = notify::allowed();
                }
            }
            Mode::Keystones => {
                if gesture.drag != Vec2::ZERO {
                    keystones.scroll = (keystones.scroll - gesture.drag.y)
                        .clamp(0.0, ui::keystones_max_scroll(&game));
                }
                if let Some(p) = tap {
                    if let Some(sel) = keystones
                        .selected
                        .filter(|_| ui::keystone_equip_rect().contains(p))
                    {
                        game.toggle_keystone(sel);
                        save(&game);
                    } else if let Some(sel) = keystones
                        .selected
                        .filter(|_| ui::keystone_info_rect().contains(p))
                    {
                        detail = Some(sel);
                    } else if let Some(t) = ui::keystone_slot_at(&game, p)
                        .or_else(|| ui::keystone_cell_at(&game, &keystones, p))
                    {
                        keystones.selected = Some(t);
                    }
                }
            }
        }

        if dev.autoplay {
            autoplay_timer -= dt;
            if autoplay_timer <= 0.0 {
                autoplay_timer = 0.15;
                run_autoplay(&mut game, t_now);
                layout = map_layout(&game);
                nav.rebuild(&game);
            }
        }

        save_timer -= dt;
        if save_timer <= 0.0 {
            save_timer = SAVE_INTERVAL;
            save(&game);
        }
        if let Some(op) = &mut opening {
            op.update(dt);
        }

        let capture = dev.shot.as_ref().filter(|s| elapsed >= s.after);
        if let Some(shot) = capture {
            if let Some(forced) = shot.mode {
                mode = forced;
            }
            if mode == Mode::Map {
                cam.fit(layout.min, layout.max);
            }
        }

        let t = get_time() as f32;
        match mode {
            Mode::Spiral => {
                let shown = game.cycle.map(|c| c.launched).unwrap_or(game.planet);
                // The world hurries while time runs; accumulated so it never jumps.
                let speed = if game.phase() == Phase::Running {
                    2.5
                } else {
                    1.0
                };
                world_t += dt * speed;
                backdrop.draw(&shown, world_t, t, dt);
                if choosing {
                    dial::draw(game.wait_hours, &sprites, &assets);
                }
                let frame = Frame::build(&game, &nav, &measure);
                render::draw_spiral(&game, &frame, nav.t, nav.last(), &sprites);
                ui::draw_hud(&game, t_now, &assets);
                if choosing {
                    ui::draw_wait_panel(&game, &sprites);
                } else {
                    ui::draw_lever_panel(&game, t_now, &assets, &sprites, cancel_armed);
                }
                ui::draw_bottom_bar(&game, t_now, None, choosing, &assets);
            }
            Mode::Map => {
                render::draw_map(&game, &layout, &cam, &sprites);
                ui::draw_hud(&game, t_now, &assets);
                ui::draw_bottom_bar(&game, t_now, Some(true), choosing, &assets);
            }
            Mode::Keystones => {
                ui::draw_keystones(&game, t_now, &keystones, &sprites, &assets);
                ui::draw_bottom_bar(&game, t_now, Some(false), choosing, &assets);
            }
            Mode::Settings => {
                let view = SettingsView {
                    notify: settings.notify,
                    supported: notify::supported(),
                    allowed: notify_allowed,
                };
                ui::draw_settings(&view, &assets);
            }
        }
        if game.phase() == Phase::Boon && opening.is_none() {
            ui::draw_boons(&game, &sprites, &assets, boon_hover);
        }
        if let Some(i) = detail {
            ui::draw_detail(&game, i, &sprites, &assets);
        }
        if let Some(op) = &mut opening {
            op.draw(&game, &sprites, &assets);
        }

        if let Some(shot) = capture {
            get_screen_data().export_png(&shot.path);
            println!(
                "wrote {} ({}x{})",
                shot.path,
                screen_width(),
                screen_height()
            );
            break;
        }

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
        Phase::Genome => {
            game.open_genome(now);
        }
        Phase::Boon => game.choose_boon(0),
    }
}

fn toggle(mode: Mode, cam: &mut Camera, layout: &Layout) -> Mode {
    match mode {
        Mode::Spiral => {
            cam.fit(layout.min, layout.max);
            Mode::Map
        }
        _ => Mode::Spiral,
    }
}

fn map_layout(game: &Game) -> Layout {
    layout::compute(game, &|s| render::text_width(s, render::LABEL_PX))
}
