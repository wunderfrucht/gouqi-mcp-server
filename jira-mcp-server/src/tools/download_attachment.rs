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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

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

    /// Optional: Save attachment to filesystem path (optional)
    /// If provided, attachment will be saved to this path.
    /// Path must be relative to current working directory for security.
    /// Example: "downloads/attachment.pdf"
    pub save_to_path: Option<String>,

    /// Whether to return content in response (optional, default: true if save_to_path not set)
    /// Set to false when save_to_path is used to avoid returning large content
    pub return_content: Option<bool>,
}

/// Result from the download_attachment tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadAttachmentResult {
    /// Attachment metadata
    pub attachment_info: AttachmentMetadata,

    /// File content (base64 encoded if base64_encoded=true)
    /// Will be empty if return_content=false
    pub content: Option<String>,

    /// Whether content is base64 encoded
    pub is_base64_encoded: bool,

    /// Path where file was saved (if save_to_path was provided)
    pub saved_to_path: Option<String>,

    /// Performance information
    pub performance: DownloadPerformance,

    /// Success message
    pub message: String,
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

        // Determine if we should return content
        let should_save = params.save_to_path.is_some();
        let should_return_content = params.return_content.unwrap_or(!should_save);

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

        // Download the actual content (raw bytes)
        let content_bytes = self.download_content(&params.attachment_id).await?;
        api_calls += 1;

        let bytes_downloaded = content_bytes.len() as u64;

        // Save to filesystem if requested
        let saved_path = if let Some(save_path) = &params.save_to_path {
            let validated_path = self.validate_and_prepare_save_path(save_path)?;
            std::fs::write(&validated_path, &content_bytes).map_err(|e| {
                JiraMcpError::internal(format!(
                    "Failed to save attachment to '{}': {}",
                    validated_path.display(),
                    e
                ))
            })?;
            info!("Saved attachment to: {}", validated_path.display());
            Some(validated_path.to_string_lossy().to_string())
        } else {
            None
        };

        // Encode content if needed for return
        let content = if should_return_content {
            if base64_encoded {
                Some(general_purpose::STANDARD.encode(&content_bytes))
            } else {
                // Try to convert to UTF-8 string
                Some(
                    String::from_utf8(content_bytes.clone()).unwrap_or_else(|_| {
                        // If not valid UTF-8, return base64 anyway
                        general_purpose::STANDARD.encode(&content_bytes)
                    }),
                )
            }
        } else {
            None
        };

        let duration = start_time.elapsed();
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

        // Build success message
        let message = match (&saved_path, should_return_content) {
            (Some(path), true) => {
                format!(
                    "Downloaded attachment '{}' ({} bytes) and saved to '{}'",
                    attachment_metadata.filename, bytes_downloaded, path
                )
            }
            (Some(path), false) => {
                format!(
                    "Downloaded and saved attachment '{}' ({} bytes) to '{}'",
                    attachment_metadata.filename, bytes_downloaded, path
                )
            }
            (None, true) => {
                format!(
                    "Downloaded attachment '{}' ({} bytes)",
                    attachment_metadata.filename, bytes_downloaded
                )
            }
            (None, false) => "Downloaded attachment (content not returned)".to_string(),
        };

        Ok(DownloadAttachmentResult {
            attachment_info: attachment_metadata,
            content,
            is_base64_encoded: base64_encoded,
            saved_to_path: saved_path,
            performance: DownloadPerformance {
                duration_ms: duration.as_millis() as u64,
                api_calls,
                bytes_downloaded,
                download_speed_bps,
            },
            message,
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

    /// Get attachment metadata using gouqi API
    async fn get_attachment_metadata(
        &self,
        attachment_id: &str,
    ) -> JiraMcpResult<AttachmentMetadata> {
        // Use gouqi's attachments().get() to fetch metadata
        let attachment = self
            .jira_client
            .client
            .attachments()
            .get(attachment_id)
            .await?;

        Ok(AttachmentMetadata {
            id: attachment_id.to_string(),
            filename: attachment.filename,
            size: attachment.size,
            mime_type: attachment.mime_type,
            author: attachment.author.display_name,
            created: attachment.created,
        })
    }

    /// Download attachment content as raw bytes using gouqi API
    async fn download_content(&self, attachment_id: &str) -> JiraMcpResult<Vec<u8>> {
        // Use gouqi's attachments().download() to fetch raw bytes
        // Use spawn_blocking with sync client to work around Send issues
        let jira_url = self.config.jira_url.clone();
        let credentials = self.config.to_gouqi_credentials();
        let id = attachment_id.to_string();

        let content_bytes = tokio::task::spawn_blocking(move || {
            let sync_client = gouqi::Jira::new(&jira_url, credentials)?;
            sync_client.attachments().download(&id)
        })
        .await
        .map_err(|e| JiraMcpError::internal(format!("Task join error: {}", e)))??;

        Ok(content_bytes)
    }

    /// Validate and prepare filesystem path for saving
    fn validate_and_prepare_save_path(&self, path_str: &str) -> JiraMcpResult<PathBuf> {
        let path = Path::new(path_str);

        // Reject absolute paths for security
        if path.is_absolute() {
            return Err(JiraMcpError::invalid_param(
                "save_to_path",
                "Absolute paths are not allowed. Use relative paths only for security.",
            ));
        }

        // Reject paths with parent directory traversal
        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                return Err(JiraMcpError::invalid_param(
                    "save_to_path",
                    "Path traversal (..) is not allowed for security.",
                ));
            }
        }

        // Get current working directory
        let cwd = std::env::current_dir().map_err(|e| {
            JiraMcpError::internal(format!("Failed to get current directory: {}", e))
        })?;

        // Join with current directory
        let full_path = cwd.join(path);

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                JiraMcpError::internal(format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        Ok(full_path)
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
            save_to_path: None,
            return_content: Some(true),
        }
    }

    // Tests disabled due to complexity of mocking
    // TODO: Implement proper mocking for tests
}
