//! Upload attachment tool for adding files to JIRA issues
//!
//! This tool uploads files as attachments to a specific JIRA issue.
//! Supports both inline base64 content and reading from filesystem.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use base64::{engine::general_purpose, Engine as _};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// Parameters for the upload_attachment tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadAttachmentParams {
    /// JIRA issue key to attach files to (required)
    /// Examples: "PROJ-123", "KEY-456"
    pub issue_key: String,

    /// Files to upload as inline base64 content (optional)
    /// Provide either this OR file_paths, not both
    pub files: Option<Vec<FileContent>>,

    /// Paths to files to upload from filesystem (optional)
    /// Provide either this OR files, not both
    /// Paths must be relative to current working directory for security
    pub file_paths: Option<Vec<String>>,

    /// Maximum total size for all uploads in bytes (optional, default: 10MB)
    pub max_total_size_bytes: Option<u64>,
}

/// Inline file content
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileContent {
    /// Filename
    pub filename: String,

    /// File content as base64 encoded string
    pub content_base64: String,

    /// Optional MIME type (will be inferred from filename if not provided)
    pub mime_type: Option<String>,
}

/// Result from the upload_attachment tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadAttachmentResult {
    /// Successfully uploaded attachments
    pub uploaded_attachments: Vec<UploadedAttachmentInfo>,

    /// Issue key that attachments were added to
    pub issue_key: String,

    /// Total number of files uploaded
    pub total_count: usize,

    /// Total bytes uploaded
    pub total_bytes: u64,

    /// Performance information
    pub performance: UploadPerformance,

    /// Success message
    pub message: String,
}

/// Information about an uploaded attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedAttachmentInfo {
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
}

/// Performance metrics for upload operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadPerformance {
    /// Time taken for the upload in milliseconds
    pub duration_ms: u64,

    /// Number of JIRA API calls made
    pub api_calls: u32,

    /// Total bytes uploaded
    pub bytes_uploaded: u64,

    /// Upload speed in bytes per second
    pub upload_speed_bps: u64,
}

/// Implementation of the upload_attachment tool
pub struct UploadAttachmentTool {
    #[allow(dead_code)]
    jira_client: Arc<JiraClient>,
    config: Arc<JiraConfig>,
    #[allow(dead_code)]
    cache: Arc<MetadataCache>,
}

impl UploadAttachmentTool {
    /// Create a new upload attachment tool
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

