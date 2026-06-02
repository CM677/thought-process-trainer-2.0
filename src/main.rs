use postflop_solver::*;
use std::collections::HashMap;
use std::error::Error;

const OUTPUT_FILE: &str = "output5.csv";

const OOP_PLAYER: usize = 0;
const IP_PLAYER: usize = 1;
const BB_CHIPS: f32 = 100.0;
const SOLVE_ITERATIONS: u32 = 500;

const FLOP: &str = "Ah7s4c";
const TURN: &str = "6s";
const TURN_BOARD: &str = "Ah7s4c6s";
const STARTING_POT: i32 = 600;
const STARTING_STACK: i32 = 9750;

const BTN_RANGE: &str =
    "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s";
const BB_RANGE: &str = "99-22,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,AJo-A6o,K9o+,QTo+,JTo,QTs-Q2s,A4s-A2s,T9o";

#[derive(Clone, Copy)]
struct Sizing {
    tree_size: &'static str,
    csv_size: &'static str,
    suffix: &'static str,
}

#[derive(Clone)]
struct HandCombo {
    cards: (Card, Card),
    label: String,
    index: usize,
}

#[derive(Clone)]
struct WeightedCombo {
    cards: (Card, Card),
    value_weight: f32,
    fold_weight: f32,
}

struct DecisionMeta {
    street: &'static str,
    board: &'static str,
    turn_card: &'static str,
    flop_action: &'static str,
    flop_bet_size: &'static str,
    turn_bet_size: &'static str,
}

struct DecisionSolve {
    meta: DecisionMeta,
    hands: Vec<HandCombo>,
    villains: Vec<HandCombo>,
    strategy: Vec<f32>,
    evs: Vec<f32>,
    hero_equities: Vec<f32>,
    check_index: usize,
    bet_index: usize,
    hand_count: usize,
    range_stats: RangeStats,
    villain_weights: Vec<WeightedCombo>,
}

struct RangeStats {
    hero_weighted_value_combos: f32,
    villain_weighted_value_combos: f32,
    hero_total_live_combos: f32,
    villain_total_live_combos: f32,
    hero_weighted_value_pct: f32,
    villain_weighted_value_pct: f32,
    nut_advantage_pct: f32,
    villain_weighted_fold_combos: f32,
    range_equity_hero: f32,
}

