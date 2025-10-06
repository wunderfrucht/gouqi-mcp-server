//! Todo tracking tool for managing todos in issue descriptions with time tracking
//!
//! This module provides comprehensive todo management:
//! - Extract todos from issue descriptions (markdown checkbox format)
//! - Add new todos to descriptions
//! - Update todo status (complete/incomplete)
//! - Track time spent on todos
//! - Log work time to JIRA worklogs

use crate::cache::MetadataCache;
use crate::config::JiraConfig;
use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::{JiraClient, WorklogInfo};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

/// Todo status for filtering
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TodoStatus {
    /// Open/incomplete todos
    Open,
    /// Completed/checked todos
    Completed,
    /// Work in progress (has active work session)
    Wip,
}

/// A todo item extracted from or to be added to an issue description
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoItem {
    /// The todo text content
    pub text: String,

    /// Whether the todo is completed (checked)
    pub completed: bool,

    /// Current status (open, completed, or wip)
    pub status: TodoStatus,

    /// Line number in the description (0-based)
    pub line_number: usize,

    /// Unique ID for tracking (generated from content hash)
    pub id: String,
}

/// Parameters for setting the base issue
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetTodoBaseParams {
    /// The JIRA issue key to use as the base (e.g., "PROJ-123")
    pub issue_key: String,
}

/// Result from setting the base issue
#[derive(Debug, Serialize)]
pub struct SetTodoBaseResult {
    /// The base issue key that was set
    pub base_issue_key: String,

    /// Success message
    pub message: String,
}

/// Parameters for listing todos
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTodosParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    /// If not provided, uses the current base issue
    #[serde(default)]
    pub issue_key: Option<String>,

    /// Filter by status (open, completed, wip)
    /// If not provided, returns all todos
    #[serde(default)]
    pub status_filter: Option<Vec<TodoStatus>>,
}

/// Result from listing todos
#[derive(Debug, Serialize)]
pub struct ListTodosResult {
    /// List of todos found
    pub todos: Vec<TodoItem>,

    /// Total count
    pub total_count: usize,

    /// Issue key
    pub issue_key: String,
}

/// Parameters for adding a new todo
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddTodoParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    /// If not provided, uses the current base issue
    #[serde(default)]
    pub issue_key: Option<String>,

    /// The todo text to add
    pub todo_text: String,

    /// Whether to add at the beginning or end of todos section (default: end)
    #[serde(default)]
    pub prepend: bool,
}

/// Result from adding a todo
#[derive(Debug, Serialize)]
pub struct AddTodoResult {
    /// The added todo
    pub todo: TodoItem,

    /// Success message
    pub message: String,

    /// Updated description
    pub updated_description: String,
}

/// Parameters for updating a todo status
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateTodoParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    /// If not provided, uses the current base issue
    #[serde(default)]
    pub issue_key: Option<String>,

    /// The todo ID or 1-based index to update
    pub todo_id_or_index: String,

    /// Whether to mark as completed
    pub completed: bool,
}

/// Result from updating a todo
#[derive(Debug, Serialize)]
pub struct UpdateTodoResult {
    /// The updated todo
    pub todo: TodoItem,

    /// Success message
    pub message: String,
}

/// Parameters for starting work on a todo
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StartTodoWorkParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    /// If not provided, uses the current base issue
    #[serde(default)]
    pub issue_key: Option<String>,

    /// The todo ID or 1-based index
    pub todo_id_or_index: String,
}

/// Result from starting work
#[derive(Debug, Serialize)]
pub struct StartTodoWorkResult {
    /// The todo being worked on
    pub todo: TodoItem,

    /// When work started
    pub started_at: String,

    /// Success message
    pub message: String,
}

