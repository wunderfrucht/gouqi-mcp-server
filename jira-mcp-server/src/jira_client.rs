//! JIRA client wrapper around gouqi
//!
//! Provides a higher-level interface to JIRA operations with error handling,
//! retry logic, and MCP-friendly response formats.

use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use gouqi::issues::AddComment;
use gouqi::r#async::Jira;
use gouqi::{Comment, Issue, SearchOptions, Session};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use tracing::{debug, error, info, instrument};

/// JIRA client wrapper that provides MCP-friendly operations
#[derive(Debug, Clone)]
pub struct JiraClient {
    pub(crate) client: Arc<Jira>,
    config: Arc<JiraConfig>,
}

/// Search result wrapper with pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub issues: Vec<IssueInfo>,
    pub total: usize,
    pub start_at: usize,
    pub max_results: usize,
    pub is_last: bool,
}

/// Simplified issue information for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueInfo {
    pub key: String,
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub issue_type: String,
    pub status: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub created: String,
    pub updated: String,
    pub project_key: String,
    pub project_name: String,
    pub labels: Vec<String>,
    pub components: Vec<String>,
}

/// Detailed issue information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueDetails {
    pub issue_info: IssueInfo,
    pub comments: Option<Vec<CommentInfo>>,
    pub attachments: Option<Vec<AttachmentInfo>>,
    pub history: Option<Vec<HistoryEntry>>,
    pub subtasks: Vec<IssueInfo>,
    pub parent: Option<IssueInfo>,
    pub linked_issues: Vec<LinkedIssue>,
}

/// Comment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentInfo {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created: String,
    pub updated: String,
}

/// Attachment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub id: String,
    pub filename: String,
    pub author: String,
    pub created: String,
    pub size: u64,
    pub mime_type: String,
}

/// History entry for issue changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub author: String,
    pub created: String,
    pub items: Vec<HistoryItem>,
}

/// Individual history item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub field: String,
    pub field_type: String,
    pub from: Option<String>,
    pub from_string: Option<String>,
    pub to: Option<String>,
    pub to_string: Option<String>,
}

/// Linked issue information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedIssue {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub link_type: String,
    pub direction: String, // "inward" or "outward"
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub account_id: String,
    pub display_name: String,
    pub email_address: Option<String>,
    pub active: bool,
}

impl JiraClient {
    /// Create a new JIRA client with the given configuration
    #[instrument(skip_all)]
    pub async fn new(config: Arc<JiraConfig>) -> JiraMcpResult<Self> {
        info!("Initializing JIRA client for URL: {}", config.jira_url);

        let credentials = config.to_gouqi_credentials();

        // Create the async gouqi client with timeout
        let client = timeout(Duration::from_secs(config.request_timeout_seconds), async {
            Jira::new(&config.jira_url, credentials)
        })
        .await
        .map_err(|_| JiraMcpError::network("Timeout connecting to JIRA instance"))?
        .map_err(JiraMcpError::from)?;

        let jira_client = Self {
            client: Arc::new(client),
            config,
        };

        // Test the connection
        jira_client.test_connection().await?;

        info!("JIRA client initialized successfully");
        Ok(jira_client)
    }

