//! Endpoint management with weighted load balancing.
//!
//! This module provides a load balancer that distributes requests
//! across multiple endpoints based on configurable weights.

use crate::config::EndpointConfig;
use crate::error::{BlazeError, Result};
use parking_lot::RwLock;
use rand::prelude::*;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A single API endpoint with health tracking.
#[derive(Debug)]
pub struct Endpoint {
    /// Endpoint configuration.
    pub config: EndpointConfig,
    /// Current number of in-flight requests.
    pub in_flight: AtomicUsize,
    /// Total successful requests.
    pub success_count: AtomicU64,
    /// Total failed requests.
    pub failure_count: AtomicU64,
    /// Total latency in microseconds.
    pub total_latency_us: AtomicU64,
    /// Whether the endpoint is healthy.
    healthy: RwLock<bool>,
    /// Last health check time.
    last_health_check: RwLock<Option<Instant>>,
    /// Consecutive failures.
    consecutive_failures: AtomicUsize,
}

impl Endpoint {
    /// Create a new endpoint from configuration.
    pub fn new(config: EndpointConfig) -> Self {
        Self {
            config,
            in_flight: AtomicUsize::new(0),
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            healthy: RwLock::new(true),
            last_health_check: RwLock::new(None),
            consecutive_failures: AtomicUsize::new(0),
        }
    }

    /// Get the endpoint URL.
    pub fn url(&self) -> &str {
        &self.config.url
    }

    /// Get the API key if configured.
    pub fn api_key(&self) -> Option<&str> {
        self.config.api_key.as_deref()
    }

    /// Get the model if configured.
    pub fn model(&self) -> Option<&str> {
        self.config.model.as_deref()
    }

    /// Check if the endpoint is healthy.
    pub fn is_healthy(&self) -> bool {
        *self.healthy.read()
    }

    /// Mark the endpoint as healthy.
    pub fn mark_healthy(&self) {
        *self.healthy.write() = true;
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// Mark the endpoint as unhealthy.
    pub fn mark_unhealthy(&self) {
        *self.healthy.write() = false;
        *self.last_health_check.write() = Some(Instant::now());
    }

    /// Check if the endpoint should be retried (after cooldown).
    pub fn should_retry(&self, cooldown: Duration) -> bool {
        if self.is_healthy() {
            return true;
        }

        let last_check = self.last_health_check.read();
        match *last_check {
            Some(instant) => instant.elapsed() >= cooldown,
            None => true,
        }
    }

    /// Record a successful request.
    pub fn record_success(&self, latency: Duration) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.mark_healthy();
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;

        // Mark unhealthy after 3 consecutive failures
        if failures >= 3 {
            self.mark_unhealthy();
        }
    }

    /// Check if we can send more requests to this endpoint.
    pub fn can_accept(&self) -> bool {
        self.in_flight.load(Ordering::Relaxed) < self.config.max_concurrent as usize
    }

    /// Acquire a slot for sending a request.
    pub fn acquire(&self) -> bool {
        let current = self.in_flight.load(Ordering::Relaxed);
        if current >= self.config.max_concurrent as usize {
            return false;
        }
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Release a slot after completing a request.
    pub fn release(&self) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get average latency in milliseconds.
    pub fn avg_latency_ms(&self) -> f64 {
        let total = self.total_latency_us.load(Ordering::Relaxed);
        let count = self.success_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            (total as f64 / count as f64) / 1000.0
        }
    }
}

/// Weighted load balancer for distributing requests across endpoints.
#[derive(Debug)]
pub struct LoadBalancer {
    endpoints: Vec<Arc<Endpoint>>,
    #[allow(dead_code)]
    total_weight: u32,
}

