//! Game state tests.
use super::*;

fn run_cycle(g: &mut Game, now: &mut f64) -> Vec<Opened> {
    assert!(g.accelerate(*now));
    *now += g.cycle.unwrap().duration + 1.0;
    g.tick(*now);
    assert_eq!(g.phase(), Phase::Genome);
    let opened = g.open_genome(*now);
    assert_eq!(g.phase(), Phase::Boon);
    g.choose_boon(0);
    assert_eq!(g.phase(), Phase::Shape);
    opened
}

#[test]
fn a_new_game_starts_shaping_with_one_animal() {
    let g = Game::new(0.0);
    assert_eq!(g.phase(), Phase::Shape);
    assert_eq!(g.discovered(), 1);
    assert_eq!(g.level[Phylogeny::ROOT], 1);
    assert_eq!(g.points_left(), planet::BASE_POINTS);
}

#[test]
fn points_are_spent_by_distance_and_refunded_by_stepping_back() {
    let mut g = Game::new(0.0);
    assert!(g.step_lever(Lever::Oxygen, 1));
    assert!(g.step_lever(Lever::Oxygen, 1));
    assert_eq!(g.points_left(), 1);
    assert!(g.step_lever(Lever::Oxygen, -1));
    assert_eq!(g.points_left(), 2, "stepping back refunds");
    assert!(g.step_lever(Lever::Land, 1));
    assert!(g.step_lever(Lever::Temperature, 1));
    assert!(!g.step_lever(Lever::Temperature, 1), "out of points");
}

#[test]
fn nothing_changes_while_time_runs() {
    let mut g = Game::new(0.0);
    assert!(g.accelerate(0.0));
    assert_eq!(g.phase(), Phase::Running);
    assert!(!g.step_lever(Lever::Oxygen, 1));
    assert!(!g.accelerate(1.0));
    g.tick(30.0);
    assert_eq!(g.phase(), Phase::Running, "the first cycle lasts a minute");
}

#[test]
fn the_first_cycle_discovers_the_roots_children() {
    let mut g = Game::new(0.0);
    g.step_lever(Lever::Oxygen, 1);
    let mut now = 0.0;
    let opened = run_cycle(&mut g, &mut now);
    assert_eq!(opened.len(), 3);
    assert!(opened.iter().all(|o| o.card.new));
    assert_eq!(g.discovered(), 4);
    assert!(g.boon_offer.is_none());
}

#[test]
fn a_long_game_progresses_and_levels_up_duplicates() {
    let mut g = Game::new(0.0);
    let mut now = 0.0;
    let target = Planet {
        land: 2,
        vegetation: 3,
        oxygen: 4,
        temperature: 3,
        volcanism: 0,
    };
    for _ in 0..80 {
        for l in Lever::ALL {
            while g.planet.get(l) < target.get(l) && g.step_lever(l, 1) {}
            while g.planet.get(l) > target.get(l) && g.step_lever(l, -1) {}
        }
        run_cycle(&mut g, &mut now);
    }
    assert!(g.discovered() > 25, "only {} found", g.discovered());
    assert!(
        (0..g.phy.len()).any(|i| g.level[i] >= 2),
        "duplicates should level"
    );
}

#[test]
fn keystones_are_limited_and_editable_until_the_genome_is_ready() {
    let mut g = Game::new(0.0);
    for i in 1..6 {
        g.unlocked[i] = true;
        g.specimens[i] = 1;
    }
    g.refresh_levels();
    for i in 1..5 {
        g.toggle_keystone(i);
    }
    assert_eq!(g.keystones.len(), 3);
    assert!(g.toggle_keystone(1), "unequip");
    g.accelerate(0.0);
    assert!(g.toggle_keystone(5), "still editable while time runs");
    assert_eq!(
        g.active_keystones(),
        vec![2, 3],
        "the launch snapshot counts"
    );
    g.skip_cycle(0.0);
    assert_eq!(g.phase(), Phase::Genome);
    assert!(!g.toggle_keystone(5), "locked once the genome is ready");
}

#[test]
fn a_point_keystone_earns_a_point_per_4h_waited() {
    let mut g = Game::new(0.0);
    let bear = equip(&mut g, "Bear");
    assert!(g.dormant_reason(bear).is_some(), "no forest on a sea world");
    g.earn_keystone_points(4.0);
    assert_eq!(
        g.max_points(),
        planet::BASE_POINTS,
        "a dormant keystone gives nothing"
    );
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 1,
        volcanism: 0,
    };
    assert_eq!(g.dormant_reason(bear), None);
    assert_eq!(g.max_points(), planet::BASE_POINTS, "earned by waiting");
    g.earn_keystone_points(4.0);
    assert_eq!(g.max_points(), planet::BASE_POINTS + 1);
}

