/// Real JIRA Cloud integration tests
/// Tests against actual JIRA instance with SCRUM project
mod common;

use common::McpTestClient;
use serde_json::json;

#[test]
fn test_get_issue_link_types_with_real_jira() {
    // Test get_issue_link_types with real JIRA
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool("get_issue_link_types", json!({}))
        .expect("Failed to get issue link types");

    println!(
        "✅ get_issue_link_types response:\n{}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Verify response structure
    assert!(response.is_object(), "Response should be an object");
}

#[test]
fn test_search_scrum_project_issues() {
    // Search for issues in SCRUM project to get real issue keys for testing
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": "SCRUM",
                "limit": 5
            }),
        )
        .expect("Failed to search issues");

    println!(
        "✅ Found SCRUM project issues:\n{}",
        serde_json::to_string_pretty(&response).unwrap()
    );
}

#[test]
fn test_list_sprints_for_scrum_board() {
    // Test listing sprints - first we need to find a board
    // Most JIRA Cloud instances have board ID 1 for the first board
    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Try board ID 1 (common default)
    let response = client.call_tool(
        "list_sprints",
        json!({
            "board_id": 1
        }),
    );

    match response {
        Ok(result) => {
            println!(
                "✅ list_sprints for board 1:\n{}",
                serde_json::to_string_pretty(&result).unwrap()
            );
        }
        Err(e) => {
            println!(
                "⚠️  Board 1 not found ({}), this is expected if board IDs are different",
                e
            );
            println!(
                "💡 To test sprint management, you need to provide a valid board_id from your JIRA"
            );
        }
    }
}

#[test]
fn test_get_issue_details_scrum_project() {
    // Test getting issue details from SCRUM project
    // First search for an issue
    let mut client = McpTestClient::new().expect("Failed to create test client");

    let search_response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": "SCRUM",
                "limit": 1
            }),
        )
        .expect("Failed to search for SCRUM issues");

    println!(
        "Search response: {}",
        serde_json::to_string_pretty(&search_response).unwrap()
    );

    // Try to extract an issue key from the response
    // MCP response structure: result.structuredContent.search_result.issues
    if let Some(result) = search_response.get("result") {
        let issues = result
            .get("structuredContent")
            .and_then(|sc| sc.get("search_result"))
            .and_then(|sr| sr.get("issues"))
            .and_then(|i| i.as_array());

        if let Some(issues_array) = issues {
            if let Some(first_issue) = issues_array.first() {
                if let Some(issue_key) = first_issue.get("key").and_then(|k| k.as_str()) {
                    println!("Found issue: {}", issue_key);

                    // Now get full details
                    let details_response = client
                        .call_tool(
                            "get_issue_details",
                            json!({
                                "issue_key": issue_key,
                                "include_comments": true
                            }),
                        )
                        .expect("Failed to get issue details");

                    println!(
                        "✅ Issue details for {}:\n{}",
                        issue_key,
                        serde_json::to_string_pretty(&details_response).unwrap()
                    );
                } else {
                    println!("⚠️  Could not extract issue key from search results");
                }
            } else {
                println!("⚠️  No issues found in SCRUM project");
            }
        } else {
            println!("⚠️  Could not parse issues array from response");
        }
    }
}

#[test]
#[ignore] // Ignore by default to avoid modifying JIRA data
fn test_link_issues_with_real_issues() {
    // This test is ignored by default because it modifies JIRA data
    // Run with: cargo test test_link_issues_with_real_issues -- --ignored

    let mut client = McpTestClient::new().expect("Failed to create test client");

    // First, get two issues from SCRUM project
    let search_response = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": "SCRUM",
                "limit": 2
            }),
        )
        .expect("Failed to search for SCRUM issues");

    println!(
        "Search results: {}",
        serde_json::to_string_pretty(&search_response).unwrap()
    );

    // Extract two issue keys
    if let Some(result) = search_response.get("result") {
        let issues = result
            .get("structuredContent")
            .and_then(|sc| sc.get("search_result"))
            .and_then(|sr| sr.get("issues"))
            .and_then(|i| i.as_array());

        if let Some(issues_array) = issues {
            if issues_array.len() >= 2 {
                let issue1_key = issues_array[0].get("key").and_then(|k| k.as_str()).unwrap();
                let issue2_key = issues_array[1].get("key").and_then(|k| k.as_str()).unwrap();

                println!("Linking {} to {}", issue1_key, issue2_key);

                // Create a link
                let link_response = client
                    .call_tool(
                        "link_issues",
                        json!({
                            "inward_issue_key": issue1_key,
                            "outward_issue_key": issue2_key,
                            "link_type": "Relates",
                            "comment": "Test link created by integration test"
                        }),
                    )
                    .expect("Failed to link issues");

                println!(
                    "✅ Link created:\n{}",
                    serde_json::to_string_pretty(&link_response).unwrap()
                );
            } else {
                println!("⚠️  Need at least 2 issues in SCRUM project to test linking");
            }
        }
    }
}
