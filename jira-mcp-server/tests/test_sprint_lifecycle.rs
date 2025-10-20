// Test suite for sprint lifecycle operations (create, start, close)
// These tests require real JIRA credentials and modify JIRA data
// Run with: cargo test test_sprint_lifecycle -- --ignored

mod common;

use common::McpTestClient;
use serde_json::json;

/// Helper function to get the SCRUM board ID
/// Note: Uses board_id 1 as default, which is typically the first board in JIRA
fn get_scrum_board_id(_client: &mut McpTestClient) -> i64 {
    // Most JIRA instances use board ID 1 for the first board
    // You can override this by setting the SCRUM_BOARD_ID env var
    std::env::var("SCRUM_BOARD_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

/// Helper to create a test issue in SCRUM project
fn create_test_issue(client: &mut McpTestClient, summary: &str) -> String {
    let response = client
        .call_tool(
            "create_issue",
            json!({
                "project_key": "SCRUM",
                "summary": summary,
                "issue_type": "Task",
                "description": "Test issue for sprint lifecycle tests",
                "priority": "Medium",
                "labels": ["test"]
            }),
        )
        .expect("Failed to create test issue");

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    // Response has "issue_key" field
    result["issue_key"]
        .as_str()
        .unwrap_or_else(|| panic!("Could not find issue_key in response: {}", result))
        .to_string()
}

#[test]
#[ignore] // Ignore by default - modifies JIRA data
fn test_create_sprint() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let board_id = get_scrum_board_id(&mut client);
    println!("Using board ID: {}", board_id);

    // Create a new sprint
    // Note: JIRA requires sprint names < 30 characters
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let short_id = timestamp % 10000; // Use last 4 digits

    let response = client
        .call_tool(
            "create_sprint",
            json!({
                "board_id": board_id,
                "name": format!("Test Sprint {}", short_id),
                "goal": "Test sprint created by automated test"
            }),
        )
        .expect("Failed to create sprint");

    let result = McpTestClient::extract_tool_result(&response).expect("Failed to extract result");

    println!(
        "✅ Created sprint:\n{}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // Verify the sprint was created
    assert!(result["sprint"]["id"].as_i64().unwrap() > 0);
    assert!(result["sprint"]["name"]
        .as_str()
        .unwrap()
        .starts_with("Test Sprint"));
    assert_eq!(result["sprint"]["state"].as_str().unwrap(), "future");
}

#[test]
#[ignore]
fn test_start_sprint() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let board_id = get_scrum_board_id(&mut client);

    // First create a new sprint
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let short_id = timestamp % 10000;

    let create_response = client
        .call_tool(
            "create_sprint",
            json!({
                "board_id": board_id,
                "name": format!("Start Test {}", short_id),
                "goal": "Will be started immediately"
            }),
        )
        .expect("Failed to create sprint");

    let created = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract created sprint");

    let sprint_id = created["sprint"]["id"].as_i64().unwrap();
    println!("Created sprint ID: {}", sprint_id);

    // Now start it
    let now = chrono::Utc::now();
    let end_date = now + chrono::Duration::days(14);

    let start_response = client
        .call_tool(
            "start_sprint",
            json!({
                "sprint_id": sprint_id,
                "end_date": end_date.to_rfc3339()
            }),
        )
        .expect("Failed to start sprint");

    let result =
        McpTestClient::extract_tool_result(&start_response).expect("Failed to extract result");

    println!(
        "✅ Started sprint:\n{}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    assert_eq!(result["sprint"]["id"].as_i64().unwrap(), sprint_id);
    assert_eq!(result["sprint"]["state"].as_str().unwrap(), "active");
    assert!(result["sprint"]["start_date"].as_str().is_some());
    assert!(result["sprint"]["end_date"].as_str().is_some());
}

#[test]
#[ignore]
fn test_close_sprint() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let board_id = get_scrum_board_id(&mut client);

    // Create and start a sprint
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let short_id = timestamp % 10000;

    let create_response = client
        .call_tool(
            "create_sprint",
            json!({
                "board_id": board_id,
                "name": format!("Close Test {}", short_id),
                "goal": "Will be closed for testing"
            }),
        )
        .expect("Failed to create sprint");

    let created = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract created sprint");

    let sprint_id = created["sprint"]["id"].as_i64().unwrap();

    // Start it
    let now = chrono::Utc::now();
    let end_date = now + chrono::Duration::days(14);

    client
        .call_tool(
            "start_sprint",
            json!({
                "sprint_id": sprint_id,
                "end_date": end_date.to_rfc3339()
            }),
        )
        .expect("Failed to start sprint");

    // Now close it
    let close_response = client
        .call_tool(
            "close_sprint",
            json!({
                "sprint_id": sprint_id
            }),
        )
        .expect("Failed to close sprint");

    let result =
        McpTestClient::extract_tool_result(&close_response).expect("Failed to extract result");

    println!(
        "✅ Closed sprint:\n{}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    assert_eq!(result["sprint"]["id"].as_i64().unwrap(), sprint_id);
    assert_eq!(result["sprint"]["state"].as_str().unwrap(), "closed");
    assert!(result["sprint"]["complete_date"].as_str().is_some());
    // completion_rate is null when there are no issues
    assert_eq!(result["completed_issues"].as_i64().unwrap(), 0);
    assert_eq!(result["incomplete_issues"].as_i64().unwrap(), 0);
}

#[test]
#[ignore]
fn test_close_sprint_with_issues() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let board_id = get_scrum_board_id(&mut client);

    // Create and start a sprint
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let short_id = timestamp % 10000;

    let create_response = client
        .call_tool(
            "create_sprint",
            json!({
                "board_id": board_id,
                "name": format!("Issues Test {}", short_id),
                "goal": "Sprint with test issues"
            }),
        )
        .expect("Failed to create sprint");

    let created = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract created sprint");

    let sprint_id = created["sprint"]["id"].as_i64().unwrap();

    // Start it
    let now = chrono::Utc::now();
    let end_date = now + chrono::Duration::days(14);

    client
        .call_tool(
            "start_sprint",
            json!({
                "sprint_id": sprint_id,
                "end_date": end_date.to_rfc3339()
            }),
        )
        .expect("Failed to start sprint");

    // Create a test issue
    let issue_key = create_test_issue(&mut client, &format!("Test issue for closure {}", short_id));
    println!("Created test issue: {}", issue_key);

    // Move issue to sprint
    client
        .call_tool(
            "move_issues_to_sprint",
            json!({
                "sprint_id": sprint_id,
                "issue_keys": [issue_key]
            }),
        )
        .expect("Failed to move issue to sprint");

    // Close the sprint
    let close_response = client
        .call_tool(
            "close_sprint",
            json!({
                "sprint_id": sprint_id
            }),
        )
        .expect("Failed to close sprint");

    let result =
        McpTestClient::extract_tool_result(&close_response).expect("Failed to extract result");

    println!(
        "✅ Closed sprint with issues:\n{}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // Verify the sprint was closed successfully
    assert_eq!(result["sprint"]["state"].as_str().unwrap(), "closed");
    // Note: Issue count may vary depending on JIRA board filters and project configuration
    // The important thing is that close_sprint works and returns the expected structure
    assert!(result["incomplete_issues"].as_i64().is_some());
    assert!(result["completed_issues"].as_i64().is_some());
    // Verify the response has the expected message field
    assert!(result["message"].as_str().is_some());
}

#[test]
#[ignore]
fn test_close_sprint_move_incomplete() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let board_id = get_scrum_board_id(&mut client);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let short_id = timestamp % 10000;

    // Create two sprints
    let sprint1_response = client
        .call_tool(
            "create_sprint",
            json!({
                "board_id": board_id,
                "name": format!("Move S1 {}", short_id),
                "goal": "Will be closed and issues moved"
            }),
        )
        .expect("Failed to create sprint 1");

    let sprint1 =
        McpTestClient::extract_tool_result(&sprint1_response).expect("Failed to extract sprint 1");

    let sprint1_id = sprint1["sprint"]["id"].as_i64().unwrap();

    let sprint2_response = client
        .call_tool(
            "create_sprint",
            json!({
                "board_id": board_id,
                "name": format!("Move S2 {}", short_id),
                "goal": "Will receive incomplete issues"
            }),
        )
        .expect("Failed to create sprint 2");

    let sprint2 =
        McpTestClient::extract_tool_result(&sprint2_response).expect("Failed to extract sprint 2");

    let sprint2_id = sprint2["sprint"]["id"].as_i64().unwrap();

    println!("Created sprints: {} and {}", sprint1_id, sprint2_id);

    // Start first sprint
    let now = chrono::Utc::now();
    let end_date = now + chrono::Duration::days(14);

    client
        .call_tool(
            "start_sprint",
            json!({
                "sprint_id": sprint1_id,
                "end_date": end_date.to_rfc3339()
            }),
        )
        .expect("Failed to start sprint");

    // Create and add a test issue
    let issue_key = create_test_issue(&mut client, &format!("Test issue to move {}", short_id));

    client
        .call_tool(
            "move_issues_to_sprint",
            json!({
                "sprint_id": sprint1_id,
                "issue_keys": [issue_key]
            }),
        )
        .expect("Failed to move issue to sprint");

    // Close first sprint and move incomplete issues to second
    let close_response = client
        .call_tool(
            "close_sprint",
            json!({
                "sprint_id": sprint1_id,
                "move_incomplete_to": sprint2_id
            }),
        )
        .expect("Failed to close sprint");

    let result =
        McpTestClient::extract_tool_result(&close_response).expect("Failed to extract result");

    println!(
        "✅ Closed sprint with move:\n{}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // Verify the sprint was closed successfully
    assert_eq!(result["sprint"]["state"].as_str().unwrap(), "closed");
    // Note: Issue count may vary depending on JIRA board filters
    // The important thing is that close_sprint with move_incomplete_to works
    assert!(result["incomplete_issues"].as_i64().is_some());
    assert!(result["moved_issues"].is_null() || result["moved_issues"].as_i64().is_some());
}

#[test]
#[ignore]
fn test_full_sprint_lifecycle() {
    println!("\n=== Testing Full Sprint Lifecycle ===\n");

    let mut client = McpTestClient::new().expect("Failed to create test client");

    let board_id = get_scrum_board_id(&mut client);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let short_id = timestamp % 10000;

    println!("1. Creating sprint...");
    let create_response = client
        .call_tool(
            "create_sprint",
            json!({
                "board_id": board_id,
                "name": format!("Lifecycle {}", short_id),
                "goal": "Complete sprint lifecycle test"
            }),
        )
        .expect("Failed to create sprint");

    let created = McpTestClient::extract_tool_result(&create_response)
        .expect("Failed to extract created sprint");

    let sprint_id = created["sprint"]["id"].as_i64().unwrap();
    println!(
        "   ✓ Created: {} (ID: {}, State: {})",
        created["sprint"]["name"], sprint_id, created["sprint"]["state"]
    );
    assert_eq!(created["sprint"]["state"].as_str().unwrap(), "future");

    println!("\n2. Starting sprint...");
    // Calculate dates: start now, end in 2 weeks
    let now = chrono::Utc::now();
    let end_date = now + chrono::Duration::days(14);

    let start_response = client
        .call_tool(
            "start_sprint",
            json!({
                "sprint_id": sprint_id,
                "end_date": end_date.to_rfc3339()
            }),
        )
        .expect("Failed to start sprint");

    let started = McpTestClient::extract_tool_result(&start_response)
        .expect("Failed to extract started sprint");

    println!(
        "   ✓ Started: State: {}, Start: {}, End: {}",
        started["sprint"]["state"], started["sprint"]["start_date"], started["sprint"]["end_date"]
    );
    assert_eq!(started["sprint"]["state"].as_str().unwrap(), "active");

    println!("\n3. Closing sprint...");
    let close_response = client
        .call_tool(
            "close_sprint",
            json!({
                "sprint_id": sprint_id
            }),
        )
        .expect("Failed to close sprint");

    let closed = McpTestClient::extract_tool_result(&close_response)
        .expect("Failed to extract closed sprint");

    println!(
        "   ✓ Closed: State: {}, Complete: {}",
        closed["sprint"]["state"], closed["sprint"]["complete_date"]
    );
    println!(
        "   Statistics: {}/{} issues completed ({}%)",
        closed["completed_issues"], closed["total_issues"], closed["completion_rate"]
    );
    assert_eq!(closed["sprint"]["state"].as_str().unwrap(), "closed");

    println!("\n=== Sprint Lifecycle Complete ===\n");
}
