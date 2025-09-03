//! Error types and handling for the JIRA MCP Server
//!
//! Provides structured error types that map to MCP JSON-RPC error codes
//! and converts various error types from dependencies into MCP-compatible errors.

use serde_json::Value;
use thiserror::Error;

/// Custom error types for the JIRA MCP Server
#[derive(Debug, Error)]
pub enum JiraMcpError {
    /// Configuration errors (-32001)
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Authentication failures (-32002)
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// Network errors (-32003)
    #[error("Network error: {message}")]
    Network { message: String },

    /// Permission denied errors (-32004)
    #[error("Permission denied: {message}")]
    Permission { message: String },

    /// Resource not found errors (-32005)
    #[error("Not found: {resource} '{key}' not found")]
    NotFound { resource: String, key: String },

    /// Invalid parameter errors (-32006)
    #[error("Invalid parameter: {parameter} - {message}")]
    InvalidParameter { parameter: String, message: String },

    /// Rate limit exceeded errors (-32007)
    #[error("Rate limit exceeded, retry after {retry_after} seconds")]
    RateLimit { retry_after: u64 },

    /// Cache errors (internal, mapped to appropriate codes)
    #[error("Cache error: {message}")]
    Cache { message: String },

    /// JQL query building errors
    #[error("JQL error: {message}")]
    JqlError { message: String },

