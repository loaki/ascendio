//! The spiral: one real root-to-tip lineage (the spine) on a logarithmic
//! coil, with every other child sprouting as a spur. Positions depend only
//! on `d`, the distance along the spine from the focus.

use macroquad::prelude::*;

use crate::game::Game;
use crate::tree::Phylogeny;

/// Angular step between consecutive generations. ~7 per revolution.
const DTHETA: f32 = 0.90;
/// Radial growth per generation; with DTHETA this gives ~3.3x per revolution.
const GROWTH: f32 = 1.185;
/// Angle of the focus on the coil: straight up.
const PHI: f32 = -std::f32::consts::FRAC_PI_2;

const D_BACK: f32 = -22.0;
const D_FWD: f32 = 3.5;

/// Within this many generations of the focus a taxon gets a readable label.
const LABEL_BAND: f32 = 2.6;
/// Only the focus's own spurs get names, or labels collide with the coil.
const SPUR_LABEL_BAND: f32 = 0.6;

/// Stays well inside the next winding, which is 3.3x out.
const SPUR_OUT: f32 = 1.9;
const SPUR_SPREAD: f32 = 0.55;

const SNAP_SPEED: f32 = 11.0;
const FLING_FRICTION: f32 = 4.5;
const FLING_STOP: f32 = 0.35;
/// Near the coil's centre a pixel of drag is an enormous angle, so the radius
/// used for that conversion is floored at this fraction of the coil radius.
const MIN_DRAG_RADIUS: f32 = 0.45;

/// A locked taxon is a leaf of the visible tree.
pub fn visible_children(game: &Game, i: usize) -> &[usize] {
    if game.unlocked[i] {
        &game.phy.taxa[i].children
    } else {
        &[]
    }
}

/// Which lineage the coil shows, and where along it we are.
pub struct Nav {
    /// Per taxon, the child the spine should continue through.
    preferred: Vec<Option<usize>>,
    /// Root to tip. `path[i + 1]` is always a child of `path[i]`.
    pub path: Vec<usize>,
    /// Taxon -> its generation on the spine.
    pub index: Vec<Option<usize>>,
    pub t: f32,
    pub t_target: f32,
    /// Generations per second while flinging.
    vel: f32,
    flinging: bool,
}

impl Nav {
    pub fn new(game: &Game) -> Self {
        let mut nav = Self {
            preferred: vec![None; game.phy.len()],
            path: Vec::new(),
            index: vec![None; game.phy.len()],
            t: 0.0,
            t_target: 0.0,
            vel: 0.0,
            flinging: false,
        };
        nav.rebuild(game);
        nav
    }

    pub fn rebuild(&mut self, game: &Game) {
        self.path.clear();
        self.index.iter_mut().for_each(|slot| *slot = None);

        let mut cur = Phylogeny::ROOT;
        loop {
            self.index[cur] = Some(self.path.len());
            self.path.push(cur);

            let kids = visible_children(game, cur);
            let next = match self.preferred[cur] {
                Some(c) if kids.contains(&c) => Some(c),
                _ => kids.first().copied(),
            };
            let Some(next) = next else { break };
            cur = next;
        }

        let last = self.last();
        self.t = self.t.clamp(0.0, last);
        self.t_target = self.t_target.clamp(0.0, last);
    }

    pub fn last(&self) -> f32 {
        self.path.len().saturating_sub(1) as f32
    }

    pub fn focus(&self) -> usize {
        self.path[self.t.round().clamp(0.0, self.last()) as usize]
    }

    /// Reroutes the spine through `taxon`, then scrolls to it.
    pub fn go_to(&mut self, game: &Game, taxon: usize) {
        let mut cur = taxon;
        while let Some(parent) = game.taxon(cur).parent {
            self.preferred[parent] = Some(cur);
            cur = parent;
        }
        self.rebuild(game);
        self.vel = 0.0;
        self.flinging = false;
        if let Some(gen) = self.index[taxon] {
            self.t_target = gen as f32;
        }
    }

