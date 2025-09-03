//! Configuration management for the JIRA MCP Server
//!
//! Handles loading configuration from environment variables, TOML files,
//! and provides sensible defaults for all settings.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tracing::{debug, info, warn};

/// Main configuration structure for the JIRA MCP Server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    /// JIRA instance URL (required)
    pub jira_url: String,

    /// Authentication configuration (required)
    pub auth: AuthConfig,

    /// Cache TTL in seconds (default: 300 = 5 minutes)
    pub cache_ttl_seconds: u64,

    /// Maximum search results to return (default: 50, max: 200)
    pub max_search_results: u32,

    /// HTTP request timeout in seconds (default: 30)
    pub request_timeout_seconds: u64,

    /// Rate limit per minute (default: 60)
    pub rate_limit_per_minute: u32,

    /// Custom issue type mappings (semantic -> JIRA names)
    pub issue_type_mappings: HashMap<String, Vec<String>>,

    /// Custom status category mappings (semantic -> JIRA statuses)
    pub status_category_mappings: HashMap<String, Vec<String>>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    /// Personal Access Token (recommended)
    PersonalAccessToken(String),

    /// Basic authentication (username + password)
    Basic { username: String, password: String },

    /// Bearer token
    Bearer(String),

    /// Anonymous access (limited functionality)
    Anonymous,
}

impl Default for JiraConfig {
    fn default() -> Self {
        Self {
            jira_url: String::new(),
            auth: AuthConfig::Anonymous,
            cache_ttl_seconds: 300, // 5 minutes
            max_search_results: 50,
            request_timeout_seconds: 30,
            rate_limit_per_minute: 60,
            issue_type_mappings: default_issue_type_mappings(),
            status_category_mappings: default_status_category_mappings(),
        }
    }
}

impl JiraConfig {
    /// Load configuration from environment variables, TOML file, and defaults
    /// Priority: env vars > TOML file > defaults
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Try to load from TOML file first
        if let Ok(file_config) = Self::load_from_file("config/jira-mcp-config.toml") {
            info!("Loaded configuration from TOML file");
            config = file_config;
        } else if let Ok(file_config) = Self::load_from_file("jira-mcp-config.toml") {
            info!("Loaded configuration from TOML file in current directory");
            config = file_config;
        } else {
            debug!("No TOML configuration file found, using defaults and environment variables");
        }

        // Override with environment variables
        config.load_from_env()?;

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from a TOML file
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

        Ok(config)
    }

    /// Load configuration from environment variables
    fn load_from_env(&mut self) -> Result<()> {
        // JIRA URL (required if not in TOML)
        if let Ok(url) = env::var("JIRA_URL") {
            self.jira_url = url;
            debug!("Loaded JIRA_URL from environment");
        }

        // Authentication configuration
        if let Ok(auth_type) = env::var("JIRA_AUTH_TYPE") {
            match auth_type.to_lowercase().as_str() {
                "pat" | "personal_access_token" => {
                    if let Ok(token) = env::var("JIRA_TOKEN") {
                        self.auth = AuthConfig::PersonalAccessToken(token);
                        debug!("Configured Personal Access Token authentication from environment");
                    }
                }
                "basic" => {
                    let username = env::var("JIRA_USERNAME")
                        .context("JIRA_USERNAME required for basic authentication")?;
                    let password = env::var("JIRA_PASSWORD")
                        .context("JIRA_PASSWORD required for basic authentication")?;
                    self.auth = AuthConfig::Basic { username, password };
                    debug!("Configured basic authentication from environment");
                }
                "bearer" => {
                    if let Ok(token) = env::var("JIRA_TOKEN") {
                        self.auth = AuthConfig::Bearer(token);
                        debug!("Configured bearer token authentication from environment");
                    }
                }
                "anonymous" => {
                    self.auth = AuthConfig::Anonymous;
                    debug!("Configured anonymous authentication from environment");
                }
                _ => {
                    warn!("Unknown JIRA_AUTH_TYPE: {}, using default", auth_type);
                }
            }
        }

        // Optional configuration overrides
        if let Ok(ttl) = env::var("JIRA_CACHE_TTL") {
            if let Ok(ttl_seconds) = ttl.parse::<u64>() {
                self.cache_ttl_seconds = ttl_seconds;
                debug!("Set cache TTL to {} seconds from environment", ttl_seconds);
            }
        }

        if let Ok(max_results) = env::var("JIRA_MAX_RESULTS") {
            if let Ok(max) = max_results.parse::<u32>() {
                self.max_search_results = max.min(200); // Cap at 200
                debug!(
                    "Set max search results to {} from environment",
                    self.max_search_results
                );
            }
        }

        if let Ok(timeout) = env::var("JIRA_REQUEST_TIMEOUT") {
            if let Ok(timeout_seconds) = timeout.parse::<u64>() {
                self.request_timeout_seconds = timeout_seconds;
                debug!(
                    "Set request timeout to {} seconds from environment",
                    timeout_seconds
                );
            }
        }

        if let Ok(rate_limit) = env::var("JIRA_RATE_LIMIT") {
            if let Ok(limit) = rate_limit.parse::<u32>() {
                self.rate_limit_per_minute = limit;
                debug!("Set rate limit to {} per minute from environment", limit);
            }
        }

        Ok(())
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        // Validate JIRA URL
        if self.jira_url.is_empty() {
            return Err(anyhow::anyhow!(
                "JIRA URL is required. Set JIRA_URL environment variable or configure in TOML file."
            ));
        }

        // Validate URL format
        if !self.jira_url.starts_with("http://") && !self.jira_url.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "JIRA URL must start with http:// or https://. Got: {}",
                self.jira_url
            ));
        }

        // Validate authentication
        match &self.auth {
            AuthConfig::PersonalAccessToken(token) => {
                if token.is_empty() {
                    return Err(anyhow::anyhow!("Personal access token cannot be empty"));
                }
            }
            AuthConfig::Basic { username, password } => {
                if username.is_empty() || password.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Username and password cannot be empty for basic auth"
                    ));
                }
            }
            AuthConfig::Bearer(token) => {
                if token.is_empty() {
                    return Err(anyhow::anyhow!("Bearer token cannot be empty"));
                }
            }
            AuthConfig::Anonymous => {
                info!("Using anonymous authentication - functionality may be limited");
            }
        }

        // Validate numeric ranges
        if self.max_search_results > 200 {
            return Err(anyhow::anyhow!("max_search_results cannot exceed 200"));
        }

        if self.cache_ttl_seconds > 3600 {
            warn!("Cache TTL is set to more than 1 hour, this may cause stale data");
        }

        info!("Configuration validation successful");
        Ok(())
    }

    /// Get the gouqi Credentials from AuthConfig
    pub fn to_gouqi_credentials(&self) -> gouqi::Credentials {
        match &self.auth {
            AuthConfig::PersonalAccessToken(token) => gouqi::Credentials::Bearer(token.clone()),
            AuthConfig::Basic { username, password } => {
                gouqi::Credentials::Basic(username.clone(), password.clone())
            }
            AuthConfig::Bearer(token) => gouqi::Credentials::Bearer(token.clone()),
            AuthConfig::Anonymous => gouqi::Credentials::Anonymous,
        }
    }
}

