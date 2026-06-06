use postflop_solver::*;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::path::Path;

const SOLVE_ITERATIONS: u32 = 500;
const BB_CHIPS: f32 = 100.0;
const STARTING_STACK_BB: f64 = 100.0;
const INPUT_FILE: &str = "stat_probe_input.csv";
const OUTPUT_FILE: &str = "stat_probe_output.csv";
const IP_PLAYER: usize = 1;
const OOP_PLAYER: usize = 0;

const BTN_OPEN_RANGE: &str =
    "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s";
const BB_VS_BTN_RANGE: &str =
    "99-22,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,AJo-A6o,K9o+,QTo+,JTo,QTs-Q2s,A4s-A2s,T9o";

const POSTFLOP_SIZINGS: [PostflopSizing; 3] = [
    PostflopSizing {
        label: "33%",
        suffix: "33",
        pct: 0.33,
        tree_size: "33%",
    },
    PostflopSizing {
        label: "75%",
        suffix: "75",
        pct: 0.75,
        tree_size: "75%",
    },
    PostflopSizing {
        label: "125%",
        suffix: "125",
        pct: 1.25,
        tree_size: "125%",
    },
];

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct PostflopSizing {
    label: &'static str,
    suffix: &'static str,
    pct: f64,
    tree_size: &'static str,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct SpotSizeConfig {
    spot_type: &'static str,
    hero_position: &'static str,
    villain_position: Option<&'static str>,
    bet_size_bb: f64,
    pot_before_bb: f64,
}

#[derive(Clone)]
struct ProbeSpot {
    spot_id: String,
    spot_type: String,
    hero_position: String,
    villain_position: Option<String>,
    board: String,
    hand: Option<String>,
}

#[derive(Clone)]
struct HandCombo {
    cards: (Card, Card),
    label: String,
    index: usize,
}

struct DecisionSolve {
    hands: Vec<HandCombo>,
    villains: Vec<HandCombo>,
    strategy: Vec<f32>,
    evs: Vec<f32>,
    hero_equities: Vec<f32>,
    check_index: usize,
    bet_index: usize,
    hand_count: usize,
    range_stats: RangeStats,
}

struct RangeStats {
    hero_weighted_value_combos: f32,
    villain_weighted_value_combos: f32,
    hero_total_live_combos: f32,
    villain_total_live_combos: f32,
    hero_weighted_value_pct: f32,
    villain_weighted_value_pct: f32,
    nut_advantage_pct: f32,
    range_equity_hero: f32,
}

struct RowValues {
    check_freq: f32,
    bet_freq: f32,
    check_ev: f32,
    bet_ev: f32,
    best_action: &'static str,
    ev: f32,
}

#[derive(Clone, Copy)]
struct PostflopPotConfig {
    starting_pot_bb: f64,
    effective_stack_bb: f64,
}

#[derive(Default)]
struct SkipSummary {
    invalid_board: usize,
    unsupported_4bp: usize,
    no_range_pair: usize,
    hand_not_in_range: usize,
    no_config: usize,
    invalid_hand: usize,
    blank_hand: usize,
    solve_failed: usize,
    duplicate_rows: usize,
}

