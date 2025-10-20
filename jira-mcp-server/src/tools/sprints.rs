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

// ============================================================================
// Sprint Lifecycle Tools (create, start, close)
// ============================================================================

/// Parameters for the create_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateSprintParams {
    /// Board ID where the sprint will be created (required)
    pub board_id: u64,

    /// Sprint name (required)
    /// Example: "Sprint 42", "PI 2025.1.3"
    pub name: String,

    /// Sprint start date (optional, ISO 8601 format)
    /// Example: "2025-01-20T00:00:00Z"
    pub start_date: Option<String>,

    /// Sprint end date (optional, ISO 8601 format)
    /// Example: "2025-02-03T23:59:59Z"
    pub end_date: Option<String>,

    /// Sprint goal/objective (optional)
    pub goal: Option<String>,
}

/// Result from the create_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSprintResult {
    /// Created sprint information
    pub sprint: SprintInfo,

    /// Success message
    pub message: String,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for CreateSprintResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize CreateSprintResult\"}}"
            ),
        }
    }
}

/// Parameters for the start_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartSprintParams {
    /// Sprint ID to start (required)
    pub sprint_id: u64,

    /// Sprint start date (optional, defaults to now if not set)
    /// Example: "2025-01-20T00:00:00Z"
    pub start_date: Option<String>,

    /// Sprint end date (required if not already set on sprint)
    /// Example: "2025-02-03T23:59:59Z"
    pub end_date: Option<String>,

    /// Update sprint goal (optional)
    pub goal: Option<String>,
}

/// Result from the start_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSprintResult {
    /// Updated sprint information
    pub sprint: SprintInfo,

    /// Number of issues in the sprint
    pub issue_count: usize,

    /// Success message
    pub message: String,

    /// Warnings (e.g., "Sprint has no issues")
    pub warnings: Vec<String>,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for StartSprintResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize StartSprintResult\"}}"
            ),
        }
    }
}

/// Parameters for the close_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseSprintParams {
    /// Sprint ID to close (required)
    pub sprint_id: u64,

    /// Target sprint ID for incomplete issues (optional)
    /// If provided, incomplete issues will be moved to this sprint
    pub move_incomplete_to: Option<u64>,
}

/// Result from the close_sprint tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseSprintResult {
    /// Updated sprint information
    pub sprint: SprintInfo,

    /// Number of completed issues
    pub completed_issues: usize,

    /// Number of incomplete issues
    pub incomplete_issues: usize,

    /// Number of issues moved (if move_incomplete_to was specified)
    pub moved_issues: Option<usize>,

    /// Success message
    pub message: String,

    /// Warnings (e.g., completion statistics)
    pub warnings: Vec<String>,
}

// Workaround for pulseengine-mcp-macros issue
impl std::fmt::Display for CloseSprintResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize CloseSprintResult\"}}"
            ),
        }
    }
}

/// Tool for creating a new sprint
pub struct CreateSprintTool {
    jira_client: Arc<JiraClient>,
}

impl CreateSprintTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: CreateSprintParams) -> JiraMcpResult<CreateSprintResult> {
        info!(
            "Creating sprint '{}' on board {}",
            params.name, params.board_id
        );

        // Validate sprint name
        if params.name.trim().is_empty() {
            return Err(JiraMcpError::invalid_param(
                "name",
                "Sprint name cannot be empty",
            ));
        }

        // Create a board object
        let board = Board {
            id: params.board_id,
            name: String::new(),
            self_link: String::new(),
            type_name: String::new(),
            location: None,
        };

        // Create the sprint using gouqi
        let sprint = self
            .jira_client
            .client
            .sprints()
            .create(board, params.name.clone())
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    JiraMcpError::not_found("board", params.board_id.to_string())
                } else {
                    JiraMcpError::internal(format!("Failed to create sprint: {}", e))
                }
            })?;

        let message = format!(
            "Successfully created sprint '{}' (ID: {}) on board {}",
            sprint.name, sprint.id, params.board_id
        );

        info!("{}", message);

        Ok(CreateSprintResult {
            sprint: SprintInfo::from(sprint),
            message,
        })
    }
}

/// Tool for starting a sprint
pub struct StartSprintTool {
    jira_client: Arc<JiraClient>,
}

impl StartSprintTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: StartSprintParams) -> JiraMcpResult<StartSprintResult> {
        info!("Starting sprint {}", params.sprint_id);

        // Get current sprint info
        let current_sprint = self
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

        // Validate sprint state
        if let Some(state) = &current_sprint.state {
            if state.to_lowercase() == "active" {
                return Err(JiraMcpError::invalid_param(
                    "sprint_id",
                    format!("Sprint '{}' is already active", current_sprint.name),
                ));
            }
            if state.to_lowercase() == "closed" {
                return Err(JiraMcpError::invalid_param(
                    "sprint_id",
                    format!(
                        "Sprint '{}' is already closed and cannot be started",
                        current_sprint.name
                    ),
                ));
            }
        }

        // Parse dates
        let start_date = if let Some(date_str) = &params.start_date {
            Some(parse_iso8601_date(date_str)?)
        } else {
            Some(time::OffsetDateTime::now_utc())
        };

        let end_date = if let Some(date_str) = &params.end_date {
            Some(parse_iso8601_date(date_str)?)
        } else {
            current_sprint.end_date
        };

        // Validate end date is set
        if end_date.is_none() {
            return Err(JiraMcpError::invalid_param(
                "end_date",
                "Sprint end date must be set before starting (provide end_date parameter or set it on the sprint)",
            ));
        }

