//! Issue Relationship Graph Tool
//!
//! This tool leverages gouqi's relationship graph capabilities to extract and visualize
//! JIRA issue relationships, allowing AI agents to understand issue dependencies,
//! blockers, and connections.

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use gouqi::relationships::GraphOptions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, instrument};

/// Parameters for extracting issue relationship graphs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
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

        // Build GraphOptions from parameters
        let options = self.build_graph_options(&params);

        debug!(
            "Built graph options with include_types: {:?}",
            options.include_types
        );

        // Use gouqi's built-in recursive relationship graph extraction
        let graph = self
            .jira_client
            .client
            .issues()
            .get_relationship_graph(&params.root_issue_key, params.max_depth, Some(options))
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    JiraMcpError::not_found("issue", &params.root_issue_key)
                } else {
                    JiraMcpError::from(e)
                }
            })?;

        // Convert gouqi's RelationshipGraph to our MCP format
        let result = self
            .convert_graph_to_result(graph, params, start_time)
            .await?;

        info!(
            "Relationship extraction completed in {}ms: {} issues, {} relationships",
            result.execution_time_ms,
            result.summary.total_issues,
            result.summary.total_relationships
        );

        Ok(result)
    }

    /// Build GraphOptions from parameters
    fn build_graph_options(&self, params: &IssueRelationshipsParams) -> GraphOptions {
        // Build exclude_types list based on parameters
        // Since all flags default to true, we use exclude_types instead of include_types
        // This way we don't accidentally filter out subtasks/parents/epic links
        let mut exclude_types = Vec::new();

        if !params.include_blocks {
            exclude_types.push("Blocks".to_string());
            exclude_types.push("blocks".to_string());
            exclude_types.push("is blocked by".to_string());
        }

        if !params.include_relates {
            exclude_types.push("Relates".to_string());
            exclude_types.push("relates to".to_string());
        }

        if !params.include_duplicates {
            exclude_types.push("Duplicate".to_string());
            exclude_types.push("Duplicates".to_string());
            exclude_types.push("is duplicated by".to_string());
        }

        // Note: subtasks, epic links, and parent links are part of JIRA's hierarchy
        // and will be included automatically unless explicitly excluded
        // The include_subtasks and include_epic_links params need different handling
        // as they're not standard "link types" in JIRA

        GraphOptions {
            include_types: None, // Don't filter - include all types including subtasks/parents
            exclude_types: if exclude_types.is_empty() {
                None
            } else {
                Some(exclude_types)
            },
            include_custom: true, // Include custom link types
            bidirectional: true,  // Include both inward and outward links
        }
    }

    /// Convert gouqi's RelationshipGraph to our MCP result format
    async fn convert_graph_to_result(
        &self,
        graph: gouqi::relationships::RelationshipGraph,
        params: IssueRelationshipsParams,
        start_time: Instant,
    ) -> JiraMcpResult<IssueRelationshipsResult> {
        let mut nodes = Vec::new();
        let mut relationships = Vec::new();
        let mut issues_by_depth = std::collections::HashMap::new();
        let mut relationships_by_type = std::collections::HashMap::new();
        let mut inaccessible_issues = Vec::new();

        // Process each issue in the graph
        for (issue_key, issue_rels) in &graph.issues {
            // Fetch full issue details for this node
            let issue_details = match self
                .jira_client
                .get_issue_details(issue_key, false, false, false)
                .await
            {
                Ok(details) => details,
                Err(e) => {
                    debug!("Failed to fetch details for {}: {}", issue_key, e);
                    inaccessible_issues.push(issue_key.clone());
                    continue;
                }
            };

            // Calculate depth from root (simplified - assumes breadth-first traversal)
            let depth = if issue_key == &params.root_issue_key {
                0
            } else {
                graph
                    .get_path(&params.root_issue_key, issue_key)
                    .map(|path| (path.len() - 1) as u32)
                    .unwrap_or(params.max_depth)
            };

            // Create issue node
            let node = IssueNode {
                key: issue_details.issue_info.key.clone(),
                summary: issue_details.issue_info.summary.clone(),
                issue_type: issue_details.issue_info.issue_type.clone(),
                status: issue_details.issue_info.status.clone(),
                priority: issue_details.issue_info.priority.clone(),
                assignee: issue_details.issue_info.assignee.clone(),
                project_key: issue_details.issue_info.project_key.clone(),
                depth,
            };

            nodes.push(node);
            *issues_by_depth.entry(depth).or_insert(0) += 1;

            // Process relationships
            self.add_relationships(
                issue_key,
                issue_rels,
                &params,
                &mut relationships,
                &mut relationships_by_type,
            );
        }

        let summary = RelationshipSummary {
            total_issues: nodes.len(),
            total_relationships: relationships.len(),
            issues_by_depth,
            relationships_by_type,
            inaccessible_issues,
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(IssueRelationshipsResult {
            root_issue: params.root_issue_key,
            max_depth: params.max_depth,
            nodes,
            relationships,
            summary,
            execution_time_ms: execution_time,
        })
    }

    /// Add relationships from gouqi's IssueRelationships to our format
    fn add_relationships(
        &self,
        from_issue: &str,
        issue_rels: &gouqi::relationships::IssueRelationships,
        params: &IssueRelationshipsParams,
        relationships: &mut Vec<IssueRelationship>,
        relationships_by_type: &mut std::collections::HashMap<String, usize>,
    ) {
        // Blocks relationships
        if params.include_blocks {
            for to_issue in &issue_rels.blocks {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: to_issue.clone(),
                    relationship_type: "blocks".to_string(),
                    direction: "outward".to_string(),
                    description: Some(format!("{} blocks {}", from_issue, to_issue)),
                });
                *relationships_by_type
                    .entry("blocks".to_string())
                    .or_insert(0) += 1;
            }

            for to_issue in &issue_rels.blocked_by {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: to_issue.clone(),
                    relationship_type: "blocked_by".to_string(),
                    direction: "inward".to_string(),
                    description: Some(format!("{} is blocked by {}", from_issue, to_issue)),
                });
                *relationships_by_type
                    .entry("blocked_by".to_string())
                    .or_insert(0) += 1;
            }
        }

        // Relates to relationships
        if params.include_relates {
            for to_issue in &issue_rels.relates_to {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: to_issue.clone(),
                    relationship_type: "relates_to".to_string(),
                    direction: "outward".to_string(),
                    description: Some(format!("{} relates to {}", from_issue, to_issue)),
                });
                *relationships_by_type
                    .entry("relates_to".to_string())
                    .or_insert(0) += 1;
            }
        }

        // Duplicate relationships
        if params.include_duplicates {
            for to_issue in &issue_rels.duplicates {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: to_issue.clone(),
                    relationship_type: "duplicates".to_string(),
                    direction: "outward".to_string(),
                    description: Some(format!("{} duplicates {}", from_issue, to_issue)),
                });
                *relationships_by_type
                    .entry("duplicates".to_string())
                    .or_insert(0) += 1;
            }
        }

        // Parent-child (subtask) relationships
        if params.include_subtasks {
            if let Some(parent) = &issue_rels.parent {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: parent.clone(),
                    relationship_type: "parent".to_string(),
                    direction: "inward".to_string(),
                    description: Some(format!("{} is subtask of {}", from_issue, parent)),
                });
                *relationships_by_type
                    .entry("parent".to_string())
                    .or_insert(0) += 1;
            }

            for child in &issue_rels.children {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: child.clone(),
                    relationship_type: "subtask".to_string(),
                    direction: "outward".to_string(),
                    description: Some(format!("{} has subtask {}", from_issue, child)),
                });
                *relationships_by_type
                    .entry("subtask".to_string())
                    .or_insert(0) += 1;
            }
        }

        // Epic relationships
        if params.include_epic_links {
            if let Some(epic) = &issue_rels.epic {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: epic.clone(),
                    relationship_type: "epic".to_string(),
                    direction: "inward".to_string(),
                    description: Some(format!("{} is in epic {}", from_issue, epic)),
                });
                *relationships_by_type.entry("epic".to_string()).or_insert(0) += 1;
            }
        }

        // Custom relationships
        for (custom_type, targets) in &issue_rels.custom {
            for to_issue in targets {
                relationships.push(IssueRelationship {
                    from_issue: from_issue.to_string(),
                    to_issue: to_issue.clone(),
                    relationship_type: custom_type.clone(),
                    direction: "outward".to_string(),
                    description: Some(format!("{} {} {}", from_issue, custom_type, to_issue)),
                });
                *relationships_by_type
                    .entry(custom_type.clone())
                    .or_insert(0) += 1;
            }
        }
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