impl LoadBalancer {
    /// Create a new load balancer from endpoint configurations.
    pub fn new(configs: Vec<EndpointConfig>) -> Result<Self> {
        if configs.is_empty() {
            return Err(BlazeError::NoEndpoints);
        }

        let endpoints: Vec<Arc<Endpoint>> = configs
            .into_iter()
            .map(|c| Arc::new(Endpoint::new(c)))
            .collect();

        let total_weight = endpoints.iter().map(|e| e.config.weight).sum();

        Ok(Self {
            endpoints,
            total_weight,
        })
    }

    /// Select an endpoint using weighted random selection.
    pub fn select(&self) -> Result<Arc<Endpoint>> {
        self.select_with_cooldown(Duration::from_secs(30))
    }

    /// Select an endpoint with a custom cooldown for unhealthy endpoints.
    pub fn select_with_cooldown(&self, cooldown: Duration) -> Result<Arc<Endpoint>> {
        // First, try to find a healthy endpoint with capacity
        let available: Vec<_> = self
            .endpoints
            .iter()
            .filter(|e| e.is_healthy() && e.can_accept())
            .collect();

        if !available.is_empty() {
            return Ok(self.weighted_select(&available));
        }

        // If no healthy endpoints, try endpoints past their cooldown
        let recovering: Vec<_> = self
            .endpoints
            .iter()
            .filter(|e| e.should_retry(cooldown) && e.can_accept())
            .collect();

        if !recovering.is_empty() {
            return Ok(self.weighted_select(&recovering));
        }

        Err(BlazeError::AllEndpointsUnhealthy)
    }

    /// Perform weighted random selection.
    fn weighted_select(&self, endpoints: &[&Arc<Endpoint>]) -> Arc<Endpoint> {
        let total: u32 = endpoints.iter().map(|e| e.config.weight).sum();
        let mut rng = rand::rng();
        let mut pick = rng.random_range(0..total);

        for endpoint in endpoints {
            if pick < endpoint.config.weight {
                return Arc::clone(endpoint);
            }
            pick -= endpoint.config.weight;
        }

        // Fallback to first endpoint (shouldn't happen)
        Arc::clone(endpoints[0])
    }

    /// Get all endpoints.
    pub fn endpoints(&self) -> &[Arc<Endpoint>] {
        &self.endpoints
    }

    /// Get the number of healthy endpoints.
    pub fn healthy_count(&self) -> usize {
        self.endpoints.iter().filter(|e| e.is_healthy()).count()
    }

    /// Get the total number of in-flight requests.
    pub fn total_in_flight(&self) -> usize {
        self.endpoints
            .iter()
            .map(|e| e.in_flight.load(Ordering::Relaxed))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_endpoint() -> EndpointConfig {
        EndpointConfig {
            url: "http://localhost:8080".to_string(),
            weight: 1,
            api_key: None,
            model: None,
            max_concurrent: 100,
        }
    }

    #[test]
    fn test_endpoint_health() {
        let endpoint = Endpoint::new(test_endpoint());
        assert!(endpoint.is_healthy());

        endpoint.mark_unhealthy();
        assert!(!endpoint.is_healthy());

        endpoint.mark_healthy();
        assert!(endpoint.is_healthy());
    }

    #[test]
    fn test_endpoint_stats() {
        let endpoint = Endpoint::new(test_endpoint());

        endpoint.record_success(Duration::from_millis(100));
        endpoint.record_success(Duration::from_millis(200));

        assert_eq!(endpoint.success_count.load(Ordering::Relaxed), 2);
        assert_eq!(endpoint.avg_latency_ms(), 150.0);
    }

    #[test]
    fn test_load_balancer() {
        let configs = vec![
            EndpointConfig {
                url: "http://a.test".to_string(),
                weight: 1,
                api_key: None,
                model: None,
                max_concurrent: 100,
            },
            EndpointConfig {
                url: "http://b.test".to_string(),
                weight: 2,
                api_key: None,
                model: None,
                max_concurrent: 100,
            },
        ];

        let lb = LoadBalancer::new(configs).unwrap();
        assert_eq!(lb.endpoints().len(), 2);
        assert_eq!(lb.healthy_count(), 2);
    }
}
