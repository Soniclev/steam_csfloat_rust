use teloxide::types::ChatId;

use crate::prices::PriceValue;

// Save state of engines once per minute.
// The probability that the program cannot process new events in time as an event arrives is calculated as follows:
// Let:
//   T_avg = Average time taken by the main thread to save state (750 ms)
//   I = Interval between state saves
//
// The probability P can be calculated using the formula:
// P = T_avg / I
//
// Calculations:
// T_avg = (500 ms + 1000 ms) / 2 = 750 ms
//
// For I = 60 seconds:
// P = 750 ms / 60 seconds
// P ≈ 0.0125
// So, the probability that the program cannot process new events in time as an event arrives is approximately 1.25%.
//
// For I = 5 minutes (300 seconds):
// P = 750 ms / 300 seconds
// P ≈ 0.0025
// So, the probability is approximately 0.25%.
//
// For I = 10 minutes (600 seconds):
// P = 750 ms / 600 seconds
// P ≈ 0.00125
// So, the probability is approximately 0.125%.
//
// For I = 20 minutes (1200 seconds):
// P = 750 ms / 1200 seconds
// P ≈ 0.000625
// So, the probability is approximately 0.0625%.
pub const DB_SAVE_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_secs(60);

// csfloat.com allows 50,000 requests from one IP on a daily basis.
// To avoid hitting the daily limit, we set a conservative interval of 3 seconds between requests.
// Calculations:
// Total number of seconds in a day: 86,400 seconds
// Maximum number of requests allowed: 50,000
// Interval between requests to stay within limit: 86,400 seconds / 50,000 requests ≈ 1.728 seconds
// Chosen interval to avoid daily limits: 3 seconds
pub const CSFLOAT_ONE_LISTING_REQ_INTERVAL: std::time::Duration =
    tokio::time::Duration::from_secs(3);

// my Telegram ID
// removed
pub const MY_TG_ID: ChatId = ChatId(0);

pub const TG_NOTIFY_MIN_PROFIT_PCT: f64 = 30.0;

pub const PERCENTILES: [(u8, f64); 5] =
    [(60, 0.60), (65, 0.65), (70, 0.70), (75, 0.75), (80, 0.80)];
pub const DESIRED_PERCENTILE: u8 = 60;

#[allow(dead_code)]
pub const PHASE_1: &str = "Phase 1";
#[allow(dead_code)]
pub const PHASE_2: &str = "Phase 2";
#[allow(dead_code)]
pub const PHASE_3: &str = "Phase 3";
#[allow(dead_code)]
pub const PHASE_4: &str = "Phase 4";

pub const LISTING_MIN_PRICE: PriceValue = 50 as PriceValue; // $0.5
pub const LISTING_MAX_PRICE: PriceValue = 75_00 as PriceValue; // $75

pub const MIN_SOLD_PER_WEEK: u64 = 50;
pub const IS_AUTOBUY_ALLOWED: bool = false;
pub const AUTOBUY_FROM_PROFIT_PCT: f64 = 45.0;

/* in Rust it's allowed to create "const" functions
pub const fn ...() {

}
 */
