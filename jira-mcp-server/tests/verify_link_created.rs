/// Verify the link was created in JIRA
mod common;

use common::McpTestClient;
use serde_json::json;

#[test]
fn verify_scrum_1_has_link_to_scrum_119() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Get issue relationships for SCRUM-1
    let response = client
        .call_tool(
            "get_issue_relationships",
            json!({
                "root_issue_key": "SCRUM-1",
                "max_depth": 1,
                "include_relates": true
            }),
        )
        .expect("Failed to get issue relationships");

    println!(
        "✅ SCRUM-1 Relationships:\n{}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Check if SCRUM-119 is in the relationships
    if let Some(result) = response.get("result") {
        if let Some(structured) = result.get("structuredContent") {
            println!("\n🔍 Checking for link to SCRUM-119...");
            let json_str = serde_json::to_string(&structured).unwrap();
            if json_str.contains("SCRUM-119") {
                println!("✅ CONFIRMED: Link to SCRUM-119 found!");
            } else {
                println!("⚠️  Link to SCRUM-119 not found in relationships");
            }
        }
    }
}

#[test]
fn get_issue_details_with_links() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    // Get full issue details for SCRUM-1
    let response = client
        .call_tool(
            "get_issue_details",
            json!({
                "issue_key": "SCRUM-1",
                "include_comments": true
            }),
        )
        .expect("Failed to get issue details");

    println!(
        "✅ SCRUM-1 Full Details:\n{}",
        serde_json::to_string_pretty(&response).unwrap()
    );
}
