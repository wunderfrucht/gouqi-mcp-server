//! JIRA MCP Server Library
//!
//! An AI-friendly JIRA integration server using the Model Context Protocol (MCP).
//! This server provides semantic tools for searching, retrieving, commenting on, and analyzing
//! relationships between JIRA issues without requiring knowledge of JQL or JIRA internals.
//!
//! ## Features
//!
//! - **AI-Friendly Interface**: Uses semantic parameters instead of JQL
//! - **Real JIRA API Integration**: Leverages gouqi 0.14.0 for Cloud/Server operations
//! - **Smart Caching**: Metadata caching with TTL for performance
//! - **Comprehensive Tools**: Search, issue details, user issues, commenting, relationship analysis
//! - **Issue Interaction**: Add comments and analyze issue relationship graphs
//! - **Error Handling**: MCP-compliant error codes and messages

use crate::cache::{MetadataCache, UserMapping};
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use crate::tools::{
    AddCommentParams, AddCommentResult, AddCommentTool, AddTodoParams, AddTodoResult,
    AssignIssueParams, AssignIssueResult, AssignIssueTool, CancelTodoWorkParams,
    CancelTodoWorkResult, CheckpointTodoWorkParams, CheckpointTodoWorkResult,
    CompleteTodoWorkParams, CompleteTodoWorkResult, CreateIssueParams, CreateIssueResult,
    CreateIssueTool, DownloadAttachmentParams, DownloadAttachmentResult, DownloadAttachmentTool,
    GetActiveWorkSessionsResult, GetAvailableTransitionsParams, GetAvailableTransitionsResult,
    GetAvailableTransitionsTool, GetCreateMetadataParams, GetCreateMetadataResult,
    GetCreateMetadataTool, GetCustomFieldsParams, GetCustomFieldsResult, GetCustomFieldsTool,
    GetIssueDetailsParams, GetIssueDetailsResult, GetIssueDetailsTool, GetUserIssuesParams,
    GetUserIssuesResult, GetUserIssuesTool, IssueRelationshipsParams, IssueRelationshipsResult,
    IssueRelationshipsTool, ListAttachmentsParams, ListAttachmentsResult, ListAttachmentsTool,
    ListTodosParams, ListTodosResult, PauseTodoWorkParams, PauseTodoWorkResult, SearchIssuesParams,
    SearchIssuesResult, SearchIssuesTool, SetTodoBaseParams, SetTodoBaseResult,
    StartTodoWorkParams, StartTodoWorkResult, TodoTracker, TransitionIssueParams,
    TransitionIssueResult, TransitionIssueTool, UpdateCustomFieldsParams, UpdateCustomFieldsResult,
    UpdateCustomFieldsTool, UpdateDescription, UpdateDescriptionParams, UpdateDescriptionResult,
    UpdateTodoParams, UpdateTodoResult,
};

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, instrument, warn};

// Re-export modules for external use
pub mod cache;
pub mod config;
pub mod error;
pub mod jira_client;
pub mod semantic_mapping;
pub mod tools;

/// Server status information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JiraServerStatus {
    pub server_name: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub jira_url: String,
    pub jira_connection_status: String,
    pub authenticated_user: Option<String>,
    pub cache_stats: cache::CacheStats,
    pub tools_count: usize,
}

/// JIRA MCP Server
///
/// Main server implementation that provides AI-friendly tools for JIRA interaction.
/// Uses the #[mcp_server] macro for automatic MCP infrastructure generation.
#[mcp_server(
    name = "JIRA MCP Server",
    version = "0.7.0",
    description = "AI-friendly JIRA integration server with semantic search, commenting, and relationship analysis capabilities",
    auth = "disabled" // Start with disabled for development, can be changed to "file" for production
)]
#[derive(Clone)]
pub struct JiraMcpServer {
    /// Server start time for uptime calculation
    start_time: Instant,

    /// JIRA client for API operations
    jira_client: Arc<JiraClient>,

    /// Configuration
    config: Arc<JiraConfig>,

    /// Metadata cache
    cache: Arc<MetadataCache>,

    /// Tool implementations
    search_tool: Arc<SearchIssuesTool>,
    issue_details_tool: Arc<GetIssueDetailsTool>,
    user_issues_tool: Arc<GetUserIssuesTool>,
    list_attachments_tool: Arc<ListAttachmentsTool>,
    download_attachment_tool: Arc<DownloadAttachmentTool>,
    add_comment_tool: Arc<AddCommentTool>,
    issue_relationships_tool: Arc<IssueRelationshipsTool>,
    update_description_tool: Arc<UpdateDescription>,
    get_available_transitions_tool: Arc<GetAvailableTransitionsTool>,
    transition_issue_tool: Arc<TransitionIssueTool>,
    assign_issue_tool: Arc<AssignIssueTool>,
    get_custom_fields_tool: Arc<GetCustomFieldsTool>,
    update_custom_fields_tool: Arc<UpdateCustomFieldsTool>,
    create_issue_tool: Arc<CreateIssueTool>,
    get_create_metadata_tool: Arc<GetCreateMetadataTool>,
    todo_tracker: Arc<TodoTracker>,
}