#[test]
fn short_waits_earn_keystone_points_no_faster_than_long_ones() {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 1,
        volcanism: 0,
    };
    equip(&mut g, "Bear");
    let mut short = 0;
    for _ in 0..3 {
        g.earn_keystone_points(2.0);
        short += g.bonus_points;
    }
    g.point_carry = 0.0;
    g.earn_keystone_points(6.0);
    // 3 x 2h: 0 + 1 + 0 (the halves carry over); 6h: 1 and a half carried.
    assert_eq!((short, g.bonus_points), (1, 1));
    assert_eq!(g.point_carry, 0.5);
}

#[test]
fn levels_follow_the_specimen_steps() {
    assert_eq!(level_for(1), 1);
    assert_eq!(level_for(2), 2);
    assert_eq!(level_for(4), 3);
    assert_eq!(level_for(8), 4);
    assert_eq!(level_for(16), 5);
    assert_eq!(level_for(999), MAX_LEVEL);
}

#[test]
fn save_round_trips() {
    let mut g = Game::new(10.0);
    g.step_lever(Lever::Oxygen, 1);
    g.accelerate(10.0);
    let back = Game::from_json(&g.to_json(), 20.0).expect("save should load");
    assert_eq!(back.phase(), Phase::Running);
    assert_eq!(back.planet, g.planet);
    assert!(
        Game::from_json("{\"dna\": 5}", 0.0).is_none(),
        "old saves are discarded"
    );
}

#[test]
fn a_save_naming_taxa_past_the_tree_is_rejected() {
    let mut g = Game::new(0.0);
    let mut now = 0.0;
    run_cycle(&mut g, &mut now);
    g.accelerate(now);
    now += g.cycle.unwrap().duration + 1.0;
    g.tick(now);
    let v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    assert!(Game::from_json(&v.to_string(), now).is_some());
    let mut bad = v.clone();
    bad["genome"][0]["taxon"] = 9999.into();
    assert!(Game::from_json(&bad.to_string(), now).is_none(), "a card");
    let mut g = Game::new(0.0);
    g.accelerate(0.0);
    let mut bad: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    bad["cycle"]["keystones"][0] = 9999.into();
    assert!(
        Game::from_json(&bad.to_string(), 0.0).is_none(),
        "a keystone"
    );
}

#[test]
fn the_most_advanced_prefers_the_youngest_of_equal_depth() {
    let mut g = Game::new(0.0);
    let find = |g: &Game, name| g.phy.taxa.iter().position(|t| t.name == name).unwrap();
    let (neanderthal, human) = (find(&g, "Neanderthal"), find(&g, "Human"));
    assert_eq!(g.taxon(neanderthal).depth, g.taxon(human).depth);
    g.unlocked[neanderthal] = true;
    assert_eq!(g.most_advanced(), neanderthal);
    g.unlocked[human] = true;
    assert_eq!(g.most_advanced(), human);
}

#[test]
fn a_waiting_genome_saved_under_its_old_name_still_loads() {
    let mut g = Game::new(0.0);
    g.accelerate(0.0);
    g.skip_cycle(0.0);
    let json = g.to_json().replace("\"genome\":", "\"nodule\":");
    let back = Game::from_json(&json, 0.0).expect("older save should load");
    assert_eq!(back.phase(), Phase::Genome);
    assert_eq!(back.genome, g.genome);
}

#[test]
fn a_save_from_a_smaller_tree_still_loads() {
    let g = Game::new(0.0);
    let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    for key in ["unlocked", "specimens", "morphs", "found_ma"] {
        v[key].as_array_mut().unwrap().truncate(10);
    }
    let back = Game::from_json(&v.to_string(), 0.0).expect("older save should load");
    assert_eq!(back.unlocked.len(), back.phy.len());
    assert_eq!(back.discovered(), 1);
}

#[test]
fn tailwind_takes_a_quarter_off_any_wait() {
    let mut g = Game::new(0.0);
    g.cycles_done = 5;
    g.boon = Some(Boon::Tailwind);
    for hours in [2.0, 6.0] {
        g.set_wait(hours);
        assert_eq!(g.next_cycle_seconds(), hours as f64 * 3600.0 * 0.75);
    }
    g.cycles_done = 0;
    assert_eq!(g.next_cycle_seconds(), 45.0);
}

#[test]
fn lens_adds_a_card_and_catalyst_scales_with_the_wait() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.set_wait(4.0);
    let plain = g.forecast().cards;
    g.boon = Some(Boon::Lens);
    assert_eq!(g.forecast().cards, plain + 1.0);
    g.boon = Some(Boon::Catalyst);
    g.set_wait(2.0);
    let short = g.forecast().odds;
    assert!(short.catalyst && !short.sure_epic, "a sure Rare");
    g.set_wait(6.0);
    assert!(
        g.forecast().odds.sure_epic,
        "a 6h wait already has its Rare"
    );
}

