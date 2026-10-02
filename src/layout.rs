//! Tidy tree layout for the map, root at the left. Subtree heights are
//! measured bottom-up, then nodes are placed in the space reserved.

use macroquad::prelude::Vec2;

use crate::game::Game;
use crate::spiral::visible_children;
use crate::tree::Phylogeny;

const NODE_H: f32 = 44.0;
/// Horizontal distance between generations. Must clear the widest label.
const LEVEL_W: f32 = 215.0;
const V_GAP: f32 = 13.0;
const MIN_NODE_W: f32 = 130.0;
/// Room for the icon plus "? ? ?".
const LOCKED_NODE_W: f32 = ICON_BUDGET + 46.0;
const PAD_X: f32 = 18.0;
/// Must match the icon geometry in `render::draw_nodes`.
const ICON_BUDGET: f32 = NODE_H * 1.14;

#[derive(Clone, Copy, Default)]
pub struct NodeBox {
    pub center: Vec2,
    pub w: f32,
    pub h: f32,
}

impl NodeBox {
    pub fn contains(&self, p: Vec2) -> bool {
        (p.x - self.center.x).abs() <= self.w * 0.5 && (p.y - self.center.y).abs() <= self.h * 0.5
    }

    pub fn left(&self) -> Vec2 {
        Vec2::new(self.center.x - self.w * 0.5, self.center.y)
    }

    pub fn right(&self) -> Vec2 {
        Vec2::new(self.center.x + self.w * 0.5, self.center.y)
    }
}

pub struct Layout {
    /// `None` for taxa that are not currently visible.
    pub boxes: Vec<Option<NodeBox>>,
    pub min: Vec2,
    pub max: Vec2,
}

impl Layout {
    pub fn get(&self, i: usize) -> Option<NodeBox> {
        self.boxes[i]
    }

    pub fn hit(&self, p: Vec2) -> Option<usize> {
        self.boxes
            .iter()
            .position(|b| b.is_some_and(|b| b.contains(p)))
    }
}

/// `measure_label` returns the rendered width of a taxon name in world units.
pub fn compute(game: &Game, measure_label: &dyn Fn(&str) -> f32) -> Layout {
    let n = game.phy.len();
    let mut widths = vec![0.0f32; n];
    let mut spans = vec![0.0f32; n];
    let mut boxes = vec![None; n];

    measure(
        game,
        Phylogeny::ROOT,
        measure_label,
        &mut widths,
        &mut spans,
    );
    place(game, Phylogeny::ROOT, 0.0, &widths, &spans, &mut boxes);

    let (mut min, mut max) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for b in boxes.iter().flatten() {
        min = min.min(b.center - Vec2::new(b.w, b.h) * 0.5);
        max = max.max(b.center + Vec2::new(b.w, b.h) * 0.5);
    }

    Layout { boxes, min, max }
}

/// Fills each node's box width and its subtree's vertical span.
fn measure(
    game: &Game,
    i: usize,
    measure_label: &dyn Fn(&str) -> f32,
    widths: &mut [f32],
    spans: &mut [f32],
) {
    widths[i] = if game.unlocked[i] || game.is_fossil(i) {
        (measure_label(game.taxon(i).name) + PAD_X + ICON_BUDGET).max(MIN_NODE_W)
    } else {
        LOCKED_NODE_W
    };

    let kids = visible_children(game, i);
    if kids.is_empty() {
        spans[i] = NODE_H;
        return;
    }

    let mut total = 0.0;
    for (k, &c) in kids.iter().enumerate() {
        measure(game, c, measure_label, widths, spans);
        total += spans[c];
        if k > 0 {
            total += V_GAP;
        }
    }
    spans[i] = total.max(NODE_H);
}

/// Places the subtree rooted at `i` with its vertical band starting at `top`.
fn place(
    game: &Game,
    i: usize,
    top: f32,
    widths: &[f32],
    spans: &[f32],
    boxes: &mut [Option<NodeBox>],
) {
    let x = game.taxon(i).depth as f32 * LEVEL_W + widths[i] * 0.5;
    let kids = visible_children(game, i);

    let cy = if kids.is_empty() {
        top + spans[i] * 0.5
    } else {
        let children_span: f32 =
            kids.iter().map(|&c| spans[c]).sum::<f32>() + V_GAP * (kids.len() - 1) as f32;

        let mut cursor = top + (spans[i] - children_span) * 0.5;
        for &c in kids {
            place(game, c, cursor, widths, spans, boxes);
            cursor += spans[c] + V_GAP;
        }

        let first = boxes[kids[0]].unwrap().center.y;
        let last = boxes[*kids.last().unwrap()].unwrap().center.y;
        (first + last) * 0.5
    };

    boxes[i] = Some(NodeBox {
        center: Vec2::new(x, cy),
        w: widths[i],
        h: NODE_H,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_width(_: &str) -> f32 {
        80.0
    }

    #[test]
    fn siblings_never_overlap() {
        let mut game = Game::new(0.0);
        for i in 0..game.phy.len() {
            game.unlocked[i] = true;
            game.level[i] = game.level[i].max(1);
        }
        let layout = compute(&game, &fixed_width);

        for depth in 0..20u8 {
            let mut column: Vec<NodeBox> = (0..game.phy.len())
                .filter(|&i| game.taxon(i).depth == depth)
                .filter_map(|i| layout.get(i))
                .collect();
            column.sort_by(|a, b| a.center.y.total_cmp(&b.center.y));
            for pair in column.windows(2) {
                let gap =
                    (pair[1].center.y - pair[1].h * 0.5) - (pair[0].center.y + pair[0].h * 0.5);
                assert!(gap >= -0.01, "overlap at depth {depth}: gap {gap}");
            }
        }
    }

    #[test]
    fn generations_advance_to_the_right() {
        let mut game = Game::new(0.0);
        for i in 0..game.phy.len() {
            game.unlocked[i] = true;
            game.level[i] = game.level[i].max(1);
        }
        let layout = compute(&game, &fixed_width);
        for i in 0..game.phy.len() {
            let Some(parent) = game.taxon(i).parent else {
                continue;
            };
            let (child, parent) = (layout.get(i).unwrap(), layout.get(parent).unwrap());
            assert!(
                child.left().x > parent.right().x,
                "generations not separated"
            );
        }
    }

    #[test]
    fn a_fresh_game_lays_out_the_root_and_its_locked_children() {
        let game = Game::new(0.0);
        let layout = compute(&game, &fixed_width);
        let visible = layout.boxes.iter().filter(|b| b.is_some()).count();
        assert_eq!(visible, 1 + game.phy.taxa[Phylogeny::ROOT].children.len());
    }
}
