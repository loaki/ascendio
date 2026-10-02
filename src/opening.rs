//! Evolving a new genome, the game's "pack opening": a neutral helix mutated
//! by three taps, a supernova in the best card's colour (the first hint of
//! rarity), the cards, a showcase per card, then a summary. Pure
//! presentation -- `Game::open_genome` has already applied the results.

use macroquad::prelude::*;

use crate::ecology::Tier;
use crate::game::{Game, Opened};
use crate::genome::Morph;
use crate::genome_bg::{GenomeBg, Rays};
use crate::pixel::rgb_of;
use crate::render::{
    self, ease_out_back, ease_out_cubic, faded, fit_px, rgb, text_centered, tier_color, CYAN, LIME,
    PINK, TEXT, TEXT_DIM,
};
use crate::sprites::Sprites;
use crate::ui::{self, Assets};

const TAPS_TO_CRACK: u32 = 3;
const PRISM: [u32; 6] = [0xFF7AB8, 0xFFC56B, 0xC5F76A, 0x6FF5E1, 0x5AA8FF, 0xC07BFF];
/// A, T, G, C.
const BASES: [u32; 4] = [0x6FF5E1, 0xFF7AB8, 0xC5F76A, 0xFFC56B];
/// 12 blocks of base pairs; each tap mutates the next four, scattered.
const MUT_ORDER: [usize; 12] = [7, 2, 10, 4, 0, 9, 5, 11, 1, 8, 3, 6];
/// Seconds into the burst at which the collapsed core detonates.
const BOOM: f32 = 0.45;
/// Before the supernova the genome must not give the tier away.
const SEALED: Color = rgb(0xBFEFFF);

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max: f32,
    color: Color,
    size: f32,
    gravity: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Sealed,
    Burst,
    Showcase(usize),
    /// Every card revealed: the opening is over.
    Done,
}

pub struct Opening {
    stage: Stage,
    t: f32,
    tell: Tier,
    taps: u32,
    /// Counts down after each tap: the glitch and the MUTATION! slam.
    mut_flash: f32,
    boomed: bool,
    /// The current card's reveal flash has fired (once, however it got there).
    reveal_fired: bool,
    shake: f32,
    freeze: f32,
    flash: f32,
    opened: Vec<Opened>,
    flipped: Vec<f32>,
    particles: Vec<Particle>,
    seed: u32,
    bg: GenomeBg,
}

fn rnd(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32) / u32::MAX as f32
}

fn lighten(c: Color, k: f32) -> Color {
    Color::new(
        c.r + (1.0 - c.r) * k,
        c.g + (1.0 - c.g) * k,
        c.b + (1.0 - c.b) * k,
        c.a,
    )
}

fn genome_center() -> Vec2 {
    vec2(screen_width() * 0.5, screen_height() * 0.44)
}

fn helix_len() -> f32 {
    render::u() * 158.0
}

/// A double helix around a vertical axis through `c`. `rung(f, k, half)`
/// colours each rung half. Back half first so the twist reads in 3D.
#[allow(clippy::too_many_arguments)]
fn draw_helix(
    c: Vec2,
    len: f32,
    amp: f32,
    freq: f32,
    phase: f32,
    fade: f32,
    tint: Option<Color>,
    rung: &dyn Fn(f32, usize, bool) -> Color,
    jitter: &dyn Fn(f32) -> f32,
) {
    let u = render::u();
    let n = 120;
    let top = c.y - len * 0.5;
    let strand_front = tint.unwrap_or(rgb(0xE6F4F0));
    let strand_back = rgb(0x3E5A66);
    for front in [false, true] {
        for i in (0..=n).step_by(5) {
            let f = i as f32 / n as f32;
            let a = phase + f * freq * std::f32::consts::TAU;
            let (sa, z) = (a.sin(), a.cos());
            if (z > 0.0) != front {
                continue;
            }
            let (y, dx) = (top + f * len, jitter(f));
            let (ax, bx) = (c.x + dx + sa * amp, c.x + dx - sa * amp);
            let mid = (ax + bx) * 0.5;
            let k = i / 5;
            let alpha = (0.35 + 0.55 * z.abs()) * fade;
            draw_line(ax, y, mid, y, u, faded(rung(f, k, false), alpha));
            draw_line(mid, y, bx, y, u, faded(rung(f, k, true), alpha));
        }
        for sign in [1.0f32, -1.0] {
            for i in 0..=n {
                let f = i as f32 / n as f32;
                let a = phase + f * freq * std::f32::consts::TAU;
                let z = a.cos() * sign;
                if (z > 0.0) != front {
                    continue;
                }
                let x = c.x + jitter(f) + a.sin() * amp * sign;
                let y = top + f * len;
                let w = if z > 0.2 { u * 1.6 } else { u * 1.1 };
                let col = if z > 0.3 { strand_front } else { strand_back };
                draw_rectangle(
                    x - w * 0.5,
                    y - w * 0.5,
                    w,
                    w,
                    faded(col, (0.5 + 0.5 * z) * fade),
                );
                if z > 0.85 && i % 5 == 0 {
                    draw_rectangle(x, y - u * 0.5, u * 0.5, u * 0.5, faded(WHITE, fade));
                }
            }
        }
    }
}

