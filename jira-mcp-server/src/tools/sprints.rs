//! Sprint management tools for JIRA Agile
//!
//! Provides tools for managing sprints, including listing sprints, getting sprint details,
//! moving issues to sprints, and getting issues in a sprint.

use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::{JiraClient, SearchResult};
use gouqi::{Board, SearchOptions, Sprint};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Parameters for the list_sprints tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListSprintsParams {
    /// Board ID to list sprints from (required)
    pub board_id: u64,

    /// Filter by sprint state (optional)
    /// Values: "active", "future", "closed"
    pub state: Option<String>,

    /// Maximum results to return (optional, default: 50, max: 100)
    pub limit: Option<u32>,

    /// Starting offset for pagination (optional, default: 0)
    pub start_at: Option<u32>,
}

/// Result from the list_sprints tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSprintsResult {
    /// List of sprints
    pub sprints: Vec<SprintInfo>,

    /// Total number of sprints
    pub total: usize,

    /// Starting offset
    pub start_at: u32,

    /// Whether there are more results
    pub has_more: bool,

    /// Board information
    pub board_id: u64,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for ListSprintsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize ListSprintsResult\"}}"
            ),
        }
    }
}

/// Parameters for the get_sprint_info tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetSprintInfoParams {
    /// Sprint ID (required)
    pub sprint_id: u64,
}

/// Result from the get_sprint_info tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSprintInfoResult {
    /// Sprint information
    pub sprint: SprintInfo,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for GetSprintInfoResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize GetSprintInfoResult\"}}"
            ),
        }
    }
}

/// Parameters for the get_sprint_issues tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetSprintIssuesParams {
    /// Sprint ID (required)
    pub sprint_id: u64,

    /// Maximum results to return (optional, default: 50, max: 200)
    pub limit: Option<u32>,

    /// Starting offset for pagination (optional, default: 0)
    pub start_at: Option<u32>,
}

/// Result from the get_sprint_issues tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSprintIssuesResult {
    /// Search result with issues
    pub search_result: SearchResult,

    /// Sprint information
    pub sprint: SprintInfo,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for GetSprintIssuesResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize GetSprintIssuesResult\"}}"
            ),
        }
    }
}

/// Parameters for the move_to_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveToSprintParams {
    /// Sprint ID to move issues to (required)
    pub sprint_id: u64,

    /// Issue keys to move (required)
    /// Examples: ["PROJ-123", "PROJ-456"]
    pub issue_keys: Vec<String>,
}

/// Result from the move_to_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveToSprintResult {
    /// Whether the operation was successful
    pub success: bool,

    /// Sprint ID
    pub sprint_id: u64,

    /// Sprint name
    pub sprint_name: String,

    /// Number of issues moved
    pub issues_moved: usize,

    /// Issue keys that were moved
    pub issue_keys: Vec<String>,

    /// Success message
    pub message: String,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for MoveToSprintResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize MoveToSprintResult\"}}"
            ),
        }
    }
}

/// Sprint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintInfo {
    /// Sprint ID
    pub id: u64,

    /// Sprint name
    pub name: String,

    /// Sprint state: "future", "active", or "closed"
    pub state: Option<String>,

    /// Sprint start date (ISO 8601 format)
    pub start_date: Option<String>,

    /// Sprint end date (ISO 8601 format)
    pub end_date: Option<String>,

    /// Sprint complete date (ISO 8601 format)
    pub complete_date: Option<String>,

    /// Origin board ID
    pub origin_board_id: Option<u64>,

    /// Direct link to the sprint
    pub self_link: String,
}

impl From<Sprint> for SprintInfo {
    fn from(sprint: Sprint) -> Self {
        SprintInfo {
            id: sprint.id,
            name: sprint.name,
            state: sprint.state,
            start_date: sprint.start_date.map(|dt| dt.to_string()),
            end_date: sprint.end_date.map(|dt| dt.to_string()),
            complete_date: sprint.complete_date.map(|dt| dt.to_string()),
            origin_board_id: sprint.origin_board_id,
            self_link: sprint.self_link,
        }
    }
}

/// Tool for listing sprints
pub struct ListSprintsTool {
    jira_client: Arc<JiraClient>,
}

impl ListSprintsTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: ListSprintsParams) -> JiraMcpResult<ListSprintsResult> {
        info!("Listing sprints for board {}", params.board_id);

        let limit = params.limit.unwrap_or(50).min(100) as u64;
        let start_at = params.start_at.unwrap_or(0) as u64;

        // Create a board object (we only need the ID)
        let board = Board {
            id: params.board_id,
            name: String::new(), // Not needed for API call
            self_link: String::new(),
            type_name: String::new(),
            location: None,
        };

