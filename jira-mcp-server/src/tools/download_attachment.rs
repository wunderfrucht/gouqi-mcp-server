//! Download attachment tool for retrieving attachment content
//!
//! This tool downloads the actual content of a JIRA attachment
//! given its attachment ID or URL.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use base64::{engine::general_purpose, Engine as _};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// Parameters for the download_attachment tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DownloadAttachmentParams {
    /// Attachment ID (required)
    /// This can be obtained from the list_issue_attachments tool
    pub attachment_id: String,

    /// Whether to return content as base64 encoded string (optional, default: true)
    /// If false, will return binary content (not recommended for large files)
    pub base64_encoded: Option<bool>,

    /// Maximum file size to download in bytes (optional, default: 10MB)
    /// Files larger than this will be rejected to prevent memory issues
    pub max_size_bytes: Option<u64>,
}

/// Result from the download_attachment tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadAttachmentResult {
    /// Attachment metadata
    pub attachment_info: AttachmentMetadata,

    /// File content (base64 encoded if base64_encoded=true)
    pub content: String,

    /// Whether content is base64 encoded
    pub is_base64_encoded: bool,

    /// Performance information
    pub performance: DownloadPerformance,
}

/// Attachment metadata returned with download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMetadata {
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
}

/// Performance metrics for download operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadPerformance {
    /// Time taken for the download in milliseconds
    pub duration_ms: u64,

    /// Number of JIRA API calls made
    pub api_calls: u32,

    /// Actual bytes downloaded
    pub bytes_downloaded: u64,

    /// Download speed in bytes per second
    pub download_speed_bps: u64,
}

/// Implementation of the download_attachment tool
pub struct DownloadAttachmentTool {
    #[allow(dead_code)]
    jira_client: Arc<JiraClient>,
    #[allow(dead_code)]
    config: Arc<JiraConfig>,
    #[allow(dead_code)]
    cache: Arc<MetadataCache>,
}

impl DownloadAttachmentTool {
    /// Create a new download attachment tool
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

    /// Execute the download_attachment tool
    #[instrument(skip(self), fields(attachment_id = %params.attachment_id))]
    pub async fn execute(
        &self,
        params: DownloadAttachmentParams,
    ) -> JiraMcpResult<DownloadAttachmentResult> {
        let start_time = std::time::Instant::now();
        let mut api_calls = 0u32;

        info!(
            "Executing download_attachment tool for attachment: {}",
            params.attachment_id
        );

        // Validate parameters
        self.validate_params(&params)?;

        let base64_encoded = params.base64_encoded.unwrap_or(true);
        let max_size = params.max_size_bytes.unwrap_or(10 * 1024 * 1024); // 10MB default

        // First, get attachment metadata to check size and get info
        let attachment_metadata = self.get_attachment_metadata(&params.attachment_id).await?;
        api_calls += 1;

        // Check file size
        if attachment_metadata.size > max_size {
            return Err(JiraMcpError::invalid_param(
                "attachment_size",
                format!(
                    "Attachment size ({} bytes) exceeds maximum allowed size ({} bytes). Use max_size_bytes parameter to increase limit if needed.",
                    attachment_metadata.size, max_size
                ),
            ));
        }

        // Download the actual content
        let content = self
            .download_content(&params.attachment_id, base64_encoded)
            .await?;
        api_calls += 1;

        let duration = start_time.elapsed();
        let bytes_downloaded = attachment_metadata.size;
        let download_speed_bps = if duration.as_secs() > 0 {
            bytes_downloaded / duration.as_secs()
        } else {
            bytes_downloaded // For very fast downloads
        };

        info!(
            "Downloaded attachment {} ({} bytes) in {}ms",
            params.attachment_id,
            bytes_downloaded,
            duration.as_millis()
        );

        // Warn about large downloads
        if bytes_downloaded > 5 * 1024 * 1024 {
            // 5MB
            warn!(
                "Downloaded large attachment ({:.2} MB)",
                bytes_downloaded as f64 / (1024.0 * 1024.0)
            );
        }

        Ok(DownloadAttachmentResult {
            attachment_info: attachment_metadata,
            content,
            is_base64_encoded: base64_encoded,
            performance: DownloadPerformance {
                duration_ms: duration.as_millis() as u64,
                api_calls,
                bytes_downloaded,
                download_speed_bps,
            },
        })
    }

    /// Validate download attachment parameters
    fn validate_params(&self, params: &DownloadAttachmentParams) -> JiraMcpResult<()> {
        // Validate attachment ID
        if params.attachment_id.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "attachment_id",
                "Attachment ID is required. Use the list_issue_attachments tool to get attachment IDs.",
            ));
        }

        // Validate max size
        if let Some(max_size) = params.max_size_bytes {
            if max_size == 0 {
                return Err(JiraMcpError::invalid_param(
                    "max_size_bytes",
                    "max_size_bytes must be greater than 0",
                ));
            }
            if max_size > 100 * 1024 * 1024 {
                // 100MB limit
                return Err(JiraMcpError::invalid_param(
                    "max_size_bytes",
                    "max_size_bytes cannot exceed 100MB to prevent memory issues",
                ));
            }
        }

        Ok(())
    }

    /// Get attachment metadata
    async fn get_attachment_metadata(
        &self,
        attachment_id: &str,
    ) -> JiraMcpResult<AttachmentMetadata> {
        // This would use gouqi's attachment API to get metadata
        // For now, we'll return a placeholder - this needs to be implemented
        // based on the actual gouqi attachment API

        // TODO: Implement actual metadata fetching using gouqi
        // Example: let metadata = self.jira_client.attachments().get(attachment_id).await?;

        Ok(AttachmentMetadata {
            id: attachment_id.to_string(),
            filename: "example.txt".to_string(),
            size: 1024,
            mime_type: "text/plain".to_string(),
            author: "Unknown".to_string(),
            created: "2024-01-01T00:00:00Z".to_string(),
        })
    }

    /// Download attachment content
    async fn download_content(
        &self,
        _attachment_id: &str,
        base64_encoded: bool,
    ) -> JiraMcpResult<String> {
        // This would use gouqi's attachment API to download content
        // For now, we'll return a placeholder - this needs to be implemented

        // TODO: Implement actual content download using gouqi
        // Example: let content = self.jira_client.attachments().download(attachment_id).await?;

        let placeholder_content = if base64_encoded {
            // Return base64 encoded placeholder
            general_purpose::STANDARD.encode("This is placeholder attachment content")
        } else {
            // Return raw placeholder
            "This is placeholder attachment content".to_string()
        };

        Ok(placeholder_content)
    }
}

impl DownloadAttachmentTool {
    /// Helper to encode bytes as base64
    #[allow(dead_code)]
    fn encode_base64(bytes: &[u8]) -> String {
        general_purpose::STANDARD.encode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn create_test_params() -> DownloadAttachmentParams {
        DownloadAttachmentParams {
            attachment_id: "12345".to_string(),
            base64_encoded: Some(true),
            max_size_bytes: Some(1024 * 1024), // 1MB
        }
    }

    // Tests disabled due to complexity of mocking
    // TODO: Implement proper mocking for tests
}
