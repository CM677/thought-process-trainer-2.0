use postflop_solver::*;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;

const OUTPUT_FILE: &str = "output4.csv";
const SPOTS_FILE: &str = "spots.json";
const PLAYER_IP: usize = 1;

#[derive(Debug, Deserialize)]
struct Spot {
    id: String,
    position_matchup: String,
    pot_type: String,
    hero_position: String,
    board: String,
}

#[derive(Clone, Copy)]
struct BetExport {
    action_index: usize,
    label: &'static str,
    spot_suffix: &'static str,
}

fn main() -> Result<(), Box<dyn Error>> {
    let spots = load_spots(SPOTS_FILE)?;
    let mut writer = csv::Writer::from_path(OUTPUT_FILE)?;

    writer.write_record([
        "spot_name",
        "street",
        "hand",
        "board",
        "player_type",
        "pot_type",
        "villain_bet_size",
        "check_freq",
        "check_ev",
        "bet_1_size",
        "bet_1_freq",
        "bet_1_ev",
        "allin_freq",
        "allin_ev",
        "fold_freq",
        "fold_ev",
        "call_freq",
        "call_ev",
        "raise_1_size",
        "raise_1_freq",
        "raise_1_ev",
        "best_action",
        "ev",
    ])?;

    for spot in &spots {
        export_spot(spot, &mut writer)?;
    }

    writer.flush()?;
    println!("Wrote {OUTPUT_FILE}");
    Ok(())
}

fn load_spots(path: &str) -> Result<Vec<Spot>, Box<dyn Error>> {
    let file = File::open(path)?;
    Ok(serde_json::from_reader(file)?)
}

fn export_spot(spot: &Spot, writer: &mut csv::Writer<File>) -> Result<(), Box<dyn Error>> {
    if spot.hero_position != "ip" {
        return Err(format!("unsupported hero_position: {}", spot.hero_position).into());
    }

    let (ip_range, oop_range) =
        lookup_ranges(&spot.position_matchup, &spot.pot_type, &spot.hero_position)?;

    let card_config = CardConfig {
        range: [oop_range.parse()?, ip_range.parse()?],
        flop: flop_from_str(&spot.board)?,
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };

    let tree_config = srp_flop_tree_config()?;
    let action_tree = ActionTree::new(tree_config)?;
    let mut game = PostFlopGame::with_config(card_config, action_tree)?;

    let (mem_usage, mem_usage_compressed) = game.memory_usage();
    println!(
        "Estimated memory: {:.2}GB uncompressed, {:.2}GB compressed",
        gb(mem_usage),
        gb(mem_usage_compressed)
    );

    game.allocate_memory(false);

    let max_iterations = 50; // CHANGE TO 200 FOR PRODUCTION
    let target_exploitability = 0.5;
    let exploitability = solve(&mut game, max_iterations, target_exploitability, true);
    println!(
        "Solved {}-{}-{} at exploitability {:.4}",
        spot.position_matchup, spot.pot_type, spot.id, exploitability
    );

    move_to_hero_betting_ip_node(&mut game)?;
    game.cache_normalized_weights();

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let allin_index = find_action(&actions, |action| matches!(action, Action::AllIn(_)))?;
    let bet_exports = collect_bet_exports(&actions)?;

    let hero_cards = game.private_cards(PLAYER_IP);
    let hero_hands = holes_to_strings(hero_cards)?;
    let strategy = game.strategy();
    let action_evs = game.expected_values_detail(PLAYER_IP);
    let hand_count = hero_hands.len();

    for (hand_index, hand) in hero_hands.iter().enumerate() {
        let check_freq = strategy_value(&strategy, check_index, hand_index, hand_count);
        let check_ev = strategy_value(&action_evs, check_index, hand_index, hand_count);
        let allin_freq = strategy_value(&strategy, allin_index, hand_index, hand_count);
        let allin_ev = strategy_value(&action_evs, allin_index, hand_index, hand_count);

        for bet in &bet_exports {
            let bet_freq = strategy_value(&strategy, bet.action_index, hand_index, hand_count);
            let bet_ev = strategy_value(&action_evs, bet.action_index, hand_index, hand_count);
            let (best_action, best_ev) = best_action_for_row(check_ev, bet_ev, allin_ev);
            let spot_name = format!(
                "{}-{}-{}-flop-hero_betting_ip-{}",
                spot.position_matchup, spot.pot_type, spot.id, bet.spot_suffix
            );

            writer.write_record([
                spot_name,
                "flop".to_string(),
                hand.to_string(),
                spot.board.clone(),
                "ip".to_string(),
                spot.pot_type.clone(),
                "null".to_string(),
                format_float(check_freq),
                format_float(check_ev),
                bet.label.to_string(),
                format_float(bet_freq),
                format_float(bet_ev),
                format_float(allin_freq),
                format_float(allin_ev),
                "null".to_string(),
                "null".to_string(),
                "null".to_string(),
                "null".to_string(),
                "null".to_string(),
                "null".to_string(),
                "null".to_string(),
                best_action.to_string(),
                format_float(best_ev),
            ])?;
        }
    }

    Ok(())
}

