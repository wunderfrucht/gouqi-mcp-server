//! Issue details tool for retrieving comprehensive issue information
//!
//! This tool provides detailed information about a specific JIRA issue,
//! with options to include additional data like comments, attachments, and history.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::{IssueDetails, JiraClient};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// Parameters for the get_issue_details tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetIssueDetailsParams {
    /// JIRA issue key (required)
    /// Examples: "PROJ-123", "KEY-456"
    pub issue_key: String,

    /// Include comments in the response (optional, default: false)
    pub include_comments: Option<bool>,

    /// Include attachment metadata in the response (optional, default: false)
    pub include_attachments: Option<bool>,

    /// Include change history in the response (optional, default: false)
    pub include_history: Option<bool>,
}

/// Result from the get_issue_details tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIssueDetailsResult {
    /// Detailed issue information
    pub issue_details: IssueDetails,

    /// Performance information
    pub performance: IssueDetailsPerformance,

    /// Additional metadata
    pub metadata: IssueDetailsMetadata,
}

/// Performance metrics for issue details operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueDetailsPerformance {
    /// Time taken for the operation in milliseconds
    pub duration_ms: u64,

    /// Whether the issue data hit the cache
    pub cache_hit: bool,

    /// Number of JIRA API calls made
    pub api_calls: u32,

    /// Size of the response data (estimated)
    pub response_size_estimate: usize,
}

/// Metadata about the issue details operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueDetailsMetadata {
    /// Issue key that was requested
    pub requested_key: String,

    /// Issue key that was actually resolved (in case of moved issues)
    pub resolved_key: String,

    /// Whether additional data was included
    pub includes_comments: bool,
    pub includes_attachments: bool,
    pub includes_history: bool,

    /// Data freshness information
    pub data_freshness: String, // "fresh", "cached", "partially_cached"
}

/// Implementation of the get_issue_details tool
pub struct GetIssueDetailsTool {
    jira_client: Arc<JiraClient>,
    #[allow(dead_code)]
    config: Arc<JiraConfig>,
    #[allow(dead_code)]
    cache: Arc<MetadataCache>,
}

impl GetIssueDetailsTool {
    /// Create a new get issue details tool
    pub fn new(
        jira_client: Arc<JiraClient>,
        config: Arc<JiraConfig>,
        cache: Arc<MetadataCache>,
    ) -> Self {
        Self {
            jira_client,
            config,
            cache,
        }
    }

