//! Semantic mapping system for translating AI-friendly parameters to JIRA concepts
//!
//! This module provides the core functionality for converting semantic parameters
//! like "story", "bug", "in_progress" to actual JIRA issue types and statuses,
//! and for building JQL queries from natural language parameters.

use crate::cache::{IssueTypeInfo, MetadataCache};
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, warn};

/// Semantic mapper that converts AI-friendly parameters to JIRA concepts
#[derive(Debug)]
pub struct SemanticMapper {
    config: Arc<JiraConfig>,
    cache: Arc<MetadataCache>,
}

/// JQL query builder result
#[derive(Debug, Clone)]
pub struct JqlQuery {
    pub jql: String,
    pub estimated_results: Option<usize>,
    pub complexity: QueryComplexity,
}

/// Query complexity indicator
#[derive(Debug, Clone, PartialEq)]
pub enum QueryComplexity {
    Simple,   // Single condition
    Moderate, // 2-3 conditions
    Complex,  // 4+ conditions or text search
}

impl SemanticMapper {
    /// Create a new semantic mapper
    pub fn new(config: Arc<JiraConfig>, cache: Arc<MetadataCache>) -> Self {
        Self { config, cache }
    }

    /// Map semantic issue types to JIRA issue type names
    pub fn map_issue_types(
        &self,
        semantic_types: &[String],
        project_key: Option<&str>,
    ) -> JiraMcpResult<Vec<String>> {
        let mut jira_types = Vec::new();

        for semantic_type in semantic_types {
            let semantic_lower = semantic_type.to_lowercase();

            // First try project-specific issue types from cache
            if let Some(project) = project_key {
                if let Some(project_types) = self.cache.get_project_issue_types(project) {
                    let matching_types =
                        self.find_matching_project_types(&semantic_lower, &project_types);
                    if !matching_types.is_empty() {
                        jira_types.extend(matching_types);
                        continue;
                    }
                }
            }

            // Fall back to configured mappings
            if let Some(mapped_types) = self.config.issue_type_mappings.get(&semantic_lower) {
                jira_types.extend(mapped_types.clone());
            } else {
                // Last resort: use the semantic type as-is (capitalized)
                let capitalized = capitalize_first(semantic_type);
                warn!(
                    "No mapping found for issue type '{}', using '{}'",
                    semantic_type, capitalized
                );
                jira_types.push(capitalized);
            }
        }

        // Remove duplicates while preserving order
        let unique_types: Vec<String> = jira_types
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        debug!(
            "Mapped semantic types {:?} to JIRA types {:?}",
            semantic_types, unique_types
        );
        Ok(unique_types)
    }

    /// Map semantic status categories to JIRA status names
    pub fn map_status_categories(
        &self,
        semantic_statuses: &[String],
    ) -> JiraMcpResult<Vec<String>> {
        let mut jira_statuses = Vec::new();

        for semantic_status in semantic_statuses {
            let semantic_lower = semantic_status.to_lowercase();

            // Try configured mappings
            if let Some(mapped_statuses) = self.config.status_category_mappings.get(&semantic_lower)
            {
                jira_statuses.extend(mapped_statuses.clone());
            } else {
                // Use as-is (capitalized)
                let capitalized = capitalize_first(semantic_status);
                warn!(
                    "No mapping found for status '{}', using '{}'",
                    semantic_status, capitalized
                );
                jira_statuses.push(capitalized);
            }
        }

        // Remove duplicates
        let unique_statuses: Vec<String> = jira_statuses
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        debug!(
            "Mapped semantic statuses {:?} to JIRA statuses {:?}",
            semantic_statuses, unique_statuses
        );
        Ok(unique_statuses)
    }

