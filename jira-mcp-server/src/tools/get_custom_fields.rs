//! Tool for discovering custom fields in JIRA issues

use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for getting custom fields
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCustomFieldsParams {
    /// The JIRA issue key to inspect (e.g., "PROJ-123")
    pub issue_key: String,
}

/// Information about a custom field
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CustomFieldInfo {
    /// Field ID (e.g., "customfield_10016")
    pub field_id: String,

    /// Field name (if available)
    pub field_name: Option<String>,

    /// Field type (string, number, object, array, etc.)
    pub field_type: String,

    /// Current value (as JSON)
    pub value: serde_json::Value,

    /// Human-readable value representation
    pub value_display: String,
}

/// Result from getting custom fields
#[derive(Debug, Serialize)]
pub struct GetCustomFieldsResult {
    /// Issue key
    pub issue_key: String,

    /// List of custom fields found
    pub custom_fields: Vec<CustomFieldInfo>,

    /// Total count
    pub total_count: usize,

    /// Common field mappings detected
    pub detected_mappings: DetectedMappings,
}

/// Detected common field mappings
#[derive(Debug, Serialize, JsonSchema)]
pub struct DetectedMappings {
    /// Story points field ID (if detected)
    pub story_points_field: Option<String>,

    /// Acceptance criteria field ID (if detected)
    pub acceptance_criteria_field: Option<String>,

    /// Sprint field ID (if detected)
    pub sprint_field: Option<String>,

    /// Epic link field ID (if detected)
    pub epic_link_field: Option<String>,
}

/// Tool for getting custom fields from an issue
pub struct GetCustomFieldsTool {
    jira_client: Arc<JiraClient>,
}

impl GetCustomFieldsTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: GetCustomFieldsParams,
    ) -> JiraMcpResult<GetCustomFieldsResult> {
        info!("Getting custom fields for issue: {}", params.issue_key);

        // Get issue with all fields
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

        let mut custom_fields = Vec::new();
        let mut detected_mappings = DetectedMappings {
            story_points_field: None,
            acceptance_criteria_field: None,
            sprint_field: None,
            epic_link_field: None,
        };

        // Iterate through all fields
        for (field_id, value) in issue.fields.iter() {
            // Only process custom fields
            if !field_id.starts_with("customfield_") {
                continue;
            }

            // Determine field type
            let field_type = match value {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::String(_) => "string",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
            }
            .to_string();

            // Create display value
            let value_display = match value {
                serde_json::Value::Null => "null".to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => {
                    if s.len() > 100 {
                        format!("{}... ({} chars)", &s[..100], s.len())
                    } else {
                        s.clone()
                    }
                }
                serde_json::Value::Array(a) => format!("Array[{}]", a.len()),
                serde_json::Value::Object(o) => {
                    // Try to extract meaningful info from objects
                    if let Some(name) = o.get("name").and_then(|v| v.as_str()) {
                        format!("Object: {}", name)
                    } else if let Some(value) = o.get("value").and_then(|v| v.as_str()) {
                        format!("Object: {}", value)
                    } else {
                        format!("Object with {} keys", o.len())
                    }
                }
            };

            // Detect common fields by analyzing values
            if value.is_number() && detected_mappings.story_points_field.is_none() {
                // Likely story points - numeric field
                detected_mappings.story_points_field = Some(field_id.clone());
            }

            if let serde_json::Value::String(s) = value {
                if s.len() > 50 && detected_mappings.acceptance_criteria_field.is_none() {
                    // Likely acceptance criteria - long text field
                    detected_mappings.acceptance_criteria_field = Some(field_id.clone());
                }
            }

            if field_type == "array" && field_id.contains("sprint") {
                detected_mappings.sprint_field = Some(field_id.clone());
            }

            if field_type == "string" && field_id.contains("epic") {
                detected_mappings.epic_link_field = Some(field_id.clone());
            }

            custom_fields.push(CustomFieldInfo {
                field_id: field_id.clone(),
                field_name: None, // We'd need field metadata API to get names
                field_type,
                value: value.clone(),
                value_display,
            });
        }

        info!(
            "Found {} custom fields for issue {}",
            custom_fields.len(),
            params.issue_key
        );

        Ok(GetCustomFieldsResult {
            issue_key: params.issue_key,
            total_count: custom_fields.len(),
            custom_fields,
            detected_mappings,
        })
    }
}
