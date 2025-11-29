//! Request and response types for API processing.
//!
//! This module defines the data structures for API requests and responses,
//! supporting flexible input formats and structured output.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// An API request read from the input file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    /// The main input content (for LLM requests).
    #[serde(default)]
    pub input: Option<String>,

    /// Custom request body (overrides default formatting).
    #[serde(default)]
    pub body: Option<Value>,

    /// Custom headers for this specific request.
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,

    /// Request-specific metadata (passed through to response).
    #[serde(default, flatten)]
    pub metadata: HashMap<String, Value>,

    /// Line number in the input file (set during parsing).
    #[serde(skip)]
    pub line_number: usize,
}

impl ApiRequest {
    /// Create a simple request with just input text.
    pub fn simple(input: impl Into<String>) -> Self {
        Self {
            input: Some(input.into()),
            body: None,
            headers: None,
            metadata: HashMap::new(),
            line_number: 0,
        }
    }

    /// Create a request with a custom body.
    pub fn with_body(body: Value) -> Self {
        Self {
            input: None,
            body: Some(body),
            headers: None,
            metadata: HashMap::new(),
            line_number: 0,
        }
    }

    /// Build the request body for an LLM endpoint.
    pub fn build_llm_body(&self, model: Option<&str>) -> Value {
        if let Some(body) = &self.body {
            // Use custom body if provided
            return body.clone();
        }

        // Build standard LLM request body
        let input = self.input.as_deref().unwrap_or("");
        let mut body = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": input
            }]
        });

        if let Some(model) = model {
            body["model"] = Value::String(model.to_string());
        }

        body
    }

    /// Get a display string for logging.
    pub fn display_input(&self) -> String {
        if let Some(input) = &self.input {
            if input.len() > 50 {
                format!("{}...", &input[..50])
            } else {
                input.clone()
            }
        } else if self.body.is_some() {
            "[custom body]".to_string()
        } else {
            "[empty]".to_string()
        }
    }
}

/// A successful API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    /// The original input (for correlation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,

    /// The response body from the API.
    pub response: Value,

    /// Response metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// Metadata about the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Which endpoint handled the request.
    pub endpoint: String,

    /// Response latency in milliseconds.
    pub latency_ms: u64,

    /// Number of retry attempts.
    pub attempts: u32,
}

impl ApiResponse {
    /// Create a new API response.
    pub fn new(input: Option<String>, response: Value) -> Self {
        Self {
            input,
            response,
            metadata: None,
        }
    }

    /// Add metadata to the response.
    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// An error response for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// The original input that failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,

    /// The original request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,

    /// Error message.
    pub error: String,

    /// HTTP status code (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,

    /// Line number in the input file.
    #[serde(skip_serializing_if = "is_zero")]
    pub line_number: usize,

    /// Number of attempts made.
    pub attempts: u32,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

impl ErrorResponse {
    /// Create a new error response.
    pub fn new(request: &ApiRequest, error: impl Into<String>, attempts: u32) -> Self {
        Self {
            input: request.input.clone(),
            body: request.body.clone(),
            error: error.into(),
            status_code: None,
            line_number: request.line_number,
            attempts,
        }
    }

    /// Set the HTTP status code.
    pub fn with_status(mut self, status: u16) -> Self {
        self.status_code = Some(status);
        self
    }
}

/// Result of processing a single request.
#[derive(Debug)]
pub enum RequestResult {
    /// Request succeeded.
    Success(ApiResponse),
    /// Request failed after all retries.
    Failure(ErrorResponse),
}

impl RequestResult {
    /// Check if this is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_request() {
        let req = ApiRequest::simple("Hello, world!");
        assert_eq!(req.input, Some("Hello, world!".to_string()));
        assert!(req.body.is_none());
    }

    #[test]
    fn test_build_llm_body() {
        let req = ApiRequest::simple("Test input");
        let body = req.build_llm_body(Some("gpt-4"));

        assert_eq!(body["model"], "gpt-4");
        assert_eq!(body["messages"][0]["content"], "Test input");
    }

    #[test]
    fn test_custom_body() {
        let custom = serde_json::json!({"custom": "data"});
        let req = ApiRequest::with_body(custom.clone());
        let body = req.build_llm_body(Some("gpt-4"));

        assert_eq!(body, custom);
    }

    #[test]
    fn test_error_response() {
        let req = ApiRequest::simple("Test");
        let err = ErrorResponse::new(&req, "Connection refused", 3);

        assert_eq!(err.error, "Connection refused");
        assert_eq!(err.attempts, 3);
    }
}
