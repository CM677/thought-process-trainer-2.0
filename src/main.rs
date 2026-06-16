use postflop_solver::*;
use std::error::Error;

const OUTPUT_FILE: &str = "output_debug_realistic_flop_first_ip_srp.csv";
const BB_FLOP_ACTIONS_OUTPUT_FILE: &str = "output_debug_bb_flop_actions.csv";

const OOP_PLAYER: usize = 0;
const IP_PLAYER: usize = 1;
const BB_CHIPS: f32 = 100.0;
const SOLVE_ITERATIONS: u32 = 500;

const FLOP: &str = "Ah7s4c";
const TURN: &str = "6s";
const TURN_BOARD: &str = "Ah7s4c6s";
const STARTING_POT: i32 = 550;
const STARTING_STACK: i32 = 9750;
const ADD_ALLIN_THRESHOLD: f64 = 0.67;
const FORCE_ALLIN_THRESHOLD: f64 = 0.8;

const BTN_RANGE: &str =
    "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,86s+,75s+,65s,54s";
const BB_RANGE: &str = "99-22,AQs-A6s,KJs-K2s,J7s-J4s,T6s,97s-96s,87s-85s,75s-74s,64s-63s,53s,43s,AJo-A6o,K9o+,QTo+,JTo,QTs-Q2s,A4s-A2s,T9o";
const UTG_RANGE: &str = "66+,A2s+,K6s+,QTs+,ATo+,KJo+,JTs,T9s,65s";
const HJ_RANGE: &str = "55+,A2s+,K5s+,Q9s+,A9o+,KTo+,QTo+,J9s+,T8s+,65s";
const CO_RANGE: &str =
    "44+,A2s+,K3s+,Q6s+,A8o+,KTo+,QTo+,JTo,J8s+,T8s+,98s,78s,67s,65s,54s,A5o";
const SB_RANGE: &str =
    "22+,A2s+,K2s+,Q2s+,A2o+,K7o+,Q9o+,J9o+,T9o,J4s+,T6s+,96s+,85s+,75s+,64s+,53s+";
const BB_VS_UTG_CALL_RANGE: &str = "JJ-22,AQs-A6s,KQs-K8s,QTs-Q9s,JTs-J9s,T9s-T8s,98s-97s,87s-86s,76s-75s,64s,54s-53s,AQo-AJo,KQo,A2s";
const BB_VS_HJ_CALL_RANGE: &str = "JJ-22,AQs-A6s,KTs-K8s,Q9s,J9s-J8s,T8s-T7s,98s-97s,87s-86s,76s-75s,64s,54s-53s,AQo-ATo,KJo+,K6s-K5s,KQo,43s,QJo,A2s";
const BB_VS_CO_CALL_RANGE: &str = "TT-22,AJs-A6s,K9s-K8s,JTs-J8s,T8s-T7s,98s-96s,87s-85s,75s,64s,54s-53s,AQo-ATo,KTo+,K6s-K2s,QTo+,JTo,Q8s-Q6s,A2s,43s";
const BB_VS_SB_CALL_RANGE: &str = "99-22,AJs-A6s,KJs-K2s,J9s-J6s,T8s-T6s,97s-95s,87s-85s,75s-74s,64s-63s,53s-52s,43s,AJo-A6o,K9o+,K6s-K2s,Q9o+,JTo,QJs-Q2s,A4s-A2s,T9o,98o";
// Temporary approximate SRP caller ranges. Refine these when dedicated preflop solves are available.
const SB_VS_BTN_CALL_RANGE: &str = "66-22,A7s-A2s,K9s-K2s,QTs-Q2s,JTs-J4s,T9s-T6s,98s-96s,87s-85s,76s-75s,65s,54s,AJo-ATo,KQo-KTo,QJo-QTo,JTo";
const SB_VS_CO_CALL_RANGE: &str = "88-22,ATs-A2s,KTs-K2s,QTs-Q6s,JTs-J8s,T9s-T8s,98s,87s,76s,65s,AQo-ATo,KQo-KTo,QJo-QTo,JTo";
const SB_VS_HJ_CALL_RANGE: &str = "99-22,ATs-A2s,KTs-K5s,QTs-Q9s,JTs,T9s,98s,87s,76s,65s,AQo-ATo,KQo-KTo,QJo";
const UTG_VS_HJ_CALL_RANGE: &str = "99-66,AQs-ATs,KQs-KJs,QJs,JTs,T9s,65s,AQo";
const BTN_VS_UTG_CALL_RANGE: &str = "JJ-77,AQs-A9s,QTs,AQo,KJs,KQs";
const BTN_VS_HJ_CALL_RANGE: &str = "JJ-66,AQs-A9s,QTs,AQo,KJs,KQs";

// Central registry retained for upcoming SRP, 3-bet, 4-bet, and 5-bet exporters.
// The current 20-spot runner uses the named constants above for its active range pairs.
#[allow(dead_code)]
const PREFLOP_RANGE_REGISTRY: &[(&str, &str)] = &[
    ("UTG RFI", UTG_RANGE),
    ("HJ RFI", HJ_RANGE),
    ("CO RFI", CO_RANGE),
    ("BTN RFI", BTN_RANGE),
    ("SB RFI", SB_RANGE),
    ("HJ vs UTG Open 3bet", "99+,ATs+,KTs+,AQo+,65s,A4s-A5s"),
    ("CO vs UTG Open 3bet", "88+,ATs+,KTs+,AQo+,65s,A4s-A5s"),
    ("CO vs HJ Open 3bet", "88+,ATs+,KTs+,AQo+,KQo,QJs,65s,A3s-A5s"),
    ("BTN vs UTG Open 3bet", "QQ+,AKs,AKo,KQo,QJs,JTs,T9s,K9s-KTs,A8s,A3s-A5s,65s"),
    ("BTN vs UTG Open call", BTN_VS_UTG_CALL_RANGE),
    ("BTN vs HJ Open 3bet", "QQ+,AKs,AKo,KQo,QJs,JTs,T9s,K9s-KTs,A3s-A8s,65s,76s,AJo"),
    ("BTN vs HJ Open call", BTN_VS_HJ_CALL_RANGE),
    ("BTN vs CO Open 3bet", "QQ+,AKs,AKo,KQo,QJs,J9s+,T9s,K8s-K9s,A2s-A3s,A5s-A7s,65s,76s,AJo-ATo,Q9s"),
    ("BTN vs CO Open call", "55-JJ,A8s-AQs,QTs,AQo,A4s,KTs+"),
    ("SB vs UTG Open 3bet", "TT+,ATs+,KTs+,QJs,AKo,A5s,65s"),
    ("SB vs HJ Open 3bet", "99+,ATs+,KTs+,QJs,AKo,JTs,A4s-A5s,65s"),
    ("SB vs CO Open 3bet", "88+,ATs+,KTs+,QTs+,AQo+,JTs,A4s-A5s,65s"),
    ("SB vs BTN Open 3bet", "66+,A7s+,K9s+,QTs+,AJo+,JTs,A4s-A5s,65s,KQo,T9s"),
    ("SB vs BTN Open call", SB_VS_BTN_CALL_RANGE),
    ("SB vs CO Open call", SB_VS_CO_CALL_RANGE),
    ("SB vs HJ Open call", SB_VS_HJ_CALL_RANGE),
    ("UTG vs HJ Open call", UTG_VS_HJ_CALL_RANGE),
    ("BB vs UTG Open 3bet", "QQ+,QJs,K7s,A5s-A3s,65s,AKs,AKo"),
    ("BB vs UTG Open call", BB_VS_UTG_CALL_RANGE),
    ("BB vs HJ Open 3bet", "QQ+,QTs+,KJs+,JTs,T9s,K7s,A5s-A3s,65s,AKs,AKo"),
    ("BB vs HJ Open call", BB_VS_HJ_CALL_RANGE),
    ("BB vs CO Open 3bet", "JJ+,AQs,Q9s+,KTs+,J9s+,T9s,K7s,A5s-A3s,65s,AKs,AKo,67s"),
    ("BB vs CO Open call", BB_VS_CO_CALL_RANGE),
    ("BB vs BTN Open 3bet", "TT+,KQs,QJs,J8s+,T7s+,A5s,65s,AKs,AQo+,67s,54s,98s,A5o"),
    ("BB vs BTN Open call", BB_RANGE),
    ("BB vs SB Open 3bet", "TT+,AQs,KQs,J8s+,T9s+,A5s,65s,AKs,AQo+,76s,54s,98s,A3o-A6o,K8o-K9o,J9o,T8o,J5s-J2s,T5s-T4s"),
    ("BB vs SB Open call", BB_VS_SB_CALL_RANGE),
    ("UTG vs HJ 3bet Defense 4bet", "KK+,AKs,AKo,AJs,KQs,A5s"),
    ("UTG vs HJ 3bet Defense call", "QQ-JJ,99-66,AQs,JTs,65s"),
    ("UTG vs CO 3bet Defense 4bet", "KK+,AKs,AKo,AJs,KQs,A5s"),
    ("UTG vs CO 3bet Defense call", "QQ-JJ,99-66,AQs,JTs,65s,KJs,ATs"),
    ("UTG vs BTN 3bet Defense 4bet", "KK+,AKs,AKo,AJs,A5s"),
    ("UTG vs BTN 3bet Defense call", "QQ-55,AQs,JTs,65s,KJs+,ATs,QJs"),
    ("UTG vs SB 3bet Defense 4bet", "KK+,AKo,A4s"),
    ("UTG vs SB 3bet Defense call", "QQ-66,AQs,JTs,65s,KJs+,ATs,QJs,AKs,AJs,A5s"),
    ("UTG vs BB 3bet Defense 4bet", "KK+,AKo,A4s"),
    ("UTG vs BB 3bet Defense call", "QQ-66,AQs,JTs,65s,KJs+,ATs,QJs,AKs,AJs,A5s,A9s"),
    ("HJ vs CO 3bet Defense 4bet", "QQ+,KJs+,AKs,AKo,AJs,A5s"),
    ("HJ vs CO 3bet Defense call", "AQs,ATs,JTs,JJ-88,55,65s"),
    ("HJ vs BTN 3bet Defense 4bet", "QQ+,KJs-KTs,AKs,AKo,A5s"),
    ("HJ vs BTN 3bet Defense call", "AQs-A9s,JTs,JJ-55,65s,KQs,T9s,QJs"),
    ("HJ vs SB 3bet Defense 4bet", "KK+,KJs-KTs,A4s"),
    ("HJ vs SB 3bet Defense call", "AQs-ATs,JTs,QQ-77,55,65s,KQs,T9s,AKs,AKo,A5s,QJs"),
    ("HJ vs BB 3bet Defense 4bet", "KK+,KTs,A3s"),
    ("HJ vs BB 3bet Defense call", "AQs-A9s,JTs,QQ-55,65s,T9s,AKs,AKo,A5s-A4s,QTs+,KTs+"),
    ("CO vs BTN 3bet Defense 4bet", "QQ+,AKs,AQo+,KTs-KJs,ATs,JTs,A4s"),
    ("CO vs BTN 3bet Defense call", "JJ-44,AJs-AQs,A9s,A5s,KQs,T9s,65s,QTs+"),
    ("CO vs SB 3bet Defense 4bet", "KK+,AKo,K9s,A4s"),
    ("CO vs SB 3bet Defense call", "QQ-77,55,ATs+,A5s,KTs+,T9s,65s,QTs+,J9s+,AQo"),
    ("CO vs BB 3bet Defense 4bet", "KK+,AKo,K9s,A7s,A2s"),
    ("CO vs BB 3bet Defense call", "QQ-55,A8s+,A5s-A3s,KTs+,T9s,65s,QTs+,JTs,AQo,67s"),
    ("BTN vs SB 3bet Defense 4bet", "QQ+,AJs,A7s,A3s,K9s,AKo,AJo"),
    ("BTN vs SB 3bet Defense call", "JJ-44,A8s+,A4s-A5s,KTs+,QTs+,J9s+,T8s+,98s,87s,67s,65s,54s,KQo,AQo"),
    ("BTN vs BB 3bet Defense 4bet", "QQ+,AJs,A2s,K7s-K6s,AKo,AJo"),
    ("BTN vs BB 3bet Defense call", "JJ-44,A3s+,K8s+,Q9s+,J8s+,T8s+,98s,87s,67s,65s,54s,KQo,AQo"),
    ("SB vs BB 3bet Defense 4bet", "AA-JJ,ATo+,AKs,A6s,A3s-A2s,K5s"),
    ("SB vs BB 3bet Defense call", "AQs-A7s,A5s-A4s,K8s+,Q9s+,J8s+,T8s+,76s,65s,54s,TT-44"),
    ("HJ vs UTG 4bet Defense shove", "KK,AKo"),
    ("HJ vs UTG 4bet Defense call", "AA,ATs+,QQ-JJ,99,65s"),
    ("CO vs UTG 4bet Defense shove", "KK,AKo"),
    ("CO vs UTG 4bet Defense call", "AA,ATs+,QQ-JJ,99,65s,KQs"),
    ("CO vs HJ 4bet Defense shove", "KK,AKo"),
    ("CO vs HJ 4bet Defense call", "AA,ATs+,QQ-JJ,99,65s,KQs,A5s"),
    ("BTN vs UTG 4bet Defense shove", "KK,AKo"),
    ("BTN vs UTG 4bet Defense call", "AA,AKs,QQ,T9s,65s,A5s"),
    ("BTN vs HJ 4bet Defense shove", "KK,AKo"),
    ("BTN vs HJ 4bet Defense call", "AA,AKs,QQ,T9s,65s,A5s,76s"),
    ("BTN vs CO 4bet Defense shove", "KK,AKo"),
    ("BTN vs CO 4bet Defense call", "AA,AKs,QQ,T9s,65s,A5s,76s,J9s"),
    ("SB vs UTG 4bet Defense shove", "KK+,AKo"),
    ("SB vs UTG 4bet Defense call", "AKs,AJs,QQ,65s"),
    ("SB vs HJ 4bet Defense shove", "KK+,AKo"),
    ("SB vs HJ 4bet Defense call", "AJs+,QQ-JJ,99,65s"),
    ("SB vs CO 4bet Defense shove", "KK+,AKo,A5s"),
    ("SB vs CO 4bet Defense call", "AJs+,QQ-JJ,99-88,65s,JTs"),
    ("SB vs BTN 4bet Defense shove", "QQ+,AKo,AKs,A5s"),
    ("SB vs BTN 4bet Defense call", "ATs+,JJ-88,65s,JTs,66,T9s,QTs,KQs,AQo"),
    ("BB vs UTG 4bet Defense shove", "AA,A5s"),
    ("BB vs UTG 4bet Defense call", "AKs,KK-QQ,65s"),
    ("BB vs HJ 4bet Defense shove", "KK+,AKo"),
    ("BB vs HJ 4bet Defense call", "AKs,QQ,65s,T9s"),
    ("BB vs CO 4bet Defense shove", "KK+,AKo,JJ"),
    ("BB vs CO 4bet Defense call", "AKs,QQ,65s,T9s,JTs,AQs"),
    ("BB vs BTN 4bet Defense shove", "KK-JJ,AKo,A5s"),
    ("BB vs BTN 4bet Defense call", "AKs,65s,T9s,JTs,76s,TT,AQo,KQs,AQs,AA"),
    ("BB vs SB 4bet Defense shove", "KK-JJ,AKo,A5s,KQs"),
    ("BB vs SB 4bet Defense call", "AA,AKs-AQs,65s,T9s,JTs,76s,TT,AQo,KQs,98s,54s"),
    ("UTG vs HJ 5bet Shove Defense call", "KK+,AKs"),
    ("UTG vs CO 5bet Shove Defense call", "KK+,AKs"),
    ("UTG vs BTN 5bet Shove Defense call", "KK+,AKs"),
    ("UTG vs SB 5bet Shove Defense call", "KK+"),
    ("UTG vs BB 5bet Shove Defense call", "KK+"),
    ("HJ vs CO 5bet Shove Defense call", "KK+,AKs,AKo"),
    ("HJ vs BTN 5bet Shove Defense call", "KK+,AKs,AKo"),
    ("HJ vs SB 5bet Shove Defense call", "KK+"),
    ("HJ vs BB 5bet Shove Defense call", "KK+"),
    ("CO vs BTN 5bet Shove Defense call", "QQ+,AKs,AKo"),
    ("CO vs SB 5bet Shove Defense call", "KK+,AKo"),
    ("CO vs BB 5bet Shove Defense call", "KK+,AKo"),
    ("BTN vs SB 5bet Shove Defense call", "QQ+,AKo"),
    ("BTN vs BB 5bet Shove Defense call", "QQ+,AKo"),
    ("SB vs BB 5bet Shove Defense call", "JJ+,AKo,AKs"),
];