#[test]
fn discovery_never_passes_ninety_percent() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.set_wait(6.0);
    g.boon = Some(Boon::Discovery);
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 2,
        volcanism: 0,
    };
    let tua = equip(&mut g, "Tuatara");
    g.growth[tua] = 10 * CHARGE_MA;
    assert_eq!(g.forecast().odds.discovery, DISCOVERY_MAX);
}

#[test]
fn the_discovery_boon_adds_a_quarter() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.boon = Some(Boon::Discovery);
    for (hours, d) in [(4.0, 0.55), (6.0, 0.7)] {
        g.set_wait(hours);
        assert!((g.forecast().odds.discovery - d).abs() < 1e-6, "{hours}h");
    }
}

#[test]
fn a_save_holding_a_lure_reads_it_as_discovery() {
    let mut g = Game::new(0.0);
    g.boon = Some(Boon::Lens);
    g.boon_offer = Some(vec![Boon::Charm, Boon::Lens]);
    let json = g
        .to_json()
        .replace("\"boon\":\"Lens\"", "\"boon\":{\"Lure\":4}")
        .replace(
            "\"boon_offer\":[\"Charm\",\"Lens\"]",
            "\"boon_offer\":[{\"Lure\":7},\"Charm\",{\"Lure\":7}]",
        );
    assert!(json.contains("Lure"), "{json}");
    let back = Game::from_json(&json, 0.0).expect("an old save still loads");
    assert_eq!(back.boon, Some(Boon::Discovery));
    assert_eq!(back.boon_offer, Some(vec![Boon::Discovery, Boon::Charm]));
}

#[test]
fn tectonics_scales_with_the_wait_just_run() {
    let mut g = Game::new(0.0);
    for (hours, points) in [(2.0, 1), (4.0, 2), (6.0, 3)] {
        g.last_hours = hours;
        assert_eq!(g.tectonics_points(), points, "{hours}h");
    }
}

#[test]
fn the_wait_is_chosen_only_after_the_tutorial() {
    let mut g = Game::new(0.0);
    g.set_wait(6.0);
    assert!(!g.wait_choosable());
    assert_eq!(g.next_hours(), wait::TUTORIAL_HOURS);
    assert_eq!(g.next_cycle_seconds(), 60.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    assert_eq!(g.next_hours(), 6.0);
    assert_eq!(g.next_cycle_seconds(), 6.0 * 3600.0);
}

#[test]
fn a_long_wait_pays_in_cards_luck_and_a_sure_rare() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.set_wait(2.0);
    let short = g.forecast();
    g.set_wait(6.0);
    let long = g.forecast();
    assert_eq!((short.odds.cards, long.odds.cards), (1, 5));
    assert!(long.odds.luck > short.odds.luck);
    assert!(long.odds.catalyst && !short.odds.catalyst);
    assert_eq!((short.ma, long.ma), (20, 60));
}

#[test]
fn the_wait_just_run_sets_the_next_budget() {
    let mut g = Game::new(0.0);
    assert_eq!(g.max_points(), planet::BASE_POINTS);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    let mut now = 0.0;
    for (hours, points) in [(2.0, 1), (4.0, 2), (6.0, 3)] {
        g.set_wait(hours);
        assert_eq!(g.forecast().points, points);
        run_cycle(&mut g, &mut now);
        g.choose_boon(usize::MAX);
        assert_eq!(g.max_points(), points, "after {hours}h");
    }
}

#[test]
fn millions_of_years_add_up_per_cycle() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.set_wait(4.5);
    g.accelerate(0.0);
    assert_eq!(g.ma_elapsed(2.25 * 3600.0), 22);
    g.skip_cycle(0.0);
    assert_eq!(g.ma_done, 45);
}

#[test]
fn a_fresh_game_has_a_score_for_the_leaderboard() {
    let g = Game::new(0.0);
    assert_eq!(g.species_ever(), 1, "the Urmetazoan");
    assert_eq!(g.showcase(), Phylogeny::ROOT);
    assert_eq!(g.rad, 0);
}

/// Every field the save gained after its format was fixed (they all
/// have a default): a save from the earliest release has none of them.
const ADDED_SINCE: [&str; 17] = [
    "ma_done",
    "wait_hours",
    "charges",
    "held",
    "gone",
    "edition",
    "last_launched",
    "wait_points",
    "rad",
    "fossil",
    "doomed",
    "doom_risk",
    "last_found",
    "growth",
    "bonus_points",
    "point_carry",
    "last_hours",
];

#[test]
fn a_save_from_the_first_release_loads_with_a_leaderboard_score() {
    let mut g = Game::new(0.0);
    let mut now = 0.0;
    for _ in 0..6 {
        run_cycle(&mut g, &mut now);
        g.choose_boon(0);
    }
    let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    for key in ADDED_SINCE {
        v.as_object_mut().unwrap().remove(key);
    }
    let back = Game::from_json(&v.to_string(), now).expect("an old save loads");
    assert_eq!(back.discovered(), g.discovered());
    // No fossil list yet: everything found counts.
    assert_eq!(back.species_ever(), g.discovered());
    // No last find kept yet: the most advanced animal stands in.
    assert_eq!(back.last_found, None);
    assert_eq!(back.showcase(), back.most_advanced());
}

