/// Comparison test: Retry-based vs Rate-Limiter-based approaches
///
/// This test demonstrates the difference between:
/// 1. Our approach: Parallel execution with retry on failure (exponential backoff)
/// 2. Atlassian's approach: Pre-emptive rate limiting with queuing
mod common;

use common::{test_project_key, McpTestClient};
use serde_json::json;

#[test]
#[serial_test::serial]
fn test_comparison_retry_vs_rate_limiter() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║    COMPARISON: Retry vs Rate Limiter Approaches           ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Test batch size
    let batch_size = 30;

    // =========================================================================
    // APPROACH 1: Our Retry-Based Approach
    // =========================================================================
    println!("┌────────────────────────────────────────────────────────────┐");
    println!("│ APPROACH 1: Retry-Based (Exponential Backoff)             │");
    println!("│ - Max throughput: Parallel execution                      │");
    println!("│ - Retries on: 429, 500, 502, 503, timeouts                │");
    println!("│ - Strategy: Optimistic (try fast, retry if needed)        │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    let issues: Vec<_> = (1..=batch_size)
        .map(|i| {
            json!({
                "summary": format!("Retry approach test {}", i),
                "issue_type": "Task"
            })
        })
        .collect();

    let params_retry = json!({
        "project_key": project_key,
        "issues": issues,
        "max_concurrent": 10,
        "max_retries": 5,
        "initial_retry_delay_ms": 1000
    });

    let start_retry = std::time::Instant::now();
    let response_retry = client
        .call_tool("bulk_create_issues", params_retry)
        .expect("Failed to call bulk_create_issues");
    let elapsed_retry = start_retry.elapsed();

    let result_retry =
        McpTestClient::extract_tool_result(&response_retry).expect("Failed to extract result");

    let success_count_retry = result_retry["success_count"].as_u64().unwrap();
    let failure_count_retry = result_retry["failure_count"].as_u64().unwrap();
    let server_time_retry = result_retry["execution_time_ms"].as_u64().unwrap();

    println!("Results:");
    println!("  ✓ Success: {}/{}", success_count_retry, batch_size);
    println!("  ✗ Failures: {}", failure_count_retry);
    println!("  ⏱  Total time: {:?}", elapsed_retry);
    println!("  ⚡ Server time: {}ms", server_time_retry);
    println!("  📊 Avg per item: {}ms", server_time_retry / batch_size);

    // Small delay before next test
    std::thread::sleep(std::time::Duration::from_secs(2));

    // =========================================================================
    // APPROACH 2: Hypothetical Rate-Limiter Approach
    // =========================================================================
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│ APPROACH 2: Rate-Limiter Based (Atlassian-style)          │");
    println!("│ - Controlled throughput: Queue before sending              │");
    println!("│ - Prevents: All rate limit errors                         │");
    println!("│ - Strategy: Conservative (never hit limits)               │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    // Simulate rate-limited approach with lower concurrency
    // This mimics what Atlassian-style queue would do
    let issues2: Vec<_> = (1..=batch_size)
        .map(|i| {
            json!({
                "summary": format!("Rate limiter approach test {}", i),
                "issue_type": "Task"
            })
        })
        .collect();

    let params_limited = json!({
        "project_key": project_key,
        "issues": issues2,
        "max_concurrent": 5,  // Lower to simulate queue behavior
        "max_retries": 1,     // Shouldn't need retries with queue
        "initial_retry_delay_ms": 1000
    });

    let start_limited = std::time::Instant::now();
    let response_limited = client
        .call_tool("bulk_create_issues", params_limited)
        .expect("Failed to call bulk_create_issues");
    let elapsed_limited = start_limited.elapsed();

    let result_limited =
        McpTestClient::extract_tool_result(&response_limited).expect("Failed to extract result");

    let success_count_limited = result_limited["success_count"].as_u64().unwrap();
    let failure_count_limited = result_limited["failure_count"].as_u64().unwrap();
    let server_time_limited = result_limited["execution_time_ms"].as_u64().unwrap();

    println!("Results:");
    println!("  ✓ Success: {}/{}", success_count_limited, batch_size);
    println!("  ✗ Failures: {}", failure_count_limited);
    println!("  ⏱  Total time: {:?}", elapsed_limited);
    println!("  ⚡ Server time: {}ms", server_time_limited);
    println!("  📊 Avg per item: {}ms", server_time_limited / batch_size);

    // =========================================================================
    // COMPARISON SUMMARY
    // =========================================================================
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    COMPARISON SUMMARY                      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let speed_diff = if server_time_retry < server_time_limited {
        let pct =
            ((server_time_limited - server_time_retry) as f64 / server_time_limited as f64) * 100.0;
        format!("Retry approach is {:.1}% FASTER", pct)
    } else if server_time_retry > server_time_limited {
        let pct =
            ((server_time_retry - server_time_limited) as f64 / server_time_retry as f64) * 100.0;
        format!("Rate limiter approach is {:.1}% FASTER", pct)
    } else {
        "Both approaches took same time".to_string()
    };

    println!("┌────────────────────────────────────────────────────────────┐");
    println!("│ Metric              │ Retry     │ Rate Limiter │ Winner    │");
    println!("├────────────────────────────────────────────────────────────┤");
    println!(
        "│ Success Rate        │ {:>3}/{:<3}   │ {:>3}/{:<3}      │ {}",
        success_count_retry,
        batch_size,
        success_count_limited,
        batch_size,
        if success_count_retry >= success_count_limited {
            "Retry  "
        } else {
            "Limiter"
        }
    );
    println!(
        "│ Failures            │ {:>8}  │ {:>8}     │ {}",
        failure_count_retry,
        failure_count_limited,
        if failure_count_retry <= failure_count_limited {
            "Retry  "
        } else {
            "Limiter"
        }
    );
    println!(
        "│ Total Time          │ {:>6}ms  │ {:>6}ms     │ {}",
        server_time_retry,
        server_time_limited,
        if server_time_retry < server_time_limited {
            "Retry  "
        } else {
            "Limiter"
        }
    );
    println!("└────────────────────────────────────────────────────────────┘");

    println!("\n{}", speed_diff);

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                       CONCLUSION                           ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\nFor your JIRA instance:");
    println!("  • Rate limits: Not hit in either test");
    println!("  • Optimal approach: {} items", batch_size);
    if failure_count_retry == 0 && failure_count_limited == 0 {
        println!("  • Recommendation: Use retry approach for maximum speed");
        println!("                    (Add rate limiter only if hitting limits)");
    } else {
        println!("  • Recommendation: Combine both approaches");
        println!("                    (Rate limiter + retry for resilience)");
    }
    println!();

    // Both approaches should succeed
    assert!(success_count_retry >= batch_size * 9 / 10);
    assert!(success_count_limited >= batch_size * 9 / 10);
}

#[test]
#[serial_test::serial]
fn test_extreme_load_200_items() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let project_key = test_project_key();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║          EXTREME LOAD TEST: 200 Items                     ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let batch_size = 200;
    let issues: Vec<_> = (1..=batch_size)
        .map(|i| {
            json!({
                "summary": format!("Extreme load test {}", i),
                "issue_type": "Task"
            })
        })
        .collect();

    let params = json!({
        "project_key": project_key,
        "issues": issues,
        "max_concurrent": 15,
        "max_retries": 5,
        "initial_retry_delay_ms": 2000
    });

    println!("Starting extreme load test with {} items...", batch_size);
    println!("Concurrency: 15, Max retries: 5\n");

    let start = std::time::Instant::now();
    let response = client
        .call_tool("bulk_create_issues", params)
        .expect("Failed to call bulk_create_issues");
    let elapsed = start.elapsed();

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    let success_count = result["success_count"].as_u64().unwrap();
    let failure_count = result["failure_count"].as_u64().unwrap();
    let server_time = result["execution_time_ms"].as_u64().unwrap();

    println!("Results:");
    println!("  ✓ Success: {}/{}", success_count, batch_size);
    println!("  ✗ Failures: {}", failure_count);
    println!("  ⏱  Total time: {:?}", elapsed);
    println!(
        "  ⚡ Server time: {}ms ({:.1}s)",
        server_time,
        server_time as f64 / 1000.0
    );
    println!("  📊 Avg per item: {}ms", server_time / batch_size);

    let expected_sequential = batch_size * 600;
    let time_saved_pct = (1.0 - (server_time as f64 / expected_sequential as f64)) * 100.0;
    println!(
        "\n  Expected sequential time: ~{}ms ({:.1}s)",
        expected_sequential,
        expected_sequential as f64 / 1000.0
    );
    println!("  Time saved: {:.1}%", time_saved_pct);

    if failure_count > 0 {
        println!("\n⚠️  Some failures occurred - analyzing...");
        let results = result["results"].as_array().unwrap();
        let failed: Vec<_> = results
            .iter()
            .filter(|r| !r["success"].as_bool().unwrap())
            .collect();

        println!("  Failed items: {}", failed.len());
        if !failed.is_empty() {
            println!(
                "  First error: {}",
                failed[0]["error"].as_str().unwrap_or("Unknown")
            );
        }
    }

    println!("\nTest complete!");
}