    /// Resolve user reference to account ID
    pub fn resolve_user_reference(&self, user_ref: &str) -> JiraMcpResult<String> {
        match user_ref.to_lowercase().as_str() {
            "me" | "current_user" | "currentuser" => self
                .cache
                .get_current_user()
                .map(|user| user.account_id)
                .ok_or_else(|| {
                    JiraMcpError::auth(
                        "Current user not found in cache. Authentication may have failed.",
                    )
                }),
            "unassigned" => {
                // For unassigned, we'll return a special marker
                Ok("UNASSIGNED".to_string())
            }
            _ => {
                // Try to resolve from cache
                if let Some(user) = self.cache.get_user_mapping(user_ref) {
                    Ok(user.account_id)
                } else {
                    // Return the identifier as-is for now
                    // In a real implementation, we might want to fetch user info
                    warn!("User reference '{}' not found in cache", user_ref);
                    Ok(user_ref.to_string())
                }
            }
        }
    }

    /// Build a JQL query from search parameters
    #[allow(clippy::too_many_arguments)]
    pub fn build_search_jql(
        &self,
        query_text: Option<&str>,
        issue_types: Option<&[String]>,
        assigned_to: Option<&str>,
        project_key: Option<&str>,
        status: Option<&[String]>,
        created_after: Option<&str>,
        labels: Option<&[String]>,
        parent_filter: Option<&str>,
        epic_filter: Option<&str>,
    ) -> JiraMcpResult<JqlQuery> {
        let mut jql_parts = Vec::new();
        let mut complexity = QueryComplexity::Simple;

        // Project filter (if specified)
        if let Some(project) = project_key {
            jql_parts.push(format!("project = \"{}\"", project));
        }

        // Text search (if specified)
        if let Some(text) = query_text {
            if !text.trim().is_empty() {
                // Use JIRA text search
                jql_parts.push(format!("text ~ \"{}\"", escape_jql_string(text)));
                complexity = QueryComplexity::Complex;
            }
        }

        // Issue types
        if let Some(types) = issue_types {
            if !types.is_empty() {
                let jira_types = self.map_issue_types(types, project_key)?;
                if !jira_types.is_empty() {
                    let types_clause = if jira_types.len() == 1 {
                        format!("issuetype = \"{}\"", jira_types[0])
                    } else {
                        let type_list = jira_types
                            .iter()
                            .map(|t| format!("\"{}\"", t))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("issuetype IN ({})", type_list)
                    };
                    jql_parts.push(types_clause);
                }
            }
        }

        // Assignee
        if let Some(assignee) = assigned_to {
            let resolved_user = self.resolve_user_reference(assignee)?;
            let assignee_clause = if resolved_user == "UNASSIGNED" {
                "assignee is EMPTY".to_string()
            } else {
                format!("assignee = \"{}\"", resolved_user)
            };
            jql_parts.push(assignee_clause);
        }

        // Status
        if let Some(statuses) = status {
            if !statuses.is_empty() {
                let jira_statuses = self.map_status_categories(statuses)?;
                if !jira_statuses.is_empty() {
                    let status_clause = if jira_statuses.len() == 1 {
                        format!("status = \"{}\"", jira_statuses[0])
                    } else {
                        let status_list = jira_statuses
                            .iter()
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("status IN ({})", status_list)
                    };
                    jql_parts.push(status_clause);
                }
            }
        }

        // Created after
        if let Some(created) = created_after {
            let date_clause = parse_date_filter(created)?;
            jql_parts.push(format!("created >= \"{}\"", date_clause));
        }

        // Labels
        if let Some(label_list) = labels {
            if !label_list.is_empty() {
                for label in label_list {
                    jql_parts.push(format!("labels = \"{}\"", label));
                }
            }
        }

        // Parent filter
        if let Some(parent) = parent_filter {
            let parent_clause = match parent.to_lowercase().as_str() {
                "none" => "parent is EMPTY".to_string(),
                "any" => "parent is not EMPTY".to_string(),
                issue_key => format!("parent = \"{}\"", issue_key),
            };
            jql_parts.push(parent_clause);
        }

        // Epic link filter
        if let Some(epic) = epic_filter {
            let epic_clause = match epic.to_lowercase().as_str() {
                "none" => "\"Epic Link\" is EMPTY".to_string(),
                "any" => "\"Epic Link\" is not EMPTY".to_string(),
                epic_key => format!("\"Epic Link\" = \"{}\"", epic_key),
            };
            jql_parts.push(epic_clause);
        }

        // Determine complexity
        if jql_parts.len() > 3 {
            complexity = QueryComplexity::Complex;
        } else if jql_parts.len() > 1 {
            complexity = QueryComplexity::Moderate;
        }

        // Build final JQL
        let jql = if jql_parts.is_empty() {
            // Default query if no filters
            "ORDER BY updated DESC".to_string()
        } else {
            let conditions = jql_parts.join(" AND ");
            format!("{} ORDER BY updated DESC", conditions)
        };

        debug!("Built JQL query: {}", jql);

        Ok(JqlQuery {
            jql,
            estimated_results: None, // Could be populated with estimate logic
            complexity,
        })
    }

