//! Search issues tool with semantic parameters
//!
//! This tool allows AI agents to search for JIRA issues using natural language
//! parameters instead of requiring knowledge of JQL syntax.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::{JiraClient, SearchResult};
use crate::semantic_mapping::{QueryComplexity, SemanticMapper};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// Parameters for the search_issues tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchIssuesParams {
    /// Natural language search text (optional)
    pub query_text: Option<String>,

    /// Semantic issue types (optional)
    /// Examples: ["story", "bug", "feature", "task", "capability"]
    pub issue_types: Option<Vec<String>>,

    /// User assignment filter (optional)
    /// Examples: "me", "current_user", username, or "unassigned"
    pub assigned_to: Option<String>,

    /// Project key for project-scoped search (optional)
    pub project_key: Option<String>,

    /// Board name for board-scoped search (optional)
    /// Note: This will be resolved to project_key if board info is cached
    pub board_name: Option<String>,

    /// Semantic status categories (optional)
    /// Examples: ["open", "in_progress", "done", "blocked"]
    #[serde(alias = "status_filter")]
    pub status: Option<Vec<String>>,

    /// Created after date filter (optional)
    /// Examples: "2024-01-01", "7 days ago", "2 weeks ago"
    pub created_after: Option<String>,

    /// Label filters (optional)
    pub labels: Option<Vec<String>>,

    /// Component filters (optional)
    /// Examples: ["Backend", "Frontend"], ["API"]
    pub components: Option<Vec<String>>,

    /// Parent issue filter (optional)
    /// Examples: "none" (no parent), "any" (has parent), "PROJ-123" (specific parent issue key)
    pub parent_filter: Option<String>,

    /// Epic link filter (optional)
    /// Examples: "none" (not in epic), "any" (in an epic), "PROJ-456" (specific epic key)
    pub epic_filter: Option<String>,

    /// Maximum results to return (optional, default: 50, max: 200)
    pub limit: Option<u32>,

    /// Starting offset for pagination (optional, default: 0)
    pub start_at: Option<u32>,
}

/// Result from the search_issues tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIssuesResult {
    /// The search results
    pub search_result: SearchResult,

    /// The JQL query that was executed
    pub jql_query: String,

    /// Query complexity indicator
    pub query_complexity: String,

    /// Performance information
    pub performance: SearchPerformance,
}

// Workaround for pulseengine-mcp-macros bug #62
// The macro uses format!("{:?}") instead of serde_json serialization
// Implement Display to return JSON format
impl std::fmt::Display for SearchIssuesResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(
                f,
                "{{\"error\": \"Failed to serialize SearchIssuesResult\"}}"
            ),
        }
    }
}

/// Performance metrics for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPerformance {
    /// Time taken for the search in milliseconds
    pub duration_ms: u64,

    /// Whether the query hit the cache
    pub cache_hit: bool,

    /// Number of JIRA API calls made
    pub api_calls: u32,

    /// Estimated result count (if available)
    pub estimated_total: Option<usize>,
}

/// Implementation of the search_issues tool
pub struct SearchIssuesTool {
    jira_client: Arc<JiraClient>,
    semantic_mapper: Arc<SemanticMapper>,
    config: Arc<JiraConfig>,
    cache: Arc<MetadataCache>,
}

impl SearchIssuesTool {
    /// Create a new search issues tool
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

