//! User issues tool for retrieving issues assigned to specific users
//!
//! This tool provides a convenient way to get issues assigned to a user
//! with semantic filtering options.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::{JiraClient, SearchResult};
use crate::semantic_mapping::SemanticMapper;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// Parameters for the get_user_issues tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetUserIssuesParams {
    /// Username, account ID, or special reference (optional)
    /// Examples: "me", "current_user", "john.doe", "account123", or omit for current user
    pub username: Option<String>,

    /// Status filter using semantic categories (optional)
    /// Examples: ["open", "in_progress", "done"]
    pub status_filter: Option<Vec<String>>,

    /// Issue type filter using semantic types (optional)
    /// Examples: ["story", "bug", "feature", "task"]
    pub issue_types: Option<Vec<String>>,

    /// Board name filter (optional)
    /// Limits results to issues from specific boards
    pub board_filter: Option<Vec<String>>,

    /// Project key filter (optional)
    /// Limits results to specific projects
    pub project_filter: Option<Vec<String>>,

    /// Due date filter (optional)
    /// Examples: "overdue", "this_week", "next_week", "2024-01-01"
    pub due_date_filter: Option<String>,

    /// Priority filter (optional)
    /// Examples: ["high", "critical", "medium"]
    pub priority_filter: Option<Vec<String>>,

    /// Only show issues updated recently (optional)
    /// Examples: "today", "7 days ago", "2024-01-01"
    pub updated_since: Option<String>,

    /// Maximum results to return (optional, default: 50, max: 200)
    pub limit: Option<u32>,

    /// Starting offset for pagination (optional, default: 0)
    pub start_at: Option<u32>,
}

/// Result from the get_user_issues tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserIssuesResult {
    /// The search results
    pub search_result: SearchResult,

    /// User information that was resolved
    pub resolved_user: UserInfo,

    /// The JQL query that was executed
    pub jql_query: String,

    /// Summary of applied filters
    pub applied_filters: AppliedFilters,

    /// Performance information
    pub performance: UserIssuesPerformance,
}

/// Information about the resolved user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User account ID
    pub account_id: String,

    /// Display name
    pub display_name: String,

    /// Email address (if available)
    pub email_address: Option<String>,

    /// Whether this is the current authenticated user
    pub is_current_user: bool,
}

/// Summary of filters that were applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedFilters {
    pub status_categories: Option<Vec<String>>,
    pub issue_types: Option<Vec<String>>,
    pub projects: Option<Vec<String>>,
    pub boards: Option<Vec<String>>,
    pub due_date: Option<String>,
    pub priorities: Option<Vec<String>>,
    pub updated_since: Option<String>,
}

/// Performance metrics for user issues operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIssuesPerformance {
    /// Time taken for the operation in milliseconds
    pub duration_ms: u64,

    /// Whether user data hit the cache
    pub user_cache_hit: bool,

    /// Whether any filter metadata hit the cache
    pub metadata_cache_hit: bool,

    /// Number of JIRA API calls made
    pub api_calls: u32,

    /// Query complexity
    pub query_complexity: String,
}

/// Implementation of the get_user_issues tool
pub struct GetUserIssuesTool {
    jira_client: Arc<JiraClient>,
    semantic_mapper: Arc<SemanticMapper>,
    config: Arc<JiraConfig>,
    cache: Arc<MetadataCache>,
}

impl GetUserIssuesTool {
    /// Create a new get user issues tool
    pub fn new(
        jira_client: Arc<JiraClient>,
        config: Arc<JiraConfig>,
        cache: Arc<MetadataCache>,
    ) -> Self {
        let semantic_mapper =
            Arc::new(SemanticMapper::new(Arc::clone(&config), Arc::clone(&cache)));

        Self {
            jira_client,
            semantic_mapper,
            config,
            cache,
        }
    }

