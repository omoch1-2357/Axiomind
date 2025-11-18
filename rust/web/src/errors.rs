/// Comprehensive error handling module for axiomind_web
///
/// This module provides:
/// - Structured error types for all components
/// - HTTP status code mappings
/// - Error response formatting
/// - Error logging utilities
use serde::{Deserialize, Serialize};
use std::fmt;
use warp::http::StatusCode;
use warp::reply::{self, Response};
use warp::Reply;

/// Standard error response format for all API endpoints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
    /// Machine-readable error code (e.g., "session_not_found")
    pub error: String,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (structured data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create error response with additional details
    pub fn with_details(
        error: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// Convert to HTTP response with specified status code
    pub fn into_response(self, status: StatusCode) -> Response {
        reply::with_status(reply::json(&self), status).into_response()
    }
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

/// Error classification for logging levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Client errors (4xx) - expected, normal operation
    Client,
    /// Server errors (5xx) - unexpected, needs investigation
    Server,
    /// Critical errors - system integrity at risk
    Critical,
}

/// Trait for converting errors to HTTP responses with proper logging
pub trait IntoErrorResponse {
    /// Get the HTTP status code for this error
    fn status_code(&self) -> StatusCode;

    /// Get the error code string (machine-readable)
    fn error_code(&self) -> &'static str;

    /// Get the error message (human-readable)
    fn error_message(&self) -> String;

    /// Get optional error details
    fn error_details(&self) -> Option<serde_json::Value> {
        None
    }

    /// Get error severity for logging
    fn severity(&self) -> ErrorSeverity {
        if self.status_code().is_server_error() {
            ErrorSeverity::Server
        } else {
            ErrorSeverity::Client
        }
    }

    /// Convert to ErrorResponse
    fn to_error_response(&self) -> ErrorResponse {
        if let Some(details) = self.error_details() {
            ErrorResponse::with_details(self.error_code(), self.error_message(), details)
        } else {
            ErrorResponse::new(self.error_code(), self.error_message())
        }
    }

    /// Convert to HTTP response with logging
    fn into_http_response(self) -> Response
    where
        Self: Sized,
    {
        let status = self.status_code();
        let severity = self.severity();
        let error_response = self.to_error_response();

        // Log error based on severity
        match severity {
            ErrorSeverity::Client => {
                log_client_error(&error_response);
            }
            ErrorSeverity::Server => {
                log_server_error(&error_response);
            }
            ErrorSeverity::Critical => {
                log_critical_error(&error_response);
            }
        }

        error_response.into_response(status)
    }
}

/// Log client error (4xx) - info level
fn log_client_error(error: &ErrorResponse) {
    eprintln!("[INFO] Client error: {} - {}", error.error, error.message);
}

/// Log server error (5xx) - error level
fn log_server_error(error: &ErrorResponse) {
    eprintln!("[ERROR] Server error: {} - {}", error.error, error.message);
}

/// Log critical error - critical level
fn log_critical_error(error: &ErrorResponse) {
    eprintln!(
        "[CRITICAL] Critical error: {} - {}",
        error.error, error.message
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn error_response_serialization() {
        let error = ErrorResponse::new("test_error", "Test error message");
        let json = serde_json::to_value(&error).expect("serialize");

        assert_eq!(json["error"], "test_error");
        assert_eq!(json["message"], "Test error message");
        assert!(json["details"].is_null());
    }

    #[test]
    fn error_response_with_details() {
        let details = json!({
            "field": "username",
            "constraint": "min_length"
        });

        let error = ErrorResponse::with_details("validation_error", "Invalid input", details);
        let json = serde_json::to_value(&error).expect("serialize");

        assert_eq!(json["error"], "validation_error");
        assert_eq!(json["details"]["field"], "username");
    }

    #[test]
    fn error_response_display() {
        let error = ErrorResponse::new("not_found", "Resource not found");
        let display = format!("{}", error);

        assert_eq!(display, "not_found: Resource not found");
    }

    #[test]
    fn error_severity_classification() {
        assert_eq!(ErrorSeverity::Client, ErrorSeverity::Client);
        assert_ne!(ErrorSeverity::Client, ErrorSeverity::Server);
    }
}