    /// Execute the search_issues tool
    #[instrument(skip(self), fields(
        query_text = params.query_text.as_deref(),
        issue_types = ?params.issue_types,
        assigned_to = params.assigned_to.as_deref(),
        project_key = params.project_key.as_deref(),
        board_name = params.board_name.as_deref(),
    ))]
    pub async fn execute(&self, params: SearchIssuesParams) -> JiraMcpResult<SearchIssuesResult> {
        let start_time = std::time::Instant::now();
        let mut api_calls = 0u32;
        let mut cache_hit = false;

        info!("Executing search_issues tool with parameters");

        // Validate parameters
        self.validate_params(&params)?;

        // Resolve board name to project key if specified
        let resolved_project_key = if let Some(board_name) = &params.board_name {
            if let Some(board_id) = self.cache.get_board_id(board_name) {
                if let Some(board_info) = self.cache.get_board_info(&board_id) {
                    cache_hit = true;
                    board_info.project_key.or(params.project_key.clone())
                } else {
                    // Board ID cached but info not cached
                    // In a complete implementation, we'd fetch board info here
                    warn!(
                        "Board ID cached but board info missing for board: {}",
                        board_name
                    );
                    params.project_key.clone()
                }
            } else {
                // Board not in cache
                // In a complete implementation, we'd search for the board here
                warn!(
                    "Board '{}' not found in cache, ignoring board filter",
                    board_name
                );
                params.project_key.clone()
            }
        } else {
            params.project_key.clone()
        };

        // Build JQL query using semantic mapper
        // Convert empty arrays to None for better AI usability
        let issue_types = params
            .issue_types
            .as_ref()
            .filter(|arr| !arr.is_empty())
            .map(|arr| arr.as_slice());
        let status = params
            .status
            .as_ref()
            .filter(|arr| !arr.is_empty())
            .map(|arr| arr.as_slice());
        let labels = params
            .labels
            .as_ref()
            .filter(|arr| !arr.is_empty())
            .map(|arr| arr.as_slice());
        let components = params
            .components
            .as_ref()
            .filter(|arr| !arr.is_empty())
            .map(|arr| arr.as_slice());

        let jql_result = self.semantic_mapper.build_search_jql_with_components(
            params.query_text.as_deref(),
            issue_types,
            params.assigned_to.as_deref(),
            resolved_project_key.as_deref(),
            status,
            params.created_after.as_deref(),
            labels,
            components,
            params.parent_filter.as_deref(),
            params.epic_filter.as_deref(),
        )?;

        // Apply pagination
        let limit = params
            .limit
            .unwrap_or(self.config.max_search_results)
            .min(200) as usize;
        let start_at = params.start_at.unwrap_or(0) as usize;

        // Execute search
        let search_result = self
            .jira_client
            .search_issues_jql(
                &jql_result.jql,
                Some(start_at),
                Some(limit),
                None, // No expand for basic search
            )
            .await?;

        api_calls += 1;
        let duration = start_time.elapsed();

        // Log performance information
        info!(
            "Search completed in {}ms, found {} issues (total: {})",
            duration.as_millis(),
            search_result.issues.len(),
            search_result.total
        );

        // Check if we should warn about large result sets
        if search_result.total > 1000 {
            warn!(
                "Large result set ({} total issues). Consider adding more specific filters.",
                search_result.total
            );
        }

        let total = search_result.total;

        Ok(SearchIssuesResult {
            search_result,
            jql_query: jql_result.jql,
            query_complexity: self.complexity_to_string(&jql_result.complexity),
            performance: SearchPerformance {
                duration_ms: duration.as_millis() as u64,
                cache_hit,
                api_calls,
                estimated_total: Some(total),
            },
        })
    }

    /// Validate search parameters
    fn validate_params(&self, params: &SearchIssuesParams) -> JiraMcpResult<()> {
        // Check limit
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
                    "Limit cannot exceed 200 per JIRA API restrictions",
                ));
            }
        }

        // Check start_at
        if let Some(start_at) = params.start_at {
            if start_at > 10000 {
                return Err(JiraMcpError::invalid_param(
                    "start_at",
                    "start_at cannot exceed 10000 per JIRA API restrictions",
                ));
            }
        }

        // Validate issue types (if specified and non-empty)
        if let Some(issue_types) = &params.issue_types {
            // Allow empty arrays - treat as None
            if !issue_types.is_empty() {
                for issue_type in issue_types {
                    if issue_type.trim().is_empty() {
                        return Err(JiraMcpError::invalid_param(
                            "issue_types",
                            "issue_type names cannot be empty",
                        ));
                    }
                }
            }
        }

        // Validate status (if specified and non-empty)
        if let Some(statuses) = &params.status {
            // Allow empty arrays - treat as None
            if !statuses.is_empty() {
                for status in statuses {
                    if status.trim().is_empty() {
                        return Err(JiraMcpError::invalid_param(
                            "status",
                            "status names cannot be empty",
                        ));
                    }
                }
            }
        }

        // Validate query_text length
        if let Some(query_text) = &params.query_text {
            if query_text.len() > 1000 {
                return Err(JiraMcpError::invalid_param(
                    "query_text",
                    "query_text cannot exceed 1000 characters",
                ));
            }
        }

        // Validate labels (if specified and non-empty)
        if let Some(labels) = &params.labels {
            // Allow empty arrays - treat as None
            if !labels.is_empty() {
                for label in labels {
                    if label.trim().is_empty() {
                        return Err(JiraMcpError::invalid_param(
                            "labels",
                            "label names cannot be empty",
                        ));
                    }
                    if label.contains(' ') {
                        return Err(JiraMcpError::invalid_param(
                            "labels",
                            format!("label '{}' cannot contain spaces", label),
                        ));
                    }
                }
            }
        }

        // Validate components (if specified and non-empty)
        if let Some(components) = &params.components {
            // Allow empty arrays - treat as None
            if !components.is_empty() {
                for component in components {
                    if component.trim().is_empty() {
                        return Err(JiraMcpError::invalid_param(
                            "components",
                            "component names cannot be empty",
                        ));
                    }
                }
            }
        }

        // Validate that at least one search criterion is provided
        // (unless it's a general "list all" query)
        let has_criteria = params.query_text.is_some()
            || params.issue_types.is_some()
            || params.assigned_to.is_some()
            || params.project_key.is_some()
            || params.board_name.is_some()
            || params.status.is_some()
            || params.created_after.is_some()
            || params.labels.is_some()
            || params.components.is_some();

        if !has_criteria {
            warn!("No search criteria provided, will return recent issues");
        }

        Ok(())
    }

    /// Convert QueryComplexity to string
    fn complexity_to_string(&self, complexity: &QueryComplexity) -> String {
        match complexity {
            QueryComplexity::Simple => "simple".to_string(),
            QueryComplexity::Moderate => "moderate".to_string(),
            QueryComplexity::Complex => "complex".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::config::JiraConfig;

    #[allow(dead_code)]
    fn create_test_params() -> SearchIssuesParams {
        SearchIssuesParams {
            query_text: Some("test query".to_string()),
            issue_types: Some(vec!["story".to_string(), "bug".to_string()]),
            assigned_to: Some("me".to_string()),
            project_key: Some("TEST".to_string()),
            board_name: None,
            status: Some(vec!["open".to_string(), "in_progress".to_string()]),
            created_after: Some("7 days ago".to_string()),
            labels: Some(vec!["urgent".to_string()]),
            components: Some(vec!["Backend".to_string()]),
            parent_filter: None,
            epic_filter: None,
            limit: Some(50),
            start_at: Some(0),
        }
    }

    /*
    // All tests disabled due to unsafe std::mem::zeroed usage
    // TODO: Implement proper mocking for tests
     */
}
