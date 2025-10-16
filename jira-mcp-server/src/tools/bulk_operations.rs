//! Bulk Operations Tools
//!
//! This module provides tools for performing bulk operations on multiple JIRA issues efficiently.
//! All bulk operations support parallel execution with configurable concurrency and proper error handling.

use crate::error::{JiraMcpError, JiraMcpResult};
use crate::jira_client::JiraClient;
use crate::tools::{CreateIssueParams, CreateIssueResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, error, info, instrument, warn};

const DEFAULT_MAX_CONCURRENT: usize = 5;
const MAX_ALLOWED_CONCURRENT: usize = 20;
const DEFAULT_MAX_RETRIES: usize = 3;
const DEFAULT_INITIAL_RETRY_DELAY_MS: u64 = 1000; // 1 second
const MIN_RETRY_DELAY_MS: u64 = 500; // 500ms minimum to prevent hammering
const MAX_RETRY_DELAY_MS: u64 = 30000; // 30 seconds

// =============================================================================
// Bulk Create Issues
// =============================================================================

/// Parameters for bulk creating issues
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkCreateIssuesParams {
    /// Project key where issues will be created
    pub project_key: String,

    /// List of issues to create (reuses CreateIssueParams structure)
    pub issues: Vec<CreateIssueParams>,

    /// Whether to stop on first error (default: false - continues on errors)
    #[serde(default)]
    pub stop_on_error: bool,

    /// Maximum number of concurrent API calls (default: 5, max: 20)
    #[serde(default)]
    pub max_concurrent: Option<usize>,

    /// Maximum number of retries for failed requests (default: 3)
    #[serde(default)]
    pub max_retries: Option<usize>,

    /// Initial retry delay in milliseconds (default: 1000ms, min: 500ms, doubles on each retry)
    #[serde(default)]
    pub initial_retry_delay_ms: Option<u64>,
}

/// Single issue creation result
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkIssueCreationResult {
    /// Index in the original request (0-based)
    pub index: usize,

    /// Created issue info (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<CreateIssueResult>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Whether this operation succeeded
    pub success: bool,
}

/// Result from bulk create issues operation
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkCreateIssuesResult {
    /// All creation results (successful and failed)
    pub results: Vec<BulkIssueCreationResult>,

    /// Number of successfully created issues
    pub success_count: usize,

    /// Number of failed creations
    pub failure_count: usize,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Summary message
    pub message: String,
}

// =============================================================================
// Bulk Transition Issues
// =============================================================================

/// Parameters for bulk transitioning issues
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkTransitionIssuesParams {
    /// List of issue keys to transition
    pub issue_keys: Vec<String>,

    /// Transition ID (optional if transition_name provided)
    #[serde(default)]
    pub transition_id: Option<String>,

    /// Transition name (optional if transition_id provided)
    #[serde(default)]
    pub transition_name: Option<String>,

    /// Optional comment to add when transitioning (same for all)
    #[serde(default)]
    pub comment: Option<String>,

    /// Optional resolution name (for transitions that require resolution)
    #[serde(default)]
    pub resolution: Option<String>,

    /// Whether to stop on first error (default: false)
    #[serde(default)]
    pub stop_on_error: bool,

    /// Maximum number of concurrent API calls (default: 5, max: 20)
    #[serde(default)]
    pub max_concurrent: Option<usize>,

    /// Maximum number of retries for failed requests (default: 3)
    #[serde(default)]
    pub max_retries: Option<usize>,

    /// Initial retry delay in milliseconds (default: 1000ms, min: 500ms, doubles on each retry)
    #[serde(default)]
    pub initial_retry_delay_ms: Option<u64>,
}

/// Single issue transition result
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkIssueTransitionResult {
    /// The issue key
    pub issue_key: String,

    /// New status (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_status: Option<String>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Whether this operation succeeded
    pub success: bool,
}

/// Result from bulk transition issues operation
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkTransitionIssuesResult {
    /// All transition results
    pub results: Vec<BulkIssueTransitionResult>,

    /// Number of successful transitions
    pub success_count: usize,

    /// Number of failed transitions
    pub failure_count: usize,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Summary message
    pub message: String,
}