impl SkipSummary {
    fn print(&self) {
        println!(
            "Skipped summary: invalid board={}, unsupported 4bp={}, no range pair={}, hand not in range={}, no config={}, invalid hand={}, blank hand={}, solve failed={}, duplicate rows={}",
            self.invalid_board,
            self.unsupported_4bp,
            self.no_range_pair,
            self.hand_not_in_range,
            self.no_config,
            self.invalid_hand,
            self.blank_hand,
            self.solve_failed,
            self.duplicate_rows,
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let input_rows = load_input_rows()?;
    let mut writer = csv::Writer::from_path(OUTPUT_FILE)?;
    write_header(&mut writer)?;

    let mut rows_written = 0usize;
    let mut seen = HashSet::new();
    let mut skipped = SkipSummary::default();

    for (row_index, input) in input_rows.iter().enumerate() {
        println!("Running stat probe row {}/{}", row_index + 1, input_rows.len());

        let board = match parse_board(&input.board) {
            Ok(board) => board,
            Err(error) => {
                eprintln!("Skipping spot {}: invalid board {}: {error}", input.spot_id, input.board);
                skipped.invalid_board += 1;
                continue;
            }
        };
        let flop_board = board_to_string(&board);

        if let Some(hand) = input.hand.as_deref() {
            match parse_hand(hand) {
                Ok(cards) => {
                    if board.contains(&cards.0) || board.contains(&cards.1) {
                        eprintln!(
                            "Skipping spot {}: hand {} contains a board card on {}",
                            input.spot_id, hand, input.board
                        );
                        skipped.hand_not_in_range += 1;
                        continue;
                    }
                }
                Err(error) => {
                    eprintln!("Skipping spot {}: invalid hand {}: {error}", input.spot_id, hand);
                    skipped.invalid_hand += 1;
                    continue;
                }
            }
        } else {
            eprintln!("Skipping spot {}: blank Hand is not expanded in this patch", input.spot_id);
            skipped.blank_hand += 1;
            continue;
        }

        println!(
            "Running spot {}: {} {} vs {} hand={} full_board={} flop={}",
            input.spot_id,
            input.spot_type,
            input.hero_position,
            input.villain_position.as_deref().unwrap_or(""),
            input.hand.as_deref().unwrap_or("ALL"),
            input.board,
            flop_board
        );

        if input.spot_type == "4bp" {
            eprintln!("Skipping spot {}: 4bp not implemented yet", input.spot_id);
            skipped.unsupported_4bp += 1;
            continue;
        }

        let Some(pot_config) = postflop_pot_config(input) else {
            eprintln!(
                "Skipping spot {}: no postflop pot config for pot_type={} hero={} villain={}",
                input.spot_id,
                input.spot_type,
                input.hero_position,
                input.villain_position.as_deref().unwrap_or("")
            );
            skipped.no_config += 1;
            continue;
        };

        let Some(range_pair) = lookup_ranges(input) else {
            eprintln!(
                "Skipping spot {}: no range pair available for pot_type={} hero={} villain={}",
                input.spot_id,
                input.spot_type,
                input.hero_position,
                input.villain_position.as_deref().unwrap_or("")
            );
            skipped.no_range_pair += 1;
            continue;
        };

        let hero_is_ip = is_in_position(&input.hero_position, input.villain_position.as_deref());
        let hero_player = if hero_is_ip { IP_PLAYER } else { OOP_PLAYER };
        let starting_pot_bb = pot_config.starting_pot_bb;
        let starting_pot = bb_to_chips(starting_pot_bb);
        let effective_stack = bb_to_chips(pot_config.effective_stack_bb);

        for sizing in POSTFLOP_SIZINGS {
            println!("Running sizing {}", sizing.label);
            let solve = match solve_spot(
                board,
                range_pair.hero_range,
                range_pair.villain_range,
                hero_is_ip,
                hero_player,
                sizing,
                starting_pot,
                effective_stack,
            ) {
                Ok(solve) => solve,
                Err(error) => {
                    eprintln!(
                        "Skipping spot {} sizing {}: solve failed for {} {} {} {}: {error}",
                        input.spot_id,
                        sizing.label,
                        input.spot_type,
                        input.hero_position,
                        input.villain_position.as_deref().unwrap_or(""),
                        input.board
                    );
                    skipped.solve_failed += 1;
                    continue;
                }
            };

            let target_hand = match input.hand.as_deref().map(str::trim).filter(|hand| !hand.is_empty()) {
                Some(hand) => {
                    let normalized = normalize_hand_label(hand);
                    let Some(combo) = solve.hands.iter().find(|combo| normalize_hand_label(&combo.label) == normalized) else {
                        eprintln!(
                            "Skipping spot {}: hand {} is not legal/in-range on board {}",
                            input.spot_id, hand, input.board
                        );
                        skipped.hand_not_in_range += 1;
                        continue;
                    };
                    vec![combo.clone()]
                }
                None => solve.hands.clone(),
            };

            for hand in target_hand {
                let key = format!(
                    "{}|{}|{}|{}|{}|{}|{}",
                    input.spot_id,
                    input.spot_type,
                    input.hero_position,
                    input.villain_position.as_deref().unwrap_or(""),
                    flop_board,
                    hand.label,
                    sizing.label
                );
                if !seen.insert(key) {
                    skipped.duplicate_rows += 1;
                    continue;
                }

                let values = row_values(&solve, hand.index);
                let hero_equity_vs_villain = raw_equity_vs_villain(hand.cards, &solve.villains, &board);
                let equity_with_draws = solve.hero_equities[hand.index];
                let raw_strength_score = raw_strength_score(hero_equity_vs_villain);
                let improvability_score = improvability_score(equity_with_draws - hero_equity_vs_villain);
                let range_advantage_score = range_advantage_score(solve.range_stats.range_equity_hero);
                let nut_advantage_score = nut_advantage_score(solve.range_stats.nut_advantage_pct);
                let spot_id = input.spot_id.clone();

                writer.write_record([
                    input.spot_type.clone(),
                    input.hero_position.clone(),
                    input.villain_position.clone().unwrap_or_default(),
                    flop_board.clone(),
                    hand.label,
                    sizing.label.to_string(),
                    format_f64(starting_pot_bb * sizing.pct),
                    format_f64(starting_pot_bb),
                    format_float(values.check_freq),
                    format_float(values.bet_freq),
                    format_float(values.check_ev),
                    format_float(values.bet_ev),
                    values.best_action.to_string(),
                    format_float(values.ev),
                    format_float(hero_equity_vs_villain),
                    format_float(equity_with_draws),
                    raw_strength_score.to_string(),
                    improvability_score.to_string(),
                    format_float(solve.range_stats.range_equity_hero),
                    range_advantage_score.to_string(),
                    format_float(solve.range_stats.hero_weighted_value_combos),
                    format_float(solve.range_stats.villain_weighted_value_combos),
                    format_float(solve.range_stats.hero_total_live_combos),
                    format_float(solve.range_stats.villain_total_live_combos),
                    format_float(solve.range_stats.hero_weighted_value_pct),
                    format_float(solve.range_stats.villain_weighted_value_pct),
                    format_float(solve.range_stats.nut_advantage_pct),
                    nut_advantage_score.to_string(),
                    SOLVE_ITERATIONS.to_string(),
                    spot_id,
                ])?;
                rows_written += 1;
            }

            println!("Rows written so far: {rows_written}");
        }
    }

    writer.flush()?;
    println!("Done. Written {rows_written} rows to {OUTPUT_FILE}");
    skipped.print();
    Ok(())
}

fn load_input_rows() -> Result<Vec<ProbeSpot>, Box<dyn Error>> {
    if !Path::new(INPUT_FILE).exists() {
        println!("{INPUT_FILE} not found. Generating deterministic fallback sample.");
        return Ok(fallback_sample());
    }

    let file = File::open(INPUT_FILE)?;
    let mut reader = csv::Reader::from_reader(file);
    let headers = reader.headers()?.clone();
    let input_format = detect_input_format(&headers)?;
    println!("Loaded {INPUT_FILE}");
    println!("Detected input format: {}", input_format.label());
    let mut rows = Vec::new();

    for (index, record) in reader.records().enumerate() {
        match record {
            Ok(record) => match normalize_record(&headers, &record, input_format, index + 1) {
                Some(row) => rows.push(row),
                None => eprintln!("Skipping input row {}: invalid row", index + 1),
            },
            Err(error) => eprintln!("Skipping input row {}: failed to parse CSV record: {error}", index + 1),
        }
    }
    println!("Rows loaded: {}", rows.len());

    if rows.is_empty() {
        println!("{INPUT_FILE} contained no valid rows. Using deterministic fallback sample.");
        Ok(fallback_sample())
    } else {
        Ok(rows)
    }
}

#[derive(Clone, Copy)]
enum InputFormat {
    NewSpreadsheet,
    OldStatProbe,
}

impl InputFormat {
    fn label(self) -> &'static str {
        match self {
            InputFormat::NewSpreadsheet => "new spreadsheet format",
            InputFormat::OldStatProbe => "old stat_probe format",
        }
    }
}

fn detect_input_format(headers: &csv::StringRecord) -> Result<InputFormat, Box<dyn Error>> {
    let has_new = ["Spot #", "Position 1", "Position 2", "Pot Type", "Hand", "Board"]
        .iter()
        .all(|header| header_index(headers, header).is_some());
    if has_new {
        return Ok(InputFormat::NewSpreadsheet);
    }

    let has_old = ["spot_type", "hero_position", "villain_position", "board", "hand"]
        .iter()
        .all(|header| header_index(headers, header).is_some());
    if has_old {
        return Ok(InputFormat::OldStatProbe);
    }

    Err(format!("unsupported {INPUT_FILE} headers: {headers:?}").into())
}

fn normalize_record(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    format: InputFormat,
    row_number: usize,
) -> Option<ProbeSpot> {
    match format {
        InputFormat::NewSpreadsheet => normalize_new_record(headers, record, row_number),
        InputFormat::OldStatProbe => normalize_old_record(headers, record, row_number),
    }
}

fn normalize_new_record(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    row_number: usize,
) -> Option<ProbeSpot> {
    let spot_id = field(headers, record, "Spot #").trim().to_string();
    let spot_id = if spot_id.is_empty() { row_number.to_string() } else { spot_id };
    let hero_position = field(headers, record, "Position 1").trim().to_uppercase();
    let villain_raw = field(headers, record, "Position 2").trim().to_uppercase();
    let pot_type = field(headers, record, "Pot Type").trim().to_uppercase();
    let hand = field(headers, record, "Hand").trim().to_string();
    let board = field(headers, record, "Board").trim().to_string();

    let spot_type = match pot_type.as_str() {
        "SRP" => "srp",
        "3BP" => "3bp",
        "4BP" => "4bp",
        other => {
            eprintln!("Skipping spot {spot_id}: unsupported pot type {other}");
            return None;
        }
    }
    .to_string();

    normalize_probe_spot(spot_id, spot_type, hero_position, Some(villain_raw), board, Some(hand))
}

fn normalize_old_record(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    row_number: usize,
) -> Option<ProbeSpot> {
    let spot_type = normalize_legacy_spot_type(field(headers, record, "spot_type").trim());
    let hero_position = field(headers, record, "hero_position").trim().to_uppercase();
    let villain_raw = field(headers, record, "villain_position").trim().to_uppercase();
    let board = field(headers, record, "board").trim().to_string();
    let hand = field(headers, record, "hand").trim().to_string();
    let spot_id = header_index(headers, "spot_id")
        .and_then(|index| record.get(index))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| row_number.to_string());