    /// Scrolls by turning the coil about its centre, so what is under the
    /// finger stays there. A vertical drag is ambiguous: both neighbours of
    /// the focus sit below it.
    pub fn drag_around(&mut self, coil_center: Vec2, from: Vec2, to: Vec2, k: f32, dt: f32) {
        let arm = from - coil_center;
        let len = arm.length();
        if len < 1.0 {
            return;
        }

        let tangent = Vec2::new(-arm.y, arm.x) / len;
        let swept = (to - from).dot(tangent) / len.max(k * MIN_DRAG_RADIUS);
        let generations = swept / DTHETA;

        // Turning toward descendants carries the focus outward: t decreases.
        self.t = (self.t - generations).clamp(0.0, self.last());
        self.t_target = self.t;

        if dt > 0.0 {
            // Smoothed, so one jittery frame cannot define the fling.
            self.vel = self.vel * 0.6 + (-generations / dt) * 0.4;
        }
        self.flinging = false;
    }

    pub fn release(&mut self) {
        self.flinging = self.vel.abs() > FLING_STOP;
        if !self.flinging {
            self.snap();
        }
    }

    pub fn step(&mut self, generations: f32) {
        self.vel = 0.0;
        self.flinging = false;
        self.t_target = (self.t_target + generations)
            .round()
            .clamp(0.0, self.last());
    }

    pub fn snap(&mut self) {
        self.vel = 0.0;
        self.flinging = false;
        self.t_target = self.t.round().clamp(0.0, self.last());
    }

    pub fn settle(&mut self, dt: f32) {
        let last = self.last();

        if self.flinging {
            self.t += self.vel * dt;
            self.vel *= (-FLING_FRICTION * dt).exp();

            if self.t <= 0.0 || self.t >= last {
                self.t = self.t.clamp(0.0, last);
                self.snap();
            } else if self.vel.abs() < FLING_STOP {
                self.snap();
            }
            return;
        }

        self.t += (self.t_target - self.t) * (1.0 - (-SNAP_SPEED * dt).exp());
        self.t = self.t.clamp(0.0, last);
    }
}

// --- screen regions ---------------------------------------------------------

pub fn focus_point() -> Vec2 {
    Vec2::new(screen_width() * 0.5, screen_height() * 0.44)
}

/// Distance from the focus to the point the coil tightens onto.
pub fn coil_radius() -> f32 {
    (screen_width() * 0.30).min(screen_height() * 0.11)
}

pub fn label_px(k: f32) -> f32 {
    k * 0.17
}

fn unit(angle: f32) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin())
}

/// Screen position at generation offset `d`, `turn` radians around the coil
/// from the spine and `out` times as far from the coil's centre.
fn polar(d: f32, turn: f32, out: f32, center: Vec2, k: f32) -> Vec2 {
    center + (unit(d * DTHETA + PHI + turn) * (GROWTH.powf(d) * out) - unit(PHI)) * k
}

/// The limit of `point_at` as `d` goes to minus infinity.
pub fn coil_center() -> Vec2 {
    focus_point() - unit(PHI) * coil_radius()
}

pub fn point_at(d: f32, center: Vec2, k: f32) -> Vec2 {
    polar(d, 0.0, 1.0, center, k)
}

pub fn spur_at(d: f32, j: usize, n: usize, center: Vec2, k: f32) -> Vec2 {
    let turn = (j as f32 - (n as f32 - 1.0) * 0.5) * SPUR_SPREAD;
    polar(d, turn, SPUR_OUT, center, k)
}

/// Relative size at offset `d`: largest at the focus.
pub fn scale_at(d: f32) -> f32 {
    1.0 / (1.0 + 0.25 * d.abs())
}

pub fn fade_at(d: f32) -> f32 {
    let f = if d < 0.0 {
        1.0 - (d / D_BACK).powi(2)
    } else {
        1.0 - (d / D_FWD).powi(3)
    };
    f.clamp(0.0, 1.0)
}

