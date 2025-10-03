//! Issue Transition Tools
//!
//! This module provides tools for managing JIRA issue status changes through transitions.
//! JIRA doesn't allow direct status updates - you must trigger transitions between states.

use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// Parameters for getting available transitions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetAvailableTransitionsParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    pub issue_key: String,
}

/// Information about an available transition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransitionInfo {
    /// Unique identifier for the transition
    pub id: String,

    /// Human-readable name of the transition (e.g., "Start Progress", "Done")
    pub name: String,

    /// The target status this transition leads to
    pub to_status: String,

    /// The target status ID
    pub to_status_id: String,
}

/// Result from get_available_transitions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetAvailableTransitionsResult {
    /// The issue key that was queried
    pub issue_key: String,

    /// Current status of the issue
    pub current_status: String,

    /// List of available transitions
    pub transitions: Vec<TransitionInfo>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Parameters for transitioning an issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TransitionIssueParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    pub issue_key: String,

    /// Transition ID (required if transition_name not provided)
    pub transition_id: Option<String>,

    /// Transition name (alternative to transition_id, e.g., "Start Progress")
    /// If both are provided, transition_id takes precedence
    pub transition_name: Option<String>,

    /// Optional comment to add when transitioning
    pub comment: Option<String>,

    /// Optional resolution name (for transitions that require resolution, e.g., "Done", "Won't Fix")
    pub resolution: Option<String>,
}

/// Result from transition_issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransitionIssueResult {
    /// Whether the transition was successful
    pub success: bool,

    /// The issue key that was transitioned
    pub issue_key: String,

    /// The transition that was executed
    pub transition_used: TransitionInfo,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Gouqi's transition response structures (for deserialization)
#[derive(Debug, Deserialize)]
struct TransitionOptions {
    transitions: Vec<TransitionOption>,
}

#[derive(Debug, Deserialize)]
struct TransitionOption {
    id: String,
    name: String,
    to: TransitionTo,
}

#[derive(Debug, Deserialize)]
struct TransitionTo {
    name: String,
    id: String,
}

/// Gouqi's transition trigger structure (for serialization)
#[derive(Debug, Serialize)]
struct TransitionTriggerOptions {
    transition: Transition,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    fields: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct Transition {
    id: String,
}

/// Tool for getting available transitions
#[derive(Debug)]
pub struct GetAvailableTransitionsTool {
    jira_client: Arc<JiraClient>,
}

impl GetAvailableTransitionsTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: GetAvailableTransitionsParams,
    ) -> JiraMcpResult<GetAvailableTransitionsResult> {
        let start_time = std::time::Instant::now();

        info!(
            "Getting available transitions for issue: {}",
            params.issue_key
        );

        // Validate issue key format
        self.validate_issue_key(&params.issue_key)?;

        // Get current issue status
        let issue = self
            .jira_client
            .client
            .issues()
            .get(&params.issue_key)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    JiraMcpError::not_found("issue", &params.issue_key)
                } else {
                    JiraMcpError::from(e)
                }
            })?;

        let current_status = issue
            .field::<serde_json::Value>("status")
            .and_then(|result| result.ok())
            .and_then(|status_obj| {
                status_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Unknown".to_string());

        // Get available transitions
        let endpoint = format!(
            "/issue/{}/transitions?expand=transitions.fields",
            params.issue_key
        );

        let transition_options: TransitionOptions = self
            .jira_client
            .client
            .get("api", &endpoint)
            .await
            .map_err(JiraMcpError::from)?;

        let transitions = transition_options
            .transitions
            .into_iter()
            .map(|t| TransitionInfo {
                id: t.id,
                name: t.name,
                to_status: t.to.name,
                to_status_id: t.to.id,
            })
            .collect::<Vec<_>>();

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Found {} available transitions for issue {} (current status: {})",
            transitions.len(),
            params.issue_key,
            current_status
        );

        Ok(GetAvailableTransitionsResult {
            issue_key: params.issue_key,
            current_status,
            transitions,
            execution_time_ms: execution_time,
        })
    }

    fn validate_issue_key(&self, issue_key: &str) -> JiraMcpResult<()> {
        if issue_key.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key cannot be empty",
            ));
        }

        let parts: Vec<&str> = issue_key.split('-').collect();
        if parts.len() != 2 {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key must be in format 'PROJECT-123'",
            ));
        }

        Ok(())
    }
}

/// Tool for transitioning an issue
#[derive(Debug)]
pub struct TransitionIssueTool {
    jira_client: Arc<JiraClient>,
}

