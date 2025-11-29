//! Main processing orchestration for batch API requests.
//!
//! This module coordinates reading requests, distributing them across
//! endpoints, and writing results with rate limiting and concurrency control.

use crate::client::ApiClient;
use crate::config::Config;
use crate::endpoint::LoadBalancer;
use crate::error::{BlazeError, Result};
use crate::request::{ApiRequest, RequestResult};
use crate::tracker::StatsTracker;
use futures::stream::{self, StreamExt};
use governor::{Quota, RateLimiter};
use indicatif::{ProgressBar, ProgressStyle};
use parking_lot::Mutex;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tracing::{info, warn};

/// Processor for batch API requests.
pub struct Processor {
    config: Arc<Config>,
    client: ApiClient,
    load_balancer: Arc<LoadBalancer>,
    stats: Arc<StatsTracker>,
}

impl Processor {
    /// Create a new processor.
    pub fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let client = ApiClient::new(Arc::clone(&config))?;
        let load_balancer = Arc::new(LoadBalancer::new(config.endpoints.clone())?);
        let stats = Arc::new(StatsTracker::new());

        Ok(Self {
            config,
            client,
            load_balancer,
            stats,
        })
    }

    /// Process requests from a file.
    pub async fn process_file(
        &self,
        input_path: PathBuf,
        output_path: Option<PathBuf>,
        error_path: PathBuf,
        show_progress: bool,
    ) -> Result<ProcessingResult> {
        // Read all requests first to get total count
        let requests = self.read_requests(&input_path).await?;
        let total = requests.len();

        info!(total_requests = total, "Loaded requests from file");
        self.stats.set_total_lines(total);

        // Setup output files
        let output_writer = if let Some(path) = &output_path {
            let file = File::create(path).await.map_err(|e| BlazeError::OutputFileWrite {
                path: path.clone(),
                source: e,
            })?;
            Some(Arc::new(Mutex::new(BufWriter::new(file))))
        } else {
            None
        };

        let error_file = File::create(&error_path).await.map_err(|e| BlazeError::OutputFileWrite {
            path: error_path.clone(),
            source: e,
        })?;
        let error_writer = Arc::new(Mutex::new(BufWriter::new(error_file)));

        // Setup progress bar
        let progress = if show_progress {
            let pb = ProgressBar::new(total as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | {msg}")
                    .unwrap()
                    .progress_chars("█▓▒░"),
            );
            pb.enable_steady_tick(Duration::from_millis(100));
            Some(pb)
        } else {
            None
        };

        // Setup rate limiter
        let rate_limiter = RateLimiter::direct(Quota::per_second(
            NonZeroU32::new(self.config.request.rate_limit).unwrap_or(NonZeroU32::MIN),
        ));

        // Process requests concurrently
        let workers = self.config.request.workers;
        let results = stream::iter(requests)
            .map(|request| {
                let client = self.client.clone();
                let lb = Arc::clone(&self.load_balancer);
                let stats = Arc::clone(&self.stats);
                let rate_limiter = &rate_limiter;
                let output = output_writer.clone();
                let errors = Arc::clone(&error_writer);
                let progress = progress.clone();

                async move {
                    // Wait for rate limiter
                    rate_limiter.until_ready().await;

                    // Select an endpoint
                    let endpoint = match lb.select() {
                        Ok(ep) => ep,
                        Err(e) => {
                            warn!("Failed to select endpoint: {}", e);
                            return Err(e);
                        }
                    };

                    // Acquire a slot
                    if !endpoint.acquire() {
                        // Wait a bit and try again
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        if !endpoint.acquire() {
                            warn!("Endpoint at capacity, waiting...");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            endpoint.acquire();
                        }
                    }

                    // Send request
                    let result = client.send_with_retry(&request, endpoint).await;

                    // Record stats and write output
                    match &result {
                        RequestResult::Success(response) => {
                            let latency = response
                                .metadata
                                .as_ref()
                                .map(|m| Duration::from_millis(m.latency_ms))
                                .unwrap_or_default();
                            stats.record_success(latency);

                            if let Some(writer) = &output {
                                let line = serde_json::to_string(&response).unwrap_or_default();
                                let mut w = writer.lock();
                                let _ = futures::executor::block_on(async {
                                    w.write_all(line.as_bytes()).await?;
                                    w.write_all(b"\n").await
                                });
                            }
                        }
                        RequestResult::Failure(error) => {
                            stats.record_failure();
                            let line = serde_json::to_string(&error).unwrap_or_default();
                            let mut w = errors.lock();
                            let _ = futures::executor::block_on(async {
                                w.write_all(line.as_bytes()).await?;
                                w.write_all(b"\n").await
                            });
                        }
                    }

                    // Update progress bar
                    if let Some(pb) = &progress {
                        let snapshot = stats.snapshot();
                        pb.set_message(format!(
                            "RPS: {:.0} | Success: {} | Failed: {} | Latency: {:.0}ms",
                            snapshot.current_rps,
                            snapshot.success_count,
                            snapshot.failure_count,
                            snapshot.avg_latency_ms
                        ));
                        pb.inc(1);
                    }

                    Ok(result)
                }
            })
            .buffer_unordered(workers)
            .collect::<Vec<_>>()
            .await;

        // Flush writers
        if let Some(writer) = &output_writer {
            let mut w = writer.lock();
            w.flush().await.ok();
        }
        {
            let mut w = error_writer.lock();
            w.flush().await.ok();
        }

        // Finish progress bar
        if let Some(pb) = &progress {
            pb.finish_with_message("Complete!");
        }

        // Build result
        let snapshot = self.stats.snapshot();
        let success_count = results.iter().filter(|r| r.as_ref().map(|r| r.is_success()).unwrap_or(false)).count();
        let failure_count = results.len() - success_count;

        Ok(ProcessingResult {
            total_processed: results.len(),
            success_count,
            failure_count,
            elapsed: snapshot.elapsed,
            avg_latency_ms: snapshot.avg_latency_ms,
            overall_rps: snapshot.overall_rps,
        })
    }

    /// Read requests from a JSONL file.
    async fn read_requests(&self, path: &PathBuf) -> Result<Vec<ApiRequest>> {
        let file = File::open(path).await.map_err(|e| BlazeError::InputFileRead {
            path: path.clone(),
            source: e,
        })?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut requests = Vec::new();
        let mut line_number = 0;

        while let Some(line) = lines.next_line().await.map_err(|e| BlazeError::InputFileRead {
            path: path.clone(),
            source: e,
        })? {
            line_number += 1;

            // Skip empty lines
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut request: ApiRequest =
                serde_json::from_str(trimmed).map_err(|e| BlazeError::JsonParse {
                    line: line_number,
                    source: e,
                })?;

            request.line_number = line_number;
            requests.push(request);
        }

        Ok(requests)
    }

    /// Get the current stats snapshot.
    pub fn stats(&self) -> crate::tracker::StatsSnapshot {
        self.stats.snapshot()
    }

    /// Get the load balancer.
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }
}

