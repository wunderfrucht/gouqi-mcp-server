use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for updating issue components
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateComponentsParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    pub issue_key: String,

    /// Component names or IDs to set. This replaces all existing components.
    /// Can be component names (e.g., "Backend") or component IDs (e.g., "10000")
    pub components: Vec<String>,
}

/// Result from updating components
#[derive(Debug, Serialize, JsonSchema)]
pub struct UpdateComponentsResult {
    /// The issue key
    pub issue_key: String,

    /// Components that were set on the issue
    pub components: Vec<ComponentInfo>,

    /// Success message
    pub message: String,
}

/// Component information
#[derive(Debug, Serialize, JsonSchema, Clone)]
pub struct ComponentInfo {
    /// Component ID
    pub id: String,

    /// Component name
    pub name: String,

    /// Component description (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Parameters for getting available components
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAvailableComponentsParams {
    /// The project key to get components for
    pub project_key: String,
}

/// Result from getting available components
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetAvailableComponentsResult {
    /// List of available components in the project
    pub components: Vec<ComponentInfo>,

    /// Total number of components
    pub total: u32,

    /// The project key
    pub project_key: String,
}

/// Tool for managing JIRA issue components
pub struct ComponentsTool {
    jira_client: Arc<JiraClient>,
}

impl ComponentsTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn update_components(
        &self,
        params: UpdateComponentsParams,
    ) -> JiraMcpResult<UpdateComponentsResult> {
        info!(
            "Updating components for issue {}: {:?}",
            params.issue_key, params.components
        );

        // If empty, clear all components
        let components_json: Vec<Value> = if params.components.is_empty() {
            vec![]
        } else {
            params
                .components
                .iter()
                .map(|comp| {
                    // Check if it's a numeric ID or a name
                    if comp.chars().all(|c| c.is_numeric()) {
                        serde_json::json!({"id": comp})
                    } else {
                        serde_json::json!({"name": comp})
                    }
                })
                .collect()
        };

        // Build the update payload
        let update_body = serde_json::json!({
            "fields": {
                "components": components_json
            }
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
                } else if e.to_string().contains("component") {
                    JiraMcpError::invalid_param(
                        "components",
                        format!("Invalid component: {}", e),
                    )
                } else {
                    JiraMcpError::internal(format!("Failed to update components: {}", e))
                }
            })?;

        // Fetch the issue to get current components
        let issue: Value = self
            .jira_client
            .client
            .get("api", &format!("/issue/{}?fields=components", params.issue_key))
            .await
            .map_err(|e| {
                JiraMcpError::internal(format!("Failed to fetch updated issue: {}", e))
            })?;

        let components: Vec<ComponentInfo> = issue["fields"]["components"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(ComponentInfo {
                            id: v["id"].as_str()?.to_string(),
                            name: v["name"].as_str()?.to_string(),
                            description: v["description"].as_str().map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let message = if params.components.is_empty() {
            format!("Successfully cleared all components from {}", params.issue_key)
        } else {
            format!(
                "Successfully set {} component(s) on {}",
                components.len(),
                params.issue_key
            )
        };

        info!("{}", message);

        Ok(UpdateComponentsResult {
            issue_key: params.issue_key,
            components,
            message,
        })
    }

    #[instrument(skip(self))]
    pub async fn get_available_components(
        &self,
        params: GetAvailableComponentsParams,
    ) -> JiraMcpResult<GetAvailableComponentsResult> {
        info!(
            "Getting available components for project {}",
            params.project_key
        );

        // Get components for the project
        let endpoint = format!("/project/{}/components", params.project_key);

        let response: Value = self
            .jira_client
            .client
            .get("api", &endpoint)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    JiraMcpError::not_found("project", &params.project_key)
                } else {
                    JiraMcpError::internal(format!("Failed to get components: {}", e))
                }
            })?;

        let components: Vec<ComponentInfo> = response
            .as_array()
            .ok_or_else(|| {
                JiraMcpError::internal("Expected array of components from API".to_string())
            })?
            .iter()
            .filter_map(|v| {
                Some(ComponentInfo {
                    id: v["id"].as_str()?.to_string(),
                    name: v["name"].as_str()?.to_string(),
                    description: v["description"].as_str().map(|s| s.to_string()),
                })
            })
            .collect();

        let total = components.len() as u32;

        info!(
            "Found {} components in project {}",
            total, params.project_key
        );

        Ok(GetAvailableComponentsResult {
            components,
            total,
            project_key: params.project_key,
        })
    }
}