/// How long the real silhouette sits locked before it bursts into colour.
const ROULETTE_HOLD: f32 = 0.4;
/// The first gap between roulette ticks, and how much each gap grows.
const ROULETTE_GAP: f32 = 0.05;
const ROULETTE_SLOWDOWN: f32 = 1.22;

/// The silhouette the roulette shows at `t` for card `i` (real `taxon`
/// once it locks at `lock`), whether a tick just happened, and whether
/// it has locked.
fn roulette(game: &Game, taxon: usize, i: usize, t: f32, lock: f32) -> (usize, bool, bool) {
    if t >= lock {
        return (taxon, t - lock < 0.06, true);
    }
    // Tick times grow apart like a slowing reel.
    let (mut at, mut gap, mut k) = (0.0, ROULETTE_GAP, 0);
    while at + gap <= t {
        at += gap;
        gap *= ROULETTE_SLOWDOWN;
        k += 1;
    }
    let n = game.phy.len();
    let mut pick = (crate::pixel::hash(k, i as i32 * 31 + taxon as i32) * n as f32) as usize % n;
    if pick == taxon {
        pick = (pick + 1) % n;
    }
    (pick, t - at < 0.04, false)
}

/// When a new species bursts into colour, in seconds into its reveal.
const REVEAL_AT: f32 = 1.8;

/// When card `new`-or-not has settled and the "next card" hint shows; a tap
/// before then skips to it.
fn settled_at(new: bool) -> f32 {
    REVEAL_AT + if new { 0.8 } else { 0.4 }
}

/// Where the card being revealed is shown.
fn showcase_center() -> Vec2 {
    vec2(screen_width() * 0.5, screen_height() * 0.36)
}
/// Where the row of card slots sits, as a fraction of the screen height.
const SLOTS_Y: f32 = 0.775;
/// Most slots in one row; more wrap to a second.
const SLOTS_PER_ROW: usize = 8;

/// Slots per row and the size of one, for `n` cards (no cap on `n`).
fn slot_layout(n: usize) -> (usize, Vec2) {
    let sw = screen_width();
    let per_row = n.clamp(1, SLOTS_PER_ROW);
    let gap = sw * 0.03;
    let w = ((sw * 0.9 - gap * (per_row as f32 - 1.0)) / per_row as f32).min(sw * 0.155);
    (per_row, vec2(w, w * 1.39))
}

/// The centre of card `i`'s slot, of `n`, under the reveal.
fn slot_pos(i: usize, n: usize) -> Vec2 {
    let (sw, sh) = (screen_width(), screen_height());
    let (per_row, size) = slot_layout(n);
    let rows = n.div_ceil(per_row).max(1);
    let (row, col) = (i / per_row, i % per_row);
    let in_row = if row + 1 == rows {
        n - row * per_row
    } else {
        per_row
    };
    let gap = sw * 0.03;
    let total = in_row as f32 * size.x + (in_row as f32 - 1.0) * gap;
    let x = (sw - total) * 0.5 + col as f32 * (size.x + gap) + size.x * 0.5;
    let y = sh * SLOTS_Y + (row as f32 - (rows as f32 - 1.0) * 0.5) * (size.y + gap * 0.6);
    vec2(x, y)
}

/// A card's slot before its reveal; `lit` while it is the one being revealed.
fn draw_empty_slot(c: Vec2, size: Vec2, lit: bool) {
    let u = render::u();
    let r = Rect::new(c.x - size.x * 0.5, c.y - size.y * 0.5, size.x, size.y);
    draw_rectangle(r.x, r.y, r.w, r.h, rgb(0x071422));
    let line = (u * 0.8).max(2.0);
    let (edge, w) = if lit {
        let pulse = 0.6 + 0.4 * (get_time() as f32 * 4.0).sin();
        (faded(WHITE, pulse), line * 1.5)
    } else {
        (rgb(0x1E4450), line)
    };
    draw_rectangle_lines(r.x, r.y, r.w, r.h, w, edge);
}

