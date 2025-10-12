/// Integration tests for labels and components tools
mod common;

use common::{test_issue_key, test_project_key, McpTestClient};
use serde_json::json;

#[test]
fn test_get_available_labels_global() {
    // Test getting global labels
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "get_available_labels",
            json!({
                "max_results": 50
            }),
        )
        .expect("Failed to call get_available_labels");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("labels").is_some(), "No labels field");
    assert!(result.get("total").is_some(), "No total field");
    assert!(result.get("is_last").is_some(), "No is_last field");

    let labels = result["labels"]
        .as_array()
        .expect("labels is not an array");

    // Labels array can be empty, but should exist
    assert!(labels.len() <= 50, "Should respect max_results limit");
}

#[test]
fn test_get_available_labels_for_project() {
    // Test getting labels for a specific project
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "get_available_labels",
            json!({
                "project_key": project_key,
                "max_results": 100
            }),
        )
        .expect("Failed to call get_available_labels");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("labels").is_some(), "No labels field");
    assert!(result.get("total").is_some(), "No total field");

    let labels = result["labels"]
        .as_array()
        .expect("labels is not an array");

    // Labels should be unique and sorted
    for i in 0..labels.len().saturating_sub(1) {
        let current = labels[i].as_str().unwrap_or("");
        let next = labels[i + 1].as_str().unwrap_or("");
        assert!(
            current <= next,
            "Labels should be sorted: {} should come before {}",
            current,
            next
        );
    }
}

#[test]
fn test_manage_labels_add() {
    // Test adding labels to an issue
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let test_label = format!("test-label-{}", chrono::Utc::now().timestamp());

    let response = client
        .call_tool(
            "manage_labels",
            json!({
                "issue_key": issue_key,
                "add_labels": [test_label.clone()]
            }),
        )
        .expect("Failed to call manage_labels");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert_eq!(
        result["issue_key"].as_str().unwrap(),
        issue_key,
        "Issue key mismatch"
    );
    assert!(result.get("added").is_some(), "No added field");
    assert!(result.get("current_labels").is_some(), "No current_labels field");
    assert!(result.get("message").is_some(), "No message field");

    let added = result["added"]
        .as_array()
        .expect("added is not an array");
    assert!(
        added.contains(&json!(test_label)),
        "Test label should be in added list"
    );

    let current_labels = result["current_labels"]
        .as_array()
        .expect("current_labels is not an array");
    assert!(
        current_labels.contains(&json!(test_label)),
        "Test label should be in current labels"
    );
}

#[test]
fn test_manage_labels_remove() {
    // Test removing labels from an issue
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let test_label = format!("test-remove-label-{}", chrono::Utc::now().timestamp());

    // First add a label
    client
        .call_tool(
            "manage_labels",
            json!({
                "issue_key": issue_key,
                "add_labels": [test_label.clone()]
            }),
        )
        .expect("Failed to add label");

    // Now remove it
    let response = client
        .call_tool(
            "manage_labels",
            json!({
                "issue_key": issue_key,
                "remove_labels": [test_label.clone()]
            }),
        )
        .expect("Failed to call manage_labels");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("removed").is_some(), "No removed field");

    let removed = result["removed"]
        .as_array()
        .expect("removed is not an array");
    assert!(
        removed.contains(&json!(test_label)),
        "Test label should be in removed list"
    );

    let current_labels = result["current_labels"]
        .as_array()
        .expect("current_labels is not an array");
    assert!(
        !current_labels.contains(&json!(test_label)),
        "Test label should not be in current labels after removal"
    );
}

#[test]
fn test_manage_labels_replace_all() {
    // Test replacing all labels on an issue
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let new_labels = vec![
        format!("replaced-label-1-{}", chrono::Utc::now().timestamp()),
        format!("replaced-label-2-{}", chrono::Utc::now().timestamp()),
    ];

    let response = client
        .call_tool(
            "manage_labels",
            json!({
                "issue_key": issue_key,
                "add_labels": new_labels,
                "replace_all": true
            }),
        )
        .expect("Failed to call manage_labels");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("added").is_some(), "No added field");
    assert!(result.get("current_labels").is_some(), "No current_labels field");

    let current_labels = result["current_labels"]
        .as_array()
        .expect("current_labels is not an array");

    // All current labels should be from the new_labels list
    for label in current_labels {
        let label_str = label.as_str().unwrap();
        assert!(
            label_str.starts_with("replaced-label-"),
            "Label {} should be one of the replaced labels",
            label_str
        );
    }
}

#[test]
#[should_panic(expected = "Tool call failed")]
fn test_manage_labels_invalid_issue() {
    // Test manage_labels with invalid issue key
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "manage_labels",
            json!({
                "issue_key": "INVALID-999999",
                "add_labels": ["test-label"]
            }),
        )
        .expect("Failed to call manage_labels");

    // Should fail
    McpTestClient::extract_tool_result(&response).expect("Should fail with invalid issue key");
}

