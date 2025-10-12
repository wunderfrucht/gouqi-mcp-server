//! Issue Links tools for JIRA
//!
//! Provides tools for managing links between JIRA issues, including creating,
//! deleting, and retrieving link types.

use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use gouqi::CreateIssueLinkInput;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for the link_issues tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkIssuesParams {
    /// The issue key that is the "source" of the link (required)
    /// Example: "PROJ-123"
    pub inward_issue_key: String,

    /// The issue key that is the "target" of the link (required)
    /// Example: "PROJ-456"
    pub outward_issue_key: String,

    /// The type of link (required)
    /// Examples: "Blocks", "Relates", "Duplicates", "Clones"
    /// Use get_issue_link_types to see available link types
    pub link_type: String,

    /// Optional comment to add when creating the link
    pub comment: Option<String>,
}

/// Result from the link_issues tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkIssuesResult {
    /// Whether the operation was successful
    pub success: bool,

    /// The inward issue key
    pub inward_issue: String,

    /// The outward issue key
    pub outward_issue: String,

    /// The link type used
    pub link_type: String,

    /// Success message
    pub message: String,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for LinkIssuesResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "{{\"error\": \"Failed to serialize LinkIssuesResult\"}}"),
        }
    }
}

/// Parameters for the delete_issue_link tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteIssueLinkParams {
    /// The ID of the link to delete (required)
    /// This is the link ID, not an issue key
    /// Example: "10001"
    pub link_id: String,
}

/// Result from the delete_issue_link tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteIssueLinkResult {
    /// Whether the operation was successful
    pub success: bool,

    /// The link ID that was deleted
    pub link_id: String,

    /// Success message
    pub message: String,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for DeleteIssueLinkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize DeleteIssueLinkResult\"}}"
            ),
        }
    }
}

/// Result from the get_issue_link_types tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIssueLinkTypesResult {
    /// List of available link types
    pub link_types: Vec<IssueLinkTypeInfo>,

    /// Total number of link types
    pub total: usize,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for GetIssueLinkTypesResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize GetIssueLinkTypesResult\"}}"
            ),
        }
    }
}

/// Information about an issue link type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLinkTypeInfo {
    /// Link type ID
    pub id: String,

    /// Link type name (use this when creating links)
    pub name: String,

    /// Description when viewing from the inward issue
    /// Example: "is blocked by"
    pub inward: String,

    /// Description when viewing from the outward issue
    /// Example: "blocks"
    pub outward: String,

    /// Direct link to the link type
    pub self_link: String,
}

/// Tool for linking issues
pub struct LinkIssuesTool {
    jira_client: Arc<JiraClient>,
}

impl LinkIssuesTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: LinkIssuesParams) -> JiraMcpResult<LinkIssuesResult> {
        info!(
            "Linking issues {} -> {} with type '{}'",
            params.inward_issue_key, params.outward_issue_key, params.link_type
        );

        // Validate parameters
        if params.inward_issue_key.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "inward_issue_key",
                "Inward issue key is required",
            ));
        }

        if params.outward_issue_key.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "outward_issue_key",
                "Outward issue key is required",
            ));
        }

        if params.link_type.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "link_type",
                "Link type is required",
            ));
        }

        // Create the link using gouqi
        let mut link_input = CreateIssueLinkInput::new(
            &params.link_type,
            &params.inward_issue_key,
            &params.outward_issue_key,
        );

        if let Some(comment) = &params.comment {
            link_input = link_input.with_comment(comment);
        }

        self.jira_client
            .client
            .issue_links()
            .create(link_input)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    JiraMcpError::not_found(
                        "issue or link type",
                        &format!(
                            "{}, {}, or {}",
                            params.inward_issue_key, params.outward_issue_key, params.link_type
                        ),
                    )
                } else {
                    JiraMcpError::internal(format!("Failed to create issue link: {}", e))
                }
            })?;

        let message = format!(
            "Successfully linked {} to {} with '{}' link type",
            params.inward_issue_key, params.outward_issue_key, params.link_type
        );

        info!("{}", message);

        Ok(LinkIssuesResult {
            success: true,
            inward_issue: params.inward_issue_key,
            outward_issue: params.outward_issue_key,
            link_type: params.link_type,
            message,
        })
    }
}

/// Tool for deleting issue links
pub struct DeleteIssueLinkTool {
    jira_client: Arc<JiraClient>,
}

impl DeleteIssueLinkTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: DeleteIssueLinkParams,
    ) -> JiraMcpResult<DeleteIssueLinkResult> {
        info!("Deleting issue link {}", params.link_id);

        if params.link_id.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "link_id",
                "Link ID is required",
            ));
        }

        self.jira_client
            .client
            .issue_links()
            .delete(&params.link_id)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    JiraMcpError::not_found("issue link", &params.link_id)
                } else {
                    JiraMcpError::internal(format!("Failed to delete issue link: {}", e))
                }
            })?;

        let message = format!("Successfully deleted issue link {}", params.link_id);

        info!("{}", message);

        Ok(DeleteIssueLinkResult {
            success: true,
            link_id: params.link_id,
            message,
        })
    }
}

/// Tool for getting available issue link types
pub struct GetIssueLinkTypesTool {
    jira_client: Arc<JiraClient>,
}

impl GetIssueLinkTypesTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self) -> JiraMcpResult<GetIssueLinkTypesResult> {
        info!("Getting issue link types");

        // Make direct API call to get link types
        // gouqi doesn't have a dedicated method for this, so we'll use the raw client
        let response: serde_json::Value = self
            .jira_client
            .client
            .get("api", "/issueLinkType")
            .await
            .map_err(|e| {
                JiraMcpError::internal(format!("Failed to get issue link types: {}", e))
            })?;

        // Parse the response
        let link_types_array = response
            .get("issueLinkTypes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                JiraMcpError::internal("Invalid response format from JIRA API".to_string())
            })?;

        let link_types: Vec<IssueLinkTypeInfo> = link_types_array
            .iter()
            .filter_map(|lt| {
                Some(IssueLinkTypeInfo {
                    id: lt.get("id")?.as_str()?.to_string(),
                    name: lt.get("name")?.as_str()?.to_string(),
                    inward: lt.get("inward")?.as_str()?.to_string(),
                    outward: lt.get("outward")?.as_str()?.to_string(),
                    self_link: lt.get("self")?.as_str()?.to_string(),
                })
            })
            .collect();

        let total = link_types.len();

        info!("Found {} issue link types", total);

        Ok(GetIssueLinkTypesResult { link_types, total })
    }
}
