/// Test rate limit handling with bulk operations
///
/// This test creates 100 issues to verify that retry logic handles rate limits properly
mod common;

use common::{test_project_key, McpTestClient};
use serde_json::json;

#[test]
#[serial_test::serial]
fn test_bulk_create_100_issues_with_retry() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_create_issues with 100 items ===");
    println!("This test verifies retry logic handles rate limits properly");

    // Create 100 issues
    let issues: Vec<_> = (1..=100)
        .map(|i| {
            json!({
                "summary": format!("Rate limit test issue {}", i),
                "issue_type": "Task"
            })
        })
        .collect();

    let params = json!({
        "project_key": project_key,
        "issues": issues,
        "max_concurrent": 10,  // Higher concurrency to potentially trigger rate limits
        "max_retries": 5,      // Allow more retries for rate limits
        "initial_retry_delay_ms": 1000  // 1 second initial delay
    });

    let start = std::time::Instant::now();

    let response = client
        .call_tool("bulk_create_issues", params)
        .expect("Failed to call bulk_create_issues");

    let elapsed = start.elapsed();

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    // Check results
    let success_count = result["success_count"].as_u64().unwrap();
    let failure_count = result["failure_count"].as_u64().unwrap();
    let server_time = result["execution_time_ms"].as_u64().unwrap();

    println!("\nBulk create 100 issues completed!");
    println!("   Success: {}", success_count);
    println!("   Failures: {}", failure_count);
    println!("   Client elapsed time: {:?}", elapsed);
    println!("   Server execution time: {}ms", server_time);
    println!("   Average per issue: {}ms", server_time / 100);

    // We expect high success rate even with rate limits due to retries
    assert!(
        success_count >= 95,
        "Expected at least 95% success rate, got {}/100",
        success_count
    );

    // Calculate time savings vs sequential
    let expected_sequential_time = 100 * 600; // ~60 seconds
    let time_saved_pct = (1.0 - (server_time as f64 / expected_sequential_time as f64)) * 100.0;

    println!(
        "   Expected sequential time: ~{}ms",
        expected_sequential_time
    );
    println!("   Time saved: ~{:.1}%", time_saved_pct);

    println!("\nTest passed - retry logic handled rate limits successfully!");
}

#[test]
#[serial_test::serial]
fn test_bulk_assign_50_issues_with_retry() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n=== Testing bulk_assign_issues with 50 items ===");

    // First, create 50 issues
    let issues: Vec<_> = (1..=50)
        .map(|i| {
            json!({
                "summary": format!("Assign test issue {}", i),
                "issue_type": "Task"
            })
        })
        .collect();

    let create_params = json!({
        "project_key": project_key,
        "issues": issues,
        "max_concurrent": 10
    });

    let create_response = client
        .call_tool("bulk_create_issues", create_params)
        .expect("Failed to create issues");

    let create_result = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract create result");

    let issue_keys: Vec<String> = create_result["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| {
            if r["success"].as_bool().unwrap() {
                Some(r["issue"]["issue_key"].as_str().unwrap().to_string())
            } else {
                None
            }
        })
        .collect();

    println!("Created {} issues", issue_keys.len());

    // Now bulk assign them
    let assign_params = json!({
        "issue_keys": issue_keys,
        "assignee": "me",
        "max_concurrent": 10,
        "max_retries": 5
    });

    let start = std::time::Instant::now();

    let assign_response = client
        .call_tool("bulk_assign_issues", assign_params)
        .expect("Failed to call bulk_assign_issues");

    let elapsed = start.elapsed();

    let result =
        McpTestClient::extract_tool_result(&assign_response).expect("Failed to extract result");

    println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

    let success_count = result["success_count"].as_u64().unwrap();
    let server_time = result["execution_time_ms"].as_u64().unwrap();

    println!("\nBulk assign completed!");
    println!("   Success: {}", success_count);
    println!("   Client elapsed time: {:?}", elapsed);
    println!("   Server execution time: {}ms", server_time);

    // We expect high success rate
    assert!(
        success_count >= issue_keys.len() as u64 * 9 / 10,
        "Expected at least 90% success rate"
    );

    println!("\nTest passed!");
}