pub fn bead_radius(k: f32, scale: f32) -> f32 {
    (k * 0.045 * scale).max(1.5)
}

// --- per-frame layout -------------------------------------------------------

/// A taxon on the spine.
pub struct Bead {
    pub taxon: usize,
    pub d: f32,
    pub pos: Vec2,
    pub scale: f32,
    /// `None` far from the focus: drawn as a dot.
    pub label: Option<String>,
    /// Half-extent of the tappable box around `pos`.
    pub half: Vec2,
}

/// A child the spine does not continue through.
pub struct Spur {
    pub taxon: usize,
    pub d: f32,
    pub from: Vec2,
    pub pos: Vec2,
    pub half: Vec2,
    pub label: Option<String>,
}

pub struct Frame {
    pub beads: Vec<Bead>,
    pub spurs: Vec<Spur>,
    pub focus: usize,
    pub center: Vec2,
    pub k: f32,
}

impl Frame {
    /// `measure` gives the rendered width of a label at a given pixel size.
    pub fn build(game: &Game, nav: &Nav, measure: &dyn Fn(&str, f32) -> f32) -> Self {
        let center = focus_point();
        let k = coil_radius();
        let (t, last) = (nav.t, nav.last());
        let px = label_px(k);

        // The `1.3` icon reservation must match `render::CHIP_ICON`.
        let chip = |text: &str, scale: f32| -> Vec2 {
            let p = px * scale;
            let icon = p * 1.3;
            Vec2::new((measure(text, p) + icon) * 0.5 + p * 0.5, p * 0.92)
        };
        // A long name on the outer coil would otherwise run off the edge.
        let on_screen = |pos: Vec2, half: Vec2| -> Vec2 {
            let m = half.x + k * 0.03;
            Vec2::new(pos.x.clamp(m, (screen_width() - m).max(m)), pos.y)
        };

        let lo = (t + D_BACK).max(0.0).floor() as usize;
        let hi = (t + D_FWD).min(last).ceil() as usize;

        let mut beads = Vec::with_capacity(hi - lo + 1);
        let mut spurs = Vec::new();

        for gen in lo..=hi {
            let d = gen as f32 - t;
            let taxon = nav.path[gen];
            let scale = scale_at(d);
            let pos = point_at(d, center, k);

            let (label, half) = if d.abs() <= LABEL_BAND {
                let text = chip_label(game, taxon);
                let half = chip(&text, scale);
                (Some(text), half)
            } else {
                (None, Vec2::splat(bead_radius(k, scale) * 2.6))
            };
            let pos = if label.is_some() {
                on_screen(pos, half)
            } else {
                pos
            };
            beads.push(Bead {
                taxon,
                d,
                pos,
                scale,
                label,
                half,
            });

            let continues_to = nav.path.get(gen + 1).copied();
            let branches: Vec<usize> = visible_children(game, taxon)
                .iter()
                .copied()
                .filter(|&c| Some(c) != continues_to)
                .collect();

            let spur_scale = scale * 0.92;
            for (j, &c) in branches.iter().enumerate() {
                let raw = spur_at(d, j, branches.len(), center, k);
                let (label, half) = if d.abs() <= SPUR_LABEL_BAND {
                    let text = chip_label(game, c);
                    let half = chip(&text, spur_scale);
                    (Some(text), half)
                } else {
                    (None, Vec2::splat(bead_radius(k, spur_scale * 0.8) * 2.6))
                };
                let pos = if label.is_some() {
                    on_screen(raw, half)
                } else {
                    raw
                };
                spurs.push(Spur {
                    taxon: c,
                    d,
                    from: point_at(d, center, k),
                    pos,
                    half,
                    label,
                });
            }
        }

        Self {
            beads,
            spurs,
            focus: nav.focus(),
            center,
            k,
        }
    }

