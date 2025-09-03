//! JIRA MCP Server Library
//!
//! An AI-friendly JIRA integration server using the Model Context Protocol (MCP).
//! This server provides semantic tools for searching, retrieving, and interacting
//! with JIRA issues without requiring knowledge of JQL or JIRA internals.
//!
//! ## Features
//!
//! - **AI-Friendly Interface**: Uses semantic parameters instead of JQL
//! - **Automatic JIRA Detection**: Leverages gouqi 0.14.0 for Cloud/Server detection
//! - **Smart Caching**: Metadata caching with TTL for performance
//! - **Comprehensive Tools**: Search, issue details, user issues
//! - **Error Handling**: MCP-compliant error codes and messages

use crate::cache::{MetadataCache, UserMapping};
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use crate::tools::{
    GetIssueDetailsParams, GetIssueDetailsResult, GetIssueDetailsTool, GetUserIssuesParams,
    GetUserIssuesResult, GetUserIssuesTool, SearchIssuesParams, SearchIssuesResult,
    SearchIssuesTool,
};

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, instrument, warn};

// Re-export modules for external use
pub mod cache;
pub mod config;
pub mod error;
pub mod jira_client;
pub mod semantic_mapping;
pub mod tools;

/// Server status information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JiraServerStatus {
    pub server_name: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub jira_url: String,
    pub jira_connection_status: String,
    pub authenticated_user: Option<String>,
    pub cache_stats: cache::CacheStats,
    pub tools_count: usize,
}

/// JIRA MCP Server
///
/// Main server implementation that provides AI-friendly tools for JIRA interaction.
/// Uses the #[mcp_server] macro for automatic MCP infrastructure generation.
#[mcp_server(
    name = "JIRA MCP Server",
    version = "0.1.0",
    description = "AI-friendly JIRA integration server with semantic search capabilities",
    auth = "disabled" // Start with disabled for development, can be changed to "file" for production
)]
#[derive(Clone)]
pub struct JiraMcpServer {
    /// Server start time for uptime calculation
    start_time: Instant,

    /// JIRA client for API operations
    jira_client: Arc<JiraClient>,

    /// Configuration
    config: Arc<JiraConfig>,

    /// Metadata cache
    cache: Arc<MetadataCache>,

    /// Tool implementations
    search_tool: Arc<SearchIssuesTool>,
    issue_details_tool: Arc<GetIssueDetailsTool>,
    user_issues_tool: Arc<GetUserIssuesTool>,
}

impl Default for JiraMcpServer {
    fn default() -> Self {
        // This is a placeholder default implementation
        // In practice, the server should be created using `new()` or `with_config()`
        panic!("JiraMcpServer cannot be created with default(). Use JiraMcpServer::new() instead.")
    }
}

