use postflop_solver::*;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

const SOLVE_ITERATIONS: u32 = 500;
const SAFE_MINIMAL_OUTPUT: bool = true;
const MAX_INPUT_ROWS_TO_PROCESS: Option<usize> = Some(10);
const BB_CHIPS: f32 = 100.0;
const STARTING_STACK_BB: f64 = 100.0;
const INPUT_FILE: &str = "stat_probe_input.csv";
const OUTPUT_FILE: &str = "stat_probe_output.csv";
const IP_PLAYER: usize = 1;
const OOP_PLAYER: usize = 0;

const SRP_IP_FLOP_SIZINGS: &[PostflopSizing] = &[
    PostflopSizing { label: "33%", suffix: "33", pct: 0.33, tree_size: "33%" },
    PostflopSizing { label: "75%", suffix: "75", pct: 0.75, tree_size: "75%" },
];
const THREE_BP_IP_FLOP_SIZINGS: &[PostflopSizing] = &[
    PostflopSizing { label: "20%", suffix: "20", pct: 0.20, tree_size: "20%" },
    PostflopSizing { label: "60%", suffix: "60", pct: 0.60, tree_size: "60%" },
    PostflopSizing { label: "125%", suffix: "125", pct: 1.25, tree_size: "125%" },
];
const FOUR_BP_IP_FLOP_SIZINGS: &[PostflopSizing] = &[
    PostflopSizing { label: "15%", suffix: "15", pct: 0.15, tree_size: "15%" },
    PostflopSizing { label: "50%", suffix: "50", pct: 0.50, tree_size: "50%" },
];
const SRP_OOP_FLOP_SIZINGS: &[PostflopSizing] = SRP_IP_FLOP_SIZINGS;
const THREE_BP_OOP_FLOP_SIZINGS: &[PostflopSizing] = THREE_BP_IP_FLOP_SIZINGS;
const FOUR_BP_OOP_FLOP_SIZINGS: &[PostflopSizing] = FOUR_BP_IP_FLOP_SIZINGS;
const IP_SRP_TURN_SIZINGS: &[&str] = &["50%", "100%"];
const IP_SRP_RIVER_SIZINGS: &[&str] = &["50%", "100%", "150%"];
const IP_3BP_TURN_SIZINGS: &[&str] = &["50%", "100%"];
const IP_3BP_RIVER_SIZINGS: &[&str] = &["50%", "100%"];
const IP_4BP_TURN_SIZINGS: &[&str] = &["25%", "67%"];
const IP_4BP_RIVER_SIZINGS: &[&str] = &["33%", "75%"];
const OOP_SRP_TURN_SIZINGS: &[&str] = &["33%", "75%"];
const OOP_SRP_RIVER_SIZINGS: &[&str] = &["33%", "100%"];
const OOP_3BP_TURN_SIZINGS: &[&str] = &["50%", "125%"];
const OOP_3BP_RIVER_SIZINGS: &[&str] = &["50%", "100%"];
const OOP_4BP_TURN_SIZINGS: &[&str] = &["25%", "67%"];
const OOP_4BP_RIVER_SIZINGS: &[&str] = &["33%", "75%"];

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct PostflopSizing {
    label: &'static str,
    suffix: &'static str,
    pct: f64,
    tree_size: &'static str,
}

