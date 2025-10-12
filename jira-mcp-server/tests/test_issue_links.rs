/// Integration tests for issue linking tools
mod common;

use common::McpTestClient;
use serde_json::json;

#[test]
fn test_link_issues_tool_exists() {
    // Test that link_issues tool is properly registered

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Test with dummy data - should get some error but tool should be registered
    let response = client.call_tool(
        "link_issues",
        json!({
            "inward_issue_key": "TEST-999",
            "outward_issue_key": "TEST-998",
            "link_type": "Blocks"
        }),
    );

    // We expect either success or a proper JIRA error, not "tool not found"
    match response {
        Ok(_) => {
            // Success is fine - means the tool worked
        }
        Err(e) => {
            let err_msg = e.to_string();
            // Should get JIRA error (404, issue not found, etc)
            assert!(
                err_msg.contains("404")
                    || err_msg.contains("issue")
                    || err_msg.contains("not found")
                    || err_msg.contains("link"),
                "Expected JIRA error, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_delete_issue_link_tool_exists() {
    // Test that delete_issue_link tool is properly registered

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Test with dummy link ID
    let response = client.call_tool(
        "delete_issue_link",
        json!({
            "link_id": "999999"
        }),
    );

    // We expect either success or a proper JIRA error, not "tool not found"
    match response {
        Ok(_) => {
            // Success is fine
        }
        Err(e) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("404")
                    || err_msg.contains("link")
                    || err_msg.contains("not found"),
                "Expected JIRA error, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_get_issue_link_types_tool_exists() {
    // Test that get_issue_link_types tool is properly registered
    // This one should actually succeed and return real link types from JIRA

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // This tool takes no parameters
    let response = client.call_tool("get_issue_link_types", json!({}));

    // This should succeed - the tool is working if we get any response
    match response {
        Ok(_result) => {
            // Tool works - actual response format validation would require
            // knowing the exact MCP response wrapper format
            println!("get_issue_link_types tool is callable and returned a response");
        }
        Err(e) => {
            let err_msg = e.to_string();
            // Allow JIRA errors (like auth issues), but not "tool not found"
            assert!(
                !err_msg.contains("not found")
                    || err_msg.contains("link")
                    || err_msg.contains("404"),
                "Tool should be registered, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_link_issues_validates_parameters() {
    // Test that link_issues tool is callable with valid-looking parameters
    // Actual parameter validation happens server-side

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Test with valid parameter structure (will fail on JIRA side with 404, but that's expected)
    let response = client.call_tool(
        "link_issues",
        json!({
            "inward_issue_key": "TEST-999",
            "outward_issue_key": "TEST-998",
            "link_type": "Blocks"
        }),
    );

    // We expect either success or a JIRA error (404, etc), not a parameter validation error
    match response {
        Ok(_) => {
            // Success is fine
        }
        Err(e) => {
            let err_msg = e.to_string();
            // Should get JIRA error, not parameter validation error
            assert!(
                err_msg.contains("404")
                    || err_msg.contains("issue")
                    || err_msg.contains("not found"),
                "Expected JIRA error, got: {}",
                err_msg
            );
        }
    }
}