impl JiraMcpServer {
    /// Create a new JIRA MCP Server with default configuration
    #[instrument]
    pub async fn new() -> JiraMcpResult<Self> {
        info!("Initializing JIRA MCP Server");

        // Load configuration
        let config = Arc::new(JiraConfig::load()?);
        info!("Configuration loaded successfully");

        // Create cache
        let cache = Arc::new(MetadataCache::new(config.cache_ttl_seconds));

        // Start cache cleanup task
        let _cleanup_handle = Arc::clone(&cache).start_cleanup_task();

        // Create JIRA client
        let jira_client = Arc::new(JiraClient::new(Arc::clone(&config)).await?);
        info!("JIRA client initialized");

        // Initialize current user in cache
        if let Ok(current_user) = jira_client.get_current_user().await {
            let user_mapping = UserMapping {
                account_id: current_user.account_id,
                display_name: current_user.display_name,
                email_address: current_user.email_address,
                username: None, // Will be filled if available
            };

            if let Err(e) = cache.set_current_user(user_mapping) {
                warn!("Failed to cache current user: {}", e);
            } else {
                info!("Current user cached successfully");
            }
        } else {
            warn!("Could not retrieve current user information");
        }

        // Create tool implementations
        let search_tool = Arc::new(SearchIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let issue_details_tool = Arc::new(GetIssueDetailsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let user_issues_tool = Arc::new(GetUserIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        info!("JIRA MCP Server initialized successfully");

        Ok(Self {
            start_time: Instant::now(),
            jira_client,
            config,
            cache,
            search_tool,
            issue_details_tool,
            user_issues_tool,
        })
    }

    /// Create server with custom configuration (for testing)
    #[instrument(skip(config))]
    pub async fn with_config(config: JiraConfig) -> JiraMcpResult<Self> {
        let config = Arc::new(config);
        let cache = Arc::new(MetadataCache::new(config.cache_ttl_seconds));
        let _cleanup_handle = Arc::clone(&cache).start_cleanup_task();

        let jira_client = Arc::new(JiraClient::new(Arc::clone(&config)).await?);

        // Try to initialize current user
        if let Ok(current_user) = jira_client.get_current_user().await {
            let user_mapping = UserMapping {
                account_id: current_user.account_id,
                display_name: current_user.display_name,
                email_address: current_user.email_address,
                username: None,
            };
            let _ = cache.set_current_user(user_mapping);
        }

        let search_tool = Arc::new(SearchIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let issue_details_tool = Arc::new(GetIssueDetailsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let user_issues_tool = Arc::new(GetUserIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        Ok(Self {
            start_time: Instant::now(),
            jira_client,
            config,
            cache,
            search_tool,
            issue_details_tool,
            user_issues_tool,
        })
    }

    /// Get server uptime in seconds
    fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get current user display name (for status)
    async fn get_current_user_name(&self) -> String {
        if let Some(user) = self.cache.get_current_user() {
            user.display_name
        } else if let Ok(user) = self.jira_client.get_current_user().await {
            user.display_name
        } else {
            "Unknown".to_string()
        }
    }
}

/// All public methods in this impl block become MCP tools automatically
/// The #[mcp_tools] macro discovers these methods and exposes them via MCP
#[mcp_tools]
impl JiraMcpServer {
    /// Search for JIRA issues using AI-friendly semantic parameters
    ///
    /// This tool allows AI agents to search for issues without needing to know JQL syntax.
    /// It accepts natural language parameters and translates them to appropriate JIRA queries.
    ///
    /// # Examples
    /// - Find all stories assigned to me: `{"issue_types": ["story"], "assigned_to": "me"}`
    /// - Find bugs in project FOO: `{"issue_types": ["bug"], "project_key": "FOO"}`
    /// - Find overdue issues: `{"status": ["open"], "created_after": "30 days ago"}`
    #[instrument(skip(self))]
    pub async fn search_issues(
        &self,
        params: SearchIssuesParams,
    ) -> anyhow::Result<SearchIssuesResult> {
        self.search_tool.execute(params).await.map_err(|e| {
            error!("search_issues failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Get detailed information about a specific JIRA issue
    ///
    /// Retrieves comprehensive information about an issue including summary, description,
    /// status, assignee, and optionally comments, attachments, and history.
    ///
    /// # Examples
    /// - Get basic issue info: `{"issue_key": "PROJ-123"}`
    /// - Get issue with comments: `{"issue_key": "PROJ-123", "include_comments": true}`
    /// - Get full issue details: `{"issue_key": "PROJ-123", "include_comments": true, "include_attachments": true, "include_history": true}`
    #[instrument(skip(self))]
    pub async fn get_issue_details(
        &self,
        params: GetIssueDetailsParams,
    ) -> anyhow::Result<GetIssueDetailsResult> {
        self.issue_details_tool.execute(params).await.map_err(|e| {
            error!("get_issue_details failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Get issues assigned to a specific user with filtering options
    ///
    /// Retrieves issues assigned to a user (defaults to current user) with various
    /// semantic filtering options for status, type, project, priority, and dates.
    ///
    /// # Examples
    /// - Get my open issues: `{"status_filter": ["open", "in_progress"]}`
    /// - Get user's bugs: `{"username": "john.doe", "issue_types": ["bug"]}`
    /// - Get overdue issues: `{"due_date_filter": "overdue", "priority_filter": ["high"]}`
    #[instrument(skip(self))]
    pub async fn get_user_issues(
        &self,
        params: GetUserIssuesParams,
    ) -> anyhow::Result<GetUserIssuesResult> {
        self.user_issues_tool.execute(params).await.map_err(|e| {
            error!("get_user_issues failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Get server status and connection information
    ///
    /// Returns comprehensive information about the server status, JIRA connection,
    /// authenticated user, cache statistics, and available tools.
    #[instrument(skip(self))]
    pub async fn get_server_status(&self) -> anyhow::Result<JiraServerStatus> {
        info!("Getting server status");

        let connection_status = match self.jira_client.get_current_user().await {
            Ok(_) => "Connected".to_string(),
            Err(e) => format!("Connection Error: {}", e),
        };

        let authenticated_user = if connection_status == "Connected" {
            Some(self.get_current_user_name().await)
        } else {
            None
        };

        Ok(JiraServerStatus {
            server_name: "JIRA MCP Server".to_string(),
            version: "0.1.0".to_string(),
            uptime_seconds: self.get_uptime_seconds(),
            jira_url: self.config.jira_url.clone(),
            jira_connection_status: connection_status,
            authenticated_user,
            cache_stats: self.cache.get_stats(),
            tools_count: 4, // search_issues, get_issue_details, get_user_issues, get_server_status
        })
    }

    /// Clear all cached metadata
    ///
    /// Clears all cached metadata including board mappings, project info, user info,
    /// and issue types. Useful when JIRA configuration changes or for troubleshooting.
    #[instrument(skip(self))]
    pub async fn clear_cache(&self) -> anyhow::Result<String> {
        info!("Clearing all cached metadata");

        match self.cache.clear_all() {
            Ok(()) => {
                info!("Cache cleared successfully");
                Ok("All cached metadata has been cleared successfully".to_string())
            }
            Err(e) => {
                error!("Failed to clear cache: {}", e);
                Err(anyhow::anyhow!("Failed to clear cache: {}", e))
            }
        }
    }

    /// Test JIRA connection and authentication
    ///
    /// Performs a connection test to the configured JIRA instance and returns
    /// detailed information about the connection status and authenticated user.
    #[instrument(skip(self))]
    pub async fn test_connection(&self) -> anyhow::Result<String> {
        info!("Testing JIRA connection");

        match self.jira_client.get_current_user().await {
            Ok(user) => {
                let message = format!(
                    "✅ Connection successful!\n\
                     JIRA URL: {}\n\
                     Authenticated as: {} ({})\n\
                     Account ID: {}\n\
                     Email: {}",
                    self.config.jira_url,
                    user.display_name,
                    user.email_address.as_deref().unwrap_or("N/A"),
                    user.account_id,
                    user.email_address.as_deref().unwrap_or("Not provided")
                );
                info!("Connection test successful for user: {}", user.display_name);
                Ok(message)
            }
            Err(e) => {
                let message = format!(
                    "❌ Connection failed!\n\
                     JIRA URL: {}\n\
                     Error: {}\n\
                     \n\
                     Please check:\n\
                     - JIRA URL is correct and accessible\n\
                     - Authentication credentials are valid\n\
                     - Network connectivity to JIRA instance",
                    self.config.jira_url, e
                );
                error!("Connection test failed: {}", e);
                Ok(message) // Return as success with error message for user feedback
            }
        }
    }
}

// Add any additional implementation methods here that are NOT MCP tools
impl JiraMcpServer {
    /// Internal method to refresh current user cache
    #[allow(dead_code)]
    async fn refresh_current_user_cache(&self) -> JiraMcpResult<()> {
        match self.jira_client.get_current_user().await {
            Ok(user) => {
                let user_mapping = UserMapping {
                    account_id: user.account_id,
                    display_name: user.display_name,
                    email_address: user.email_address,
                    username: None,
                };
                self.cache.set_current_user(user_mapping)
            }
            Err(e) => Err(e),
        }
    }

    /// Internal method to validate tool parameters (common validations)
    #[allow(dead_code)]
    fn validate_common_params(
        &self,
        limit: Option<u32>,
        start_at: Option<u32>,
    ) -> JiraMcpResult<()> {
        if let Some(limit) = limit {
            if limit == 0 {
                return Err(JiraMcpError::invalid_param(
                    "limit",
                    "Limit must be greater than 0",
                ));
            }
            if limit > 200 {
                return Err(JiraMcpError::invalid_param(
                    "limit",
                    "Limit cannot exceed 200",
                ));
            }
        }

        if let Some(start_at) = start_at {
            if start_at > 10000 {
                return Err(JiraMcpError::invalid_param(
                    "start_at",
                    "start_at cannot exceed 10000",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, JiraConfig};

    // Note: These tests require a real JIRA instance for integration testing
    // Unit tests are included in individual modules

    #[tokio::test]
    async fn test_server_creation_with_invalid_config() {
        let config = JiraConfig {
            jira_url: "invalid-url".to_string(),
            auth: AuthConfig::Anonymous,
            ..Default::default()
        };

        // Should fail validation
        assert!(JiraMcpServer::with_config(config).await.is_err());
    }

    #[test]
    fn test_uptime_calculation() {
        let start_time = Instant::now();
        // Sleep is not needed for this test, just checking the calculation
        let elapsed = start_time.elapsed().as_secs();
        // elapsed is u64, which is always >= 0, so we just check it's a reasonable value
        assert!(elapsed < 10); // Should be very small since we just started
    }
}