#[allow(dead_code)]
struct SizingProfile {
    ip_flop: &'static [PostflopSizing],
    ip_turn: &'static [&'static str],
    ip_river: &'static [&'static str],
    oop_flop: &'static [PostflopSizing],
    oop_turn: &'static [&'static str],
    oop_river: &'static [&'static str],
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
    evs_available: bool,
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
    println!("Input interpretation: Position 1 = IP / hero, Position 2 = OOP / villain.");
    let rows_to_process = match MAX_INPUT_ROWS_TO_PROCESS {
        Some(limit) => {
            println!("MAX_INPUT_ROWS_TO_PROCESS = {limit}");
            println!("Processing first {limit} input rows only for testing.");
            input_rows.len().min(limit)
        }
        None => {
            println!("MAX_INPUT_ROWS_TO_PROCESS = None");
            println!("Processing all {} input rows.", input_rows.len());
            input_rows.len()
        }
    };
    let mut writer = csv::Writer::from_path(OUTPUT_FILE)?;
    write_header(&mut writer)?;

    let mut rows_written = 0usize;
    let mut seen = HashSet::new();
    let mut skipped = SkipSummary::default();

    for (row_index, input) in input_rows.iter().take(rows_to_process).enumerate() {
        println!("Running stat probe row {}/{}", row_index + 1, rows_to_process);

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
            "Running spot {}: {} IP={} OOP={} hand={} full_board={} flop={}",
            input.spot_id,
            input.spot_type,
            input.hero_position,
            input.villain_position.as_deref().unwrap_or(""),
            input.hand.as_deref().unwrap_or("ALL"),
            input.board,
            flop_board
        );

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
        println!(
            "Range keys: IP={} | OOP={}",
            range_pair.hero_range_key,
            range_pair.villain_range_key
        );

        let hero_is_ip = true;
        let hero_player = IP_PLAYER;
        let starting_pot_bb = pot_config.starting_pot_bb;
        let starting_pot = bb_to_chips(starting_pot_bb);
        let effective_stack = bb_to_chips(pot_config.effective_stack_bb);
        let sizings = ip_flop_sizings(input.spot_type.as_str());

        for &sizing in sizings {
            println!("Running IP flop sizing {}", sizing.label);
            let solve_result = catch_unwind(AssertUnwindSafe(|| {
                solve_spot(
                    board,
                    range_pair.hero_range.as_str(),
                    range_pair.villain_range.as_str(),
                    hero_is_ip,
                    hero_player,
                    sizing,
                    starting_pot,
                    effective_stack,
                )
            }));
            let solve = match solve_result {
                Ok(Ok(solve)) => solve,
                Ok(Err(error)) => {
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
                Err(_) => {
                    eprintln!(
                        "Skipping spot {} sizing {}: postflop-solver panicked; continuing without crashing stat_probe",
                        input.spot_id,
                        sizing.label
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
                            "Skipping spot {}: hand {} is not in IP range. IP position = {} OOP position = {} Pot type = {} IP range key = {} IP range string = {} board={}",
                            input.spot_id,
                            hand,
                            input.hero_position,
                            input.villain_position.as_deref().unwrap_or(""),
                            input.spot_type,
                            range_pair.hero_range_key,
                            range_pair.hero_range,
                            input.board
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
                let range_advantage_score = if SAFE_MINIMAL_OUTPUT {
                    0
                } else {
                    range_advantage_score(solve.range_stats.range_equity_hero)
                };
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
    hero_range: String,
    villain_range: String,
    hero_range_key: String,
    villain_range_key: String,
}

fn lookup_ranges(input: &ProbeSpot) -> Option<RangePair> {
    let hero = input.hero_position.as_str();
    let villain = input.villain_position.as_deref().unwrap_or("");
    match input.spot_type.as_str() {
        "srp" => lookup_srp_ranges(hero, villain),
        "3bp" => lookup_3bp_ranges(hero, villain),
        "4bp" => lookup_4bp_ranges(hero, villain),
        _ => None,
    }
}

fn lookup_srp_ranges(hero: &str, villain: &str) -> Option<RangePair> {
    let hero_range = rfi_range(hero).or_else(|| missing_range("RFI", hero, ""))?;
    let villain_range = vs_open_call_range(villain, hero)
        .or_else(|| missing_range("vs open call", villain, hero))?;
    Some(make_range_pair(
        hero_range,
        villain_range,
        format!("{hero} RFI"),
        format!("{villain} vs {hero} Open call"),
    ))
}

fn lookup_3bp_ranges(hero: &str, villain: &str) -> Option<RangePair> {
    if let (Some(hero_4bet), Some(hero_call), Some(villain_3bet)) = (
        threebet_defense_4bet_range(hero, villain),
        threebet_defense_call_range(hero, villain),
        vs_open_3bet_range(villain, hero),
    ) {
        let hero_range = combine_ranges(&[hero_4bet, hero_call]);
        return Some(make_range_pair(
            &hero_range,
            villain_3bet,
            format!("{hero} vs {villain} 3bet Defense 4bet+call"),
            format!("{villain} vs {hero} Open 3bet"),
        ));
    }

    if let (Some(hero_3bet), Some(villain_4bet), Some(villain_call)) = (
        vs_open_3bet_range(hero, villain),
        threebet_defense_4bet_range(villain, hero),
        threebet_defense_call_range(villain, hero),
    ) {
        let villain_range = combine_ranges(&[villain_4bet, villain_call]);
        return Some(make_range_pair(
            hero_3bet,
            &villain_range,
            format!("{hero} vs {villain} Open 3bet"),
            format!("{villain} vs {hero} 3bet Defense 4bet+call"),
        ));
    }

    eprintln!("Missing 3BP range pair for IP={hero} OOP={villain}");
    None
}

fn lookup_4bp_ranges(hero: &str, villain: &str) -> Option<RangePair> {
    if let (Some(hero_4bet), Some(villain_shove), Some(villain_call)) = (
        threebet_defense_4bet_range(hero, villain),
        fourbet_defense_shove_range(villain, hero),
        fourbet_defense_call_range(villain, hero),
    ) {
        let villain_range = combine_ranges(&[villain_shove, villain_call]);
        return Some(make_range_pair(
            hero_4bet,
            &villain_range,
            format!("{hero} vs {villain} 3bet Defense 4bet"),
            format!("{villain} vs {hero} 4bet Defense shove+call"),
        ));
    }

    if let (Some(hero_shove), Some(hero_call), Some(villain_4bet)) = (
        fourbet_defense_shove_range(hero, villain),
        fourbet_defense_call_range(hero, villain),
        threebet_defense_4bet_range(villain, hero),
    ) {
        let hero_range = combine_ranges(&[hero_shove, hero_call]);
        return Some(make_range_pair(
            &hero_range,
            villain_4bet,
            format!("{hero} vs {villain} 4bet Defense shove+call"),
            format!("{villain} vs {hero} 3bet Defense 4bet"),
        ));
    }

    eprintln!("Missing 4BP range pair for IP={hero} OOP={villain}; skipping exact spot");
    None
}

fn make_range_pair(
    hero_range: &str,
    villain_range: &str,
    hero_range_key: String,
    villain_range_key: String,
) -> RangePair {
    RangePair {
        hero_range: normalize_range_for_solver(hero_range),
        villain_range: normalize_range_for_solver(villain_range),
        hero_range_key,
        villain_range_key,
    }
}

fn missing_range(kind: &str, position: &str, versus: &str) -> Option<&'static str> {
    eprintln!("Missing range key: kind={kind} position={position} versus={versus}");
    None
}

fn combine_ranges(ranges: &[&str]) -> String {
    ranges.iter().copied().filter(|range| !range.trim().is_empty()).collect::<Vec<_>>().join(",")
}

fn rfi_range(position: &str) -> Option<&'static str> {
    match position {
        "UTG" => Some("66+,A2s+,K6s+,QTs+,ATo+,KJo+,JTs,T9s,65s"),
        "HJ" => Some("55+,A2s+,K5s+,Q9s+,A9o+,KTo+,QTo+,J9s+,T8s+,65s"),
        "CO" => Some("44+,A2s+,K3s+,Q6s+,A8o+,KTo+,QTo+,JTo,J8s+,T8s+,98s,78s,67s,65s,54s,A5o"),
        "BTN" => Some("22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s"),
        "SB" => Some("22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,85s+,75s+,64s+,53s+"),
        _ => None,
    }
}

fn vs_open_3bet_range(defender: &str, opener: &str) -> Option<&'static str> {
    match (defender, opener) {
        ("HJ", "UTG") => Some("99+,ATs+,KTs+,AQo+,65s,A4s-A5s"),
        ("CO", "UTG") => Some("88+,ATs+,KTs+,AQo+,65s,A4s-A5s"),
        ("CO", "HJ") => Some("88+,ATs+,KTs+,AQo+,KQo,QJs,65s,A3s-A5s"),
        ("BTN", "UTG") => Some("QQ+,AKs,AKo,KQo,QJs,JTs,T9s,K9s-KTs,A8s,A3s-A5s,65s"),
        ("BTN", "HJ") => Some("QQ+,AKs,AKo,KQo,QJs,JTs,T9s,K9s-KTs,A3s-A8s,65s,76s,AJo"),
        ("BTN", "CO") => Some("QQ+,AKs,AKo,KQo,QJs,J9s+,T9s,K8s-K9s,A2s-A3s,A5s-A7s,65s,76s,AJo-ATo,Q9s"),
        ("SB", "UTG") => Some("TT+,ATs+,KTs+,QJs,AKo,A5s,65s"),
        ("SB", "HJ") => Some("99+,ATs+,KTs+,QJs,AKo,JTs,A4s-A5s,65s"),
        ("SB", "CO") => Some("88+,ATs+,KTs+,QTs+,AQo+,JTs,A4s-A5s,65s"),
        ("SB", "BTN") => Some("66+,A7s+,K9s+,QTs+,AJo+,JTs,A4s-A5s,65s,KQo,T9s"),
        ("BB", "UTG") => Some("QQ+,QJs,K7s,A5s-A3s,65s,AKs,AKo"),
        ("BB", "HJ") => Some("QQ+,QTs+,KJs+,JTs,T9s,K7s,A5s-A3s,65s,AKs,AKo"),
        ("BB", "CO") => Some("JJ+,AQs,Q9s+,KTs+,J9s+,T9s,K7s,A5s-A3s,65s,AKs,AKo,67s"),
        ("BB", "BTN") => Some("TT+,KQs,QJs,J8s+,T7s+,A5s,65s,AKs,AQo+,67s,54s,98s,A5o"),
        ("BB", "SB") => Some("TT+,AQs,KQs,J8s+,T9s+,A5s,65s,AKs,AQo+,76s,54s,98s,A3o-A6o,K8o-K9o,J9o,T8o,J5s-J2s,T5s-T4s"),
        _ => None,
    }
}

fn vs_open_call_range(defender: &str, opener: &str) -> Option<&'static str> {
    match (defender, opener) {
        ("BTN", "UTG") => Some("77-JJ,A9s-AQs,QTs,AQo,KJs,KQs"),
        ("BTN", "HJ") => Some("66-JJ,A9s-AQs,QTs,AQo,KJs,KQs"),
        ("BTN", "CO") => Some("55-JJ,A8s-AQs,QTs,AQo,A4s,KTs+"),
        ("BB", "UTG") => Some("22-JJ,AQs-A6s,KQs-K8s,QTs-Q9s,JTs-J9s,T9s-T8s,98s-97s,87s-86s,76s-75s,64s,54s-53s,AJo-AQo,KQo,A2s"),
        ("BB", "HJ") => Some("22-JJ,AQs-A6s,KTs-K8s,Q9s,J9s-J8s,T8s-T7s,98s-97s,87s-86s,76s-75s,64s,54s-53s,ATo-AQo,KJo+,K5s-K6s,KQo,43s,QJo,A2s"),
        ("BB", "CO") => Some("22-TT,AJs-A6s,K9s-K8s,JTs-J8s,T8s-T7s,98s-96s,87s-85s,75s,64s,54s-53s,ATo-AQo,KTo+,K2s-K6s,QTo+,JTo,Q8s-Q6s,A2s,43s"),
        ("BB", "BTN") => Some("22-99,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,A6o-AJo,K9o+,QTo+,JTo,QTs-Q2s,A2s-A4s,T9o"),
        ("BB", "SB") => Some("22-99,AJs-A6s,KJs-K2s,J9s-J6s,T8s-T6s,97s-95s,87s-85s,75s-74s,64s-63s,53s-52s,43s,A6o-AJo,K9o+,K2s-K6s,Q9o+,JTo,QJs-Q2s,A2s-A4s,T9o,98o"),
        _ => None,
    }
}