/// Parameters for completing work on a todo
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompleteTodoWorkParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    /// If not provided, uses the current base issue
    #[serde(default)]
    pub issue_key: Option<String>,

    /// The todo ID or 1-based index
    pub todo_id_or_index: String,

    /// Optional comment for the worklog entry
    pub worklog_comment: Option<String>,

    /// Whether to mark the todo as completed (default: true)
    #[serde(default = "default_true")]
    pub mark_completed: bool,

    /// Explicit time spent in hours (for multi-day sessions)
    /// Required if session spans more than 24 hours
    pub time_spent_hours: Option<f64>,

    /// Explicit time spent in minutes (for multi-day sessions)
    /// Alternative to time_spent_hours
    pub time_spent_minutes: Option<u64>,

    /// Explicit time spent in seconds (for multi-day sessions)
    /// Most precise option, alternative to hours/minutes
    pub time_spent_seconds: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Parameters for pausing work on a todo
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PauseTodoWorkParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    /// If not provided, uses the current base issue
    #[serde(default)]
    pub issue_key: Option<String>,

    /// The todo ID or 1-based index
    pub todo_id_or_index: String,

    /// Optional comment for the worklog entry
    pub worklog_comment: Option<String>,
}

/// Result from pausing work
#[derive(Debug, Serialize)]
pub struct PauseTodoWorkResult {
    /// The todo that was worked on
    pub todo: TodoItem,

    /// Time spent in seconds
    pub time_spent_seconds: u64,

    /// Time spent in human-readable format
    pub time_spent_formatted: String,

    /// The created worklog
    pub worklog: WorklogInfo,

    /// Success message
    pub message: String,
}

/// Parameters for canceling work on a todo
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelTodoWorkParams {
    /// The JIRA issue key (e.g., "PROJ-123")
    /// If not provided, uses the current base issue
    #[serde(default)]
    pub issue_key: Option<String>,

    /// The todo ID or 1-based index
    pub todo_id_or_index: String,
}

/// Result from canceling work
#[derive(Debug, Serialize)]
pub struct CancelTodoWorkResult {
    /// The todo that was being worked on
    pub todo: TodoItem,

    /// Time that would have been logged (in seconds)
    pub discarded_time_seconds: u64,

    /// Success message
    pub message: String,
}

/// Active work session information
#[derive(Debug, Serialize, JsonSchema)]
pub struct ActiveWorkSession {
    /// Issue key
    pub issue_key: String,

    /// Todo ID
    pub todo_id: String,

    /// Todo text
    pub todo_text: String,

    /// When work started
    pub started_at: String,

    /// Current duration in seconds
    pub duration_seconds: u64,

    /// Current duration formatted
    pub duration_formatted: String,
}

/// Result from getting active work sessions
#[derive(Debug, Serialize)]
pub struct GetActiveWorkSessionsResult {
    /// List of active work sessions
    pub sessions: Vec<ActiveWorkSession>,

    /// Total number of active sessions
    pub total_count: usize,
}

/// Result from completing work
#[derive(Debug, Serialize)]
pub struct CompleteTodoWorkResult {
    /// The todo that was worked on
    pub todo: TodoItem,

    /// Time spent in seconds
    pub time_spent_seconds: u64,

    /// Time spent in human-readable format
    pub time_spent_formatted: String,

    /// The created worklog
    pub worklog: WorklogInfo,

    /// Success message
    pub message: String,
}

/// Work tracking entry
#[derive(Debug, Clone)]
struct WorkSession {
    issue_key: String,
    todo_id: String,
    todo_text: String,
    started_at: DateTime<Utc>,
}

/// Todo tracker implementation
pub struct TodoTracker {
    jira_client: Arc<JiraClient>,
    #[allow(dead_code)]
    config: Arc<JiraConfig>,
    #[allow(dead_code)]
    cache: Arc<MetadataCache>,
    // Track active work sessions
    active_sessions: Arc<RwLock<HashMap<String, WorkSession>>>,
    // Base issue context
    base_issue: Arc<RwLock<Option<String>>>,
}

