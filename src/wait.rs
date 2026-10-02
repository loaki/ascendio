//! How long the player lets time run (2 to 6 hours, chosen on the dial) and
//! what each length is worth. Waiting longer pays more per hour, so one
//! 6-hour wait always beats two 3-hour ones: the same number of cards, but
//! with better odds, more morphs and a guaranteed Rare.

pub const MIN_HOURS: f32 = 2.0;
pub const MAX_HOURS: f32 = 6.0;
/// The dial moves in half-hour steps.
pub const STEP_HOURS: f32 = 0.5;
pub const DEFAULT_HOURS: f32 = 4.0;
pub const MA_PER_HOUR: f32 = 10.0;
/// The tutorial cycles (1 and 20 minutes) pay like a wait this long...
pub const TUTORIAL_HOURS: f32 = 3.0;
/// ...but with this many cards, so the first genomes fill the tree.
pub const TUTORIAL_CARDS: f32 = 3.0;
/// Cards at 2h, 4h and 6h; half hours in between are interpolated, the
/// fraction being the chance of one more.
const CARDS_AT: [(f32, f32); 3] = [(2.0, 1.0), (4.0, 2.0), (6.0, 5.0)];
/// Adjustment points for the next shaping at 2h, 4h and 6h; half hours in
/// between round down.
const POINTS_AT: [(f32, f32); 3] = [(2.0, 1.0), (4.0, 2.0), (6.0, 3.0)];
/// From this long, morphs are 1.5x as likely.
pub const MORPH_HOURS: f32 = 4.0;
/// A wait this long guarantees one Rare-or-better card and doubles morphs.
pub const SURE_RARE_HOURS: f32 = 6.0;

/// Clamps to the dial's range and rounds to its step.
pub fn snap(hours: f32) -> f32 {
    ((hours / STEP_HOURS).round() * STEP_HOURS).clamp(MIN_HOURS, MAX_HOURS)
}

/// Millions of years that pass during a wait.
pub fn ma(hours: f32) -> u32 {
    (hours * MA_PER_HOUR).round() as u32
}

/// What a wait adds to the genome it produces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bonus {
    /// Base cards (`CARDS_AT`); the fraction is the chance of one more.
    pub cards: f32,
    /// Rarity luck, growing with the square of the time past the minimum.
    pub luck: f32,
    pub morph_mult: f32,
    pub sure_rare: bool,
}

/// `table` read at `hours`, linear between its points.
fn lerp(table: &[(f32, f32)], hours: f32) -> f32 {
    let h = hours.clamp(MIN_HOURS, MAX_HOURS);
    table
        .windows(2)
        .find(|w| h <= w[1].0)
        .map(|w| {
            let ((h0, c0), (h1, c1)) = (w[0], w[1]);
            c0 + (c1 - c0) * (h - h0) / (h1 - h0)
        })
        .unwrap_or(table[table.len() - 1].1)
}

fn cards(hours: f32) -> f32 {
    lerp(&CARDS_AT, hours)
}

/// Adjustment points a finished wait of `hours` gives the next shaping.
pub fn points(hours: f32) -> u8 {
    lerp(&POINTS_AT, hours).floor() as u8
}

pub fn bonus(hours: f32) -> Bonus {
    let past = (hours - MIN_HOURS).max(0.0);
    Bonus {
        cards: cards(hours),
        luck: 0.75 * past * past,
        morph_mult: if hours >= SURE_RARE_HOURS {
            2.0
        } else if hours >= MORPH_HOURS {
            1.5
        } else {
            1.0
        },
        sure_rare: hours >= SURE_RARE_HOURS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome;

    /// Expected Rare-or-better cards and morphs from one genome.
    fn expected(hours: f32) -> (f32, f32) {
        let b = bonus(hours);
        let w = genome::tier_weights(b.luck);
        let rare = b.cards * (w[2] + w[3] + w[4]) / 100.0;
        let rare = if b.sure_rare { rare.max(1.0) } else { rare };
        (rare, b.cards * genome::morph_chance(b.morph_mult))
    }

    #[test]
    fn one_long_wait_beats_two_short_ones() {
        let (rare6, morph6) = expected(6.0);
        let (rare3, morph3) = expected(3.0);
        assert!(bonus(6.0).cards >= 2.0 * bonus(3.0).cards);
        assert!(rare6 > 2.0 * rare3, "{rare6} vs {}", 2.0 * rare3);
        assert!(morph6 > 2.0 * morph3, "{morph6} vs {}", 2.0 * morph3);
        let (rare4, _) = expected(4.0);
        let (rare2, _) = expected(2.0);
        assert!(rare4 > 2.0 * rare2);
    }

    #[test]
    fn every_extra_half_hour_is_worth_more_than_the_last() {
        let mut prev = expected(MIN_HOURS).0 / MIN_HOURS;
        let mut h = MIN_HOURS + STEP_HOURS;
        while h <= MAX_HOURS {
            let per_hour = expected(h).0 / h;
            assert!(per_hour > prev, "{h}h pays {per_hour}/h, less than {prev}");
            prev = per_hour;
            h += STEP_HOURS;
        }
    }

    #[test]
    fn cards_follow_the_wait() {
        assert_eq!(bonus(2.0).cards, 1.0);
        assert_eq!(bonus(3.0).cards, 1.5);
        assert_eq!(bonus(4.0).cards, 2.0);
        assert_eq!(bonus(5.0).cards, 3.5);
        assert_eq!(bonus(6.0).cards, 5.0);
    }

    #[test]
    fn points_follow_the_wait() {
        let at: Vec<u8> = [2.0, 3.0, 4.0, 5.0, 5.5, 6.0].map(points).to_vec();
        assert_eq!(at, [1, 1, 2, 2, 2, 3]);
    }

    #[test]
    fn the_dial_snaps_to_half_hours_in_range() {
        assert_eq!(snap(3.3), 3.5);
        assert_eq!(snap(0.0), MIN_HOURS);
        assert_eq!(snap(9.0), MAX_HOURS);
        assert_eq!(ma(4.5), 45);
    }
}