struct RowValues {
    check_freq: f32,
    check_ev: f32,
    bet_freq: f32,
    bet_ev: f32,
    best_action: &'static str,
    ev: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModalAction {
    Check,
    Bet,
    Fold,
    Call,
    Raise,
    Other,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Writing {OUTPUT_FILE}");
    println!("Hardcoded flop: {FLOP}, turn: {TURN}");

    let flop_sizings = [
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
    let turn_sizings = [
        Sizing {
            tree_size: "50%",
            csv_size: "0.50",
            suffix: "50",
        },
        Sizing {
            tree_size: "100%",
            csv_size: "1.00",
            suffix: "100",
        },
    ];

    let mut flop_solves = Vec::new();
    let mut flop_branch_data = Vec::new();
    let xx_source_solve = solve_flop_all_sizes_for_xx()?;

    for sizing in flop_sizings {
        println!("Running flop solve: {}", sizing.tree_size);
        let solved = solve_flop_sizing(sizing)?;
        let branch = inspect_flop_branches(&solved)?;
        println!(
            "BB raise option versus flop {} bet: {}",
            sizing.tree_size, branch.bb_raise_available
        );
        flop_branch_data.push(branch);
        flop_solves.push(solved);
    }

    println!(
        "BB turn donk filtering enabled: OOP gets one 50% pot donk option before BTN's turn decision"
    );

    println!(
        "XX branch source: dedicated flop solve with BTN sizes 33%, 75%, and 125% all available."
    );
    println!(
        "XX branch uses BTN/IP modal check-back hands from that dedicated solve after BB checks flop."
    );
    println!(
        "XX branch BB pre-filter range starts from the full BB live flop range, not a flop-call range."
    );
    let xx_hero = branch_hero_hands(&xx_source_solve, ModalAction::Check);
    let xx_villain = xx_source_solve.villains.clone();
    println!(
        "XX branch before turn donk filtering: hero check-back combos={}, BB live combos={}",
        xx_hero.len(),
        xx_villain.len()
    );

    let turn_branches = [
        ("xx", "null", 600, 9750, xx_hero, xx_villain),
        (
            "b33c",
            "0.33",
            996,
            9552,
            branch_hero_hands(&flop_solves[0], ModalAction::Bet),
            flop_branch_data[0].bb_call_hands.clone(),
        ),
        (
            "b75c",
            "0.75",
            1500,
            9300,
            branch_hero_hands(&flop_solves[1], ModalAction::Bet),
            flop_branch_data[1].bb_call_hands.clone(),
        ),
        (
            "b125c",
            "1.25",
            2100,
            9000,
            branch_hero_hands(&flop_solves[2], ModalAction::Bet),
            flop_branch_data[2].bb_call_hands.clone(),
        ),
    ];

    let mut turn_solves = Vec::new();
    for (flop_action, flop_bet_size, pot, stack, hero_hands, villain_hands) in turn_branches {
        let checked_villains =
            filter_bb_turn_checks(flop_action, pot, stack, &hero_hands, &villain_hands)?;
        println!(
            "Turn branch {flop_action}: hero combos={}, villain combos after check filter={}",
            hero_hands.len(),
            checked_villains.len()
        );
        for sizing in turn_sizings {
            if hero_hands.is_empty() || checked_villains.is_empty() {
                println!(
                    "Skipping turn branch {flop_action}, sizing {} because a branch range is empty",
                    sizing.tree_size
                );
                continue;
            }
            println!(
                "Running turn branch {flop_action}, turn sizing {}",
                sizing.tree_size
            );
            turn_solves.push(solve_turn_sizing(
                flop_action,
                flop_bet_size,
                sizing,
                pot,
                stack,
                &hero_hands,
                &checked_villains,
            )?);
        }
    }

    let mut writer = csv::Writer::from_path(OUTPUT_FILE)?;
    write_header(&mut writer)?;

    let mut total_rows = 0usize;
    let mut street_counts: HashMap<&'static str, usize> = HashMap::new();
    for solve in flop_solves.iter().chain(turn_solves.iter()) {
        let rows = export_decision_rows(&mut writer, solve)?;
        println!(
            "Rows for {} {} {} / {}: {}",
            solve.meta.street, solve.meta.flop_action, solve.meta.flop_bet_size, solve.meta.turn_bet_size, rows
        );
        total_rows += rows;
        *street_counts.entry(solve.meta.street).or_default() += rows;
    }

    writer.flush()?;
    println!("Street row counts: {street_counts:?}");
    println!("Done. Written {total_rows} rows to {OUTPUT_FILE}");
    println!("Example spot_name: btn-bb-srp-ah7s4c-6s");
    Ok(())
}

fn solve_flop_sizing(sizing: Sizing) -> Result<DecisionSolve, Box<dyn Error>> {
    let mut game = build_flop_game(sizing, BTN_RANGE.parse()?, BB_RANGE.parse()?)?;
    game.allocate_memory(false);
    solve(&mut game, SOLVE_ITERATIONS, 0.5, true);
    move_to_ip_decision(&mut game)?;
    extract_decision(
        game,
        DecisionMeta {
            street: "flop",
            board: FLOP,
            turn_card: "null",
            flop_action: "null",
            flop_bet_size: sizing.csv_size,
            turn_bet_size: "null",
        },
    )
}

fn solve_flop_all_sizes_for_xx() -> Result<DecisionSolve, Box<dyn Error>> {
    println!("Running dedicated xx-source flop solve with BTN sizes 33%, 75%, 125%");
    let mut game = build_flop_all_sizes_game(BTN_RANGE.parse()?, BB_RANGE.parse()?)?;
    game.allocate_memory(false);
    solve(&mut game, SOLVE_ITERATIONS, 0.5, true);
    move_to_ip_decision(&mut game)?;
    extract_decision(
        game,
        DecisionMeta {
            street: "flop",
            board: FLOP,
            turn_card: "null",
            flop_action: "xx-source",
            flop_bet_size: "multi",
            turn_bet_size: "null",
        },
    )
}

fn solve_turn_sizing(
    flop_action: &'static str,
    flop_bet_size: &'static str,
    sizing: Sizing,
    pot: i32,
    stack: i32,
    hero_hands: &[HandCombo],
    villain_hands: &[HandCombo],
) -> Result<DecisionSolve, Box<dyn Error>> {
    let turn_board = turn_board_cards()?;
    let hero_live = filter_dead_hands(hero_hands, &turn_board);
    let villain_live = filter_dead_hands(villain_hands, &turn_board);
    let ip_range = range_from_hands(&hero_live)?;
    let oop_range = range_from_hands(&villain_live)?;

    let mut game = build_turn_game(sizing, pot, stack, ip_range, oop_range)?;
    game.allocate_memory(false);
    solve(&mut game, SOLVE_ITERATIONS, 0.5, true);
    move_to_ip_decision(&mut game)?;
    extract_decision(
        game,
        DecisionMeta {
            street: "turn",
            board: TURN_BOARD,
            turn_card: TURN,
            flop_action,
            flop_bet_size,
            turn_bet_size: sizing.csv_size,
        },
    )
}

fn build_flop_game(sizing: Sizing, ip_range: Range, oop_range: Range) -> Result<PostFlopGame, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [oop_range, ip_range],
        flop: flop_from_str(FLOP)?,
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };
    let tree_config = TreeConfig {
        initial_state: BoardState::Flop,
        starting_pot: STARTING_POT,
        effective_stack: STARTING_STACK,
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
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    Ok(PostFlopGame::with_config(card_config, ActionTree::new(tree_config)?)?)
}

fn build_flop_all_sizes_game(ip_range: Range, oop_range: Range) -> Result<PostFlopGame, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [oop_range, ip_range],
        flop: flop_from_str(FLOP)?,
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };
    let tree_config = TreeConfig {
        initial_state: BoardState::Flop,
        starting_pot: STARTING_POT,
        effective_stack: STARTING_STACK,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [
            Default::default(),
            BetSizeOptions::try_from(("33%, 75%, 125%", ""))?,
        ],
        turn_bet_sizes: Default::default(),
        river_bet_sizes: Default::default(),
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    Ok(PostFlopGame::with_config(card_config, ActionTree::new(tree_config)?)?)
}

