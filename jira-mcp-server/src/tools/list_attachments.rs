//! List attachments tool for retrieving attachment metadata from issues
//!
//! This tool lists all attachments for a specific JIRA issue,
//! returning metadata like filename, size, content type, and download URL.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// Parameters for the list_issue_attachments tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListAttachmentsParams {
    /// JIRA issue key (required)
    /// Examples: "PROJ-123", "KEY-456"
    pub issue_key: String,
}

/// Attachment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    /// Attachment ID
    pub id: String,

    /// Filename
    pub filename: String,

    /// File size in bytes
    pub size: u64,

    /// MIME type
    pub mime_type: String,

    /// Author who uploaded the attachment
    pub author: String,

    /// When the attachment was created
    pub created: String,

    /// Content URL for downloading
    pub content_url: String,

    /// Thumbnail URL (if available)
    pub thumbnail_url: Option<String>,
}

/// Result from the list_issue_attachments tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAttachmentsResult {
    /// List of attachments
    pub attachments: Vec<AttachmentInfo>,

    /// Issue key that was queried
    pub issue_key: String,

    /// Total number of attachments
    pub total_count: usize,

    /// Performance information
    pub performance: AttachmentsPerformance,
}

/// Performance metrics for attachments operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentsPerformance {
    /// Time taken for the operation in milliseconds
    pub duration_ms: u64,

    /// Whether the data hit the cache
    pub cache_hit: bool,

    /// Number of JIRA API calls made
    pub api_calls: u32,
}

/// Implementation of the list_issue_attachments tool
pub struct ListAttachmentsTool {
    jira_client: Arc<JiraClient>,
    #[allow(dead_code)]
    config: Arc<JiraConfig>,
    #[allow(dead_code)]
    cache: Arc<MetadataCache>,
}

impl ListAttachmentsTool {
    /// Create a new list attachments tool
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

    /// Execute the list_issue_attachments tool
    #[instrument(skip(self), fields(issue_key = %params.issue_key))]
    pub async fn execute(
        &self,
        params: ListAttachmentsParams,
    ) -> JiraMcpResult<ListAttachmentsResult> {
        let start_time = std::time::Instant::now();
        let mut api_calls = 0u32;
        let cache_hit = false;

        info!(
            "Executing list_issue_attachments tool for issue: {}",
            params.issue_key
        );

        // Validate parameters
        self.validate_params(&params)?;

        // Normalize issue key
        let normalized_key = params.issue_key.trim().to_uppercase();

        // For now, we'll use the issue details to get attachments
        // In the future, we could add a direct attachment listing method to the client
        let issue_details = self
            .jira_client
            .get_issue_details(&normalized_key, false, true, false)
            .await?;

        api_calls += 1;

        // Extract attachments from issue details
        let attachments = if let Some(issue_attachments) = &issue_details.attachments {
            issue_attachments
                .iter()
                .map(|att| AttachmentInfo {
                    id: att.id.clone(),
                    filename: att.filename.clone(),
                    size: att.size,
                    mime_type: att.mime_type.clone(),
                    author: att.author.clone(),
                    created: att.created.clone(),
                    content_url: format!("jira://attachment/{}", att.id),
                    thumbnail_url: None, // Would need to be implemented based on JIRA API
                })
                .collect()
        } else {
            Vec::new()
        };

        let duration = start_time.elapsed();
        let total_count = attachments.len();

        info!(
            "Found {} attachments for issue {} in {}ms",
            total_count,
            normalized_key,
            duration.as_millis()
        );

        // Warn about large numbers of attachments
        if total_count > 50 {
            warn!(
                "Issue {} has {} attachments, response may be large",
                normalized_key, total_count
            );
        }

        Ok(ListAttachmentsResult {
            attachments,
            issue_key: normalized_key,
            total_count,
            performance: AttachmentsPerformance {
                duration_ms: duration.as_millis() as u64,
                cache_hit,
                api_calls,
            },
        })
    }

    /// Validate list attachments parameters
    fn validate_params(&self, params: &ListAttachmentsParams) -> JiraMcpResult<()> {
        // Validate issue key format
        if params.issue_key.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key is required. Please provide a JIRA issue key (e.g., 'PROJ-123')",
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn create_test_params() -> ListAttachmentsParams {
        ListAttachmentsParams {
            issue_key: "PROJ-123".to_string(),
        }
    }

    // Tests disabled due to complexity of mocking
    // TODO: Implement proper mocking for tests
}
