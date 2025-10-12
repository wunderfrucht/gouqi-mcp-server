/// Integration tests for issue CRUD operations
mod common;

use common::{test_issue_key, test_project_key, McpTestClient};
use serde_json::json;

#[test]
fn test_get_issue_details() {
    // Test get_issue_details for a known issue

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let response = client
        .call_tool(
            "get_issue_details",
            json!({
                "issue_key": issue_key
            }),
        )
        .expect("Failed to call get_issue_details");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify issue_details structure
    assert!(
        result.get("issue_details").is_some(),
        "No issue_details in result"
    );

    let issue = &result["issue_details"]["issue_info"];

    // Verify key fields
    assert!(issue.get("key").is_some(), "No key field");
    assert_eq!(
        issue["key"].as_str().unwrap(),
        issue_key,
        "Issue key mismatch"
    );
    assert!(issue.get("id").is_some(), "No id field");
    assert!(issue.get("summary").is_some(), "No summary field");
    assert!(issue.get("status").is_some(), "No status field");
    assert!(issue.get("issue_type").is_some(), "No issue_type field");
}

#[test]
fn test_get_issue_details_with_comments() {
    // Test get_issue_details with include_comments flag

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let response = client
        .call_tool(
            "get_issue_details",
            json!({
                "issue_key": issue_key,
                "include_comments": true
            }),
        )
        .expect("Failed to call get_issue_details");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Should include comments (structure depends on implementation)
    assert!(result.get("issue_details").is_some());
    assert!(result["issue_details"].get("issue_info").is_some());
}

#[test]
#[ignore] // include_relationships parameter is not implemented in get_issue_details tool
fn test_get_issue_details_with_relationships() {
    // Test get_issue_details with include_relationships flag

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let response = client
        .call_tool(
            "get_issue_details",
            json!({
                "issue_key": issue_key,
                "include_relationships": true
            }),
        )
        .expect("Failed to call get_issue_details");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("issue_details").is_some());
    // Relationships might be empty, but structure should be there
}

#[test]
#[should_panic(expected = "Tool call failed")]
fn test_get_issue_details_invalid_key() {
    // Test get_issue_details with invalid issue key
    // This should raise an error

    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "get_issue_details",
            json!({
                "issue_key": "INVALID-999999"
            }),
        )
        .expect("Failed to call get_issue_details");

    // This should fail
    McpTestClient::extract_tool_result(&response).expect("Should fail with invalid key");
}

#[test]
fn test_get_create_metadata() {
    // Test get_create_metadata for a project

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "get_create_metadata",
            json!({
                "project_key": project_key
            }),
        )
        .expect("Failed to call get_create_metadata");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    // Verify metadata structure
    assert!(result.get("project_name").is_some(), "No project_name");
    assert!(result.get("issue_types").is_some(), "No issue_types");

    let issue_types = result["issue_types"]
        .as_array()
        .expect("issue_types is not an array");

    assert!(!issue_types.is_empty(), "No issue types returned");

    // Verify issue type metadata
    let first_type = &issue_types[0];
    assert!(first_type.get("name").is_some(), "No name in issue type");
    assert!(
        first_type.get("required_fields").is_some(),
        "No required_fields"
    );

    let required_fields = first_type["required_fields"]
        .as_array()
        .expect("required_fields is not an array");
    assert!(
        !required_fields.is_empty(),
        "No required fields for issue type"
    );
}

#[test]
fn test_get_create_metadata_with_issue_type() {
    // Test get_create_metadata filtered by issue type

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    let response = client
        .call_tool(
            "get_create_metadata",
            json!({
                "project_key": project_key,
                "issue_type": "Task"
            }),
        )
        .expect("Failed to call get_create_metadata");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("issue_types").is_some());

    let issue_types = result["issue_types"]
        .as_array()
        .expect("issue_types is not an array");

    // If filtered by type, should have specific type or fewer results
    if !issue_types.is_empty() {
        // At least one should be Task (or the name mapping might differ)
        let has_task = issue_types
            .iter()
            .any(|it| it["name"].as_str().unwrap_or("") == "Task");

        // Either it found Task or no matching types were found
        assert!(
            has_task || issue_types.is_empty(),
            "Expected to find Task type when filtering"
        );
    }
}
