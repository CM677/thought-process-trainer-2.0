use postflop_solver::*;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

const OUTPUT_FILE: &str = "output4.csv";
const SPOTS_FILE: &str = "spots.json";

const OOP_PLAYER: usize = 0;
const HERO_PLAYER: usize = 1;
const BB_CHIPS: f32 = 100.0;
const STARTING_POT: i32 = 600;
const EFFECTIVE_STACK: i32 = 9750;

const BTN_RANGE: &str =
    "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s";
const BB_RANGE: &str = "99-22,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,AJo-A6o,K9o+,QTo+,JTo,QTs-Q2s,A4s-A2s,T9o";

#[derive(Debug, Deserialize)]
struct Spot {
    id: String,
    position_matchup: String,
    pot_type: String,
    board: String,
}

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
    strategy_index: usize,
}

struct SolvedSizing {
    sizing: Sizing,
    hands: Vec<HandCombo>,
    villain_cards: Vec<(Card, Card)>,
    equity_with_draws: Vec<f32>,
    strategy: Vec<f32>,
    evs: Vec<f32>,
    check_index: usize,
    bet_1_index: usize,
    allin_index: Option<usize>,
    hand_count: usize,
}

struct RowValues {
    check_freq: f32,
    check_ev: f32,
    bet_1_freq: f32,
    bet_1_ev: f32,
    allin_freq: Option<f32>,
    allin_ev: Option<f32>,
}

struct HandStats {
    hero_equity_vs_villain: f32,
    equity_with_draws: f32,
    villain_weighted_value_combos: f32,
    hero_blocks_value_combos: f32,
    villain_weighted_fold_combos: f32,
    hero_blocks_fold_combos: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let spot = load_first_spot()?;
    let board = parse_board(&spot.board)?;
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

    let solved_sizings = solve_all_sizings(&sizings, &spot.board)?;
    let first_solve = solved_sizings
        .first()
        .ok_or("no solved sizings were produced")?;
    let mut writer = csv::Writer::from_path(OUTPUT_FILE)?;
    write_header(&mut writer)?;

    let mut rows_written = 0;