fn build_turn_game(
    sizing: Sizing,
    pot: i32,
    stack: i32,
    ip_range: Range,
    oop_range: Range,
) -> Result<PostFlopGame, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [oop_range, ip_range],
        flop: flop_from_str(FLOP)?,
        turn: parse_card(TURN)?,
        river: NOT_DEALT,
    };
    let tree_config = TreeConfig {
        initial_state: BoardState::Turn,
        starting_pot: pot,
        effective_stack: stack,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: Default::default(),
        turn_bet_sizes: [
            Default::default(),
            BetSizeOptions::try_from((sizing.tree_size, ""))?,
        ],
        river_bet_sizes: Default::default(),
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    Ok(PostFlopGame::with_config(card_config, ActionTree::new(tree_config)?)?)
}

fn build_turn_donk_filter_game(
    pot: i32,
    stack: i32,
    ip_range: Range,
    oop_range: Range,
) -> Result<PostFlopGame, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [oop_range, ip_range],
        flop: flop_from_str(FLOP)?,
        turn: parse_card(TURN)?,
        river: NOT_DEALT,
    };
    let tree_config = TreeConfig {
        initial_state: BoardState::Turn,
        starting_pot: pot,
        effective_stack: stack,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: Default::default(),
        turn_bet_sizes: [
            BetSizeOptions::try_from(("50%", ""))?,
            // DONK_FILTER_BTN_CAN_RAISE
            // IP/BTN can raise 3x after facing OOP/BB's 50% turn donk in this filter solve.
            BetSizeOptions::try_from(("50%, 100%", "3x"))?,
        ],
        river_bet_sizes: Default::default(),
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    Ok(PostFlopGame::with_config(card_config, ActionTree::new(tree_config)?)?)
}