// =============================================================================
// Bulk Update Fields
// =============================================================================

/// Parameters for bulk updating fields
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkUpdateFieldsParams {
    /// List of issue keys to update
    pub issue_keys: Vec<String>,

    /// Field updates to apply to all issues (field_id -> value)
    pub field_updates: HashMap<String, serde_json::Value>,

    /// Whether to stop on first error (default: false)
    #[serde(default)]
    pub stop_on_error: bool,

    /// Maximum number of concurrent API calls (default: 5, max: 20)
    #[serde(default)]
    pub max_concurrent: Option<usize>,

    /// Maximum number of retries for failed requests (default: 3)
    #[serde(default)]
    pub max_retries: Option<usize>,

    /// Initial retry delay in milliseconds (default: 1000ms, min: 500ms, doubles on each retry)
    #[serde(default)]
    pub initial_retry_delay_ms: Option<u64>,
}

/// Single issue update result
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkIssueUpdateResult {
    /// The issue key
    pub issue_key: String,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Whether this operation succeeded
    pub success: bool,
}

/// Result from bulk update fields operation
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkUpdateFieldsResult {
    /// All update results
    pub results: Vec<BulkIssueUpdateResult>,

    /// Number of successful updates
    pub success_count: usize,

    /// Number of failed updates
    pub failure_count: usize,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Summary message
    pub message: String,
}

// =============================================================================
// Bulk Assign Issues
// =============================================================================

/// Parameters for bulk assigning issues
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkAssignIssuesParams {
    /// List of issue keys to assign
    pub issue_keys: Vec<String>,

    /// Assignee to set (supports "me" for current user, or null to unassign)
    pub assignee: Option<String>,

    /// Whether to stop on first error (default: false)
    #[serde(default)]
    pub stop_on_error: bool,

    /// Maximum number of concurrent API calls (default: 5, max: 20)
    #[serde(default)]
    pub max_concurrent: Option<usize>,

    /// Maximum number of retries for failed requests (default: 3)
    #[serde(default)]
    pub max_retries: Option<usize>,

    /// Initial retry delay in milliseconds (default: 1000ms, min: 500ms, doubles on each retry)
    #[serde(default)]
    pub initial_retry_delay_ms: Option<u64>,
}

/// Result from bulk assign issues operation
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkAssignIssuesResult {
    /// All assignment results
    pub results: Vec<BulkIssueUpdateResult>,

    /// Number of successful assignments
    pub success_count: usize,

    /// Number of failed assignments
    pub failure_count: usize,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Assignee that was set
    pub assignee: String,

    /// Summary message
    pub message: String,
}

// =============================================================================
// Bulk Add Labels
// =============================================================================

/// Parameters for bulk adding/removing labels
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkAddLabelsParams {
    /// List of issue keys to update
    pub issue_keys: Vec<String>,

    /// Labels to add to all issues
    #[serde(default)]
    pub add_labels: Vec<String>,

    /// Labels to remove from all issues
    #[serde(default)]
    pub remove_labels: Vec<String>,

    /// Whether to stop on first error (default: false)
    #[serde(default)]
    pub stop_on_error: bool,

    /// Maximum number of concurrent API calls (default: 5, max: 20)
    #[serde(default)]
    pub max_concurrent: Option<usize>,

    /// Maximum number of retries for failed requests (default: 3)
    #[serde(default)]
    pub max_retries: Option<usize>,

    /// Initial retry delay in milliseconds (default: 1000ms, min: 500ms, doubles on each retry)
    #[serde(default)]
    pub initial_retry_delay_ms: Option<u64>,
}

/// Result from bulk add labels operation
#[derive(Debug, Serialize, JsonSchema)]
pub struct BulkAddLabelsResult {
    /// All label update results
    pub results: Vec<BulkIssueUpdateResult>,

    /// Number of successful updates
    pub success_count: usize,

    /// Number of failed updates
    pub failure_count: usize,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Summary message
    pub message: String,
}

// =============================================================================
// Tool Implementations
// =============================================================================