    normalize_probe_spot(spot_id, spot_type, hero_position, Some(villain_raw), board, Some(hand))
}

fn normalize_probe_spot(
    spot_id: String,
    spot_type: String,
    hero_position: String,
    villain_position: Option<String>,
    board: String,
    hand: Option<String>,
) -> Option<ProbeSpot> {
    if !matches!(spot_type.as_str(), "srp" | "3bp" | "4bp") {
        eprintln!("Skipping spot {spot_id}: unsupported spot_type={spot_type}");
        return None;
    }

    if !is_position(&hero_position) {
        eprintln!("Skipping spot {spot_id}: invalid hero_position={hero_position}");
        return None;
    }

    let villain_position = villain_position
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    if let Some(villain) = &villain_position {
        if !is_position(villain) {
            eprintln!("Skipping spot {spot_id}: invalid villain_position={villain}");
            return None;
        }
    }

    let hand = hand.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());

    Some(ProbeSpot {
        spot_id,
        spot_type,
        hero_position,
        villain_position,
        board,
        hand,
    })
}

fn normalize_legacy_spot_type(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "rfi" | "vs_open" | "srp" => "srp".to_string(),
        "vs_3bet" | "3bp" => "3bp".to_string(),
        "4bp" => "4bp".to_string(),
        other => other.to_string(),
    }
}

