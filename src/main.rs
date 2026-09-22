//! Ascendio -- an incremental game about unlocking the animal tree of life.
//!
//! You start as the Urmetazoan, the last common ancestor of all animals.
//! Every tap is a roll for a descendant lineage to split off.
//!
//! Two views: the spiral, which winds one lineage from the root out to a tip
//! with its branches sprouting off it, and the map, which shows the whole
//! discovered tree at once.

mod facts;
mod game;
mod layout;
mod render;
mod spiral;
mod sprites;
mod tree;
mod upgrades;
mod view;

use macroquad::prelude::*;

use game::{EvolveResult, Game};
use layout::Layout;
use spiral::{Frame, Hit, Nav};
use sprites::Sprites;
use view::{Camera, Input};

/// How hard you have to pinch closed before the map opens.
const PINCH_TO_MAP: f32 = 0.82;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Spiral,
    Map,
    /// The bottom bar's one functional side tab. Reachable from either of the
    /// above and always returns to whichever of them was active. Evolve sits
    /// centred in the same bar, between this and a `Leaderboard` placeholder
    /// that has no screen yet -- neither of those needs a `Mode` of its own:
    /// Evolve is a real-time action, not a screen, and Leaderboard is inert.
    Upgrades,
}

/// Dev aid: `ASCENDIO_TIME_SCALE=60` makes DNA accrue 60x faster, so the
/// whole economy can be felt in one sitting instead of over hours.
fn time_scale() -> f64 {
    std::env::var("ASCENDIO_TIME_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v: &f64| v > 0.0)
        .unwrap_or(1.0)
}

/// Seconds since the epoch. The economy runs on this rather than on frame
/// deltas, so time with the app backgrounded still counts.
fn wall_clock(scale: f64) -> f64 {
    macroquad::miniquad::date::now() * scale
}

/// Dev aid: `ASCENDIO_AUTOPLAY=1` taps random unlocked taxa on a timer, which
/// is how screenshots and demo recordings get a filled-in tree without a human.
fn autoplay_enabled() -> bool {
    std::env::var("ASCENDIO_AUTOPLAY").is_ok_and(|v| v != "0")
}

/// Drives `render::draw_evolve_fx`: which taxon to show and how far into the
/// particle-then-summon sequence we are. Lives independently of `Mode`, so
/// Evolve never has to change the screen you're looking at to be seen.
struct EvolveFx {
    elapsed: f32,
    taxon: usize,
    miss: bool,
}

/// Picks a random taxon Evolve could act on: unlocked, with something left to
/// give -- a child to discover, or (once that branch is complete) a
/// descendant to level up. `None` only once the entire tree is finished.
fn pick_evolve_target(game: &Game) -> Option<usize> {
    let live: Vec<usize> = (0..game.phy.len())
        .filter(|&i| game.unlocked[i] && !game.taxon(i).children.is_empty())
        .collect();
    if live.is_empty() {
        None
    } else {
        Some(live[macroquad::rand::gen_range(0, live.len())])
    }
}

/// Dev aid: `ASCENDIO_SHOT=out.png` writes a PNG of the game's own framebuffer
/// and exits. `ASCENDIO_SHOT_AFTER` (seconds, default 5) controls how long
/// autoplay runs first; `ASCENDIO_SHOT_MODE=map` captures the map instead.
fn screenshot_request() -> Option<(String, f32, Option<Mode>)> {
    let path = std::env::var("ASCENDIO_SHOT").ok()?;
    let after = std::env::var("ASCENDIO_SHOT_AFTER")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5.0);
    // Unset captures whatever view is live, which is how the toggle gets tested.
    let mode = match std::env::var("ASCENDIO_SHOT_MODE").as_deref() {
        Ok("map") => Some(Mode::Map),
        Ok("spiral") => Some(Mode::Spiral),
        Ok("upgrades") => Some(Mode::Upgrades),
        _ => None,
    };
    Some((path, after, mode))
}