#[test]
fn a_save_from_when_pity_existed_still_loads() {
    let g = Game::new(0.0);
    let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    v.as_object_mut().unwrap().insert(
        "pity".into(),
        serde_json::json!({"rare_ma": 80, "since_legendary": 39, "ever_morphed": true}),
    );
    assert!(Game::from_json(&v.to_string(), 0.0).is_some());
}

#[test]
fn the_leaderboard_animal_is_the_last_one_found() {
    let mut g = Game::new(0.0);
    let mut now = 0.0;
    let opened = run_cycle(&mut g, &mut now);
    let last_new = opened
        .iter()
        .rev()
        .find(|o| o.card.new)
        .map(|o| o.card.taxon);
    assert!(
        last_new.is_some(),
        "the first genome always finds something"
    );
    assert!(g.last_found.is_some());
    assert!(g.unlocked[g.showcase()]);
    let back = Game::from_json(&g.to_json(), now).unwrap();
    assert_eq!(back.last_found, g.last_found, "kept in the save");
}

#[test]
fn a_save_from_before_the_dial_keeps_its_millions_of_years() {
    let mut g = Game::new(0.0);
    g.cycles_done = 4;
    let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    let obj = v.as_object_mut().unwrap();
    obj.remove("ma_done");
    obj.remove("wait_hours");
    let back = Game::from_json(&v.to_string(), 0.0).expect("older save should load");
    assert_eq!(back.ma_done, 80);
    assert_eq!(back.wait_hours, wait::DEFAULT_HOURS);
}

#[test]
fn eras_read_from_ages() {
    assert_eq!(era_for(800.0), "Ediacaran");
    assert_eq!(era_for(400.0), "Devonian");
    assert_eq!(era_for(0.3), "Quaternary");
}

fn equip(g: &mut Game, name: &str) -> usize {
    let t = g.phy.taxa.iter().position(|t| t.name == name).unwrap();
    g.unlocked[t] = true;
    g.specimens[t] = 1;
    g.refresh_levels();
    assert!(g.toggle_keystone(t), "couldn't equip {name}");
    t
}

fn report(g: &Game, t: usize) -> Report {
    g.keystone_reports()
        .into_iter()
        .find(|r| r.taxon == t)
        .unwrap()
}

fn luck(r: &Report) -> f32 {
    r.gains
        .iter()
        .map(|g| if let Gain::Luck(x) = g { *x } else { 0.0 })
        .sum()
}

const REEF: Planet = Planet {
    land: 1,
    vegetation: 0,
    oxygen: 3,
    temperature: 3,
    volcanism: 0,
};

#[test]
fn an_old_save_keeps_this_shapings_keystone_points() {
    let mut g = Game::new(0.0);
    g.planet = REEF;
    let coral = equip(&mut g, "Coral");
    g.morphs[coral] = Morph::Giant.bit();
    let mut v: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    let o = v.as_object_mut().unwrap();
    for key in ["growth", "bonus_points", "point_carry", "last_hours"] {
        o.remove(key);
    }
    let back = Game::from_json(&v.to_string(), 0.0).unwrap();
    assert_eq!(
        back.max_points(),
        planet::BASE_POINTS + 1,
        "the giant's point"
    );
}

#[test]
fn amber_doubles_the_bonus() {
    let mut g = Game::new(0.0);
    g.planet = REEF;
    let coral = equip(&mut g, "Coral");
    let plain = luck(&report(&g, coral));
    assert!(plain > 0.0);
    g.morphs[coral] = Morph::Amber.bit();
    assert_eq!(luck(&report(&g, coral)), plain * 2.0);
}

#[test]
fn albinos_sunburn_and_melanistic_coats_ignore_temperature() {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 3,
        volcanism: 0,
    };
    let ape = equip(&mut g, "Ape");
    g.morphs[ape] = Morph::Albino.bit() | Morph::Melanistic.bit();
    g.edition[ape] = PICKED | Morph::Albino.bit();
    assert!(g.dormant_reason(ape).unwrap().contains("albino"));
    g.edition[ape] = PICKED | Morph::Melanistic.bit();
    g.planet.temperature = 0;
    assert_eq!(g.dormant_reason(ape), None, "a Snowball ape");
    g.edition[ape] = PICKED;
    assert_eq!(g.dormant_reason(ape).as_deref(), Some("too cold"));
}