impl TransitionIssueTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: TransitionIssueParams,
    ) -> JiraMcpResult<TransitionIssueResult> {
        let start_time = std::time::Instant::now();

        info!("Transitioning issue: {}", params.issue_key);

        // Validate parameters
        self.validate_params(&params)?;

        // Get available transitions to resolve transition_name to ID if needed
        let available_transitions = self.get_available_transitions(&params.issue_key).await?;

        // Determine which transition to use
        let transition_to_use = if let Some(transition_id) = &params.transition_id {
            // Find by ID
            available_transitions
                .iter()
                .find(|t| &t.id == transition_id)
                .ok_or_else(|| {
                    JiraMcpError::invalid_param(
                        "transition_id",
                        format!(
                            "Transition ID '{}' not available for this issue. Available: {}",
                            transition_id,
                            available_transitions
                                .iter()
                                .map(|t| format!("{} (id: {})", t.name, t.id))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                })?
                .clone()
        } else if let Some(transition_name) = &params.transition_name {
            // Find by name (case-insensitive)
            let name_lower = transition_name.to_lowercase();
            available_transitions
                .iter()
                .find(|t| t.name.to_lowercase() == name_lower)
                .ok_or_else(|| {
                    JiraMcpError::invalid_param(
                        "transition_name",
                        format!(
                            "Transition '{}' not available for this issue. Available: {}",
                            transition_name,
                            available_transitions
                                .iter()
                                .map(|t| t.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                })?
                .clone()
        } else {
            unreachable!("validate_params should have caught this")
        };

        info!(
            "Using transition: {} (id: {}) -> {}",
            transition_to_use.name, transition_to_use.id, transition_to_use.to_status
        );

        // Build transition request
        let mut fields = std::collections::BTreeMap::new();

        // Add comment if provided
        if let Some(comment_text) = &params.comment {
            fields.insert(
                "comment".to_string(),
                serde_json::json!([{
                    "add": {
                        "body": comment_text
                    }
                }]),
            );
        }

        // Add resolution if provided
        if let Some(resolution_name) = &params.resolution {
            fields.insert(
                "resolution".to_string(),
                serde_json::json!({
                    "name": resolution_name
                }),
            );
        }

        let trigger_options = TransitionTriggerOptions {
            transition: Transition {
                id: transition_to_use.id.clone(),
            },
            fields,
        };

        // Execute transition
        let endpoint = format!("/issue/{}/transitions", params.issue_key);
        self.jira_client
            .client
            .post::<serde_json::Value, _>("api", &endpoint, trigger_options)
            .await
            .or_else(|e| {
                // JIRA transitions endpoint returns 204 No Content on success,
                // which can cause serde deserialization errors. Treat these as success.
                if e.to_string().contains("expected value") {
                    debug!("Ignoring deserialization error (likely 204 No Content response)");
                    Ok(serde_json::Value::Null)
                } else {
                    Err(JiraMcpError::from(e))
                }
            })?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Successfully transitioned issue {} to status: {}",
            params.issue_key, transition_to_use.to_status
        );

        Ok(TransitionIssueResult {
            success: true,
            issue_key: params.issue_key,
            transition_used: transition_to_use,
            execution_time_ms: execution_time,
        })
    }

    fn validate_params(&self, params: &TransitionIssueParams) -> JiraMcpResult<()> {
        // Validate issue key
        if params.issue_key.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key cannot be empty",
            ));
        }

        let parts: Vec<&str> = params.issue_key.split('-').collect();
        if parts.len() != 2 {
            return Err(JiraMcpError::invalid_param(
                "issue_key",
                "Issue key must be in format 'PROJECT-123'",
            ));
        }

        // Must provide either transition_id or transition_name
        if params.transition_id.is_none() && params.transition_name.is_none() {
            return Err(JiraMcpError::invalid_param(
                "transition_id or transition_name",
                "Must provide either transition_id or transition_name",
            ));
        }

        // Validate transition_id if provided
        if let Some(id) = &params.transition_id {
            if id.trim().is_empty() {
                return Err(JiraMcpError::invalid_param(
                    "transition_id",
                    "Transition ID cannot be empty",
                ));
            }
        }

        // Validate transition_name if provided
        if let Some(name) = &params.transition_name {
            if name.trim().is_empty() {
                return Err(JiraMcpError::invalid_param(
                    "transition_name",
                    "Transition name cannot be empty",
                ));
            }
        }

        Ok(())
    }

    async fn get_available_transitions(
        &self,
        issue_key: &str,
    ) -> JiraMcpResult<Vec<TransitionInfo>> {
        let endpoint = format!("/issue/{}/transitions?expand=transitions.fields", issue_key);

        let transition_options: TransitionOptions = self
            .jira_client
            .client
            .get("api", &endpoint)
            .await
            .map_err(JiraMcpError::from)?;

        Ok(transition_options
            .transitions
            .into_iter()
            .map(|t| TransitionInfo {
                id: t.id,
                name: t.name,
                to_status: t.to.name,
                to_status_id: t.to.id,
            })
            .collect())
    }
}
