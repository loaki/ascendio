//! Ascendio -- a daily game about the animal tree of life: shape the planet,
//! let time run, evolve the genome it produced. See `docs/DESIGN.md`.

mod backdrop;
mod ecology;
mod facts;
mod game;
mod genome_bg;
mod layout;
mod nodule;
mod opening;
mod planet;
mod render;
mod save;
mod spiral;
mod sprites;
mod tree;
mod ui;
mod view;

use macroquad::prelude::*;

use backdrop::Backdrop;
use game::{Game, Phase};
use layout::Layout;
use opening::Opening;
use planet::Lever;
use spiral::{Frame, Hit, Nav};
use sprites::Sprites;
use ui::{Assets, KeystoneView};
use view::{Camera, Input};

/// How hard you have to pinch closed before the map opens.
const PINCH_TO_MAP: f32 = 0.82;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Spiral,
    Map,
    Keystones,
}

/// Dev aid: `ASCENDIO_TIME_SCALE=3600` makes an hour pass every second.
fn time_scale() -> f64 {
    std::env::var("ASCENDIO_TIME_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v: &f64| v > 0.0)
        .unwrap_or(1.0)
}

/// Wall clock, so time with the app closed still counts.
fn wall_clock(scale: f64) -> f64 {
    macroquad::miniquad::date::now() * scale
}

fn env_flag(key: &str) -> bool {
    std::env::var(key).is_ok_and(|v| v != "0")
}

/// Dev aid: `ASCENDIO_SHOT=out.png` writes a PNG of the framebuffer and
/// exits after `ASCENDIO_SHOT_AFTER` seconds (default 5).
/// `ASCENDIO_SHOT_MODE=map|spiral|keystones` captures that screen instead.
fn screenshot_request() -> Option<(String, f32, Option<Mode>)> {
    let path = std::env::var("ASCENDIO_SHOT").ok()?;
    let after = std::env::var("ASCENDIO_SHOT_AFTER")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5.0);
    let mode = match std::env::var("ASCENDIO_SHOT_MODE").as_deref() {
        Ok("map") => Some(Mode::Map),
        Ok("spiral") => Some(Mode::Spiral),
        Ok("keystones") => Some(Mode::Keystones),
        _ => None,
    };
    Some((path, after, mode))
}