fn header_index(headers: &csv::StringRecord, name: &str) -> Option<usize> {
    headers.iter().position(|header| header.trim().eq_ignore_ascii_case(name))
}

fn field<'a>(headers: &csv::StringRecord, record: &'a csv::StringRecord, name: &str) -> &'a str {
    header_index(headers, name).and_then(|index| record.get(index)).unwrap_or("")
}

fn fallback_sample() -> Vec<ProbeSpot> {
    const NUM_BOARDS: usize = 100;
    let mut rng = Lcg::new(0xC0D3_2026);
    let mut boards = HashSet::new();
    let mut rows = Vec::new();

    while rows.len() < NUM_BOARDS {
        let board = random_board(&mut rng);
        if boards.insert(board.clone()) {
            rows.push(ProbeSpot {
                spot_id: format!("fallback-{}", rows.len() + 1),
                spot_type: "srp".to_string(),
                hero_position: "BTN".to_string(),
                villain_position: Some("BB".to_string()),
                board,
                hand: None,
            });
        }
    }

    rows
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_usize(&mut self, max: usize) -> usize {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.state >> 32) as usize) % max
    }
}

fn random_board(rng: &mut Lcg) -> String {
    let mut cards = Vec::new();
    while cards.len() < 3 {
        let card = rng.next_usize(52) as Card;
        if !cards.contains(&card) {
            cards.push(card);
        }
    }
    cards.into_iter().map(card_to_string).collect::<Vec<_>>().join("")
}

struct RangePair {
    hero_range: &'static str,
    villain_range: &'static str,
}