fn threebet_defense_4bet_range(opener: &str, threebettor: &str) -> Option<&'static str> {
    match (opener, threebettor) {
        ("UTG", "HJ") => Some("KK+,AKs,AKo,AJs,KQs,A5s"),
        ("UTG", "CO") => Some("KK+,AKs,AKo,AJs,KQs,A5s"),
        ("UTG", "BTN") => Some("KK+,AKs,AKo,AJs,A5s"),
        ("UTG", "SB") => Some("KK+,AKo,A4s"),
        ("UTG", "BB") => Some("KK+,AKo,A4s"),
        ("HJ", "CO") => Some("QQ+,KJs+,AKs,AKo,AJs,A5s"),
        ("HJ", "BTN") => Some("QQ+,KJs-KTs,AKs,AKo,A5s"),
        ("HJ", "SB") => Some("KK+,KJs-KTs,A4s"),
        ("HJ", "BB") => Some("KK+,KTs,A3s"),
        ("CO", "BTN") => Some("QQ+,AKs,AQo+,KTs-KJs,ATs,JTs,A4s"),
        ("CO", "SB") => Some("KK+,AKo,K9s,A4s"),
        ("CO", "BB") => Some("KK+,AKo,K9s,A7s,A2s"),
        ("BTN", "SB") => Some("QQ+,AJs,A7s,A3s,K9s,AKo,AJo"),
        ("BTN", "BB") => Some("QQ+,AJs,A2s,K7s-K6s,AKo,AJo"),
        ("SB", "BB") => Some("AA-JJ,ATo+,AKs,A6s,A3s-A2s,K5s"),
        _ => None,
    }
}