fn window_conf() -> Conf {
    let dim = |key: &str, fallback: i32| {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(fallback)
    };
    Conf {
        window_title: "Ascendio".to_owned(),
        window_width: dim("ASCENDIO_W", 480),
        window_height: dim("ASCENDIO_H", 960),
        window_resizable: false,
        high_dpi: true,
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
    let scale = time_scale();
    let autoplay = env_flag("ASCENDIO_AUTOPLAY");
    let dev = env_flag("ASCENDIO_DEV") || autoplay;
    // Dev tooling never touches the player's real save.
    let persist = !autoplay && !env_flag("ASCENDIO_SCRATCH");
    let save = |g: &Game| {
        if persist {
            g.save();
        }
    };
    // Dev aid: `ASCENDIO_DEMO_TAPS=n` starts on a ready nodule and taps the
    // opening `n` times, half a second apart.
    let demo_taps: Option<u32> = std::env::var("ASCENDIO_DEMO_TAPS")
        .ok()
        .and_then(|v| v.parse().ok());
    if env_flag("ASCENDIO_FRESH") {
        save::clear();
    }
    let now = || wall_clock(scale);
    let mut game = if persist {
        Game::load(now()).unwrap_or_else(|| Game::new(now()))
    } else {
        Game::new(now())
    };
    // Dev aid: `ASCENDIO_PLANET=land,veg,o2,temp,volc` forces the planet.
    if let Ok(spec) = std::env::var("ASCENDIO_PLANET") {
        let v: Vec<u8> = spec
            .split(',')
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        if let [land, vegetation, oxygen, temperature, volcanism] = v[..] {
            game.planet = planet::Planet {
                land,
                vegetation,
                oxygen,
                temperature,
                volcanism,
            };
            game.shaped_from = game.planet;
        }
    }
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
    let mut fact_index = macroquad::rand::gen_range(0, facts::FACTS.len());
    let mut last_phase = game.phase();

    let shot = screenshot_request();
    let mut elapsed = 0.0f32;
    let mut save_timer = 0.0f32;
    let mut autoplay_timer = 0.0f32;
    const SAVE_INTERVAL: f32 = 3.0;

    loop {
        let dt = get_frame_time();
        elapsed += dt;
        let t_now = now();
        game.tick(t_now);

        if game.phase() != last_phase {
            last_phase = game.phase();
            if last_phase == Phase::Running {
                fact_index = macroquad::rand::gen_range(0, facts::FACTS.len());
            }
            save(&game);
        }

        if is_key_pressed(KeyCode::Escape) {
            save(&game);
            break;
        }
        if dev && is_key_pressed(KeyCode::R) {
            game = Game::new(t_now);
            nav = Nav::new(&game);
            layout = map_layout(&game);
            mode = Mode::Spiral;
            detail = None;
            opening = None;
        }
        if dev && is_key_pressed(KeyCode::T) {
            game.skip_cycle(t_now);
        }
        if is_key_pressed(KeyCode::M) && matches!(mode, Mode::Spiral | Mode::Map) {
            mode = toggle(mode, &mut cam, &layout);
        }

        let gesture = input.poll();
        let measure = |s: &str, px: f32| render::text_width(s, px);
        let mut tap = gesture.tap;
        if demo_taps.is_some() {
            if opening.is_none() && game.phase() == Phase::Nodule {
                let tell = game.nodule_tell().unwrap_or(ecology::Tier::Common);
                opening = Some(Opening::new(tell, 7));
            }
            demo_timer -= dt;
            if demo_left > 0 && demo_timer <= 0.0 {
                demo_timer = 0.5;
                demo_left -= 1;
                tap = Some(vec2(screen_width() * 0.5, screen_height() * 0.5));
            }
        }

        // Overlays eat taps first.
        if let Some(op) = &mut opening {
            if let Some(p) = tap.take() {
                let results = op.wants_results().then(|| game.open_nodule(t_now));
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
                    Phase::Shape => {
                        game.accelerate(t_now);
                    }
                    Phase::Nodule => {
                        let tell = game.nodule_tell().unwrap_or(ecology::Tier::Common);
                        opening = Some(Opening::new(tell, (t_now * 1000.0) as u32));
                    }
                    _ => {}
                }
                tap = None;
            } else if mode == Mode::Spiral
                && game.phase() == Phase::Nodule
                && ui::panel_rect().contains(p)
            {
                let tell = game.nodule_tell().unwrap_or(ecology::Tier::Common);
                opening = Some(Opening::new(tell, (t_now * 1000.0) as u32));
                tap = None;
            }
        }

        match mode {
            Mode::Spiral => {
                if gesture.drag != Vec2::ZERO && !ui::panel_rect().contains(gesture.pointer) {
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
                if gesture.wheel != 0.0 {
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
                    let lever_hit = Lever::ALL.iter().enumerate().find_map(|(i, &l)| {
                        [(false, -1i8), (true, 1i8)]
                            .into_iter()
                            .find(|&(plus, _)| ui::lever_button(i, plus).contains(p))
                            .map(|(_, d)| (l, d))
                    });
                    if let Some((lever, delta)) = lever_hit {
                        game.step_lever(lever, delta);
                    } else if !ui::panel_rect().contains(p) {
                        let frame = Frame::build(&game, &nav, &measure);
                        match frame.hit(p) {
                            Some(Hit::Jump(i)) if i == frame.focus => detail = Some(i),
                            Some(Hit::Jump(i)) => nav.go_to(&game, i),
                            _ => {}
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

        if autoplay {
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

        let capture_now = shot.as_ref().is_some_and(|&(_, after, _)| elapsed >= after);
        if capture_now {
            if let Some(forced) = shot.as_ref().unwrap().2 {
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
                let frame = Frame::build(&game, &nav, &measure);
                render::draw_spiral(&game, &frame, nav.t, nav.last(), &sprites);
                ui::draw_hud(&game, t_now);
                ui::draw_lever_panel(&game, t_now, &assets, facts::FACTS[fact_index]);
                ui::draw_bottom_bar(&game, t_now, None, &assets);
            }
            Mode::Map => {
                render::draw_map(&game, &layout, &cam, &sprites);
                ui::draw_hud(&game, t_now);
                ui::draw_bottom_bar(&game, t_now, Some(true), &assets);
            }
            Mode::Keystones => {
                ui::draw_keystones(&game, t_now, &keystones, &sprites, &assets);
                ui::draw_bottom_bar(&game, t_now, Some(false), &assets);
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

        if capture_now {
            let (path, _, _) = shot.as_ref().unwrap();
            get_screen_data().export_png(path);
            println!("wrote {path} ({}x{})", screen_width(), screen_height());
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
        Phase::Nodule => {
            game.open_nodule(now);
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