    /// Execute the get_user_issues tool
    #[instrument(skip(self), fields(
        username = params.username.as_deref(),
        status_filter = ?params.status_filter,
        issue_types = ?params.issue_types,
        project_filter = ?params.project_filter,
    ))]
    pub async fn execute(&self, params: GetUserIssuesParams) -> JiraMcpResult<GetUserIssuesResult> {
        let start_time = std::time::Instant::now();
        let mut api_calls = 0u32;
        let mut user_cache_hit = false;
        let mut metadata_cache_hit = false;

        info!("Executing get_user_issues tool");

        // Validate parameters
        self.validate_params(&params)?;

        // Resolve user (default to current user if none specified)
        let user_info = self
            .resolve_user(&params.username, &mut user_cache_hit)
            .await?;

        // Build filters and resolve them to JIRA terms
        let applied_filters = self.build_applied_filters(&params, &mut metadata_cache_hit)?;

        // Build JQL query
        let jql_result =
            self.build_user_issues_jql(&user_info.account_id, &params, &applied_filters)?;

        // Apply pagination
        let limit = params
            .limit
            .unwrap_or(self.config.max_search_results)
            .min(200) as usize;
        let start_at = params.start_at.unwrap_or(0) as usize;

        // Execute search
        let search_result = self
            .jira_client
            .search_issues_jql(&jql_result.jql, Some(start_at), Some(limit), None)
            .await?;

        api_calls += 1;
        let duration = start_time.elapsed();

        info!(
            "Found {} issues for user {} in {}ms",
            search_result.issues.len(),
            user_info.display_name,
            duration.as_millis()
        );

        // Warn about large result sets
        if search_result.total > 500 {
            warn!(
                "User {} has {} total issues. Consider adding more filters for better performance.",
                user_info.display_name, search_result.total
            );
        }

        Ok(GetUserIssuesResult {
            search_result,
            resolved_user: user_info,
            jql_query: jql_result.jql,
            applied_filters,
            performance: UserIssuesPerformance {
                duration_ms: duration.as_millis() as u64,
                user_cache_hit,
                metadata_cache_hit,
                api_calls,
                query_complexity: format!("{:?}", jql_result.complexity),
            },
        })
    }

    /// Validate user issues parameters
    fn validate_params(&self, params: &GetUserIssuesParams) -> JiraMcpResult<()> {
        // Validate limit
        if let Some(limit) = params.limit {
            if limit == 0 {
                return Err(JiraMcpError::invalid_param(
                    "limit",
                    "Limit must be greater than 0",
                ));
            }
            if limit > 200 {
                return Err(JiraMcpError::invalid_param(
                    "limit",
                    "Limit cannot exceed 200",
                ));
            }
        }

        // Validate start_at
        if let Some(start_at) = params.start_at {
            if start_at > 10000 {
                return Err(JiraMcpError::invalid_param(
                    "start_at",
                    "start_at cannot exceed 10000",
                ));
            }
        }

        // Allow empty arrays - treat as None (more AI-friendly)
        // Just validate content if arrays are non-empty
        if let Some(status_filter) = &params.status_filter {
            for status in status_filter {
                if status.trim().is_empty() {
                    return Err(JiraMcpError::invalid_param(
                        "status_filter",
                        "status names cannot be empty",
                    ));
                }
            }
        }

        if let Some(issue_types) = &params.issue_types {
            for issue_type in issue_types {
                if issue_type.trim().is_empty() {
                    return Err(JiraMcpError::invalid_param(
                        "issue_types",
                        "issue type names cannot be empty",
                    ));
                }
            }
        }

        if let Some(board_filter) = &params.board_filter {
            for board in board_filter {
                if board.trim().is_empty() {
                    return Err(JiraMcpError::invalid_param(
                        "board_filter",
                        "board names cannot be empty",
                    ));
                }
            }
        }

        if let Some(project_filter) = &params.project_filter {
            for project in project_filter {
                if project.trim().is_empty() {
                    return Err(JiraMcpError::invalid_param(
                        "project_filter",
                        "project keys cannot be empty",
                    ));
                }
            }
        }

        if let Some(priority_filter) = &params.priority_filter {
            for priority in priority_filter {
                if priority.trim().is_empty() {
                    return Err(JiraMcpError::invalid_param(
                        "priority_filter",
                        "priority names cannot be empty",
                    ));
                }
            }
        }

        // Validate due date filter format
        if let Some(due_date) = &params.due_date_filter {
            if due_date.trim().is_empty() {
                return Err(JiraMcpError::invalid_param(
                    "due_date_filter",
                    "due_date_filter cannot be empty",
                ));
            }
        }

        Ok(())
    }

    /// Resolve user reference to user information
    async fn resolve_user(
        &self,
        username: &Option<String>,
        cache_hit: &mut bool,
    ) -> JiraMcpResult<UserInfo> {
        let user_ref = username.as_deref().unwrap_or("me");

        // Try to resolve from cache first
        if let Some(_account_id) = self.cache.resolve_user_reference(user_ref) {
            *cache_hit = true;

            // Get additional user info from cache
            if user_ref == "me" || user_ref == "current_user" {
                if let Some(current_user) = self.cache.get_current_user() {
                    return Ok(UserInfo {
                        account_id: current_user.account_id,
                        display_name: current_user.display_name,
                        email_address: current_user.email_address,
                        is_current_user: true,
                    });
                }
            } else if let Some(user_mapping) = self.cache.get_user_mapping(user_ref) {
                return Ok(UserInfo {
                    account_id: user_mapping.account_id,
                    display_name: user_mapping.display_name,
                    email_address: user_mapping.email_address,
                    is_current_user: false,
                });
            }
        }

        // Cache miss - need to resolve via API
        *cache_hit = false;

        if user_ref == "me" || user_ref == "current_user" {
            // Get current user
            let user_info = self.jira_client.get_current_user().await?;

            Ok(UserInfo {
                account_id: user_info.account_id,
                display_name: user_info.display_name,
                email_address: user_info.email_address,
                is_current_user: true,
            })
        } else {
            // Try to get user by identifier
            let user_info = self.jira_client.get_user_by_identifier(user_ref).await?;

            Ok(UserInfo {
                account_id: user_info.account_id,
                display_name: user_info.display_name,
                email_address: user_info.email_address,
                is_current_user: false,
            })
        }
    }

    /// Build applied filters by resolving semantic filters to JIRA terms
    fn build_applied_filters(
        &self,
        params: &GetUserIssuesParams,
        _cache_hit: &mut bool,
    ) -> JiraMcpResult<AppliedFilters> {
        // Convert empty arrays to None for better AI usability
        let project_filter = params
            .project_filter
            .as_ref()
            .filter(|arr| !arr.is_empty())
            .cloned();
        let board_filter = params
            .board_filter
            .as_ref()
            .filter(|arr| !arr.is_empty())
            .cloned();
        let priority_filter = params
            .priority_filter
            .as_ref()
            .filter(|arr| !arr.is_empty())
            .cloned();

        let mut applied_filters = AppliedFilters {
            status_categories: None,
            issue_types: None,
            projects: project_filter,
            boards: board_filter,
            due_date: params.due_date_filter.clone(),
            priorities: priority_filter,
            updated_since: params.updated_since.clone(),
        };

        // Resolve status categories (only if non-empty)
        if let Some(status_filter) = &params.status_filter {
            if !status_filter.is_empty() {
                let jira_statuses = self.semantic_mapper.map_status_categories(status_filter)?;
                applied_filters.status_categories = Some(jira_statuses);
            }
        }

        // Resolve issue types (only if non-empty)
        if let Some(issue_types) = &params.issue_types {
            if !issue_types.is_empty() {
                // Try to determine project context for better mapping
                let project_key = params
                    .project_filter
                    .as_ref()
                    .and_then(|projects| projects.first());
                let jira_types = self
                    .semantic_mapper
                    .map_issue_types(issue_types, project_key.map(|s| s.as_str()))?;
                applied_filters.issue_types = Some(jira_types);
            }
        }

        // Note: Board resolution would require additional API calls, skipping for now
        // In a complete implementation, we'd resolve board names to project keys here

        Ok(applied_filters)
    }

    /// Build JQL query for user issues
    fn build_user_issues_jql(
        &self,
        account_id: &str,
        _params: &GetUserIssuesParams,
        applied_filters: &AppliedFilters,
    ) -> JiraMcpResult<crate::semantic_mapping::JqlQuery> {
        let mut jql_parts = vec![format!("assignee = \"{}\"", account_id)];

        // Add status filter
        if let Some(statuses) = &applied_filters.status_categories {
            if statuses.len() == 1 {
                jql_parts.push(format!("status = \"{}\"", statuses[0]));
            } else {
                let status_list = statuses
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql_parts.push(format!("status IN ({})", status_list));
            }
        }

        // Add issue type filter
        if let Some(types) = &applied_filters.issue_types {
            if types.len() == 1 {
                jql_parts.push(format!("issuetype = \"{}\"", types[0]));
            } else {
                let type_list = types
                    .iter()
                    .map(|t| format!("\"{}\"", t))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql_parts.push(format!("issuetype IN ({})", type_list));
            }
        }

        // Add project filter
        if let Some(projects) = &applied_filters.projects {
            if projects.len() == 1 {
                jql_parts.push(format!("project = \"{}\"", projects[0]));
            } else {
                let project_list = projects
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql_parts.push(format!("project IN ({})", project_list));
            }
        }

        // Add priority filter
        if let Some(priorities) = &applied_filters.priorities {
            if priorities.len() == 1 {
                jql_parts.push(format!("priority = \"{}\"", priorities[0]));
            } else {
                let priority_list = priorities
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql_parts.push(format!("priority IN ({})", priority_list));
            }
        }

        // Add due date filter
        if let Some(due_date) = &applied_filters.due_date {
            match due_date.to_lowercase().as_str() {
                "overdue" => jql_parts.push("due < now()".to_string()),
                "today" => jql_parts.push("due = now()".to_string()),
                "this_week" => {
                    jql_parts.push("due >= startOfWeek() AND due <= endOfWeek()".to_string())
                }
                "next_week" => {
                    jql_parts.push("due >= startOfWeek(1w) AND due <= endOfWeek(1w)".to_string())
                }
                _ => {
                    // Assume it's a date string
                    jql_parts.push(format!("due <= \"{}\"", due_date));
                }
            }
        }

        // Add updated since filter
        if let Some(updated_since) = &applied_filters.updated_since {
            if updated_since.to_lowercase() == "today" {
                jql_parts.push("updated >= startOfDay()".to_string());
            } else if updated_since.contains("ago") {
                // Parse relative date
                let date_filter = crate::semantic_mapping::parse_date_filter(updated_since)
                    .unwrap_or_else(|_| updated_since.clone());
                jql_parts.push(format!("updated >= \"{}\"", date_filter));
            } else {
                jql_parts.push(format!("updated >= \"{}\"", updated_since));
            }
        }

        // Order by updated date descending
        jql_parts.push("ORDER BY updated DESC".to_string());

        let jql = jql_parts.join(" AND ");

        // Determine complexity
        let complexity = if jql_parts.len() > 4 {
            crate::semantic_mapping::QueryComplexity::Complex
        } else if jql_parts.len() > 2 {
            crate::semantic_mapping::QueryComplexity::Moderate
        } else {
            crate::semantic_mapping::QueryComplexity::Simple
        };

        Ok(crate::semantic_mapping::JqlQuery {
            jql,
            estimated_results: None,
            complexity,
        })
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(dead_code)]
    fn create_test_params() -> GetUserIssuesParams {
        GetUserIssuesParams {
            username: Some("me".to_string()),
            status_filter: Some(vec!["open".to_string(), "in_progress".to_string()]),
            issue_types: Some(vec!["story".to_string(), "bug".to_string()]),
            board_filter: None,
            project_filter: Some(vec!["TEST".to_string()]),
            due_date_filter: Some("overdue".to_string()),
            priority_filter: Some(vec!["high".to_string()]),
            updated_since: Some("7 days ago".to_string()),
            limit: Some(50),
            start_at: Some(0),
        }
    }

    // #[test]
    // fn test_param_validation_success() {
    //     // Disabled: Uses unsafe std::mem::zeroed which causes undefined behavior
    //     // TODO: Implement proper mocking for JiraClient
    // }

    // #[test]
    // fn test_param_validation_invalid_limit() {
    //     // Disabled: Uses unsafe std::mem::zeroed which causes undefined behavior
    //     // TODO: Implement proper mocking for JiraClient
    // }

    // #[test]
    // fn test_param_validation_empty_arrays() {
    //     // Disabled: Uses unsafe std::mem::zeroed which causes undefined behavior
    //     // TODO: Implement proper mocking for JiraClient
    // }
}