fn window_conf() -> Conf {
    // Portrait by default, matching the target phone aspect. `ASCENDIO_W` and
    // `ASCENDIO_H` override it so other aspect ratios can be checked on desktop.
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

#[macroquad::main(window_conf)]
async fn main() {
    macroquad::rand::srand(macroquad::miniquad::date::now() as u64);

    let scale = time_scale();
    let mut game = Game::new(wall_clock(scale));
    let mut nav = Nav::new(&game);
    let mut layout = map_layout(&game);
    // Textures only, so this survives a reset -- the tree shape never changes.
    let sprites = Sprites::build(&game.phy);

    let mut cam = Camera::new();
    let mut input = Input::new();
    let mut mode = Mode::Spiral;
    // Where the bottom bar's Upgrades tab returns to when left.
    let mut return_mode = Mode::Spiral;
    let mut upgrades_scroll = 0.0f32;

    let mut pinch_accum = 1.0f32;
    // Counts down from EVOLVE_ANIM_SECONDS after a successful Evolve tap;
    // draw_focus_card eases its flash-and-punch out over that window. Purely
    // local to the spiral card, so it does nothing on the other two screens.
    let mut evolve_anim = 0.0f32;
    // The particle-and-summon overlay: mode-independent, so Evolve never has
    // to leave whatever screen it was pressed from to be seen. `None`
    // between Evolves.
    let mut evolve_fx: Option<EvolveFx> = None;

    // "Did you know?" fact shown on the map view. A random starting point so
    // a fresh game does not always open on the same one; changes only when
    // tapped (see the map tap handling below), never on a timer.
    let mut fact_index = macroquad::rand::gen_range(0, facts::FACTS.len());

    let autoplay = autoplay_enabled();
    let mut autoplay_timer = 0.0f32;
    let shot = screenshot_request();
    let mut elapsed = 0.0f32;

    loop {
        let dt = get_frame_time();
        elapsed += dt;
        game.tick(wall_clock(scale));

        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::R) {
            game = Game::new(wall_clock(scale));
            nav = Nav::new(&game);
            layout = map_layout(&game);
            mode = Mode::Spiral;
            return_mode = Mode::Spiral;
            upgrades_scroll = 0.0;
            evolve_anim = 0.0;
            evolve_fx = None;
        }
        if is_key_pressed(KeyCode::M) && mode != Mode::Upgrades {
            mode = toggle(mode, &mut cam, &layout);
        }
        if is_key_pressed(KeyCode::U) {
            (mode, return_mode) = enter_or_leave_upgrades(mode, return_mode);
        }
        if is_key_pressed(KeyCode::F) && mode == Mode::Map {
            cam.fit(layout.min, layout.max);
        }

        let gesture = input.poll();
        let measure = |s: &str, px: f32| render::text_width(s, px);

        let mut unlocked_now = None;

        // Chrome shared by every screen: the top-right button (Spiral<->Map,
        // or "back" out of Upgrades), the bottom bar's Upgrades tab, and
        // Evolve itself, centred in that same bar so it is reachable no
        // matter which screen is showing.
        let mut tap = gesture.tap;

        // The evolve reveal holds on screen once finished -- rather than
        // auto-dismissing on a timer -- so a tap anywhere is what it takes to
        // move on, and it eats that tap so nothing underneath also reacts to
        // it. Checked first since the overlay sits on top of everything.
        if evolve_fx.is_some() {
            if tap.is_some() {
                if evolve_fx
                    .as_ref()
                    .is_some_and(|fx| fx.elapsed >= render::EVOLVE_FX_SECONDS)
                {
                    evolve_fx = None;
                }
                tap = None;
            }
        } else if let Some(p) = tap {
            if render::mode_button().contains(p) {
                mode = if mode == Mode::Upgrades {
                    return_mode
                } else {
                    toggle(mode, &mut cam, &layout)
                };
                tap = None;
            } else if render::bottom_tab_rect(0).contains(p) {
                (mode, return_mode) = enter_or_leave_upgrades(mode, return_mode);
                tap = None;
            } else if render::bottom_evolve_rect().contains(p) && game.can_evolve() {
                // Evolve can land on any eligible lineage, not just whatever
                // the spiral happens to be showing -- but the reveal overlay
                // plays on top of whatever screen is up, so there is no need
                // to jump there any more; the current screen stays put.
                if let Some(target) = pick_evolve_target(&game) {
                    match game.evolve(target) {
                        EvolveResult::Unlocked(new) => {
                            unlocked_now = Some(new);
                            evolve_anim = render::EVOLVE_ANIM_SECONDS;
                            evolve_fx = Some(EvolveFx {
                                elapsed: 0.0,
                                taxon: new,
                                miss: false,
                            });
                        }
                        EvolveResult::Leveled(child) => {
                            nav.go_to(&game, child);
                            nav.t = nav.t_target;
                            evolve_anim = render::EVOLVE_ANIM_SECONDS;
                            evolve_fx = Some(EvolveFx {
                                elapsed: 0.0,
                                taxon: child,
                                miss: false,
                            });
                        }
                        EvolveResult::Miss => {
                            nav.go_to(&game, target);
                            nav.t = nav.t_target;
                            evolve_anim = render::EVOLVE_ANIM_SECONDS;
                            evolve_fx = Some(EvolveFx {
                                elapsed: 0.0,
                                taxon: target,
                                miss: true,
                            });
                        }
                        EvolveResult::Exhausted | EvolveResult::NotReady => {}
                    }
                }
                tap = None;
            }
        }

        match mode {
            Mode::Spiral => {
                if gesture.drag != Vec2::ZERO {
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

                // Pinching closed is the "show me more" direction.
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
                    match Frame::build(&game, &nav, &measure).hit(p) {
                        // The card is informational now -- Evolve lives in
                        // the bottom bar, and can land on any eligible
                        // lineage, not just whichever one is focused here.
                        Some(Hit::Card) => {}
                        // Reroutes the lineage through that branch.
                        Some(Hit::Jump(i)) => nav.go_to(&game, i),
                        None => {}
                    }
                }

                // Ease toward the target once the finger is off.
                if gesture.drag == Vec2::ZERO {
                    nav.settle(dt);
                }
            }

            Mode::Upgrades => {
                if gesture.drag != Vec2::ZERO {
                    let viewport = screen_height() - render::bar_height() * 2.0;
                    let max_scroll = (render::upgrades_content_height() - viewport).max(0.0);
                    upgrades_scroll = (upgrades_scroll - gesture.drag.y).clamp(0.0, max_scroll);
                }
                if let Some(p) = tap {
                    let rows = upgrades::rows(&game.phy);
                    let hit = rows.iter().enumerate().find(|&(row_i, _)| {
                        render::upgrade_row_rect(row_i, upgrades_scroll).contains(p)
                    });
                    if let Some((_, &(taxon, _))) = hit {
                        game.buy_upgrade_level(taxon);
                    }
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
                // Tapping the fact panel advances it -- no timer, so it never
                // changes out from under someone mid-read. Checked first
                // since the panel floats over the top of the tree.
                // Tapping a species elsewhere on the map opens it in the spiral.
                if let Some(p) = tap {
                    if render::fact_panel_rect().contains(p) {
                        fact_index = (fact_index + 1) % facts::FACTS.len();
                    } else if let Some(i) = layout.hit(cam.screen_to_world(p)) {
                        nav.go_to(&game, i);
                        nav.t = nav.t_target;
                        mode = Mode::Spiral;
                    }
                }
                cam.clamp_to(layout.min, layout.max);
            }
        }

        if autoplay {
            autoplay_timer -= dt;
            if autoplay_timer <= 0.0 {
                autoplay_timer = 0.03;
                // Also buy whatever upgrades it can afford, so a dev/screenshot
                // run exercises the shop instead of only ever hoarding for Evolve.
                for i in 0..game.phy.len() {
                    if game.can_afford_upgrade_level(i) {
                        game.buy_upgrade_level(i);
                    }
                }
                if game.can_evolve() {
                    let live: Vec<usize> = (0..game.phy.len())
                        .filter(|&i| game.unlocked[i] && !game.taxon(i).children.is_empty())
                        .collect();
                    // Skipping the attempt, not the frame -- `continue` here
                    // would jump over next_frame().await and spin the loop.
                    if !live.is_empty() {
                        let pick = live[macroquad::rand::gen_range(0, live.len())];
                        if let EvolveResult::Unlocked(new) = game.evolve(pick) {
                            unlocked_now = Some(new);
                        }
                    }
                }
            }
        }

        // A discovery extends the tree, so re-route the lineage through the
        // new species and glide to it.
        if let Some(new) = unlocked_now {
            layout = map_layout(&game);
            nav.go_to(&game, new);
        }

        game.update(dt);
        evolve_anim = (evolve_anim - dt).max(0.0);
        // Advances the animation, then holds at the fully-revealed frame --
        // `draw_evolve_fx` clamps past `EVOLVE_FX_SECONDS` -- until the tap
        // handling above dismisses it. No timer clears it here any more.
        if let Some(fx) = &mut evolve_fx {
            fx.elapsed += dt;
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

        match mode {
            Mode::Spiral => {
                let frame = Frame::build(&game, &nav, &measure);
                render::draw_spiral(&game, &frame, nav.t, nav.last(), &sprites, evolve_anim);
            }
            Mode::Map => render::draw_map(&game, &layout, &cam, &sprites, facts::FACTS[fact_index]),
            Mode::Upgrades => render::draw_upgrades(&game, &sprites, upgrades_scroll),
        }
        // Drawn last, on top of whichever screen is showing, regardless of
        // where Evolve was pressed from.
        if let Some(fx) = &evolve_fx {
            render::draw_evolve_fx(&game, &sprites, fx.taxon, fx.miss, fx.elapsed);
        }

        if capture_now {
            let (path, _, _) = shot.as_ref().unwrap();
            // Must read the framebuffer before the swap clears it.
            get_screen_data().export_png(path);
            println!("wrote {path} ({}x{})", screen_width(), screen_height());
            break;
        }

        next_frame().await;
    }
}

fn toggle(mode: Mode, cam: &mut Camera, layout: &Layout) -> Mode {
    match mode {
        Mode::Spiral => {
            cam.fit(layout.min, layout.max);
            Mode::Map
        }
        // Toggle is only ever called on Spiral/Map -- Upgrades has its own
        // return-to-caller path via `enter_or_leave_upgrades`.
        _ => Mode::Spiral,
    }
}

/// The bottom bar's one tab: tapping it enters Upgrades from wherever you
/// were, and tapping it again (or the top-right BACK button) leaves the way
/// you came in.
fn enter_or_leave_upgrades(mode: Mode, return_mode: Mode) -> (Mode, Mode) {
    if mode == Mode::Upgrades {
        (return_mode, return_mode)
    } else {
        (Mode::Upgrades, mode)
    }
}

fn map_layout(game: &Game) -> Layout {
    layout::compute(game, &|s| render::text_width(s, render::LABEL_PX))
}