/// Default issue type mappings (semantic -> JIRA issue type names)
fn default_issue_type_mappings() -> HashMap<String, Vec<String>> {
    let mut mappings = HashMap::new();

    mappings.insert(
        "story".to_string(),
        vec!["Story".to_string(), "User Story".to_string()],
    );
    mappings.insert(
        "feature".to_string(),
        vec!["Feature".to_string(), "New Feature".to_string()],
    );
    mappings.insert(
        "capability".to_string(),
        vec!["Capability".to_string(), "Epic".to_string()],
    );
    mappings.insert(
        "bug".to_string(),
        vec!["Bug".to_string(), "Defect".to_string()],
    );
    mappings.insert(
        "task".to_string(),
        vec!["Task".to_string(), "Sub-task".to_string()],
    );
    mappings.insert(
        "improvement".to_string(),
        vec!["Improvement".to_string(), "Enhancement".to_string()],
    );

    mappings
}

/// Default status category mappings (semantic -> JIRA status names)
fn default_status_category_mappings() -> HashMap<String, Vec<String>> {
    let mut mappings = HashMap::new();

    mappings.insert(
        "open".to_string(),
        vec![
            "Open".to_string(),
            "To Do".to_string(),
            "Backlog".to_string(),
            "New".to_string(),
        ],
    );

    mappings.insert(
        "in_progress".to_string(),
        vec![
            "In Progress".to_string(),
            "In Development".to_string(),
            "In Review".to_string(),
            "Code Review".to_string(),
            "Testing".to_string(),
        ],
    );

    mappings.insert(
        "done".to_string(),
        vec![
            "Done".to_string(),
            "Closed".to_string(),
            "Resolved".to_string(),
            "Complete".to_string(),
        ],
    );

    mappings.insert(
        "blocked".to_string(),
        vec![
            "Blocked".to_string(),
            "On Hold".to_string(),
            "Waiting".to_string(),
            "Paused".to_string(),
        ],
    );

    mappings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = JiraConfig::default();
        assert_eq!(config.cache_ttl_seconds, 300);
        assert_eq!(config.max_search_results, 50);
        assert_eq!(config.request_timeout_seconds, 30);
        assert!(!config.issue_type_mappings.is_empty());
        assert!(!config.status_category_mappings.is_empty());
    }

    #[test]
    fn test_env_var_loading() {
        env::set_var("JIRA_URL", "https://test.atlassian.net");
        env::set_var("JIRA_AUTH_TYPE", "pat");
        env::set_var("JIRA_TOKEN", "test_token");
        env::set_var("JIRA_CACHE_TTL", "600");

        let mut config = JiraConfig::default();
        config.load_from_env().unwrap();

        assert_eq!(config.jira_url, "https://test.atlassian.net");
        assert_eq!(config.cache_ttl_seconds, 600);

        match config.auth {
            AuthConfig::PersonalAccessToken(token) => assert_eq!(token, "test_token"),
            _ => panic!("Expected PersonalAccessToken auth"),
        }

        // Cleanup
        env::remove_var("JIRA_URL");
        env::remove_var("JIRA_AUTH_TYPE");
        env::remove_var("JIRA_TOKEN");
        env::remove_var("JIRA_CACHE_TTL");
    }

    #[test]
    fn test_validation_errors() {
        let mut config = JiraConfig::default();

        // Empty URL should fail validation
        assert!(config.validate().is_err());

        // Invalid URL format should fail
        config.jira_url = "not-a-url".to_string();
        assert!(config.validate().is_err());

        // Valid URL should pass
        config.jira_url = "https://test.atlassian.net".to_string();
        assert!(config.validate().is_ok());
    }
}