    /// The taxon a tap at `p` landed on.
    pub fn hit(&self, p: Vec2) -> Option<usize> {
        // Spurs sit on top of the coil, so they win ties.
        let mut spurs: Vec<&Spur> = self.spurs.iter().collect();
        spurs.sort_by(|a, b| a.d.abs().partial_cmp(&b.d.abs()).unwrap());
        if let Some(s) = spurs.into_iter().find(|s| inside(p, s.pos, s.half)) {
            return Some(s.taxon);
        }

        let mut beads: Vec<&Bead> = self.beads.iter().collect();
        beads.sort_by(|a, b| a.d.abs().partial_cmp(&b.d.abs()).unwrap());
        beads
            .into_iter()
            .find(|b| inside(p, b.pos, b.half))
            .map(|b| b.taxon)
    }
}

fn inside(p: Vec2, center: Vec2, half: Vec2) -> bool {
    (p.x - center.x).abs() <= half.x && (p.y - center.y).abs() <= half.y
}

fn chip_label(game: &Game, taxon: usize) -> String {
    if !game.unlocked[taxon] {
        return "???".to_string();
    }
    let name = game.taxon(taxon).name;
    match game.level[taxon] {
        0 | 1 => name.to_string(),
        lv => format!("{name} ·{lv}"),
    }
}

