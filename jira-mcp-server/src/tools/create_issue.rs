use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for creating a new JIRA issue
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateIssueParams {
    /// Project key where the issue will be created (e.g., "PROJ", "DEV")
    /// Can be inferred from parent_issue_key if not provided
    #[serde(default)]
    pub project_key: Option<String>,

    /// Issue summary/title (required)
    /// Keep it concise and descriptive (e.g., "Fix login button alignment")
    pub summary: String,

    /// Issue description in markdown format
    /// Can include todo checklists, code blocks, etc.
    #[serde(default)]
    pub description: Option<String>,

    /// Issue type (default: "Task")
    /// Common types: "Task", "Bug", "Story", "Epic", "Subtask"
    #[serde(default)]
    pub issue_type: Option<String>,

    /// Priority (e.g., "High", "Medium", "Low", "Highest", "Lowest")
    /// Defaults to project's default priority if not specified
    #[serde(default)]
    pub priority: Option<String>,

    /// Assignee - can be:
    /// - "me" or "self" to assign to yourself
    /// - A username or account ID
    /// - null/unspecified to leave unassigned
    #[serde(default)]
    pub assignee: Option<String>,

    /// Labels to add to the issue
    #[serde(default)]
    pub labels: Vec<String>,

    /// Components to add to the issue
    #[serde(default)]
    pub components: Vec<String>,

    /// Parent issue key for creating subtasks (e.g., "PROJ-123")
    /// If provided, issue_type will be set to "Subtask" automatically
    #[serde(default)]
    pub parent_issue_key: Option<String>,

    /// Epic link - associate this issue with an epic
    #[serde(default)]
    pub epic_link: Option<String>,

    /// Story points (if your project uses them)
    #[serde(default)]
    pub story_points: Option<f64>,

    /// Custom field updates as a map of field_id -> value
    /// Use get_custom_fields to discover field IDs
    #[serde(default)]
    pub custom_fields: HashMap<String, serde_json::Value>,

    /// Initial todo checklist items to add to description
    /// Automatically formats as markdown checkboxes
    #[serde(default)]
    pub initial_todos: Vec<String>,

    /// Auto-assign to yourself (default: false)
    /// Convenience shorthand for assignee: "me"
    #[serde(default)]
    pub assign_to_me: bool,
}

/// Result from creating an issue
#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateIssueResult {
    /// The created issue key (e.g., "PROJ-456")
    pub issue_key: String,

    /// The issue ID
    pub issue_id: String,

    /// Direct URL to the issue
    pub issue_url: String,

    /// Summary of what was created
    pub summary: String,

    /// Issue type that was created
    pub issue_type: String,

    /// Project key
    pub project_key: String,

    /// Success message
    pub message: String,
}

/// Tool for creating JIRA issues
pub struct CreateIssueTool {
    jira_client: Arc<JiraClient>,
}

impl CreateIssueTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: CreateIssueParams) -> JiraMcpResult<CreateIssueResult> {
        info!("Creating new JIRA issue: {}", params.summary);

        // Determine project key
        let project_key = if let Some(key) = params.project_key {
            key
        } else if let Some(parent_key) = &params.parent_issue_key {
            // Extract project key from parent (e.g., "PROJ-123" -> "PROJ")
            parent_key
                .split('-')
                .next()
                .ok_or_else(|| {
                    JiraMcpError::invalid_param(
                        "parent_issue_key",
                        "Invalid parent issue key format",
                    )
                })?
                .to_string()
        } else {
            return Err(JiraMcpError::invalid_param(
                "project_key",
                "Either project_key or parent_issue_key must be provided",
            ));
        };

        // Determine issue type
        let issue_type = if params.parent_issue_key.is_some() {
            "Subtask".to_string()
        } else {
            params.issue_type.unwrap_or_else(|| "Task".to_string())
        };

