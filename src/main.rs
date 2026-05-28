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
const OOP_PLAYER: usize = 0;

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
    allin_freq: Option<f32>,
    allin_ev: Option<f32>,
}

struct SharedStats {
    hero_equity_vs_villain: f32,
    equity_with_draws: f32,
    villain_weighted_value_combos: f32,
    hero_blocks_value_combos: f32,
    villain_weighted_fold_combos: f32,
    hero_blocks_fold_combos: f32,
}

struct PreSolveStats {
    hero_equity_vs_villain: f32,
    villain_weighted_value_combos: f32,
    hero_blocks_value_combos: f32,
    villain_weighted_fold_combos: f32,
    hero_blocks_fold_combos: f32,
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

    let pre_solve_stats = calculate_pre_solve_stats(&sizings[0])?;
    let mut shared_stats: Option<SharedStats> = None;

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
        "hero_equity_vs_villain",
        "equity_with_draws",
        "villain_weighted_value_combos",
        "hero_blocks_value_combos",
        "villain_weighted_fold_combos",
        "hero_blocks_fold_combos",
    ])?;

    for sizing in sizings {
        let (values, equity_with_draws) = solve_sizing(&sizing, shared_stats.is_none())?;
        let stats = shared_stats.get_or_insert_with(|| SharedStats {
            hero_equity_vs_villain: pre_solve_stats.hero_equity_vs_villain,
            equity_with_draws: equity_with_draws
                .expect("first solve must return hero equity including draws"),
            villain_weighted_value_combos: pre_solve_stats.villain_weighted_value_combos,
            hero_blocks_value_combos: pre_solve_stats.hero_blocks_value_combos,
            villain_weighted_fold_combos: pre_solve_stats.villain_weighted_fold_combos,
            hero_blocks_fold_combos: pre_solve_stats.hero_blocks_fold_combos,
        });

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
            format_optional_float(values.allin_freq),
            format_optional_float(values.allin_ev),
            "null".to_string(),
            "null".to_string(),
            "null".to_string(),
            "null".to_string(),
            "null".to_string(),
            "null".to_string(),
            "null".to_string(),
            best_action.to_string(),
            format_float(best_ev),
            format_float(stats.hero_equity_vs_villain),
            format_float(stats.equity_with_draws),
            format_float(stats.villain_weighted_value_combos),
            format_float(stats.hero_blocks_value_combos),
            format_float(stats.villain_weighted_fold_combos),
            format_float(stats.hero_blocks_fold_combos),
        ])?;
    }

    writer.flush()?;
    println!("Wrote {OUTPUT_FILE}");
    Ok(())
}

fn solve_sizing(
    sizing: &Sizing,
    capture_equity_with_draws: bool,
) -> Result<(RowValues, Option<f32>), Box<dyn Error>> {
    let mut game = build_game(sizing)?;
    game.allocate_memory(false);
    solve(&mut game, 50, 0.5, true);

    let equity_with_draws = if capture_equity_with_draws {
        game.back_to_root();
        game.cache_normalized_weights();
        let hand_index = find_hand_index(&game)?;
        Some(game.equity(HERO_PLAYER)[hand_index])
    } else {
        None
    };

    move_to_hero_root_decision(&mut game)?;
    game.cache_normalized_weights();

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let bet_1_index = find_action(&actions, |action| matches!(action, Action::Bet(_)))?;
    let allin_index = actions
        .iter()
        .position(|action| matches!(action, Action::AllIn(_)));

    let hand_index = find_hand_index(&game)?;
    let hand_count = game.private_cards(HERO_PLAYER).len();
    let strategy = game.strategy();
    let evs = game.expected_values_detail(HERO_PLAYER);

    Ok((
        RowValues {
            check_freq: action_value(&strategy, check_index, hand_index, hand_count),
            check_ev: ev_to_bb(action_value(&evs, check_index, hand_index, hand_count)),
            bet_1_freq: action_value(&strategy, bet_1_index, hand_index, hand_count),
            bet_1_ev: ev_to_bb(action_value(&evs, bet_1_index, hand_index, hand_count)),
            allin_freq: allin_index.map(|index| action_value(&strategy, index, hand_index, hand_count)),
            allin_ev: allin_index.map(|index| ev_to_bb(action_value(&evs, index, hand_index, hand_count))),
        },
        equity_with_draws,
    ))
}

fn build_game(sizing: &Sizing) -> Result<PostFlopGame, Box<dyn Error>> {
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
        add_allin_threshold: 1.5,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };

    let action_tree = ActionTree::new(tree_config)?;
    Ok(PostFlopGame::with_config(card_config, action_tree)?)
}