fn lookup_ranges(input: &ProbeSpot) -> Option<RangePair> {
    let villain = input.villain_position.as_deref();
    match (input.spot_type.as_str(), input.hero_position.as_str(), villain) {
        ("srp", "BTN", None) | ("srp", "BTN", Some("BB")) => Some(RangePair {
            hero_range: BTN_OPEN_RANGE,
            villain_range: BB_VS_BTN_RANGE,
        }),
        ("srp", "BB", Some("BTN")) => Some(RangePair {
            hero_range: BB_VS_BTN_RANGE,
            villain_range: BTN_OPEN_RANGE,
        }),
        ("srp", hero, villain) => {
            let villain = villain.unwrap_or("");
            warn_range_fallback("SRP", hero, villain);
            Some(fallback_ranges_for_positions(hero, villain))
        }
        ("3bp", hero, villain) => {
            let villain = villain.unwrap_or("");
            warn_range_fallback("3BP", hero, villain);
            Some(fallback_ranges_for_positions(hero, villain))
        }
        ("4bp", _, _) => None,
        _ => None,
    }
}

fn warn_range_fallback(pot_type: &str, hero: &str, villain: &str) {
    eprintln!(
        "WARNING: using closest available {pot_type} fallback range for {hero} vs {villain}"
    );
}

fn fallback_ranges_for_positions(hero: &str, villain: &str) -> RangePair {
    if hero == "BB" || postflop_position_order(hero) < postflop_position_order(villain) {
        RangePair {
            hero_range: BB_VS_BTN_RANGE,
            villain_range: BTN_OPEN_RANGE,
        }
    } else {
        RangePair {
            hero_range: BTN_OPEN_RANGE,
            villain_range: BB_VS_BTN_RANGE,
        }
    }
}

fn postflop_pot_config(input: &ProbeSpot) -> Option<PostflopPotConfig> {
    match input.spot_type.as_str() {
        "srp" => Some(PostflopPotConfig {
            starting_pot_bb: 5.5,
            effective_stack_bb: STARTING_STACK_BB,
        }),
        "3bp" => Some(PostflopPotConfig {
            starting_pot_bb: 18.0,
            effective_stack_bb: STARTING_STACK_BB,
        }),
        "4bp" => None,
        _ => None,
    }
}

fn solve_spot(
    board: [Card; 3],
    hero_range: &str,
    villain_range: &str,
    hero_is_ip: bool,
    hero_player: usize,
    sizing: PostflopSizing,
    starting_pot: i32,
    effective_stack: i32,
) -> Result<DecisionSolve, Box<dyn Error>> {
    let (oop_range, ip_range): (Range, Range) = if hero_is_ip {
        (villain_range.parse()?, hero_range.parse()?)
    } else {
        (hero_range.parse()?, villain_range.parse()?)
    };

    let mut game = build_game(
        board,
        oop_range,
        ip_range,
        hero_is_ip,
        sizing,
        starting_pot,
        effective_stack,
    )?;
    game.allocate_memory(false);
    solve(&mut game, SOLVE_ITERATIONS, 0.5, true);
    move_to_hero_decision(&mut game, hero_player)?;

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let bet_index = find_action(&actions, |action| matches!(action, Action::Bet(_)))?;
    let hand_count = game.private_cards(hero_player).len();
    let villain_player = 1 - hero_player;
    let villain_count = game.private_cards(villain_player).len();
    let hands = player_hands(&game, hero_player)?;
    let villains = player_hands(&game, villain_player)?;
    let strategy = game.strategy().to_vec();
    let evs = game.expected_values_detail(hero_player).to_vec();
    let hero_equities = game.equity(hero_player).to_vec();
    let villain_equities = game.equity(villain_player).to_vec();
    let hero_weights = game.normalized_weights(hero_player).to_vec();
    let range_stats = calculate_range_stats(
        &hero_equities,
        &villain_equities,
        &hero_weights,
        hand_count,
        villain_count,
    );

    Ok(DecisionSolve {
        hands,
        villains,
        strategy,
        evs,
        hero_equities,
        check_index,
        bet_index,
        hand_count,
        range_stats,
    })
}