    /// Execute the upload_attachment tool
    pub async fn execute(
        &self,
        params: UploadAttachmentParams,
    ) -> JiraMcpResult<UploadAttachmentResult> {
        let start_time = std::time::Instant::now();
        let mut api_calls = 0u32;

        info!(
            "Executing upload_attachment tool for issue: {}",
            params.issue_key
        );

        // Validate parameters
        self.validate_params(&params)?;

        let max_total_size = params.max_total_size_bytes.unwrap_or(10 * 1024 * 1024); // 10MB default

        // Prepare files for upload
        let files_to_upload = self.prepare_files(&params, max_total_size)?;

        if files_to_upload.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "files",
                "No files provided for upload. Provide either 'files' or 'file_paths'.",
            ));
        }

        let total_count = files_to_upload.len();
        let total_bytes: u64 = files_to_upload
            .iter()
            .map(|(_, bytes)| bytes.len() as u64)
            .sum();

        info!(
            "Uploading {} file(s) ({} bytes total) to issue {}",
            total_count, total_bytes, params.issue_key
        );

        // Upload using gouqi's upload_attachment API
        // Use spawn_blocking with sync client to work around Send issues
        let jira_url = self.config.jira_url.clone();
        let credentials = self.config.to_gouqi_credentials();
        let issue_key = params.issue_key.clone();

        let uploaded = tokio::task::spawn_blocking(move || {
            let sync_client = gouqi::Jira::new(&jira_url, credentials)?;

            // Convert Vec<(String, Vec<u8>)> to Vec<(&str, Vec<u8>)> for gouqi API
            let files_for_upload: Vec<(&str, Vec<u8>)> = files_to_upload
                .iter()
                .map(|(name, bytes)| (name.as_str(), bytes.clone()))
                .collect();

            sync_client
                .issues()
                .upload_attachment(&issue_key, files_for_upload)
        })
        .await
        .map_err(|e| JiraMcpError::internal(format!("Task join error: {}", e)))??;

        api_calls += 1;

        // Convert response to our format
        let uploaded_attachments: Vec<UploadedAttachmentInfo> = uploaded
            .into_iter()
            .map(|att| UploadedAttachmentInfo {
                id: "".to_string(), // AttachmentResponse doesn't have id
                filename: att.filename,
                size: att.size,
                mime_type: att.mime_type,
                author: att.author.display_name,
                created: att.created,
                content_url: att.content,
            })
            .collect();

        let duration = start_time.elapsed();
        let upload_speed_bps = if duration.as_secs() > 0 {
            total_bytes / duration.as_secs()
        } else {
            total_bytes
        };

        info!(
            "Successfully uploaded {} attachments ({} bytes) in {}ms",
            total_count,
            total_bytes,
            duration.as_millis()
        );

        // Warn about large uploads
        if total_bytes > 5 * 1024 * 1024 {
            // 5MB
            warn!(
                "Uploaded large attachments ({:.2} MB total)",
                total_bytes as f64 / (1024.0 * 1024.0)
            );
        }

        let message = format!(
            "Successfully uploaded {} file(s) ({} bytes) to issue '{}'",
            total_count, total_bytes, params.issue_key
        );

        Ok(UploadAttachmentResult {
            uploaded_attachments,
            issue_key: params.issue_key,
            total_count,
            total_bytes,
            performance: UploadPerformance {
                duration_ms: duration.as_millis() as u64,
                api_calls,
                bytes_uploaded: total_bytes,
                upload_speed_bps,
            },
            message,
        })
    }

    /// Validate upload attachment parameters
    fn validate_params(&self, params: &UploadAttachmentParams) -> JiraMcpResult<()> {
        // Validate issue key
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

        // Validate that either files or file_paths is provided, but not both
        match (&params.files, &params.file_paths) {
            (None, None) => {
                return Err(JiraMcpError::invalid_param(
                    "files",
                    "Either 'files' or 'file_paths' must be provided",
                ));
            }
            (Some(_), Some(_)) => {
                return Err(JiraMcpError::invalid_param(
                    "files",
                    "Cannot provide both 'files' and 'file_paths'. Choose one approach.",
                ));
            }
            _ => {}
        }

        // Validate max size
        if let Some(max_size) = params.max_total_size_bytes {
            if max_size == 0 {
                return Err(JiraMcpError::invalid_param(
                    "max_total_size_bytes",
                    "max_total_size_bytes must be greater than 0",
                ));
            }
            if max_size > 100 * 1024 * 1024 {
                // 100MB limit
                return Err(JiraMcpError::invalid_param(
                    "max_total_size_bytes",
                    "max_total_size_bytes cannot exceed 100MB to prevent memory issues",
                ));
            }
        }

        Ok(())
    }

    /// Prepare files for upload (from inline content or filesystem)
    fn prepare_files(
        &self,
        params: &UploadAttachmentParams,
        max_total_size: u64,
    ) -> JiraMcpResult<Vec<(String, Vec<u8>)>> {
        let mut files_to_upload: Vec<(String, Vec<u8>)> = Vec::new();
        let mut total_size: u64 = 0;

        // Handle inline files
        if let Some(files) = &params.files {
            for file in files {
                // Decode base64 content
                let bytes = general_purpose::STANDARD
                    .decode(&file.content_base64)
                    .map_err(|e| {
                        JiraMcpError::invalid_param(
                            "content_base64",
                            format!("Invalid base64 content for file '{}': {}", file.filename, e),
                        )
                    })?;

                total_size += bytes.len() as u64;
                if total_size > max_total_size {
                    return Err(JiraMcpError::invalid_param(
                        "files",
                        format!(
                            "Total upload size ({} bytes) exceeds maximum ({} bytes)",
                            total_size, max_total_size
                        ),
                    ));
                }

                files_to_upload.push((file.filename.clone(), bytes));
            }
        }

        // Handle file paths
        if let Some(file_paths) = &params.file_paths {
            for path_str in file_paths {
                let (filename, bytes) = self.read_file_from_path(path_str)?;

                total_size += bytes.len() as u64;
                if total_size > max_total_size {
                    return Err(JiraMcpError::invalid_param(
                        "file_paths",
                        format!(
                            "Total upload size ({} bytes) exceeds maximum ({} bytes)",
                            total_size, max_total_size
                        ),
                    ));
                }

                files_to_upload.push((filename, bytes));
            }
        }

        Ok(files_to_upload)
    }

    /// Read file from filesystem with security validation
    fn read_file_from_path(&self, path_str: &str) -> JiraMcpResult<(String, Vec<u8>)> {
        let path = Path::new(path_str);

        // Reject absolute paths for security
        if path.is_absolute() {
            return Err(JiraMcpError::invalid_param(
                "file_paths",
                "Absolute paths are not allowed. Use relative paths only for security.",
            ));
        }

        // Reject paths with parent directory traversal
        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                return Err(JiraMcpError::invalid_param(
                    "file_paths",
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

        // Verify file exists
        if !full_path.exists() {
            return Err(JiraMcpError::invalid_param(
                "file_paths",
                format!("File not found: '{}'", path_str),
            ));
        }

        // Verify it's a file (not a directory)
        if !full_path.is_file() {
            return Err(JiraMcpError::invalid_param(
                "file_paths",
                format!("Path is not a file: '{}'", path_str),
            ));
        }

        // Read file content
        let bytes = std::fs::read(&full_path).map_err(|e| {
            JiraMcpError::internal(format!(
                "Failed to read file '{}': {}",
                full_path.display(),
                e
            ))
        })?;

        // Extract filename
        let filename = full_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                JiraMcpError::internal(format!(
                    "Failed to extract filename from path: '{}'",
                    path_str
                ))
            })?
            .to_string();

        Ok((filename, bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn create_test_params() -> UploadAttachmentParams {
        UploadAttachmentParams {
            issue_key: "PROJ-123".to_string(),
            files: Some(vec![FileContent {
                filename: "test.txt".to_string(),
                content_base64: general_purpose::STANDARD.encode(b"test content"),
                mime_type: Some("text/plain".to_string()),
            }]),
            file_paths: None,
            max_total_size_bytes: Some(1024 * 1024), // 1MB
        }
    }

    // Tests disabled due to complexity of mocking
    // TODO: Implement proper mocking for tests
}
