use postflop_solver::*;
use std::error::Error;

const OUTPUT_FILE: &str = "output4.csv";
const HERO_PLAYER: usize = 1;

const HAND: &str = "Kh4h";
const BOARD: &str = "Ah7s4c";
const BTN_RANGE: &str =
    "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s";
const BB_RANGE: &str = "99-22,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,AJo-A6o,K9o+,QTo+,JTo,QTs-Q2s,A4s-A2s,T9o";

struct SolveSize {
    tree_size: &'static str,
    csv_size: &'static str,
    suffix: &'static str,
}

struct ActionValues {
    check_freq: f32,
    check_ev: f32,
    bet_freq: f32,
    bet_ev: f32,
    allin_freq: f32,
    allin_ev: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let sizes = [
        SolveSize {
            tree_size: "33%",
            csv_size: "0.33",
            suffix: "33",
        },
        SolveSize {
            tree_size: "75%",
            csv_size: "0.75",
            suffix: "75",
        },
        SolveSize {
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

    for size in sizes {
        let values = solve_one_size(size.tree_size)?;
        let (best_action, ev) = best_action(values.check_ev, values.bet_ev, values.allin_ev);

        writer.write_record([
            format!("btn-vs-bb-srp-01-flop-hero_betting_ip-{}", size.suffix),
            "flop".to_string(),
            HAND.to_string(),
            BOARD.to_string(),
            "ip".to_string(),
            "srp".to_string(),
            "null".to_string(),
            format_float(values.check_freq),
            format_float(values.check_ev),
            size.csv_size.to_string(),
            format_float(values.bet_freq),
            format_float(values.bet_ev),
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
            format_float(ev),
        ])?;
    }

    writer.flush()?;
    println!("Wrote {OUTPUT_FILE}");
    Ok(())
}

fn solve_one_size(ip_bet_size: &str) -> Result<ActionValues, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [BB_RANGE.parse()?, BTN_RANGE.parse()?],
        flop: flop_from_str(BOARD)?,
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };

    let tree_config = TreeConfig {
        initial_state: BoardState::Flop,
        starting_pot: 600,
        effective_stack: 9750,
        rake_rate: 0.05,
        rake_cap: 300.0,
        flop_bet_sizes: [
            Default::default(),
            BetSizeOptions::try_from((ip_bet_size, ""))?,
        ],
        turn_bet_sizes: Default::default(),
        river_bet_sizes: Default::default(),
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 1.5,
        force_allin_threshold: 0.15,
        merging_threshold: 0.0,
    };

    let action_tree = ActionTree::new(tree_config)?;
    let mut game = PostFlopGame::with_config(card_config, action_tree)?;
    game.allocate_memory(false);
    solve(&mut game, 50, 0.5, true);

    move_to_hero_betting_node(&mut game)?;
    game.cache_normalized_weights();

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let bet_index = find_action(&actions, |action| matches!(action, Action::Bet(_)))?;
    let allin_index = find_action(&actions, |action| matches!(action, Action::AllIn(_)))?;

    let hand_index = find_hand_index(&game)?;
    let hand_count = game.private_cards(HERO_PLAYER).len();
    let strategy = game.strategy();
    let evs = game.expected_values_detail(HERO_PLAYER);

    Ok(ActionValues {
        check_freq: value_at(&strategy, check_index, hand_index, hand_count),
        check_ev: value_at(&evs, check_index, hand_index, hand_count),
        bet_freq: value_at(&strategy, bet_index, hand_index, hand_count),
        bet_ev: value_at(&evs, bet_index, hand_index, hand_count),
        allin_freq: value_at(&strategy, allin_index, hand_index, hand_count),
        allin_ev: value_at(&evs, allin_index, hand_index, hand_count),
    })
}

fn move_to_hero_betting_node(game: &mut PostFlopGame) -> Result<(), Box<dyn Error>> {
    game.back_to_root();
    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    game.play(check_index);

    if game.current_player() != HERO_PLAYER {
        return Err("expected IP hero to act after OOP checks flop".into());
    }

    Ok(())
}

fn find_hand_index(game: &PostFlopGame) -> Result<usize, Box<dyn Error>> {
    let cards = game.private_cards(HERO_PLAYER);
    let hands = holes_to_strings(cards)?;

    hands
        .iter()
        .position(|hand| hand == HAND)
        .ok_or_else(|| format!("{HAND} was not found in BTN range on {BOARD}").into())
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

fn value_at(values: &[f32], action_index: usize, hand_index: usize, hand_count: usize) -> f32 {
    values[action_index * hand_count + hand_index]
}

fn best_action(check_ev: f32, bet_ev: f32, allin_ev: f32) -> (&'static str, f32) {
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