#[test]
fn the_best_morph_is_the_strongest_that_stays_awake() {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 2,
        volcanism: 0,
    };
    let ape = equip(&mut g, "Ape");
    assert_eq!(g.edition(ape), Morph::None, "nothing owned");
    g.morphs[ape] = Morph::Giant.bit() | Morph::Albino.bit() | Morph::Melanistic.bit();
    assert!(g.edition_is_best(ape));
    g.planet.temperature = 3;
    assert_eq!(g.edition(ape), Morph::Giant, "too warm for an albino");
    g.planet.temperature = 1;
    assert_eq!(g.edition(ape), Morph::Melanistic, "only it lives this cold");
    assert_eq!(g.dormant_reason(ape), None);
    g.morphs[ape] |= Morph::Amber.bit();
    g.planet.temperature = 3;
    assert_eq!(g.edition(ape), Morph::Amber);
    let bear = equip(&mut g, "Bear");
    g.morphs[bear] = Morph::Giant.bit() | Morph::Albino.bit();
    g.planet.temperature = 2;
    assert_eq!(g.dormant_reason(bear), None);
    assert_eq!(g.edition(bear), Morph::Albino);
}

#[test]
fn a_giant_keystone_gives_an_adjustment_point_no_copier_takes() {
    let mut g = Game::new(0.0);
    g.planet = REEF;
    let clown = equip(&mut g, "Clownfish");
    let base = g.max_points();
    g.morphs[clown] = Morph::Giant.bit();
    assert_eq!(g.edition(clown), Morph::Giant);
    g.earn_keystone_points(4.0);
    assert_eq!(g.max_points(), base + 1);
    let octo = equip(&mut g, "Octopus");
    let copied = report(&g, octo);
    assert_eq!(copied.copied_from, Some(clown));
    assert!(!copied.gains.contains(&Gain::Points(1)), "{copied:?}");
    g.earn_keystone_points(4.0);
    assert_eq!(g.max_points(), base + 1, "the octopus adds no point");
    g.planet.temperature = 0;
    assert!(g.dormant_reason(clown).is_some());
    assert!(!report(&g, clown).gains.contains(&Gain::Points(1)));
}

fn find(g: &Game, name: &str) -> usize {
    g.phy.taxa.iter().position(|t| t.name == name).unwrap()
}

#[test]
fn copiers_copy_their_kin_above_but_never_a_legendary() {
    let mut g = Game::new(0.0);
    g.planet = REEF;
    let clown = equip(&mut g, "Clownfish");
    let octo = equip(&mut g, "Octopus");
    let (c, o) = (report(&g, clown), report(&g, octo));
    assert!(!c.gains.is_empty());
    assert_eq!(o.gains, c.gains);
    assert_eq!(o.copied_from, Some(clown));
    assert!(o.summary().ends_with("copied"), "{}", o.summary());
    let coral = find(&g, "Coral");
    g.unlocked[coral] = true;
    g.keystones = vec![coral, octo];
    assert!(matches!(report(&g, octo).status, Status::Waiting(_)));
    let mega = find(&g, "Megalodon");
    g.unlocked[mega] = true;
    g.keystones = vec![mega, octo];
    assert!(g.dormant_reason(mega).is_none());
    assert!(matches!(report(&g, octo).status, Status::Waiting(_)));
}

#[test]
fn the_kins_are_fish_mammals_and_birds() {
    let g = Game::new(0.0);
    let kin = |name, k| g.is_kin(find(&g, name), k);
    assert!(kin("Shark", Kin::Fish) && kin("Coelacanth", Kin::Fish));
    assert!(!kin("Frog", Kin::Fish), "a tetrapod");
    assert!(kin("Lion", Kin::Mammal) && !kin("Lizard", Kin::Mammal));
    assert!(kin("Owl", Kin::Bird) && kin("Dodo", Kin::Bird) && !kin("Bat", Kin::Bird));
}

#[test]
fn the_t_rex_eats_the_keystone_in_the_last_slot() {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 5,
        volcanism: 1,
    };
    let first = equip(&mut g, "Tardigrade");
    let rex = equip(&mut g, "T. rex");
    assert_eq!(g.dormant_reason(rex), None);
    // The T. rex is last: the one before it is eaten.
    assert_eq!(
        g.dormant_reason(first).as_deref(),
        Some("eaten by the T. rex")
    );
    g.keystones = vec![rex, first];
    assert_eq!(
        g.dormant_reason(first).as_deref(),
        Some("eaten by the T. rex")
    );
    assert!(report(&g, rex).gains.contains(&Gain::Cards(2.0)));
}

#[test]
fn the_dodo_triples_luck_then_sleeps_until_found_again() {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 3,
        vegetation: 5,
        oxygen: 3,
        temperature: 4,
        volcanism: 0,
    };
    let dodo = equip(&mut g, "Dodo");
    let s = g.keystone_strength(dodo);
    assert_eq!(luck(&report(&g, dodo)), 3.0 * s);
    g.advance_keystones(g.planet, u32::from(CHARGE_MA));
    assert!(g.dormant_reason(dodo).unwrap().contains("worn out"));
    g.genome = Some(vec![Card {
        taxon: dodo,
        tier: Tier::Legendary,
        morph: Morph::None,
        new: false,
        note: None,
    }]);
    g.open_genome(0.0);
    assert_eq!(g.dormant_reason(dodo), None, "a duplicate wakes it");
}