    /// Execute the get_issue_details tool
    #[instrument(skip(self), fields(
        issue_key = %params.issue_key,
        include_comments = params.include_comments.unwrap_or(false),
        include_attachments = params.include_attachments.unwrap_or(false),
        include_history = params.include_history.unwrap_or(false),
    ))]
    pub async fn execute(
        &self,
        params: GetIssueDetailsParams,
    ) -> JiraMcpResult<GetIssueDetailsResult> {
        let start_time = std::time::Instant::now();
        let mut api_calls = 0u32;
        let cache_hit = false;

        info!(
            "Executing get_issue_details tool for issue: {}",
            params.issue_key
        );

        // Validate parameters
        self.validate_params(&params)?;

        // Extract options with defaults
        let include_comments = params.include_comments.unwrap_or(false);
        let include_attachments = params.include_attachments.unwrap_or(false);
        let include_history = params.include_history.unwrap_or(false);

        // Normalize issue key (convert to uppercase, handle different formats)
        let normalized_key = self.normalize_issue_key(&params.issue_key)?;

        // Get issue details from JIRA
        let issue_details = self
            .jira_client
            .get_issue_details(
                &normalized_key,
                include_comments,
                include_attachments,
                include_history,
            )
            .await?;

        api_calls += 1;
        let duration = start_time.elapsed();

        // Estimate response size (rough calculation)
        let response_size_estimate = self.estimate_response_size(
            &issue_details,
            include_comments,
            include_attachments,
            include_history,
        );

        // Determine data freshness
        let data_freshness = if cache_hit {
            "cached".to_string()
        } else {
            "fresh".to_string()
        };

        info!(
            "Retrieved issue details for {} in {}ms (size: ~{} bytes)",
            normalized_key,
            duration.as_millis(),
            response_size_estimate
        );

        // Check for potential issues with the response
        self.validate_response(&issue_details)?;

        Ok(GetIssueDetailsResult {
            issue_details,
            performance: IssueDetailsPerformance {
                duration_ms: duration.as_millis() as u64,
                cache_hit,
                api_calls,
                response_size_estimate,
            },
            metadata: IssueDetailsMetadata {
                requested_key: params.issue_key,
                resolved_key: normalized_key,
                includes_comments: include_comments,
                includes_attachments: include_attachments,
                includes_history: include_history,
                data_freshness,
            },
        })
    }

    /// Validate issue details parameters
    fn validate_params(&self, params: &GetIssueDetailsParams) -> JiraMcpResult<()> {
        // Validate issue key format
        if params.issue_key.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key is required. Please provide a JIRA issue key (e.g., 'PROJ-123'). Use the search_issues tool first to find issues if you don't know the key.",
            ));
        }

        // Basic format validation (PROJECT-NUMBER pattern)
        let key = params.issue_key.trim();
        if !key.contains('-') {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key must follow PROJECT-NUMBER format (e.g., 'PROJ-123')",
            ));
        }

        let parts: Vec<&str> = key.split('-').collect();
        if parts.len() < 2 {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key must contain at least one hyphen separating project and number",
            ));
        }

        // Validate project part (should be alphabetic)
        let project_part = parts[0];
        if project_part.is_empty() || !project_part.chars().all(|c| c.is_alphabetic()) {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Project part of issue key must contain only letters",
            ));
        }

        // Validate number part (should be numeric)
        let number_part = parts[1];
        if number_part.is_empty() || !number_part.chars().all(|c| c.is_numeric()) {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Number part of issue key must contain only digits",
            ));
        }

        // Check key length
        if key.len() > 100 {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key cannot exceed 100 characters",
            ));
        }

        Ok(())
    }

    /// Normalize issue key (uppercase, trim whitespace)
    fn normalize_issue_key(&self, key: &str) -> JiraMcpResult<String> {
        let normalized = key.trim().to_uppercase();

        // Additional validation after normalization
        if normalized.len() < 3 {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key too short after normalization",
            ));
        }

        Ok(normalized)
    }

    /// Validate the response from JIRA
    fn validate_response(&self, issue_details: &IssueDetails) -> JiraMcpResult<()> {
        // Check if essential fields are present
        if issue_details.issue_info.key.is_empty() {
            warn!("Issue response missing key field");
        }

        if issue_details.issue_info.summary.is_empty() {
            warn!("Issue response missing summary field");
        }

        // Check for very large responses that might cause issues
        if issue_details.comments.as_ref().map_or(0, |c| c.len()) > 1000 {
            warn!(
                "Issue has {} comments, response may be large",
                issue_details.comments.as_ref().unwrap().len()
            );
        }

        if issue_details.attachments.as_ref().map_or(0, |a| a.len()) > 100 {
            warn!(
                "Issue has {} attachments, response may be large",
                issue_details.attachments.as_ref().unwrap().len()
            );
        }

        Ok(())
    }

    /// Estimate response size in bytes (rough calculation)
    fn estimate_response_size(
        &self,
        issue_details: &IssueDetails,
        include_comments: bool,
        include_attachments: bool,
        include_history: bool,
    ) -> usize {
        let mut size = 0;

        // Base issue info (rough estimate)
        size += 1000; // Base issue fields
        size += issue_details.issue_info.summary.len() * 2; // UTF-8 overhead
        size += issue_details
            .issue_info
            .description
            .as_ref()
            .map_or(0, |d| d.len() * 2);

        // Comments
        if include_comments {
            if let Some(comments) = &issue_details.comments {
                size += comments.len() * 500; // Rough estimate per comment
                size += comments.iter().map(|c| c.body.len() * 2).sum::<usize>();
            }
        }

        // Attachments metadata
        if include_attachments {
            if let Some(attachments) = &issue_details.attachments {
                size += attachments.len() * 200; // Rough estimate per attachment metadata
            }
        }

        // History
        if include_history {
            if let Some(history) = &issue_details.history {
                size += history.len() * 300; // Rough estimate per history entry
            }
        }

        size
    }

    /// Extract issue project key from issue key
    pub fn extract_project_key(&self, issue_key: &str) -> Option<String> {
        issue_key.split('-').next().map(|s| s.to_uppercase())
    }

    /// Check if issue key format is valid (basic check)
    pub fn is_valid_issue_key_format(&self, key: &str) -> bool {
        if key.trim().is_empty() {
            return false;
        }

        let parts: Vec<&str> = key.split('-').collect();
        if parts.len() < 2 {
            return false;
        }

        let project_part = parts[0];
        let number_part = parts[1];

        !project_part.is_empty()
            && project_part.chars().all(|c| c.is_alphabetic())
            && !number_part.is_empty()
            && number_part.chars().all(|c| c.is_numeric())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::config::JiraConfig;

    #[allow(dead_code)]
    fn create_test_params() -> GetIssueDetailsParams {
        GetIssueDetailsParams {
            issue_key: "PROJ-123".to_string(),
            include_comments: Some(true),
            include_attachments: Some(true),
            include_history: Some(false),
        }
    }

    // Tests disabled due to unsafe std::mem::zeroed usage
    // TODO: Implement proper mocking for tests
    /*
    #[test]
    fn test_param_validation_success() {
        // Disabled: unsafe test
    }

    #[test]
    fn test_param_validation_empty_key() {
        // Disabled: unsafe test
    }
    */

    /*
    // All tests disabled due to unsafe std::mem::zeroed usage
    // TODO: Implement proper mocking for tests
     */
}
