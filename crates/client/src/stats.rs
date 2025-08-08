use core::mem;
use core::time::Duration;

use protocol::ids::IdSet;

/// Efficiently collected processing statistics.
#[derive(Default)]
pub struct Stats {
    pub no_output_buffer: usize,
    pub no_input_buffer: usize,
    pub non_ready: usize,
    pub non_ready_set: IdSet,
    pub not_self_triggered: usize,
    pub signal_error: usize,
    pub signal_error_set: IdSet,
    pub signal_ok: usize,
    pub signal_ok_set: IdSet,
    pub timing_sum: u64,
    pub timing_count: usize,
}

impl Stats {
    /// Merge this statistics with another.
    pub fn merge(&mut self, other: &mut Self) {
        self.no_output_buffer += mem::take(&mut other.no_output_buffer);
        self.no_input_buffer += mem::take(&mut other.no_input_buffer);
        self.non_ready += mem::take(&mut other.non_ready);
        self.non_ready_set |= mem::take(&mut other.non_ready_set);
        self.not_self_triggered += mem::take(&mut other.not_self_triggered);
        self.signal_error += mem::take(&mut other.signal_error);
        self.signal_error_set |= mem::take(&mut other.signal_error_set);
        self.signal_ok += mem::take(&mut other.signal_ok);
        self.signal_ok_set |= mem::take(&mut other.signal_ok_set);
        self.timing_sum += mem::take(&mut other.timing_sum);
        self.timing_count += mem::take(&mut other.timing_count);
    }

    /// Report statistics to the tracing logger.
    pub fn report(&mut self) {
        if self.non_ready > 0 {
            tracing::warn!(self.non_ready, ?self.non_ready_set);
            self.non_ready = 0;
            self.non_ready_set.clear();
        }

        if self.signal_error > 0 || self.signal_ok > 0 {
            tracing::warn!(self.signal_error, self.signal_ok, ?self.signal_error_set, ?self.signal_ok_set);
            self.signal_error = 0;
            self.signal_error_set.clear();
            self.signal_ok = 0;
            self.signal_ok_set.clear();
        }

        if self.no_input_buffer > 0 {
            tracing::warn!(self.no_input_buffer);
            self.no_input_buffer = 0;
        }

        if self.not_self_triggered > 0 {
            tracing::warn!(self.not_self_triggered);
            self.not_self_triggered = 0;
        }

        if self.no_output_buffer > 0 {
            tracing::warn!(self.no_output_buffer);
            self.no_output_buffer = 0;
        }

        if self.timing_count > 0 {
            let average_timing =
                Duration::from_nanos((self.timing_sum as f64 / self.timing_count as f64) as u64);
            tracing::warn!(self.timing_count, self.timing_sum, ?average_timing);
            self.timing_count = 0;
            self.timing_sum = 0;
        }
    }
}
