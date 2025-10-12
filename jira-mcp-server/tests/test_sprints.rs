/// Integration tests for sprint management tools
mod common;

use common::McpTestClient;
use serde_json::json;

#[test]
fn test_list_sprints_tool_exists() {
    // Test that list_sprints tool is properly registered
    // This is a basic smoke test - actual sprint testing requires a JIRA board

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Test with a dummy board_id - we expect a proper error, not a "tool not found" error
    let response = client.call_tool("list_sprints", json!({"board_id": 999999}));

    // We expect either success (if board exists) or a proper JIRA error (not MCP error)
    // The important thing is that the tool is registered and callable
    match response {
        Ok(_) => {
            // Success is fine - means the tool worked
        }
        Err(e) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("404") || err_msg.contains("sprint") || err_msg.contains("board"),
                "Expected JIRA error, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_get_sprint_info_tool_exists() {
    // Test that get_sprint_info tool is properly registered

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Test with a dummy sprint_id
    let response = client.call_tool("get_sprint_info", json!({"sprint_id": 999999}));

    // We expect either success or a proper JIRA error, not "tool not found"
    match response {
        Ok(_) => {
            // Success is fine
        }
        Err(e) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("404") || err_msg.contains("sprint"),
                "Expected JIRA error, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_get_sprint_issues_tool_exists() {
    // Test that get_sprint_issues tool is properly registered

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Test with a dummy sprint_id
    let response = client.call_tool("get_sprint_issues", json!({"sprint_id": 999999}));

    // We expect either success or a proper JIRA error, not "tool not found"
    match response {
        Ok(_) => {
            // Success is fine
        }
        Err(e) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("404") || err_msg.contains("sprint"),
                "Expected JIRA error, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_move_to_sprint_tool_exists() {
    // Test that move_to_sprint tool is properly registered

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Test with dummy data - empty issue_keys should trigger validation error
    let response = client.call_tool(
        "move_to_sprint",
        json!({
            "sprint_id": 999999,
            "issue_keys": []
        }),
    );

    // We expect an error (either validation or JIRA error)
    // The important thing is that the tool is registered and callable
    match response {
        Ok(_) => {
            // If it succeeded, that's unexpected but still shows the tool works
        }
        Err(e) => {
            let err_msg = e.to_string();
            // Should get some error about issue_keys, sprint, or 404
            assert!(
                err_msg.contains("issue_keys")
                    || err_msg.contains("required")
                    || err_msg.contains("404")
                    || err_msg.contains("sprint"),
                "Expected error about issue_keys or sprint, got: {}",
                err_msg
            );
        }
    }
}
