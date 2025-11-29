//! # Blaze API
//!
//! High-performance async API client with load balancing for batch LLM processing.
//!
//! Blaze API is designed to handle massive throughput (10,000+ requests per second)
//! with intelligent load balancing, automatic retries, and comprehensive error handling.
//!
//! ## Features
//!
//! - **Weighted Load Balancing**: Distribute requests across multiple endpoints based on weights
//! - **Automatic Retries**: Exponential backoff with jitter for failed requests
//! - **Rate Limiting**: Control throughput to respect API limits
//! - **Connection Pooling**: Efficient HTTP/2 connection management
//! - **Progress Tracking**: Real-time statistics and progress visualization
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use blaze_api::{Config, Processor, EndpointConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config {
//!         endpoints: vec![EndpointConfig {
//!             url: "https://api.example.com/v1/completions".to_string(),
//!             weight: 1,
//!             api_key: Some("your-api-key".to_string()),
//!             model: Some("gpt-4".to_string()),
//!             max_concurrent: 100,
//!         }],
//!         ..Default::default()
//!     };
//!
//!     let processor = Processor::new(config)?;
//!     let result = processor.process_file(
//!         "requests.jsonl".into(),
//!         Some("results.jsonl".into()),
//!         "errors.jsonl".into(),
//!         true,
//!     ).await?;
//!
//!     result.print_summary();
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! Blaze supports configuration via:
//! - Command-line arguments
//! - Environment variables (prefixed with `BLAZE_`)
//! - JSON configuration files
//!
//! See [`Config`] for all available options.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod client;
pub mod config;
pub mod endpoint;
pub mod error;
pub mod processor;
pub mod request;
pub mod tracker;

// Re-exports for convenience
pub use config::{Args, Config, EndpointConfig, RequestConfig, RetryConfig};
pub use endpoint::{Endpoint, LoadBalancer};
pub use error::{BlazeError, Result};
pub use processor::{ProcessingResult, Processor};
pub use request::{ApiRequest, ApiResponse, ErrorResponse, RequestResult};
pub use tracker::{StatsSnapshot, StatsTracker};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration for quick setup.
impl Default for Config {
    fn default() -> Self {
        Self {
            endpoints: vec![],
            request: RequestConfig::default(),
            retry: RetryConfig::default(),
        }
    }
}