fn lookup_ranges(
    position_matchup: &str,
    pot_type: &str,
    hero_position: &str,
) -> Result<(&'static str, &'static str), Box<dyn Error>> {
    match (position_matchup, pot_type, hero_position) {
        ("btn-vs-bb", "srp", "ip") => Ok((
            "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s",
            "99-22,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,AJo-A6o,K9o+,QTo+,JTo,QTs-Q2s,A4s-A2s,T9o",
        )),
        _ => Err(format!(
            "no ranges for {position_matchup} {pot_type} hero_position={hero_position}"
        )
        .into()),
    }
}

fn srp_flop_tree_config() -> Result<TreeConfig, Box<dyn Error>> {
    let flop_ip_bets = BetSizeOptions::try_from(("33%, 75%, 125%", "50%"))?;

    Ok(TreeConfig {
        initial_state: BoardState::Flop,
        starting_pot: 600,
        effective_stack: 9750,
        rake_rate: 0.05,
        rake_cap: 300.0,
        flop_bet_sizes: [Default::default(), flop_ip_bets],
        turn_bet_sizes: Default::default(),
        river_bet_sizes: Default::default(),
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 1.5,
        force_allin_threshold: 0.15,
        merging_threshold: 0.0,
    })
}

fn move_to_hero_betting_ip_node(game: &mut PostFlopGame) -> Result<(), Box<dyn Error>> {
    game.back_to_root();
    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    game.play(check_index);

    if game.current_player() != PLAYER_IP {
        return Err("expected IP to act after OOP checks the flop".into());
    }

    Ok(())
}

fn collect_bet_exports(actions: &[Action]) -> Result<Vec<BetExport>, Box<dyn Error>> {
    let mut bets = Vec::new();

    for (action_index, action) in actions.iter().enumerate() {
        if let Action::Bet(action_amount) = action {
            bets.push((action_index, *action_amount));
        }
    }

    bets.sort_by_key(|(_, amount)| *amount);

    if bets.len() != 3 {
        return Err(format!("expected exactly 3 flop IP bet sizes, found {}", bets.len()).into());
    }

    Ok(vec![
        BetExport {
            action_index: bets[0].0,
            label: "0.33",
            spot_suffix: "33",
        },
        BetExport {
            action_index: bets[1].0,
            label: "0.75",
            spot_suffix: "75",
        },
        BetExport {
            action_index: bets[2].0,
            label: "1.25",
            spot_suffix: "125",
        },
    ])
}

fn find_action(
    actions: &[Action],
    predicate: impl Fn(&Action) -> bool,
) -> Result<usize, Box<dyn Error>> {
    actions
        .iter()
        .position(predicate)
        .ok_or_else(|| format!("action not found in {actions:?}").into())
}

fn strategy_value(values: &[f32], action_index: usize, hand_index: usize, hand_count: usize) -> f32 {
    values[action_index * hand_count + hand_index]
}

fn best_action_for_row(check_ev: f32, bet_ev: f32, allin_ev: f32) -> (&'static str, f32) {
    if check_ev >= bet_ev && check_ev >= allin_ev {
        ("check", check_ev)
    } else if bet_ev >= allin_ev {
        ("bet_1", bet_ev)
    } else {
        ("allin", allin_ev)
    }
}

fn format_float(value: f32) -> String {
    format!("{value:.6}")
}

fn gb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}
