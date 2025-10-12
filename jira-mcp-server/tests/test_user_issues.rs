/// Integration tests for get_user_issues tool
mod common;

use common::{test_project_key, McpTestClient};
use serde_json::json;

#[test]
fn test_get_user_issues_default() {
    // Test get_user_issues with no parameters
    // Should return issues assigned to the current user
    // After bug fix #19, this should work correctly

    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool("get_user_issues", json!({}))
        .expect("Failed to call get_user_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify response structure
    assert!(
        result.get("search_result").is_some(),
        "No search_result field"
    );
    assert!(
        result.get("resolved_user").is_some(),
        "No resolved_user field"
    );
    assert!(result.get("jql_query").is_some(), "No jql_query field");

    // Verify JQL does NOT have invalid syntax (bug #19)
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");

    // ORDER BY should NOT be joined with AND
    assert!(
        !jql.contains(" AND ORDER BY"),
        "JQL should not have 'AND ORDER BY' (bug #19). Got: {}",
        jql
    );

    // Should have proper format: conditions ORDER BY ...
    assert!(
        jql.contains("ORDER BY updated DESC"),
        "JQL should end with 'ORDER BY updated DESC'. Got: {}",
        jql
    );

    // Verify user info
    let user_info = &result["resolved_user"];
    assert!(user_info.get("account_id").is_some(), "No account_id");
    assert!(user_info.get("display_name").is_some(), "No display_name");

    let is_current_user = user_info["is_current_user"]
        .as_bool()
        .expect("is_current_user is not a boolean");
    assert!(is_current_user, "User should be current user");

    // Verify search results
    let search_result = &result["search_result"];
    assert!(search_result.get("issues").is_some(), "No issues");
    assert!(search_result.get("total").is_some(), "No total");
}

#[test]
fn test_get_user_issues_with_status_filter() {
    // Test get_user_issues with status filter

    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "get_user_issues",
            json!({
                "status_filter": ["To Do", "In Progress"]
            }),
        )
        .expect("Failed to call get_user_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify JQL syntax is correct (bug #19)
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");
    assert!(!jql.contains(" AND ORDER BY"), "Invalid JQL syntax");
    assert!(jql.contains("ORDER BY updated DESC"), "Missing ORDER BY");

    // Verify status filter is applied
    assert!(
        jql.to_lowercase().contains("status"),
        "JQL should include status filter"
    );
}

#[test]
fn test_get_user_issues_with_project_filter() {
    // Test get_user_issues with project filter

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "get_user_issues",
            json!({
                "project_filter": [project_key.clone()]
            }),
        )
        .expect("Failed to call get_user_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify JQL syntax (bug #19)
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");
    assert!(!jql.contains(" AND ORDER BY"), "Invalid JQL syntax");
    assert!(jql.contains(&project_key), "JQL should include project key");

    // Verify all returned issues are from the specified project
    let issues = result["search_result"]["issues"]
        .as_array()
        .expect("issues is not an array");

    for issue in issues {
        let key = issue["key"].as_str().expect("Issue key is not a string");
        assert!(
            key.starts_with(&project_key),
            "Issue {} does not belong to project {}",
            key,
            project_key
        );
    }
}

#[test]
fn test_get_user_issues_with_issue_types() {
    // Test get_user_issues with issue type filter

    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "get_user_issues",
            json!({
                "issue_types": ["Task", "Story"]
            }),
        )
        .expect("Failed to call get_user_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify JQL syntax (bug #19)
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");
    assert!(!jql.contains(" AND ORDER BY"), "Invalid JQL syntax");
    assert!(
        jql.to_lowercase().contains("issuetype"),
        "JQL should include issue type filter"
    );
}

#[test]
fn test_get_user_issues_performance_metrics() {
    // Test that get_user_issues includes performance metrics

    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool("get_user_issues", json!({"limit": 5}))
        .expect("Failed to call get_user_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify performance metrics
    assert!(
        result.get("performance").is_some(),
        "No performance metrics"
    );

    let perf = &result["performance"];
    assert!(perf.get("duration_ms").is_some(), "No duration_ms");
    assert!(perf.get("api_calls").is_some(), "No api_calls");
    assert!(perf.get("user_cache_hit").is_some(), "No user_cache_hit");
    assert!(
        perf.get("query_complexity").is_some(),
        "No query_complexity"
    );

    let duration = perf["duration_ms"]
        .as_u64()
        .or_else(|| perf["duration_ms"].as_i64().map(|v| v as u64))
        .expect("duration_ms is not a number");
    assert!(duration > 0, "Duration should be greater than 0");
}

#[test]
fn test_get_user_issues_combined_filters() {
    // Test get_user_issues with multiple filters combined

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "get_user_issues",
            json!({
                "project_filter": [project_key],
                "status_filter": ["To Do", "In Progress"],
                "issue_types": ["Task"],
                "limit": 5
            }),
        )
        .expect("Failed to call get_user_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify JQL syntax with multiple filters (bug #19)
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");

    assert!(
        !jql.contains(" AND ORDER BY"),
        "Invalid JQL syntax with multiple filters"
    );

    // All filters should be joined with AND, then ORDER BY at the end
    assert!(jql.contains("assignee"), "JQL should have assignee filter");
    assert!(
        jql.contains("ORDER BY updated DESC"),
        "JQL should end with ORDER BY"
    );

    // Multiple filters should be present
    let and_count = jql.matches(" AND ").count();
    assert!(
        and_count >= 2,
        "Expected at least 2 AND clauses for multiple filters"
    );
}