    /// Build JQL for user-assigned issues
    pub fn build_user_issues_jql(
        &self,
        user_ref: Option<&str>,
        status_filter: Option<&[String]>,
        issue_types: Option<&[String]>,
        project_key: Option<&str>,
    ) -> JiraMcpResult<JqlQuery> {
        let user_account_id = if let Some(user) = user_ref {
            self.resolve_user_reference(user)?
        } else {
            // Default to current user
            self.cache
                .get_current_user()
                .map(|u| u.account_id)
                .ok_or_else(|| {
                    JiraMcpError::auth("No user specified and current user not available")
                })?
        };

        self.build_search_jql(
            None,
            issue_types,
            Some(&user_account_id),
            project_key,
            status_filter,
            None,
            None,
            None,
            None,
        )
    }

    /// Find matching issue types in project-specific types
    fn find_matching_project_types(
        &self,
        semantic_type: &str,
        project_types: &[IssueTypeInfo],
    ) -> Vec<String> {
        let mut matches = Vec::new();

        for issue_type in project_types {
            let type_name_lower = issue_type.name.to_lowercase();

            // Direct match
            if type_name_lower == semantic_type {
                matches.push(issue_type.name.clone());
                continue;
            }

            // Partial match for common patterns
            match semantic_type {
                "story" => {
                    if type_name_lower.contains("story") {
                        matches.push(issue_type.name.clone());
                    }
                }
                "bug" => {
                    if type_name_lower.contains("bug") || type_name_lower.contains("defect") {
                        matches.push(issue_type.name.clone());
                    }
                }
                "feature" => {
                    if type_name_lower.contains("feature") {
                        matches.push(issue_type.name.clone());
                    }
                }
                "task" => {
                    if type_name_lower.contains("task") {
                        matches.push(issue_type.name.clone());
                    }
                }
                "capability" | "epic" => {
                    if type_name_lower.contains("epic") || type_name_lower.contains("capability") {
                        matches.push(issue_type.name.clone());
                    }
                }
                _ => {}
            }
        }

        matches
    }
}

/// Capitalize the first letter of a string
fn capitalize_first(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }

    let mut chars: Vec<char> = s.chars().collect();
    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
    chars.into_iter().collect()
}

/// Escape special characters in JQL string literals
fn escape_jql_string(s: &str) -> String {
    s.replace("\\", "\\\\").replace("\"", "\\\"")
}

/// Parse date filter string to JQL-compatible format
pub fn parse_date_filter(date_str: &str) -> JiraMcpResult<String> {
    let trimmed = date_str.trim().to_lowercase();

    // Handle relative dates
    if trimmed.contains("ago") {
        // Parse patterns like "7 days ago", "2 weeks ago", "1 month ago"
        if let Some(relative_date) = parse_relative_date(&trimmed) {
            return Ok(relative_date);
        }
    }

    // Handle absolute dates (try to parse as ISO date)
    if trimmed.len() >= 10 && trimmed.contains("-") {
        // Assume YYYY-MM-DD format, return as-is for JQL
        return Ok(date_str.to_string());
    }

    // Default fallback
    Err(JiraMcpError::invalid_param(
        "created_after",
        format!(
            "Invalid date format: '{}'. Use formats like '2024-01-01' or '7 days ago'",
            date_str
        ),
    ))
}

