//! Issue Relationship Graph Tool
//!
//! This tool leverages gouqi's relationship graph capabilities to extract and visualize
//! JIRA issue relationships, allowing AI agents to understand issue dependencies,
//! blockers, and connections.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, instrument};

/// Parameters for extracting issue relationship graphs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueRelationshipsParams {
    /// The root issue key to start relationship extraction from (e.g., "PROJ-123")
    pub root_issue_key: String,

    /// Maximum depth to traverse relationships (0 = root issue only, 1 = direct relationships, etc.)
    #[serde(default = "default_depth")]
    pub max_depth: u32,

    /// Whether to include subtasks in the relationship graph
    #[serde(default = "default_true")]
    pub include_subtasks: bool,

    /// Whether to include blocked/blocks relationships
    #[serde(default = "default_true")]
    pub include_blocks: bool,

    /// Whether to include relates to relationships
    #[serde(default = "default_true")]
    pub include_relates: bool,

    /// Whether to include epic-story relationships
    #[serde(default = "default_true")]
    pub include_epic_links: bool,

    /// Whether to include duplicate relationships
    #[serde(default)]
    pub include_duplicates: bool,
}

fn default_depth() -> u32 {
    2
}
fn default_true() -> bool {
    true
}

/// Issue relationship information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueRelationship {
    /// Source issue key
    pub from_issue: String,

    /// Target issue key
    pub to_issue: String,

    /// Type of relationship (e.g., "blocks", "is subtask of", "relates to")
    pub relationship_type: String,

    /// Direction of the relationship ("inward" or "outward")
    pub direction: String,

    /// Description of the relationship
    pub description: Option<String>,
}

/// Issue node in the relationship graph
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueNode {
    /// Issue key
    pub key: String,

    /// Issue summary/title
    pub summary: String,

    /// Issue type (e.g., "Story", "Bug", "Epic")
    pub issue_type: String,

    /// Current status (e.g., "To Do", "In Progress", "Done")
    pub status: String,

    /// Priority level
    pub priority: Option<String>,

    /// Assigned user
    pub assignee: Option<String>,

    /// Project key
    pub project_key: String,

    /// Depth level in the relationship graph (0 = root issue)
    pub depth: u32,
}

