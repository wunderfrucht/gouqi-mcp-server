//! Tool for updating custom fields in JIRA issues

use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for updating custom fields
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateCustomFieldsParams {
    /// The JIRA issue key to update (e.g., "PROJ-123")
    pub issue_key: String,

    /// Story points value (will auto-detect field ID)
    #[serde(default)]
    pub story_points: Option<f64>,

    /// Acceptance criteria text (will auto-detect field ID)
    #[serde(default)]
    pub acceptance_criteria: Option<String>,

    /// Custom field updates by field ID
    /// Map of field_id -> value (as JSON)
    #[serde(default)]
    pub custom_field_updates: Option<std::collections::HashMap<String, serde_json::Value>>,

    /// Override story points field ID (if auto-detection fails)
    #[serde(default)]
    pub story_points_field_id: Option<String>,

    /// Override acceptance criteria field ID (if auto-detection fails)
    #[serde(default)]
    pub acceptance_criteria_field_id: Option<String>,
}

/// Result from updating custom fields
#[derive(Debug, Serialize)]
pub struct UpdateCustomFieldsResult {
    /// Issue key
    pub issue_key: String,

    /// Fields that were updated
    pub updated_fields: Vec<String>,

    /// Success message
    pub message: String,
}

/// Tool for updating custom fields
pub struct UpdateCustomFieldsTool {
    jira_client: Arc<JiraClient>,
}

impl UpdateCustomFieldsTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: UpdateCustomFieldsParams,
    ) -> JiraMcpResult<UpdateCustomFieldsResult> {
        info!("Updating custom fields for issue: {}", params.issue_key);

        let mut updates = serde_json::Map::new();
        let mut updated_field_names = Vec::new();

        // Handle story points
        if let Some(points) = params.story_points {
            let field_id = if let Some(id) = params.story_points_field_id {
                id
            } else {
                // Try common field IDs
                self.detect_story_points_field(&params.issue_key)
                    .await?
                    .unwrap_or_else(|| "customfield_10016".to_string())
            };

            updates.insert(field_id.clone(), serde_json::json!(points));
            updated_field_names.push(format!("story_points ({})", field_id));
        }

        // Handle acceptance criteria
        if let Some(criteria) = params.acceptance_criteria {
            let field_id = if let Some(id) = params.acceptance_criteria_field_id {
                id
            } else {
                self.detect_acceptance_criteria_field(&params.issue_key)
                    .await?
                    .unwrap_or_else(|| "customfield_10100".to_string())
            };

            updates.insert(field_id.clone(), serde_json::json!(criteria));
            updated_field_names.push(format!("acceptance_criteria ({})", field_id));
        }

        // Handle direct custom field updates
        if let Some(custom_updates) = params.custom_field_updates {
            for (field_id, value) in custom_updates {
                updates.insert(field_id.clone(), value);
                updated_field_names.push(field_id);
            }
        }

        if updates.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "updates",
                "No field updates specified",
            ));
        }

        // Build the update request
        let update_body = serde_json::json!({
            "fields": updates
        });

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
                    JiraMcpError::invalid_param(
                        "field_updates",
                        format!("Invalid field update: {}", e),
                    )
                } else {
                    JiraMcpError::from(e)
                }
            })?;

        info!(
            "Updated {} custom fields for issue {}",
            updated_field_names.len(),
            params.issue_key
        );

        Ok(UpdateCustomFieldsResult {
            issue_key: params.issue_key,
            updated_fields: updated_field_names.clone(),
            message: format!(
                "Successfully updated {} field(s): {}",
                updated_field_names.len(),
                updated_field_names.join(", ")
            ),
        })
    }

    /// Try to detect the story points field ID for an issue
    async fn detect_story_points_field(&self, issue_key: &str) -> JiraMcpResult<Option<String>> {
        let issue = self
            .jira_client
            .client
            .issues()
            .get(issue_key)
            .await
            .map_err(JiraMcpError::from)?;

        // Try common field IDs
        let common_ids = vec![
            "customfield_10016",
            "customfield_10026",
            "customfield_10106",
        ];

        for field_id in common_ids {
            if let Some(value) = issue.fields.get(field_id) {
                if value.is_number() {
                    return Ok(Some(field_id.to_string()));
                }
            }
        }

        // Search for any numeric custom field (heuristic)
        for (field_id, value) in issue.fields.iter() {
            if field_id.starts_with("customfield_") && value.is_number() {
                return Ok(Some(field_id.clone()));
            }
        }

        Ok(None)
    }

    /// Try to detect the acceptance criteria field ID for an issue
    async fn detect_acceptance_criteria_field(
        &self,
        issue_key: &str,
    ) -> JiraMcpResult<Option<String>> {
        let issue = self
            .jira_client
            .client
            .issues()
            .get(issue_key)
            .await
            .map_err(JiraMcpError::from)?;

        // Try common field IDs
        let common_ids = vec![
            "customfield_10100",
            "customfield_10007",
            "customfield_10200",
        ];

        for field_id in common_ids {
            if let Some(value) = issue.fields.get(field_id) {
                if value.is_string() {
                    return Ok(Some(field_id.to_string()));
                }
            }
        }

        Ok(None)
    }
}
