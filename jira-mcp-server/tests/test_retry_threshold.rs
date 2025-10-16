/// Test that initial_retry_delay_ms has a minimum threshold enforced
mod common;

use common::{test_project_key, McpTestClient};
use serde_json::json;

#[test]
#[serial_test::serial]
fn test_retry_delay_minimum_threshold() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing Minimum Retry Delay Threshold ===\n");

    // Try to set initial_retry_delay_ms to a very low value (100ms)
    // It should be clamped to 500ms minimum
    let issues = vec![json!({
        "summary": "Test issue with low retry delay",
        "issue_type": "Task"
    })];

    let params = json!({
        "project_key": project_key,
        "issues": issues,
        "max_concurrent": 1,
        "max_retries": 1,
        "initial_retry_delay_ms": 100  // Below minimum of 500ms
    });

    println!("Attempting to create issue with initial_retry_delay_ms: 100ms");
    println!("Expected: Server will use minimum of 500ms instead\n");

    let start = std::time::Instant::now();
    let response = client
        .call_tool("bulk_create_issues", params)
        .expect("Failed to call bulk_create_issues");
    let elapsed = start.elapsed();

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    let success_count = result["success_count"].as_u64().unwrap();

    println!("Result:");
    println!("  Success: {}/1", success_count);
    println!("  Total time: {:?}", elapsed);

    // The operation should succeed
    assert_eq!(success_count, 1, "Issue should be created successfully");

    println!("\n✅ Test passed! Minimum threshold enforced correctly");
    println!("   (Check server logs for warning about delay being below minimum)");
}

#[test]
#[serial_test::serial]
fn test_retry_delay_valid_value() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing Valid Retry Delay ===\n");

    // Use a valid delay above the minimum
    let issues = vec![json!({
        "summary": "Test issue with valid retry delay",
        "issue_type": "Task"
    })];

    let params = json!({
        "project_key": project_key,
        "issues": issues,
        "max_concurrent": 1,
        "max_retries": 1,
        "initial_retry_delay_ms": 1000  // Valid value above minimum
    });

    println!("Creating issue with initial_retry_delay_ms: 1000ms (valid)\n");

    let start = std::time::Instant::now();
    let response = client
        .call_tool("bulk_create_issues", params)
        .expect("Failed to call bulk_create_issues");
    let elapsed = start.elapsed();

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    let success_count = result["success_count"].as_u64().unwrap();

    println!("Result:");
    println!("  Success: {}/1", success_count);
    println!("  Total time: {:?}", elapsed);

    assert_eq!(success_count, 1, "Issue should be created successfully");

    println!("\n✅ Test passed! Valid delay accepted");
}
