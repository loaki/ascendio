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
/// The tutorial cycles (1 and 20 minutes) pay like a wait this long.
pub const TUTORIAL_HOURS: f32 = 3.0;
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
    /// One card per hour; the fraction is the chance of one more.
    pub cards: f32,
    /// Rarity luck, growing with the square of the time past the minimum.
    pub luck: f32,
    pub morph_mult: f32,
    pub sure_rare: bool,
}

pub fn bonus(hours: f32) -> Bonus {
    let past = (hours - MIN_HOURS).max(0.0);
    Bonus {
        cards: hours,
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
    fn the_dial_snaps_to_half_hours_in_range() {
        assert_eq!(snap(3.3), 3.5);
        assert_eq!(snap(0.0), MIN_HOURS);
        assert_eq!(snap(9.0), MAX_HOURS);
        assert_eq!(ma(4.5), 45);
    }
}