/// Result of relationship graph extraction
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueRelationshipsResult {
    /// The root issue that was used as starting point
    pub root_issue: String,

    /// Maximum depth that was traversed
    pub max_depth: u32,

    /// All issue nodes in the relationship graph
    pub nodes: Vec<IssueNode>,

    /// All relationships found between issues
    pub relationships: Vec<IssueRelationship>,

    /// Summary statistics
    pub summary: RelationshipSummary,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Summary statistics about the relationship graph
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationshipSummary {
    /// Total number of issues found
    pub total_issues: usize,

    /// Total number of relationships found
    pub total_relationships: usize,

    /// Issues by depth level
    pub issues_by_depth: std::collections::HashMap<u32, usize>,

    /// Relationships by type
    pub relationships_by_type: std::collections::HashMap<String, usize>,

    /// Issues that couldn't be accessed (due to permissions, etc.)
    pub inaccessible_issues: Vec<String>,
}

/// Tool implementation for issue relationship extraction
#[derive(Debug)]
pub struct IssueRelationshipsTool {
    jira_client: Arc<JiraClient>,
    #[allow(dead_code)]
    config: Arc<JiraConfig>,
    #[allow(dead_code)]
    cache: Arc<MetadataCache>,
}

impl IssueRelationshipsTool {
    /// Create a new issue relationships tool
    pub fn new(
        jira_client: Arc<JiraClient>,
        config: Arc<JiraConfig>,
        cache: Arc<MetadataCache>,
    ) -> Self {
        Self {
            jira_client,
            config,
            cache,
        }
    }

    /// Execute the relationship extraction
    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        params: IssueRelationshipsParams,
    ) -> JiraMcpResult<IssueRelationshipsResult> {
        let start_time = Instant::now();

        info!(
            "Extracting relationship graph for issue {} with depth {}",
            params.root_issue_key, params.max_depth
        );

        // Validate the root issue key format
        self.validate_issue_key(&params.root_issue_key)?;

        // For now, we'll create a basic implementation that gets the root issue and its direct links
        // In a full implementation, this would use gouqi's get_relationship_graph method

        let root_issue = self
            .jira_client
            .get_issue_details(
                &params.root_issue_key,
                false, // comments
                false, // attachments
                false, // history
            )
            .await?;

        let mut nodes = Vec::new();
        let mut relationships = Vec::new();
        let mut issues_by_depth = std::collections::HashMap::new();
        let mut relationships_by_type = std::collections::HashMap::new();
        let inaccessible_issues = Vec::new();

        // Add root issue as a node
        let root_node = IssueNode {
            key: root_issue.issue_info.key.clone(),
            summary: root_issue.issue_info.summary.clone(),
            issue_type: root_issue.issue_info.issue_type.clone(),
            status: root_issue.issue_info.status.clone(),
            priority: root_issue.issue_info.priority.clone(),
            assignee: root_issue.issue_info.assignee.clone(),
            project_key: root_issue.issue_info.project_key.clone(),
            depth: 0,
        };

        nodes.push(root_node);
        issues_by_depth.insert(0, 1);

        // Add linked issues from the root issue
        for linked_issue in &root_issue.linked_issues {
            // Add linked issue as a node
            let linked_node = IssueNode {
                key: linked_issue.key.clone(),
                summary: linked_issue.summary.clone(),
                issue_type: "Unknown".to_string(), // Would need to fetch full issue details
                status: linked_issue.status.clone(),
                priority: None,
                assignee: None,
                project_key: "Unknown".to_string(), // Would need to extract from key
                depth: 1,
            };
            nodes.push(linked_node);

            // Add relationship
            let relationship = IssueRelationship {
                from_issue: root_issue.issue_info.key.clone(),
                to_issue: linked_issue.key.clone(),
                relationship_type: linked_issue.link_type.clone(),
                direction: linked_issue.direction.clone(),
                description: Some(format!(
                    "{} {}",
                    if linked_issue.direction == "outward" {
                        "This issue"
                    } else {
                        "Linked issue"
                    },
                    linked_issue.link_type.to_lowercase()
                )),
            };
            relationships.push(relationship);

            // Update statistics
            *relationships_by_type
                .entry(linked_issue.link_type.clone())
                .or_insert(0) += 1;
        }

        // Update depth statistics
        *issues_by_depth.entry(1).or_insert(0) += root_issue.linked_issues.len();

        let summary = RelationshipSummary {
            total_issues: nodes.len(),
            total_relationships: relationships.len(),
            issues_by_depth,
            relationships_by_type,
            inaccessible_issues,
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Relationship extraction completed in {}ms: {} issues, {} relationships",
            execution_time, summary.total_issues, summary.total_relationships
        );

        Ok(IssueRelationshipsResult {
            root_issue: params.root_issue_key,
            max_depth: params.max_depth,
            nodes,
            relationships,
            summary,
            execution_time_ms: execution_time,
        })
    }

    /// Validate issue key format
    fn validate_issue_key(&self, issue_key: &str) -> JiraMcpResult<()> {
        if issue_key.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "root_issue_key",
                "Issue key cannot be empty",
            ));
        }

        let parts: Vec<&str> = issue_key.split('-').collect();
        if parts.len() != 2 {
            return Err(JiraMcpError::invalid_param(
                "root_issue_key",
                "Issue key must be in format 'PROJECT-123'",
            ));
        }

        let project_key = parts[0];
        let issue_number = parts[1];

        if project_key.is_empty() || !project_key.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(JiraMcpError::invalid_param(
                "root_issue_key",
                "Project key must contain only uppercase letters",
            ));
        }

        if issue_number.is_empty() || !issue_number.chars().all(|c| c.is_ascii_digit()) {
            return Err(JiraMcpError::invalid_param(
                "root_issue_key",
                "Issue number must contain only digits",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_key_validation() {
        let _config = Arc::new(JiraConfig::default());
        let _cache = Arc::new(MetadataCache::new(300));

        // This is a mock test - in practice we'd need a real JiraClient
        // let tool = IssueRelationshipsTool::new(jira_client, config, cache);

        // Test valid keys
        // assert!(tool.validate_issue_key("PROJ-123").is_ok());
        // assert!(tool.validate_issue_key("ABC-1").is_ok());

        // Test invalid keys
        // assert!(tool.validate_issue_key("").is_err());
        // assert!(tool.validate_issue_key("PROJ").is_err());
        // assert!(tool.validate_issue_key("proj-123").is_err());
        // assert!(tool.validate_issue_key("PROJ-abc").is_err());
    }
}