/// Parse relative date strings like "7 days ago"
fn parse_relative_date(date_str: &str) -> Option<String> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() >= 3 && parts[parts.len() - 1] == "ago" {
        if let Ok(num) = parts[0].parse::<u32>() {
            let unit = parts[1];
            match unit {
                "day" | "days" => Some(format!("-{}d", num)),
                "week" | "weeks" => Some(format!("-{}w", num)),
                "month" | "months" => Some(format!("-{}M", num)),
                "year" | "years" => Some(format!("-{}y", num)),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, JiraConfig};

    fn create_test_config() -> Arc<JiraConfig> {
        Arc::new(JiraConfig {
            jira_url: "https://test.atlassian.net".to_string(),
            auth: AuthConfig::Anonymous,
            ..Default::default()
        })
    }

    #[test]
    fn test_issue_type_mapping() {
        let config = create_test_config();
        let cache = Arc::new(MetadataCache::new(300));
        let mapper = SemanticMapper::new(config, cache);

        let semantic_types = vec!["story".to_string(), "bug".to_string()];
        let jira_types = mapper.map_issue_types(&semantic_types, None).unwrap();

        assert!(jira_types.contains(&"Story".to_string()));
        assert!(jira_types.contains(&"Bug".to_string()));
    }

    #[test]
    fn test_status_mapping() {
        let config = create_test_config();
        let cache = Arc::new(MetadataCache::new(300));
        let mapper = SemanticMapper::new(config, cache);

        let semantic_statuses = vec!["open".to_string(), "in_progress".to_string()];
        let jira_statuses = mapper.map_status_categories(&semantic_statuses).unwrap();

        assert!(jira_statuses.contains(&"Open".to_string()));
        assert!(jira_statuses.contains(&"In Progress".to_string()));
    }

    #[test]
    fn test_jql_building() {
        let config = create_test_config();
        let cache = Arc::new(MetadataCache::new(300));
        let mapper = SemanticMapper::new(config, cache);

        let query = mapper
            .build_search_jql(
                Some("test query"),
                Some(&["story".to_string()]),
                None,
                Some("TEST"),
                Some(&["open".to_string()]),
                None,
                None,
                None,
                None,
            )
            .unwrap();

        assert!(query.jql.contains("project = \"TEST\""));
        assert!(query.jql.contains("text ~ \"test query\""));
        assert!(query.jql.contains("issuetype"));
        assert!(query.jql.contains("status"));
        assert_eq!(query.complexity, QueryComplexity::Complex);
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("test"), "Test");
        assert_eq!(capitalize_first("TEST"), "TEST");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }

    #[test]
    fn test_escape_jql_string() {
        assert_eq!(escape_jql_string("test\"quote"), "test\\\"quote");
        assert_eq!(escape_jql_string("test\\backslash"), "test\\\\backslash");
        assert_eq!(escape_jql_string("normal text"), "normal text");
    }

    #[test]
    fn test_relative_date_parsing() {
        assert_eq!(parse_relative_date("7 days ago"), Some("-7d".to_string()));
        assert_eq!(parse_relative_date("2 weeks ago"), Some("-2w".to_string()));
        assert_eq!(parse_relative_date("1 month ago"), Some("-1M".to_string()));
        assert_eq!(parse_relative_date("invalid format"), None);
    }

    #[test]
    fn test_date_filter_parsing() {
        assert_eq!(parse_date_filter("2024-01-01").unwrap(), "2024-01-01");
        assert_eq!(parse_date_filter("7 days ago").unwrap(), "-7d");
        assert!(parse_date_filter("invalid").is_err());
    }
}
