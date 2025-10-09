use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for getting issue creation metadata
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCreateMetadataParams {
    /// Project key to get metadata for (e.g., "PROJ", "DEV")
    pub project_key: String,

    /// Optional: Get metadata for a specific issue type only
    /// If not provided, returns metadata for all available issue types
    #[serde(default)]
    pub issue_type: Option<String>,

    /// Include field schemas and allowed values (default: true)
    /// Set to false for a simpler response with just required fields
    #[serde(default = "default_true")]
    pub include_schemas: bool,
}

fn default_true() -> bool {
    true
}

/// Information about a field
#[derive(Debug, Serialize, JsonSchema)]
pub struct FieldInfo {
    /// Field key/ID (e.g., "summary", "description", "customfield_10016")
    pub field_id: String,

    /// Human-readable field name
    pub name: String,

    /// Whether this field is required
    pub required: bool,

    /// Field type (e.g., "string", "array", "option", "user", "priority")
    pub field_type: String,

    /// For fields with predefined values, the allowed values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,

    /// Schema information (more detailed type info)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,

    /// Whether this field has a default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_default_value: Option<bool>,

    /// Auto-complete URL if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_complete_url: Option<String>,
}

/// Metadata for an issue type
#[derive(Debug, Serialize, JsonSchema)]
pub struct IssueTypeMetadata {
    /// Issue type name (e.g., "Task", "Bug", "Story")
    pub name: String,

    /// Issue type ID
    pub id: String,

    /// Description of the issue type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this is a subtask type
    pub is_subtask: bool,

    /// All required fields for this issue type
    pub required_fields: Vec<String>,

    /// All available fields (required + optional)
    pub all_fields: Vec<FieldInfo>,

    /// Quick reference: fields grouped by category
    pub field_summary: FieldSummary,
}

/// Summary of fields by category for quick reference
#[derive(Debug, Serialize, JsonSchema)]
pub struct FieldSummary {
    /// Required standard fields (summary, project, etc.)
    pub required_standard: Vec<String>,

    /// Required custom fields
    pub required_custom: Vec<String>,

    /// Optional standard fields that are commonly used
    pub optional_standard: Vec<String>,

    /// Optional custom fields
    pub optional_custom: Vec<String>,
}

/// Result from getting creation metadata
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetCreateMetadataResult {
    /// Project key
    pub project_key: String,

    /// Project ID
    pub project_id: String,

    /// Project name
    pub project_name: String,

    /// Available issue types with their metadata
    pub issue_types: Vec<IssueTypeMetadata>,

    /// Quick reference: most commonly needed fields across all types
    pub common_required_fields: Vec<String>,

    /// Instructions for creating issues
    pub usage_hints: Vec<String>,
}

/// Tool for getting issue creation metadata
pub struct GetCreateMetadataTool {
    jira_client: Arc<JiraClient>,
}

