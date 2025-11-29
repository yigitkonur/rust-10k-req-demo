//! Statistics tracking for request processing.
//!
//! This module provides real-time tracking of request statistics
//! including success/failure counts, latency, and throughput.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Statistics tracker for request processing.
#[derive(Debug)]
pub struct StatsTracker {
    /// Start time of processing.
    start_time: Instant,
    /// Total requests processed.
    total_processed: AtomicU64,
    /// Successful requests.
    success_count: AtomicU64,
    /// Failed requests.
    failure_count: AtomicU64,
    /// Total latency in microseconds.
    total_latency_us: AtomicU64,
    /// Requests in the last second (for RPS calculation).
    recent_requests: Mutex<VecDeque<Instant>>,
    /// Total input lines.
    total_lines: AtomicUsize,
}

impl StatsTracker {
    /// Create a new statistics tracker.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_processed: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            recent_requests: Mutex::new(VecDeque::new()),
            total_lines: AtomicUsize::new(0),
        }
    }

    /// Set the total number of input lines.
    pub fn set_total_lines(&self, total: usize) {
        self.total_lines.store(total, Ordering::Relaxed);
    }

    /// Record a successful request.
    pub fn record_success(&self, latency: Duration) {
        self.total_processed.fetch_add(1, Ordering::Relaxed);
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
        self.record_recent();
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        self.total_processed.fetch_add(1, Ordering::Relaxed);
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        self.record_recent();
    }

    /// Record a request for RPS calculation.
    fn record_recent(&self) {
        let now = Instant::now();
        let mut recent = self.recent_requests.lock();
        recent.push_back(now);

        // Remove entries older than 1 second
        let cutoff = now - Duration::from_secs(1);
        while let Some(front) = recent.front() {
            if *front < cutoff {
                recent.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get the current requests per second.
    pub fn requests_per_second(&self) -> f64 {
        let now = Instant::now();
        let mut recent = self.recent_requests.lock();

        // Remove old entries
        let cutoff = now - Duration::from_secs(1);
        while let Some(front) = recent.front() {
            if *front < cutoff {
                recent.pop_front();
            } else {
                break;
            }
        }

        recent.len() as f64
    }

    /// Get the current statistics snapshot.
    pub fn snapshot(&self) -> StatsSnapshot {
        let elapsed = self.start_time.elapsed();
        let total = self.total_processed.load(Ordering::Relaxed);
        let success = self.success_count.load(Ordering::Relaxed);
        let failure = self.failure_count.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);
        let total_lines = self.total_lines.load(Ordering::Relaxed);

        let avg_latency_ms = if success > 0 {
            (total_latency as f64 / success as f64) / 1000.0
        } else {
            0.0
        };

        let overall_rps = if elapsed.as_secs_f64() > 0.0 {
            total as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let progress = if total_lines > 0 {
            (total as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };

        StatsSnapshot {
            elapsed,
            total_processed: total,
            success_count: success,
            failure_count: failure,
            avg_latency_ms,
            current_rps: self.requests_per_second(),
            overall_rps,
            total_lines,
            progress,
        }
    }
}

impl Default for StatsTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// A snapshot of current statistics.
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    /// Elapsed time since start.
    pub elapsed: Duration,
    /// Total requests processed.
    pub total_processed: u64,
    /// Successful requests.
    pub success_count: u64,
    /// Failed requests.
    pub failure_count: u64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Current requests per second.
    pub current_rps: f64,
    /// Overall requests per second.
    pub overall_rps: f64,
    /// Total input lines.
    pub total_lines: usize,
    /// Progress percentage.
    pub progress: f64,
}

impl StatsSnapshot {
    /// Get the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.total_processed > 0 {
            (self.success_count as f64 / self.total_processed as f64) * 100.0
        } else {
            100.0
        }
    }

    /// Get the estimated time remaining.
    pub fn eta(&self) -> Option<Duration> {
        if self.overall_rps > 0.0 && self.total_lines > 0 {
            let remaining = self.total_lines.saturating_sub(self.total_processed as usize);
            let seconds = remaining as f64 / self.overall_rps;
            Some(Duration::from_secs_f64(seconds))
        } else {
            None
        }
    }

    /// Format as a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "Processed: {}/{} ({:.1}%) | Success: {} | Failed: {} | Avg Latency: {:.1}ms | RPS: {:.0}",
            self.total_processed,
            self.total_lines,
            self.progress,
            self.success_count,
            self.failure_count,
            self.avg_latency_ms,
            self.current_rps
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_tracking() {
        let tracker = StatsTracker::new();
        tracker.set_total_lines(100);

        tracker.record_success(Duration::from_millis(50));
        tracker.record_success(Duration::from_millis(100));
        tracker.record_failure();

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.total_processed, 3);
        assert_eq!(snapshot.success_count, 2);
        assert_eq!(snapshot.failure_count, 1);
        assert_eq!(snapshot.avg_latency_ms, 75.0);
    }

    #[test]
    fn test_success_rate() {
        let tracker = StatsTracker::new();

        for _ in 0..8 {
            tracker.record_success(Duration::from_millis(10));
        }
        for _ in 0..2 {
            tracker.record_failure();
        }

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.success_rate(), 80.0);
    }
}