fn calculate_pre_solve_stats(sizing: &Sizing) -> Result<PreSolveStats, Box<dyn Error>> {
    let game = build_game(sizing)?;
    let hero = parse_hand(HAND)?;
    let board = parse_board(BOARD)?;
    let hero_made_rank = evaluate_5(&[hero.0, hero.1, board[0], board[1], board[2]]);
    let mut blocked_value_details = Vec::new();
    let mut blocked_fold_details = Vec::new();

    let mut hero_equity_points = 0.0;
    let mut total_villain_combos = 0.0;
    let mut villain_weighted_value_combos = 0.0;
    let mut hero_blocks_value_combos = 0.0;
    let mut villain_weighted_fold_combos = 0.0;
    let mut hero_blocks_fold_combos = 0.0;

    for &(villain_a, villain_b) in game.private_cards(OOP_PLAYER) {
        if overlaps_any(&[villain_a, villain_b], &[hero.0, hero.1]) {
            continue;
        }

        let villain_made_rank = evaluate_5(&[villain_a, villain_b, board[0], board[1], board[2]]);
        hero_equity_points += match hero_made_rank.cmp(&villain_made_rank) {
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Equal => 0.5,
            std::cmp::Ordering::Less => 0.0,
        };
        total_villain_combos += 1.0;

        let villain_equity = heads_up_equity_with_runouts((villain_a, villain_b), hero, board);
        let value_weight = if villain_equity >= 0.85 {
            1.0
        } else if villain_equity >= 0.70 {
            0.5
        } else {
            0.0
        };
        let fold_weight = if villain_equity <= 0.25 {
            1.0
        } else if villain_equity <= 0.35 {
            0.5
        } else {
            0.0
        };

        villain_weighted_value_combos += value_weight;
        villain_weighted_fold_combos += fold_weight;

        if value_weight > 0.0 && hero_blocks_combo(villain_a, villain_b, hero) {
            hero_blocks_value_combos += value_weight;
            blocked_value_details.push(format!(
                "{}{} equity={:.4} weight={:.1}",
                card_to_string(villain_a),
                card_to_string(villain_b),
                villain_equity,
                value_weight
            ));
        }

        if fold_weight > 0.0 && hero_blocks_combo(villain_a, villain_b, hero) {
            hero_blocks_fold_combos += fold_weight;
            blocked_fold_details.push(format!(
                "{}{} equity={:.4} weight={:.1}",
                card_to_string(villain_a),
                card_to_string(villain_b),
                villain_equity,
                fold_weight
            ));
        }
    }

    print_blocker_breakdown(
        "value",
        villain_weighted_value_combos,
        hero_blocks_value_combos,
        &blocked_value_details,
    );
    print_blocker_breakdown(
        "fold",
        villain_weighted_fold_combos,
        hero_blocks_fold_combos,
        &blocked_fold_details,
    );

    Ok(PreSolveStats {
        hero_equity_vs_villain: hero_equity_points / total_villain_combos,
        villain_weighted_value_combos,
        hero_blocks_value_combos,
        villain_weighted_fold_combos,
        hero_blocks_fold_combos,
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

fn best_action(check_ev: f32, bet_1_ev: f32, allin_ev: Option<f32>) -> (&'static str, f32) {
    let mut best_name = "check";
    let mut best_ev = check_ev;

    if bet_1_ev > best_ev {
        best_name = "bet_1";
        best_ev = bet_1_ev;
    }

    if let Some(ev) = allin_ev {
        if ev > best_ev {
            best_name = "allin";
            best_ev = ev;
        }
    }

    (best_name, best_ev)
}

fn print_sanity_check(sizing: &Sizing, values: &RowValues) {
    println!(
        "Solved {} sizing: check_ev={:.4} BB, bet_1_ev={:.4} BB, allin_ev={} BB",
        sizing.csv_size,
        values.check_ev,
        values.bet_1_ev,
        format_optional_float(values.allin_ev)
    );

    let allin_ev_failed = values
        .allin_ev
        .map(|ev| ev > 50.0 || ev < 0.0)
        .unwrap_or(false);

    if values.check_ev > 50.0
        || values.bet_1_ev > 50.0
        || values.check_ev < 0.0
        || values.bet_1_ev < 0.0
        || allin_ev_failed
    {
        eprintln!("WARNING: EV sanity check failed; EV conversion may be wrong.");
    }
}

fn format_float(value: f32) -> String {
    format!("{value:.6}")
}

fn format_optional_float(value: Option<f32>) -> String {
    value.map(format_float).unwrap_or_else(|| "null".to_string())
}

fn heads_up_equity_with_runouts(
    player: (Card, Card),
    opponent: (Card, Card),
    board: [Card; 3],
) -> f32 {
    let dead = [player.0, player.1, opponent.0, opponent.1, board[0], board[1], board[2]];
    let deck: Vec<Card> = (0..52).filter(|card| !dead.contains(card)).collect();
    let mut points = 0.0;
    let mut total = 0.0;

    for i in 0..deck.len() {
        for j in (i + 1)..deck.len() {
            let turn = deck[i];
            let river = deck[j];
            let player_rank = evaluate_best(&[
                player.0, player.1, board[0], board[1], board[2], turn, river,
            ]);
            let opponent_rank = evaluate_best(&[
                opponent.0, opponent.1, board[0], board[1], board[2], turn, river,
            ]);

            points += match player_rank.cmp(&opponent_rank) {
                std::cmp::Ordering::Greater => 1.0,
                std::cmp::Ordering::Equal => 0.5,
                std::cmp::Ordering::Less => 0.0,
            };
            total += 1.0;
        }
    }

    points / total
}

fn evaluate_best(cards: &[Card; 7]) -> u32 {
    let mut best = 0;

    for a in 0..3 {
        for b in (a + 1)..4 {
            for c in (b + 1)..5 {
                for d in (c + 1)..6 {
                    for e in (d + 1)..7 {
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
        _ if is_flush => {
            let ranks: Vec<u8> = ranks_desc(&rank_counts);
            encode_rank(5, &ranks)
        }
        _ => {
            if let Some(high) = straight_high {
                encode_rank(4, &[high])
            } else {
                match groups.as_slice() {
                    [(3, trips), (1, kicker_1), (1, kicker_2)] => {
                        encode_rank(3, &[*trips, *kicker_1, *kicker_2])
                    }
                    [(2, high_pair), (2, low_pair), (1, kicker)] => {
                        encode_rank(2, &[*high_pair, *low_pair, *kicker])
                    }
                    [(2, pair), (1, kicker_1), (1, kicker_2), (1, kicker_3)] => {
                        encode_rank(1, &[*pair, *kicker_1, *kicker_2, *kicker_3])
                    }
                    _ => {
                        let ranks: Vec<u8> = ranks_desc(&rank_counts);
                        encode_rank(0, &ranks)
                    }
                }
            }
        }
    }
}

fn straight_high(rank_counts: &[u8; 13]) -> Option<u8> {
    if rank_counts[12] > 0
        && rank_counts[0] > 0
        && rank_counts[1] > 0
        && rank_counts[2] > 0
        && rank_counts[3] > 0
    {
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
    (0..13)
        .rev()
        .filter(|&rank| rank_counts[rank] > 0)
        .map(|rank| rank as u8)
        .collect()
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

fn print_blocker_breakdown(pool_name: &str, pool_weight: f32, blocked_weight: f32, details: &[String]) {
    println!(
        "{} pool blocker breakdown: blocked_weight={:.1}, pool_weight={:.1}, blocked_combos={}",
        pool_name,
        blocked_weight,
        pool_weight,
        details.len()
    );

    if details.is_empty() {
        println!("  no blocked {pool_name} combos");
    } else {
        for detail in details {
            println!("  {detail}");
        }
    }
}

fn card_to_string(card: Card) -> String {
    let rank = match rank(card) {
        0 => '2',
        1 => '3',
        2 => '4',
        3 => '5',
        4 => '6',
        5 => '7',
        6 => '8',
        7 => '9',
        8 => 'T',
        9 => 'J',
        10 => 'Q',
        11 => 'K',
        12 => 'A',
        _ => '?',
    };
    let suit = match suit(card) {
        0 => 'c',
        1 => 'd',
        2 => 'h',
        3 => 's',
        _ => '?',
    };

    format!("{rank}{suit}")
}

fn overlaps_any(cards: &[Card], dead_cards: &[Card]) -> bool {
    cards.iter().any(|card| dead_cards.contains(card))
}

fn parse_board(board: &str) -> Result<[Card; 3], Box<dyn Error>> {
    if board.len() != 6 {
        return Err(format!("board must contain exactly 3 cards: {board}").into());
    }

    Ok([
        parse_card(&board[0..2])?,
        parse_card(&board[2..4])?,
        parse_card(&board[4..6])?,
    ])
}

fn parse_hand(hand: &str) -> Result<(Card, Card), Box<dyn Error>> {
    if hand.len() != 4 {
        return Err(format!("hand must contain exactly 2 cards: {hand}").into());
    }

    Ok((parse_card(&hand[0..2])?, parse_card(&hand[2..4])?))
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