    /// Internal server errors
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl JiraMcpError {
    /// Get the MCP JSON-RPC error code for this error
    pub fn error_code(&self) -> i32 {
        match self {
            JiraMcpError::Configuration { .. } => -32001,
            JiraMcpError::Authentication { .. } => -32002,
            JiraMcpError::Network { .. } => -32003,
            JiraMcpError::Permission { .. } => -32004,
            JiraMcpError::NotFound { .. } => -32005,
            JiraMcpError::InvalidParameter { .. } => -32006,
            JiraMcpError::RateLimit { .. } => -32007,
            JiraMcpError::Cache { .. } => -32003, // Network error category
            JiraMcpError::JqlError { .. } => -32006, // Invalid parameter category
            JiraMcpError::Internal { .. } => -32603, // Internal error
        }
    }

    /// Get the error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            JiraMcpError::Configuration { .. } => "configuration",
            JiraMcpError::Authentication { .. } => "authentication",
            JiraMcpError::Network { .. } => "network",
            JiraMcpError::Permission { .. } => "permission",
            JiraMcpError::NotFound { .. } => "not_found",
            JiraMcpError::InvalidParameter { .. } => "invalid_parameter",
            JiraMcpError::RateLimit { .. } => "rate_limit",
            JiraMcpError::Cache { .. } => "cache",
            JiraMcpError::JqlError { .. } => "jql",
            JiraMcpError::Internal { .. } => "internal",
        }
    }

    /// Get additional error data for MCP error responses
    pub fn error_data(&self) -> Option<Value> {
        let mut data = serde_json::Map::new();
        data.insert(
            "category".to_string(),
            Value::String(self.category().to_string()),
        );

        match self {
            JiraMcpError::RateLimit { retry_after } => {
                data.insert(
                    "retry_after".to_string(),
                    Value::Number((*retry_after).into()),
                );
                Some(Value::Object(data))
            }
            JiraMcpError::NotFound { resource, key } => {
                data.insert("resource".to_string(), Value::String(resource.clone()));
                data.insert("key".to_string(), Value::String(key.clone()));
                Some(Value::Object(data))
            }
            JiraMcpError::InvalidParameter { parameter, .. } => {
                data.insert("parameter".to_string(), Value::String(parameter.clone()));
                Some(Value::Object(data))
            }
            _ => {
                if !data.is_empty() {
                    Some(Value::Object(data))
                } else {
                    None
                }
            }
        }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        JiraMcpError::Configuration {
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        JiraMcpError::Authentication {
            message: message.into(),
        }
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        JiraMcpError::Network {
            message: message.into(),
        }
    }

    /// Create a permission error
    pub fn permission(message: impl Into<String>) -> Self {
        JiraMcpError::Permission {
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>, key: impl Into<String>) -> Self {
        JiraMcpError::NotFound {
            resource: resource.into(),
            key: key.into(),
        }
    }

    /// Create an invalid parameter error
    pub fn invalid_param(parameter: impl Into<String>, message: impl Into<String>) -> Self {
        JiraMcpError::InvalidParameter {
            parameter: parameter.into(),
            message: message.into(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit(retry_after: u64) -> Self {
        JiraMcpError::RateLimit { retry_after }
    }

    /// Create a cache error
    pub fn cache(message: impl Into<String>) -> Self {
        JiraMcpError::Cache {
            message: message.into(),
        }
    }

    /// Create a JQL error
    pub fn jql(message: impl Into<String>) -> Self {
        JiraMcpError::JqlError {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        JiraMcpError::Internal {
            message: message.into(),
        }
    }
}

/// Convert from gouqi errors to JiraMcpError
impl From<gouqi::Error> for JiraMcpError {
    fn from(err: gouqi::Error) -> Self {
        match err {
            // Map gouqi errors to appropriate MCP error types
            gouqi::Error::Http(_) => JiraMcpError::network(format!("HTTP error: {}", err)),
            gouqi::Error::IO(_) => JiraMcpError::network(format!("IO error: {}", err)),
            gouqi::Error::Serde(_) => {
                JiraMcpError::internal(format!("Serialization error: {}", err))
            }
            gouqi::Error::Unauthorized => JiraMcpError::auth("JIRA authentication failed"),
            gouqi::Error::NotFound => JiraMcpError::not_found("resource", "unknown"),
            gouqi::Error::Fault { .. } => {
                JiraMcpError::internal(format!("JIRA API error: {}", err))
            }
            _ => JiraMcpError::internal(format!("JIRA client error: {}", err)),
        }
    }
}

/// Convert from serde_json errors
impl From<serde_json::Error> for JiraMcpError {
    fn from(err: serde_json::Error) -> Self {
        JiraMcpError::internal(format!("JSON error: {}", err))
    }
}

/// Convert from TOML parsing errors
impl From<toml::de::Error> for JiraMcpError {
    fn from(err: toml::de::Error) -> Self {
        JiraMcpError::config(format!("TOML parsing error: {}", err))
    }
}

/// Convert from generic anyhow errors
impl From<anyhow::Error> for JiraMcpError {
    fn from(err: anyhow::Error) -> Self {
        // Try to determine the category based on the error message
        let message = err.to_string();
        let lower_message = message.to_lowercase();

        if lower_message.contains("authentication") || lower_message.contains("unauthorized") {
            JiraMcpError::auth(message)
        } else if lower_message.contains("not found") || lower_message.contains("404") {
            JiraMcpError::not_found("resource", "unknown")
        } else if lower_message.contains("permission")
            || lower_message.contains("forbidden")
            || lower_message.contains("403")
        {
            JiraMcpError::permission(message)
        } else if lower_message.contains("network")
            || lower_message.contains("connection")
            || lower_message.contains("timeout")
        {
            JiraMcpError::network(message)
        } else if lower_message.contains("rate limit") || lower_message.contains("429") {
            JiraMcpError::rate_limit(60) // Default retry after 60 seconds
        } else if lower_message.contains("config") {
            JiraMcpError::config(message)
        } else {
            JiraMcpError::internal(message)
        }
    }
}

/// Helper function to extract retry-after from HTTP errors
pub fn extract_retry_after(error_message: &str) -> Option<u64> {
    // Simple pattern matching to extract retry-after seconds
    // This would need to be more sophisticated in a real implementation
    if error_message.contains("retry-after") {
        // Try to extract the number
        for word in error_message.split_whitespace() {
            if let Ok(seconds) = word.parse::<u64>() {
                return Some(seconds);
            }
        }
    }
    None
}

/// Result type alias for JIRA MCP operations
pub type JiraMcpResult<T> = Result<T, JiraMcpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(JiraMcpError::config("test").error_code(), -32001);
        assert_eq!(JiraMcpError::auth("test").error_code(), -32002);
        assert_eq!(JiraMcpError::network("test").error_code(), -32003);
        assert_eq!(JiraMcpError::permission("test").error_code(), -32004);
        assert_eq!(
            JiraMcpError::not_found("issue", "KEY-123").error_code(),
            -32005
        );
        assert_eq!(
            JiraMcpError::invalid_param("status", "invalid").error_code(),
            -32006
        );
        assert_eq!(JiraMcpError::rate_limit(60).error_code(), -32007);
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(JiraMcpError::config("test").category(), "configuration");
        assert_eq!(JiraMcpError::auth("test").category(), "authentication");
        assert_eq!(JiraMcpError::network("test").category(), "network");
        assert_eq!(
            JiraMcpError::not_found("issue", "KEY-123").category(),
            "not_found"
        );
    }

    #[test]
    fn test_error_data() {
        let rate_limit_error = JiraMcpError::rate_limit(120);
        let data = rate_limit_error.error_data().unwrap();

        assert_eq!(data["category"], "rate_limit");
        assert_eq!(data["retry_after"], 120);

        let not_found_error = JiraMcpError::not_found("issue", "KEY-123");
        let data = not_found_error.error_data().unwrap();

        assert_eq!(data["category"], "not_found");
        assert_eq!(data["resource"], "issue");
        assert_eq!(data["key"], "KEY-123");
    }

    #[test]
    fn test_anyhow_conversion() {
        let auth_error = anyhow::anyhow!("Authentication failed");
        let jira_error: JiraMcpError = auth_error.into();
        assert_eq!(jira_error.category(), "authentication");

        let not_found_error = anyhow::anyhow!("Issue not found");
        let jira_error: JiraMcpError = not_found_error.into();
        assert_eq!(jira_error.category(), "not_found");
    }

    #[test]
    fn test_retry_after_extraction() {
        assert_eq!(
            extract_retry_after("Rate limit exceeded, retry-after 60"),
            Some(60)
        );
        assert_eq!(extract_retry_after("No retry info"), None);
    }
}