    /// Test the connection to the JIRA instance
    #[instrument(skip_all)]
    async fn test_connection(&self) -> JiraMcpResult<()> {
        debug!("Testing JIRA connection");

        // Try to get current user info to test authentication
        match self.get_current_user().await {
            Ok(user) => {
                info!(
                    "Connection test successful, authenticated as: {}",
                    user.display_name
                );
                Ok(())
            }
            Err(e) => {
                error!("Connection test failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get current user information
    #[instrument(skip_all)]
    pub async fn get_current_user(&self) -> JiraMcpResult<UserInfo> {
        debug!("Fetching current user information");

        let timeout_duration = Duration::from_secs(self.config.request_timeout_seconds);

        let session = timeout(timeout_duration, async {
            // Use session() method to get current user info
            self.client.session().await
        })
        .await
        .map_err(|_| JiraMcpError::network("Timeout getting current user"))?
        .map_err(JiraMcpError::from)?;

        Ok(self.convert_session_to_user_info(&session))
    }

    /// Search for issues using JQL
    #[instrument(skip(self))]
    pub async fn search_issues_jql(
        &self,
        jql: &str,
        start_at: Option<usize>,
        max_results: Option<usize>,
        expand: Option<Vec<String>>,
    ) -> JiraMcpResult<SearchResult> {
        let start = start_at.unwrap_or(0);
        let max = max_results.unwrap_or(self.config.max_search_results as usize);
        let max = max.min(200); // Cap at 200 per JIRA API limits

        debug!(
            "Searching issues with JQL: '{}', start: {}, max: {}",
            jql, start, max
        );

        let mut search_options = SearchOptions::builder()
            .start_at(start as u64)
            .max_results(max as u64)
            .build();

        // Add expand options if specified
        if let Some(expand_fields) = expand {
            search_options = SearchOptions::builder()
                .start_at(start as u64)
                .max_results(max as u64)
                .expand(expand_fields)
                .build();
        }

        let timeout_duration = Duration::from_secs(self.config.request_timeout_seconds);

        let search_result = timeout(timeout_duration, async {
            self.client.search().list(jql, &search_options).await
        })
        .await
        .map_err(|_| JiraMcpError::network("Timeout during search"))?
        .map_err(JiraMcpError::from)?;

        // Convert to our format
        let issues: Vec<IssueInfo> = search_result
            .issues
            .iter()
            .map(|issue| self.convert_issue_info(issue))
            .collect();

        let result = SearchResult {
            issues,
            total: search_result.total as usize,
            start_at: search_result.start_at as usize,
            max_results: search_result.max_results as usize,
            is_last: (start + max) >= (search_result.total as usize),
        };

        info!(
            "Found {} issues (showing {}-{} of {})",
            result.issues.len(),
            result.start_at,
            result.start_at + result.issues.len(),
            result.total
        );

        Ok(result)
    }

    /// Get detailed issue information
    #[instrument(skip(self))]
    pub async fn get_issue_details(
        &self,
        issue_key: &str,
        include_comments: bool,
        include_attachments: bool,
        include_history: bool,
    ) -> JiraMcpResult<IssueDetails> {
        debug!("Fetching issue details for: {}", issue_key);

        let timeout_duration = Duration::from_secs(self.config.request_timeout_seconds);

        // Build expand parameters
        let mut expand_fields = Vec::new();
        if include_comments {
            expand_fields.push("comment");
        }
        if include_history {
            expand_fields.push("changelog");
        }

        // Get issue with expand parameters
        let issue = if !expand_fields.is_empty() {
            let expand_param = expand_fields.join(",");
            let endpoint = format!("/issue/{}?expand={}", issue_key, expand_param);

            timeout(timeout_duration, async {
                self.client.get("api", &endpoint).await
            })
            .await
            .map_err(|_| JiraMcpError::network(format!("Timeout getting issue {}", issue_key)))?
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    JiraMcpError::not_found("issue", issue_key)
                } else {
                    JiraMcpError::from(e)
                }
            })?
        } else {
            timeout(timeout_duration, async {
                self.client.issues().get(issue_key).await
            })
            .await
            .map_err(|_| JiraMcpError::network(format!("Timeout getting issue {}", issue_key)))?
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    JiraMcpError::not_found("issue", issue_key)
                } else {
                    JiraMcpError::from(e)
                }
            })?
        };

        let issue_info = self.convert_issue_info(&issue);

        // Extract comments from issue if requested
        let comments = if include_comments {
            issue.comments().map(|comments_obj| {
                comments_obj
                    .comments
                    .iter()
                    .map(|c| self.convert_comment_info(c))
                    .collect()
            })
        } else {
            None
        };

        // Extract attachments from issue fields if requested
        let attachments = if include_attachments {
            issue
                .field::<Vec<gouqi::Attachment>>("attachment")
                .and_then(|result| result.ok())
                .map(|attachments_vec| {
                    attachments_vec
                        .iter()
                        .map(|a| self.convert_attachment_info(a))
                        .collect()
                })
        } else {
            None
        };

        // Extract changelog (history) from issue if requested
        let history = if include_history {
            issue
                .field::<gouqi::Changelog>("changelog")
                .and_then(|result| result.ok())
                .map(|changelog| {
                    changelog
                        .histories
                        .iter()
                        .map(|h| self.convert_history_entry(h))
                        .collect()
                })
        } else {
            None
        };

        // Extract linked issues
        let linked_issues = self.extract_linked_issues(&issue);

        Ok(IssueDetails {
            issue_info,
            comments,
            attachments,
            history,
            subtasks: Vec::new(), // TODO: Extract from issuelinks where link type is subtask
            parent: None,         // TODO: Extract from parent field
            linked_issues,
        })
    }

    /// Convert gouqi Issue to our IssueInfo format
    fn convert_issue_info(&self, issue: &Issue) -> IssueInfo {
        IssueInfo {
            key: issue.key.clone(),
            id: issue.id.clone(),
            summary: issue.summary().unwrap_or_default(),
            description: issue.description(),
            issue_type: issue
                .issue_type()
                .map(|it| it.name.clone())
                .unwrap_or_default(),
            status: issue.status().map(|s| s.name.clone()).unwrap_or_default(),
            priority: issue.priority().map(|p| p.name.clone()),
            assignee: issue.assignee().map(|u| u.display_name.clone()),
            reporter: issue.reporter().map(|u| u.display_name.clone()),
            created: issue.created().map(|dt| dt.to_string()).unwrap_or_default(),
            updated: issue.updated().map(|dt| dt.to_string()).unwrap_or_default(),
            project_key: issue.project().map(|p| p.key.clone()).unwrap_or_default(),
            project_name: issue.project().map(|p| p.name.clone()).unwrap_or_default(),
            labels: issue.labels(),
            components: Vec::new(), // Components would need to be implemented based on gouqi API
        }
    }

    /// Convert gouqi Session to our UserInfo format
    fn convert_session_to_user_info(&self, session: &Session) -> UserInfo {
        UserInfo {
            account_id: session.name.clone(),
            display_name: session.name.clone(),
            email_address: None,
            active: true,
        }
    }

    /// Convert gouqi Comment to our CommentInfo format
    fn convert_comment_info(&self, comment: &Comment) -> CommentInfo {
        CommentInfo {
            id: comment.id.clone().unwrap_or_else(|| {
                format!(
                    "comment-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                )
            }),
            author: comment
                .author
                .as_ref()
                .map(|u| u.display_name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            body: comment.body.clone(),
            created: comment
                .created
                .as_ref()
                .map(|dt| dt.to_string())
                .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string()),
            updated: comment
                .updated
                .as_ref()
                .map(|dt| dt.to_string())
                .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string()),
        }
    }

    /// Convert gouqi Attachment to our AttachmentInfo format
    fn convert_attachment_info(&self, attachment: &gouqi::Attachment) -> AttachmentInfo {
        AttachmentInfo {
            id: attachment.id.clone(),
            filename: attachment.filename.clone(),
            author: attachment.author.display_name.clone(),
            created: attachment.created.clone(),
            size: attachment.size,
            mime_type: attachment.mime_type.clone(),
        }
    }

    /// Convert gouqi History to our HistoryEntry format
    fn convert_history_entry(&self, history: &gouqi::History) -> HistoryEntry {
        // Generate a pseudo-ID from timestamp and author
        let id = format!(
            "history-{}-{}",
            history.created.replace([':', '.', '-'], ""),
            history.author.name.as_deref().unwrap_or("unknown")
        );

        HistoryEntry {
            id,
            author: history.author.display_name.clone(),
            created: history.created.clone(),
            items: history
                .items
                .iter()
                .map(|item| HistoryItem {
                    field: item.field.clone(),
                    field_type: "custom".to_string(), // gouqi doesn't provide field_type
                    from: item.from.clone(),
                    from_string: item.from_string.clone(),
                    to: item.to.clone(),
                    to_string: item.to_string.clone(),
                })
                .collect(),
        }
    }

    /// Extract linked issues from an Issue
    fn extract_linked_issues(&self, issue: &Issue) -> Vec<LinkedIssue> {
        issue
            .links()
            .and_then(|result| result.ok())
            .map(|links| {
                links
                    .iter()
                    .filter_map(|link| {
                        // Extract the linked issue information
                        if let Some(outward_issue) = &link.outward_issue {
                            Some(LinkedIssue {
                                key: outward_issue.key.clone(),
                                summary: outward_issue.summary().unwrap_or_default(),
                                status: outward_issue
                                    .status()
                                    .map(|s| s.name.clone())
                                    .unwrap_or_else(|| "Unknown".to_string()),
                                link_type: link.link_type.outward.clone(),
                                direction: "outward".to_string(),
                            })
                        } else {
                            link.inward_issue.as_ref().map(|inward_issue| LinkedIssue {
                                key: inward_issue.key.clone(),
                                summary: inward_issue.summary().unwrap_or_default(),
                                status: inward_issue
                                    .status()
                                    .map(|s| s.name.clone())
                                    .unwrap_or_else(|| "Unknown".to_string()),
                                link_type: link.link_type.inward.clone(),
                                direction: "inward".to_string(),
                            })
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get user information by username or account ID
    #[instrument(skip(self))]
    pub async fn get_user_by_identifier(&self, identifier: &str) -> JiraMcpResult<UserInfo> {
        debug!("Fetching user info for: {}", identifier);

        let _timeout_duration = Duration::from_secs(self.config.request_timeout_seconds);

        // For now, return a placeholder as gouqi may not have direct user lookup
        // This would need to be implemented based on available gouqi methods
        Ok(UserInfo {
            account_id: identifier.to_string(),
            display_name: identifier.to_string(),
            email_address: None,
            active: true,
        })
    }

    /// Add a comment to a JIRA issue
    #[instrument(skip(self))]
    pub async fn add_comment(
        &self,
        issue_key: &str,
        comment_body: &str,
    ) -> JiraMcpResult<CommentInfo> {
        info!("Adding comment to issue: {}", issue_key);

        let timeout_duration = Duration::from_secs(self.config.request_timeout_seconds);

        // Create the comment request
        let add_comment = AddComment {
            body: comment_body.to_string(),
        };

        // Call the real gouqi comment API
        let comment = timeout(timeout_duration, async {
            self.client.issues().comment(issue_key, add_comment).await
        })
        .await
        .map_err(|_| {
            JiraMcpError::network(format!("Timeout adding comment to issue {}", issue_key))
        })?
        .map_err(|e| {
            if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                JiraMcpError::not_found("issue", issue_key)
            } else if e.to_string().contains("403") || e.to_string().contains("Forbidden") {
                JiraMcpError::permission(format!(
                    "Permission denied adding comment to issue {}",
                    issue_key
                ))
            } else {
                JiraMcpError::from(e)
            }
        })?;

        info!("Successfully added comment to issue {}", issue_key);
        Ok(self.convert_comment_info(&comment))
    }

    /// Build a JQL query for user-assigned issues
    pub fn build_user_issues_jql(
        &self,
        account_id: &str,
        status_filter: Option<&[String]>,
        issue_types: Option<&[String]>,
    ) -> String {
        let mut jql_parts = vec![format!("assignee = \"{}\"", account_id)];

        if let Some(statuses) = status_filter {
            if !statuses.is_empty() {
                let status_list = statuses
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql_parts.push(format!("status IN ({})", status_list));
            }
        }

        if let Some(types) = issue_types {
            if !types.is_empty() {
                let type_list = types
                    .iter()
                    .map(|t| format!("\"{}\"", t))
                    .collect::<Vec<_>>()
                    .join(", ");
                jql_parts.push(format!("issuetype IN ({})", type_list));
            }
        }

        // Order by updated date descending
        jql_parts.push("ORDER BY updated DESC".to_string());

        jql_parts.join(" AND ")
    }
}

#[cfg(test)]
mod tests {

    // Note: Tests are commented out due to unsafe mock usage
    // Proper mocking would require a trait-based approach or dependency injection
    //
    // #[test]
    // fn test_user_issues_jql_building() {
    //     // Would test JQL building logic with a proper mock
    // }
}