/// The coil sampled for drawing, as `(point, d)` pairs.
pub fn curve(center: Vec2, k: f32, t: f32, last: f32) -> Vec<(Vec2, f32)> {
    let lo = (-t).max(D_BACK);
    let hi = (last - t).min(D_FWD);
    let step = 0.04;

    let n = ((hi - lo) / step).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| {
            let d = (lo + i as f32 * step).min(hi);
            (point_at(d, center, k), d)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlock_all(game: &mut Game) {
        for i in 0..game.phy.len() {
            game.unlocked[i] = true;
            game.level[i] = game.level[i].max(1);
        }
    }

    #[test]
    fn every_step_along_the_spine_is_a_real_parent_child_edge() {
        let mut game = Game::new(0.0);
        unlock_all(&mut game);
        let mut nav = Nav::new(&game);

        for target in 0..game.phy.len() {
            nav.go_to(&game, target);
            for pair in nav.path.windows(2) {
                assert_eq!(
                    game.taxon(pair[1]).parent,
                    Some(pair[0]),
                    "{} is not a child of {}",
                    game.taxon(pair[1]).name,
                    game.taxon(pair[0]).name,
                );
            }
        }
    }

    #[test]
    fn go_to_puts_the_taxon_on_the_spine_and_scrolls_to_it() {
        let mut game = Game::new(0.0);
        unlock_all(&mut game);
        let mut nav = Nav::new(&game);

        let human = game
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Human")
            .unwrap();
        nav.go_to(&game, human);
        assert_eq!(nav.index[human], Some(nav.t_target as usize));
        assert!(nav.path.contains(&human));
    }

    #[test]
    fn sister_clades_never_appear_as_ancestor_and_descendant() {
        let mut game = Game::new(0.0);
        unlock_all(&mut game);
        let mut nav = Nav::new(&game);

        let find = |n: &str| game.phy.taxa.iter().position(|t| t.name == n).unwrap();
        let (cnidarian, protostome) = (find("Cnidarian"), find("Protostome"));

        for target in [cnidarian, protostome] {
            nav.go_to(&game, target);
            let on_spine = nav.path.contains(&cnidarian) && nav.path.contains(&protostome);
            assert!(!on_spine, "sister clades ended up on one lineage");
        }
    }

    #[test]
    fn a_fresh_spine_is_the_root_and_one_locked_child() {
        let game = Game::new(0.0);
        let nav = Nav::new(&game);
        assert_eq!(nav.path[0], Phylogeny::ROOT);
        assert_eq!(
            nav.path.len(),
            2,
            "root plus the child it continues through"
        );
        assert_eq!(nav.focus(), Phylogeny::ROOT);
    }

    #[test]
    fn the_focus_sits_exactly_at_the_focus_point() {
        let center = Vec2::new(200.0, 400.0);
        assert!(point_at(0.0, center, 300.0).distance(center) < 0.001);
    }

    #[test]
    fn the_coil_tightens_toward_ancestors_and_opens_toward_descendants() {
        let (center, k) = (Vec2::ZERO, 300.0);
        let back = point_at(-5.0, center, k).distance(point_at(-6.0, center, k));
        let near = point_at(0.0, center, k).distance(point_at(-1.0, center, k));
        let fwd = point_at(5.0, center, k).distance(point_at(6.0, center, k));
        assert!(back < near, "{back} !< {near}");
        assert!(fwd > near, "{fwd} !> {near}");
    }

    #[test]
    fn spurs_sit_clear_of_the_coil_they_branch_from() {
        let (center, k) = (Vec2::ZERO, 300.0);
        for d in [-2.0, 0.0, 2.0] {
            let on_coil = point_at(d, center, k);
            let spur = spur_at(d, 0, 1, center, k);
            assert!(spur.distance(on_coil) > k * 0.3, "spur too close at d={d}");
        }
    }

    /// A drag from the focus bead, `px` along the coil toward descendants.
    fn sweep(nav: &mut Nav, px: f32, dt: f32) {
        let (c, k) = (Vec2::ZERO, 160.0);
        let from = c + Vec2::new(PHI.cos(), PHI.sin()) * k;
        let arm = from - c;
        let tangent = Vec2::new(-arm.y, arm.x).normalize();
        nav.drag_around(c, from, from + tangent * px, k, dt);
    }

    fn mid_spine() -> (Game, Nav) {
        let mut game = Game::new(0.0);
        unlock_all(&mut game);
        let mut nav = Nav::new(&game);
        let human = game
            .phy
            .taxa
            .iter()
            .position(|t| t.name == "Human")
            .unwrap();
        nav.go_to(&game, human);
        nav.t = nav.t_target;
        (game, nav)
    }

    #[test]
    fn dragging_the_coil_round_carries_the_lineage_with_the_finger() {
        let (_game, mut nav) = mid_spine();
        let before = nav.t;
        sweep(&mut nav, 40.0, 1.0 / 60.0);
        assert!(nav.t < before, "{} !< {before}", nav.t);

        let back = nav.t;
        sweep(&mut nav, -40.0, 1.0 / 60.0);
        assert!(
            nav.t > back,
            "reversing the drag did not reverse the scroll"
        );
    }

    #[test]
    fn dragging_near_the_coil_centre_does_not_jump() {
        let (_game, mut nav) = mid_spine();
        let before = nav.t;
        let (c, k) = (Vec2::ZERO, 160.0);
        let from = c + Vec2::new(2.0, 0.0);
        nav.drag_around(c, from, from + Vec2::new(0.0, 30.0), k, 1.0 / 60.0);
        assert!(
            (nav.t - before).abs() < 1.0,
            "jumped {}",
            (nav.t - before).abs()
        );
    }

    #[test]
    fn a_fling_decays_and_settles_on_a_whole_generation() {
        let (_game, mut nav) = mid_spine();
        let dt = 1.0 / 60.0;
        for _ in 0..6 {
            sweep(&mut nav, 55.0, dt);
        }
        nav.release();
        assert!(nav.flinging, "a fast drag should fling");

        for _ in 0..600 {
            nav.settle(dt);
            if !nav.flinging && (nav.t - nav.t_target).abs() < 0.001 {
                break;
            }
        }
        assert!(!nav.flinging, "fling never stopped");
        assert!(
            (nav.t - nav.t.round()).abs() < 0.01,
            "settled off-grid at {}",
            nav.t
        );
    }

    #[test]
    fn a_slow_drag_snaps_without_flinging() {
        let (_game, mut nav) = mid_spine();
        sweep(&mut nav, 0.4, 1.0 / 60.0);
        nav.release();
        assert!(!nav.flinging, "a crawl should not fling");
    }

    #[test]
    fn the_focus_is_the_largest_bead() {
        assert!(scale_at(0.0) > scale_at(1.0));
        assert!(scale_at(0.0) > scale_at(-1.0));
    }
}
