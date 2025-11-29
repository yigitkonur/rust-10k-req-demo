//! Custom error types for Blaze API.
//!
//! This module defines all error types used throughout the application,
//! following Rust best practices with `thiserror` for library errors.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during API processing.
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum BlazeError {
    /// Failed to read the input file.
    #[error("failed to read input file '{path}': {source}")]
    InputFileRead {
        /// Path to the file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to write to the output file.
    #[error("failed to write to output file '{path}': {source}")]
    OutputFileWrite {
        /// Path to the file that could not be written.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse JSON from the input file.
    #[error("failed to parse JSON at line {line}: {source}")]
    JsonParse {
        /// Line number where the error occurred.
        line: usize,
        /// The underlying JSON parsing error.
        #[source]
        source: serde_json::Error,
    },

    /// Failed to serialize JSON for output.
    #[error("failed to serialize JSON: {0}")]
    JsonSerialize(#[from] serde_json::Error),

    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// No endpoints configured.
    #[error("no endpoints configured - at least one endpoint is required")]
    NoEndpoints,

    /// All endpoints are unhealthy.
    #[error("all endpoints are currently unhealthy")]
    AllEndpointsUnhealthy,

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Request timed out.
    #[error("request timed out after {attempts} attempts")]
    Timeout {
        /// Number of attempts made before timeout.
        attempts: u32,
    },

    /// Rate limit exceeded.
    #[error("rate limit exceeded for endpoint '{endpoint}'")]
    RateLimitExceeded {
        /// The endpoint that exceeded its rate limit.
        endpoint: String,
    },

    /// Invalid response from API.
    #[error("invalid API response: {message}")]
    InvalidResponse {
        /// Description of what was invalid.
        message: String,
    },

    /// Endpoint returned an error status.
    #[error("endpoint returned error status {status}: {body}")]
    EndpointError {
        /// HTTP status code returned.
        status: u16,
        /// Response body content.
        body: String,
    },
}

/// Result type alias for Blaze operations.
pub type Result<T> = std::result::Result<T, BlazeError>;