/// Tool for bulk operations on JIRA issues
pub struct BulkOperationsTool {
    jira_client: Arc<JiraClient>,
}

impl BulkOperationsTool {
    pub fn new(jira_client: Arc<JiraClient>) -> Self {
        Self { jira_client }
    }

    /// Get the effective concurrency limit
    fn get_concurrency_limit(&self, max_concurrent: Option<usize>) -> usize {
        max_concurrent
            .unwrap_or(DEFAULT_MAX_CONCURRENT)
            .clamp(1, MAX_ALLOWED_CONCURRENT)
    }

    /// Get the effective retry configuration
    fn get_retry_config(
        &self,
        max_retries: Option<usize>,
        initial_delay_ms: Option<u64>,
    ) -> (usize, u64) {
        let retries = max_retries.unwrap_or(DEFAULT_MAX_RETRIES);
        let mut initial_delay = initial_delay_ms.unwrap_or(DEFAULT_INITIAL_RETRY_DELAY_MS);

        // Enforce minimum retry delay to prevent hammering the API
        if initial_delay < MIN_RETRY_DELAY_MS {
            warn!(
                "initial_retry_delay_ms ({}) is below minimum ({}ms). Using minimum value to prevent API hammering.",
                initial_delay,
                MIN_RETRY_DELAY_MS
            );
            initial_delay = MIN_RETRY_DELAY_MS;
        }

        (retries, initial_delay)
    }