#[test]
fn test_get_available_components() {
    // Test getting components for a project
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "get_available_components",
            json!({
                "project_key": project_key
            }),
        )
        .expect("Failed to call get_available_components");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("components").is_some(), "No components field");
    assert!(result.get("total").is_some(), "No total field");
    assert_eq!(
        result["project_key"].as_str().unwrap(),
        project_key,
        "Project key mismatch"
    );

    let components = result["components"]
        .as_array()
        .expect("components is not an array");

    // If components exist, verify their structure
    if !components.is_empty() {
        let first_component = &components[0];
        assert!(first_component.get("id").is_some(), "No id field");
        assert!(first_component.get("name").is_some(), "No name field");
        // description is optional
    }
}

#[test]
#[ignore] // This test requires knowing available components and may modify issue state
fn test_update_components() {
    // Test updating components on an issue
    // This test is ignored because it requires knowing which components are available
    // and may modify the test issue's state
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();
    let project_key = test_project_key();

    // First get available components
    let components_response = client
        .call_tool(
            "get_available_components",
            json!({
                "project_key": project_key
            }),
        )
        .expect("Failed to get components");

    let components_result = McpTestClient::extract_tool_result(&components_response)
        .expect("Failed to extract components result");

    let available_components = components_result["components"]
        .as_array()
        .expect("components is not an array");

    if available_components.is_empty() {
        eprintln!("No components available in project, skipping update test");
        return;
    }

    // Use the first available component
    let component_name = available_components[0]["name"]
        .as_str()
        .expect("component name is not a string");

    let response = client
        .call_tool(
            "update_components",
            json!({
                "issue_key": issue_key,
                "components": [component_name]
            }),
        )
        .expect("Failed to call update_components");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert_eq!(
        result["issue_key"].as_str().unwrap(),
        issue_key,
        "Issue key mismatch"
    );
    assert!(result.get("components").is_some(), "No components field");
    assert!(result.get("message").is_some(), "No message field");

    let components = result["components"]
        .as_array()
        .expect("components is not an array");

    // Should have exactly one component
    assert_eq!(components.len(), 1, "Should have exactly one component");
    assert_eq!(
        components[0]["name"].as_str().unwrap(),
        component_name,
        "Component name mismatch"
    );
}

#[test]
#[ignore] // This test may modify issue state
fn test_update_components_clear() {
    // Test clearing all components from an issue
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let response = client
        .call_tool(
            "update_components",
            json!({
                "issue_key": issue_key,
                "components": []
            }),
        )
        .expect("Failed to call update_components");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert_eq!(
        result["issue_key"].as_str().unwrap(),
        issue_key,
        "Issue key mismatch"
    );

    let components = result["components"]
        .as_array()
        .expect("components is not an array");

    // Should have no components
    assert!(
        components.is_empty(),
        "Components should be empty after clearing"
    );
}

#[test]
#[should_panic(expected = "Tool call failed")]
fn test_update_components_invalid_issue() {
    // Test update_components with invalid issue key
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "update_components",
            json!({
                "issue_key": "INVALID-999999",
                "components": ["TestComponent"]
            }),
        )
        .expect("Failed to call update_components");

    // Should fail
    McpTestClient::extract_tool_result(&response)
        .expect("Should fail with invalid issue key");
}

#[test]
#[should_panic(expected = "Tool call failed")]
fn test_get_available_components_invalid_project() {
    // Test get_available_components with invalid project key
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "get_available_components",
            json!({
                "project_key": "INVALID999"
            }),
        )
        .expect("Failed to call get_available_components");

    // Should fail
    McpTestClient::extract_tool_result(&response)
        .expect("Should fail with invalid project key");
}

#[test]
fn test_search_issues_with_labels() {
    // Test search_issues with label filtering
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    // Search with a label filter
    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "labels": ["test"],
                "limit": 10
            }),
        )
        .expect("Failed to call search_issues");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("search_result").is_some(), "No search_result field");
    assert!(result.get("jql_query").is_some(), "No jql_query field");

    let jql = result["jql_query"].as_str().unwrap();
    assert!(
        jql.contains("labels ="),
        "JQL should contain label filter: {}",
        jql
    );
}

#[test]
fn test_search_issues_with_components() {
    // Test search_issues with component filtering
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    // Search with a component filter
    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "components": ["Backend"],
                "limit": 10
            }),
        )
        .expect("Failed to call search_issues");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("search_result").is_some(), "No search_result field");
    assert!(result.get("jql_query").is_some(), "No jql_query field");

    let jql = result["jql_query"].as_str().unwrap();
    assert!(
        jql.contains("component"),
        "JQL should contain component filter: {}",
        jql
    );
}

#[test]
fn test_search_issues_with_multiple_components() {
    // Test search_issues with multiple component filtering
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    // Search with multiple component filters
    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": project_key,
                "components": ["Backend", "Frontend"],
                "limit": 10
            }),
        )
        .expect("Failed to call search_issues");

    let result = McpTestClient::extract_tool_result(&response)
        .expect("Failed to extract tool result");

    // Verify structure
    assert!(result.get("jql_query").is_some(), "No jql_query field");

    let jql = result["jql_query"].as_str().unwrap();
    assert!(
        jql.contains("component IN"),
        "JQL should contain component IN clause for multiple components: {}",
        jql
    );
}