impl GetCreateMetadataTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: GetCreateMetadataParams,
    ) -> JiraMcpResult<GetCreateMetadataResult> {
        info!(
            "Getting creation metadata for project: {}",
            params.project_key
        );

        // Call JIRA createmeta API
        let endpoint = format!(
            "/issue/createmeta?projectKeys={}&expand=projects.issuetypes.fields",
            params.project_key
        );

        let response: serde_json::Value = self
            .jira_client
            .client
            .get("api", &endpoint)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    JiraMcpError::not_found("project", &params.project_key)
                } else {
                    JiraMcpError::internal(format!("Failed to get metadata: {}", e))
                }
            })?;

        let projects = response["projects"]
            .as_array()
            .ok_or_else(|| JiraMcpError::internal("Invalid metadata response"))?;

        if projects.is_empty() {
            return Err(JiraMcpError::not_found("project", &params.project_key));
        }

        let project = &projects[0];
        let project_id = project["id"].as_str().unwrap_or_default().to_string();
        let project_name = project["name"].as_str().unwrap_or_default().to_string();

        let issue_types_data = project["issuetypes"]
            .as_array()
            .ok_or_else(|| JiraMcpError::internal("No issue types found"))?;

        let mut issue_types = Vec::new();
        let mut all_required_fields = std::collections::HashSet::new();

        for issue_type_data in issue_types_data {
            let issue_type_name = issue_type_data["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            // Filter by issue type if specified
            if let Some(ref filter_type) = params.issue_type {
                if issue_type_name.to_lowercase() != filter_type.to_lowercase() {
                    continue;
                }
            }

            let issue_type_id = issue_type_data["id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let description = issue_type_data["description"]
                .as_str()
                .map(|s| s.to_string());
            let is_subtask = issue_type_data["subtask"].as_bool().unwrap_or(false);

            let fields_data = issue_type_data["fields"]
                .as_object()
                .ok_or_else(|| JiraMcpError::internal("No fields data"))?;

            let mut all_fields = Vec::new();
            let mut required_fields = Vec::new();
            let mut required_standard = Vec::new();
            let mut required_custom = Vec::new();
            let mut optional_standard = Vec::new();
            let mut optional_custom = Vec::new();

            for (field_id, field_data) in fields_data {
                let name = field_data["name"].as_str().unwrap_or_default().to_string();
                let required = field_data["required"].as_bool().unwrap_or(false);
                let field_type = field_data["schema"]["type"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();

                // Extract allowed values for fields with predefined options
                let allowed_values = field_data["allowedValues"].as_array().map(|values| {
                    values
                        .iter()
                        .filter_map(|v| {
                            v["name"]
                                .as_str()
                                .or_else(|| v["value"].as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                });

                let has_default = field_data["hasDefaultValue"].as_bool();
                let auto_complete_url = field_data["autoCompleteUrl"]
                    .as_str()
                    .map(|s| s.to_string());

                let schema = if params.include_schemas {
                    Some(field_data["schema"].clone())
                } else {
                    None
                };

                let field_info = FieldInfo {
                    field_id: field_id.clone(),
                    name: name.clone(),
                    required,
                    field_type: field_type.clone(),
                    allowed_values,
                    schema,
                    has_default_value: has_default,
                    auto_complete_url,
                };

                all_fields.push(field_info);

                if required {
                    required_fields.push(field_id.clone());
                    all_required_fields.insert(field_id.clone());

                    if field_id.starts_with("customfield_") {
                        required_custom.push(format!("{} ({})", name, field_id));
                    } else {
                        required_standard.push(field_id.clone());
                    }
                } else if field_id.starts_with("customfield_") {
                    optional_custom.push(format!("{} ({})", name, field_id));
                } else {
                    // Only include commonly used optional standard fields
                    if matches!(
                        field_id.as_str(),
                        "description"
                            | "priority"
                            | "labels"
                            | "components"
                            | "assignee"
                            | "duedate"
                            | "timetracking"
                    ) {
                        optional_standard.push(field_id.clone());
                    }
                }
            }

            let field_summary = FieldSummary {
                required_standard,
                required_custom,
                optional_standard,
                optional_custom,
            };

            issue_types.push(IssueTypeMetadata {
                name: issue_type_name,
                id: issue_type_id,
                description,
                is_subtask,
                required_fields,
                all_fields,
                field_summary,
            });
        }

        if issue_types.is_empty() && params.issue_type.is_some() {
            return Err(JiraMcpError::invalid_param(
                "issue_type",
                format!(
                    "Issue type '{}' not found in project {}",
                    params.issue_type.unwrap(),
                    params.project_key
                ),
            ));
        }

        // Generate usage hints
        let mut usage_hints = vec![
            "Use create_issue tool to create issues in this project".to_string(),
            format!(
                "Available issue types: {}",
                issue_types
                    .iter()
                    .map(|it| it.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ];

        if !all_required_fields.is_empty() {
            let common_required: Vec<String> = all_required_fields.iter().cloned().collect();
            usage_hints.push(format!(
                "Common required fields across all types: {}",
                common_required.join(", ")
            ));
        }

        usage_hints.push(
            "For custom fields, use the field_id (e.g., 'customfield_10016') in custom_fields parameter".to_string()
        );

        Ok(GetCreateMetadataResult {
            project_key: params.project_key,
            project_id,
            project_name,
            issue_types,
            common_required_fields: all_required_fields.into_iter().collect(),
            usage_hints,
        })
    }
}