#[allow(dead_code)]
const DEBUG_SCENARIOS_20: [DebugScenario; 20] = [
    DebugScenario {
        spot_id: 1,
        hero_position: "BTN",
        villain_position: "BB",
        reference_hand: "Kh4h",
        board: "3s4s5c",
        full_board: "3s4s5c6d2h",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(BB_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 2,
        hero_position: "UTG",
        villain_position: "BB",
        reference_hand: "Ah6h",
        board: "Ad7d4h",
        full_board: "Ad7d4h2d4c",
        ip_range: Some(UTG_RANGE),
        oop_range: Some(BB_VS_UTG_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 7,
        hero_position: "CO",
        villain_position: "BB",
        reference_hand: "AhTs",
        board: "7d8h4c",
        full_board: "7d8h4c2sQh",
        ip_range: Some(CO_RANGE),
        oop_range: Some(BB_VS_CO_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 14,
        hero_position: "CO",
        villain_position: "BB",
        reference_hand: "AsQh",
        board: "Jd2d8d",
        full_board: "Jd2d8d4c4d",
        ip_range: Some(CO_RANGE),
        oop_range: Some(BB_VS_CO_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 17,
        hero_position: "BTN",
        villain_position: "BB",
        reference_hand: "Ks9d",
        board: "3hKdAh",
        full_board: "3hKdAh4hQc",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(BB_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 21,
        hero_position: "BTN",
        villain_position: "SB",
        reference_hand: "5s4s",
        board: "9s4h2h",
        full_board: "9s4h2h4dAd",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(SB_VS_BTN_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 24,
        hero_position: "UTG",
        villain_position: "BB",
        reference_hand: "Tc9c",
        board: "5h7s9d",
        full_board: "5h7s9d8c9h",
        ip_range: Some(UTG_RANGE),
        oop_range: Some(BB_VS_UTG_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 29,
        hero_position: "BTN",
        villain_position: "BB",
        reference_hand: "Qc5c",
        board: "Jh4h8d",
        full_board: "Jh4h8d8hJs",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(BB_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 32,
        hero_position: "BTN",
        villain_position: "BB",
        reference_hand: "Ah4d",
        board: "4s2s2d",
        full_board: "4s2s2dKhJs",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(BB_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 33,
        hero_position: "BB",
        villain_position: "SB",
        reference_hand: "Kd3d",
        board: "JhAdQs",
        full_board: "JhAdQsAs2c",
        ip_range: Some(BB_VS_SB_CALL_RANGE),
        oop_range: Some(SB_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 39,
        hero_position: "UTG",
        villain_position: "BB",
        reference_hand: "JcTc",
        board: "KdAd4d",
        full_board: "KdAd4dJd9s",
        ip_range: Some(UTG_RANGE),
        oop_range: Some(BB_VS_UTG_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 41,
        hero_position: "UTG",
        villain_position: "BB",
        reference_hand: "QcTc",
        board: "AcAs4c",
        full_board: "AcAs4c2c3s",
        ip_range: Some(UTG_RANGE),
        oop_range: Some(BB_VS_UTG_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 45,
        hero_position: "HJ",
        villain_position: "UTG",
        reference_hand: "AdJd",
        board: "4sKcKh",
        full_board: "4sKcKh2h8h",
        ip_range: Some(HJ_RANGE),
        oop_range: Some(UTG_VS_HJ_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 46,
        hero_position: "CO",
        villain_position: "BB",
        reference_hand: "7d8d",
        board: "JsThKh",
        full_board: "JsThKh8sAs",
        ip_range: Some(CO_RANGE),
        oop_range: Some(BB_VS_CO_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 49,
        hero_position: "HJ",
        villain_position: "SB",
        reference_hand: "KdTs",
        board: "KcQc8d",
        full_board: "KcQc8d7cJd",
        ip_range: Some(HJ_RANGE),
        oop_range: Some(SB_VS_HJ_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 55,
        hero_position: "BTN",
        villain_position: "BB",
        reference_hand: "9d8d",
        board: "Td9hAd",
        full_board: "Td9hAdQh4d",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(BB_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 60,
        hero_position: "BTN",
        villain_position: "SB",
        reference_hand: "7d5d",
        board: "Td4c3h",
        full_board: "Td4c3h3dTc",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(SB_VS_BTN_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 62,
        hero_position: "CO",
        villain_position: "BB",
        reference_hand: "8s8c",
        board: "Qc5sKh",
        full_board: "Qc5sKhKcTd",
        ip_range: Some(CO_RANGE),
        oop_range: Some(BB_VS_CO_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 64,
        hero_position: "CO",
        villain_position: "SB",
        reference_hand: "KdTs",
        board: "5dAsTh",
        full_board: "5dAsTh8cJd",
        ip_range: Some(CO_RANGE),
        oop_range: Some(SB_VS_CO_CALL_RANGE),
        missing_range_key: None,
    },
    DebugScenario {
        spot_id: 66,
        hero_position: "BTN",
        villain_position: "BB",
        reference_hand: "7h7c",
        board: "2s8d7s",
        full_board: "2s8d7s9hKc",
        ip_range: Some(BTN_RANGE),
        oop_range: Some(BB_RANGE),
        missing_range_key: None,
    },
];

macro_rules! debug_scenario {
    ($spot:literal, "UTG", "BB", $hand:literal, $board:literal, $full:literal) => {
        DebugScenario { spot_id: $spot, hero_position: "UTG", villain_position: "BB", reference_hand: $hand, board: $board, full_board: $full, ip_range: Some(UTG_RANGE), oop_range: Some(BB_VS_UTG_CALL_RANGE), missing_range_key: None }
    };
    ($spot:literal, "HJ", "BB", $hand:literal, $board:literal, $full:literal) => {
        DebugScenario { spot_id: $spot, hero_position: "HJ", villain_position: "BB", reference_hand: $hand, board: $board, full_board: $full, ip_range: Some(HJ_RANGE), oop_range: Some(BB_VS_HJ_CALL_RANGE), missing_range_key: None }
    };
    ($spot:literal, "CO", "BB", $hand:literal, $board:literal, $full:literal) => {
        DebugScenario { spot_id: $spot, hero_position: "CO", villain_position: "BB", reference_hand: $hand, board: $board, full_board: $full, ip_range: Some(CO_RANGE), oop_range: Some(BB_VS_CO_CALL_RANGE), missing_range_key: None }
    };
    ($spot:literal, "BTN", "BB", $hand:literal, $board:literal, $full:literal) => {
        DebugScenario { spot_id: $spot, hero_position: "BTN", villain_position: "BB", reference_hand: $hand, board: $board, full_board: $full, ip_range: Some(BTN_RANGE), oop_range: Some(BB_RANGE), missing_range_key: None }
    };
    ($spot:literal, "BB", "SB", $hand:literal, $board:literal, $full:literal) => {
        DebugScenario { spot_id: $spot, hero_position: "BB", villain_position: "SB", reference_hand: $hand, board: $board, full_board: $full, ip_range: Some(BB_VS_SB_CALL_RANGE), oop_range: Some(SB_RANGE), missing_range_key: None }
    };
    ($spot:literal, "BTN", "UTG", $hand:literal, $board:literal, $full:literal) => {
        DebugScenario { spot_id: $spot, hero_position: "BTN", villain_position: "UTG", reference_hand: $hand, board: $board, full_board: $full, ip_range: Some(BTN_VS_UTG_CALL_RANGE), oop_range: Some(UTG_RANGE), missing_range_key: None }
    };
    ($spot:literal, "BTN", "HJ", $hand:literal, $board:literal, $full:literal) => {
        DebugScenario { spot_id: $spot, hero_position: "BTN", villain_position: "HJ", reference_hand: $hand, board: $board, full_board: $full, ip_range: Some(BTN_VS_HJ_CALL_RANGE), oop_range: Some(HJ_RANGE), missing_range_key: None }
    };
}

#[allow(dead_code)]
const ALL_DEBUG_SCENARIOS: [DebugScenario; 100] = [
    debug_scenario!(1, "BTN", "BB", "Kh4h", "3s4s5c", "3s4s5c6d2h"),
    debug_scenario!(2, "UTG", "BB", "Ah6h", "Ad7d4h", "Ad7d4h2d4c"),
    debug_scenario!(7, "CO", "BB", "AhTs", "7d8h4c", "7d8h4c2sQh"),
    debug_scenario!(13, "CO", "BB", "AsQh", "Jd2d8d", "Jd2d8d4c4d"),
    debug_scenario!(16, "BTN", "BB", "Ks9d", "3hKdAh", "3hKdAh4hQc"),
    debug_scenario!(20, "UTG", "BB", "Tc9c", "5h7s9d", "5h7s9d8c9h"),
    debug_scenario!(25, "BTN", "BB", "Qc5c", "Jh4h8d", "Jh4h8d8hJs"),
    debug_scenario!(28, "BTN", "BB", "Ah4d", "4s2s2d", "4s2s2dKhJs"),
    debug_scenario!(29, "BB", "SB", "Kd3d", "JhAdQs", "JhAdQsAs2c"),
    debug_scenario!(35, "UTG", "BB", "JcTc", "KdAd4d", "KdAd4dJd9s"),
    debug_scenario!(37, "UTG", "BB", "QcTc", "AcAs4c", "AcAs4c2c3s"),
    debug_scenario!(40, "CO", "BB", "7d8d", "JsThKh", "JsThKh8sAs"),
    debug_scenario!(48, "BTN", "BB", "9d8d", "Td9hAd", "Td9hAdQh4d"),
    debug_scenario!(54, "CO", "BB", "8s8c", "Qc5sKh", "Qc5sKhKcTd"),
    debug_scenario!(57, "BTN", "BB", "7h7c", "2s8d7s", "2s8d7s9hKc"),
    debug_scenario!(64, "CO", "BB", "KhKc", "5d4s9h", "5d4s9hQs9s"),
    debug_scenario!(69, "BTN", "BB", "Qd9h", "ThKdAd", "ThKdAd9cTs"),
    debug_scenario!(74, "BTN", "UTG", "8d8c", "7sQsAh", "7sQsAh7c8h"),
    debug_scenario!(75, "BB", "SB", "Ac8h", "4s5s3h", "4s5s3h4d8s"),
    debug_scenario!(82, "UTG", "BB", "AsKs", "9dAc9c", "9dAc9cTh2c"),
    debug_scenario!(91, "UTG", "BB", "AhTd", "TsJhAc", "TsJhAc3s6h"),
    debug_scenario!(92, "BB", "SB", "7h4h", "Ad3cAc", "Ad3cAcJcQh"),
    debug_scenario!(93, "CO", "BB", "Ah9h", "KdQd4d", "KdQd4dAs2d"),
    debug_scenario!(95, "BTN", "HJ", "JdJs", "QdJcTc", "QdJcTc7s5d"),
    debug_scenario!(96, "BTN", "BB", "TdTc", "3cJdAs", "3cJdAs6hKd"),
    debug_scenario!(98, "CO", "BB", "AsKs", "7c8cTs", "7c8cTs4d9h"),
    debug_scenario!(99, "BTN", "HJ", "TdTh", "4hTcAh", "4hTcAhJh8s"),
    debug_scenario!(102, "BTN", "BB", "Ac8c", "Ah7d9c", "Ah7d9c3s9d"),
    debug_scenario!(103, "HJ", "BB", "As3s", "9h9sAc", "9h9sAcKh8c"),
    debug_scenario!(110, "BTN", "BB", "Jc7c", "8sTs7d", "8sTs7dTd9d"),
    debug_scenario!(113, "BTN", "UTG", "QsTs", "Th7dKc", "Th7dKc4sJd"),
    debug_scenario!(116, "BTN", "UTG", "KsJs", "8s6h3d", "8s6h3d9d9s"),
    debug_scenario!(118, "CO", "BB", "Qh6h", "6d6c5s", "6d6c5sJsTd"),
    debug_scenario!(119, "BB", "SB", "QdTc", "8dTh2d", "8dTh2d8cAc"),
    debug_scenario!(124, "BTN", "HJ", "Td9d", "4h8cKc", "4h8cKc9c3h"),
    debug_scenario!(126, "UTG", "BB", "6h5h", "Kd8h3c", "Kd8h3c4d2d"),
    debug_scenario!(127, "HJ", "BB", "Kh7h", "8dAhJc", "8dAhJc3cQd"),
    debug_scenario!(130, "HJ", "BB", "Kh6h", "8s9sAs", "8s9sAs5c3s"),
    debug_scenario!(131, "BTN", "BB", "9s6s", "Ac7cJs", "Ac7cJs7s3s"),
    debug_scenario!(141, "BTN", "BB", "Jd9d", "5cTc9s", "5cTc9sTd8h"),
    debug_scenario!(142, "BTN", "UTG", "As9s", "9hAh8h", "9hAh8h9cTh"),
    debug_scenario!(146, "BTN", "HJ", "Ah9h", "9sQsQd", "9sQsQd7d5d"),
    debug_scenario!(148, "BTN", "UTG", "8d8s", "Td8c7d", "Td8c7dQs8h"),
    debug_scenario!(149, "UTG", "BB", "Kh6h", "3d9sQh", "3d9sQhTc3h"),
    debug_scenario!(153, "BTN", "HJ", "AcJc", "8d3d5d", "8d3d5d4c9d"),
    debug_scenario!(154, "BTN", "HJ", "8s8d", "Ts5h8h", "Ts5h8hKs5c"),
    debug_scenario!(155, "BB", "SB", "Ad9d", "AcJc9s", "AcJc9sKh7s"),
    debug_scenario!(156, "BTN", "BB", "Ac5c", "7cAd2d", "7cAd2d2c7h"),
    debug_scenario!(160, "UTG", "BB", "JcTc", "AdJd2c", "AdJd2cTh5h"),
    debug_scenario!(161, "BTN", "BB", "Jd5d", "8d4d6d", "8d4d6dTd2s"),
    debug_scenario!(163, "BTN", "HJ", "7d7c", "9cKs9d", "9cKs9d4c4h"),
    debug_scenario!(164, "BTN", "BB", "Ac5s", "Ah8sJd", "Ah8sJd2dTh"),
    debug_scenario!(167, "BTN", "BB", "5c4c", "QsTh6s", "QsTh6sJs2c"),
    debug_scenario!(176, "BTN", "BB", "Qh9c", "9s2hKs", "9s2hKsAs9h"),
    debug_scenario!(180, "UTG", "BB", "AhTc", "9s4cQd", "9s4cQd6c2c"),
    debug_scenario!(192, "BTN", "UTG", "Ac9c", "As5sJh", "As5sJhTcAd"),
    debug_scenario!(193, "HJ", "BB", "AcTc", "As7d2c", "As7d2c2h3s"),
    debug_scenario!(197, "BB", "SB", "Js8s", "Kd3sKh", "Kd3sKh9s4h"),
    debug_scenario!(203, "BB", "SB", "Kh2h", "3d5c9h", "3d5c9h7h2c"),
    debug_scenario!(205, "BB", "SB", "3c3d", "QsQhAs", "QsQhAsJd7h"),
    debug_scenario!(206, "BTN", "HJ", "Ah9h", "4c6c2h", "4c6c2h4d8d"),
    debug_scenario!(209, "BB", "SB", "Qc5c", "Qs2hJd", "Qs2hJdTh5d"),
    debug_scenario!(212, "BTN", "BB", "Td9d", "Ac5cKc", "Ac5cKc5d8h"),
    debug_scenario!(216, "BB", "SB", "Qh8h", "4hJdJs", "4hJdJs7s6c"),
    debug_scenario!(217, "CO", "BB", "Kc3c", "Jd8s7d", "Jd8s7d6d4h"),
    debug_scenario!(218, "UTG", "BB", "As5s", "3hThAc", "3hThAc3c3s"),
    debug_scenario!(224, "CO", "BB", "Kh3h", "7h4sQc", "7h4sQc9c5c"),
    debug_scenario!(228, "BTN", "BB", "6s5s", "8s9h5h", "8s9h5h3sQh"),
    debug_scenario!(229, "UTG", "BB", "As9s", "JsTs3d", "JsTs3dQs4d"),
    debug_scenario!(231, "BTN", "HJ", "7s7h", "7dQh2h", "7dQh2h8s3s"),
    debug_scenario!(234, "UTG", "BB", "AcQh", "8sKcTh", "8sKcTh5d8c"),
    debug_scenario!(245, "BTN", "UTG", "Ad9d", "8h8s9s", "8h8s9sQd5s"),
    debug_scenario!(246, "BB", "SB", "Kd5d", "AcQsQc", "AcQsQc2h8c"),
    debug_scenario!(247, "UTG", "BB", "Ks6s", "As5dAh", "As5dAh9s2d"),
    debug_scenario!(248, "UTG", "BB", "6d6s", "9h5c6h", "9h5c6h2d8d"),
    debug_scenario!(252, "BB", "SB", "8s6s", "5s6hTs", "5s6hTsTdKc"),
    debug_scenario!(256, "BB", "SB", "Jc7c", "4hQc7h", "4hQc7hJd7s"),
    debug_scenario!(260, "BTN", "HJ", "AcQd", "2h2cAs", "2h2cAs4s4h"),
    debug_scenario!(262, "UTG", "BB", "KcQc", "2s7h3h", "2s7h3h4c8c"),
    debug_scenario!(269, "BTN", "HJ", "AdTd", "3dKh3s", "3dKh3sJs2h"),
    debug_scenario!(272, "BTN", "BB", "AdQd", "8c2c7d", "8c2c7d6h9c"),
    debug_scenario!(274, "BB", "SB", "8s6s", "8c5h9c", "8c5h9cQc9d"),
    debug_scenario!(280, "BB", "SB", "Ks9d", "4s9cJd", "4s9cJd9s3s"),
    debug_scenario!(285, "BTN", "BB", "Ts9h", "9s5sJh", "9s5sJhQh5h"),
    debug_scenario!(288, "BB", "SB", "9c7c", "6c6hJh", "6c6hJhTc8s"),
    debug_scenario!(289, "BTN", "BB", "As2s", "6hTc7c", "6hTc7cTs3d"),
    debug_scenario!(290, "BB", "SB", "QcTd", "KhAd3c", "KhAd3cTc6d"),
    debug_scenario!(292, "CO", "BB", "QdQc", "8dKhKc", "8dKhKcQsAh"),
    debug_scenario!(295, "CO", "BB", "JcTc", "5cAsJh", "5cAsJh9dQs"),
    debug_scenario!(297, "BTN", "BB", "AhQh", "Ac6s6d", "Ac6s6d7cJd"),
    debug_scenario!(298, "UTG", "BB", "8h8s", "KhJd5c", "KhJd5c2dAs"),
    debug_scenario!(301, "BB", "SB", "As5s", "5h8s6s", "5h8s6s4d4s"),
    debug_scenario!(302, "BTN", "BB", "Ad2c", "4h7c9s", "4h7c9s6sJh"),
    debug_scenario!(305, "BTN", "BB", "6h5h", "8c5c6s", "8c5c6s2d8d"),
    debug_scenario!(312, "UTG", "BB", "8s8h", "Kc7dQs", "Kc7dQs8cAd"),
    debug_scenario!(313, "CO", "BB", "Qc6c", "9c9d4h", "9c9d4h5d6h"),
    debug_scenario!(316, "BB", "SB", "6c6h", "QhJdKc", "QhJdKcQs4d"),
    debug_scenario!(322, "HJ", "BB", "8d8h", "Td6sJc", "Td6sJcAsQd"),
    debug_scenario!(325, "BB", "SB", "6c3c", "4hQc7h", "4hQc7hAd4d"),
    debug_scenario!(331, "UTG", "BB", "AdTh", "5h6h3c", "5h6h3cAs6c"),
];

const DEBUG_SCENARIOS: [DebugScenario; 1] = [
    debug_scenario!(1, "BTN", "BB", "Kh4h", "3s4s5c", "3s4s5c6d2h"),
];

#[derive(Clone, Copy)]
struct DebugScenario {
    spot_id: u32,
    hero_position: &'static str,
    villain_position: &'static str,
    reference_hand: &'static str,
    board: &'static str,
    full_board: &'static str,
    ip_range: Option<&'static str>,
    oop_range: Option<&'static str>,
    missing_range_key: Option<&'static str>,
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

struct SolvedDebugBranch {
    decision: DecisionSolve,
    bb_actions: BbFlopActionExtract,
}

struct BbFlopActionExtract {
    hands: Vec<HandCombo>,
    ip_hands: Vec<HandCombo>,
    branch_suffix: &'static str,
    branch_size: &'static str,
    root_found: bool,
    check_found: bool,
    donk33_found: bool,
    donk75_found: bool,
    vs_ip_bet_found: bool,
    donk33_vs_raise_found: bool,
    donk75_vs_raise_found: bool,
    ip_vs_donk_found: bool,
    ip_vs_check_raise_found: bool,
    check_freq: Vec<Option<f32>>,
    donk33_freq: Vec<Option<f32>>,
    donk75_freq: Vec<Option<f32>>,
    vs_bet_fold_freq: Vec<Option<f32>>,
    vs_bet_call_freq: Vec<Option<f32>>,
    vs_bet_raise_freq: Vec<Option<f32>>,
    donk33_vs_raise_fold_freq: Vec<Option<f32>>,
    donk33_vs_raise_call_freq: Vec<Option<f32>>,
    donk75_vs_raise_fold_freq: Vec<Option<f32>>,
    donk75_vs_raise_call_freq: Vec<Option<f32>>,
    btn_vs_donk_fold_freq: Vec<Option<f32>>,
    btn_vs_donk_call_freq: Vec<Option<f32>>,
    btn_vs_donk_raise_freq: Vec<Option<f32>>,
    btn_vs_check_raise_fold_freq: Vec<Option<f32>>,
    btn_vs_check_raise_call_freq: Vec<Option<f32>>,
}

struct ActionExportCounts {
    total_rows: usize,
    bb_rows: usize,
    btn_rows: usize,
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
    ];

    let scenarios = ALL_DEBUG_SCENARIOS
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, scenario)| {
            scenario.ip_range.is_some()
                && scenario.oop_range.is_some()
                && scenario.missing_range_key.is_none()
        })
        .take(100)
        .collect::<Vec<_>>();

    println!("Running first 100 IP SRP realistic flop ranges");
    println!("Using realistic flop tree, not check-down tree");
    println!("CODEX_TEST_FIRST_100_REALISTIC_TREE_WITH_BTN_AND_BB_ACTION_STATS");
    println!(
        "Starting pot {:.1}BB, effective stack {:.1}BB",
        STARTING_POT as f32 / BB_CHIPS,
        STARTING_STACK as f32 / BB_CHIPS
    );
    println!("TURN/RIVER STILL SIMPLIFIED IN THIS DEBUG TREE");
    println!(
        "Scoring config: raw strength brackets = 0-8,8-17,17-27,27-38,38-50,50-62,62-73,73-83,83-92,92-100."
    );
    println!("Nut advantage thresholds = 80/90.");
    println!("Blockers-to-value thresholds = 70/85.");
    println!("CODEX_TEST_EXPORT_BB_FLOP_ACTIONS");
    let mut writer = csv::Writer::from_path(OUTPUT_FILE)?;
    write_debug_header(&mut writer)?;
    let mut bb_writer = csv::Writer::from_path(BB_FLOP_ACTIONS_OUTPUT_FILE)?;
    write_bb_flop_actions_header(&mut bb_writer)?;
    let mut total_rows = 0usize;
    let mut total_bb_rows = 0usize;
    let mut total_bb_hand_rows = 0usize;
    let mut total_btn_action_rows = 0usize;
    let mut scenarios_exported = std::collections::BTreeSet::new();
    let mut branch_solves = 0usize;

    for (attempt_index, (index, scenario)) in scenarios.iter().copied().enumerate() {
        let attempt_number = attempt_index + 1;
        let scenario_id = index + 1;
        println!(
            "Scenario {}/100: spot_id={} {} vs {} board={} tree_mode=REALISTIC_FLOP_TREE",
            attempt_number,
            scenario.spot_id,
            scenario.hero_position,
            scenario.villain_position,
            scenario.board
        );
        match hero_range_combo_counts(&scenario) {
            Ok((total_combos, live_combos)) => {
                let skipped_board_combos = total_combos.saturating_sub(live_combos);
                println!("Skipped {skipped_board_combos} combos because they contained board cards.");
            }
            Err(error) => {
                println!(
                    "Scenario spot_id={} combo-count diagnostic failed: {error}",
                    scenario.spot_id
                );
            }
        }

        for sizing in flop_sizings {
            println!(
                "Scenario {}/100 spot_id={} spot_name={} board={} branch={} tree_mode=REALISTIC_FLOP_TREE",
                attempt_number,
                scenario.spot_id,
                debug_spot_name(&scenario, sizing.suffix),
                scenario.board,
                sizing.tree_size
            );
            match solve_debug_scenario(scenario, sizing) {
                Ok(solved) => {
                    let added_btn_rows = export_debug_range_rows(
                        &mut writer,
                        scenario_id,
                        &scenario,
                        &solved.decision,
                    )?;
                    let action_counts = export_bb_flop_actions_rows(
                        &mut bb_writer,
                        scenario_id as u32,
                        &scenario,
                        &solved.bb_actions,
                    )?;
                    total_rows += added_btn_rows;
                    total_bb_rows += action_counts.total_rows;
                    total_bb_hand_rows += action_counts.bb_rows;
                    total_btn_action_rows += action_counts.btn_rows;
                    branch_solves += 1;
                    scenarios_exported.insert(scenario.spot_id);
                    println!("Existing BTN CSV row count added: {added_btn_rows}");
                    println!("BB_HAND_ACTIONS rows added: {}", action_counts.bb_rows);
                    println!("BTN_HAND_ACTIONS rows added: {}", action_counts.btn_rows);
                    println!("BB root node found: {}", solved.bb_actions.root_found);
                    println!(
                        "BB response after check -> BTN bet found: {}",
                        solved.bb_actions.vs_ip_bet_found
                    );
                    println!(
                        "BTN response vs BB donk found: {}",
                        solved.bb_actions.ip_vs_donk_found
                    );
                    println!(
                        "BB response after donk -> BTN raise found: {}",
                        solved.bb_actions.donk33_vs_raise_found
                            || solved.bb_actions.donk75_vs_raise_found
                    );
                    println!(
                        "BTN response vs BB check-raise found: {}",
                        solved.bb_actions.ip_vs_check_raise_found
                    );
                }
                Err(error) => {
                    println!(
                        "Scenario spot_id={} branch {} failed: {error}. Continuing.",
                        scenario.spot_id, sizing.tree_size
                    );
                }
            }
        }
    }

    writer.flush()?;
    bb_writer.flush()?;
    println!("Total scenarios attempted: {}", scenarios.len());
    println!("Total scenarios solved/exported: {}", scenarios_exported.len());
    println!("Total branch solves: {branch_solves}");
    println!("Output path for existing BTN/IP CSV: {OUTPUT_FILE}");
    println!("Output path for {BB_FLOP_ACTIONS_OUTPUT_FILE}: {BB_FLOP_ACTIONS_OUTPUT_FILE}");
    println!("Total BTN/IP rows: {total_rows}");
    println!("Total BB CSV rows: {total_bb_rows}");
    println!("BB_HAND_ACTIONS rows: {total_bb_hand_rows}");
    println!("BTN_HAND_ACTIONS rows: {total_btn_action_rows}");
    Ok(())
}

fn solve_debug_scenario(
    scenario: DebugScenario,
    sizing: Sizing,
) -> Result<SolvedDebugBranch, Box<dyn Error>> {
    let ip_range_text = scenario
        .ip_range
        .ok_or_else(|| format!("missing hero range for Spot {}", scenario.spot_id))?;
    let oop_range_text = scenario
        .oop_range
        .ok_or_else(|| format!("missing villain range for Spot {}", scenario.spot_id))?;
    let ip_range = normalize_range_for_solver(ip_range_text).parse()?;
    let oop_range = normalize_range_for_solver(oop_range_text).parse()?;
    let mut game = build_debug_flop_game(
        scenario,
        sizing,
        ip_range,
        oop_range,
    )?;
    game.allocate_memory(false);
    solve(&mut game, SOLVE_ITERATIONS, 0.5, true);
    let bb_actions = extract_bb_flop_actions(&mut game, scenario, sizing)?;
    move_to_ip_decision(&mut game)?;
    let decision = extract_decision(
        game,
        DecisionMeta {
            street: "flop",
            board: scenario.board,
            turn_card: "null",
            flop_action: "null",
            flop_bet_size: sizing.csv_size,
            turn_bet_size: "null",
        },
    )?;
    Ok(SolvedDebugBranch {
        decision,
        bb_actions,
    })
}

fn build_debug_flop_game(
    scenario: DebugScenario,
    sizing: Sizing,
    ip_range: Range,
    oop_range: Range,
) -> Result<PostFlopGame, Box<dyn Error>> {
    let card_config = CardConfig {
        range: [oop_range, ip_range],
        flop: flop_from_str(scenario.board)?,
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
            BetSizeOptions::try_from(("33%, 75%", "3x"))?,
            BetSizeOptions::try_from((sizing.tree_size, "3x"))?,
        ],
        turn_bet_sizes: Default::default(),
        river_bet_sizes: Default::default(),
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: ADD_ALLIN_THRESHOLD,
        force_allin_threshold: FORCE_ALLIN_THRESHOLD,
        merging_threshold: 0.0,
    };
    let mut action_tree = ActionTree::new(tree_config)?;
    prune_actions_after_raise(&mut action_tree)?;
    audit_realistic_flop_tree(&mut action_tree, scenario, sizing)?;
    Ok(PostFlopGame::with_config(
        card_config,
        action_tree,
    )?)
}

fn prune_actions_after_raise(tree: &mut ActionTree) -> Result<(), String> {
    let mut lines_to_remove = Vec::new();
    collect_reraise_lines(tree, &mut lines_to_remove)?;
    tree.back_to_root();
    for line in lines_to_remove {
        tree.remove_line(&line)?;
    }
    Ok(())
}

fn collect_reraise_lines(
    tree: &mut ActionTree,
    lines_to_remove: &mut Vec<Vec<Action>>,
) -> Result<(), String> {
    let actions = tree.available_actions().to_vec();
    let facing_raise = matches!(tree.history().last(), Some(Action::Raise(_)));

    for action in actions {
        if facing_raise && matches!(action, Action::Raise(_) | Action::AllIn(_)) {
            let mut line = tree.history().to_vec();
            line.push(action);
            lines_to_remove.push(line);
            continue;
        }

        tree.play(action)?;
        if !tree.is_terminal_node() {
            collect_reraise_lines(tree, lines_to_remove)?;
        }
        tree.undo()?;
    }
    Ok(())
}

fn audit_realistic_flop_tree(
    tree: &mut ActionTree,
    scenario: DebugScenario,
    sizing: Sizing,
) -> Result<(), String> {
    tree.back_to_root();
    let oop_root_actions = tree.available_actions().to_vec();
    let oop_check = find_tree_action(&oop_root_actions, |action| matches!(action, Action::Check))?;
    let oop_donks = oop_root_actions
        .iter()
        .copied()
        .filter(|action| matches!(action, Action::Bet(_)))
        .collect::<Vec<_>>();

    tree.play(oop_check)?;
    let ip_after_check_actions = tree.available_actions().to_vec();
    let ip_bet = find_tree_action(&ip_after_check_actions, |action| matches!(action, Action::Bet(_)))?;

    tree.play(ip_bet)?;
    let oop_response_to_ip_bet = tree.available_actions().to_vec();
    let oop_raise =
        find_tree_action(&oop_response_to_ip_bet, |action| matches!(action, Action::Raise(_)))?;

    tree.play(oop_raise)?;
    let ip_response_to_check_raise = tree.available_actions().to_vec();
    tree.back_to_root();

    let mut donk_response_logs = Vec::new();
    let mut can_ip_raise_donk = true;
    let mut oop_after_ip_raise_logs = Vec::new();
    for donk in &oop_donks {
        tree.play(*donk)?;
        let ip_response = tree.available_actions().to_vec();
        let ip_raise = find_tree_action(&ip_response, |action| matches!(action, Action::Raise(_)))?;
        donk_response_logs.push(format!("{donk:?} -> {ip_response:?}"));
        tree.play(ip_raise)?;
        let oop_after_raise = tree.available_actions().to_vec();
        require_fold_call_only("OOP response to IP raise versus donk", &oop_after_raise)?;
        oop_after_ip_raise_logs.push(format!("{donk:?}, {ip_raise:?} -> {oop_after_raise:?}"));
        can_ip_raise_donk &= has_tree_action(&ip_response, |action| matches!(action, Action::Raise(_)));
        tree.back_to_root();
    }

    let can_oop_donk = oop_donks.len() == 2;
    let can_oop_check_raise =
        has_tree_action(&oop_response_to_ip_bet, |action| matches!(action, Action::Raise(_)));
    let can_ip_reraise =
        has_tree_action(&ip_response_to_check_raise, |action| {
            matches!(action, Action::Raise(_) | Action::AllIn(_))
        });

    require_fold_call_raise("OOP response to IP bet", &oop_response_to_ip_bet)?;
    require_fold_call_only("IP response to OOP check-raise", &ip_response_to_check_raise)?;

    println!();
    println!("TREE CONFIG");
    println!("Pot type: SRP");
    println!("Board: {}", scenario.board);
    println!("Branch sizing: {}", sizing.tree_size);
    println!(
        "Starting pot: {} chips ({:.1}BB)",
        STARTING_POT,
        STARTING_POT as f32 / BB_CHIPS
    );
    println!(
        "Effective stack: {} chips ({:.1}BB)",
        STARTING_STACK,
        STARTING_STACK as f32 / BB_CHIPS
    );
    println!("OOP flop sizes: 33%, 75%");
    println!("IP flop sizes after OOP check: {}", sizing.tree_size);
    println!("OOP root actions: {oop_root_actions:?}");
    println!("IP actions after OOP check: {ip_after_check_actions:?}");
    println!("OOP response to IP bet: {oop_response_to_ip_bet:?}");
    println!("IP response to OOP check-raise: {ip_response_to_check_raise:?}");
    println!("IP response to OOP donk: {donk_response_logs:?}");
    println!("OOP response to IP raise: {oop_after_ip_raise_logs:?}");
    println!("Raise sizes: 3x the previous bet");
    println!(
        "All-in thresholds: add <= {:.2} pot, force when post-call SPR <= {:.2}",
        ADD_ALLIN_THRESHOLD,
        FORCE_ALLIN_THRESHOLD
    );
    println!("Can OOP donk flop: {can_oop_donk}");
    println!("Can OOP check-raise flop: {can_oop_check_raise}");
    println!("Can IP raise donk: {can_ip_raise_donk}");
    println!("Can IP re-raise after check-raise: {can_ip_reraise}");
    println!("Turn/river status: TURN/RIVER STILL SIMPLIFIED IN THIS DEBUG TREE");
    println!();

    if !can_oop_donk || !can_oop_check_raise || !can_ip_raise_donk || can_ip_reraise {
        return Err("realistic flop tree audit failed".to_string());
    }
    Ok(())
}

fn find_tree_action<F>(actions: &[Action], predicate: F) -> Result<Action, String>
where
    F: Fn(&Action) -> bool,
{
    actions
        .iter()
        .copied()
        .find(predicate)
        .ok_or_else(|| format!("required action not found in {actions:?}"))
}

fn has_tree_action<F>(actions: &[Action], predicate: F) -> bool
where
    F: Fn(&Action) -> bool,
{
    actions.iter().any(predicate)
}

fn require_fold_call_raise(label: &str, actions: &[Action]) -> Result<(), String> {
    let has_fold = has_tree_action(actions, |action| matches!(action, Action::Fold));
    let has_call = has_tree_action(actions, |action| matches!(action, Action::Call));
    let has_raise = has_tree_action(actions, |action| matches!(action, Action::Raise(_)));
    if has_fold && has_call && has_raise {
        Ok(())
    } else {
        Err(format!("{label} must contain Fold, Call, Raise; got {actions:?}"))
    }
}

fn require_fold_call_only(label: &str, actions: &[Action]) -> Result<(), String> {
    let valid = actions.len() == 2
        && has_tree_action(actions, |action| matches!(action, Action::Fold))
        && has_tree_action(actions, |action| matches!(action, Action::Call));
    if valid {
        Ok(())
    } else {
        Err(format!("{label} must contain only Fold and Call; got {actions:?}"))
    }
}

fn hero_range_combo_counts(
    scenario: &DebugScenario,
) -> Result<(usize, usize), Box<dyn Error>> {
    let range_text = scenario
        .ip_range
        .ok_or_else(|| format!("missing hero range for Spot {}", scenario.spot_id))?;
    let range: Range = normalize_range_for_solver(range_text).parse()?;
    let (all_combos, _) = range.get_hands_weights(0);
    let dead_cards_mask = parse_board(scenario.board)?
        .into_iter()
        .fold(0u64, |mask, card| mask | (1u64 << card));
    let (live_combos, _) = range.get_hands_weights(dead_cards_mask);
    Ok((all_combos.len(), live_combos.len()))
}

fn normalize_range_for_solver(range: &str) -> String {
    range
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(normalize_range_token)
        .collect::<Vec<_>>()
        .join(",")
}

fn normalize_range_token(token: &str) -> String {
    let compact = token
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if let Some((first, second)) = compact.split_once('-') {
        let first = normalize_hand_class(first);
        let second = normalize_hand_class(second);
        return if hand_class_key(&first) >= hand_class_key(&second) {
            format!("{first}-{second}")
        } else {
            format!("{second}-{first}")
        };
    }

    let suffix = compact.strip_suffix('+').map(|_| "+").unwrap_or("");
    let class = compact.strip_suffix('+').unwrap_or(&compact);
    format!("{}{}", normalize_hand_class(class), suffix)
}

fn normalize_hand_class(class: &str) -> String {
    let compact = class
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let bytes = compact.as_bytes();
    if bytes.len() < 2 {
        return compact.to_uppercase();
    }

    let first = (bytes[0] as char).to_ascii_uppercase();
    let second = (bytes[1] as char).to_ascii_uppercase();
    let suffix = compact[2..].to_ascii_lowercase();
    if rank_value(first) >= rank_value(second) {
        return format!("{first}{second}{suffix}");
    }

    format!("{second}{first}{suffix}")
}

fn hand_class_key(class: &str) -> (u8, u8) {
    let bytes = class.as_bytes();
    if bytes.len() < 2 {
        return (0, 0);
    }
    (
        rank_value((bytes[0] as char).to_ascii_uppercase()),
        rank_value((bytes[1] as char).to_ascii_uppercase()),
    )
}

fn rank_value(rank: char) -> u8 {
    match rank {
        '2'..='9' => rank as u8 - b'0',
        'T' => 10,
        'J' => 11,
        'Q' => 12,
        'K' => 13,
        'A' => 14,
        _ => 0,
    }
}

fn export_debug_range_rows(
    writer: &mut csv::Writer<std::fs::File>,
    scenario_id: usize,
    scenario: &DebugScenario,
    solve: &DecisionSolve,
) -> Result<usize, Box<dyn Error>> {
    let board = parse_board(scenario.board)?.to_vec();
    for hand in &solve.hands {
        write_debug_row(writer, scenario_id, scenario, solve, hand, &board)?;
    }
    Ok(solve.hands.len())
}

#[allow(dead_code)]
fn export_debug_hand_row(
    writer: &mut csv::Writer<std::fs::File>,
    scenario: &DebugScenario,
    solve: &DecisionSolve,
) -> Result<(), Box<dyn Error>> {
    let target_hand = (
        parse_card(&scenario.reference_hand[0..2])?,
        parse_card(&scenario.reference_hand[2..4])?,
    );
    let hand = solve
        .hands
        .iter()
        .find(|hand| {
            hand.cards == target_hand
                || hand.cards == (target_hand.1, target_hand.0)
        })
        .ok_or_else(|| {
            format!(
                "hero hand {} is not live/in range for Spot {} on {}",
                scenario.reference_hand, scenario.spot_id, scenario.board
            )
        })?;
    let board = parse_board(scenario.board)?.to_vec();
    write_debug_row(writer, 0, scenario, solve, hand, &board)
}

fn write_debug_row(
    writer: &mut csv::Writer<std::fs::File>,
    scenario_id: usize,
    scenario: &DebugScenario,
    solve: &DecisionSolve,
    hand: &HandCombo,
    board: &[Card],
) -> Result<(), Box<dyn Error>> {
    let values = row_values(solve, hand.index);
    let hero_equity_vs_villain =
        raw_equity_vs_villain(hand.cards, &solve.villains, board);
    let equity_with_draws = solve.hero_equities[hand.index];
    let (blocks_value, blocks_fold) =
        blocker_totals(hand.cards, &solve.villain_weights);
    let raw_strength_score = raw_strength_score(hero_equity_vs_villain);
    let improvability_score =
        improvability_score(equity_with_draws - hero_equity_vs_villain);
    let range_advantage_score =
        range_advantage_score(solve.range_stats.range_equity_hero);
    let nut_advantage_score =
        nut_advantage_score(solve.range_stats.nut_advantage_pct);

    writer.write_record([
        scenario_id.to_string(),
        scenario.spot_id.to_string(),
        format!(
            "{}-vs-{}-srp-spot-{}-debug-range-{}",
            scenario.hero_position.to_lowercase(),
            scenario.villain_position.to_lowercase(),
            scenario.spot_id,
            sizing_suffix(solve)
        ),
        "flop".to_string(),
        hand.label.clone(),
        scenario.board.to_string(),
        scenario.full_board.to_string(),
        scenario.reference_hand.to_string(),
        scenario.hero_position.to_string(),
        scenario.villain_position.to_string(),
        "ip".to_string(),
        "srp".to_string(),
        solve.meta.flop_bet_size.to_string(),
        format_float(values.check_freq),
        format_float(values.check_ev),
        solve.meta.flop_bet_size.to_string(),
        format_float(values.bet_freq),
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
        format_float(solve.range_stats.villain_weighted_fold_combos),
        format_float(blocks_value),
        format_float(blocks_fold),
    ])?;
    Ok(())
}

fn sizing_suffix(solve: &DecisionSolve) -> &'static str {
    match solve.meta.flop_bet_size {
        "0.33" => "33",
        "0.75" => "75",
        "1.25" => "125",
        _ => "unknown",
    }
}

fn write_debug_header(
    writer: &mut csv::Writer<std::fs::File>,
) -> Result<(), Box<dyn Error>> {
    writer.write_record([
        "scenario_id",
        "spot_id",
        "spot_name",
        "street",
        "hand",
        "board",
        "full_board",
        "original_input_hand",
        "hero_position",
        "villain_position",
        "player_type",
        "pot_type",
        "flop_bet_size",
        "check_freq",
        "check_ev",
        "bet_1_size",
        "bet_1_freq",
        "bet_1_ev",
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
        "villain_weighted_fold_combos",
        "hero_blocks_value_combos",
        "hero_blocks_fold_combos",
    ])?;
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

// CODEX_TEST_EXPORT_BB_FLOP_ACTIONS
fn extract_bb_flop_actions(
    game: &mut PostFlopGame,
    scenario: DebugScenario,
    sizing: Sizing,
) -> Result<BbFlopActionExtract, Box<dyn Error>> {
    println!(
        "BB flop action export: board={} spot={} OOP={} IP={} branch={}",
        scenario.board, scenario.spot_id, scenario.villain_position, scenario.hero_position, sizing.tree_size
    );

    game.back_to_root();
    game.cache_normalized_weights();
    let hands = player_hands(game, OOP_PLAYER)?;
    let ip_hands = player_hands(game, IP_PLAYER)?;
    let hand_count = game.private_cards(OOP_PLAYER).len();
    let ip_hand_count = game.private_cards(IP_PLAYER).len();
    let mut extract = BbFlopActionExtract {
        hands,
        ip_hands,
        branch_suffix: sizing.suffix,
        branch_size: sizing.csv_size,
        root_found: game.current_player() == OOP_PLAYER,
        check_found: false,
        donk33_found: false,
        donk75_found: false,
        vs_ip_bet_found: false,
        donk33_vs_raise_found: false,
        donk75_vs_raise_found: false,
        ip_vs_donk_found: false,
        ip_vs_check_raise_found: false,
        check_freq: empty_freqs(hand_count),
        donk33_freq: empty_freqs(hand_count),
        donk75_freq: empty_freqs(hand_count),
        vs_bet_fold_freq: empty_freqs(hand_count),
        vs_bet_call_freq: empty_freqs(hand_count),
        vs_bet_raise_freq: empty_freqs(hand_count),
        donk33_vs_raise_fold_freq: empty_freqs(hand_count),
        donk33_vs_raise_call_freq: empty_freqs(hand_count),
        donk75_vs_raise_fold_freq: empty_freqs(hand_count),
        donk75_vs_raise_call_freq: empty_freqs(hand_count),
        btn_vs_donk_fold_freq: empty_freqs(ip_hand_count),
        btn_vs_donk_call_freq: empty_freqs(ip_hand_count),
        btn_vs_donk_raise_freq: empty_freqs(ip_hand_count),
        btn_vs_check_raise_fold_freq: empty_freqs(ip_hand_count),
        btn_vs_check_raise_call_freq: empty_freqs(ip_hand_count),
    };

    if !extract.root_found {
        println!(
            "BB flop action export: root missing, current_player={}",
            game.current_player()
        );
        return Ok(extract);
    }

    let root_actions = game.available_actions().to_vec();
    println!("BB flop action export: root BB actions = {root_actions:?}");
    let root_strategy = game.strategy().to_vec();
    let check_index = root_actions.iter().position(|action| matches!(action, Action::Check));
    let donk33_index = find_bet_action_for_size(&root_actions, STARTING_POT, 0.33);
    let donk75_index = find_bet_action_for_size(&root_actions, STARTING_POT, 0.75);
    extract.check_found = check_index.is_some();
    extract.donk33_found = donk33_index.is_some();
    extract.donk75_found = donk75_index.is_some();
    fill_action_freqs(&mut extract.check_freq, &root_strategy, check_index, hand_count);
    fill_action_freqs(&mut extract.donk33_freq, &root_strategy, donk33_index, hand_count);
    fill_action_freqs(&mut extract.donk75_freq, &root_strategy, donk75_index, hand_count);
    println!(
        "BB flop action export: root found={}, check_found={}, donk33_found={}, donk75_found={}",
        extract.root_found, extract.check_found, extract.donk33_found, extract.donk75_found
    );

    if let Some(check_index) = check_index {
        game.play(check_index);
        let ip_actions = game.available_actions().to_vec();
        let ip_bet_index = find_bet_action_for_size(&ip_actions, STARTING_POT, sizing_as_decimal(sizing));
        if let Some(ip_bet_index) = ip_bet_index {
            game.play(ip_bet_index);
            game.cache_normalized_weights();
            let response_actions = game.available_actions().to_vec();
            println!("BB flop action export: after check -> IP bet, BB actions = {response_actions:?}");
            let response_strategy = game.strategy().to_vec();
            let fold_index = response_actions.iter().position(|action| matches!(action, Action::Fold));
            let call_index = response_actions.iter().position(|action| matches!(action, Action::Call));
            let raise_index = response_actions.iter().position(|action| matches!(action, Action::Raise(_)));
            extract.vs_ip_bet_found = fold_index.is_some() || call_index.is_some() || raise_index.is_some();
            fill_action_freqs(&mut extract.vs_bet_fold_freq, &response_strategy, fold_index, hand_count);
            fill_action_freqs(&mut extract.vs_bet_call_freq, &response_strategy, call_index, hand_count);
            fill_action_freqs(&mut extract.vs_bet_raise_freq, &response_strategy, raise_index, hand_count);
            if let Some(raise_index) = raise_index {
                game.play(raise_index);
                game.cache_normalized_weights();
                let ip_check_raise_actions = game.available_actions().to_vec();
                println!(
                    "BB flop action export: after check -> IP bet -> BB raise, BTN actions = {ip_check_raise_actions:?}"
                );
                let ip_check_raise_strategy = game.strategy().to_vec();
                let ip_fold_index = ip_check_raise_actions
                    .iter()
                    .position(|action| matches!(action, Action::Fold));
                let ip_call_index = ip_check_raise_actions
                    .iter()
                    .position(|action| matches!(action, Action::Call));
                extract.ip_vs_check_raise_found =
                    ip_fold_index.is_some() || ip_call_index.is_some();
                fill_action_freqs(
                    &mut extract.btn_vs_check_raise_fold_freq,
                    &ip_check_raise_strategy,
                    ip_fold_index,
                    ip_hand_count,
                );
                fill_action_freqs(
                    &mut extract.btn_vs_check_raise_call_freq,
                    &ip_check_raise_strategy,
                    ip_call_index,
                    ip_hand_count,
                );
            }
        } else {
            println!(
                "BB flop action export: IP branch bet {} not found after BB checks; actions={ip_actions:?}",
                sizing.tree_size
            );
        }
        game.back_to_root();
    }

    fill_donk_raise_response(game, donk33_index, "33", hand_count, ip_hand_count, &mut extract)?;
    fill_donk_raise_response(game, donk75_index, "75", hand_count, ip_hand_count, &mut extract)?;
    game.back_to_root();

    println!(
        "BB flop action export: response after check -> BTN bet found={}",
        extract.vs_ip_bet_found
    );
    println!(
        "BB flop action export: response after donk -> BTN raise found 33={} 75={}",
        extract.donk33_vs_raise_found, extract.donk75_vs_raise_found
    );
    println!(
        "BB flop action export: BTN response vs BB donk found={}",
        extract.ip_vs_donk_found
    );
    println!(
        "BB flop action export: BTN response vs BB check-raise found={}",
        extract.ip_vs_check_raise_found
    );
    println!(
        "BB flop action export: using each combo's primary donk line for bb_donk_then_vs_ip_raise_* columns when both donk sizes exist."
    );
    print_bb_action_summaries(&extract);

    Ok(extract)
}

fn empty_freqs(hand_count: usize) -> Vec<Option<f32>> {
    vec![None; hand_count]
}

fn fill_action_freqs(
    target: &mut [Option<f32>],
    strategy: &[f32],
    action_index: Option<usize>,
    hand_count: usize,
) {
    if let Some(action_index) = action_index {
        for hand_index in 0..hand_count {
            target[hand_index] = Some(action_value(strategy, action_index, hand_index, hand_count));
        }
    }
}

fn fill_donk_raise_response(
    game: &mut PostFlopGame,
    donk_index: Option<usize>,
    donk_label: &str,
    hand_count: usize,
    ip_hand_count: usize,
    extract: &mut BbFlopActionExtract,
) -> Result<(), Box<dyn Error>> {
    game.back_to_root();
    let Some(donk_index) = donk_index else {
        println!("BB flop action export: donk {donk_label} node missing at root");
        return Ok(());
    };

    game.play(donk_index);
    let ip_response_actions = game.available_actions().to_vec();
    println!("BB flop action export: after BB donk {donk_label}, BTN actions = {ip_response_actions:?}");
    if donk_label == extract.branch_suffix {
        game.cache_normalized_weights();
        let ip_response_strategy = game.strategy().to_vec();
        let ip_fold_index = ip_response_actions
            .iter()
            .position(|action| matches!(action, Action::Fold));
        let ip_call_index = ip_response_actions
            .iter()
            .position(|action| matches!(action, Action::Call));
        let ip_raise_index = ip_response_actions
            .iter()
            .position(|action| matches!(action, Action::Raise(_)));
        extract.ip_vs_donk_found =
            ip_fold_index.is_some() || ip_call_index.is_some() || ip_raise_index.is_some();
        fill_action_freqs(
            &mut extract.btn_vs_donk_fold_freq,
            &ip_response_strategy,
            ip_fold_index,
            ip_hand_count,
        );
        fill_action_freqs(
            &mut extract.btn_vs_donk_call_freq,
            &ip_response_strategy,
            ip_call_index,
            ip_hand_count,
        );
        fill_action_freqs(
            &mut extract.btn_vs_donk_raise_freq,
            &ip_response_strategy,
            ip_raise_index,
            ip_hand_count,
        );
    }
    let ip_raise_index = ip_response_actions
        .iter()
        .position(|action| matches!(action, Action::Raise(_)));
    let Some(ip_raise_index) = ip_raise_index else {
        println!("BB flop action export: BTN raise missing after BB donk {donk_label}");
        game.back_to_root();
        return Ok(());
    };

    game.play(ip_raise_index);
    game.cache_normalized_weights();
    let oop_response_actions = game.available_actions().to_vec();
    println!("BB flop action export: after BB donk {donk_label} -> BTN raise, BB actions = {oop_response_actions:?}");
    let response_strategy = game.strategy().to_vec();
    let fold_index = oop_response_actions
        .iter()
        .position(|action| matches!(action, Action::Fold));
    let call_index = oop_response_actions
        .iter()
        .position(|action| matches!(action, Action::Call));

    match donk_label {
        "33" => {
            extract.donk33_vs_raise_found = fold_index.is_some() || call_index.is_some();
            fill_action_freqs(
                &mut extract.donk33_vs_raise_fold_freq,
                &response_strategy,
                fold_index,
                hand_count,
            );
            fill_action_freqs(
                &mut extract.donk33_vs_raise_call_freq,
                &response_strategy,
                call_index,
                hand_count,
            );
        }
        "75" => {
            extract.donk75_vs_raise_found = fold_index.is_some() || call_index.is_some();
            fill_action_freqs(
                &mut extract.donk75_vs_raise_fold_freq,
                &response_strategy,
                fold_index,
                hand_count,
            );
            fill_action_freqs(
                &mut extract.donk75_vs_raise_call_freq,
                &response_strategy,
                call_index,
                hand_count,
            );
        }
        _ => {}
    }

    game.back_to_root();
    Ok(())
}

fn find_bet_action_for_size(actions: &[Action], pot: i32, size: f64) -> Option<usize> {
    let expected = pot as f64 * size;
    actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| match action {
            Action::Bet(amount) => Some((index, ((*amount as f64) - expected).abs())),
            _ => None,
        })
        .filter(|(_, diff)| *diff <= 2.0)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, _)| index)
}

fn sizing_as_decimal(sizing: Sizing) -> f64 {
    sizing.csv_size.parse::<f64>().unwrap_or(0.0)
}

fn write_bb_flop_actions_header(writer: &mut csv::Writer<std::fs::File>) -> Result<(), Box<dyn Error>> {
    writer.write_record([
        "row_type",
        "scenario_id",
        "spot_id",
        "spot_name",
        "board",
        "full_board",
        "oop_position",
        "ip_position",
        "flop_bet_size_branch",
        "bb_hand",
        "bb_combo",
        "bb_original_check_freq",
        "bb_original_donk_33_freq",
        "bb_original_donk_75_freq",
        "bb_vs_ip_bet_fold_freq",
        "bb_vs_ip_bet_call_freq",
        "bb_vs_ip_bet_raise_freq",
        "bb_donk_then_vs_ip_raise_fold_freq",
        "bb_donk_then_vs_ip_raise_call_freq",
        "bb_primary_original_action",
        "bb_primary_vs_ip_bet_response",
        "bb_primary_donk_vs_raise_response",
        "btn_hand",
        "btn_combo",
        "btn_vs_bb_donk_fold_freq",
        "btn_vs_bb_donk_call_freq",
        "btn_vs_bb_donk_raise_freq",
        "btn_vs_bb_check_raise_fold_freq",
        "btn_vs_bb_check_raise_call_freq",
        "btn_primary_vs_bb_donk_response",
        "btn_primary_vs_bb_check_raise_response",
        "source_node_status",
    ])?;
    Ok(())
}

fn export_bb_flop_actions_rows(
    writer: &mut csv::Writer<std::fs::File>,
    scenario_id: u32,
    scenario: &DebugScenario,
    extract: &BbFlopActionExtract,
) -> Result<ActionExportCounts, Box<dyn Error>> {
    let mut counts = ActionExportCounts {
        total_rows: 0,
        bb_rows: 0,
        btn_rows: 0,
    };
    for hand in &extract.hands {
        let i = hand.index;
        let (donk_raise_fold, donk_raise_call, primary_donk_status) = primary_donk_raise_response(extract, i);
        let primary_original = classify_primary(
            &[
                ("CHECK", extract.check_freq[i]),
                ("DONK_33", extract.donk33_freq[i]),
                ("DONK_75", extract.donk75_freq[i]),
            ],
            "UNAVAILABLE",
        );
        let primary_vs_ip_bet = classify_primary(
            &[
                ("CHECK_CALL", extract.vs_bet_call_freq[i]),
                ("CHECK_FOLD", extract.vs_bet_fold_freq[i]),
                ("CHECK_RAISE", extract.vs_bet_raise_freq[i]),
            ],
            "UNAVAILABLE",
        );
        let primary_donk_vs_raise = classify_primary(
            &[
                ("DONK_CALL_VS_RAISE", donk_raise_call),
                ("DONK_FOLD_VS_RAISE", donk_raise_fold),
            ],
            "UNAVAILABLE",
        );
        writer.write_record([
            "BB_HAND_ACTIONS".to_string(),
            scenario_id.to_string(),
            scenario.spot_id.to_string(),
            debug_spot_name(scenario, extract.branch_suffix),
            scenario.board.to_string(),
            scenario.full_board.to_string(),
            scenario.villain_position.to_string(),
            scenario.hero_position.to_string(),
            extract.branch_size.to_string(),
            hand_class_label(hand.cards),
            hand.label.clone(),
            format_optional_freq(extract.check_freq[i]),
            format_optional_freq(extract.donk33_freq[i]),
            format_optional_freq(extract.donk75_freq[i]),
            format_optional_freq(extract.vs_bet_fold_freq[i]),
            format_optional_freq(extract.vs_bet_call_freq[i]),
            format_optional_freq(extract.vs_bet_raise_freq[i]),
            format_optional_freq(donk_raise_fold),
            format_optional_freq(donk_raise_call),
            primary_original,
            primary_vs_ip_bet,
            primary_donk_vs_raise,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            source_node_status(extract, &primary_donk_status),
        ])?;
        counts.total_rows += 1;
        counts.bb_rows += 1;
    }

    for hand in &extract.ip_hands {
        let i = hand.index;
        let primary_vs_donk = classify_primary(
            &[
                ("CALL_VS_DONK", extract.btn_vs_donk_call_freq[i]),
                ("FOLD_VS_DONK", extract.btn_vs_donk_fold_freq[i]),
                ("RAISE_VS_DONK", extract.btn_vs_donk_raise_freq[i]),
            ],
            "UNAVAILABLE",
        );
        let primary_vs_check_raise = classify_primary(
            &[
                ("CALL_VS_CHECK_RAISE", extract.btn_vs_check_raise_call_freq[i]),
                ("FOLD_VS_CHECK_RAISE", extract.btn_vs_check_raise_fold_freq[i]),
            ],
            "UNAVAILABLE",
        );
        writer.write_record([
            "BTN_HAND_ACTIONS".to_string(),
            scenario_id.to_string(),
            scenario.spot_id.to_string(),
            debug_spot_name(scenario, extract.branch_suffix),
            scenario.board.to_string(),
            scenario.full_board.to_string(),
            scenario.villain_position.to_string(),
            scenario.hero_position.to_string(),
            extract.branch_size.to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            hand_class_label(hand.cards),
            hand.label.clone(),
            format_optional_freq(extract.btn_vs_donk_fold_freq[i]),
            format_optional_freq(extract.btn_vs_donk_call_freq[i]),
            format_optional_freq(extract.btn_vs_donk_raise_freq[i]),
            format_optional_freq(extract.btn_vs_check_raise_fold_freq[i]),
            format_optional_freq(extract.btn_vs_check_raise_call_freq[i]),
            primary_vs_donk,
            primary_vs_check_raise,
            btn_source_node_status(extract),
        ])?;
        counts.total_rows += 1;
        counts.btn_rows += 1;
    }
    println!(
        "BB flop action export: branch {} exported {} BB rows and {} BTN rows",
        extract.branch_size, counts.bb_rows, counts.btn_rows
    );
    Ok(counts)
}

fn debug_spot_name(scenario: &DebugScenario, suffix: &str) -> String {
    format!(
        "{}-vs-{}-srp-{:02}-flop-bb-actions-{}",
        scenario.hero_position.to_lowercase(),
        scenario.villain_position.to_lowercase(),
        scenario.spot_id,
        suffix
    )
}

fn primary_donk_raise_response(
    extract: &BbFlopActionExtract,
    hand_index: usize,
) -> (Option<f32>, Option<f32>, String) {
    let donk33 = extract.donk33_freq[hand_index].unwrap_or(-1.0);
    let donk75 = extract.donk75_freq[hand_index].unwrap_or(-1.0);
    if donk33 < 0.0 && donk75 < 0.0 {
        return (None, None, "DONK_NODE_MISSING".to_string());
    }
    if donk75 > donk33 {
        (
            extract.donk75_vs_raise_fold_freq[hand_index],
            extract.donk75_vs_raise_call_freq[hand_index],
            if extract.donk75_vs_raise_found {
                "VS_RAISE_FOUND_PRIMARY_DONK_75"
            } else {
                "RAISE_RESPONSE_MISSING"
            }
            .to_string(),
        )
    } else {
        (
            extract.donk33_vs_raise_fold_freq[hand_index],
            extract.donk33_vs_raise_call_freq[hand_index],
            if extract.donk33_vs_raise_found {
                "VS_RAISE_FOUND_PRIMARY_DONK_33"
            } else {
                "RAISE_RESPONSE_MISSING"
            }
            .to_string(),
        )
    }
}

fn classify_primary(options: &[(&str, Option<f32>)], unavailable: &str) -> String {
    options
        .iter()
        .filter_map(|(label, value)| value.map(|value| (*label, value)))
        .fold(None, |best: Option<(&str, f32)>, current| match best {
            Some((_, best_value)) if current.1 <= best_value => best,
            _ => Some(current),
        })
        .map(|(label, _)| label.to_string())
        .unwrap_or_else(|| unavailable.to_string())
}

fn source_node_status(extract: &BbFlopActionExtract, primary_donk_status: &str) -> String {
    if !extract.root_found {
        return "UNAVAILABLE".to_string();
    }
    let mut parts = Vec::new();
    parts.push("ROOT_FOUND");
    parts.push(if extract.check_found { "CHECK_FOUND" } else { "CHECK_MISSING" });
    parts.push(if extract.donk33_found || extract.donk75_found {
        "DONK_FOUND"
    } else {
        "DONK_NODE_MISSING"
    });
    parts.push(if extract.vs_ip_bet_found {
        "VS_BET_FOUND"
    } else {
        "VS_BET_MISSING"
    });
    parts.push(primary_donk_status);
    parts.join("_")
}

fn btn_source_node_status(extract: &BbFlopActionExtract) -> String {
    if !extract.root_found {
        return "UNAVAILABLE".to_string();
    }
    let mut parts = Vec::new();
    parts.push("ROOT_FOUND");
    parts.push(if extract.ip_vs_donk_found {
        "BTN_VS_DONK_FOUND"
    } else {
        "BTN_VS_DONK_MISSING"
    });
    parts.push(if extract.ip_vs_check_raise_found {
        "BTN_VS_CHECK_RAISE_FOUND"
    } else {
        "BTN_VS_CHECK_RAISE_MISSING"
    });
    parts.join("_")
}

fn print_bb_action_summaries(extract: &BbFlopActionExtract) {
    let mut original_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut vs_bet_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut donk_raise_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut btn_vs_donk_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut btn_vs_check_raise_counts = std::collections::BTreeMap::<String, usize>::new();
    for hand in &extract.hands {
        let i = hand.index;
        *original_counts
            .entry(classify_primary(
                &[
                    ("CHECK", extract.check_freq[i]),
                    ("DONK_33", extract.donk33_freq[i]),
                    ("DONK_75", extract.donk75_freq[i]),
                ],
                "UNAVAILABLE",
            ))
            .or_insert(0) += 1;
        *vs_bet_counts
            .entry(classify_primary(
                &[
                    ("CHECK_CALL", extract.vs_bet_call_freq[i]),
                    ("CHECK_FOLD", extract.vs_bet_fold_freq[i]),
                    ("CHECK_RAISE", extract.vs_bet_raise_freq[i]),
                ],
                "UNAVAILABLE",
            ))
            .or_insert(0) += 1;
        let (fold, call, _) = primary_donk_raise_response(extract, i);
        *donk_raise_counts
            .entry(classify_primary(
                &[("DONK_CALL_VS_RAISE", call), ("DONK_FOLD_VS_RAISE", fold)],
                "UNAVAILABLE",
            ))
            .or_insert(0) += 1;
    }
    for hand in &extract.ip_hands {
        let i = hand.index;
        *btn_vs_donk_counts
            .entry(classify_primary(
                &[
                    ("CALL_VS_DONK", extract.btn_vs_donk_call_freq[i]),
                    ("FOLD_VS_DONK", extract.btn_vs_donk_fold_freq[i]),
                    ("RAISE_VS_DONK", extract.btn_vs_donk_raise_freq[i]),
                ],
                "UNAVAILABLE",
            ))
            .or_insert(0) += 1;
        *btn_vs_check_raise_counts
            .entry(classify_primary(
                &[
                    ("CALL_VS_CHECK_RAISE", extract.btn_vs_check_raise_call_freq[i]),
                    ("FOLD_VS_CHECK_RAISE", extract.btn_vs_check_raise_fold_freq[i]),
                ],
                "UNAVAILABLE",
            ))
            .or_insert(0) += 1;
    }
    println!("BB primary original action counts: {original_counts:?}");
    println!("BB primary vs IP bet response counts: {vs_bet_counts:?}");
    println!("BB primary donk-vs-raise response counts: {donk_raise_counts:?}");
    println!("BTN primary vs BB donk response counts: {btn_vs_donk_counts:?}");
    println!("BTN primary vs BB check-raise response counts: {btn_vs_check_raise_counts:?}");
}

fn format_optional_freq(value: Option<f32>) -> String {
    value.map(format_float).unwrap_or_default()
}

fn hand_class_label(cards: (Card, Card)) -> String {
    let r1 = rank(cards.0);
    let r2 = rank(cards.1);
    if r1 == r2 {
        return format!("{}{}", rank_char(r1), rank_char(r2));
    }
    let suited = suit(cards.0) == suit(cards.1);
    let (high, low) = if r1 >= r2 { (r1, r2) } else { (r2, r1) };
    format!("{}{}{}", rank_char(high), rank_char(low), if suited { "s" } else { "o" })
}

fn rank_char(rank: u8) -> char {
    match rank {
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
    }
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
    // Nut advantage uses strict thresholds: 80-90% = 0.5, 90%+ = 1.0.
    let hero_weighted_value_combos = hero_equities
        .iter()
        .take(hero_count)
        .map(|&equity| nut_weight_from_equity(equity))
        .sum();
    let villain_weighted_value_combos = villain_equities
        .iter()
        .take(villain_count)
        .map(|&equity| nut_weight_from_equity(equity))
        .sum();
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
            // Blockers to value intentionally uses broader thresholds:
            // 70-85% = 0.5, 85%+ = 1.0.
            value_weight: blocker_value_weight_from_equity(equities[combo.index]),
            fold_weight: fold_weight(equities[combo.index]),
        })
        .collect()
}

fn nut_weight_from_equity(equity: f32) -> f32 {
    if equity >= 0.90 {
        1.0
    } else if equity >= 0.80 {
        0.5
    } else {
        0.0
    }
}

fn blocker_value_weight_from_equity(equity: f32) -> f32 {
    if equity >= 0.85 {
        1.0
    } else if equity >= 0.70 {
        0.5
    } else {
        0.0
    }
}

fn raw_strength_score(equity: f32) -> u8 {
    score_from_thresholds(
        equity * 100.0,
        &[
            (92.0, 10),
            (83.0, 9),
            (73.0, 8),
            (62.0, 7),
            (50.0, 6),
            (38.0, 5),
            (27.0, 4),
            (17.0, 3),
            (8.0, 2),
        ],
    )
}

fn improvability_score(delta: f32) -> u8 {
    score_from_thresholds(
        delta * 100.0,
        &[
            (32.0, 10),
            (26.0, 9),
            (20.0, 8),
            (15.0, 7),
            (10.0, 6),
            (5.0, 5),
            (0.0, 4),
            (-4.0, 3),
            (-10.0, 2),
        ],
    )
}

fn range_advantage_score(range_equity: f32) -> u8 {
    score_from_thresholds(
        range_equity * 100.0,
        &[
            (68.0, 10),
            (63.0, 9),
            (58.0, 8),
            (54.0, 7),
            (50.0, 6),
            (46.0, 5),
            (42.0, 4),
            (37.0, 3),
            (32.0, 2),
        ],
    )
}

fn nut_advantage_score(nut_advantage: f32) -> u8 {
    score_from_thresholds(
        nut_advantage * 100.0,
        &[
            (8.0, 10),
            (5.0, 9),
            (3.0, 8),
            (1.5, 7),
            (0.0, 6),
            (-1.5, 5),
            (-3.0, 4),
            (-5.0, 3),
            (-8.0, 2),
        ],
    )
}

fn score_from_thresholds(value: f32, thresholds: &[(f32, u8)]) -> u8 {
    for &(minimum, score) in thresholds {
        if value >= minimum {
            return score;
        }
    }
    1
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