        let options = SearchOptions::builder()
            .max_results(limit)
            .start_at(start_at)
            .build();

        let result = self
            .jira_client
            .client
            .sprints()
            .list(&board, &options)
            .await
            .map_err(|e| JiraMcpError::internal(format!("Failed to list sprints: {}", e)))?;

        // Filter by state if requested
        let mut sprints: Vec<SprintInfo> =
            result.values.into_iter().map(SprintInfo::from).collect();

        if let Some(state_filter) = &params.state {
            let state_lower = state_filter.to_lowercase();
            sprints.retain(|s| {
                s.state
                    .as_ref()
                    .map(|st| st.to_lowercase() == state_lower)
                    .unwrap_or(false)
            });
        }

        let total = sprints.len();
        let has_more = !result.is_last;

        info!(
            "Found {} sprints for board {} (has_more: {})",
            total, params.board_id, has_more
        );

        Ok(ListSprintsResult {
            sprints,
            total,
            start_at: start_at as u32,
            has_more,
            board_id: params.board_id,
        })
    }
}

/// Tool for getting sprint information
pub struct GetSprintInfoTool {
    jira_client: Arc<JiraClient>,
}

impl GetSprintInfoTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: GetSprintInfoParams) -> JiraMcpResult<GetSprintInfoResult> {
        info!("Getting sprint info for sprint {}", params.sprint_id);

        let sprint = self
            .jira_client
            .client
            .sprints()
            .get(params.sprint_id.to_string())
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    JiraMcpError::not_found("sprint", params.sprint_id.to_string())
                } else {
                    JiraMcpError::internal(format!("Failed to get sprint: {}", e))
                }
            })?;

        Ok(GetSprintInfoResult {
            sprint: SprintInfo::from(sprint),
        })
    }
}

/// Tool for getting issues in a sprint
pub struct GetSprintIssuesTool {
    jira_client: Arc<JiraClient>,
}

impl GetSprintIssuesTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: GetSprintIssuesParams,
    ) -> JiraMcpResult<GetSprintIssuesResult> {
        info!("Getting issues for sprint {}", params.sprint_id);

        // First get sprint info
        let sprint = self
            .jira_client
            .client
            .sprints()
            .get(params.sprint_id.to_string())
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    JiraMcpError::not_found("sprint", params.sprint_id.to_string())
                } else {
                    JiraMcpError::internal(format!("Failed to get sprint: {}", e))
                }
            })?;

        // Use JQL to get issues in this sprint
        // Sprint field uses customfield_10020 or similar, but we can use "Sprint = <sprint_id>"
        let jql = format!("Sprint = {}", params.sprint_id);

        let limit = params.limit.unwrap_or(50).min(200) as usize;
        let start_at = params.start_at.unwrap_or(0) as usize;

        let search_result = self
            .jira_client
            .search_issues_jql(&jql, Some(start_at), Some(limit), None)
            .await
            .map_err(|e| JiraMcpError::internal(format!("Failed to get sprint issues: {}", e)))?;

        info!(
            "Found {} issues in sprint {}",
            search_result.total, params.sprint_id
        );

        Ok(GetSprintIssuesResult {
            search_result,
            sprint: SprintInfo::from(sprint),
        })
    }
}

/// Tool for moving issues to a sprint
pub struct MoveToSprintTool {
    jira_client: Arc<JiraClient>,
}

impl MoveToSprintTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: MoveToSprintParams) -> JiraMcpResult<MoveToSprintResult> {
        info!(
            "Moving {} issues to sprint {}",
            params.issue_keys.len(),
            params.sprint_id
        );

        if params.issue_keys.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "issue_keys",
                "At least one issue key is required",
            ));
        }

        // Get sprint info first
        let sprint = self
            .jira_client
            .client
            .sprints()
            .get(params.sprint_id.to_string())
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    JiraMcpError::not_found("sprint", params.sprint_id.to_string())
                } else {
                    JiraMcpError::internal(format!("Failed to get sprint: {}", e))
                }
            })?;

        // Move issues to sprint
        self.jira_client
            .client
            .sprints()
            .move_issues(params.sprint_id, params.issue_keys.clone())
            .await
            .map_err(|e| {
                JiraMcpError::internal(format!("Failed to move issues to sprint: {}", e))
            })?;

        let issues_moved = params.issue_keys.len();
        let message = format!(
            "Successfully moved {} issue(s) to sprint '{}'",
            issues_moved, sprint.name
        );

        info!("{}", message);

        Ok(MoveToSprintResult {
            success: true,
            sprint_id: params.sprint_id,
            sprint_name: sprint.name,
            issues_moved,
            issue_keys: params.issue_keys,
            message,
        })
    }
}
