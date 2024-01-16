use std::collections::HashMap;

use circular_buffer::CircularBuffer;
use std::fmt::Write;
use std::time::Duration;
use tracing::info;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum StatsKind {
    CsfloatOneListingResponse,
    CsfloatListingsResponse,
    SteamResponse,
    UpdatedCsfloatListings,
    ProfitableListing,
}

const STATS_SIZE: usize = 1_000;

pub struct Stats {
    hm: HashMap<StatsKind, CircularBuffer<STATS_SIZE, Duration>>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats { hm: HashMap::new() }
    }
    pub fn register_duration(&mut self, kind: StatsKind, duration: Duration) {
        let entry = self.hm.entry(kind).or_default();
        entry.push_back(duration)
    }

    pub fn print(&self) {
        const PERCENTILES: [u32; 4] = [50, 90, 95, 99];

        // Create a buffer to accumulate log messages
        let mut buffer = String::new();

        for (kind, durations) in &self.hm {
            writeln!(
                buffer,
                "Stats for {:?} ({} records):",
                kind,
                durations.len()
            )
            .unwrap();

            if !durations.is_empty() {
                let mean = durations.iter().sum::<Duration>() / durations.len() as u32;
                writeln!(
                    buffer,
                    "  Mean: {:?} ({}/s)",
                    mean,
                    self.calculate_rate(mean)
                )
                .unwrap();
                for &percentile in PERCENTILES.iter() {
                    let percentile_value = self.get_percentile(durations, percentile);
                    let rate = self.calculate_rate(percentile_value);

                    writeln!(
                        buffer,
                        "  {}th Percentile: {:?} (rate {}/s)",
                        percentile, percentile_value, rate
                    )
                    .unwrap();
                }
            } else {
                writeln!(buffer, "  No records available.").unwrap();
            }
        }

        // Print all accumulated log messages at once
        info!("{}", buffer);
    }

    fn calculate_rate(&self, duration: Duration) -> u32 {
        if duration.as_nanos() > 0 {
            1_000_000_000 / duration.as_nanos() as u32
        } else {
            0
        }
    }

    fn get_percentile(
        &self,
        durations: &CircularBuffer<STATS_SIZE, Duration>,
        percentile: u32,
    ) -> Duration {
        let mut sorted_times: Vec<_> = durations.iter().collect();
        sorted_times.sort();

        let index = ((percentile as f64 / 100.0) * sorted_times.len() as f64) as usize;
        *sorted_times[index]
    }
}