fn filter_bb_turn_checks(
    flop_action: &'static str,
    pot: i32,
    stack: i32,
    hero_hands: &[HandCombo],
    villain_hands: &[HandCombo],
) -> Result<Vec<HandCombo>, Box<dyn Error>> {
    let turn_board = turn_board_cards()?;
    let hero_live = filter_dead_hands(hero_hands, &turn_board);
    let villain_live = filter_dead_hands(villain_hands, &turn_board);

    if hero_live.is_empty() || villain_live.is_empty() {
        println!(
            "Turn branch {flop_action}: BB donk filter skipped because a live range is empty"
        );
        return Ok(Vec::new());
    }

    let ip_range = range_from_hands(&hero_live)?;
    let oop_range = range_from_hands(&villain_live)?;
    let mut game = build_turn_donk_filter_game(pot, stack, ip_range, oop_range)?;
    game.allocate_memory(false);
    solve(&mut game, SOLVE_ITERATIONS, 0.5, true);
    game.back_to_root();
    game.cache_normalized_weights();

    if game.current_player() != OOP_PLAYER {
        println!(
            "Turn branch {flop_action}: expected OOP to act for donk filter, current player={}",
            game.current_player()
        );
        return Ok(villain_live);
    }

    let actions = game.available_actions();
    println!("Turn branch {flop_action}: BB root actions = {actions:?}");
    println!("Turn branch {flop_action}: BB donk actions available: {actions:?}");
    let check_index = actions.iter().position(|action| matches!(action, Action::Check));
    let bet_index = actions.iter().position(|action| matches!(action, Action::Bet(_)));
    println!(
        "Turn branch {flop_action}: check_index={check_index:?}, bet_index={bet_index:?}"
    );
    if check_index.is_none() || bet_index.is_none() {
        println!(
            "Turn branch {flop_action}: no BB donk bet action found; keeping all {} combos",
            villain_live.len()
        );
        return Ok(villain_live);
    }

    // BetSizeOptions::try_from((bet_sizes, raise_sizes)): the first string is the
    // acting player's bet sizes; the second string is the opponent's raise sizes
    // after facing that bet. In this diagnostic donk-filter tree OOP uses
    // ("50%", ""), so BTN may have no raise sizes versus BB's donk.
    log_btn_response_to_donk(&mut game, flop_action, bet_index.expect("checked above"))?;
    game.back_to_root();
    game.cache_normalized_weights();

    let strategy = game.strategy().to_vec();
    let hand_count = game.private_cards(OOP_PLAYER).len();
    let villains = player_hands(&game, OOP_PLAYER)?;
    let mut kept = Vec::new();
    let mut removed = Vec::new();
    let mut check_sum = 0.0;
    let mut bet_sum = 0.0;
    let mut check_gt_70 = 0;
    let mut check_50_70 = 0;
    let mut check_30_50 = 0;
    let mut check_lt_30 = 0;
    let mut removed_examples = Vec::new();
    let check_index = check_index.expect("checked above");
    let bet_index = bet_index.expect("checked above");

    for villain in villains {
        let check_freq = action_value(&strategy, check_index, villain.index, hand_count);
        let bet_freq = action_value(&strategy, bet_index, villain.index, hand_count);
        check_sum += check_freq;
        bet_sum += bet_freq;
        if check_freq > 0.70 {
            check_gt_70 += 1;
        } else if check_freq >= 0.50 {
            check_50_70 += 1;
        } else if check_freq >= 0.30 {
            check_30_50 += 1;
        } else {
            check_lt_30 += 1;
        }

        match modal_action_for_hand(&actions, &strategy, villain.index, hand_count) {
            ModalAction::Check => kept.push(villain),
            ModalAction::Bet => {
                if removed_examples.len() < 8 {
                    removed_examples.push(format!(
                        "{} check={:.3} donk={:.3}",
                        villain.label, check_freq, bet_freq
                    ));
                }
                removed.push(villain);
            }
            _ => kept.push(villain),
        }
    }

    println!("Turn branch {flop_action}: BB combos before filter = {}", hand_count);
    println!(
        "Turn branch {flop_action}: avg check freq = {:.4}",
        check_sum / hand_count as f32
    );
    println!(
        "Turn branch {flop_action}: avg donk freq = {:.4}",
        bet_sum / hand_count as f32
    );
    if flop_action == "xx" {
        println!("XX branch BB turn filtering:");
        println!("total combos before = {}", hand_count);
        println!("avg check freq = {:.4}", check_sum / hand_count as f32);
        println!("avg donk freq = {:.4}", bet_sum / hand_count as f32);
        println!("check > 70% combos = {check_gt_70}");
        println!("check 50-70% combos (including 50%) = {check_50_70}");
        println!("check 30-50% combos = {check_30_50}");
        println!("check < 30% combos = {check_lt_30}");
    }
    println!("Turn branch {flop_action}: check > 70% combos = {check_gt_70}");
    println!("Turn branch {flop_action}: check 50-70% combos (including 50%) = {check_50_70}");
    println!("Turn branch {flop_action}: check 30-50% combos = {check_30_50}");
    println!("Turn branch {flop_action}: check < 30% combos = {check_lt_30}");
    println!(
        "Turn branch {flop_action}: modal donk removed = {}",
        removed.len()
    );
    println!(
        "Turn branch {flop_action}: modal check kept = {}",
        kept.len()
    );
    println!("Turn branch {flop_action}: sample removed donks = {removed_examples:?}");

    Ok(kept)
}

fn log_btn_response_to_donk(
    game: &mut PostFlopGame,
    flop_action: &'static str,
    bet_index: usize,
) -> Result<(), Box<dyn Error>> {
    game.play(bet_index);
    println!(
        "Turn branch {flop_action}: after BB donk, current_player = {}",
        game.current_player()
    );
    let response_actions = game.available_actions();
    println!(
        "Turn branch {flop_action}: BTN response actions vs donk = {response_actions:?}"
    );
    let raise_available = response_actions
        .iter()
        .any(|action| matches!(action, Action::Raise(_)));
    println!("Turn branch {flop_action}: BTN raise vs donk available = {raise_available}");
    if !raise_available {
        println!(
            "Turn branch {flop_action}: BTN cannot raise versus BB turn donk in the current donk-filter tree. This may make BB donk frequencies too high."
        );
    }

    Ok(())
}

