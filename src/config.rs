//! Configuration management for Blaze API.
//!
//! Supports configuration via CLI arguments, environment variables,
//! and configuration files with sensible defaults.

use crate::error::{BlazeError, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Duration;

/// CLI arguments for the Blaze API client.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "blaze",
    author = "Yiğit Konur <yigit@wope.com>",
    version,
    about = "🔥 High-performance async API client with load balancing",
    long_about = "Blaze API is a blazing-fast API client designed for batch LLM processing.\n\n\
                  It supports weighted load balancing, automatic retries with exponential backoff,\n\
                  and can handle 10,000+ requests per second on modest hardware.",
    after_help = "EXAMPLES:\n    \
        blaze --input requests.jsonl --output results.jsonl\n    \
        blaze -i data.jsonl -o out.jsonl --rate 5000 --workers 100\n    \
        blaze --config endpoints.json --input batch.jsonl"
)]
pub struct Args {
    /// Path to the JSONL file containing requests
    #[arg(short, long, env = "BLAZE_INPUT")]
    pub input: PathBuf,

    /// Path to save successful responses (optional)
    #[arg(short, long, env = "BLAZE_OUTPUT")]
    pub output: Option<PathBuf>,

    /// Path to save error responses
    #[arg(short, long, default_value = "errors.jsonl", env = "BLAZE_ERRORS")]
    pub errors: PathBuf,

    /// Maximum requests per second
    #[arg(short, long, default_value = "1000", env = "BLAZE_RATE")]
    pub rate: u32,

    /// Maximum retry attempts per request
    #[arg(short = 'a', long, default_value = "3", env = "BLAZE_MAX_ATTEMPTS")]
    pub max_attempts: u32,

    /// Number of concurrent workers
    #[arg(short, long, default_value = "50", env = "BLAZE_WORKERS")]
    pub workers: usize,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30", env = "BLAZE_TIMEOUT")]
    pub timeout: u64,

    /// Path to endpoint configuration file (JSON)
    #[arg(short, long, env = "BLAZE_CONFIG")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, env = "BLAZE_VERBOSE")]
    pub verbose: bool,

    /// Output logs as JSON
    #[arg(long, env = "BLAZE_JSON_LOGS")]
    pub json_logs: bool,

    /// Disable progress bar
    #[arg(long, env = "BLAZE_NO_PROGRESS")]
    pub no_progress: bool,

    /// Dry run - validate config without sending requests
    #[arg(long)]
    pub dry_run: bool,
}

impl Args {
    /// Parse CLI arguments.
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

/// Configuration for a single API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// The endpoint URL.
    pub url: String,

    /// Weight for load balancing (higher = more traffic).
    #[serde(default = "default_weight")]
    pub weight: u32,

    /// API key for authentication.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Model identifier (for LLM endpoints).
    #[serde(default)]
    pub model: Option<String>,

    /// Maximum concurrent requests to this endpoint.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
}

fn default_weight() -> u32 {
    1
}

fn default_max_concurrent() -> u32 {
    100
}

/// Full application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// API endpoints for load balancing.
    pub endpoints: Vec<EndpointConfig>,

    /// Request settings.
    #[serde(default)]
    pub request: RequestConfig,

    /// Retry settings.
    #[serde(default)]
    pub retry: RetryConfig,
}

/// Request-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    /// Request timeout.
    #[serde(with = "humantime_serde", default = "default_timeout")]
    pub timeout: Duration,

    /// Maximum requests per second.
    #[serde(default = "default_rate")]
    pub rate_limit: u32,

    /// Number of concurrent workers.
    #[serde(default = "default_workers")]
    pub workers: usize,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            rate_limit: default_rate(),
            workers: default_workers(),
        }
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_rate() -> u32 {
    1000
}

fn default_workers() -> usize {
    50
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Initial backoff duration.
    #[serde(with = "humantime_serde", default = "default_initial_backoff")]
    pub initial_backoff: Duration,

    /// Maximum backoff duration.
    #[serde(with = "humantime_serde", default = "default_max_backoff")]
    pub max_backoff: Duration,

    /// Backoff multiplier.
    #[serde(default = "default_multiplier")]
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_backoff: default_initial_backoff(),
            max_backoff: default_max_backoff(),
            multiplier: default_multiplier(),
        }
    }
}

fn default_max_attempts() -> u32 {
    3
}

fn default_initial_backoff() -> Duration {
    Duration::from_millis(100)
}

fn default_max_backoff() -> Duration {
    Duration::from_secs(10)
}

fn default_multiplier() -> f64 {
    2.0
}

impl Config {
    /// Load configuration from a file.
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| BlazeError::InputFileRead {
            path: path.clone(),
            source: e,
        })?;

        serde_json::from_str(&content).map_err(|e| BlazeError::JsonParse { line: 0, source: e })
    }

    /// Create configuration from CLI arguments.
    pub fn from_args(args: &Args) -> Result<Self> {
        let config = if let Some(config_path) = &args.config {
            let mut config = Self::from_file(config_path)?;
            // Override with CLI args
            config.request.rate_limit = args.rate;
            config.request.workers = args.workers;
            config.request.timeout = Duration::from_secs(args.timeout);
            config.retry.max_attempts = args.max_attempts;
            config
        } else {
            // Use default endpoint from environment or error
            let endpoint = EndpointConfig {
                url: std::env::var("BLAZE_ENDPOINT_URL")
                    .unwrap_or_else(|_| "http://localhost:8080/v1/completions".to_string()),
                weight: 1,
                api_key: std::env::var("BLAZE_API_KEY").ok(),
                model: std::env::var("BLAZE_MODEL").ok(),
                max_concurrent: 100,
            };

            Self {
                endpoints: vec![endpoint],
                request: RequestConfig {
                    timeout: Duration::from_secs(args.timeout),
                    rate_limit: args.rate,
                    workers: args.workers,
                },
                retry: RetryConfig {
                    max_attempts: args.max_attempts,
                    ..Default::default()
                },
            }
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.endpoints.is_empty() {
            return Err(BlazeError::NoEndpoints);
        }

        for endpoint in &self.endpoints {
            if endpoint.url.is_empty() {
                return Err(BlazeError::InvalidConfig(
                    "endpoint URL cannot be empty".to_string(),
                ));
            }
            if endpoint.weight == 0 {
                return Err(BlazeError::InvalidConfig(
                    "endpoint weight must be greater than 0".to_string(),
                ));
            }
        }

        if self.request.workers == 0 {
            return Err(BlazeError::InvalidConfig(
                "workers must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    /// Get the rate limit as a NonZeroU32.
    pub fn rate_limit_nonzero(&self) -> NonZeroU32 {
        NonZeroU32::new(self.request.rate_limit).unwrap_or(NonZeroU32::MIN)
    }
}

/// Custom serde module for humantime Duration parsing.
mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}s", duration.as_secs()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Simple parsing: support "30s", "100ms", or just seconds as number
        if let Some(secs) = s.strip_suffix('s') {
            secs.parse::<u64>()
                .map(Duration::from_secs)
                .map_err(serde::de::Error::custom)
        } else if let Some(ms) = s.strip_suffix("ms") {
            ms.parse::<u64>()
                .map(Duration::from_millis)
                .map_err(serde::de::Error::custom)
        } else {
            s.parse::<u64>()
                .map(Duration::from_secs)
                .map_err(serde::de::Error::custom)
        }
    }
}