fn threebet_defense_call_range(opener: &str, threebettor: &str) -> Option<&'static str> {
    match (opener, threebettor) {
        ("UTG", "HJ") => Some("QQ-JJ,99-66,AQs,JTs,65s"),
        ("UTG", "CO") => Some("QQ-JJ,99-66,AQs,JTs,65s,KJs,ATs"),
        ("UTG", "BTN") => Some("QQ-55,AQs,JTs,65s,KJs+,ATs,QJs"),
        ("UTG", "SB") => Some("QQ-66,AQs,JTs,65s,KJs+,ATs,QJs,AKs,AJs,A5s"),
        ("UTG", "BB") => Some("QQ-66,AQs,JTs,65s,KJs+,ATs,QJs,AKs,AJs,A5s,A9s"),
        ("HJ", "CO") => Some("AQs,ATs,JTs,JJ-88,55,65s"),
        ("HJ", "BTN") => Some("AQs-A9s,JTs,JJ-55,65s,KQs,T9s,QJs"),
        ("HJ", "SB") => Some("AQs-ATs,JTs,QQ-77,55,65s,KQs,T9s,AKs,AKo,A5s,QJs"),
        ("HJ", "BB") => Some("AQs-A9s,JTs,QQ-55,65s,T9s,AKs,AKo,A5s-A4s,QTs+,KTs+"),
        ("CO", "BTN") => Some("JJ-44,AJs-AQs,A9s,A5s,KQs,T9s,65s,QTs+"),
        ("CO", "SB") => Some("QQ-77,55,ATs+,A5s,KTs+,T9s,65s,QTs+,J9s+,AQo"),
        ("CO", "BB") => Some("QQ-55,A8s+,A5s-A3s,KTs+,T9s,65s,QTs+,JTs,AQo,67s"),
        ("BTN", "SB") => Some("JJ-44,A8s+,A4s-A5s,KTs+,QTs+,J9s+,T8s+,98s,87s,67s,65s,54s,KQo,AQo"),
        ("BTN", "BB") => Some("JJ-44,A3s+,K8s+,Q9s+,J8s+,T8s+,98s,87s,67s,65s,54s,KQo,AQo"),
        ("SB", "BB") => Some("AQs-A7s,A5s-A4s,K8s+,Q9s+,J8s+,T8s+,76s,65s,54s,TT-44"),
        _ => None,
    }
}