impl Default for JiraMcpServer {
    fn default() -> Self {
        // This is a placeholder default implementation
        // In practice, the server should be created using `new()` or `with_config()`
        panic!("JiraMcpServer cannot be created with default(). Use JiraMcpServer::new() instead.")
    }
}

impl JiraMcpServer {
    /// Create a new JIRA MCP Server with default configuration
    #[instrument]
    pub async fn new() -> JiraMcpResult<Self> {
        info!("Initializing JIRA MCP Server");

        // Load configuration
        let config = Arc::new(JiraConfig::load()?);
        info!("Configuration loaded successfully");

        // Create cache
        let cache = Arc::new(MetadataCache::new(config.cache_ttl_seconds));

        // Start cache cleanup task
        let _cleanup_handle = Arc::clone(&cache).start_cleanup_task();

        // Create JIRA client
        let jira_client = Arc::new(JiraClient::new(Arc::clone(&config)).await?);
        info!("JIRA client initialized");

        // Initialize current user in cache
        if let Ok(current_user) = jira_client.get_current_user().await {
            let user_mapping = UserMapping {
                account_id: current_user.account_id,
                display_name: current_user.display_name,
                email_address: current_user.email_address,
                username: None, // Will be filled if available
            };

            if let Err(e) = cache.set_current_user(user_mapping) {
                warn!("Failed to cache current user: {}", e);
            } else {
                info!("Current user cached successfully");
            }
        } else {
            warn!("Could not retrieve current user information");
        }

        // Create tool implementations
        let search_tool = Arc::new(SearchIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let issue_details_tool = Arc::new(GetIssueDetailsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let user_issues_tool = Arc::new(GetUserIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let list_attachments_tool = Arc::new(ListAttachmentsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let download_attachment_tool = Arc::new(DownloadAttachmentTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let add_comment_tool = Arc::new(AddCommentTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let issue_relationships_tool = Arc::new(IssueRelationshipsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let update_description_tool = Arc::new(UpdateDescription::new(Arc::clone(&jira_client)));

        let get_available_transitions_tool =
            Arc::new(GetAvailableTransitionsTool::new(Arc::clone(&jira_client)));

        let transition_issue_tool = Arc::new(TransitionIssueTool::new(Arc::clone(&jira_client)));

        let assign_issue_tool = Arc::new(AssignIssueTool::new(Arc::clone(&jira_client)));

        let get_custom_fields_tool = Arc::new(GetCustomFieldsTool::new(Arc::clone(&jira_client)));

        let update_custom_fields_tool =
            Arc::new(UpdateCustomFieldsTool::new(Arc::clone(&jira_client)));

        let create_issue_tool = Arc::new(CreateIssueTool::new(Arc::clone(&jira_client)));

        let get_create_metadata_tool =
            Arc::new(GetCreateMetadataTool::new(Arc::clone(&jira_client)));

        let todo_tracker = Arc::new(TodoTracker::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        // Start auto-checkpoint background task (every 30 minutes)
        let _auto_checkpoint_handle = Arc::clone(&todo_tracker).start_auto_checkpoint_task(30);
        info!("Auto-checkpoint task started (interval: 30 minutes)");

        info!("JIRA MCP Server initialized successfully");

        Ok(Self {
            start_time: Instant::now(),
            jira_client,
            config,
            cache,
            search_tool,
            issue_details_tool,
            user_issues_tool,
            list_attachments_tool,
            download_attachment_tool,
            add_comment_tool,
            issue_relationships_tool,
            update_description_tool,
            get_available_transitions_tool,
            transition_issue_tool,
            assign_issue_tool,
            get_custom_fields_tool,
            update_custom_fields_tool,
            create_issue_tool,
            get_create_metadata_tool,
            todo_tracker,
        })
    }

    /// Create server with custom configuration (for testing)
    #[instrument(skip(config))]
    pub async fn with_config(config: JiraConfig) -> JiraMcpResult<Self> {
        let config = Arc::new(config);
        let cache = Arc::new(MetadataCache::new(config.cache_ttl_seconds));
        let _cleanup_handle = Arc::clone(&cache).start_cleanup_task();

        let jira_client = Arc::new(JiraClient::new(Arc::clone(&config)).await?);

        // Try to initialize current user
        if let Ok(current_user) = jira_client.get_current_user().await {
            let user_mapping = UserMapping {
                account_id: current_user.account_id,
                display_name: current_user.display_name,
                email_address: current_user.email_address,
                username: None,
            };
            let _ = cache.set_current_user(user_mapping);
        }

        let search_tool = Arc::new(SearchIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let issue_details_tool = Arc::new(GetIssueDetailsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let user_issues_tool = Arc::new(GetUserIssuesTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let list_attachments_tool = Arc::new(ListAttachmentsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let download_attachment_tool = Arc::new(DownloadAttachmentTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let add_comment_tool = Arc::new(AddCommentTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let issue_relationships_tool = Arc::new(IssueRelationshipsTool::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        let update_description_tool = Arc::new(UpdateDescription::new(Arc::clone(&jira_client)));

        let get_available_transitions_tool =
            Arc::new(GetAvailableTransitionsTool::new(Arc::clone(&jira_client)));

        let transition_issue_tool = Arc::new(TransitionIssueTool::new(Arc::clone(&jira_client)));

        let assign_issue_tool = Arc::new(AssignIssueTool::new(Arc::clone(&jira_client)));

        let get_custom_fields_tool = Arc::new(GetCustomFieldsTool::new(Arc::clone(&jira_client)));

        let update_custom_fields_tool =
            Arc::new(UpdateCustomFieldsTool::new(Arc::clone(&jira_client)));

        let create_issue_tool = Arc::new(CreateIssueTool::new(Arc::clone(&jira_client)));

        let get_create_metadata_tool =
            Arc::new(GetCreateMetadataTool::new(Arc::clone(&jira_client)));

        let todo_tracker = Arc::new(TodoTracker::new(
            Arc::clone(&jira_client),
            Arc::clone(&config),
            Arc::clone(&cache),
        ));

        Ok(Self {
            start_time: Instant::now(),
            jira_client,
            config,
            cache,
            search_tool,
            issue_details_tool,
            user_issues_tool,
            list_attachments_tool,
            download_attachment_tool,
            add_comment_tool,
            issue_relationships_tool,
            update_description_tool,
            get_available_transitions_tool,
            transition_issue_tool,
            assign_issue_tool,
            get_custom_fields_tool,
            update_custom_fields_tool,
            create_issue_tool,
            get_create_metadata_tool,
            todo_tracker,
        })
    }

    /// Get server uptime in seconds
    fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get current user display name (for status)
    async fn get_current_user_name(&self) -> String {
        if let Some(user) = self.cache.get_current_user() {
            user.display_name
        } else if let Ok(user) = self.jira_client.get_current_user().await {
            user.display_name
        } else {
            "Unknown".to_string()
        }
    }
}

/// All public methods in this impl block become MCP tools automatically
/// The #[mcp_tools] macro discovers these methods and exposes them via MCP
#[mcp_tools]
impl JiraMcpServer {
    /// Search for JIRA issues using AI-friendly semantic parameters
    ///
    /// This tool allows AI agents to search for issues without needing to know JQL syntax.
    /// It accepts natural language parameters and translates them to appropriate JIRA queries.
    ///
    /// # Examples
    /// - Find all stories assigned to me: `{"issue_types": ["story"], "assigned_to": "me"}`
    /// - Find bugs in project FOO: `{"issue_types": ["bug"], "project_key": "FOO"}`
    /// - Find overdue issues: `{"status": ["open"], "created_after": "30 days ago"}`
    #[instrument(skip(self))]
    pub async fn search_issues(
        &self,
        params: SearchIssuesParams,
    ) -> anyhow::Result<SearchIssuesResult> {
        self.search_tool.execute(params).await.map_err(|e| {
            error!("search_issues failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Get detailed information about a specific JIRA issue
    ///
    /// Retrieves comprehensive information about an issue including summary, description,
    /// status, assignee, and optionally comments, attachments, and history.
    ///
    /// # Examples
    /// - Get basic issue info: `{"issue_key": "PROJ-123"}`
    /// - Get issue with comments: `{"issue_key": "PROJ-123", "include_comments": true}`
    /// - Get full issue details: `{"issue_key": "PROJ-123", "include_comments": true, "include_attachments": true, "include_history": true}`
    #[instrument(skip(self))]
    pub async fn get_issue_details(
        &self,
        params: GetIssueDetailsParams,
    ) -> anyhow::Result<GetIssueDetailsResult> {
        self.issue_details_tool.execute(params).await.map_err(|e| {
            error!("get_issue_details failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Get issues assigned to a specific user with filtering options
    ///
    /// Retrieves issues assigned to a user (defaults to current user) with various
    /// semantic filtering options for status, type, project, priority, and dates.
    ///
    /// # Examples
    /// - Get my open issues: `{"status_filter": ["open", "in_progress"]}`
    /// - Get user's bugs: `{"username": "john.doe", "issue_types": ["bug"]}`
    /// - Get overdue issues: `{"due_date_filter": "overdue", "priority_filter": ["high"]}`
    #[instrument(skip(self))]
    pub async fn get_user_issues(
        &self,
        params: GetUserIssuesParams,
    ) -> anyhow::Result<GetUserIssuesResult> {
        self.user_issues_tool.execute(params).await.map_err(|e| {
            error!("get_user_issues failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Get server status and connection information
    ///
    /// Returns comprehensive information about the server status, JIRA connection,
    /// authenticated user, cache statistics, and available tools.
    #[instrument(skip(self))]
    pub async fn get_server_status(&self) -> anyhow::Result<JiraServerStatus> {
        info!("Getting server status");

        let connection_status = match self.jira_client.get_current_user().await {
            Ok(_) => "Connected".to_string(),
            Err(e) => format!("Connection Error: {}", e),
        };

        let authenticated_user = if connection_status == "Connected" {
            Some(self.get_current_user_name().await)
        } else {
            None
        };

        Ok(JiraServerStatus {
            server_name: "JIRA MCP Server".to_string(),
            version: "0.7.0".to_string(),
            uptime_seconds: self.get_uptime_seconds(),
            jira_url: self.config.jira_url.clone(),
            jira_connection_status: connection_status,
            authenticated_user,
            cache_stats: self.cache.get_stats(),
            tools_count: 28, // search_issues, get_issue_details, get_user_issues, list_issue_attachments, download_attachment, get_server_status, clear_cache, test_connection, add_comment, update_issue_description, get_issue_relationships, get_available_transitions, transition_issue, assign_issue, get_custom_fields, update_custom_fields, create_issue, get_create_metadata, list_todos, add_todo, update_todo, start_todo_work, complete_todo_work, checkpoint_todo_work, pause_todo_work, cancel_todo_work, get_active_work_sessions, set_todo_base
        })
    }

    /// Clear all cached metadata
    ///
    /// Clears all cached metadata including board mappings, project info, user info,
    /// and issue types. Useful when JIRA configuration changes or for troubleshooting.
    #[instrument(skip(self))]
    pub async fn clear_cache(&self) -> anyhow::Result<String> {
        info!("Clearing all cached metadata");

        match self.cache.clear_all() {
            Ok(()) => {
                info!("Cache cleared successfully");
                Ok("All cached metadata has been cleared successfully".to_string())
            }
            Err(e) => {
                error!("Failed to clear cache: {}", e);
                Err(anyhow::anyhow!("Failed to clear cache: {}", e))
            }
        }
    }

    /// List all attachments for a specific JIRA issue
    ///
    /// Returns metadata about all attachments on an issue, including filenames,
    /// sizes, content types, and attachment IDs needed for downloading.
    ///
    /// # Examples
    /// - List all attachments: `{"issue_key": "PROJ-123"}`
    #[instrument(skip(self))]
    pub async fn list_issue_attachments(
        &self,
        params: ListAttachmentsParams,
    ) -> anyhow::Result<ListAttachmentsResult> {
        self.list_attachments_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("list_issue_attachments failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Download attachment content from a JIRA issue
    ///
    /// Downloads the actual content of an attachment given its attachment ID.
    /// Content is returned as base64 encoded string by default for safety.
    ///
    /// # Examples
    /// - Download attachment: `{"attachment_id": "12345"}`
    /// - Download with size limit: `{"attachment_id": "12345", "max_size_bytes": 5242880}`
    /// - Download as raw content: `{"attachment_id": "12345", "base64_encoded": false}`
    #[instrument(skip(self))]
    pub async fn download_attachment(
        &self,
        params: DownloadAttachmentParams,
    ) -> anyhow::Result<DownloadAttachmentResult> {
        self.download_attachment_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("download_attachment failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Test JIRA connection and authentication
    ///
    /// Performs a connection test to the configured JIRA instance and returns
    /// detailed information about the connection status and authenticated user.
    #[instrument(skip(self))]
    pub async fn test_connection(&self) -> anyhow::Result<String> {
        info!("Testing JIRA connection");

        match self.jira_client.get_current_user().await {
            Ok(user) => {
                let message = format!(
                    "✅ Connection successful!\n\
                     JIRA URL: {}\n\
                     Authenticated as: {} ({})\n\
                     Account ID: {}\n\
                     Email: {}",
                    self.config.jira_url,
                    user.display_name,
                    user.email_address.as_deref().unwrap_or("N/A"),
                    user.account_id,
                    user.email_address.as_deref().unwrap_or("Not provided")
                );
                info!("Connection test successful for user: {}", user.display_name);
                Ok(message)
            }
            Err(e) => {
                let message = format!(
                    "❌ Connection failed!\n\
                     JIRA URL: {}\n\
                     Error: {}\n\
                     \n\
                     Please check:\n\
                     - JIRA URL is correct and accessible\n\
                     - Authentication credentials are valid\n\
                     - Network connectivity to JIRA instance",
                    self.config.jira_url, e
                );
                error!("Connection test failed: {}", e);
                Ok(message) // Return as success with error message for user feedback
            }
        }
    }

    /// Add a comment to a JIRA issue
    ///
    /// Adds a comment to the specified JIRA issue with the provided text content.
    /// This tool provides a simple way to add comments without requiring knowledge
    /// of JIRA's comment API structure.
    ///
    /// # Examples
    /// - Add a simple comment: `{"issue_key": "PROJ-123", "comment_body": "This looks good to me!"}`
    /// - Add a detailed comment: `{"issue_key": "PROJ-123", "comment_body": "I've tested this feature and found the following:\n\n1. Works as expected\n2. Performance is good\n3. Ready for deployment"}`
    #[instrument(skip(self))]
    pub async fn add_comment(&self, params: AddCommentParams) -> anyhow::Result<AddCommentResult> {
        self.add_comment_tool.execute(params).await.map_err(|e| {
            error!("add_comment failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Update the description of a JIRA issue
    ///
    /// Updates the description field of a JIRA issue. Supports three modes:
    /// - append (default): Adds content to the end of the existing description
    /// - prepend: Adds content to the beginning of the existing description
    /// - replace: Completely replaces the description with new content
    ///
    /// # Examples
    /// - Append to description: `{"issue_key": "PROJ-123", "content": "Additional context: This fixes the login issue"}`
    /// - Replace description: `{"issue_key": "PROJ-123", "content": "New complete description", "mode": "replace"}`
    /// - Prepend to description: `{"issue_key": "PROJ-123", "content": "⚠️ URGENT: ", "mode": "prepend"}`
    #[instrument(skip(self))]
    pub async fn update_issue_description(
        &self,
        params: UpdateDescriptionParams,
    ) -> anyhow::Result<UpdateDescriptionResult> {
        self.update_description_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("update_issue_description failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Extract issue relationship graph
    ///
    /// Analyzes JIRA issue relationships to build a comprehensive relationship graph
    /// showing how issues are connected through links, subtasks, epics, and other relationships.
    /// This tool helps understand issue dependencies, blockers, and project structure.
    ///
    /// # Examples
    /// - Basic relationship extraction: `{"root_issue_key": "PROJ-123"}`
    /// - Deep relationship analysis: `{"root_issue_key": "PROJ-123", "max_depth": 3}`
    /// - Custom relationship filters: `{"root_issue_key": "PROJ-123", "include_duplicates": true, "include_epic_links": false}`
    #[instrument(skip(self))]
    pub async fn get_issue_relationships(
        &self,
        params: IssueRelationshipsParams,
    ) -> anyhow::Result<IssueRelationshipsResult> {
        self.issue_relationships_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("get_issue_relationships failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Get available transitions for an issue
    ///
    /// Returns the list of workflow transitions available for a specific JIRA issue.
    /// Different issues may have different available transitions depending on their
    /// current status, workflow, and issue type.
    ///
    /// # Examples
    /// - Get available transitions: `{"issue_key": "PROJ-123"}`
    #[instrument(skip(self))]
    pub async fn get_available_transitions(
        &self,
        params: GetAvailableTransitionsParams,
    ) -> anyhow::Result<GetAvailableTransitionsResult> {
        self.get_available_transitions_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("get_available_transitions failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Transition an issue to a new status
    ///
    /// Executes a workflow transition on a JIRA issue to change its status.
    /// You can specify the transition either by ID or by name. Optionally add
    /// a comment and/or set a resolution when transitioning.
    ///
    /// # Examples
    /// - Transition by name: `{"issue_key": "PROJ-123", "transition_name": "Start Progress"}`
    /// - Transition by ID: `{"issue_key": "PROJ-123", "transition_id": "11"}`
    /// - Transition with comment: `{"issue_key": "PROJ-123", "transition_name": "Done", "comment": "Work completed"}`
    /// - Transition with resolution: `{"issue_key": "PROJ-123", "transition_name": "Done", "resolution": "Fixed"}`
    #[instrument(skip(self))]
    pub async fn transition_issue(
        &self,
        params: TransitionIssueParams,
    ) -> anyhow::Result<TransitionIssueResult> {
        self.transition_issue_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("transition_issue failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Assign a JIRA issue to a user
    ///
    /// Assigns an issue to a specific user or unassigns it. You can use:
    /// - "me" or "self" to assign to yourself
    /// - A specific username or account ID
    /// - null/empty to unassign the issue
    ///
    /// This is particularly useful for:
    /// - Automated testing (assign issues to yourself)
    /// - Workflow automation (assign based on conditions)
    /// - Task distribution (assign to team members)
    ///
    /// # Examples
    /// - Assign to yourself: `{"issue_key": "PROJ-123", "assignee": "me"}`
    /// - Assign to user: `{"issue_key": "PROJ-123", "assignee": "john.doe@example.com"}`
    /// - Unassign: `{"issue_key": "PROJ-123", "assignee": null}`
    #[instrument(skip(self))]
    pub async fn assign_issue(
        &self,
        params: AssignIssueParams,
    ) -> anyhow::Result<AssignIssueResult> {
        self.assign_issue_tool.execute(params).await.map_err(|e| {
            error!("assign_issue failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Get custom fields from a JIRA issue
    ///
    /// Discovers and returns all custom fields present in a JIRA issue, including
    /// their field IDs, types, current values, and human-readable displays.
    /// Also attempts to detect common fields like story points and acceptance criteria.
    ///
    /// This is useful for:
    /// - Understanding what custom fields are available in your JIRA instance
    /// - Finding the correct field ID for updating custom fields
    /// - Inspecting current custom field values
    ///
    /// # Examples
    /// - Get all custom fields: `{"issue_key": "PROJ-123"}`
    #[instrument(skip(self))]
    pub async fn get_custom_fields(
        &self,
        params: GetCustomFieldsParams,
    ) -> anyhow::Result<GetCustomFieldsResult> {
        self.get_custom_fields_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("get_custom_fields failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Update custom fields in a JIRA issue
    ///
    /// Updates custom field values in a JIRA issue. Supports updating by field ID
    /// or using convenience parameters for common fields like story points and
    /// acceptance criteria (with automatic field detection).
    ///
    /// This tool allows you to:
    /// - Update story points using auto-detection or explicit field ID
    /// - Update acceptance criteria using auto-detection or explicit field ID
    /// - Update any custom field by providing its field ID and value
    ///
    /// # Examples
    /// - Set story points: `{"issue_key": "PROJ-123", "story_points": 5}`
    /// - Set acceptance criteria: `{"issue_key": "PROJ-123", "acceptance_criteria": "User can login successfully"}`
    /// - Update specific field: `{"issue_key": "PROJ-123", "custom_field_updates": {"customfield_10050": "value"}}`
    /// - Override field ID: `{"issue_key": "PROJ-123", "story_points": 8, "story_points_field_id": "customfield_10016"}`
    #[instrument(skip(self))]
    pub async fn update_custom_fields(
        &self,
        params: UpdateCustomFieldsParams,
    ) -> anyhow::Result<UpdateCustomFieldsResult> {
        self.update_custom_fields_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("update_custom_fields failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Get issue creation metadata for a JIRA project
    ///
    /// Discovers what issue types are available in a project and what fields are
    /// required or optional for each type. This is essential for understanding what
    /// parameters to provide when creating issues, especially for custom fields.
    ///
    /// Use this tool to:
    /// - Discover available issue types (Task, Bug, Story, Epic, etc.)
    /// - Find required fields for a specific issue type
    /// - Get allowed values for constrained fields (priorities, components, etc.)
    /// - Identify custom field IDs and their types
    ///
    /// # Examples
    /// - Get all issue types: `{"project_key": "PROJ"}`
    /// - Get Bug metadata only: `{"project_key": "PROJ", "issue_type": "Bug"}`
    /// - Get detailed schemas: `{"project_key": "PROJ", "issue_type": "Story", "include_schemas": true}`
    #[instrument(skip(self))]
    pub async fn get_create_metadata(
        &self,
        params: GetCreateMetadataParams,
    ) -> anyhow::Result<GetCreateMetadataResult> {
        self.get_create_metadata_tool
            .execute(params)
            .await
            .map_err(|e| {
                error!("get_create_metadata failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Create a new JIRA issue
    ///
    /// Creates a new issue in JIRA with comprehensive parameter support.
    /// Designed to be simple for basic use cases while supporting advanced features.
    ///
    /// Key features:
    /// - Simple: Just provide summary and project_key for basic tasks
    /// - Smart defaults: Auto-detects subtasks, handles "assign_to_me", etc.
    /// - initial_todos: Automatically formats todo checklists
    /// - Custom fields: Full support for any custom field
    /// - Epic/Story points: Convenience parameters with auto-detection
    ///
    /// IMPORTANT: Use get_create_metadata first to discover:
    /// - Available issue types for your project
    /// - Required fields for each issue type
    /// - Allowed values for constrained fields
    /// - Custom field IDs and types
    ///
    /// # Examples
    /// - Simple task: `{"project_key": "PROJ", "summary": "Fix login bug"}`
    /// - Bug with priority: `{"project_key": "PROJ", "summary": "Payment fails", "issue_type": "Bug", "priority": "High"}`
    /// - Story with todos: `{"project_key": "PROJ", "summary": "Dark mode", "issue_type": "Story", "initial_todos": ["Design colors", "Implement toggle"], "assign_to_me": true}`
    /// - Subtask: `{"parent_issue_key": "PROJ-123", "summary": "Write tests"}`
    #[instrument(skip(self))]
    pub async fn create_issue(
        &self,
        params: CreateIssueParams,
    ) -> anyhow::Result<CreateIssueResult> {
        self.create_issue_tool.execute(params).await.map_err(|e| {
            error!("create_issue failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// List todos from an issue description
    ///
    /// Parses markdown-style checkboxes from an issue description and returns
    /// them as structured todo items. Supports formats like `- [ ] todo` and `- [x] completed`.
    /// Allows filtering by status: open, completed, or wip (work in progress).
    ///
    /// # Examples
    /// - List all todos: `{"issue_key": "PROJ-123"}`
    /// - List open todos: `{"status_filter": ["open"]}`
    /// - List work in progress: `{"status_filter": ["wip"]}`
    /// - List open and wip: `{"status_filter": ["open", "wip"]}`
    #[instrument(skip(self))]
    pub async fn list_todos(&self, params: ListTodosParams) -> anyhow::Result<ListTodosResult> {
        self.todo_tracker.list_todos(params).await.map_err(|e| {
            error!("list_todos failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Add a new todo to an issue description
    ///
    /// Adds a new markdown-style checkbox todo to an issue's description.
    /// Automatically creates a "Todos" section if one doesn't exist, or adds
    /// to an existing todo section.
    ///
    /// # Examples
    /// - Add todo at end: `{"issue_key": "PROJ-123", "todo_text": "Review code changes"}`
    /// - Add todo at beginning: `{"issue_key": "PROJ-123", "todo_text": "Urgent: Fix bug", "prepend": true}`
    #[instrument(skip(self))]
    pub async fn add_todo(&self, params: AddTodoParams) -> anyhow::Result<AddTodoResult> {
        self.todo_tracker.add_todo(params).await.map_err(|e| {
            error!("add_todo failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Update a todo's completion status
    ///
    /// Marks a todo as completed (checked) or incomplete (unchecked) in the issue description.
    /// You can specify the todo by its ID or by its 1-based index in the list.
    ///
    /// # Examples
    /// - Complete a todo: `{"issue_key": "PROJ-123", "todo_id_or_index": "1", "completed": true}`
    /// - Reopen a todo: `{"issue_key": "PROJ-123", "todo_id_or_index": "todo-abc123", "completed": false}`
    #[instrument(skip(self))]
    pub async fn update_todo(&self, params: UpdateTodoParams) -> anyhow::Result<UpdateTodoResult> {
        self.todo_tracker.update_todo(params).await.map_err(|e| {
            error!("update_todo failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Start tracking work time on a todo
    ///
    /// Begins tracking time spent working on a specific todo. Creates a work session
    /// that will be used to calculate time when you complete the work. You must
    /// complete the work session before starting another one on the same todo.
    ///
    /// # Examples
    /// - Start work on first todo: `{"issue_key": "PROJ-123", "todo_id_or_index": "1"}`
    /// - Start work by todo ID: `{"issue_key": "PROJ-123", "todo_id_or_index": "todo-abc123"}`
    #[instrument(skip(self))]
    pub async fn start_todo_work(
        &self,
        params: StartTodoWorkParams,
    ) -> anyhow::Result<StartTodoWorkResult> {
        self.todo_tracker
            .start_todo_work(params)
            .await
            .map_err(|e| {
                error!("start_todo_work failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Complete work on a todo and log time spent
    ///
    /// Completes a work session for a todo, calculates the time spent, and logs it
    /// as a worklog entry in JIRA. Optionally marks the todo as completed.
    ///
    /// IMPORTANT: For sessions spanning multiple days (>24 hours), you MUST provide
    /// explicit time using time_spent_hours, time_spent_minutes, or time_spent_seconds.
    /// This prevents accidentally logging extremely long sessions.
    ///
    /// # Examples
    /// - Complete same-day work: `{"todo_id_or_index": "1"}`
    /// - Multi-day work: `{"todo_id_or_index": "1", "time_spent_hours": 8.5}`
    /// - With minutes: `{"todo_id_or_index": "1", "time_spent_minutes": 480}`
    /// - Without marking done: `{"todo_id_or_index": "1", "time_spent_hours": 6, "mark_completed": false}`
    /// - With comment: `{"todo_id_or_index": "1", "time_spent_hours": 7, "worklog_comment": "Completed feature implementation"}`
    #[instrument(skip(self))]
    pub async fn complete_todo_work(
        &self,
        params: CompleteTodoWorkParams,
    ) -> anyhow::Result<CompleteTodoWorkResult> {
        self.todo_tracker
            .complete_todo_work(params)
            .await
            .map_err(|e| {
                error!("complete_todo_work failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Checkpoint work progress - log time but keep session active
    ///
    /// Creates a checkpoint by logging the time accumulated since the session started
    /// (or since the last checkpoint), then resets the timer to continue tracking.
    /// Perfect for logging progress during long work sessions without stopping the timer.
    ///
    /// Benefits:
    /// - Avoid multi-day session issues by checkpointing before midnight
    /// - Create incremental progress records in JIRA
    /// - Maintain accurate time tracking for long sessions
    /// - Survive server restarts with logged time
    ///
    /// # Examples
    /// - Regular checkpoint: `{"todo_id_or_index": "1"}`
    /// - With comment: `{"todo_id_or_index": "1", "worklog_comment": "Completed initial implementation"}`
    /// - End of day checkpoint: `{"todo_id_or_index": "1", "worklog_comment": "End of day checkpoint, will continue tomorrow"}`
    #[instrument(skip(self))]
    pub async fn checkpoint_todo_work(
        &self,
        params: CheckpointTodoWorkParams,
    ) -> anyhow::Result<CheckpointTodoWorkResult> {
        self.todo_tracker
            .checkpoint_todo_work(params)
            .await
            .map_err(|e| {
                error!("checkpoint_todo_work failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Set the base issue for todo operations
    ///
    /// Sets a default JIRA issue to use for all subsequent todo commands.
    /// After setting a base issue, you can omit the issue_key parameter in
    /// list_todos, add_todo, update_todo, start_todo_work, and complete_todo_work.
    ///
    /// # Examples
    /// - Set base issue: `{"issue_key": "PROJ-123"}`
    ///
    /// Then you can use:
    /// - `list_todos({})` instead of `list_todos({"issue_key": "PROJ-123"})`
    /// - `add_todo({"todo_text": "New task"})` instead of providing issue_key
    #[instrument(skip(self))]
    pub async fn set_todo_base(
        &self,
        params: SetTodoBaseParams,
    ) -> anyhow::Result<SetTodoBaseResult> {
        self.todo_tracker.set_todo_base(params).await.map_err(|e| {
            error!("set_todo_base failed: {}", e);
            anyhow::anyhow!(e)
        })
    }

    /// Pause work on a todo and save progress
    ///
    /// Stops the active work session, calculates time spent, and logs it to JIRA.
    /// Unlike complete_todo_work, this doesn't mark the todo as completed - perfect
    /// for end-of-day saves or when you need to switch tasks temporarily.
    ///
    /// # Examples
    /// - Pause at end of day: `{"todo_id_or_index": "1", "worklog_comment": "End of day, will continue tomorrow"}`
    /// - Quick pause: `{"todo_id_or_index": "1"}`
    #[instrument(skip(self))]
    pub async fn pause_todo_work(
        &self,
        params: PauseTodoWorkParams,
    ) -> anyhow::Result<PauseTodoWorkResult> {
        self.todo_tracker
            .pause_todo_work(params)
            .await
            .map_err(|e| {
                error!("pause_todo_work failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Cancel an active work session without logging time
    ///
    /// Discards the current work session without creating a worklog entry.
    /// Useful when you started tracking the wrong todo or need to abandon work.
    ///
    /// # Examples
    /// - Cancel wrong session: `{"todo_id_or_index": "1"}`
    #[instrument(skip(self))]
    pub async fn cancel_todo_work(
        &self,
        params: CancelTodoWorkParams,
    ) -> anyhow::Result<CancelTodoWorkResult> {
        self.todo_tracker
            .cancel_todo_work(params)
            .await
            .map_err(|e| {
                error!("cancel_todo_work failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Get all active work sessions
    ///
    /// Returns a list of all currently active work sessions showing what's being
    /// tracked, when it started, and how long you've been working on it.
    ///
    /// # Examples
    /// - List all active sessions: `{}`
    #[instrument(skip(self))]
    pub async fn get_active_work_sessions(&self) -> anyhow::Result<GetActiveWorkSessionsResult> {
        self.todo_tracker
            .get_active_work_sessions()
            .await
            .map_err(|e| {
                error!("get_active_work_sessions failed: {}", e);
                anyhow::anyhow!(e)
            })
    }
}

// Add any additional implementation methods here that are NOT MCP tools
impl JiraMcpServer {
    /// Internal method to refresh current user cache
    #[allow(dead_code)]
    async fn refresh_current_user_cache(&self) -> JiraMcpResult<()> {
        match self.jira_client.get_current_user().await {
            Ok(user) => {
                let user_mapping = UserMapping {
                    account_id: user.account_id,
                    display_name: user.display_name,
                    email_address: user.email_address,
                    username: None,
                };
                self.cache.set_current_user(user_mapping)
            }
            Err(e) => Err(e),
        }
    }

    /// Internal method to validate tool parameters (common validations)
    #[allow(dead_code)]
    fn validate_common_params(
        &self,
        limit: Option<u32>,
        start_at: Option<u32>,
    ) -> JiraMcpResult<()> {
        if let Some(limit) = limit {
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

        if let Some(start_at) = start_at {
            if start_at > 10000 {
                return Err(JiraMcpError::invalid_param(
                    "start_at",
                    "start_at cannot exceed 10000",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, JiraConfig};

    // Note: These tests require a real JIRA instance for integration testing
    // Unit tests are included in individual modules

    #[tokio::test]
    async fn test_server_creation_with_invalid_config() {
        let config = JiraConfig {
            jira_url: "invalid-url".to_string(),
            auth: AuthConfig::Anonymous,
            ..Default::default()
        };

        // Should fail validation
        assert!(JiraMcpServer::with_config(config).await.is_err());
    }

    #[test]
    fn test_uptime_calculation() {
        let start_time = Instant::now();
        // Sleep is not needed for this test, just checking the calculation
        let elapsed = start_time.elapsed().as_secs();
        // elapsed is u64, which is always >= 0, so we just check it's a reasonable value
        assert!(elapsed < 10); // Should be very small since we just started
    }
}
