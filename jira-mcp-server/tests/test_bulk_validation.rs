/// Validation tests for bulk operations
///
/// These tests verify that bulk operations properly validate inputs and handle errors.
/// We test various error scenarios to ensure the API is safe and provides good feedback.
mod common;

use common::{test_project_key, McpTestClient};
use serde_json::json;

// =============================================================================
// Test: Invalid Parameters - Missing Required Fields
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_create_missing_project_key() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== Testing bulk_create_issues with missing project_key ===");

    // Try to create without project_key - should fail validation
    let params = json!({
        "issues": [
            {"summary": "Test Issue"}
        ]
    });

    let response = client.call_tool("bulk_create_issues", params);

    // Should fail because project_key is required
    assert!(
        response.is_err() || {
            if let Ok(resp) = response {
                let result = McpTestClient::extract_tool_result(&resp);
                result.is_err() || result.unwrap()["failure_count"].as_u64().unwrap() > 0
            } else {
                true
            }
        }
    );

    println!("✓ Properly rejected missing project_key");
}

#[test]
#[serial_test::serial]
fn test_bulk_transition_missing_transition() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== Testing bulk_transition_issues without transition ===");

    // Try to transition without specifying transition_id or transition_name
    let params = json!({
        "issue_keys": ["SCRUM-1", "SCRUM-2"]
    });

    let response = client.call_tool("bulk_transition_issues", params);

    // Should fail because either transition_id or transition_name is required
    assert!(
        response.is_err() || {
            if let Ok(resp) = response {
                let result = McpTestClient::extract_tool_result(&resp);
                result.is_err()
            } else {
                true
            }
        }
    );

    println!("✓ Properly rejected missing transition parameters");
}

// =============================================================================
// Test: Invalid Field Names in bulk_update_fields
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_update_invalid_field_names() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_update_fields with invalid field names ===");

    // First create a test issue
    let create_params = json!({
        "project_key": project_key,
        "summary": "Test Issue for Invalid Field Update"
    });

    let create_response = client
        .call_tool("create_issue", create_params)
        .expect("Failed to create test issue");

    let create_result =
        McpTestClient::extract_tool_result(&create_response).expect("Failed to extract result");

    let issue_key = create_result["issue_key"].as_str().unwrap();
    println!("Created test issue: {}", issue_key);

    // Try to update with invalid field name
    let update_params = json!({
        "issue_keys": [issue_key],
        "field_updates": {
            "nonexistent_field_12345": "some value"
        }
    });

    let update_response = client
        .call_tool("bulk_update_fields", update_params)
        .expect("Tool call succeeded");

    let result =
        McpTestClient::extract_tool_result(&update_response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Should have failure
    assert_eq!(result["failure_count"].as_u64().unwrap(), 1);

    let results = result["results"].as_array().unwrap();
    assert!(!results[0]["success"].as_bool().unwrap());
    assert!(results[0]["error"].is_string());

    println!("✓ Properly handled invalid field name with error message");
}

// =============================================================================
// Test: Invalid Issue Keys
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_operations_invalid_issue_keys() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== Testing bulk operations with invalid issue keys ===");

    // Try to assign non-existent issues
    let params = json!({
        "issue_keys": ["INVALID-99999", "NOTEXIST-88888"],
        "assignee": "me"
    });

    let response = client
        .call_tool("bulk_assign_issues", params)
        .expect("Tool call succeeded");

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // All should fail since issues don't exist
    assert_eq!(result["success_count"].as_u64().unwrap(), 0);
    assert_eq!(result["failure_count"].as_u64().unwrap(), 2);

    // Check error messages exist
    let results = result["results"].as_array().unwrap();
    for result_item in results {
        assert!(!result_item["success"].as_bool().unwrap());
        assert!(result_item["error"].is_string());
        let error_msg = result_item["error"].as_str().unwrap();
        assert!(
            error_msg.contains("404")
                || error_msg.contains("Not Found")
                || error_msg.contains("not found")
                || error_msg.contains("NotFound")
        );
    }

    println!("✓ Properly handled invalid issue keys with appropriate errors");
}

// =============================================================================
// Test: Invalid Data Types
// =============================================================================

#[test]
#[serial_test::serial]
fn test_bulk_update_wrong_priority_format() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_update_fields with wrong priority format ===");

    // Create a test issue
    let create_params = json!({
        "project_key": project_key,
        "summary": "Test Issue for Wrong Priority Format"
    });

    let create_response = client
        .call_tool("create_issue", create_params)
        .expect("Failed to create test issue");

    let create_result =
        McpTestClient::extract_tool_result(&create_response).expect("Failed to extract result");

    let issue_key = create_result["issue_key"].as_str().unwrap();

    // Try to update priority with wrong format (should be {"name": "High"}, not just "High")
    let update_params = json!({
        "issue_keys": [issue_key],
        "field_updates": {
            "priority": "InvalidPriorityValue"  // Wrong format - should be an object
        }
    });

    let update_response = client
        .call_tool("bulk_update_fields", update_params)
        .expect("Tool call succeeded");

    let result =
        McpTestClient::extract_tool_result(&update_response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Should fail with validation error
    assert_eq!(result["failure_count"].as_u64().unwrap(), 1);

    println!("✓ Properly rejected invalid data type");
}

// =============================================================================
// Test: Performance - Large Batch
// =============================================================================

#[test]
#[serial_test::serial]
fn test_performance_large_batch() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Performance Test: Creating 20 issues ===");

    let issues: Vec<_> = (1..=20)
        .map(|i| {
            json!({
                "summary": format!("Performance test issue {}", i),
                "issue_type": "Task"
            })
        })
        .collect();

    let create_params = json!({
        "project_key": project_key,
        "issues": issues,
        "max_concurrent": 10  // Use higher concurrency
    });

    let start = std::time::Instant::now();

    let response = client
        .call_tool("bulk_create_issues", create_params)
        .expect("Failed to create issues");

    let elapsed = start.elapsed();

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    assert_eq!(result["success_count"].as_u64().unwrap(), 20);

    let server_time = result["execution_time_ms"].as_u64().unwrap();

    println!("\n✅ Created 20 issues successfully");
    println!("   Client elapsed time: {:?}", elapsed);
    println!("   Server execution time: {}ms", server_time);
    println!("   Average per issue: {}ms", server_time / 20);

    // With parallel execution, this should be much faster than 20 sequential calls
    // Sequential would be ~20 issues * 600ms = 12000ms
    // Parallel (10 concurrent) should be ~2 * 600ms = 1200ms
    println!("   Expected sequential time: ~12000ms");
    println!(
        "   Time saved: ~{:.1}%",
        (1.0 - (server_time as f64 / 12000.0)) * 100.0
    );
}