        // Build description with initial todos if provided
        let description = if !params.initial_todos.is_empty() {
            let todo_lines: Vec<String> = params
                .initial_todos
                .iter()
                .map(|todo| format!("- [ ] {}", todo))
                .collect();

            let todo_section = format!("\n\n## Tasks\n\n{}", todo_lines.join("\n"));

            match params.description {
                Some(desc) => format!("{}{}", desc, todo_section),
                None => todo_section.trim_start().to_string(),
            }
        } else {
            params.description.unwrap_or_default()
        };

        // Determine assignee
        let assignee_value = if params.assign_to_me {
            Some("me")
        } else {
            params.assignee.as_deref()
        };

        let assignee_id = match assignee_value {
            Some("me") | Some("self") => {
                let user = self.jira_client.get_current_user().await?;
                Some(user.account_id)
            }
            Some(assignee) => Some(assignee.to_string()),
            None => None,
        };

        // Build the fields object
        let mut fields = serde_json::json!({
            "project": {
                "key": project_key
            },
            "summary": params.summary,
            "issuetype": {
                "name": issue_type
            }
        });

        // Add optional fields
        if !description.is_empty() {
            fields["description"] = serde_json::json!(description);
        }

        if let Some(priority) = params.priority {
            fields["priority"] = serde_json::json!({ "name": priority });
        }

        if let Some(assignee_id) = assignee_id {
            fields["assignee"] = serde_json::json!({ "accountId": assignee_id });
        }

        if !params.labels.is_empty() {
            fields["labels"] = serde_json::json!(params.labels);
        }

        if !params.components.is_empty() {
            fields["components"] = serde_json::json!(params
                .components
                .iter()
                .map(|c| serde_json::json!({ "name": c }))
                .collect::<Vec<_>>());
        }

        if let Some(parent_key) = params.parent_issue_key {
            fields["parent"] = serde_json::json!({ "key": parent_key });
        }

        // Add custom fields
        for (field_id, value) in params.custom_fields {
            fields[field_id] = value;
        }

        // Handle common custom fields with convenience parameters
        if let Some(story_points) = params.story_points {
            // Try common story points field IDs
            // Users can override with custom_fields if different
            if !fields
                .as_object()
                .unwrap()
                .contains_key("customfield_10016")
            {
                fields["customfield_10016"] = serde_json::json!(story_points);
            }
        }

        if let Some(epic_link) = params.epic_link {
            // Try common epic link field IDs
            if !fields
                .as_object()
                .unwrap()
                .contains_key("customfield_10014")
            {
                fields["customfield_10014"] = serde_json::json!(epic_link);
            }
        }

        // Create the issue
        let create_body = serde_json::json!({ "fields": fields });

        let response: serde_json::Value = self
            .jira_client
            .client
            .post("api", "/issue", create_body)
            .await
            .map_err(|e| {
                if e.to_string().contains("project is required")
                    || e.to_string().contains("project does not exist")
                {
                    JiraMcpError::invalid_param(
                        "project_key",
                        format!("Invalid project: {}", project_key),
                    )
                } else if e.to_string().contains("valid issue type") {
                    JiraMcpError::invalid_param(
                        "issue_type",
                        format!("Invalid issue type: {}", issue_type),
                    )
                } else {
                    JiraMcpError::internal(format!("Failed to create issue: {}", e))
                }
            })?;

        let issue_key = response["key"]
            .as_str()
            .ok_or_else(|| JiraMcpError::internal("No issue key in response"))?
            .to_string();

        let issue_id = response["id"]
            .as_str()
            .ok_or_else(|| JiraMcpError::internal("No issue ID in response"))?
            .to_string();

        let issue_url = format!("{}/browse/{}", self.jira_client.base_url(), issue_key);

        info!("Successfully created issue: {}", issue_key);

        Ok(CreateIssueResult {
            issue_key: issue_key.clone(),
            issue_id,
            issue_url: issue_url.clone(),
            summary: params.summary.clone(),
            issue_type: issue_type.clone(),
            project_key: project_key.clone(),
            message: format!(
                "Successfully created {} '{}' in project {}. View at: {}",
                issue_type.to_lowercase(),
                params.summary,
                project_key,
                issue_url
            ),
        })
    }
}