fn extract_decision(mut game: PostFlopGame, meta: DecisionMeta) -> Result<DecisionSolve, Box<dyn Error>> {
    game.cache_normalized_weights();
    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let bet_index = find_action(&actions, |action| matches!(action, Action::Bet(_)))?;
    let hand_count = game.private_cards(IP_PLAYER).len();
    let villain_count = game.private_cards(OOP_PLAYER).len();
    let hands = player_hands(&game, IP_PLAYER)?;
    let villains = player_hands(&game, OOP_PLAYER)?;
    let strategy = game.strategy().to_vec();
    let evs = game.expected_values_detail(IP_PLAYER).to_vec();
    let hero_weights = game.normalized_weights(IP_PLAYER).to_vec();
    let hero_equities = game.equity(IP_PLAYER).to_vec();
    let villain_equities = game.equity(OOP_PLAYER).to_vec();
    let range_stats = calculate_range_stats(
        &hero_equities,
        &villain_equities,
        &hero_weights,
        hand_count,
        villain_count,
    );
    let villain_weights = villain_weighted_combos(&villains, &villain_equities);

    Ok(DecisionSolve {
        meta,
        hands,
        villains,
        strategy,
        evs,
        hero_equities,
        check_index,
        bet_index,
        hand_count,
        range_stats,
        villain_weights,
    })
}

fn inspect_flop_branches(solve: &DecisionSolve) -> Result<FlopBranchData, Box<dyn Error>> {
    let sizing = match solve.meta.flop_bet_size {
        "0.33" => Sizing { tree_size: "33%", csv_size: "0.33", suffix: "33" },
        "0.75" => Sizing { tree_size: "75%", csv_size: "0.75", suffix: "75" },
        _ => Sizing { tree_size: "125%", csv_size: "1.25", suffix: "125" },
    };
    let mut game = build_flop_game(sizing, BTN_RANGE.parse()?, BB_RANGE.parse()?)?;
    game.allocate_memory(false);
    solve_game_for_inspection(&mut game);
    move_to_ip_decision(&mut game)?;
    game.play(solve.bet_index);
    game.cache_normalized_weights();
    let actions = game.available_actions();
    let call_index = actions.iter().position(|action| matches!(action, Action::Call));
    let raise_available = actions.iter().any(|action| matches!(action, Action::Raise(_)));
    let strategy = game.strategy().to_vec();
    let hand_count = game.private_cards(OOP_PLAYER).len();
    let villains = player_hands(&game, OOP_PLAYER)?;
    let mut bb_call_hands = Vec::new();

    if let Some(call_index) = call_index {
        for villain in &villains {
            if modal_action_for_hand(&actions, &strategy, villain.index, hand_count) == ModalAction::Call {
                let _ = call_index;
                bb_call_hands.push(villain.clone());
            }
        }
    }

    Ok(FlopBranchData {
        bb_call_hands,
        bb_raise_available: raise_available,
    })
}

struct FlopBranchData {
    bb_call_hands: Vec<HandCombo>,
    bb_raise_available: bool,
}

fn solve_game_for_inspection(game: &mut PostFlopGame) {
    let _ = solve(game, SOLVE_ITERATIONS, 0.5, true);
}

fn export_decision_rows(writer: &mut csv::Writer<std::fs::File>, solve: &DecisionSolve) -> Result<usize, Box<dyn Error>> {
    let board_cards = board_cards_for_meta(&solve.meta)?;
    let mut rows = 0;
    for hand in &solve.hands {
        let values = row_values(solve, hand.index);
        let hero_equity_vs_villain = raw_equity_vs_villain(hand.cards, &solve.villains, &board_cards);
        let (blocks_value, blocks_fold) = blocker_totals(hand.cards, &solve.villain_weights);
        writer.write_record([
            "btn-bb-srp-ah7s4c-6s".to_string(),
            solve.meta.street.to_string(),
            hand.label.clone(),
            solve.meta.board.to_string(),
            solve.meta.turn_card.to_string(),
            "ip".to_string(),
            "srp".to_string(),
            solve.meta.flop_action.to_string(),
            solve.meta.flop_bet_size.to_string(),
            solve.meta.turn_bet_size.to_string(),
            format_float(values.check_freq),
            format_float(values.check_ev),
            solve_bet_size_label(solve).to_string(),
            format_float(values.bet_freq),
            format_float(values.bet_ev),
            values.best_action.to_string(),
            format_float(values.ev),
            format_float(hero_equity_vs_villain),
            format_float(solve.hero_equities[hand.index]),
            format_float(solve.range_stats.hero_weighted_value_combos),
            format_float(solve.range_stats.villain_weighted_value_combos),
            format_float(solve.range_stats.hero_total_live_combos),
            format_float(solve.range_stats.villain_total_live_combos),
            format_float(solve.range_stats.hero_weighted_value_pct),
            format_float(solve.range_stats.villain_weighted_value_pct),
            format_float(solve.range_stats.nut_advantage_pct),
            format_float(solve.range_stats.villain_weighted_fold_combos),
            format_float(blocks_value),
            format_float(blocks_fold),
            format_float(solve.range_stats.range_equity_hero),
        ])?;
        if rows < 3 {
            println!(
                "Example row: street={} hand={} flop_action={} flop_bet_size={} turn_bet_size={}",
                solve.meta.street,
                hand.label,
                solve.meta.flop_action,
                solve.meta.flop_bet_size,
                solve.meta.turn_bet_size
            );
        }
        rows += 1;
    }
    Ok(rows)
}

