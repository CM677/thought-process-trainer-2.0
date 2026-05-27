use postflop_solver::*;
use std::error::Error;

const OUTPUT_FILE: &str = "output4.csv";

const HERO_PLAYER: usize = 1;
const BB_CHIPS: f32 = 100.0;
const STARTING_POT: i32 = 600;
const EFFECTIVE_STACK: i32 = 9750;

const HAND: &str = "Kh4h";
const BOARD: &str = "Ah7s4c";
const BTN_RANGE: &str =
    "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s";
const BB_RANGE: &str = "99-22,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,AJo-A6o,K9o+,QTo+,JTo,QTs-Q2s,A4s-A2s,T9o";

struct Sizing {
    tree_size: &'static str,
    csv_size: &'static str,
    suffix: &'static str,
}

struct RowValues {
    check_freq: f32,
    check_ev: f32,
    bet_1_freq: f32,
    bet_1_ev: f32,
    allin_freq: f32,
    allin_ev: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let sizings = [
        Sizing {
            tree_size: "33%",
            csv_size: "0.33",
            suffix: "33",
        },
        Sizing {
            tree_size: "75%",
            csv_size: "0.75",
            suffix: "75",
        },
        Sizing {
            tree_size: "125%",
            csv_size: "1.25",
            suffix: "125",
        },
    ];

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

    for sizing in sizings {
        let values = solve_sizing(&sizing)?;
        print_sanity_check(&sizing, &values);

        let (best_action, best_ev) =
            best_action(values.check_ev, values.bet_1_ev, values.allin_ev);

        writer.write_record([
            format!("btn-vs-bb-srp-01-flop-hero_betting_ip-{}", sizing.suffix),
            "flop".to_string(),
            HAND.to_string(),
            BOARD.to_string(),
            "ip".to_string(),
            "srp".to_string(),
            "null".to_string(),
            format_float(values.check_freq),
            format_float(values.check_ev),
            sizing.csv_size.to_string(),
            format_float(values.bet_1_freq),
            format_float(values.bet_1_ev),
            format_float(values.allin_freq),
            format_float(values.allin_ev),
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

    writer.flush()?;
    println!("Wrote {OUTPUT_FILE}");
    Ok(())
}

fn solve_sizing(sizing: &Sizing) -> Result<RowValues, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [BB_RANGE.parse()?, BTN_RANGE.parse()?],
        flop: flop_from_str(BOARD)?,
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };

    let tree_config = TreeConfig {
        initial_state: BoardState::Flop,
        starting_pot: STARTING_POT,
        effective_stack: EFFECTIVE_STACK,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [
            Default::default(),
            BetSizeOptions::try_from((sizing.tree_size, ""))?,
        ],
        turn_bet_sizes: Default::default(),
        river_bet_sizes: Default::default(),
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 100.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };

    let action_tree = ActionTree::new(tree_config)?;
    let mut game = PostFlopGame::with_config(card_config, action_tree)?;
    game.allocate_memory(false);
    solve(&mut game, 50, 0.5, true);

    move_to_hero_root_decision(&mut game)?;
    game.cache_normalized_weights();

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let bet_1_index = find_action(&actions, |action| matches!(action, Action::Bet(_)))?;
    let allin_index = find_action(&actions, |action| matches!(action, Action::AllIn(_)))?;

    let hand_index = find_hand_index(&game)?;
    let hand_count = game.private_cards(HERO_PLAYER).len();
    let strategy = game.strategy();
    let evs = game.expected_values_detail(HERO_PLAYER);

    Ok(RowValues {
        check_freq: action_value(&strategy, check_index, hand_index, hand_count),
        check_ev: ev_to_bb(action_value(&evs, check_index, hand_index, hand_count)),
        bet_1_freq: action_value(&strategy, bet_1_index, hand_index, hand_count),
        bet_1_ev: ev_to_bb(action_value(&evs, bet_1_index, hand_index, hand_count)),
        allin_freq: action_value(&strategy, allin_index, hand_index, hand_count),
        allin_ev: ev_to_bb(action_value(&evs, allin_index, hand_index, hand_count)),
    })
}

fn move_to_hero_root_decision(game: &mut PostFlopGame) -> Result<(), Box<dyn Error>> {
    game.back_to_root();

    if game.current_player() == HERO_PLAYER {
        return Ok(());
    }

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    game.play(check_index);

    if game.current_player() != HERO_PLAYER {
        return Err(format!(
            "expected hero player {HERO_PLAYER} to act, current player is {}",
            game.current_player()
        )
        .into());
    }

    Ok(())
}

fn find_hand_index(game: &PostFlopGame) -> Result<usize, Box<dyn Error>> {
    let hands = holes_to_strings(game.private_cards(HERO_PLAYER))?;

    hands
        .iter()
        .position(|hand| hand == HAND)
        .ok_or_else(|| format!("{HAND} was not found for player {HERO_PLAYER} on {BOARD}").into())
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

fn action_value(values: &[f32], action_index: usize, hand_index: usize, hand_count: usize) -> f32 {
    values[action_index * hand_count + hand_index]
}

fn ev_to_bb(ev_in_chips: f32) -> f32 {
    ev_in_chips / BB_CHIPS
}

fn best_action(check_ev: f32, bet_1_ev: f32, allin_ev: f32) -> (&'static str, f32) {
    if check_ev >= bet_1_ev && check_ev >= allin_ev {
        ("check", check_ev)
    } else if bet_1_ev >= allin_ev {
        ("bet_1", bet_1_ev)
    } else {
        ("allin", allin_ev)
    }
}

fn print_sanity_check(sizing: &Sizing, values: &RowValues) {
    println!(
        "Solved {} sizing: check_ev={:.4} BB, bet_1_ev={:.4} BB, allin_ev={:.4} BB",
        sizing.csv_size, values.check_ev, values.bet_1_ev, values.allin_ev
    );

    if values.check_ev > 50.0
        || values.bet_1_ev > 50.0
        || values.allin_ev > 50.0
        || values.check_ev < 0.0
        || values.bet_1_ev < 0.0
        || values.allin_ev < 0.0
    {
        eprintln!("WARNING: EV sanity check failed; EV conversion may be wrong.");
    }
}

fn format_float(value: f32) -> String {
    format!("{value:.6}")
}