        // Build update request
        let update_data = gouqi::UpdateSprint {
            name: None, // Keep current name
            start_date,
            end_date,
            state: Some("active".to_string()),
        };

        // Update sprint to active state
        let updated_sprint = self
            .jira_client
            .client
            .sprints()
            .update(params.sprint_id, update_data)
            .await
            .map_err(|e| JiraMcpError::internal(format!("Failed to start sprint: {}", e)))?;

        // Get issue count in sprint
        let jql = format!("Sprint = {}", params.sprint_id);
        let search_result = self
            .jira_client
            .search_issues_jql(&jql, Some(0), Some(1), None)
            .await
            .map_err(|e| JiraMcpError::internal(format!("Failed to count sprint issues: {}", e)))?;

        let issue_count = search_result.total;

        // Generate warnings
        let mut warnings = Vec::new();
        if issue_count == 0 {
            warnings.push("Warning: Sprint has no issues".to_string());
        }

        let message = format!(
            "Successfully started sprint '{}' with {} issue(s)",
            updated_sprint.name, issue_count
        );

        info!("{}", message);

        Ok(StartSprintResult {
            sprint: SprintInfo::from(updated_sprint),
            issue_count,
            message,
            warnings,
        })
    }
}

/// Tool for closing a sprint
pub struct CloseSprintTool {
    jira_client: Arc<JiraClient>,
}

impl CloseSprintTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, params: CloseSprintParams) -> JiraMcpResult<CloseSprintResult> {
        info!("Closing sprint {}", params.sprint_id);

        // Get current sprint info
        let current_sprint = self
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

        // Validate sprint state
        if let Some(state) = &current_sprint.state {
            if state.to_lowercase() == "closed" {
                return Err(JiraMcpError::invalid_param(
                    "sprint_id",
                    format!("Sprint '{}' is already closed", current_sprint.name),
                ));
            }
        }

        // Get issue statistics before closing
        let jql = format!("Sprint = {}", params.sprint_id);
        let all_issues = self
            .jira_client
            .search_issues_jql(&jql, Some(0), Some(1000), None)
            .await
            .map_err(|e| JiraMcpError::internal(format!("Failed to get sprint issues: {}", e)))?;

        let total_issues = all_issues.total;
        let completed_issues = all_issues
            .issues
            .iter()
            .filter(|issue| {
                issue.status.to_lowercase() == "done"
                    || issue.status.to_lowercase() == "closed"
                    || issue.status.to_lowercase() == "resolved"
            })
            .count();
        let incomplete_issues = total_issues - completed_issues;

        // Move incomplete issues if requested
        let mut moved_issues = None;
        if let Some(target_sprint_id) = params.move_incomplete_to {
            if incomplete_issues > 0 {
                let incomplete_keys: Vec<String> = all_issues
                    .issues
                    .iter()
                    .filter(|issue| {
                        issue.status.to_lowercase() != "done"
                            && issue.status.to_lowercase() != "closed"
                            && issue.status.to_lowercase() != "resolved"
                    })
                    .map(|issue| issue.key.clone())
                    .collect();

                info!(
                    "Moving {} incomplete issue(s) to sprint {}",
                    incomplete_keys.len(),
                    target_sprint_id
                );

                self.jira_client
                    .client
                    .sprints()
                    .move_issues(target_sprint_id, incomplete_keys)
                    .await
                    .map_err(|e| {
                        JiraMcpError::internal(format!("Failed to move incomplete issues: {}", e))
                    })?;

                moved_issues = Some(incomplete_issues);
            }
        }

        // Build update request
        // Note: JIRA automatically sets complete_date to now when transitioning to closed
        let update_data = gouqi::UpdateSprint {
            name: None,
            start_date: None,
            end_date: None,
            state: Some("closed".to_string()),
        };

        // Close the sprint
        let updated_sprint = self
            .jira_client
            .client
            .sprints()
            .update(params.sprint_id, update_data)
            .await
            .map_err(|e| JiraMcpError::internal(format!("Failed to close sprint: {}", e)))?;

        // Generate warnings
        let mut warnings = Vec::new();
        if total_issues > 0 {
            let completion_rate = (completed_issues as f64 / total_issues as f64) * 100.0;
            warnings.push(format!(
                "Sprint completion: {}/{} issues ({:.1}%)",
                completed_issues, total_issues, completion_rate
            ));
        }
        if incomplete_issues > 0 && moved_issues.is_none() {
            warnings.push(format!(
                "{} incomplete issue(s) remain in closed sprint (not moved)",
                incomplete_issues
            ));
        }

        let message = if let Some(moved_count) = moved_issues {
            format!(
                "Successfully closed sprint '{}' ({}/{} completed, {} moved to next sprint)",
                updated_sprint.name, completed_issues, total_issues, moved_count
            )
        } else {
            format!(
                "Successfully closed sprint '{}' ({}/{} completed)",
                updated_sprint.name, completed_issues, total_issues
            )
        };

        info!("{}", message);

        Ok(CloseSprintResult {
            sprint: SprintInfo::from(updated_sprint),
            completed_issues,
            incomplete_issues,
            moved_issues,
            message,
            warnings,
        })
    }
}

/// Helper function to parse ISO 8601 date strings
fn parse_iso8601_date(date_str: &str) -> JiraMcpResult<time::OffsetDateTime> {
    time::OffsetDateTime::parse(
        date_str,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|e| {
        JiraMcpError::invalid_param(
            "date",
            format!("Invalid ISO 8601 date format: {} (error: {})", date_str, e),
        )
    })
}