fn write_header(writer: &mut csv::Writer<std::fs::File>) -> Result<(), Box<dyn Error>> {
    writer.write_record([
        "spot_name",
        "street",
        "hand",
        "board",
        "turn_card",
        "player_type",
        "pot_type",
        "flop_action",
        "flop_bet_size",
        "turn_bet_size",
        "check_freq",
        "check_ev",
        "bet_1_size",
        "bet_1_freq",
        "bet_1_ev",
        "best_action",
        "ev",
        "hero_equity_vs_villain",
        "equity_with_draws",
        "hero_weighted_value_combos",
        "villain_weighted_value_combos",
        "hero_total_live_combos",
        "villain_total_live_combos",
        "hero_weighted_value_pct",
        "villain_weighted_value_pct",
        "nut_advantage_pct",
        "villain_weighted_fold_combos",
        "hero_blocks_value_combos",
        "hero_blocks_fold_combos",
        "range_equity_hero",
    ])?;
    Ok(())
}

fn move_to_ip_decision(game: &mut PostFlopGame) -> Result<(), Box<dyn Error>> {
    game.back_to_root();
    if game.current_player() == IP_PLAYER {
        return Ok(());
    }
    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    game.play(check_index);
    if game.current_player() != IP_PLAYER {
        return Err(format!("expected IP to act, got player {}", game.current_player()).into());
    }
    Ok(())
}

fn branch_hero_hands(solve: &DecisionSolve, needed: ModalAction) -> Vec<HandCombo> {
    solve
        .hands
        .iter()
        .filter(|hand| {
            let check = action_value(&solve.strategy, solve.check_index, hand.index, solve.hand_count);
            let bet = action_value(&solve.strategy, solve.bet_index, hand.index, solve.hand_count);
            let modal = if bet > check { ModalAction::Bet } else { ModalAction::Check };
            // Tie-breaker: check wins ties over bet.
            modal == needed
        })
        .cloned()
        .collect()
}

fn row_values(solve: &DecisionSolve, hand_index: usize) -> RowValues {
    let check_freq = action_value(&solve.strategy, solve.check_index, hand_index, solve.hand_count);
    let check_ev = ev_to_bb(action_value(&solve.evs, solve.check_index, hand_index, solve.hand_count));
    let bet_freq = action_value(&solve.strategy, solve.bet_index, hand_index, solve.hand_count);
    let bet_ev = ev_to_bb(action_value(&solve.evs, solve.bet_index, hand_index, solve.hand_count));
    let (best_action, ev) = if bet_ev > check_ev {
        ("bet_1", bet_ev)
    } else {
        ("check", check_ev)
    };
    RowValues {
        check_freq,
        check_ev,
        bet_freq,
        bet_ev,
        best_action,
        ev,
    }
}

fn modal_action_for_hand(actions: &[Action], strategy: &[f32], hand_index: usize, hand_count: usize) -> ModalAction {
    let mut best = (ModalAction::Other, -1.0f32);
    for (action_index, action) in actions.iter().enumerate() {
        let freq = action_value(strategy, action_index, hand_index, hand_count);
        let modal = match action {
            Action::Check => ModalAction::Check,
            Action::Bet(_) | Action::AllIn(_) => ModalAction::Bet,
            Action::Fold => ModalAction::Fold,
            Action::Call => ModalAction::Call,
            Action::Raise(_) => ModalAction::Raise,
            _ => ModalAction::Other,
        };
        // Deterministic tie-breaker: earlier action in solver action list wins ties.
        if freq > best.1 {
            best = (modal, freq);
        }
    }
    best.0
}

fn player_hands(game: &PostFlopGame, player: usize) -> Result<Vec<HandCombo>, Box<dyn Error>> {
    let cards = game.private_cards(player);
    let labels = holes_to_strings(cards)?;
    Ok(cards
        .iter()
        .enumerate()
        .map(|(index, &(a, b))| HandCombo {
            cards: (a, b),
            label: labels[index].clone(),
            index,
        })
        .collect())
}

