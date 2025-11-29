//! HTTP client with retry logic and connection pooling.
//!
//! This module provides a high-performance HTTP client optimized for
//! high-throughput API requests with automatic retries.

use crate::config::Config;
use crate::endpoint::Endpoint;
use crate::error::{BlazeError, Result};
use crate::request::{ApiRequest, ApiResponse, ErrorResponse, RequestResult, ResponseMetadata};
use reqwest::{header, Client};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, trace, warn};

/// HTTP client wrapper with retry logic.
#[derive(Debug, Clone)]
pub struct ApiClient {
    client: Client,
    config: Arc<Config>,
}

impl ApiClient {
    /// Create a new API client.
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .timeout(config.request.timeout)
            .pool_max_idle_per_host(config.request.workers)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .default_headers(headers)
            .gzip(true)
            .brotli(true)
            .build()
            .map_err(BlazeError::HttpRequest)?;

        Ok(Self {
            client,
            config: config,
        })
    }

    /// Send a request to an endpoint with retries.
    pub async fn send_with_retry(
        &self,
        request: &ApiRequest,
        endpoint: Arc<Endpoint>,
    ) -> RequestResult {
        let mut attempts = 0;
        let mut last_error: Option<String> = None;
        let mut last_status: Option<u16> = None;

        let body = request.build_llm_body(endpoint.model());
        let start = Instant::now();

        while attempts < self.config.retry.max_attempts {
            attempts += 1;

            match self.send_once(&body, &endpoint).await {
                Ok(response) => {
                    let latency = start.elapsed();
                    endpoint.record_success(latency);
                    endpoint.release();

                    let api_response = ApiResponse::new(request.input.clone(), response)
                        .with_metadata(ResponseMetadata {
                            endpoint: endpoint.url().to_string(),
                            latency_ms: latency.as_millis() as u64,
                            attempts,
                        });

                    return RequestResult::Success(api_response);
                }
                Err((error, status)) => {
                    last_error = Some(error.clone());
                    last_status = status;

                    // Don't retry on certain status codes
                    if let Some(code) = status {
                        if code == 400 || code == 401 || code == 403 || code == 404 {
                            warn!(
                                endpoint = endpoint.url(),
                                status = code,
                                "Non-retryable error"
                            );
                            break;
                        }
                    }

                    if attempts < self.config.retry.max_attempts {
                        let backoff = self.calculate_backoff(attempts);
                        debug!(
                            attempt = attempts,
                            max_attempts = self.config.retry.max_attempts,
                            backoff_ms = backoff.as_millis(),
                            error = %error,
                            "Request failed, retrying"
                        );
                        sleep(backoff).await;
                    }
                }
            }
        }

        endpoint.record_failure();
        endpoint.release();

        let error_response =
            ErrorResponse::new(request, last_error.unwrap_or_else(|| "Unknown error".to_string()), attempts);

        let error_response = if let Some(status) = last_status {
            error_response.with_status(status)
        } else {
            error_response
        };

        RequestResult::Failure(error_response)
    }

    /// Send a single request without retries.
    async fn send_once(
        &self,
        body: &serde_json::Value,
        endpoint: &Endpoint,
    ) -> std::result::Result<serde_json::Value, (String, Option<u16>)> {
        let mut request = self.client.post(endpoint.url()).json(body);

        // Add authorization header if API key is configured
        if let Some(api_key) = endpoint.api_key() {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", api_key));
        }

        trace!(endpoint = endpoint.url(), "Sending request");

        let response = request.send().await.map_err(|e| {
            let error = format!("Request failed: {}", e);
            (error, e.status().map(|s| s.as_u16()))
        })?;

        let status = response.status();

        if status.is_success() {
            let body: serde_json::Value = response.json().await.map_err(|e| {
                (format!("Failed to parse response: {}", e), Some(status.as_u16()))
            })?;
            Ok(body)
        } else {
            let error_body = response.text().await.unwrap_or_default();
            let truncated = if error_body.len() > 500 {
                format!("{}...", &error_body[..500])
            } else {
                error_body
            };
            Err((
                format!("HTTP {}: {}", status.as_u16(), truncated),
                Some(status.as_u16()),
            ))
        }
    }

    /// Calculate backoff duration for a given attempt.
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base = self.config.retry.initial_backoff.as_millis() as f64;
        let multiplier = self.config.retry.multiplier.powi(attempt as i32 - 1);
        let backoff_ms = base * multiplier;

        // Add jitter (±25%)
        let jitter = 1.0 + (rand::random::<f64>() - 0.5) * 0.5;
        let final_ms = (backoff_ms * jitter) as u64;

        Duration::from_millis(final_ms.min(self.config.retry.max_backoff.as_millis() as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EndpointConfig, RequestConfig, RetryConfig};

    fn test_config() -> Config {
        Config {
            endpoints: vec![EndpointConfig {
                url: "http://localhost:8080".to_string(),
                weight: 1,
                api_key: None,
                model: None,
                max_concurrent: 100,
            }],
            request: RequestConfig::default(),
            retry: RetryConfig::default(),
        }
    }

    #[test]
    fn test_backoff_calculation() {
        let config = Arc::new(test_config());
        let client = ApiClient::new(config).unwrap();

        let b1 = client.calculate_backoff(1);
        let b2 = client.calculate_backoff(2);
        let b3 = client.calculate_backoff(3);

        // Backoff should generally increase (allowing for jitter)
        assert!(b1 < Duration::from_secs(1));
        assert!(b2 < Duration::from_secs(2));
        assert!(b3 < Duration::from_secs(5));
    }
}