/// A revealed card's slot: its rarity colour and its animal.
fn draw_filled_slot(game: &Game, sprites: &Sprites, o: &Opened, c: Vec2, size: Vec2) {
    let u = render::u();
    let r = Rect::new(c.x - size.x * 0.5, c.y - size.y * 0.5, size.x, size.y);
    let col = tier_color(o.card.tier);
    draw_rectangle(r.x, r.y, r.w, r.h, rgb(0x071422));
    draw_rectangle(r.x, r.y, r.w, r.h, faded(col, 0.18));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, (u * 0.8).max(2.0), col);
    ui::draw_taxon(
        sprites,
        game,
        o.card.taxon,
        o.card.morph,
        c,
        size.x * 0.72,
        WHITE,
    );
    let px = size.x * 0.26;
    // NEW along the top for a first discovery.
    if o.card.new {
        text_centered(
            "NEW",
            c.x,
            r.y + px * 1.05,
            fit_px("NEW", r.w * 0.85, px),
            PINK,
        );
    }
    // The morph on a band across the bottom, in its tab colour.
    if o.card.morph != Morph::None {
        let (fill, ink) = ui::morph_colors(o.card.morph);
        let band = px * 1.15;
        let inset = (u * 0.8).max(2.0);
        draw_rectangle(
            r.x + inset,
            r.y + r.h - band - inset,
            r.w - inset * 2.0,
            band,
            fill,
        );
        let name = o.card.morph.name().to_uppercase();
        text_centered(
            &name,
            c.x,
            r.y + r.h - inset - band * 0.25,
            fit_px(&name, r.w * 0.85, px * 0.9),
            ink,
        );
    }
}

impl Opening {
    /// Opens `game`'s waiting genome; `seed` only varies the effects.
    pub fn new(game: &Game, seed: u32) -> Self {
        Self {
            stage: Stage::Sealed,
            t: 0.0,
            tell: game.genome_tell().unwrap_or(Tier::Common),
            taps: 0,
            mut_flash: 0.0,
            boomed: false,
            reveal_fired: false,
            shake: 0.0,
            freeze: 0.0,
            flash: 0.0,
            opened: Vec::new(),
            flipped: Vec::new(),
            particles: Vec::new(),
            seed: seed.max(1),
            bg: GenomeBg::new(),
        }
    }

    pub fn is_done(&self) -> bool {
        self.stage == Stage::Done
    }

    /// The next tap bursts it: `main` opens the genome and passes the results.
    pub fn wants_results(&self) -> bool {
        self.stage == Stage::Sealed && self.taps + 1 >= TAPS_TO_CRACK
    }

    fn burst_particles(
        &mut self,
        at: Vec2,
        n: usize,
        speed: f32,
        colors: &[Color],
        gravity: f32,
        size: f32,
    ) {
        for _ in 0..n {
            let a = rnd(&mut self.seed) * std::f32::consts::TAU;
            let s = speed * (0.3 + rnd(&mut self.seed) * 0.7);
            let color = colors[(rnd(&mut self.seed) * colors.len() as f32) as usize % colors.len()];
            let max = 0.5 + rnd(&mut self.seed) * 0.9;
            self.particles.push(Particle {
                pos: at,
                vel: vec2(a.cos(), a.sin()) * s,
                life: max,
                max,
                color,
                size: size * (0.5 + rnd(&mut self.seed)),
                gravity,
            });
        }
    }

    /// `results` must be `Some` when `wants_results()` was true (best last).
    pub fn tap(&mut self, results: Option<Vec<Opened>>) {
        let u = render::u();
        match self.stage {
            Stage::Sealed => {
                self.taps += 1;
                self.mut_flash = 1.0;
                self.shake = 4.0 * u * (self.taps as f32 / TAPS_TO_CRACK as f32 + 0.4);
                let block = MUT_ORDER[((self.taps - 1) * 4) as usize % 12];
                let c = genome_center();
                let at = vec2(
                    c.x,
                    c.y - helix_len() * 0.5 + helix_len() * (block as f32 + 0.5) / 12.0,
                );
                let mut cols = vec![SEALED, WHITE];
                cols.extend(BASES.iter().map(|&h| rgb(h)));
                self.burst_particles(
                    at,
                    18 + self.taps as usize * 8,
                    70.0 * u,
                    &cols,
                    0.0,
                    u * 1.3,
                );
                if self.taps >= TAPS_TO_CRACK {
                    self.opened = results.unwrap_or_default();
                    self.flipped = vec![0.0; self.opened.len()];
                    self.stage = Stage::Burst;
                    self.t = 0.0;
                    self.boomed = false;
                }
            }
            Stage::Burst => {
                if self.t > BOOM + 0.6 {
                    self.reveal_next();
                }
            }
            Stage::Showcase(i) => {
                let full = settled_at(self.opened[i].card.new);
                if self.t < full {
                    self.t = full;
                } else {
                    self.reveal_next();
                }
            }
            Stage::Done => {}
        }
    }