impl TodoTracker {
    pub fn new(
        jira_client: Arc<JiraClient>,
        config: Arc<JiraConfig>,
        cache: Arc<MetadataCache>,
    ) -> Self {
        Self {
            jira_client,
            config,
            cache,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            base_issue: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the base issue for todo operations
    #[instrument(skip(self))]
    pub async fn set_todo_base(
        &self,
        params: SetTodoBaseParams,
    ) -> JiraMcpResult<SetTodoBaseResult> {
        info!("Setting base issue to: {}", params.issue_key);

        // Verify the issue exists
        let _issue = self
            .jira_client
            .get_issue_details(&params.issue_key, false, false, false)
            .await?;

        // Set the base issue
        {
            let mut base = self.base_issue.write().await;
            *base = Some(params.issue_key.clone());
        }

        info!("Base issue set successfully: {}", params.issue_key);

        Ok(SetTodoBaseResult {
            base_issue_key: params.issue_key.clone(),
            message: format!(
                "Base issue set to {}. You can now omit issue_key in todo commands.",
                params.issue_key
            ),
        })
    }

    /// Get the base issue or return an error if not set
    async fn get_issue_key(&self, provided: Option<String>) -> JiraMcpResult<String> {
        if let Some(key) = provided {
            return Ok(key);
        }

        let base = self.base_issue.read().await;
        base.clone().ok_or_else(|| {
            JiraMcpError::invalid_param(
                "issue_key",
                "No issue_key provided and no base issue set. Use set_todo_base first or provide issue_key.",
            )
        })
    }

    /// List todos from an issue description
    #[instrument(skip(self))]
    pub async fn list_todos(&self, params: ListTodosParams) -> JiraMcpResult<ListTodosResult> {
        let issue_key = self.get_issue_key(params.issue_key).await?;
        info!("Listing todos from issue: {}", issue_key);

        // Get the issue details
        let issue = self
            .jira_client
            .get_issue_details(&issue_key, false, false, false)
            .await?;

        let description = issue.issue_info.description.as_deref().unwrap_or("");
        let mut todos = self.parse_todos_with_status(description, &issue_key).await;

        // Apply status filter if provided
        if let Some(ref filters) = params.status_filter {
            todos.retain(|todo| filters.contains(&todo.status));
        }

        info!(
            "Found {} todos in issue {} (filtered)",
            todos.len(),
            issue_key
        );

        Ok(ListTodosResult {
            total_count: todos.len(),
            todos,
            issue_key,
        })
    }

    /// Add a new todo to an issue description
    #[instrument(skip(self))]
    pub async fn add_todo(&self, params: AddTodoParams) -> JiraMcpResult<AddTodoResult> {
        let issue_key = self.get_issue_key(params.issue_key).await?;
        info!("Adding todo to issue {}: {}", issue_key, params.todo_text);

        // Get current description
        let issue = self
            .jira_client
            .get_issue_details(&issue_key, false, false, false)
            .await?;
        let current_description = issue.issue_info.description.as_deref().unwrap_or("");

        // Generate new description with the added todo
        let new_description =
            Self::add_todo_to_description(current_description, &params.todo_text, params.prepend);

        // Update the issue description
        self.update_description(&issue_key, &new_description)
            .await?;

        // Parse todos again to get the newly added one
        let updated_todos = self
            .parse_todos_with_status(&new_description, &issue_key)
            .await;
        let new_todo = if params.prepend {
            updated_todos.first()
        } else {
            updated_todos.last()
        }
        .cloned()
        .ok_or_else(|| JiraMcpError::internal("Failed to find newly added todo"))?;

        info!("Successfully added todo to issue {}", issue_key);

        Ok(AddTodoResult {
            todo: new_todo,
            message: format!("Todo added to issue {}", issue_key),
            updated_description: new_description,
        })
    }

    /// Update a todo's completion status
    #[instrument(skip(self))]
    pub async fn update_todo(&self, params: UpdateTodoParams) -> JiraMcpResult<UpdateTodoResult> {
        let issue_key = self.get_issue_key(params.issue_key).await?;
        info!(
            "Updating todo in issue {}: {}",
            issue_key, params.todo_id_or_index
        );

        // Get current description and todos
        let issue = self
            .jira_client
            .get_issue_details(&issue_key, false, false, false)
            .await?;
        let current_description = issue.issue_info.description.as_deref().unwrap_or("");
        let todos = self
            .parse_todos_with_status(current_description, &issue_key)
            .await;

        // Find the todo to update
        let todo_index = Self::resolve_todo_index(&todos, &params.todo_id_or_index)?;
        let todo = todos
            .get(todo_index)
            .ok_or_else(|| JiraMcpError::invalid_param("todo_id_or_index", "Todo not found"))?;

        // Update the description
        let new_description =
            Self::update_todo_status(current_description, todo.line_number, params.completed);

        self.update_description(&issue_key, &new_description)
            .await?;

        // Get updated todo
        let updated_todos = self
            .parse_todos_with_status(&new_description, &issue_key)
            .await;
        let updated_todo = updated_todos
            .get(todo_index)
            .cloned()
            .ok_or_else(|| JiraMcpError::internal("Failed to find updated todo"))?;

        info!("Successfully updated todo in issue {}", issue_key);

        Ok(UpdateTodoResult {
            todo: updated_todo,
            message: format!(
                "Todo {} in issue {}",
                if params.completed {
                    "completed"
                } else {
                    "reopened"
                },
                issue_key
            ),
        })
    }

    /// Start tracking work time on a todo
    #[instrument(skip(self))]
    pub async fn start_todo_work(
        &self,
        params: StartTodoWorkParams,
    ) -> JiraMcpResult<StartTodoWorkResult> {
        let issue_key = self.get_issue_key(params.issue_key).await?;
        info!(
            "Starting work on todo in issue {}: {}",
            issue_key, params.todo_id_or_index
        );

        // Get todos
        let issue = self
            .jira_client
            .get_issue_details(&issue_key, false, false, false)
            .await?;
        let description = issue.issue_info.description.as_deref().unwrap_or("");
        let todos = self.parse_todos_with_status(description, &issue_key).await;

        // Find the todo
        let todo_index = Self::resolve_todo_index(&todos, &params.todo_id_or_index)?;
        let todo = todos
            .get(todo_index)
            .cloned()
            .ok_or_else(|| JiraMcpError::invalid_param("todo_id_or_index", "Todo not found"))?;

        // Create work session
        let session_key = format!("{}:{}", issue_key, todo.id);
        let started_at = Utc::now();

        let session = WorkSession {
            issue_key: issue_key.clone(),
            todo_id: todo.id.clone(),
            todo_text: todo.text.clone(),
            started_at,
        };

        // Store the session
        {
            let mut sessions = self.active_sessions.write().await;
            if sessions.contains_key(&session_key) {
                warn!("Work session already active for {}:{}", issue_key, todo.id);
                return Err(JiraMcpError::invalid_param(
                    "todo_id_or_index",
                    "Work session already active for this todo. Complete the existing session first.".to_string(),
                ));
            }
            sessions.insert(session_key, session);
        }

        info!("Started work tracking for todo in issue {}", issue_key);

        Ok(StartTodoWorkResult {
            todo,
            started_at: started_at.to_rfc3339(),
            message: format!("Started tracking work on todo in issue {}", issue_key),
        })
    }

    /// Pause work on a todo and log time spent so far
    #[instrument(skip(self))]
    pub async fn pause_todo_work(
        &self,
        params: PauseTodoWorkParams,
    ) -> JiraMcpResult<PauseTodoWorkResult> {
        let issue_key = self.get_issue_key(params.issue_key).await?;
        info!(
            "Pausing work on todo in issue {}: {}",
            issue_key, params.todo_id_or_index
        );

        // Get todos
        let issue = self
            .jira_client
            .get_issue_details(&issue_key, false, false, false)
            .await?;
        let description = issue.issue_info.description.as_deref().unwrap_or("");
        let todos = self.parse_todos_with_status(description, &issue_key).await;

        // Find the todo
        let todo_index = Self::resolve_todo_index(&todos, &params.todo_id_or_index)?;
        let todo = todos
            .get(todo_index)
            .cloned()
            .ok_or_else(|| JiraMcpError::invalid_param("todo_id_or_index", "Todo not found"))?;

        // Get and remove the work session
        let session_key = format!("{}:{}", issue_key, todo.id);
        let session = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_key).ok_or_else(|| {
                JiraMcpError::invalid_param(
                    "todo_id_or_index",
                    "No active work session for this todo. Start work first.",
                )
            })?
        };

        // Calculate time spent
        let now = Utc::now();
        let duration = now.signed_duration_since(session.started_at);
        let time_spent_seconds = duration.num_seconds().max(0) as u64;

        // Add worklog entry
        let worklog_comment = params
            .worklog_comment
            .unwrap_or_else(|| format!("Partial work on todo: {}", todo.text));

        let worklog = self
            .jira_client
            .add_worklog(
                &issue_key,
                time_spent_seconds,
                Some(worklog_comment),
                Some(session.started_at),
            )
            .await?;

        let time_formatted = Self::format_duration(time_spent_seconds);

        info!(
            "Paused work on todo in issue {}: {} logged",
            issue_key, time_formatted
        );

        Ok(PauseTodoWorkResult {
            todo,
            time_spent_seconds,
            time_spent_formatted: time_formatted.clone(),
            worklog,
            message: format!(
                "Logged {} to issue {}. Session paused, you can start work again later.",
                time_formatted, issue_key
            ),
        })
    }