#[test]
fn the_tuatara_grows_while_the_planet_keeps_changing() {
    let mut g = Game::new(0.0);
    let cool = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 2,
        volcanism: 0,
    };
    g.planet = cool;
    let tua = equip(&mut g, "Tuatara");
    g.advance_keystones(cool, u32::from(CHARGE_MA));
    assert_eq!(g.growth[tua], CHARGE_MA);
    g.advance_keystones(cool, u32::from(CHARGE_MA));
    assert_eq!(g.growth[tua], 0, "the same planet twice");
    g.advance_keystones(Planet { oxygen: 4, ..cool }, u32::from(CHARGE_MA));
    assert_eq!(g.growth[tua], CHARGE_MA);
    assert!(report(&g, tua)
        .gains
        .iter()
        .any(|x| matches!(x, Gain::Discovery(_))));
}

#[test]
fn the_coelacanth_grows_while_the_planet_stays_the_same() {
    let mut g = Game::new(0.0);
    let deep = Planet {
        land: 1,
        vegetation: 0,
        oxygen: 2,
        temperature: 1,
        volcanism: 0,
    };
    g.planet = deep;
    let fish = equip(&mut g, "Coelacanth");
    assert_eq!(g.dormant_reason(fish), None);
    let s = g.keystone_strength(fish);
    assert_eq!(luck(&report(&g, fish)), s, "its base luck from the start");
    g.advance_keystones(deep, u32::from(CHARGE_MA));
    assert_eq!(g.growth[fish], 0, "nothing to compare the first wait to");
    g.advance_keystones(deep, u32::from(CHARGE_MA));
    g.advance_keystones(deep, u32::from(CHARGE_MA));
    assert_eq!(g.growth[fish], 2 * CHARGE_MA);
    assert!(
        (luck(&report(&g, fish)) - s * 1.4).abs() < 1e-4,
        "+20% a charge"
    );
    // Full at 5 charges: twice its base, no more.
    for _ in 0..10 {
        g.advance_keystones(deep, u32::from(CHARGE_MA));
    }
    assert_eq!(g.growth[fish], 5 * CHARGE_MA);
    assert!((luck(&report(&g, fish)) - s * 2.0).abs() < 1e-4);
    assert!(
        (g.effects().discovery + COELACANTH_DISCOVERY).abs() < 1e-6,
        "its catch: fewer new species"
    );
    assert_eq!(g.effects().morph_mult_keystone, 1.0, "not the Tuatara's");
    g.advance_keystones(Planet { oxygen: 3, ..deep }, u32::from(CHARGE_MA));
    assert_eq!(g.growth[fish], 0, "the planet changed");
}

#[test]
fn the_platypus_has_every_biome_bonus_and_a_longer_wait() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.planet = Planet {
        land: 2,
        vegetation: 1,
        oxygen: 3,
        temperature: 1,
        volcanism: 0,
    };
    let plain = g.next_cycle_seconds();
    let platypus = equip(&mut g, "Platypus");
    assert_eq!(g.dormant_reason(platypus), None);
    assert_eq!(g.active_biomes(), Biome::ALL.to_vec());
    // 50% longer, less the Ice age's 15% it brings along.
    let expected = plain * 1.5 * 0.85;
    assert!((g.next_cycle_seconds() - expected).abs() < 1e-3);
}

#[test]
fn megalodon_doubles_luck_and_locks_the_cold_out() {
    let mut g = Game::new(0.0);
    g.planet = Planet { land: 1, ..REEF };
    equip(&mut g, "Coral");
    let before = g.effects().luck;
    equip(&mut g, "Megalodon");
    assert_eq!(g.effects().luck, (before * 2.0).min(KEYSTONE_LUCK_MAX));
    assert!(!g.can_step(Lever::Temperature, -1));
    assert!(g.lever_lock(Lever::Temperature, -1).is_some());
    assert!(g.lever_lock(Lever::Temperature, 1).is_none());
}

#[test]
fn the_wait_never_crowds_out_keystone_luck() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.set_wait(6.0);
    g.planet = Planet { land: 1, ..REEF };
    let wait = g.forecast().odds.luck;
    assert_eq!(wait, 12.0, "a 6h wait alone");
    equip(&mut g, "Coral");
    let one = g.effects().luck;
    assert!(one > 0.0);
    assert_eq!(g.forecast().odds.luck, wait + one, "all of it counts");
    // The Megalodon's x2 counts too, up to the keystones' cap.
    equip(&mut g, "Megalodon");
    assert_eq!(
        g.forecast().odds.luck,
        wait + (one * 2.0).min(KEYSTONE_LUCK_MAX)
    );
    assert!(g.forecast().odds.luck > 15.0, "past the old cap of 15");
}

