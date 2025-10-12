use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for managing issue labels
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ManageLabelsParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    pub issue_key: String,

    /// Labels to add to the issue
    #[serde(default)]
    pub add_labels: Option<Vec<String>>,

    /// Labels to remove from the issue
    #[serde(default)]
    pub remove_labels: Option<Vec<String>>,

    /// If true, replaces all existing labels with add_labels (ignores remove_labels)
    #[serde(default)]
    pub replace_all: bool,
}

/// Result from managing labels
#[derive(Debug, Serialize, JsonSchema)]
pub struct ManageLabelsResult {
    /// The issue key
    pub issue_key: String,

    /// Labels that were added
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<String>,

    /// Labels that were removed
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,

    /// Current labels on the issue after the operation
    pub current_labels: Vec<String>,

    /// Success message
    pub message: String,
}

/// Parameters for getting available labels
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAvailableLabelsParams {
    /// Optional project key to filter labels by project (searches issues in project)
    #[serde(default)]
    pub project_key: Option<String>,

    /// Maximum number of labels to return (default: 1000)
    #[serde(default)]
    pub max_results: Option<u32>,

    /// Starting index for pagination (default: 0)
    #[serde(default)]
    pub start_at: Option<u32>,
}

/// Result from getting available labels
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetAvailableLabelsResult {
    /// List of available labels
    pub labels: Vec<String>,

    /// Total number of labels found
    pub total: u32,

    /// Starting index
    pub start_at: u32,

    /// Maximum results per page
    pub max_results: u32,

    /// Whether this is the last page
    pub is_last: bool,
}

/// Tool for managing JIRA issue labels
pub struct LabelsTool {
    jira_client: Arc<JiraClient>,
}

impl LabelsTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn manage_labels(
        &self,
        params: ManageLabelsParams,
    ) -> JiraMcpResult<ManageLabelsResult> {
        info!(
            "Managing labels for issue {}: add={:?}, remove={:?}, replace={}",
            params.issue_key, params.add_labels, params.remove_labels, params.replace_all
        );

        // Validate that at least one operation is specified
        if !params.replace_all
            && params.add_labels.as_ref().is_none_or(|v| v.is_empty())
            && params.remove_labels.as_ref().is_none_or(|v| v.is_empty())
        {
            return Err(JiraMcpError::invalid_param(
                "labels",
                "Must specify at least one label to add or remove",
            ));
        }

        let added = params.add_labels.clone().unwrap_or_default();
        let removed = params.remove_labels.clone().unwrap_or_default();

        // Build the update payload
        let update_body = if params.replace_all {
            // Replace all labels with the provided list
            serde_json::json!({
                "fields": {
                    "labels": added
                }
            })
        } else {
            // Add and/or remove specific labels
            let mut label_operations = Vec::new();

            if let Some(add_labels) = &params.add_labels {
                for label in add_labels {
                    label_operations.push(serde_json::json!({"add": label}));
                }
            }

            if let Some(remove_labels) = &params.remove_labels {
                for label in remove_labels {
                    label_operations.push(serde_json::json!({"remove": label}));
                }
            }

            serde_json::json!({
                "update": {
                    "labels": label_operations
                }
            })
        };

        // Make the API call
        let endpoint = format!("/issue/{}", params.issue_key);
        self.jira_client
            .client
            .put::<(), _>("api", &endpoint, update_body)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    JiraMcpError::not_found("issue", &params.issue_key)
                } else {
                    JiraMcpError::internal(format!("Failed to update labels: {}", e))
                }
            })?;

        // Fetch the issue to get current labels
        let issue: Value = self
            .jira_client
            .client
            .get("api", &format!("/issue/{}?fields=labels", params.issue_key))
            .await
            .map_err(|e| JiraMcpError::internal(format!("Failed to fetch updated issue: {}", e)))?;

        let current_labels = issue["fields"]["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let message = if params.replace_all {
            format!(
                "Successfully replaced all labels on {} with {} label(s)",
                params.issue_key,
                added.len()
            )
        } else {
            let mut parts = Vec::new();
            if !added.is_empty() {
                parts.push(format!("added {} label(s)", added.len()));
            }
            if !removed.is_empty() {
                parts.push(format!("removed {} label(s)", removed.len()));
            }
            format!(
                "Successfully {} on {}",
                parts.join(" and "),
                params.issue_key
            )
        };

        info!("{}", message);

        Ok(ManageLabelsResult {
            issue_key: params.issue_key,
            added,
            removed,
            current_labels,
            message,
        })
    }

    #[instrument(skip(self))]
    pub async fn get_available_labels(
        &self,
        params: GetAvailableLabelsParams,
    ) -> JiraMcpResult<GetAvailableLabelsResult> {
        let max_results = params.max_results.unwrap_or(1000).min(1000);
        let start_at = params.start_at.unwrap_or(0);

        info!(
            "Getting available labels (project={:?}, max_results={}, start_at={})",
            params.project_key, max_results, start_at
        );

        if let Some(project_key) = &params.project_key {
            // Get labels specific to a project by searching issues
            let jql = format!("project = {} AND labels is not EMPTY", project_key);
            // URL encode the JQL manually
            let encoded_jql = jql.replace(" ", "%20").replace("=", "%3D");
            let endpoint = format!(
                "/search?jql={}&fields=labels&maxResults={}",
                encoded_jql, max_results
            );

            let response: Value = self
                .jira_client
                .client
                .get("api", &endpoint)
                .await
                .map_err(|e| {
                    JiraMcpError::internal(format!("Failed to search for labels: {}", e))
                })?;

            // Extract unique labels from all issues
            let mut labels_set = std::collections::HashSet::new();
            if let Some(issues) = response["issues"].as_array() {
                for issue in issues {
                    if let Some(labels) = issue["fields"]["labels"].as_array() {
                        for label in labels {
                            if let Some(label_str) = label.as_str() {
                                labels_set.insert(label_str.to_string());
                            }
                        }
                    }
                }
            }

            let mut labels: Vec<String> = labels_set.into_iter().collect();
            labels.sort();

            let total = labels.len() as u32;
            let is_last = true; // We fetch all at once from issues

            Ok(GetAvailableLabelsResult {
                labels,
                total,
                start_at,
                max_results,
                is_last,
            })
        } else {
            // Get global labels using v2 API (v3 doesn't have this endpoint)
            let endpoint = format!("/label?startAt={}&maxResults={}", start_at, max_results);

            let response: Value = self
                .jira_client
                .client
                .get("api", &endpoint)
                .await
                .map_err(|e| {
                    JiraMcpError::internal(format!("Failed to get available labels: {}", e))
                })?;

            let labels: Vec<String> = response["values"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let total = response["total"].as_u64().unwrap_or(labels.len() as u64) as u32;
            let is_last = response["isLast"].as_bool().unwrap_or(true);

            Ok(GetAvailableLabelsResult {
                labels,
                total,
                start_at,
                max_results,
                is_last,
            })
        }
    }
}