fn build_game(
    board: [Card; 3],
    oop_range: Range,
    ip_range: Range,
    hero_is_ip: bool,
    sizing: PostflopSizing,
    starting_pot: i32,
    effective_stack: i32,
) -> Result<PostFlopGame, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [oop_range, ip_range],
        flop: board,
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };
    let hero_bet_options = BetSizeOptions::try_from((sizing.tree_size, ""))?;
    let flop_bet_sizes = if hero_is_ip {
        [Default::default(), hero_bet_options]
    } else {
        [hero_bet_options, Default::default()]
    };
    let tree_config = TreeConfig {
        initial_state: BoardState::Flop,
        starting_pot,
        effective_stack,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes,
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

fn move_to_hero_decision(game: &mut PostFlopGame, hero_player: usize) -> Result<(), Box<dyn Error>> {
    game.back_to_root();
    if game.current_player() == hero_player {
        return Ok(());
    }

    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    game.play(check_index);

    if game.current_player() != hero_player {
        return Err(format!("expected hero player {hero_player} to act, got {}", game.current_player()).into());
    }
    Ok(())
}

fn row_values(solve: &DecisionSolve, hand_index: usize) -> RowValues {
    let check_freq = action_value(&solve.strategy, solve.check_index, hand_index, solve.hand_count);
    let check_ev = ev_to_bb(action_value(&solve.evs, solve.check_index, hand_index, solve.hand_count));
    let bet_freq = action_value(&solve.strategy, solve.bet_index, hand_index, solve.hand_count);
    let bet_ev = ev_to_bb(action_value(&solve.evs, solve.bet_index, hand_index, solve.hand_count));
    let (best_action, ev) = if bet_ev > check_ev {
        ("bet", bet_ev)
    } else {
        ("check", check_ev)
    };

    RowValues {
        check_freq,
        bet_freq,
        check_ev,
        bet_ev,
        best_action,
        ev,
    }
}

fn player_hands(game: &PostFlopGame, player: usize) -> Result<Vec<HandCombo>, Box<dyn Error>> {
    let cards = game.private_cards(player);
    let labels = holes_to_strings(cards);
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
    let hero_weighted_value_combos: f32 = hero_equities.iter().take(hero_count).map(|&eq| value_weight(eq)).sum();
    let villain_weighted_value_combos: f32 = villain_equities.iter().take(villain_count).map(|&eq| value_weight(eq)).sum();
    let hero_total_live_combos = hero_count as f32;
    let villain_total_live_combos = villain_count as f32;
    let hero_weighted_value_pct = safe_div(hero_weighted_value_combos, hero_total_live_combos);
    let villain_weighted_value_pct = safe_div(villain_weighted_value_combos, villain_total_live_combos);
    let (weighted_equity_sum, weight_sum) = hero_equities
        .iter()
        .take(hero_count)
        .zip(hero_weights.iter().take(hero_count))
        .fold((0.0, 0.0), |(eq_sum, w_sum), (&equity, &weight)| {
            (eq_sum + equity * weight, w_sum + weight)
        });
    let range_equity_hero = safe_div(weighted_equity_sum, weight_sum);

    RangeStats {
        hero_weighted_value_combos,
        villain_weighted_value_combos,
        hero_total_live_combos,
        villain_total_live_combos,
        hero_weighted_value_pct,
        villain_weighted_value_pct,
        nut_advantage_pct: hero_weighted_value_pct - villain_weighted_value_pct,
        range_equity_hero,
    }
}

fn raw_equity_vs_villain(hero: (Card, Card), villains: &[HandCombo], board: &[Card; 3]) -> f32 {
    let mut points = 0.0;
    let mut total = 0.0;
    let hero_rank = evaluate_made_hand(hero, board);
    for villain in villains {
        if hero_blocks_combo(villain.cards.0, villain.cards.1, hero) {
            continue;
        }
        let villain_rank = evaluate_made_hand(villain.cards, board);
        points += match hero_rank.cmp(&villain_rank) {
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Equal => 0.5,
            std::cmp::Ordering::Less => 0.0,
        };
        total += 1.0;
    }
    safe_div(points, total)
}

fn raw_strength_score(equity: f32) -> u8 {
    score_from_thresholds(equity * 100.0, &[
        (92.0, 10),
        (83.0, 9),
        (73.0, 8),
        (62.0, 7),
        (50.0, 6),
        (38.0, 5),
        (27.0, 4),
        (17.0, 3),
        (8.0, 2),
    ])
}

fn improvability_score(delta: f32) -> u8 {
    score_from_thresholds(delta * 100.0, &[
        (32.0, 10),
        (26.0, 9),
        (20.0, 8),
        (15.0, 7),
        (10.0, 6),
        (5.0, 5),
        (0.0, 4),
        (-4.0, 3),
        (-10.0, 2),
    ])
}

fn range_advantage_score(range_equity: f32) -> u8 {
    score_from_thresholds(range_equity * 100.0, &[
        (68.0, 10),
        (63.0, 9),
        (58.0, 8),
        (54.0, 7),
        (50.0, 6),
        (46.0, 5),
        (42.0, 4),
        (37.0, 3),
        (32.0, 2),
    ])
}

fn nut_advantage_score(nut_advantage: f32) -> u8 {
    score_from_thresholds(nut_advantage * 100.0, &[
        (8.0, 10),
        (5.0, 9),
        (3.0, 8),
        (1.5, 7),
        (0.0, 6),
        (-1.5, 5),
        (-3.0, 4),
        (-5.0, 3),
        (-8.0, 2),
    ])
}

fn score_from_thresholds(value: f32, thresholds: &[(f32, u8)]) -> u8 {
    for &(min, score) in thresholds {
        if value >= min {
            return score;
        }
    }
    1
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

fn safe_div(numerator: f32, denominator: f32) -> f32 {
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn write_header(writer: &mut csv::Writer<File>) -> Result<(), Box<dyn Error>> {
    writer.write_record([
        "spot_type",
        "hero_position",
        "villain_position",
        "board",
        "hand",
        "bet_size",
        "bet_size_bb",
        "pot_before_bb",
        "check_freq",
        "bet_freq",
        "check_ev",
        "bet_ev",
        "best_action",
        "ev",
        "hero_equity_vs_villain",
        "equity_with_draws",
        "raw_strength_score",
        "improvability_score",
        "range_equity_hero",
        "range_advantage_score",
        "hero_weighted_value_combos",
        "villain_weighted_value_combos",
        "hero_total_live_combos",
        "villain_total_live_combos",
        "hero_weighted_value_pct",
        "villain_weighted_value_pct",
        "nut_advantage_pct",
        "nut_advantage_score",
        "iteration_count",
        "spot_id",
    ])?;
    Ok(())
}

fn is_position(position: &str) -> bool {
    matches!(position, "UTG" | "HJ" | "CO" | "BTN" | "SB" | "BB")
}

fn is_in_position(hero: &str, villain: Option<&str>) -> bool {
    let Some(villain) = villain else {
        return hero != "SB";
    };
    postflop_position_order(hero) > postflop_position_order(villain)
}

fn postflop_position_order(position: &str) -> i32 {
    match position {
        "SB" => 0,
        "BB" => 1,
        "UTG" => 2,
        "HJ" => 3,
        "CO" => 4,
        "BTN" => 5,
        _ => -1,
    }
}

fn bb_to_chips(bb: f64) -> i32 {
    (bb * BB_CHIPS as f64).round() as i32
}

fn normalize_hand_label(hand: &str) -> String {
    let hand = hand.trim();
    if hand.len() != 4 {
        return hand.to_ascii_lowercase();
    }
    let first = parse_card(&hand[0..2]);
    let second = parse_card(&hand[2..4]);
    match (first, second) {
        (Ok(a), Ok(b)) => {
            let mut cards = [a, b];
            cards.sort_by(|left, right| right.cmp(left));
            format!("{}{}", card_to_string(cards[0]), card_to_string(cards[1])).to_ascii_lowercase()
        }
        _ => hand.to_ascii_lowercase(),
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

fn format_f64(value: f64) -> String {
    format!("{value:.6}")
}

fn evaluate_made_hand(hand: (Card, Card), board: &[Card; 3]) -> u32 {
    evaluate_best_from_cards(&[hand.0, hand.1, board[0], board[1], board[2]])
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
                    [(2, pair), (1, k1), (1, k2), (1, k3)] => {
                        encode_rank(1, &[*pair, *k1, *k2, *k3])
                    }
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
    let board = board
        .trim()
        .chars()
        .filter(|card| !card.is_whitespace() && *card != ',' && *card != '-')
        .collect::<String>();
    if board.len() < 6 || board.len() % 2 != 0 {
        return Err(format!("board must contain at least 3 cards: {board}").into());
    }
    let mut all_cards = Vec::new();
    for index in (0..board.len()).step_by(2) {
        all_cards.push(parse_card(&board[index..index + 2])?);
    }
    if all_cards.len() < 3 {
        return Err(format!("board must contain at least 3 cards: {board}").into());
    }
    let cards = [all_cards[0], all_cards[1], all_cards[2]];
    if cards[0] == cards[1] || cards[0] == cards[2] || cards[1] == cards[2] {
        return Err(format!("first 3 board cards contain duplicates: {board}").into());
    }
    Ok(cards)
}

fn board_to_string(board: &[Card; 3]) -> String {
    board.iter().map(|&card| card_to_string(card)).collect::<Vec<_>>().join("")
}

fn parse_hand(hand: &str) -> Result<(Card, Card), Box<dyn Error>> {
    let hand = hand.trim();
    if hand.len() != 4 {
        return Err(format!("hand must contain exactly 2 cards: {hand}").into());
    }
    let cards = (parse_card(&hand[0..2])?, parse_card(&hand[2..4])?);
    if cards.0 == cards.1 {
        return Err(format!("hand contains duplicate cards: {hand}").into());
    }
    Ok(cards)
}

fn parse_card(card: &str) -> Result<Card, Box<dyn Error>> {
    let bytes = card.as_bytes();
    if bytes.len() != 2 {
        return Err(format!("invalid card: {card}").into());
    }
    let rank = match (bytes[0] as char).to_ascii_uppercase() {
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
    let suit = match (bytes[1] as char).to_ascii_lowercase() {
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

fn holes_to_strings(cards: &[(Card, Card)]) -> Vec<String> {
    cards
        .iter()
        .map(|&(a, b)| format!("{}{}", card_to_string(a), card_to_string(b)))
        .collect()
}

#[allow(dead_code)]
static SPOT_SIZE_CONFIGS: &[SpotSizeConfig] = &[
    SpotSizeConfig { spot_type: "rfi", hero_position: "UTG", villain_position: None, bet_size_bb: 2.5, pot_before_bb: 1.5 },
    SpotSizeConfig { spot_type: "rfi", hero_position: "HJ", villain_position: None, bet_size_bb: 2.5, pot_before_bb: 1.5 },
    SpotSizeConfig { spot_type: "rfi", hero_position: "CO", villain_position: None, bet_size_bb: 2.5, pot_before_bb: 1.5 },
    SpotSizeConfig { spot_type: "rfi", hero_position: "BTN", villain_position: None, bet_size_bb: 2.5, pot_before_bb: 1.5 },
    SpotSizeConfig { spot_type: "rfi", hero_position: "SB", villain_position: None, bet_size_bb: 3.0, pot_before_bb: 1.5 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "HJ", villain_position: Some("UTG"), bet_size_bb: 8.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "CO", villain_position: Some("UTG"), bet_size_bb: 8.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "CO", villain_position: Some("HJ"), bet_size_bb: 8.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BTN", villain_position: Some("UTG"), bet_size_bb: 8.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BTN", villain_position: Some("HJ"), bet_size_bb: 8.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BTN", villain_position: Some("CO"), bet_size_bb: 8.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "SB", villain_position: Some("UTG"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "SB", villain_position: Some("HJ"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "SB", villain_position: Some("CO"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "SB", villain_position: Some("BTN"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BB", villain_position: Some("UTG"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BB", villain_position: Some("HJ"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BB", villain_position: Some("CO"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BB", villain_position: Some("BTN"), bet_size_bb: 10.0, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_open", hero_position: "BB", villain_position: Some("SB"), bet_size_bb: 9.5, pot_before_bb: 4.0 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "UTG", villain_position: Some("HJ"), bet_size_bb: 20.0, pot_before_bb: 12.0 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "UTG", villain_position: Some("CO"), bet_size_bb: 20.0, pot_before_bb: 12.0 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "UTG", villain_position: Some("BTN"), bet_size_bb: 20.0, pot_before_bb: 12.0 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "UTG", villain_position: Some("SB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "UTG", villain_position: Some("BB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "HJ", villain_position: Some("CO"), bet_size_bb: 20.0, pot_before_bb: 12.0 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "HJ", villain_position: Some("BTN"), bet_size_bb: 20.0, pot_before_bb: 12.0 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "HJ", villain_position: Some("SB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "HJ", villain_position: Some("BB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "CO", villain_position: Some("BTN"), bet_size_bb: 20.0, pot_before_bb: 12.0 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "CO", villain_position: Some("SB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "CO", villain_position: Some("BB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "BTN", villain_position: Some("SB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
    SpotSizeConfig { spot_type: "vs_3bet", hero_position: "BTN", villain_position: Some("BB"), bet_size_bb: 22.5, pot_before_bb: 13.5 },
];
