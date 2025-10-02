use crate::error::JiraMcpResult;
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, info, instrument};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateMode {
    /// Replace the entire description
    Replace,
    /// Append content to the end of the description (default)
    #[default]
    Append,
    /// Prepend content to the beginning of the description
    Prepend,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateDescriptionParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    pub issue_key: String,

    /// The content to add or replace
    pub content: String,

    /// How to update the description: "replace", "append" (default), or "prepend"
    #[serde(default)]
    pub mode: UpdateMode,
}

#[derive(Debug, Serialize)]
pub struct UpdateDescriptionResult {
    /// Whether the update was successful
    pub success: bool,

    /// The issue key that was updated
    pub issue_key: String,

    /// The mode used for the update
    pub mode: String,

    /// The new description (for confirmation)
    pub new_description: String,
}

pub struct UpdateDescription {
    jira_client: Arc<JiraClient>,
}

impl UpdateDescription {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self), fields(issue_key = %params.issue_key))]
    pub async fn execute(
        &self,
        params: UpdateDescriptionParams,
    ) -> JiraMcpResult<UpdateDescriptionResult> {
        info!(
            "Updating description for issue {} with mode: {:?}",
            params.issue_key, params.mode
        );

        // First, get the current description if we're appending or prepending
        let new_description = match params.mode {
            UpdateMode::Replace => params.content.clone(),
            UpdateMode::Append | UpdateMode::Prepend => {
                debug!(
                    "Fetching current description for issue: {}",
                    params.issue_key
                );

                let current_issue = self
                    .jira_client
                    .client
                    .issues()
                    .get(&params.issue_key)
                    .await?;

                let current_description =
                    current_issue.description().unwrap_or_default().to_string();

                match params.mode {
                    UpdateMode::Append => {
                        if current_description.is_empty() {
                            params.content.clone()
                        } else {
                            format!("{}\n\n{}", current_description, params.content)
                        }
                    }
                    UpdateMode::Prepend => {
                        if current_description.is_empty() {
                            params.content.clone()
                        } else {
                            format!("{}\n\n{}", params.content, current_description)
                        }
                    }
                    UpdateMode::Replace => unreachable!(),
                }
            }
        };

        debug!(
            "New description length: {} characters",
            new_description.len()
        );

        // Build the update payload using gouqi's EditIssue
        let mut fields = BTreeMap::new();
        fields.insert(
            "description".to_string(),
            serde_json::json!(new_description),
        );

        let edit_issue = gouqi::issues::EditIssue { fields };

        // Perform the update
        self.jira_client
            .client
            .issues()
            .edit(&params.issue_key, edit_issue)
            .await?;

        info!(
            "Successfully updated description for issue {}",
            params.issue_key
        );

        Ok(UpdateDescriptionResult {
            success: true,
            issue_key: params.issue_key,
            mode: format!("{:?}", params.mode).to_lowercase(),
            new_description,
        })
    }
}
