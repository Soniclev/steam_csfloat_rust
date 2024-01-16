// steam_analyzer.rs

use std::ops::Add;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    consts::PERCENTILES,
    prices::{PriceValue, PriceValueTrait},
};

const MEDIAN_LOWER_LIMIT_COEF: f64 = 0.9;
const MEDIAN_UPPER_LIMIT_COEF: f64 = 1.1;
const REL_STD_MAX: f64 = 0.03;

#[derive(Deserialize)]
struct Point {
    date: String,
    avg_price: f64,
    amount: String,
}

pub fn steam_date_str_to_datetime(s: &str) -> DateTime<Utc> {
    let s = s.split(':').next().unwrap();
    // only seconds can be unfilled in chrono parser....💩😮
    // add fake minutes to the string💀
    let performed = String::from(s).add(" 00");
    let naive = NaiveDateTime::parse_from_str(&performed, "%b %d %Y %H %M")
        .expect("Failed to parse date string");
    DateTime::from_naive_utc_and_offset(naive, Utc)
}

lazy_static! {
    static ref SELL_HISTORY_REGEX: Regex = Regex::new(r#"\s+var line1=([^;]+);"#).unwrap();
}

pub fn extract_sell_history(
    response: &str,
    parse_until: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, f64, i32)> {
    if let Some(caps) = SELL_HISTORY_REGEX.captures(response) {
        if let Ok(encoded_data) = caps[1].parse::<String>() {
            if let Ok(j) = serde_json::from_str::<Vec<Point>>(&encoded_data) {
                let mut result: Vec<(DateTime<Utc>, f64, i32)> = Vec::new();
                result.reserve_exact(7 * 24); // points for each hour

                for point in j.into_iter().rev() {
                    let date = steam_date_str_to_datetime(&point.date);
                    if date < parse_until {
                        break;
                    }
                    let avg_price = point.avg_price;
                    let amount = point.amount.parse::<i32>().unwrap();
                    result.push((date, avg_price, amount));
                }

                result.reverse();
                return result;
            }
        }
    }
    Vec::new()
}

pub fn analyze_steam_sell_history(
    response: &str,
    current_datetime: DateTime<Utc>,
) -> Option<AnalysisResult> {
    let days = 7;
    let date_range_start = current_datetime - Duration::days(days);
    let history_data = extract_sell_history(response, date_range_start);
    let filtered_data: Vec<_> = history_data
        .into_iter()
        .filter(|&(date, _, _)| date_range_start <= date && date <= current_datetime)
        .collect();

    let mut prices: Vec<f64> = filtered_data
        .iter()
        .map(|x| (x.1 * 100.0).round() / 100.0)
        .collect();
    if prices.len() < 5 {
        return None;
    }

    prices.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = prices.len() / 2;
    let median = {
        if prices.len() % 2 == 0 {
            (prices[mid - 1] + prices[mid]) / 2.0
        } else {
            prices[mid]
        }
    };
    let upper_limit = median * MEDIAN_UPPER_LIMIT_COEF;
    let lower_limit = median * MEDIAN_LOWER_LIMIT_COEF;

    let sold_per_week = filtered_data.iter().map(|x| x.2).sum::<i32>();

    let mut prices: Vec<_> = filtered_data
        .into_iter()
        .map(|x| x.1)
        .filter(|&p| lower_limit <= p && p <= upper_limit)
        .collect();

    let sma = simple_moving_average(&prices, 3);
    if sma.is_empty() {
        return Some(AnalysisResult {
            rsd: None,
            is_stable: None,
            sold_per_week: None,
            percentiles: vec![],
            percentiles_no_fee: vec![],
        });
    }
    let sma_mean = mean(&sma).unwrap();
    let sma_std = std_deviation(&sma, sma_mean).unwrap();
    let sma_rel_std = sma_std / sma_mean;

    let is_stable = sma_rel_std < REL_STD_MAX;
    prices.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

    let percentiles: Vec<(u8, PriceValue)> = PERCENTILES
        .iter()
        .map(|(percentile_value, percentile)| {
            let price_value =
                PriceValue::from_usd_f64(calculate_percentile(&prices, *percentile).unwrap());
            (*percentile_value, price_value)
        })
        .collect();

    Some(AnalysisResult {
        rsd: Some(sma_rel_std),
        is_stable: Some(is_stable),
        sold_per_week: Some(sold_per_week),
        percentiles,
        percentiles_no_fee: vec![],
    })
}

fn calculate_percentile(data: &[f64], percentile: f64) -> Option<f64> {
    // Step 1: Calculate the index
    let n = data.len() as f64;
    let index = percentile * (n - 1.0);

    // Step 2: Interpolate if necessary
    let lower_index = index.floor() as usize;
    let upper_index = index.ceil() as usize;

    if lower_index == upper_index {
        // If the index is an integer, return the value at that index
        data.get(lower_index).cloned()
    } else {
        // Interpolate between values at lower and upper indices
        let lower_value = data[lower_index];
        let upper_value = data[upper_index];
        let fraction = index.fract();

        // Linear interpolation formula
        Some((1.0 - fraction) * lower_value + fraction * upper_value)
    }
}

// https://github.com/chinanf-boy/rust-cookbook-zh/blob/master/src/science/mathematics/statistics/standard-deviation.md
fn std_deviation(data: &[f64], mean: f64) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    let variance = data
        .iter()
        .map(|&value| {
            let diff = mean - value;

            diff * diff
        })
        .sum::<f64>()
        / data.len() as f64;

    Some(variance.sqrt())
}