    /// Starts the next card's reveal, or ends the opening once every card
    /// is revealed.
    fn reveal_next(&mut self) {
        let u = render::u();
        let Some(i) = self.flipped.iter().position(|&f| f <= 0.0) else {
            self.stage = Stage::Done;
            return;
        };
        self.flipped[i] = 0.001;
        let col = tier_color(self.opened[i].card.tier);
        let at = slot_pos(i, self.opened.len());
        self.burst_particles(at, 24, 120.0 * u, &[col, WHITE], 0.0, u * 1.3);
        self.shake = (1.5 + self.opened[i].card.tier.index() as f32) * u;
        self.stage = Stage::Showcase(i);
        self.t = 0.0;
        self.reveal_fired = false;
    }

    /// The flash as card `i`'s animal shows: a burst of particles for a new
    /// species, a smaller jolt for a duplicate.
    fn fire_reveal(&mut self, i: usize) {
        let u = render::u();
        let card = &self.opened[i].card;
        let tier = card.tier;
        if !card.new {
            self.flash = 0.6;
            self.shake = (3.0 + tier.index() as f32 * 2.0) * u;
            return;
        }
        self.flash = 1.0;
        self.shake = (6.0 + tier.index() as f32 * 3.0) * u;
        let mut cols = vec![tier_color(tier), WHITE];
        if tier >= Tier::Epic {
            cols.extend(PRISM.iter().map(|&h| rgb(h)));
        }
        self.burst_particles(
            showcase_center(),
            90 + tier.index() * 30,
            220.0 * u,
            &cols,
            30.0 * u,
            u * 1.6,
        );
    }

    /// The roulette for card `i`: other silhouettes flicker past, slowing
    /// down, until the real one (`taxon`) locks in before `REVEAL_AT`.
    fn draw_roulette(&self, game: &Game, sprites: &Sprites, taxon: usize, i: usize, c: Vec2) {
        let u = render::u();
        let size = screen_width() * 0.42;
        let (shown, ticked, locked) = roulette(game, taxon, i, self.t, REVEAL_AT - ROULETTE_HOLD);
        let tint = if ticked {
            Color::new(0.24, 0.27, 0.32, 1.0)
        } else {
            render::SILHOUETTE
        };
        ui::draw_taxon(sprites, game, shown, Morph::None, c, size * 0.9, tint);
        let (label, col) = if locked {
            ("LOCKED", WHITE)
        } else {
            ("? ? ?", CYAN)
        };
        text_centered(label, c.x, c.y + size * 0.62, u * 9.0, col);
    }

    /// The row of slots: revealed cards filled, card `current` lit until
    /// it is `shown`, the rest empty.
    fn draw_slots(&self, game: &Game, sprites: &Sprites, current: usize, shown: bool) {
        let n = self.opened.len();
        let (_, size) = slot_layout(n);
        for (k, o) in self.opened.iter().enumerate() {
            let c = slot_pos(k, n);
            if k < current || (k == current && shown) {
                draw_filled_slot(game, sprites, o, c, size);
            } else {
                draw_empty_slot(c, size, k == current);
            }
        }
    }

