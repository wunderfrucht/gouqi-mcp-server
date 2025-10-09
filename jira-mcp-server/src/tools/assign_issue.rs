use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for assigning an issue
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AssignIssueParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    pub issue_key: String,

    /// The assignee to set. Can be:
    /// - "me" or "self" to assign to yourself
    /// - A username or account ID
    /// - null or empty string to unassign
    #[serde(default)]
    pub assignee: Option<String>,
}

/// Result from assigning an issue
#[derive(Debug, Serialize, JsonSchema)]
pub struct AssignIssueResult {
    /// The issue key
    pub issue_key: String,

    /// The new assignee (or "Unassigned" if cleared)
    pub assignee: String,

    /// Success message
    pub message: String,
}

/// Tool for assigning JIRA issues
pub struct AssignIssueTool {
    jira_client: Arc<JiraClient>,
}

impl AssignIssueTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: AssignIssueParams) -> JiraMcpResult<AssignIssueResult> {
        info!(
            "Assigning issue {} to {:?}",
            params.issue_key, params.assignee
        );

        // Normalize assignee value
        let assignee_value = match params.assignee.as_deref() {
            Some("me") | Some("self") => {
                // Get current user's account ID
                let user_info = self.jira_client.get_current_user().await?;
                Some(user_info.account_id)
            }
            Some("") | None => None, // Unassign
            Some(assignee) => Some(assignee.to_string()),
        };

        // Build the update payload
        let update_body = if let Some(assignee) = &assignee_value {
            serde_json::json!({
                "fields": {
                    "assignee": {
                        "accountId": assignee
                    }
                }
            })
        } else {
            // Set to null to unassign
            serde_json::json!({
                "fields": {
                    "assignee": null
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
                } else if e.to_string().contains("400") || e.to_string().contains("Bad Request") {
                    JiraMcpError::invalid_param("assignee", format!("Invalid assignee: {}", e))
                } else {
                    JiraMcpError::internal(format!("Failed to assign issue: {}", e))
                }
            })?;

        let assignee_display = assignee_value
            .as_deref()
            .unwrap_or("Unassigned")
            .to_string();

        let message = if assignee_value.is_some() {
            format!(
                "Successfully assigned {} to {}",
                params.issue_key, assignee_display
            )
        } else {
            format!("Successfully unassigned {}", params.issue_key)
        };

        info!("{}", message);

        Ok(AssignIssueResult {
            issue_key: params.issue_key,
            assignee: assignee_display,
            message,
        })
    }
}