fn calculate_range_stats(
    hero_equities: &[f32],
    villain_equities: &[f32],
    hero_weights: &[f32],
    hero_count: usize,
    villain_count: usize,
) -> RangeStats {
    let hero_weighted_value_combos = hero_equities.iter().take(hero_count).map(|&eq| value_weight(eq)).sum();
    let villain_weighted_value_combos = villain_equities.iter().take(villain_count).map(|&eq| value_weight(eq)).sum();
    let villain_weighted_fold_combos = villain_equities.iter().take(villain_count).map(|&eq| fold_weight(eq)).sum();
    let hero_total_live_combos = hero_count as f32;
    let villain_total_live_combos = villain_count as f32;
    let hero_weighted_value_pct = hero_weighted_value_combos / hero_total_live_combos;
    let villain_weighted_value_pct = villain_weighted_value_combos / villain_total_live_combos;
    // postflop-solver's `game.equity(player)` returns per-private-hand equities,
    // not a single range-equity scalar. Convert it to range equity by weighting
    // each combo by `normalized_weights(player)` at the current decision point.
    let (weighted_equity_sum, weight_sum) = hero_equities
        .iter()
        .zip(hero_weights.iter())
        .take(hero_count)
        .fold((0.0, 0.0), |(eq_sum, w_sum), (&equity, &weight)| {
            (eq_sum + equity * weight, w_sum + weight)
        });
    let range_equity_hero = if weight_sum == 0.0 {
        0.0
    } else {
        weighted_equity_sum / weight_sum
    };
    RangeStats {
        hero_weighted_value_combos,
        villain_weighted_value_combos,
        hero_total_live_combos,
        villain_total_live_combos,
        hero_weighted_value_pct,
        villain_weighted_value_pct,
        nut_advantage_pct: hero_weighted_value_pct - villain_weighted_value_pct,
        villain_weighted_fold_combos,
        range_equity_hero,
    }
}

fn villain_weighted_combos(villains: &[HandCombo], equities: &[f32]) -> Vec<WeightedCombo> {
    villains
        .iter()
        .map(|combo| WeightedCombo {
            cards: combo.cards,
            value_weight: value_weight(equities[combo.index]),
            fold_weight: fold_weight(equities[combo.index]),
        })
        .collect()
}

fn value_weight(equity: f32) -> f32 {
    if equity >= 0.85 {
        1.0
    } else if equity >= 0.70 {
        0.5
    } else {
        0.0
    }
}

fn fold_weight(equity: f32) -> f32 {
    if equity <= 0.25 {
        1.0
    } else if equity <= 0.35 {
        0.5
    } else {
        0.0
    }
}

fn raw_equity_vs_villain(hero: (Card, Card), villains: &[HandCombo], board: &[Card]) -> f32 {
    let mut points = 0.0;
    let mut total = 0.0;
    let hero_rank = evaluate_made_hand(hero, &board);
    for villain in villains {
        if hero_blocks_combo(villain.cards.0, villain.cards.1, hero) {
            continue;
        }
        let villain_rank = evaluate_made_hand(villain.cards, &board);
        points += match hero_rank.cmp(&villain_rank) {
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Equal => 0.5,
            std::cmp::Ordering::Less => 0.0,
        };
        total += 1.0;
    }
    if total == 0.0 { 0.0 } else { points / total }
}

fn blocker_totals(hero: (Card, Card), villain_weights: &[WeightedCombo]) -> (f32, f32) {
    let mut value = 0.0;
    let mut fold = 0.0;
    for combo in villain_weights {
        if hero_blocks_combo(combo.cards.0, combo.cards.1, hero) {
            value += combo.value_weight;
            fold += combo.fold_weight;
        }
    }
    (value, fold)
}

fn range_from_hands(hands: &[HandCombo]) -> Result<Range, Box<dyn Error>> {
    let cards: Vec<(Card, Card)> = hands.iter().map(|hand| hand.cards).collect();
    let weights = vec![1.0; cards.len()];
    Ok(Range::from_hands_weights(&cards, &weights)?)
}

fn filter_dead_hands(hands: &[HandCombo], board: &[Card]) -> Vec<HandCombo> {
    hands
        .iter()
        .filter(|hand| !board.contains(&hand.cards.0) && !board.contains(&hand.cards.1))
        .cloned()
        .collect()
}

fn board_cards_for_meta(meta: &DecisionMeta) -> Result<Vec<Card>, Box<dyn Error>> {
    if meta.street == "turn" {
        Ok(turn_board_cards()?)
    } else {
        Ok(parse_board(FLOP)?.to_vec())
    }
}

fn turn_board_cards() -> Result<Vec<Card>, Box<dyn Error>> {
    let flop = parse_board(FLOP)?;
    Ok(vec![flop[0], flop[1], flop[2], parse_card(TURN)?])
}

fn solve_bet_size_label(solve: &DecisionSolve) -> &'static str {
    if solve.meta.street == "flop" {
        solve.meta.flop_bet_size
    } else {
        solve.meta.turn_bet_size
    }
}