#[allow(dead_code)]
fn fourbet_defense_shove_range(defender: &str, fourbettor: &str) -> Option<&'static str> {
    match (defender, fourbettor) {
        ("HJ", "UTG") => Some("KK,AKo"), ("CO", "UTG") => Some("KK,AKo"), ("CO", "HJ") => Some("KK,AKo"),
        ("BTN", "UTG") => Some("KK,AKo"), ("BTN", "HJ") => Some("KK,AKo"), ("BTN", "CO") => Some("KK,AKo"),
        ("SB", "UTG") => Some("KK+,AKo"), ("SB", "HJ") => Some("KK+,AKo"), ("SB", "CO") => Some("KK+,AKo,A5s"),
        ("SB", "BTN") => Some("QQ+,AKo,AKs,A5s"), ("BB", "UTG") => Some("AA,A5s"), ("BB", "HJ") => Some("KK+,AKo"),
        ("BB", "CO") => Some("KK+,AKo,JJ"), ("BB", "BTN") => Some("KK-JJ,AKo,A5s"), ("BB", "SB") => Some("KK-JJ,AKo,A5s,KQs"),
        _ => None,
    }
}

#[allow(dead_code)]
fn fourbet_defense_call_range(defender: &str, fourbettor: &str) -> Option<&'static str> {
    match (defender, fourbettor) {
        ("HJ", "UTG") => Some("AA,ATs+,QQ-JJ,99,65s"), ("CO", "UTG") => Some("AA,ATs+,QQ-JJ,99,65s,KQs"),
        ("CO", "HJ") => Some("AA,ATs+,QQ-JJ,99,65s,KQs,A5s"), ("BTN", "UTG") => Some("AA,AKs,QQ,T9s,65s,A5s"),
        ("BTN", "HJ") => Some("AA,AKs,QQ,T9s,65s,A5s,76s"), ("BTN", "CO") => Some("AA,AKs,QQ,T9s,65s,A5s,76s,J9s"),
        ("SB", "UTG") => Some("AKs,AJs,QQ,65s"), ("SB", "HJ") => Some("AJs+,QQ-JJ,99,65s"),
        ("SB", "CO") => Some("AJs+,QQ-JJ,99-88,65s,JTs"), ("SB", "BTN") => Some("ATs+,JJ-88,65s,JTs,66,T9s,QTs,KQs,AQo"),
        ("BB", "UTG") => Some("AKs,KK-QQ,65s"), ("BB", "HJ") => Some("AKs,QQ,65s,T9s"),
        ("BB", "CO") => Some("AKs,QQ,65s,T9s,JTs,AQs"), ("BB", "BTN") => Some("AKs,65s,T9s,JTs,76s,TT,AQo,KQs,AQs,AA"),
        ("BB", "SB") => Some("AA,AKs-AQs,65s,T9s,JTs,76s,TT,AQo,KQs,98s,54s"),
        _ => None,
    }
}