    /// Execute an async operation with exponential backoff retry logic
    async fn retry_with_backoff<F, Fut, T>(
        operation: F,
        max_retries: usize,
        initial_delay_ms: u64,
        operation_name: &str,
    ) -> JiraMcpResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = JiraMcpResult<T>>,
    {
        let mut attempt = 0;
        let mut delay_ms = initial_delay_ms;

        loop {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("{} succeeded after {} retries", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    // Check if error is retryable
                    let error_str = e.to_string();
                    let is_rate_limit =
                        error_str.contains("429") || error_str.contains("rate limit");
                    let is_timeout =
                        error_str.contains("timeout") || error_str.contains("timed out");
                    let is_server_error = error_str.contains("500")
                        || error_str.contains("502")
                        || error_str.contains("503");
                    let is_retryable = is_rate_limit || is_timeout || is_server_error;

                    if !is_retryable || attempt >= max_retries {
                        // Non-retryable error or max retries exceeded
                        if attempt > 0 {
                            warn!("{} failed after {} retries: {}", operation_name, attempt, e);
                        }
                        return Err(e);
                    }

                    // Log retry attempt
                    warn!(
                        "{} failed (attempt {}/{}), retrying in {}ms: {}",
                        operation_name,
                        attempt + 1,
                        max_retries,
                        delay_ms,
                        e
                    );

                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

                    // Exponential backoff: double the delay, but cap at MAX_RETRY_DELAY_MS
                    delay_ms = (delay_ms * 2).min(MAX_RETRY_DELAY_MS);
                    attempt += 1;
                }
            }
        }
    }

    // =========================================================================
    // Bulk Create Issues
    // =========================================================================

    #[instrument(skip(self))]
    pub async fn bulk_create_issues(
        &self,
        params: BulkCreateIssuesParams,
    ) -> JiraMcpResult<BulkCreateIssuesResult> {
        let start_time = std::time::Instant::now();
        let total_issues = params.issues.len();

        info!(
            "Bulk creating {} issues in project {}",
            total_issues, params.project_key
        );

        if total_issues == 0 {
            return Err(JiraMcpError::invalid_param(
                "issues",
                "Must provide at least one issue to create",
            ));
        }

        let concurrency_limit = self.get_concurrency_limit(params.max_concurrent);
        let (max_retries, initial_retry_delay_ms) =
            self.get_retry_config(params.max_retries, params.initial_retry_delay_ms);

        debug!(
            "Using concurrency limit: {} for {} issues, retries: {}, initial delay: {}ms",
            concurrency_limit, total_issues, max_retries, initial_retry_delay_ms
        );

        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let mut pending_count = 0;
        let mut success_count = 0;
        let mut failure_count = 0;

        for (index, mut issue_params) in params.issues.into_iter().enumerate() {
            // Set project_key if not provided
            if issue_params.project_key.is_none() {
                issue_params.project_key = Some(params.project_key.clone());
            }

            let client = Arc::clone(&self.jira_client);
            let stop_on_error = params.stop_on_error;
            let retry_config = (max_retries, initial_retry_delay_ms);

            join_set.spawn(async move {
                let result = Self::create_single_issue_with_retry(
                    client,
                    issue_params,
                    retry_config.0,
                    retry_config.1,
                )
                .await;
                (index, result)
            });

            pending_count += 1;

            // If we've reached concurrency limit, wait for some to complete
            if pending_count >= concurrency_limit {
                if let Some(result) = join_set.join_next().await {
                    match result {
                        Ok((idx, res)) => {
                            let success = res.is_ok();
                            if success {
                                success_count += 1;
                            } else {
                                failure_count += 1;
                            }
                            results.push((idx, res));

                            if !success && stop_on_error {
                                warn!("Stopping bulk create due to error (stop_on_error=true)");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                            failure_count += 1;
                        }
                    }
                    pending_count -= 1;
                }
            }
        }

        // Wait for remaining tasks
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((idx, res)) => {
                    let success = res.is_ok();
                    if success {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                    }
                    results.push((idx, res));
                }
                Err(e) => {
                    error!("Task join error: {}", e);
                    failure_count += 1;
                }
            }
        }

        // Sort results by original index
        results.sort_by_key(|(idx, _)| *idx);

        let final_results: Vec<BulkIssueCreationResult> = results
            .into_iter()
            .map(|(index, result)| match result {
                Ok(issue) => BulkIssueCreationResult {
                    index,
                    issue: Some(issue),
                    error: None,
                    success: true,
                },
                Err(e) => BulkIssueCreationResult {
                    index,
                    issue: None,
                    error: Some(e.to_string()),
                    success: false,
                },
            })
            .collect();

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Bulk create completed: {} succeeded, {} failed in {}ms",
            success_count, failure_count, execution_time
        );

        Ok(BulkCreateIssuesResult {
            results: final_results,
            success_count,
            failure_count,
            execution_time_ms: execution_time,
            message: format!(
                "Bulk created {}/{} issues successfully ({} failed)",
                success_count, total_issues, failure_count
            ),
        })
    }

    async fn create_single_issue_with_retry(
        client: Arc<JiraClient>,
        params: CreateIssueParams,
        max_retries: usize,
        initial_delay_ms: u64,
    ) -> JiraMcpResult<CreateIssueResult> {
        use crate::tools::CreateIssueTool;

        Self::retry_with_backoff(
            || async {
                let tool = CreateIssueTool::new(Arc::clone(&client));
                tool.execute(params.clone()).await
            },
            max_retries,
            initial_delay_ms,
            &format!("create_issue({})", params.summary),
        )
        .await
    }

    // =========================================================================
    // Bulk Transition Issues
    // =========================================================================

    #[instrument(skip(self))]
    pub async fn bulk_transition_issues(
        &self,
        params: BulkTransitionIssuesParams,
    ) -> JiraMcpResult<BulkTransitionIssuesResult> {
        let start_time = std::time::Instant::now();
        let total_issues = params.issue_keys.len();

        info!("Bulk transitioning {} issues", total_issues);

        if total_issues == 0 {
            return Err(JiraMcpError::invalid_param(
                "issue_keys",
                "Must provide at least one issue key",
            ));
        }

        if params.transition_id.is_none() && params.transition_name.is_none() {
            return Err(JiraMcpError::invalid_param(
                "transition_id or transition_name",
                "Must provide either transition_id or transition_name",
            ));
        }

        let concurrency_limit = self.get_concurrency_limit(params.max_concurrent);
        let (max_retries, initial_retry_delay_ms) =
            self.get_retry_config(params.max_retries, params.initial_retry_delay_ms);

        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let mut pending_count = 0;
        let mut success_count = 0;
        let mut failure_count = 0;

        for issue_key in params.issue_keys.iter() {
            let client = Arc::clone(&self.jira_client);
            let issue_key = issue_key.clone();
            let transition_id = params.transition_id.clone();
            let transition_name = params.transition_name.clone();
            let comment = params.comment.clone();
            let resolution = params.resolution.clone();
            let retry_config = (max_retries, initial_retry_delay_ms);

            join_set.spawn(async move {
                let result = Self::transition_single_issue_with_retry(
                    client,
                    issue_key.clone(),
                    transition_id,
                    transition_name,
                    comment,
                    resolution,
                    retry_config.0,
                    retry_config.1,
                )
                .await;
                (issue_key, result)
            });

            pending_count += 1;

            if pending_count >= concurrency_limit {
                if let Some(result) = join_set.join_next().await {
                    match result {
                        Ok((key, res)) => {
                            let success = res.is_ok();
                            if success {
                                success_count += 1;
                            } else {
                                failure_count += 1;
                            }
                            results.push((key, res));

                            if !success && params.stop_on_error {
                                warn!("Stopping bulk transition due to error (stop_on_error=true)");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                            failure_count += 1;
                        }
                    }
                    pending_count -= 1;
                }
            }
        }

        // Wait for remaining tasks
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((key, res)) => {
                    let success = res.is_ok();
                    if success {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                    }
                    results.push((key, res));
                }
                Err(e) => {
                    error!("Task join error: {}", e);
                    failure_count += 1;
                }
            }
        }

        let final_results: Vec<BulkIssueTransitionResult> = results
            .into_iter()
            .map(|(issue_key, result)| match result {
                Ok(new_status) => BulkIssueTransitionResult {
                    issue_key,
                    new_status: Some(new_status),
                    error: None,
                    success: true,
                },
                Err(e) => BulkIssueTransitionResult {
                    issue_key,
                    new_status: None,
                    error: Some(e.to_string()),
                    success: false,
                },
            })
            .collect();

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Bulk transition completed: {} succeeded, {} failed in {}ms",
            success_count, failure_count, execution_time
        );

        Ok(BulkTransitionIssuesResult {
            results: final_results,
            success_count,
            failure_count,
            execution_time_ms: execution_time,
            message: format!(
                "Bulk transitioned {}/{} issues successfully ({} failed)",
                success_count, total_issues, failure_count
            ),
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn transition_single_issue_with_retry(
        client: Arc<JiraClient>,
        issue_key: String,
        transition_id: Option<String>,
        transition_name: Option<String>,
        comment: Option<String>,
        resolution: Option<String>,
        max_retries: usize,
        initial_delay_ms: u64,
    ) -> JiraMcpResult<String> {
        use crate::tools::{TransitionIssueParams, TransitionIssueTool};

        let params = TransitionIssueParams {
            issue_key: issue_key.clone(),
            transition_id,
            transition_name,
            comment,
            resolution,
        };

        Self::retry_with_backoff(
            || async {
                let tool = TransitionIssueTool::new(Arc::clone(&client));
                let result = tool.execute(params.clone()).await?;
                Ok(result.transition_used.to_status)
            },
            max_retries,
            initial_delay_ms,
            &format!("transition_issue({})", issue_key),
        )
        .await
    }

    // =========================================================================
    // Bulk Update Fields
    // =========================================================================

    #[instrument(skip(self))]
    pub async fn bulk_update_fields(
        &self,
        params: BulkUpdateFieldsParams,
    ) -> JiraMcpResult<BulkUpdateFieldsResult> {
        let start_time = std::time::Instant::now();
        let total_issues = params.issue_keys.len();

        info!("Bulk updating fields on {} issues", total_issues);

        if total_issues == 0 {
            return Err(JiraMcpError::invalid_param(
                "issue_keys",
                "Must provide at least one issue key",
            ));
        }

        if params.field_updates.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "field_updates",
                "Must provide at least one field to update",
            ));
        }

        let concurrency_limit = self.get_concurrency_limit(params.max_concurrent);
        let (max_retries, initial_retry_delay_ms) =
            self.get_retry_config(params.max_retries, params.initial_retry_delay_ms);

        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let mut pending_count = 0;
        let mut success_count = 0;
        let mut failure_count = 0;

        for issue_key in params.issue_keys.iter() {
            let client = Arc::clone(&self.jira_client);
            let issue_key = issue_key.clone();
            let field_updates = params.field_updates.clone();
            let retry_config = (max_retries, initial_retry_delay_ms);

            join_set.spawn(async move {
                let result = Self::update_single_issue_fields_with_retry(
                    client,
                    issue_key.clone(),
                    field_updates,
                    retry_config.0,
                    retry_config.1,
                )
                .await;
                (issue_key, result)
            });

            pending_count += 1;

            if pending_count >= concurrency_limit {
                if let Some(result) = join_set.join_next().await {
                    match result {
                        Ok((key, res)) => {
                            let success = res.is_ok();
                            if success {
                                success_count += 1;
                            } else {
                                failure_count += 1;
                            }
                            results.push((key, res));

                            if !success && params.stop_on_error {
                                warn!("Stopping bulk update due to error (stop_on_error=true)");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                            failure_count += 1;
                        }
                    }
                    pending_count -= 1;
                }
            }
        }

        // Wait for remaining tasks
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((key, res)) => {
                    let success = res.is_ok();
                    if success {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                    }
                    results.push((key, res));
                }
                Err(e) => {
                    error!("Task join error: {}", e);
                    failure_count += 1;
                }
            }
        }

        let final_results: Vec<BulkIssueUpdateResult> = results
            .into_iter()
            .map(|(issue_key, result)| match result {
                Ok(_) => BulkIssueUpdateResult {
                    issue_key,
                    error: None,
                    success: true,
                },
                Err(e) => BulkIssueUpdateResult {
                    issue_key,
                    error: Some(e.to_string()),
                    success: false,
                },
            })
            .collect();

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Bulk update completed: {} succeeded, {} failed in {}ms",
            success_count, failure_count, execution_time
        );

        Ok(BulkUpdateFieldsResult {
            results: final_results,
            success_count,
            failure_count,
            execution_time_ms: execution_time,
            message: format!(
                "Bulk updated {}/{} issues successfully ({} failed)",
                success_count, total_issues, failure_count
            ),
        })
    }

    async fn update_single_issue_fields_with_retry(
        client: Arc<JiraClient>,
        issue_key: String,
        field_updates: HashMap<String, serde_json::Value>,
        max_retries: usize,
        initial_delay_ms: u64,
    ) -> JiraMcpResult<()> {
        Self::retry_with_backoff(
            || async {
                let update_body = serde_json::json!({
                    "fields": field_updates
                });

                let endpoint = format!("/issue/{}", issue_key);
                client
                    .client
                    .put::<(), _>("api", &endpoint, update_body)
                    .await
                    .map_err(|e| {
                        if e.to_string().contains("404") {
                            JiraMcpError::not_found("issue", &issue_key)
                        } else {
                            JiraMcpError::internal(format!("Failed to update issue: {}", e))
                        }
                    })?;

                Ok(())
            },
            max_retries,
            initial_delay_ms,
            &format!("update_fields({})", issue_key),
        )
        .await
    }

    // =========================================================================
    // Bulk Assign Issues
    // =========================================================================

    #[instrument(skip(self))]
    pub async fn bulk_assign_issues(
        &self,
        params: BulkAssignIssuesParams,
    ) -> JiraMcpResult<BulkAssignIssuesResult> {
        let start_time = std::time::Instant::now();
        let total_issues = params.issue_keys.len();

        info!("Bulk assigning {} issues", total_issues);

        if total_issues == 0 {
            return Err(JiraMcpError::invalid_param(
                "issue_keys",
                "Must provide at least one issue key",
            ));
        }

        // Resolve "me" to account ID if needed
        let assignee_value = match params.assignee.as_deref() {
            Some("me") | Some("self") => {
                let user = self.jira_client.get_current_user().await?;
                Some(user.account_id)
            }
            Some(assignee) => Some(assignee.to_string()),
            None => None,
        };

        let assignee_display = assignee_value
            .as_deref()
            .unwrap_or("Unassigned")
            .to_string();

        let concurrency_limit = self.get_concurrency_limit(params.max_concurrent);
        let (max_retries, initial_retry_delay_ms) =
            self.get_retry_config(params.max_retries, params.initial_retry_delay_ms);

        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let mut pending_count = 0;
        let mut success_count = 0;
        let mut failure_count = 0;

        for issue_key in params.issue_keys.iter() {
            let client = Arc::clone(&self.jira_client);
            let issue_key = issue_key.clone();
            let assignee = assignee_value.clone();
            let retry_config = (max_retries, initial_retry_delay_ms);

            join_set.spawn(async move {
                let result = Self::assign_single_issue_with_retry(
                    client,
                    issue_key.clone(),
                    assignee,
                    retry_config.0,
                    retry_config.1,
                )
                .await;
                (issue_key, result)
            });

            pending_count += 1;

            if pending_count >= concurrency_limit {
                if let Some(result) = join_set.join_next().await {
                    match result {
                        Ok((key, res)) => {
                            let success = res.is_ok();
                            if success {
                                success_count += 1;
                            } else {
                                failure_count += 1;
                            }
                            results.push((key, res));

                            if !success && params.stop_on_error {
                                warn!("Stopping bulk assign due to error (stop_on_error=true)");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                            failure_count += 1;
                        }
                    }
                    pending_count -= 1;
                }
            }
        }

        // Wait for remaining tasks
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((key, res)) => {
                    let success = res.is_ok();
                    if success {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                    }
                    results.push((key, res));
                }
                Err(e) => {
                    error!("Task join error: {}", e);
                    failure_count += 1;
                }
            }
        }

        let final_results: Vec<BulkIssueUpdateResult> = results
            .into_iter()
            .map(|(issue_key, result)| match result {
                Ok(_) => BulkIssueUpdateResult {
                    issue_key,
                    error: None,
                    success: true,
                },
                Err(e) => BulkIssueUpdateResult {
                    issue_key,
                    error: Some(e.to_string()),
                    success: false,
                },
            })
            .collect();

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Bulk assign completed: {} succeeded, {} failed in {}ms",
            success_count, failure_count, execution_time
        );

        Ok(BulkAssignIssuesResult {
            results: final_results,
            success_count,
            failure_count,
            execution_time_ms: execution_time,
            assignee: assignee_display,
            message: format!(
                "Bulk assigned {}/{} issues successfully ({} failed)",
                success_count, total_issues, failure_count
            ),
        })
    }

    async fn assign_single_issue_with_retry(
        client: Arc<JiraClient>,
        issue_key: String,
        assignee: Option<String>,
        max_retries: usize,
        initial_delay_ms: u64,
    ) -> JiraMcpResult<()> {
        Self::retry_with_backoff(
            || async {
                let update_body = if let Some(assignee_id) = &assignee {
                    serde_json::json!({
                        "fields": {
                            "assignee": {
                                "accountId": assignee_id
                            }
                        }
                    })
                } else {
                    serde_json::json!({
                        "fields": {
                            "assignee": null
                        }
                    })
                };

                let endpoint = format!("/issue/{}", issue_key);
                client
                    .client
                    .put::<(), _>("api", &endpoint, update_body)
                    .await
                    .map_err(|e| {
                        if e.to_string().contains("404") {
                            JiraMcpError::not_found("issue", &issue_key)
                        } else {
                            JiraMcpError::internal(format!("Failed to assign issue: {}", e))
                        }
                    })?;

                Ok(())
            },
            max_retries,
            initial_delay_ms,
            &format!("assign_issue({})", issue_key),
        )
        .await
    }

    // =========================================================================
    // Bulk Add Labels
    // =========================================================================

    #[instrument(skip(self))]
    pub async fn bulk_add_labels(
        &self,
        params: BulkAddLabelsParams,
    ) -> JiraMcpResult<BulkAddLabelsResult> {
        let start_time = std::time::Instant::now();
        let total_issues = params.issue_keys.len();

        info!("Bulk updating labels on {} issues", total_issues);

        if total_issues == 0 {
            return Err(JiraMcpError::invalid_param(
                "issue_keys",
                "Must provide at least one issue key",
            ));
        }

        if params.add_labels.is_empty() && params.remove_labels.is_empty() {
            return Err(JiraMcpError::invalid_param(
                "add_labels or remove_labels",
                "Must provide at least one label to add or remove",
            ));
        }

        let concurrency_limit = self.get_concurrency_limit(params.max_concurrent);
        let (max_retries, initial_retry_delay_ms) =
            self.get_retry_config(params.max_retries, params.initial_retry_delay_ms);

        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let mut pending_count = 0;
        let mut success_count = 0;
        let mut failure_count = 0;

        for issue_key in params.issue_keys.iter() {
            let client = Arc::clone(&self.jira_client);
            let issue_key = issue_key.clone();
            let add_labels = params.add_labels.clone();
            let remove_labels = params.remove_labels.clone();
            let retry_config = (max_retries, initial_retry_delay_ms);

            join_set.spawn(async move {
                let result = Self::update_single_issue_labels_with_retry(
                    client,
                    issue_key.clone(),
                    add_labels,
                    remove_labels,
                    retry_config.0,
                    retry_config.1,
                )
                .await;
                (issue_key, result)
            });

            pending_count += 1;

            if pending_count >= concurrency_limit {
                if let Some(result) = join_set.join_next().await {
                    match result {
                        Ok((key, res)) => {
                            let success = res.is_ok();
                            if success {
                                success_count += 1;
                            } else {
                                failure_count += 1;
                            }
                            results.push((key, res));

                            if !success && params.stop_on_error {
                                warn!(
                                    "Stopping bulk label update due to error (stop_on_error=true)"
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                            failure_count += 1;
                        }
                    }
                    pending_count -= 1;
                }
            }
        }

        // Wait for remaining tasks
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((key, res)) => {
                    let success = res.is_ok();
                    if success {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                    }
                    results.push((key, res));
                }
                Err(e) => {
                    error!("Task join error: {}", e);
                    failure_count += 1;
                }
            }
        }

        let final_results: Vec<BulkIssueUpdateResult> = results
            .into_iter()
            .map(|(issue_key, result)| match result {
                Ok(_) => BulkIssueUpdateResult {
                    issue_key,
                    error: None,
                    success: true,
                },
                Err(e) => BulkIssueUpdateResult {
                    issue_key,
                    error: Some(e.to_string()),
                    success: false,
                },
            })
            .collect();

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Bulk label update completed: {} succeeded, {} failed in {}ms",
            success_count, failure_count, execution_time
        );

        Ok(BulkAddLabelsResult {
            results: final_results,
            success_count,
            failure_count,
            execution_time_ms: execution_time,
            message: format!(
                "Bulk updated labels on {}/{} issues successfully ({} failed)",
                success_count, total_issues, failure_count
            ),
        })
    }

    async fn update_single_issue_labels_with_retry(
        client: Arc<JiraClient>,
        issue_key: String,
        add_labels: Vec<String>,
        remove_labels: Vec<String>,
        max_retries: usize,
        initial_delay_ms: u64,
    ) -> JiraMcpResult<()> {
        Self::retry_with_backoff(
            || async {
                let mut label_operations = Vec::new();

                for label in &add_labels {
                    label_operations.push(serde_json::json!({"add": label}));
                }

                for label in &remove_labels {
                    label_operations.push(serde_json::json!({"remove": label}));
                }

                let update_body = serde_json::json!({
                    "update": {
                        "labels": label_operations
                    }
                });

                let endpoint = format!("/issue/{}", issue_key);
                client
                    .client
                    .put::<(), _>("api", &endpoint, update_body)
                    .await
                    .map_err(|e| {
                        if e.to_string().contains("404") {
                            JiraMcpError::not_found("issue", &issue_key)
                        } else {
                            JiraMcpError::internal(format!("Failed to update labels: {}", e))
                        }
                    })?;

                Ok(())
            },
            max_retries,
            initial_delay_ms,
            &format!("update_labels({})", issue_key),
        )
        .await
    }
}