fn find_action(actions: &[Action], predicate: impl Fn(&Action) -> bool) -> Result<usize, Box<dyn Error>> {
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

fn format_float(value: f32) -> String {
    format!("{value:.6}")
}

fn evaluate_made_hand(hand: (Card, Card), board: &[Card]) -> u32 {
    let mut cards = vec![hand.0, hand.1];
    cards.extend_from_slice(board);
    evaluate_best_from_cards(&cards)
}

fn evaluate_best_from_cards(cards: &[Card]) -> u32 {
    let mut best = 0;
    for a in 0..(cards.len() - 4) {
        for b in (a + 1)..(cards.len() - 3) {
            for c in (b + 1)..(cards.len() - 2) {
                for d in (c + 1)..(cards.len() - 1) {
                    for e in (d + 1)..cards.len() {
                        best = best.max(evaluate_5(&[
                            cards[a], cards[b], cards[c], cards[d], cards[e],
                        ]));
                    }
                }
            }
        }
    }
    best
}

fn evaluate_5(cards: &[Card; 5]) -> u32 {
    let mut rank_counts = [0u8; 13];
    let mut suit_counts = [0u8; 4];
    for &card in cards {
        rank_counts[rank(card) as usize] += 1;
        suit_counts[suit(card) as usize] += 1;
    }
    let is_flush = suit_counts.iter().any(|&count| count == 5);
    let straight_high = straight_high(&rank_counts);
    let mut groups = Vec::new();
    for rank in (0..13).rev() {
        let count = rank_counts[rank];
        if count > 0 {
            groups.push((count, rank as u8));
        }
    }
    groups.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    if is_flush {
        if let Some(high) = straight_high {
            return encode_rank(8, &[high]);
        }
    }
    match groups.as_slice() {
        [(4, quad), (1, kicker)] => encode_rank(7, &[*quad, *kicker]),
        [(3, trips), (2, pair)] => encode_rank(6, &[*trips, *pair]),
        _ if is_flush => encode_rank(5, &ranks_desc(&rank_counts)),
        _ => {
            if let Some(high) = straight_high {
                encode_rank(4, &[high])
            } else {
                match groups.as_slice() {
                    [(3, trips), (1, k1), (1, k2)] => encode_rank(3, &[*trips, *k1, *k2]),
                    [(2, p1), (2, p2), (1, k)] => encode_rank(2, &[*p1, *p2, *k]),
                    [(2, pair), (1, k1), (1, k2), (1, k3)] => encode_rank(1, &[*pair, *k1, *k2, *k3]),
                    _ => encode_rank(0, &ranks_desc(&rank_counts)),
                }
            }
        }
    }
}

fn straight_high(rank_counts: &[u8; 13]) -> Option<u8> {
    if rank_counts[12] > 0 && rank_counts[0] > 0 && rank_counts[1] > 0 && rank_counts[2] > 0 && rank_counts[3] > 0 {
        return Some(3);
    }
    for high in (4..13).rev() {
        if (0..5).all(|offset| rank_counts[high - offset] > 0) {
            return Some(high as u8);
        }
    }
    None
}

fn ranks_desc(rank_counts: &[u8; 13]) -> Vec<u8> {
    (0..13).rev().filter(|&rank| rank_counts[rank] > 0).map(|rank| rank as u8).collect()
}

fn encode_rank(category: u8, ranks: &[u8]) -> u32 {
    let mut value = (category as u32) << 20;
    for (index, &rank) in ranks.iter().enumerate() {
        value |= (rank as u32) << (4 * (4 - index));
    }
    value
}

fn hero_blocks_combo(card_a: Card, card_b: Card, hero: (Card, Card)) -> bool {
    card_a == hero.0 || card_a == hero.1 || card_b == hero.0 || card_b == hero.1
}

fn parse_board(board: &str) -> Result<[Card; 3], Box<dyn Error>> {
    if board.len() != 6 {
        return Err(format!("board must contain exactly 3 cards: {board}").into());
    }
    Ok([parse_card(&board[0..2])?, parse_card(&board[2..4])?, parse_card(&board[4..6])?])
}

fn parse_card(card: &str) -> Result<Card, Box<dyn Error>> {
    let bytes = card.as_bytes();
    if bytes.len() != 2 {
        return Err(format!("invalid card: {card}").into());
    }
    let rank = match bytes[0] as char {
        '2' => 0,
        '3' => 1,
        '4' => 2,
        '5' => 3,
        '6' => 4,
        '7' => 5,
        '8' => 6,
        '9' => 7,
        'T' => 8,
        'J' => 9,
        'Q' => 10,
        'K' => 11,
        'A' => 12,
        _ => return Err(format!("invalid card rank: {card}").into()),
    };
    let suit = match bytes[1] as char {
        'c' => 0,
        'd' => 1,
        'h' => 2,
        's' => 3,
        _ => return Err(format!("invalid card suit: {card}").into()),
    };
    Ok(4 * rank + suit)
}

fn rank(card: Card) -> u8 {
    card / 4
}

fn suit(card: Card) -> u8 {
    card % 4
}