    /// "tap: next card (3 left)", or how to finish.
    fn next_hint(&self) -> String {
        let left = self.flipped.iter().filter(|&&f| f <= 0.0).count();
        match left {
            0 => "tap to finish".into(),
            1 => "tap: the last card".into(),
            k => format!("tap: next card  ({k} left)"),
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.freeze > 0.0 {
            self.freeze -= dt;
            self.flash = 1.0;
            return;
        }
        self.t += dt;
        self.shake = (self.shake - dt * 40.0 * render::u()).max(0.0);
        self.flash = (self.flash - dt * 1.8).max(0.0);
        for f in self.flipped.iter_mut() {
            if *f > 0.0 {
                *f = (*f + dt * 4.0).min(1.0);
            }
        }
        for p in self.particles.iter_mut() {
            p.life -= dt;
            p.vel.y += p.gravity * dt;
            p.vel *= 1.0 - dt * 1.2;
            p.pos += p.vel * dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        self.mut_flash = (self.mut_flash - dt * 1.4).max(0.0);
        if self.stage == Stage::Burst {
            let u = render::u();
            if !self.boomed {
                self.shake = self.shake.max(self.t / BOOM * 3.0 * u);
                if self.t >= BOOM {
                    self.boomed = true;
                    let power = 1.0 + self.tell.index() as f32 * 0.35;
                    self.freeze = if self.tell == Tier::Legendary {
                        0.35
                    } else {
                        0.1 + self.tell.index() as f32 * 0.04
                    };
                    self.flash = 1.0;
                    self.shake = 16.0 * u * power;
                    let c = genome_center();
                    let best = tier_color(self.tell);
                    let mut cols = vec![WHITE, rgb(0xFFF0C0)];
                    cols.extend(BASES.iter().map(|&h| rgb(h)));
                    let extra = 2 + self.tell.index() * 2;
                    cols.extend((0..extra).map(|i| lighten(best, (i % 3) as f32 * 0.25)));
                    self.burst_particles(
                        c,
                        160 + self.tell.index() * 50,
                        300.0 * u,
                        &cols,
                        25.0 * u,
                        u * 1.7,
                    );
                }
            }
            if self.t - BOOM > 1.3 + self.tell.index() as f32 * 0.15 {
                self.reveal_next();
            }
        }
        if let Stage::Showcase(i) = self.stage {
            // A new species flashes just before it shows, a duplicate as it does.
            let at = if self.opened[i].card.new {
                REVEAL_AT - 0.05
            } else {
                REVEAL_AT
            };
            if !self.reveal_fired && self.t >= at {
                self.reveal_fired = true;
                self.fire_reveal(i);
            }
        }
    }

    fn shake_offset(&mut self) -> Vec2 {
        if self.shake <= 0.1 {
            return Vec2::ZERO;
        }
        vec2(rnd(&mut self.seed) - 0.5, rnd(&mut self.seed) - 0.5) * self.shake * 2.0
    }

    pub fn draw(&mut self, game: &Game, sprites: &Sprites, assets: &Assets) {
        let (sw, sh) = (screen_width(), screen_height());
        let off = self.shake_offset();
        let t = get_time() as f32;
        let tell = tier_color(self.tell);
        let rays = self.ray_spec();
        self.bg.draw(t, rays.as_ref());

        match self.stage {
            Stage::Sealed => self.draw_sealed(assets, off, t),
            Stage::Burst => self.draw_burst(assets, off, t, tell),
            Stage::Showcase(i) => self.draw_showcase(game, sprites, assets, off, t, i),
            Stage::Done => {}
        }

        for p in &self.particles {
            let a = (p.life / p.max).clamp(0.0, 1.0);
            let s = p.size * (0.6 + a * 0.6);
            draw_rectangle(
                p.pos.x + off.x - s * 0.5,
                p.pos.y + off.y - s * 0.5,
                s,
                s,
                faded(p.color, a),
            );
        }
        if self.flash > 0.0 {
            draw_rectangle(0.0, 0.0, sw, sh, faded(WHITE, self.flash.min(1.0) * 0.85));
        }
    }

    /// The light the backdrop paints on the genome, when it paints it.
    fn ray_spec(&self) -> Option<Rays> {
        match self.stage {
            Stage::Burst if self.t >= BOOM => {
                let b = self.t - BOOM;
                let tell = tier_color(self.tell);
                Some(Rays {
                    center: vec2(0.5, 0.44),
                    color: rgb_of(tell),
                    alt: rgb_of(lighten(tell, 0.45)),
                    halo: (0.9 - b * 0.3).max(0.3),
                    burst: Some(b),
                })
            }
            Stage::Showcase(i) if self.opened[i].card.new => {
                let col = tier_color(self.opened[i].card.tier);
                let revealed = self.t >= REVEAL_AT;
                Some(Rays {
                    center: vec2(0.5, 0.36),
                    color: rgb_of(col),
                    alt: rgb_of(lighten(col, 0.45)),
                    halo: if revealed { 0.45 } else { 0.15 + self.t * 0.08 },
                    burst: None,
                })
            }
            _ => None,
        }
    }

    fn draw_sealed(&self, assets: &Assets, off: Vec2, t: f32) {
        let u = render::u();
        let (sw, sh) = (screen_width(), screen_height());
        let c = genome_center() + off;
        let n = self.taps;
        let charge = n as f32 / TAPS_TO_CRACK as f32;
        let pulse = 0.5 + 0.5 * (t * 3.0).sin();

        for i in 0..(10 + n * 8) {
            let f = (t * (0.25 + n as f32 * 0.08) + i as f32 * 0.137) % 1.0;
            let a = i as f32 * 2.4 + t * (0.8 + n as f32 * 0.4);
            let r = (1.0 - f) * (70.0 + (i % 4) as f32 * 8.0) * u;
            let p = c + vec2(a.cos() * r, a.sin() * r * 1.6);
            draw_rectangle(
                p.x,
                p.y,
                u * 1.4,
                u * 0.8,
                faded(rgb(BASES[i as usize % 4]), f * 0.8),
            );
        }

        let mutated = &MUT_ORDER[..(n * 4) as usize];
        let fresh = if n > 0 {
            &MUT_ORDER[((n - 1) * 4) as usize..(n * 4) as usize]
        } else {
            &[][..]
        };
        let flashing = self.mut_flash > 0.75;
        let rung = |f: f32, k: usize, half: bool| -> Color {
            let block = ((f * 12.0) as usize).min(11);
            if mutated.contains(&block) {
                if fresh.contains(&block) && flashing || half {
                    WHITE
                } else {
                    SEALED
                }
            } else {
                rgb(BASES[(k + if half { 2 } else { 0 }) % 4])
            }
        };
        let spin = 1.3 + n as f32 * 1.0 + self.mut_flash * 5.0;
        let amp = (16.0 + n as f32 * 1.5) * u;
        let len = helix_len();
        // Glitch: a chromatic split and torn bands.
        let glitch = self.mut_flash > 0.7;
        let seed = (t * 40.0) as u32;
        let jitter = |f: f32| -> f32 {
            if !glitch {
                return 0.0;
            }
            let band = (f * 20.0) as u32;
            let mut sd = band.wrapping_mul(2_654_435_761) ^ seed;
            if rnd(&mut sd) > 0.75 {
                (rnd(&mut sd) - 0.5) * 16.0 * u
            } else {
                0.0
            }
        };
        if glitch {
            let none = |_: f32| 0.0;
            draw_helix(
                c + vec2(-3.0 * u, 0.0),
                len,
                amp,
                3.2,
                t * spin,
                0.45,
                Some(rgb(0xFF3A7A)),
                &rung,
                &none,
            );
            draw_helix(
                c + vec2(3.0 * u, 0.0),
                len,
                amp,
                3.2,
                t * spin,
                0.45,
                Some(rgb(0x3AD8FF)),
                &rung,
                &none,
            );
        }
        draw_helix(c, len, amp, 3.2, t * spin, 1.0, None, &rung, &jitter);
        let cp = (t * 0.9) % 1.0;
        assets.glow(
            vec2(c.x, c.y + len * 0.5 - len * cp),
            u * (5.0 + n as f32 * 2.0),
            WHITE,
            0.25 + charge * 0.35,
        );

        if n > 0 && self.mut_flash > 0.0 {
            let k = ((1.0 - self.mut_flash) / 0.12).min(1.0);
            text_centered(
                "MUTATION!",
                sw * 0.5,
                sh * 0.13 - (1.0 - k) * u * 6.0,
                u * 13.0,
                faded(WHITE, self.mut_flash.min(1.0) * 2.0),
            );
        } else {
            text_centered(
                "A NEW GENOME HAS EVOLVED",
                sw * 0.5,
                sh * 0.13,
                fit_px("A NEW GENOME HAS EVOLVED", sw * 0.9, u * 7.0),
                TEXT_DIM,
            );
        }
        text_centered(
            "TAP TO MUTATE",
            sw * 0.5,
            sh * 0.79,
            u * 8.0,
            faded(TEXT, 0.6 + pulse * 0.4),
        );
    }

    fn draw_burst(&self, assets: &Assets, off: Vec2, t: f32, tell: Color) {
        let u = render::u();
        let (sw, sh) = (screen_width(), screen_height());
        let c = genome_center() + off;
        if self.t < BOOM {
            let k = ease_out_cubic(self.t / (BOOM - 0.07));
            draw_rectangle(0.0, 0.0, sw, sh, faded(BLACK, k * 0.5));
            assets.glow(c, u * (40.0 + k * 30.0), SEALED, 0.4 + k * 0.5);
            let len = helix_len() * (1.0 - k * 0.97);
            if len > u * 4.0 {
                let rung = |_: f32, _: usize, half: bool| if half { WHITE } else { SEALED };
                let none = |_: f32| 0.0;
                draw_helix(
                    c,
                    len,
                    18.0 * u * (1.0 - k * 0.8),
                    3.2 + k * 3.0,
                    t * 20.0,
                    1.0 - k * 0.2,
                    None,
                    &rung,
                    &none,
                );
            }
            let flicker = if self.t > BOOM - 0.07 {
                (t * 60.0).sin() * 2.0 * u
            } else {
                0.0
            };
            draw_circle(c.x, c.y, (3.0 + k * 9.0) * u + flicker, WHITE);
            for i in 0..40 {
                let a = i as f32 * 0.9 + t * 4.0;
                let d = (1.0 - ((t * 1.5 + i as f32 * 0.05) % 1.0)) * 90.0 * u;
                let p = c + vec2(a.cos(), a.sin()) * d;
                draw_rectangle(p.x, p.y, u, u, rgb(BASES[i % 4]));
            }
            return;
        }
        let b = self.t - BOOM;
        let k = ease_out_cubic(b / 0.8);
        if self.tell == Tier::Legendary {
            let w = sw * (0.2 + k * 0.3);
            draw_rectangle(
                c.x - w * 0.5,
                0.0,
                w,
                sh,
                faded(tell, 0.25 * (1.0 - b * 0.4).max(0.0)),
            );
            draw_rectangle(
                c.x - w * 0.15,
                0.0,
                w * 0.3,
                sh,
                faded(WHITE, 0.3 * (1.0 - b * 0.5).max(0.0)),
            );
        }
        assets.glow(c, u * 60.0, WHITE, (0.9 - b).max(0.0));
        let label = match self.tell {
            Tier::Legendary => "LEGENDARY!",
            Tier::Epic => "EPIC!",
            Tier::Rare => "RARE!",
            Tier::Uncommon | Tier::Common => "EVOLVED!",
        };
        let scale = 1.0 + 1.5 * (1.0 - ease_out_back((b / 0.35).min(1.0)));
        text_centered(label, sw * 0.5, sh * 0.2, u * 14.0 * scale.max(1.0), tell);
        // The cards fly out to their slots under the reveal, still empty.
        let n = self.opened.len();
        let (_, size) = slot_layout(n);
        for i in 0..n {
            let local = ((b - 0.3 - i as f32 * 0.07) / 0.5).clamp(0.0, 1.0);
            if local <= 0.0 {
                continue;
            }
            let p = c.lerp(slot_pos(i, n), ease_out_back(local));
            draw_empty_slot(p, size * (0.3 + 0.7 * local), false);
        }
    }

    fn draw_showcase(
        &self,
        game: &Game,
        sprites: &Sprites,
        assets: &Assets,
        off: Vec2,
        t: f32,
        i: usize,
    ) {
        let u = render::u();
        let (sw, sh) = (screen_width(), screen_height());
        let o = self.opened[i].clone();
        let taxon = o.card.taxon;
        let tx = game.taxon(taxon);
        let tier = o.card.tier;
        let col = tier_color(tier);
        let c = showcase_center() + off;
        let hint = self.next_hint();
        // Every card's slot, this one filling once its animal shows.
        let shown = self.t >= REVEAL_AT;
        self.draw_slots(game, sprites, i, shown);

        if !o.card.new {
            // A duplicate spins too, then turns into a specimen.
            if !shown {
                self.draw_roulette(game, sprites, taxon, i, c);
                return;
            }
            let since = self.t - REVEAL_AT;
            let k = ease_out_back((since / 0.35).min(1.0));
            assets.glow(c, sw * 0.35, col, 0.3);
            ui::draw_taxon(sprites, game, taxon, o.card.morph, c, sw * 0.32 * k, WHITE);
            text_centered(
                tx.name,
                sw * 0.5,
                sh * 0.52,
                fit_px(tx.name, sw * 0.9, u * 10.0),
                TEXT,
            );
            text_centered("+1 SPECIMEN", sw * 0.5, sh * 0.575, u * 8.0, LIME);
            let bar = Rect::new(sw * 0.2, sh * 0.6, sw * 0.6, u * 3.0);
            let (have, step) = game.level_progress(taxon);
            render::draw_meter(
                bar.x,
                bar.y,
                bar.w,
                bar.h,
                if step == 0 {
                    1.0
                } else {
                    have as f32 / step as f32 * ease_out_cubic(since / 0.6)
                },
                LIME,
            );
            let lv = if o.level_after > o.level_before {
                format!("LEVEL UP!  Lv {} -> {}", o.level_before, o.level_after)
            } else {
                format!("Lv {}", o.level_after)
            };
            text_centered(
                &lv,
                sw * 0.5,
                sh * 0.655,
                u * 7.0,
                if o.level_after > o.level_before {
                    GOLD
                } else {
                    TEXT_DIM
                },
            );
            if o.card.morph != Morph::None {
                let m = o.card.morph.name().to_uppercase();
                let label = if o.first_morph {
                    format!("NEW MORPH: {m}!")
                } else {
                    format!("{m} MORPH")
                };
                text_centered(&label, sw * 0.5, sh * 0.695, u * 8.0, PINK);
            }
            if let Some(note) = &o.card.note {
                for (k, line) in render::wrap_lines(note, sw * 0.84, u * 5.5)
                    .iter()
                    .enumerate()
                {
                    text_centered(
                        line,
                        sw * 0.5,
                        sh * 0.865 + k as f32 * u * 7.0,
                        u * 5.5,
                        GOLD,
                    );
                }
            }
            if self.t >= settled_at(false) {
                text_centered(&hint, sw * 0.5, sh * 0.95, u * 5.5, faded(TEXT_DIM, 0.8));
            }
            return;
        }

        let revealed = self.t >= REVEAL_AT;
        let sprite_size = sw * 0.42;
        if revealed {
            let k = ease_out_back(((self.t - REVEAL_AT) / 0.4).min(1.0));
            if o.card.morph == Morph::Amber || tier == Tier::Legendary {
                for (j, &h) in PRISM.iter().enumerate() {
                    let a = t * 0.8 + j as f32;
                    assets.glow(
                        c + vec2(a.cos(), a.sin()) * sw * 0.18,
                        sw * 0.12,
                        rgb(h),
                        0.25,
                    );
                }
            }
            ui::draw_taxon(
                sprites,
                game,
                taxon,
                o.card.morph,
                c,
                sprite_size * (0.6 + 0.4 * k),
                WHITE,
            );
        } else {
            self.draw_roulette(game, sprites, taxon, i, c);
        }

        let eco = tx.eco;
        let clues = [
            (
                "HABITAT",
                eco.needs
                    .habitats
                    .iter()
                    .map(|h| h.name())
                    .collect::<Vec<_>>()
                    .join(" / "),
            ),
            ("CLADE", tx.clade.to_string()),
            ("ORIGIN", tx.age_label()),
        ];
        let y0 = sh * 0.60;
        for (k, (label, value)) in clues.iter().enumerate() {
            // Held back until the roulette locks, so they don't give it away.
            let appear = REVEAL_AT - ROULETTE_HOLD + k as f32 * 0.12;
            if self.t < appear {
                continue;
            }
            let y = y0 + k as f32 * u * 9.0;
            render::text(label, sw * 0.14, y, u * 6.0, TEXT_DIM);
            let chars = (((self.t - appear) / 0.2).min(1.0) * value.len() as f32) as usize;
            let shown: String = value.chars().take(chars.max(1)).collect();
            let vw = render::text_width(&shown, u * 6.5);
            render::text(&shown, sw * 0.86 - vw, y, u * 6.5, TEXT);
        }

        if revealed {
            let k = ((self.t - REVEAL_AT) / 0.3).min(1.0);
            let stamp = 1.0 + 1.2 * (1.0 - ease_out_back(k));
            let label = if o.card.morph != Morph::None {
                format!("NEW  ·  {}", o.card.morph.name().to_uppercase())
            } else {
                "NEW SPECIES".to_string()
            };
            text_centered(&label, sw * 0.5, sh * 0.12, u * 9.0 * stamp, PINK);
            text_centered(
                tx.name,
                sw * 0.5,
                sh * 0.20,
                fit_px(tx.name, sw * 0.9, u * 12.0),
                TEXT,
            );
            text_centered(tier.name(), sw * 0.5, sh * 0.25, u * 7.5, col);
            let d = format!("Keystone: {}", eco.describe());
            text_centered(&d, sw * 0.5, sh * 0.86, fit_px(&d, sw * 0.9, u * 6.0), LIME);
            if self.t >= settled_at(true) {
                text_centered(&hint, sw * 0.5, sh * 0.92, u * 5.5, faded(TEXT_DIM, 0.8));
            }
        }
    }
}