#[test]
fn archaeopteryx_triples_morphs_and_takes_a_boon() {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 3,
        volcanism: 0,
    };
    let plain = g.forecast().odds.morph_mult;
    let bird = equip(&mut g, "Archaeopteryx");
    assert_eq!(g.dormant_reason(bird), None);
    assert_eq!(g.forecast().odds.morph_mult, plain * 3.0);
    assert!(g.effects().fewer_boons);
    assert_eq!(g.roll_boons().len(), 2);
}

#[test]
fn keystones_swapped_during_a_wait_count_from_the_next_one() {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 3,
        volcanism: 0,
    };
    let mut now = 0.0;
    let bird = equip(&mut g, "Archaeopteryx");
    assert!(g.accelerate(now));
    assert!(g.toggle_keystone(bird), "unequipped while time runs");
    now += g.cycle.unwrap().duration + 1.0;
    g.tick(now);
    g.open_genome(now);
    assert_eq!(g.boon_offer.as_ref().unwrap().len(), 2, "launched with it");
    g.choose_boon(0);
    // Equipped again mid-wait, it only takes a boon from the next one.
    assert!(g.accelerate(now));
    assert!(g.toggle_keystone(bird));
    now += g.cycle.unwrap().duration + 1.0;
    g.tick(now);
    g.open_genome(now);
    assert_eq!(
        g.boon_offer.as_ref().unwrap().len(),
        3,
        "launched without it"
    );
}

#[test]
fn charges_cant_be_banked_by_sitting_a_wait_out() {
    let mut g = Game::new(0.0);
    let cool = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 2,
        volcanism: 0,
    };
    g.planet = cool;
    let tua = equip(&mut g, "Tuatara");
    g.advance_keystones(cool, u32::from(CHARGE_MA));
    assert_eq!(g.growth[tua], CHARGE_MA);
    // Unequipped for a wait on the same planet, which would empty them.
    assert!(g.toggle_keystone(tua));
    g.advance_keystones(cool, u32::from(CHARGE_MA));
    assert_eq!(g.growth[tua], 0, "sitting out empties them too");
    // Unequipped mid-wait, it still ran it: the launch set counts.
    assert!(g.toggle_keystone(tua));
    g.planet = Planet { oxygen: 4, ..cool };
    g.advance_keystones(g.planet, u32::from(CHARGE_MA));
    assert_eq!(g.growth[tua], CHARGE_MA);
    assert!(g.accelerate(0.0));
    assert!(g.toggle_keystone(tua));
    let now = g.cycle.unwrap().duration + 1.0;
    g.tick(now);
    assert_eq!(
        g.growth[tua], 0,
        "a wait on the same planet, launched with it"
    );
}

#[test]
fn the_tuatara_halves_morphs() {
    let mut g = Game::new(0.0);
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 2,
        volcanism: 0,
    };
    let (wait, morphs) = (g.next_cycle_seconds(), g.forecast().odds.morph_mult);
    equip(&mut g, "Tuatara");
    assert_eq!(g.next_cycle_seconds(), wait, "no longer wait");
    assert_eq!(g.forecast().odds.morph_mult, morphs * 0.5);
}

#[test]
fn three_keystones_of_a_biome_give_its_bonus() {
    let mut g = Game::new(0.0);
    g.planet = REEF;
    equip(&mut g, "Coral");
    equip(&mut g, "Clownfish");
    let morph = g.effects().morph;
    assert!(g.active_biomes().is_empty(), "two isn't enough");
    equip(&mut g, "Seahorse");
    assert_eq!(g.active_biomes(), vec![Biome::Reef]);
    let seahorse = g.effects().morph;
    assert!(seahorse >= morph + 0.5, "{morph} -> {seahorse}");
}

#[test]
fn discovery_grows_with_the_wait_and_keystones() {
    let mut g = Game::new(0.0);
    assert_eq!(g.forecast().odds.discovery, 1.0, "the tutorial only finds");
    g.cycles_done = planet::TUTORIAL_CYCLES;
    for (hours, d) in [(2.0, 0.15), (4.0, 0.3), (6.0, 0.45)] {
        g.set_wait(hours);
        assert!((g.forecast().odds.discovery - d).abs() < 1e-6, "{hours}h");
    }
    g.planet = Planet {
        land: 3,
        vegetation: 3,
        oxygen: 3,
        temperature: 2,
        volcanism: 0,
    };
    g.set_wait(4.0);
    let tua = equip(&mut g, "Tuatara");
    g.growth[tua] = 2 * CHARGE_MA;
    assert!(g.forecast().odds.discovery > 0.3);
}

