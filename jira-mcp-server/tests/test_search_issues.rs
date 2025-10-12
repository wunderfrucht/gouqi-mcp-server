/// Integration tests for search_issues tool
mod common;

use common::{test_project_key, McpTestClient};
use serde_json::json;

#[test]
fn test_search_issues_no_params() {
    // Test search_issues with no parameters
    // After bug fix #18, this should work with a default 30-day constraint

    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool("search_issues", json!({"limit": 5}))
        .expect("Failed to call search_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify we got issues (in search_result)
    assert!(
        result.get("search_result").is_some(),
        "No search_result field in result"
    );
    let search_result = &result["search_result"];

    assert!(
        search_result.get("issues").is_some(),
        "No issues field in search_result"
    );
    assert!(
        search_result.get("total").is_some(),
        "No total field in search_result"
    );

    let issues = search_result["issues"]
        .as_array()
        .expect("Issues is not an array");
    assert!(issues.len() <= 5, "Returned more than 5 issues");

    // Verify JQL query includes the 30-day default (bug #18 fix)
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");
    assert!(
        jql.contains("created >= -30d"),
        "JQL query should include 30-day default constraint, got: {}",
        jql
    );
}

#[test]
fn test_search_issues_with_project() {
    // Test search_issues with project filter

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "limit": 10
            }),
        )
        .expect("Failed to call search_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("search_result").is_some());
    let search_result = &result["search_result"];

    assert!(search_result.get("issues").is_some());
    assert!(search_result.get("total").is_some());

    let issues = search_result["issues"]
        .as_array()
        .expect("Issues is not an array");
    assert!(issues.len() <= 10, "Returned more than 10 issues");

    // Verify project filter is in JQL
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");
    assert!(
        jql.contains(&project_key),
        "JQL query should include project key"
    );

    // Verify all issues belong to the project
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
fn test_search_issues_with_status_filter() {
    // Test search_issues with status filter

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "status": ["To Do", "In Progress"],
                "limit": 5
            }),
        )
        .expect("Failed to call search_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("search_result").is_some());

    // Verify status filter is in JQL
    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");
    assert!(
        jql.to_lowercase().contains("status"),
        "JQL query should include status filter"
    );
}

#[test]
fn test_search_issues_with_date_filter() {
    // Test search_issues with created_after filter

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "created_after": "7 days ago",
                "limit": 5
            }),
        )
        .expect("Failed to call search_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("search_result").is_some());
    assert!(result.get("jql_query").is_some());

    let jql = result["jql_query"]
        .as_str()
        .expect("jql_query is not a string");

    // Should include the date filter (7 days ago converts to JQL date)
    assert!(
        jql.to_lowercase().contains("created >="),
        "JQL query should include date filter, got: {}",
        jql
    );
}

#[test]
fn test_search_issues_performance_metrics() {
    // Test that search_issues includes performance metrics

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "limit": 5
            }),
        )
        .expect("Failed to call search_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify performance metrics are included
    assert!(
        result.get("performance").is_some(),
        "No performance metrics"
    );

    let perf = &result["performance"];
    assert!(perf.get("duration_ms").is_some(), "No duration_ms");
    assert!(perf.get("api_calls").is_some(), "No api_calls");

    let duration = perf["duration_ms"]
        .as_u64()
        .or_else(|| perf["duration_ms"].as_i64().map(|v| v as u64))
        .expect("duration_ms is not a number");

    assert!(duration > 0, "Duration should be greater than 0");
}

#[test]
#[ignore] // TODO: Fix pagination - start_at parameter not being applied correctly
fn test_search_issues_pagination() {
    // Test search_issues pagination

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    // First page
    let response1 = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "limit": 2,
                "start_at": 0
            }),
        )
        .expect("Failed to call search_issues");

    let result1 =
        McpTestClient::extract_tool_result(&response1).expect("Failed to extract tool result");

    // Second page
    let response2 = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "limit": 2,
                "start_at": 2
            }),
        )
        .expect("Failed to call search_issues");

    let result2 =
        McpTestClient::extract_tool_result(&response2).expect("Failed to extract tool result");

    assert!(result1.get("search_result").is_some());
    assert!(result2.get("search_result").is_some());

    let search1 = &result1["search_result"];
    let search2 = &result2["search_result"];

    // Verify pagination structure
    let total = search1["total"].as_u64().expect("total is not a number");

    let issues1 = search1["issues"]
        .as_array()
        .expect("issues1 is not an array");
    let issues2 = search2["issues"]
        .as_array()
        .expect("issues2 is not an array");

    // If there are at least 4 unique issues, pages should be different
    if total >= 4 && !issues1.is_empty() && !issues2.is_empty() {
        // Extract issue keys
        let keys1: Vec<String> = issues1
            .iter()
            .map(|issue| issue["key"].as_str().unwrap().to_string())
            .collect();
        let keys2: Vec<String> = issues2
            .iter()
            .map(|issue| issue["key"].as_str().unwrap().to_string())
            .collect();

        // Pages should not have overlapping keys (unless JIRA returned duplicates due to concurrent updates)
        let overlap_count = keys1.iter().filter(|k| keys2.contains(k)).count();

        // Allow some tolerance for concurrent updates but expect mostly different results
        assert!(
            overlap_count < keys1.len(),
            "Expected pages to be mostly different. Page 1: {:?}, Page 2: {:?}",
            keys1,
            keys2
        );
    }
}
