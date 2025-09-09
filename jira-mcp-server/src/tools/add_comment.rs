//! Add comment tool for adding comments to JIRA issues
//!
//! This tool allows AI agents to add comments to JIRA issues with
//! simple parameters and comprehensive error handling.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::{CommentInfo, JiraClient};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for the add_comment tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddCommentParams {
    /// JIRA issue key (required)
    /// Examples: "PROJ-123", "KEY-456"
    pub issue_key: String,

    /// Comment body text (required)
    /// The text content of the comment to add
    pub comment_body: String,

    /// Visibility restriction (optional)
    /// Can be used to restrict comment visibility to specific groups or roles
    pub visibility: Option<String>,
}

/// Result from the add_comment tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCommentResult {
    /// The created comment information
    pub comment: CommentInfo,

    /// Issue key that was commented on
    pub issue_key: String,

    /// Success message
    pub message: String,

    /// Performance information
    pub performance: CommentPerformance,
}

/// Performance metrics for comment operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPerformance {
    /// Time taken for the operation in milliseconds
    pub duration_ms: u64,

    /// Number of JIRA API calls made
    pub api_calls: u32,

    /// Whether the operation succeeded
    pub success: bool,
}

/// Implementation of the add_comment tool
pub struct AddCommentTool {
    jira_client: Arc<JiraClient>,
    #[allow(dead_code)]
    config: Arc<JiraConfig>,
    #[allow(dead_code)]
    cache: Arc<MetadataCache>,
}

impl AddCommentTool {
    /// Create a new add comment tool
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

    /// Execute the add_comment tool
    #[instrument(skip(self), fields(
        issue_key = params.issue_key.as_str(),
        comment_length = params.comment_body.len(),
    ))]
    pub async fn execute(&self, params: AddCommentParams) -> JiraMcpResult<AddCommentResult> {
        let start_time = std::time::Instant::now();
        let mut api_calls = 0u32;

        info!("Adding comment to issue: {}", params.issue_key);

        // Validate parameters
        self.validate_params(&params)?;

        // Add the comment using the JIRA client
        let comment = self
            .jira_client
            .add_comment(&params.issue_key, &params.comment_body)
            .await?;

        api_calls += 1;
        let duration = start_time.elapsed();

        info!(
            "Comment added successfully to issue {} in {}ms",
            params.issue_key,
            duration.as_millis()
        );

        Ok(AddCommentResult {
            comment,
            issue_key: params.issue_key.clone(),
            message: format!("Comment successfully added to issue {}", params.issue_key),
            performance: CommentPerformance {
                duration_ms: duration.as_millis() as u64,
                api_calls,
                success: true,
            },
        })
    }

    /// Validate add comment parameters
    fn validate_params(&self, params: &AddCommentParams) -> JiraMcpResult<()> {
        // Validate issue key
        if params.issue_key.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key cannot be empty",
            ));
        }

        // Validate issue key format (basic check)
        if !params.issue_key.contains('-') {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key must be in format 'PROJECT-NUMBER' (e.g., 'PROJ-123')",
            ));
        }

        // Validate comment body
        if params.comment_body.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "comment_body",
                "Comment body cannot be empty",
            ));
        }

        // Check comment length (JIRA has limits)
        if params.comment_body.len() > 32_768 {
            return Err(JiraMcpError::invalid_param(
                "comment_body",
                "Comment body cannot exceed 32,768 characters",
            ));
        }

        // Validate visibility if provided
        if let Some(visibility) = &params.visibility {
            if visibility.trim().is_empty() {
                return Err(JiraMcpError::invalid_param(
                    "visibility",
                    "Visibility cannot be empty if provided",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(dead_code)]
    fn create_test_params() -> AddCommentParams {
        AddCommentParams {
            issue_key: "TEST-123".to_string(),
            comment_body: "This is a test comment".to_string(),
            visibility: None,
        }
    }

    // Tests disabled due to unsafe mock usage
    // TODO: Implement proper mocking for tests
}