#[allow(dead_code)]
fn fivebet_shove_defense_call_range(defender: &str, shover: &str) -> Option<&'static str> {
    match (defender, shover) {
        ("UTG", "HJ") | ("UTG", "CO") | ("UTG", "BTN") => Some("KK+,AKs"),
        ("UTG", "SB") | ("UTG", "BB") => Some("KK+"),
        ("HJ", "CO") | ("HJ", "BTN") => Some("KK+,AKs,AKo"),
        ("HJ", "SB") | ("HJ", "BB") => Some("KK+"),
        ("CO", "BTN") => Some("QQ+,AKs,AKo"),
        ("CO", "SB") | ("CO", "BB") => Some("KK+,AKo"),
        ("BTN", "SB") | ("BTN", "BB") => Some("QQ+,AKo"),
        ("SB", "BB") => Some("JJ+,AKo,AKs"),
        _ => None,
    }
}

fn normalize_range_for_solver(range: &str) -> String {
    range
        .split(',')
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(strip_range_label)
        .flat_map(|token| token.split(',').map(str::to_string).collect::<Vec<_>>())
        .map(|token| normalize_range_token(&token))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn strip_range_label(token: &str) -> &str {
    token.rsplit_once(':').map(|(_, range)| range.trim()).unwrap_or(token)
}

fn normalize_range_token(token: &str) -> String {
    let token = token.trim();
    let Some((left, right)) = token.split_once('-') else {
        return normalize_combo_class(token);
    };
    let left = normalize_combo_class(left);
    let right = normalize_combo_class(right);
    if combo_class_order(&left) < combo_class_order(&right) {
        format!("{}-{}", right.trim(), left.trim())
    } else {
        format!("{}-{}", left.trim(), right.trim())
    }
}

fn normalize_combo_class(class: &str) -> String {
    let class = class.trim();
    let chars = class.chars().collect::<Vec<_>>();
    if chars.len() < 3 {
        return class.to_string();
    }

    let first = chars[0];
    let second = chars[1];
    let suffix = chars[2..].iter().collect::<String>();
    if first == second || rank_order(first) >= rank_order(second) {
        class.to_string()
    } else {
        format!("{second}{first}{suffix}")
    }
}

fn combo_class_order(class: &str) -> i32 {
    let class = class.trim().trim_end_matches('+');
    let chars = class.chars().collect::<Vec<_>>();
    if chars.len() < 2 {
        return 0;
    }
    rank_order(chars[0]) * 20 + rank_order(chars[1])
}

fn rank_order(rank: char) -> i32 {
    match rank.to_ascii_uppercase() {
        '2' => 2, '3' => 3, '4' => 4, '5' => 5, '6' => 6, '7' => 7, '8' => 8,
        '9' => 9, 'T' => 10, 'J' => 11, 'Q' => 12, 'K' => 13, 'A' => 14,
        _ => 0,
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
        "4bp" => Some(PostflopPotConfig {
            starting_pot_bb: 45.0,
            effective_stack_bb: STARTING_STACK_BB,
        }),
        _ => None,
    }
}

fn sizing_profile(pot_type: &str) -> SizingProfile {
    match pot_type {
        "3bp" => SizingProfile {
            ip_flop: THREE_BP_IP_FLOP_SIZINGS,
            ip_turn: IP_3BP_TURN_SIZINGS,
            ip_river: IP_3BP_RIVER_SIZINGS,
            oop_flop: THREE_BP_OOP_FLOP_SIZINGS,
            oop_turn: OOP_3BP_TURN_SIZINGS,
            oop_river: OOP_3BP_RIVER_SIZINGS,
        },
        "4bp" => SizingProfile {
            ip_flop: FOUR_BP_IP_FLOP_SIZINGS,
            ip_turn: IP_4BP_TURN_SIZINGS,
            ip_river: IP_4BP_RIVER_SIZINGS,
            oop_flop: FOUR_BP_OOP_FLOP_SIZINGS,
            oop_turn: OOP_4BP_TURN_SIZINGS,
            oop_river: OOP_4BP_RIVER_SIZINGS,
        },
        _ => SizingProfile {
            ip_flop: SRP_IP_FLOP_SIZINGS,
            ip_turn: IP_SRP_TURN_SIZINGS,
            ip_river: IP_SRP_RIVER_SIZINGS,
            oop_flop: SRP_OOP_FLOP_SIZINGS,
            oop_turn: OOP_SRP_TURN_SIZINGS,
            oop_river: OOP_SRP_RIVER_SIZINGS,
        },
    }
}

fn ip_flop_sizings(pot_type: &str) -> &'static [PostflopSizing] {
    sizing_profile(pot_type).ip_flop
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
    println!("Building game...");
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
    println!("Game built.");
    game.allocate_memory(false);
    game.cache_normalized_weights();
    println!("Solving game...");
    solve(&mut game, SOLVE_ITERATIONS, 0.0, false);
    println!("Game solved.");
    game.cache_normalized_weights();
    move_to_hero_decision(&mut game, hero_player)?;

    println!("Extracting strategy...");
    let actions = game.available_actions();
    let check_index = find_action(&actions, |action| matches!(action, Action::Check))?;
    let bet_index = find_action(&actions, |action| matches!(action, Action::Bet(_)))?;
    let hand_count = game.private_cards(hero_player).len();
    let villain_player = 1 - hero_player;
    let villain_count = game.private_cards(villain_player).len();
    let hands = player_hands(&game, hero_player)?;
    let villains = player_hands(&game, villain_player)?;
    let strategy = game.strategy().to_vec();
    println!("Extracting EVs...");
    eprintln!(
        "WARNING: expected_values_detail disabled in stat_probe to avoid normalized_weights cache panic; using frequency-based best_action."
    );
    let action_count = actions.len();
    let evs = vec![0.0; action_count * hand_count];
    let evs_available = false;
    println!("Extracting equities...");
    let (hero_equities, villain_equities) = if SAFE_MINIMAL_OUTPUT {
        eprintln!("WARNING: range_equity_hero temporarily disabled to avoid normalized_weights cache panic");
        (vec![0.0; hand_count], vec![0.0; villain_count])
    } else {
        (game.equity(hero_player).to_vec(), game.equity(villain_player).to_vec())
    };
    println!("Writing rows...");
    let range_stats = calculate_range_stats(
        &hero_equities,
        &villain_equities,
        hand_count,
        villain_count,
    );

    Ok(DecisionSolve {
        hands,
        villains,
        strategy,
        evs,
        evs_available,
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
    let bet_freq = action_value(&solve.strategy, solve.bet_index, hand_index, solve.hand_count);
    let check_ev = if solve.evs_available {
        ev_to_bb(action_value(&solve.evs, solve.check_index, hand_index, solve.hand_count))
    } else {
        0.0
    };
    let bet_ev = if solve.evs_available {
        ev_to_bb(action_value(&solve.evs, solve.bet_index, hand_index, solve.hand_count))
    } else {
        0.0
    };
    let (best_action, ev) = if solve.evs_available {
        if bet_ev > check_ev {
            ("bet", bet_ev)
        } else {
            ("check", check_ev)
        }
    } else if bet_freq > check_freq {
        ("bet", 0.0)
    } else {
        ("check", 0.0)
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
    hero_count: usize,
    villain_count: usize,
) -> RangeStats {
    let hero_weighted_value_combos: f32 = hero_equities.iter().take(hero_count).map(|&eq| value_weight(eq)).sum();
    let villain_weighted_value_combos: f32 = villain_equities.iter().take(villain_count).map(|&eq| value_weight(eq)).sum();
    let hero_total_live_combos = hero_count as f32;
    let villain_total_live_combos = villain_count as f32;
    let hero_weighted_value_pct = safe_div(hero_weighted_value_combos, hero_total_live_combos);
    let villain_weighted_value_pct = safe_div(villain_weighted_value_combos, villain_total_live_combos);
    let range_equity_hero = weighted_average_equity_without_normalized_cache(hero_equities, None, hero_count);

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

fn weighted_average_equity_without_normalized_cache(
    equities: &[f32],
    weights: Option<&[f32]>,
    combo_count: usize,
) -> f32 {
    if equities.len() < combo_count {
        eprintln!(
            "WARNING: only {} equity entries available for {combo_count} hero combos; using available entries.",
            equities.len()
        );
    }

    let mut numerator = 0.0;
    let mut denominator = 0.0;
    let usable_count = equities.len().min(combo_count);

    for index in 0..usable_count {
        let equity = equities[index];
        if !equity.is_finite() {
            eprintln!("WARNING: skipping non-finite hero equity at index {index}: {equity}");
            continue;
        }

        let weight = weights.and_then(|values| values.get(index).copied()).unwrap_or(1.0);
        if weight.is_finite() && weight > 0.0 {
            numerator += equity * weight;
            denominator += weight;
        }
    }

    if denominator <= 0.0 {
        eprintln!("WARNING: no positive hero combo weights available for range_equity_hero; using 0.0");
        return 0.0;
    }

    numerator / denominator
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