fn mean(data: &[f64]) -> Option<f64> {
    let sum = data.iter().sum::<f64>();
    let count = data.len() as f64;

    match count {
        positive if positive > 0.0 => Some(sum / count),
        _ => None,
    }
}

pub fn simple_moving_average(array_prices: &[f64], window: u32) -> Vec<f64> {
    let interval = window as usize;
    let mut index = interval - 1;
    let length = array_prices.len();

    let mut results = Vec::new();

    while index < length {
        index += 1;

        let start_index = index - interval;
        let interval_slice = &array_prices[start_index..index];
        let sum: f64 = interval_slice.iter().sum();
        let interval_float = interval as f64;
        results.push(sum / interval_float);
    }

    results
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub rsd: Option<f64>,
    pub is_stable: Option<bool>,
    pub sold_per_week: Option<i32>,
    pub percentiles: Vec<(u8, PriceValue)>,
    pub percentiles_no_fee: Vec<(u8, PriceValue)>,
}

impl AnalysisResult {
    pub fn get_price_by_percentile(&self, desired_percentile: u8) -> Option<PriceValue> {
        if !self.percentiles.is_empty() {
            for &(percentile, price) in self.percentiles.iter() {
                if percentile == desired_percentile {
                    return Some(price as PriceValue);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_price_by_percentile_with_existing_percentile() {
        // Create an AnalysisResult instance with some percentiles
        let analysis_result = AnalysisResult {
            rsd: Some(0.5),
            is_stable: Some(true),
            sold_per_week: Some(10),
            percentiles: vec![(25, 10), (50, 20), (75, 30)],
            percentiles_no_fee: vec![],
        };

        // Test for an existing percentile (50th percentile)
        let desired_percentile = 50;
        let expected_price = 20;
        assert_eq!(
            analysis_result.get_price_by_percentile(desired_percentile),
            Some(expected_price)
        );
    }

    #[test]
    fn test_get_price_by_percentile_with_non_existing_percentile() {
        // Create an AnalysisResult instance with some percentiles
        let analysis_result = AnalysisResult {
            rsd: Some(0.5),
            is_stable: Some(true),
            sold_per_week: Some(10),
            percentiles: vec![(25, 10), (50, 20), (75, 30)],
            percentiles_no_fee: vec![],
        };

        // Test for a non-existing percentile (80th percentile)
        let desired_percentile = 80;
        assert_eq!(
            analysis_result.get_price_by_percentile(desired_percentile),
            None
        );
    }

    #[test]
    fn test_get_price_by_percentile_with_empty_percentiles() {
        // Create an AnalysisResult instance with empty percentiles
        let analysis_result = AnalysisResult {
            rsd: Some(0.5),
            is_stable: Some(true),
            sold_per_week: Some(10),
            percentiles: vec![],
            percentiles_no_fee: vec![],
        };

        // Test for any percentile on an empty set
        let desired_percentile = 50;
        assert_eq!(
            analysis_result.get_price_by_percentile(desired_percentile),
            None
        );
    }

    #[test]
    fn test_mean_with_positive_values() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let expected_mean = 3.0;
        assert_eq!(mean(&data), Some(expected_mean));
    }

    #[test]
    fn test_mean_with_zero_values() {
        let data = vec![];
        assert_eq!(mean(&data), None);
    }

    #[test]
    fn test_mean_with_single_value() {
        let data = vec![42.0];
        assert_eq!(mean(&data), Some(42.0));
    }

    #[test]
    fn test_mean_with_negative_values() {
        let data = vec![-1.0, -2.0, -3.0];
        let expected_mean = -2.0;
        assert_eq!(mean(&data), Some(expected_mean));
    }

    #[test]
    fn test_calculate_percentile_with_existing_percentile() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let percentile = 0.5; // 50th percentile
        let expected_value = 30.0;
        assert_eq!(
            calculate_percentile(&data, percentile),
            Some(expected_value)
        );
    }

    #[test]
    fn test_calculate_percentile_with_non_existing_percentile() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let percentile = 0.8; // 75th percentile
        assert_eq!(calculate_percentile(&data, percentile), Some(42.0));
    }

    #[test]
    fn test_calculate_percentile_with_empty_data() {
        let data = vec![];
        let percentile = 0.5; // 50th percentile
        assert_eq!(calculate_percentile(&data, percentile), None);
    }

    #[test]
    fn test_calculate_percentile_with_single_value() {
        let data = vec![42.0];
        let percentile = 0.5; // 50th percentile
        assert_eq!(calculate_percentile(&data, percentile), Some(42.0));
    }

    #[test]
    fn test_calculate_percentile_with_fractional_index() {
        let data = vec![10.0, 20.0, 30.0, 40.0];
        let percentile = 0.6; // 60th percentile
        let expected_value = 28.0; // Interpolated value between 20 and 30
        assert!(calculate_percentile(&data, percentile).unwrap() - expected_value < f64::EPSILON);
    }
}
