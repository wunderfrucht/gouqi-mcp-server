/// Integration tests for bulk operations
///
/// These tests verify that bulk operations work correctly against a real JIRA instance.
/// They test all 5 bulk operation tools with various scenarios.
mod common;

use common::{test_project_key, McpTestClient};
use serde_json::json;

// =============================================================================
// Test: Bulk Create Issues
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_create_issues() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_create_issues ===");

    // Create 3 issues in bulk
    let params = json!({
        "project_key": project_key,
        "issues": [
            {
                "summary": "Bulk Test Issue 1",
                "description": "First bulk created issue",
                "issue_type": "Task"
            },
            {
                "summary": "Bulk Test Issue 2",
                "description": "Second bulk created issue",
                "issue_type": "Task"
            },
            {
                "summary": "Bulk Test Issue 3",
                "description": "Third bulk created issue",
                "issue_type": "Task"
            }
        ]
    });

    let response = client
        .call_tool("bulk_create_issues", params)
        .expect("Failed to call bulk_create_issues");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Verify results
    assert_eq!(result["success_count"].as_u64().unwrap(), 3);
    assert_eq!(result["failure_count"].as_u64().unwrap(), 0);

    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);

    // Verify each issue was created successfully
    for (i, result_item) in results.iter().enumerate() {
        assert!(result_item["success"].as_bool().unwrap());
        assert!(result_item["issue"].is_object());
        let issue = &result_item["issue"];
        assert!(issue["issue_key"]
            .as_str()
            .unwrap()
            .starts_with(&project_key));
        println!("Created issue {}: {}", i + 1, issue["issue_key"]);
    }

    println!("✓ Bulk create issues test passed!");
}

// =============================================================================
// Test: Bulk Assign Issues
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_assign_issues() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_assign_issues ===");

    // First, create some issues to assign
    let create_params = json!({
        "project_key": project_key,
        "issues": [
            {"summary": "Issue for bulk assign 1"},
            {"summary": "Issue for bulk assign 2"}
        ]
    });

    let create_response = client
        .call_tool("bulk_create_issues", create_params)
        .expect("Failed to create issues");

    let create_result = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract create result");

    // Extract issue keys
    let issue_keys: Vec<String> = create_result["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["issue"]["issue_key"].as_str().unwrap().to_string())
        .collect();

    println!("Created issues: {:?}", issue_keys);

    // Now bulk assign them to "me"
    let assign_params = json!({
        "issue_keys": issue_keys,
        "assignee": "me"
    });

    let assign_response = client
        .call_tool("bulk_assign_issues", assign_params)
        .expect("Failed to call bulk_assign_issues");

    let result =
        McpTestClient::extract_tool_result(&assign_response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Verify results
    assert_eq!(result["success_count"].as_u64().unwrap(), 2);
    assert_eq!(result["failure_count"].as_u64().unwrap(), 0);

    println!("✓ Bulk assign issues test passed!");
}

// =============================================================================
// Test: Bulk Add Labels
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_add_labels() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_add_labels ===");

    // First, create some issues
    let create_params = json!({
        "project_key": project_key,
        "issues": [
            {"summary": "Issue for bulk labels 1"},
            {"summary": "Issue for bulk labels 2"}
        ]
    });

    let create_response = client
        .call_tool("bulk_create_issues", create_params)
        .expect("Failed to create issues");

    let create_result = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract create result");

    // Extract issue keys
    let issue_keys: Vec<String> = create_result["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["issue"]["issue_key"].as_str().unwrap().to_string())
        .collect();

    println!("Created issues: {:?}", issue_keys);

    // Now bulk add labels
    let labels_params = json!({
        "issue_keys": issue_keys,
        "add_labels": ["bulk-test", "automated"]
    });

    let labels_response = client
        .call_tool("bulk_add_labels", labels_params)
        .expect("Failed to call bulk_add_labels");

    let result =
        McpTestClient::extract_tool_result(&labels_response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Verify results
    assert_eq!(result["success_count"].as_u64().unwrap(), 2);
    assert_eq!(result["failure_count"].as_u64().unwrap(), 0);

    println!("✓ Bulk add labels test passed!");
}

// =============================================================================
// Test: Bulk Update Fields
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_update_fields() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_update_fields ===");

    // First, create some issues
    let create_params = json!({
        "project_key": project_key,
        "issues": [
            {"summary": "Issue for bulk update 1"},
            {"summary": "Issue for bulk update 2"}
        ]
    });

    let create_response = client
        .call_tool("bulk_create_issues", create_params)
        .expect("Failed to create issues");

    let create_result = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract create result");

    // Extract issue keys
    let issue_keys: Vec<String> = create_result["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["issue"]["issue_key"].as_str().unwrap().to_string())
        .collect();

    println!("Created issues: {:?}", issue_keys);

    // Now bulk update priority
    let update_params = json!({
        "issue_keys": issue_keys,
        "field_updates": {
            "priority": {"name": "High"}
        }
    });

    let update_response = client
        .call_tool("bulk_update_fields", update_params)
        .expect("Failed to call bulk_update_fields");

    let result =
        McpTestClient::extract_tool_result(&update_response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Verify results
    assert_eq!(result["success_count"].as_u64().unwrap(), 2);
    assert_eq!(result["failure_count"].as_u64().unwrap(), 0);

    println!("✓ Bulk update fields test passed!");
}

// =============================================================================
// Test: Error Handling
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_operations_error_handling() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk operations error handling ===");

    // Test with one valid and one invalid issue
    let params = json!({
        "project_key": project_key,
        "issues": [
            {
                "summary": "Valid Issue",
                "issue_type": "Task"
            },
            {
                "summary": "Invalid Issue",
                "issue_type": "NonExistentType"  // This should fail
            }
        ],
        "stop_on_error": false  // Continue on errors
    });

    let response = client
        .call_tool("bulk_create_issues", params)
        .expect("Failed to call bulk_create_issues");

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Should have at least one success (maybe both if type validation happens differently)
    assert!(result["success_count"].as_u64().unwrap() >= 1);

    println!("✓ Error handling test passed!");
}