    /// Cancel work on a todo without logging time
    #[instrument(skip(self))]
    pub async fn cancel_todo_work(
        &self,
        params: CancelTodoWorkParams,
    ) -> JiraMcpResult<CancelTodoWorkResult> {
        let issue_key = self.get_issue_key(params.issue_key).await?;
        info!(
            "Canceling work on todo in issue {}: {}",
            issue_key, params.todo_id_or_index
        );

        // Get todos
        let issue = self
            .jira_client
            .get_issue_details(&issue_key, false, false, false)
            .await?;
        let description = issue.issue_info.description.as_deref().unwrap_or("");
        let todos = self.parse_todos_with_status(description, &issue_key).await;

        // Find the todo
        let todo_index = Self::resolve_todo_index(&todos, &params.todo_id_or_index)?;
        let todo = todos
            .get(todo_index)
            .cloned()
            .ok_or_else(|| JiraMcpError::invalid_param("todo_id_or_index", "Todo not found"))?;

        // Get and remove the work session
        let session_key = format!("{}:{}", issue_key, todo.id);
        let session = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_key).ok_or_else(|| {
                JiraMcpError::invalid_param(
                    "todo_id_or_index",
                    "No active work session for this todo.",
                )
            })?
        };

        // Calculate time that would have been logged
        let now = Utc::now();
        let duration = now.signed_duration_since(session.started_at);
        let discarded_time_seconds = duration.num_seconds().max(0) as u64;

        info!(
            "Canceled work on todo in issue {}: {} discarded",
            issue_key,
            Self::format_duration(discarded_time_seconds)
        );

        Ok(CancelTodoWorkResult {
            todo,
            discarded_time_seconds,
            message: format!(
                "Work session canceled. {} of work was discarded (not logged).",
                Self::format_duration(discarded_time_seconds)
            ),
        })
    }

    /// Get all active work sessions
    #[instrument(skip(self))]
    pub async fn get_active_work_sessions(&self) -> JiraMcpResult<GetActiveWorkSessionsResult> {
        info!("Getting active work sessions");

        let sessions = self.active_sessions.read().await;
        let now = Utc::now();

        let active_sessions: Vec<ActiveWorkSession> = sessions
            .values()
            .map(|session| {
                let duration = now.signed_duration_since(session.started_at);
                let duration_seconds = duration.num_seconds().max(0) as u64;

                ActiveWorkSession {
                    issue_key: session.issue_key.clone(),
                    todo_id: session.todo_id.clone(),
                    todo_text: session.todo_text.clone(),
                    started_at: session.started_at.to_rfc3339(),
                    duration_seconds,
                    duration_formatted: Self::format_duration(duration_seconds),
                }
            })
            .collect();

        let total = active_sessions.len();

        info!("Found {} active work sessions", total);

        Ok(GetActiveWorkSessionsResult {
            sessions: active_sessions,
            total_count: total,
        })
    }

    /// Complete work on a todo and log time
    #[instrument(skip(self))]
    pub async fn complete_todo_work(
        &self,
        params: CompleteTodoWorkParams,
    ) -> JiraMcpResult<CompleteTodoWorkResult> {
        let issue_key = self.get_issue_key(params.issue_key).await?;
        info!(
            "Completing work on todo in issue {}: {}",
            issue_key, params.todo_id_or_index
        );

        // Get todos
        let issue = self
            .jira_client
            .get_issue_details(&issue_key, false, false, false)
            .await?;
        let description = issue.issue_info.description.as_deref().unwrap_or("");
        let todos = self.parse_todos_with_status(description, &issue_key).await;

        // Find the todo
        let todo_index = Self::resolve_todo_index(&todos, &params.todo_id_or_index)?;
        let mut todo = todos
            .get(todo_index)
            .cloned()
            .ok_or_else(|| JiraMcpError::invalid_param("todo_id_or_index", "Todo not found"))?;

        // Get and remove the work session
        let session_key = format!("{}:{}", issue_key, todo.id);
        let session = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_key).ok_or_else(|| {
                JiraMcpError::invalid_param(
                    "todo_id_or_index",
                    "No active work session for this todo. Start work first.",
                )
            })?
        };

        let now = Utc::now();
        let duration = now.signed_duration_since(session.started_at);
        let auto_calculated_seconds = duration.num_seconds().max(0) as u64;

        // Check if the session crosses a day boundary (different dates)
        let started_date = session.started_at.date_naive();
        let current_date = now.date_naive();
        let crosses_day_boundary = started_date != current_date;

        // Also check for sessions longer than 24 hours
        const SECONDS_PER_DAY: u64 = 86400; // 24 * 60 * 60
        let is_multi_day = crosses_day_boundary || auto_calculated_seconds > SECONDS_PER_DAY;

        // Calculate explicit time if provided
        let explicit_time_seconds = if let Some(seconds) = params.time_spent_seconds {
            Some(seconds)
        } else if let Some(minutes) = params.time_spent_minutes {
            Some(minutes * 60)
        } else {
            params.time_spent_hours.map(|hours| (hours * 3600.0) as u64)
        };

        // Validate multi-day sessions
        if is_multi_day && explicit_time_seconds.is_none() {
            let session_duration = Self::format_duration(auto_calculated_seconds);
            let started_datetime = session.started_at.format("%B %d, %Y at %H:%M");
            let current_datetime = now.format("%B %d, %Y at %H:%M");

            let day_info = if crosses_day_boundary {
                format!(
                    "Started {} and ending {}",
                    started_datetime, current_datetime
                )
            } else {
                format!("Started {}", started_datetime)
            };

            return Err(JiraMcpError::invalid_param(
                "time_spent_hours",
                format!(
                    "Session spans multiple days. {}\n\
                     Auto-calculated time: {}\n\
                     \n\
                     Please provide explicit time worked:\n\
                     - Use 'time_spent_hours' (e.g., 8.5 for 8.5 hours)\n\
                     - Use 'time_spent_minutes' (e.g., 480 for 8 hours)\n\
                     - Use 'time_spent_seconds' for precise control\n\
                     \n\
                     Or better yet, use 'pause_todo_work' to log partial progress.\n\
                     \n\
                     Example: {{\"todo_id_or_index\": \"{}\", \"time_spent_hours\": 8}}",
                    day_info, session_duration, params.todo_id_or_index
                ),
            ));
        }

        // Use explicit time if provided, otherwise use auto-calculated
        let time_spent_seconds = explicit_time_seconds.unwrap_or(auto_calculated_seconds);

        // Warn if explicit time differs significantly from auto-calculated
        if let Some(explicit) = explicit_time_seconds {
            if auto_calculated_seconds > 0 {
                let diff_percent = if explicit > auto_calculated_seconds {
                    ((explicit - auto_calculated_seconds) as f64 / auto_calculated_seconds as f64)
                        * 100.0
                } else {
                    ((auto_calculated_seconds - explicit) as f64 / auto_calculated_seconds as f64)
                        * 100.0
                };

                if diff_percent > 20.0 {
                    warn!(
                        "Explicit time ({}) differs from auto-calculated ({}) by {:.1}%",
                        Self::format_duration(explicit),
                        Self::format_duration(auto_calculated_seconds),
                        diff_percent
                    );
                }
            }
        }

        // Add worklog entry
        let worklog_comment = params
            .worklog_comment
            .unwrap_or_else(|| format!("Work on todo: {}", todo.text));

        let worklog = self
            .jira_client
            .add_worklog(
                &issue_key,
                time_spent_seconds,
                Some(worklog_comment),
                Some(session.started_at),
            )
            .await?;

        // Mark todo as completed if requested
        if params.mark_completed && !todo.completed {
            let updated_description = Self::update_todo_status(description, todo.line_number, true);
            self.update_description(&issue_key, &updated_description)
                .await?;
            todo.completed = true;
            todo.status = TodoStatus::Completed;
        }

        let time_formatted = Self::format_duration(time_spent_seconds);

        info!(
            "Completed work on todo in issue {}: {} logged",
            issue_key, time_formatted
        );

        Ok(CompleteTodoWorkResult {
            todo,
            time_spent_seconds,
            time_spent_formatted: time_formatted.clone(),
            worklog,
            message: format!("Logged {} to issue {}", time_formatted, issue_key),
        })
    }

    // Helper methods

    /// Parse markdown checkboxes from description with status detection
    async fn parse_todos_with_status(&self, description: &str, issue_key: &str) -> Vec<TodoItem> {
        let mut todos = Vec::new();
        let sessions = self.active_sessions.read().await;

        for (line_num, line) in description.lines().enumerate() {
            let trimmed = line.trim();

            // Match patterns like:
            // - [ ] todo item
            // - [x] todo item
            // * [ ] todo item
            // * [x] todo item
            if let Some(todo_text) = Self::parse_checkbox_line(trimmed) {
                let completed = trimmed.contains("[x]") || trimmed.contains("[X]");
                let id = Self::generate_todo_id(&todo_text, line_num);

                // Determine status based on completion and active work session
                let session_key = format!("{}:{}", issue_key, id);
                let status = if sessions.contains_key(&session_key) {
                    TodoStatus::Wip
                } else if completed {
                    TodoStatus::Completed
                } else {
                    TodoStatus::Open
                };

                todos.push(TodoItem {
                    text: todo_text,
                    completed,
                    status,
                    line_number: line_num,
                    id,
                });
            }
        }

        todos
    }

    /// Parse a single checkbox line
    fn parse_checkbox_line(line: &str) -> Option<String> {
        // Match: - [ ] text or - [x] text
        if line.starts_with("- [") || line.starts_with("* [") {
            if let Some(idx) = line.find(']') {
                if idx + 1 < line.len() {
                    let text = line[idx + 1..].trim().to_string();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
        }
        None
    }

    /// Generate a unique ID for a todo
    fn generate_todo_id(text: &str, line_num: usize) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        line_num.hash(&mut hasher);
        format!("todo-{:x}", hasher.finish())
    }

    /// Add a todo to a description (preserves all existing content)
    fn add_todo_to_description(description: &str, todo_text: &str, prepend: bool) -> String {
        let new_todo_line = format!("- [ ] {}", todo_text);

        // Convert to owned strings to avoid lifetime issues
        let mut lines: Vec<String> = description.lines().map(String::from).collect();

        // Find existing todos
        let mut first_todo_line = None;
        let mut last_todo_line = None;

        for (i, line) in lines.iter().enumerate() {
            if Self::parse_checkbox_line(line.trim()).is_some() {
                if first_todo_line.is_none() {
                    first_todo_line = Some(i);
                }
                last_todo_line = Some(i);
            }
        }

        if let (Some(first), Some(last)) = (first_todo_line, last_todo_line) {
            // Todos exist, insert near them
            let insert_pos = if prepend { first } else { last + 1 };
            lines.insert(insert_pos, new_todo_line);
        } else {
            // No todos exist, look for a "Todos" section header
            let mut todos_header_idx = None;
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim().to_lowercase();
                if trimmed.starts_with("## todo")
                    || trimmed.starts_with("# todo")
                    || trimmed == "todos:"
                    || trimmed == "**todos**"
                {
                    todos_header_idx = Some(i);
                    break;
                }
            }

            if let Some(header_idx) = todos_header_idx {
                // Insert after the header (and skip any blank lines)
                let mut insert_pos = header_idx + 1;
                while insert_pos < lines.len() && lines[insert_pos].trim().is_empty() {
                    insert_pos += 1;
                }
                lines.insert(insert_pos, new_todo_line);
            } else {
                // Create a new Todos section at the end
                if !lines.is_empty() && !lines.last().unwrap().trim().is_empty() {
                    lines.push(String::new());
                }
                lines.push("## Todos".to_string());
                lines.push(String::new());
                lines.push(new_todo_line);
            }
        }

        lines.join("\n")
    }

    /// Update todo status in description (preserves all existing content)
    fn update_todo_status(description: &str, line_number: usize, completed: bool) -> String {
        let mut lines: Vec<String> = description.lines().map(String::from).collect();

        if line_number < lines.len() {
            let line = &lines[line_number];
            let new_line = if completed {
                // Replace [ ] with [x], but preserve the rest of the line
                line.replace("[ ]", "[x]")
            } else {
                // Replace [x] or [X] with [ ], but preserve the rest of the line
                line.replace("[x]", "[ ]").replace("[X]", "[ ]")
            };
            lines[line_number] = new_line;
        }

        lines.join("\n")
    }

    /// Resolve a todo ID or 1-based index to an array index
    fn resolve_todo_index(todos: &[TodoItem], id_or_index: &str) -> JiraMcpResult<usize> {
        // Try as ID first
        if let Some(pos) = todos.iter().position(|t| t.id == id_or_index) {
            return Ok(pos);
        }

        // Try as 1-based index
        if let Ok(idx) = id_or_index.parse::<usize>() {
            if idx > 0 && idx <= todos.len() {
                return Ok(idx - 1);
            }
        }

        Err(JiraMcpError::invalid_param(
            "todo_id_or_index",
            format!(
                "Invalid todo ID or index: {}. Use a todo ID or 1-based index (1-{})",
                id_or_index,
                todos.len()
            ),
        ))
    }

    /// Format duration in seconds to human-readable format
    fn format_duration(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m", minutes)
        } else {
            format!("{}s", secs)
        }
    }

    /// Update issue description
    async fn update_description(&self, issue_key: &str, description: &str) -> JiraMcpResult<()> {
        use crate::tools::update_description::{
            UpdateDescription, UpdateDescriptionParams, UpdateMode,
        };

        let updater = UpdateDescription::new(self.jira_client.clone());
        updater
            .execute(UpdateDescriptionParams {
                issue_key: issue_key.to_string(),
                content: description.to_string(),
                mode: UpdateMode::Replace,
            })
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(TodoTracker::format_duration(45), "45s");
        assert_eq!(TodoTracker::format_duration(90), "1m");
        assert_eq!(TodoTracker::format_duration(3665), "1h 1m");
        assert_eq!(TodoTracker::format_duration(7200), "2h 0m");
    }
}