#[test]
fn a_save_from_before_picking_wears_the_best_morph() {
    let mut g = Game::new(0.0);
    let t = equip(&mut g, "Urmetazoan");
    g.morphs[t] = Morph::Giant.bit() | Morph::Amber.bit();
    for old in [0, Morph::Giant.bit()] {
        g.edition[t] = old;
        assert_eq!(g.edition(t), Morph::Amber, "{old}");
    }
}

#[test]
fn editions_cycle_through_owned_morphs() {
    let mut g = Game::new(0.0);
    let t = equip(&mut g, "Urmetazoan");
    assert!(!g.cycle_edition(t), "nothing to pick");
    g.morphs[t] = Morph::Giant.bit() | Morph::Amber.bit();
    assert_eq!(g.edition(t), Morph::Amber, "best by default");
    let mut seen = Vec::new();
    for _ in 0..4 {
        assert!(g.cycle_edition(t));
        seen.push((g.edition_is_best(t), g.edition(t)));
    }
    assert_eq!(
        seen,
        [
            (false, Morph::None),
            (false, Morph::Giant),
            (false, Morph::Amber),
            (true, Morph::Amber),
        ]
    );
}

fn with_human() -> (Game, usize) {
    let mut g = Game::new(0.0);
    g.planet = Planet {
        land: 4,
        vegetation: 2,
        oxygen: 3,
        temperature: 3,
        volcanism: 0,
    };
    let human = equip(&mut g, "Human");
    (g, human)
}

#[test]
fn human_doubles_luck_adds_cards_and_risks_the_earth() {
    let (g, human) = with_human();
    let r = report(&g, human);
    assert_eq!(r.status, Status::Active);
    let e = g.effects();
    assert_eq!(e.doom, DOOM_CHANCE);
    assert!(e.extra_card >= 2.0);
    assert!(r.gains.contains(&Gain::LuckMult(2.0)));
}

#[test]
fn about_one_4h_wait_in_five_is_doomed_with_human() {
    let (mut g, _) = with_human();
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.set_wait(wait::DEFAULT_HOURS);
    let mut now = 0.0;
    let mut doomed = 0;
    for _ in 0..200 {
        g.keystones = vec![g.keystones[0]];
        g.planet = with_human().0.planet;
        g.accelerate(now);
        now += g.cycle.unwrap().duration + 1.0;
        g.tick(now);
        assert!((g.doom_risk - DOOM_CHANCE).abs() < 1e-6);
        doomed += g.doomed as u32;
        g.genome = None;
        g.doomed = false;
    }
    assert!((20..=60).contains(&doomed), "{doomed} of 200");
}

#[test]
fn the_doom_compounds_so_short_waits_risk_the_same() {
    let (mut g, _) = with_human();
    g.cycles_done = planet::TUTORIAL_CYCLES;
    let mut risk = |hours: f32| {
        g.set_wait(hours);
        g.accelerate(0.0);
        g.skip_cycle(0.0);
        g.genome = None;
        g.doomed = false;
        g.planet = with_human().0.planet;
        g.doom_risk
    };
    let (short, long) = (risk(2.0), risk(6.0));
    let three_short = 1.0 - (1.0 - short).powi(3);
    assert!((three_short - long).abs() < 1e-5, "{three_short} vs {long}");
}

#[test]
fn ending_the_earth_keeps_fossils_and_adds_rad() {
    let (mut g, human) = with_human();
    g.specimens[human] = 9;
    g.morphs[human] = Morph::Amber.bit();
    g.ma_done = 500;
    g.end_earth();
    assert_eq!(g.rad, 1);
    assert_eq!(g.discovered(), 1, "back to the Urmetazoan");
    assert!(g.is_fossil(human));
    assert_eq!(g.fossils(), 1);
    assert!(g.keystones.is_empty());
    assert_eq!(g.ma_done, 0);
    assert_eq!(g.planet, Planet::default());
    assert_eq!(g.morphs[human], Morph::Amber.bit(), "morphs kept");
    assert!(!g.toggle_keystone(human), "a fossil must be found again");
    let odds = g.forecast().odds;
    assert!(odds.luck >= RAD_LUCK && odds.morph_rad == 1.0 + RAD_MORPH);
}

#[test]
fn a_fossil_found_again_keeps_its_level() {
    let (mut g, human) = with_human();
    g.specimens[human] = 9;
    g.end_earth();
    g.genome = Some(vec![Card {
        taxon: human,
        tier: Tier::Legendary,
        morph: Morph::None,
        new: true,
        note: None,
    }]);
    g.open_genome(0.0);
    assert!(g.unlocked[human] && !g.is_fossil(human));
    assert_eq!(g.specimens[human], 10);
    assert_eq!(g.level[human], level_for(10));
}

#[test]
fn cards_are_no_longer_capped() {
    let (mut g, _) = with_human();
    g.cycles_done = planet::TUTORIAL_CYCLES;
    g.set_wait(6.0);
    g.boon = Some(Boon::Lens);
    assert!(g.forecast().odds.cards >= 8, "5 + 2 + 1");
}