    for (hand_position, hand) in first_solve.hands.iter().enumerate() {
        println!("Processing hand: {}", hand.label);
        let stats = calculate_hand_stats(
            hand.cards,
            &first_solve.villain_cards,
            first_solve.equity_with_draws[hand.strategy_index],
            board,
        );

        for solved in &solved_sizings {
            let solved_hand = &solved.hands[hand_position];
            let values = row_values(solved, solved_hand.strategy_index);
            let (best_action, best_ev) =
                best_action(values.check_ev, values.bet_1_ev, values.allin_ev);

            writer.write_record([
                format!(
                    "{}-{}-{}-flop-hero_betting_ip-{}",
                    spot.position_matchup, spot.pot_type, spot.id, solved.sizing.suffix
                ),
                "flop".to_string(),
                solved_hand.label.clone(),
                spot.board.clone(),
                "ip".to_string(),
                spot.pot_type.clone(),
                "null".to_string(),
                format_float(values.check_freq),
                format_float(values.check_ev),
                solved.sizing.csv_size.to_string(),
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
            rows_written += 1;
        }
    }

    writer.flush()?;
    println!("Done. Written {rows_written} rows to {OUTPUT_FILE}");
    Ok(())
}

fn load_first_spot() -> Result<Spot, Box<dyn Error>> {
    let path = spots_path()?;
    let file = File::open(&path)?;
    let mut spots: Vec<Spot> = serde_json::from_reader(file)?;

    if spots.is_empty() {
        return Err(format!("no spots found in {}", path.display()).into());
    }

    Ok(spots.remove(0))
}

fn spots_path() -> Result<PathBuf, Box<dyn Error>> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or("could not determine binary directory")?;
    let exe_spots = exe_dir.join(SPOTS_FILE);

    if exe_spots.exists() {
        return Ok(exe_spots);
    }

    let cwd_spots = std::env::current_dir()?.join(SPOTS_FILE);
    if cwd_spots.exists() {
        return Ok(cwd_spots);
    }

    Ok(exe_spots)
}

fn write_header(writer: &mut csv::Writer<std::fs::File>) -> Result<(), Box<dyn Error>> {
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
    Ok(())
}

fn solve_all_sizings(
    sizings: &[Sizing; 3],
    board_text: &str,
) -> Result<Vec<SolvedSizing>, Box<dyn Error>> {
    let mut solved = Vec::new();

    for sizing in sizings {
        let solved_sizing = solve_sizing(*sizing, board_text)?;
        println!("Solved {} sizing, iterating hands...", sizing.tree_size);
        solved.push(solved_sizing);
    }

    Ok(solved)
}

fn solve_sizing(sizing: Sizing, board_text: &str) -> Result<SolvedSizing, Box<dyn Error>> {
    let mut game = build_game(sizing, board_text)?;
    game.allocate_memory(false);
    solve(&mut game, 50, 0.5, true);

    game.back_to_root();
    game.cache_normalized_weights();
    let hands = live_hero_hands(&game, board_text)?;
    let board = parse_board(board_text)?;
    let villain_cards = game
        .private_cards(OOP_PLAYER)
        .iter()
        .copied()
        .filter(|(card_a, card_b)| !board.contains(card_a) && !board.contains(card_b))
        .collect();
    let equity_with_draws = game.equity(HERO_PLAYER).to_vec();

    move_to_hero_root_decision(&mut game)?;
    game.cache_normalized_weights();

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let bet_1_index = find_action(&actions, |action| matches!(action, Action::Bet(_)))?;
    let allin_index = actions
        .iter()
        .position(|action| matches!(action, Action::AllIn(_)));
    let hand_count = game.private_cards(HERO_PLAYER).len();
    let strategy = game.strategy().to_vec();
    let evs = game.expected_values_detail(HERO_PLAYER).to_vec();

    Ok(SolvedSizing {
        sizing,
        hands,
        villain_cards,
        equity_with_draws,
        strategy,
        evs,
        check_index,
        bet_1_index,
        allin_index,
        hand_count,
    })
}

fn build_game(sizing: Sizing, board_text: &str) -> Result<PostFlopGame, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [BB_RANGE.parse()?, BTN_RANGE.parse()?],
        flop: flop_from_str(board_text)?,
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

fn live_hero_hands(
    game: &PostFlopGame,
    board_text: &str,
) -> Result<Vec<HandCombo>, Box<dyn Error>> {
    let board = parse_board(board_text)?;
    let cards = game.private_cards(HERO_PLAYER);
    let labels = holes_to_strings(cards)?;
    let mut hands = Vec::new();

    for (index, &(card_a, card_b)) in cards.iter().enumerate() {
        if board.contains(&card_a) || board.contains(&card_b) {
            continue;
        }

        hands.push(HandCombo {
            cards: (card_a, card_b),
            label: labels[index].clone(),
            strategy_index: index,
        });
    }

    Ok(hands)
}

fn calculate_hand_stats(
    hero: (Card, Card),
    villain_cards: &[(Card, Card)],
    equity_with_draws: f32,
    board: [Card; 3],
) -> HandStats {
    let hero_made_rank = evaluate_5(&[hero.0, hero.1, board[0], board[1], board[2]]);
    let runouts = hero_runout_ranks(hero, board);
    let mut hero_equity_points = 0.0;
    let mut total_villain_combos = 0.0;
    let mut villain_weighted_value_combos = 0.0;
    let mut hero_blocks_value_combos = 0.0;
    let mut villain_weighted_fold_combos = 0.0;
    let mut hero_blocks_fold_combos = 0.0;

    for &(villain_a, villain_b) in villain_cards {
        let blocked = hero_blocks_combo(villain_a, villain_b, hero);

        if !blocked {
            let villain_made_rank =
                evaluate_5(&[villain_a, villain_b, board[0], board[1], board[2]]);
            hero_equity_points += match hero_made_rank.cmp(&villain_made_rank) {
                std::cmp::Ordering::Greater => 1.0,
                std::cmp::Ordering::Equal => 0.5,
                std::cmp::Ordering::Less => 0.0,
            };
            total_villain_combos += 1.0;
        }

        let villain_equity =
            villain_equity_against_cached_hero((villain_a, villain_b), board, &runouts);
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

        if blocked {
            hero_blocks_value_combos += value_weight;
            hero_blocks_fold_combos += fold_weight;
        }
    }

    HandStats {
        hero_equity_vs_villain: hero_equity_points / total_villain_combos,
        equity_with_draws,
        villain_weighted_value_combos,
        hero_blocks_value_combos,
        villain_weighted_fold_combos,
        hero_blocks_fold_combos,
    }
}

fn hero_runout_ranks(hero: (Card, Card), board: [Card; 3]) -> Vec<(Card, Card, u32)> {
    let dead = [hero.0, hero.1, board[0], board[1], board[2]];
    let deck: Vec<Card> = (0..52).filter(|card| !dead.contains(card)).collect();
    let mut runouts = Vec::new();

    for i in 0..deck.len() {
        for j in (i + 1)..deck.len() {
            let turn = deck[i];
            let river = deck[j];
            let rank = evaluate_best(&[hero.0, hero.1, board[0], board[1], board[2], turn, river]);
            runouts.push((turn, river, rank));
        }
    }

    runouts
}

fn villain_equity_against_cached_hero(
    villain: (Card, Card),
    board: [Card; 3],
    runouts: &[(Card, Card, u32)],
) -> f32 {
    let mut points = 0.0;
    let mut total = 0.0;

    for &(turn, river, hero_rank) in runouts {
        if villain.0 == turn || villain.0 == river || villain.1 == turn || villain.1 == river {
            continue;
        }

        let villain_rank = evaluate_best(&[
            villain.0, villain.1, board[0], board[1], board[2], turn, river,
        ]);
        points += match villain_rank.cmp(&hero_rank) {
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Equal => 0.5,
            std::cmp::Ordering::Less => 0.0,
        };
        total += 1.0;
    }

    if total == 0.0 {
        0.0
    } else {
        points / total
    }
}

fn row_values(solved: &SolvedSizing, hand_index: usize) -> RowValues {
    RowValues {
        check_freq: action_value(
            &solved.strategy,
            solved.check_index,
            hand_index,
            solved.hand_count,
        ),
        check_ev: ev_to_bb(action_value(
            &solved.evs,
            solved.check_index,
            hand_index,
            solved.hand_count,
        )),
        bet_1_freq: action_value(
            &solved.strategy,
            solved.bet_1_index,
            hand_index,
            solved.hand_count,
        ),
        bet_1_ev: ev_to_bb(action_value(
            &solved.evs,
            solved.bet_1_index,
            hand_index,
            solved.hand_count,
        )),
        allin_freq: solved
            .allin_index
            .map(|index| action_value(&solved.strategy, index, hand_index, solved.hand_count)),
        allin_ev: solved.allin_index.map(|index| {
            ev_to_bb(action_value(
                &solved.evs,
                index,
                hand_index,
                solved.hand_count,
            ))
        }),
    }
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

fn format_float(value: f32) -> String {
    format!("{value:.6}")
}

fn format_optional_float(value: Option<f32>) -> String {
    value.map(format_float).unwrap_or_else(|| "null".to_string())
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
        _ if is_flush => encode_rank(5, &ranks_desc(&rank_counts)),
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
                    _ => encode_rank(0, &ranks_desc(&rank_counts)),
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