/// Result of processing a batch of requests.
#[derive(Debug)]
pub struct ProcessingResult {
    /// Total requests processed.
    pub total_processed: usize,
    /// Successful requests.
    pub success_count: usize,
    /// Failed requests.
    pub failure_count: usize,
    /// Total elapsed time.
    pub elapsed: Duration,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Overall requests per second.
    pub overall_rps: f64,
}

impl ProcessingResult {
    /// Get the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.total_processed > 0 {
            (self.success_count as f64 / self.total_processed as f64) * 100.0
        } else {
            100.0
        }
    }

    /// Print a summary of the results.
    pub fn print_summary(&self) {
        println!("\n{}", "═".repeat(60));
        println!("                    PROCESSING COMPLETE");
        println!("{}", "═".repeat(60));
        println!("  Total Processed:  {}", self.total_processed);
        println!(
            "  Successful:       {} ({:.1}%)",
            self.success_count,
            self.success_rate()
        );
        println!("  Failed:           {}", self.failure_count);
        println!("  Elapsed Time:     {:.2}s", self.elapsed.as_secs_f64());
        println!("  Avg Latency:      {:.1}ms", self.avg_latency_ms);
        println!("  Throughput:       {:.0} req/sec", self.overall_rps);
        println!("{}", "═".repeat(60));
    }
}
