/// AI Agent Real-World Scenario Tests
///
/// These tests simulate how an AI agent would actually use the JIRA MCP server
/// to accomplish common tasks. Each test represents a realistic workflow.
mod common;

use common::{test_issue_key, test_project_key, McpTestClient};
use serde_json::json;

// =============================================================================
// SCENARIO 1: Daily Standup Preparation
// =============================================================================
// An AI agent prepares daily standup information for a developer

#[test]
fn scenario_daily_standup_prep() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== SCENARIO: Daily Standup Preparation ===");

    // 1. Get my issues that are in progress
    println!("\n1. Checking what I'm currently working on...");
    let in_progress = client
        .call_tool(
            "get_user_issues",
            json!({
                "status_filter": ["In Progress", "in progress", "In Development"]
            }),
        )
        .expect("Failed to get in progress issues");

    let in_progress_result =
        McpTestClient::extract_tool_result(&in_progress).expect("Failed to extract result");
    println!(
        "   Found {} issues in progress",
        in_progress_result["search_result"]["total"]
            .as_u64()
            .unwrap()
    );

    // 2. Check for any blockers or high priority items
    println!("\n2. Checking for blockers and high priority items...");
    let blockers = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": test_project_key(),
                "assigned_to": "me",
                "status_filter": ["Open", "To Do", "Blocked"]
            }),
        )
        .expect("Failed to search for blockers");

    let blockers_result =
        McpTestClient::extract_tool_result(&blockers).expect("Failed to extract result");
    println!(
        "   Found {} high priority/blocked items",
        blockers_result["search_result"]["total"].as_u64().unwrap()
    );

    // 3. Check work logged yesterday
    println!("\n3. Checking work completed yesterday...");
    let yesterday = client
        .call_tool(
            "get_user_issues",
            json!({
                "status_filter": ["Done", "Resolved", "Closed"],
                "updated_after": "1 day ago"
            }),
        )
        .expect("Failed to get completed work");

    let yesterday_result =
        McpTestClient::extract_tool_result(&yesterday).expect("Failed to extract result");
    println!(
        "   Completed {} issues yesterday",
        yesterday_result["search_result"]["total"].as_u64().unwrap()
    );

    println!("\n✓ Standup prep complete!");
}

// =============================================================================
// SCENARIO 2: Bug Triage and Assignment
// =============================================================================
// An AI agent helps triage new bugs and assign them to appropriate developers

#[test]
fn scenario_bug_triage() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== SCENARIO: Bug Triage ===");

    // 1. Find all unassigned bugs
    println!("\n1. Finding unassigned bugs...");
    let unassigned_bugs = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": test_project_key(),
                "issue_types": ["Bug"],
                "status_filter": ["Open", "To Do", "Backlog"],
                "assigned_to": "unassigned"
            }),
        )
        .expect("Failed to search for unassigned bugs");

    let bugs_result =
        McpTestClient::extract_tool_result(&unassigned_bugs).expect("Failed to extract result");
    let bug_count = bugs_result["search_result"]["total"].as_u64().unwrap();
    println!("   Found {} unassigned bugs", bug_count);

    if bug_count > 0 {
        // 2. Get details of the first bug
        let issues = bugs_result["search_result"]["issues"]
            .as_array()
            .expect("Expected issues array");
        if let Some(first_bug) = issues.first() {
            let bug_key = first_bug["key"].as_str().expect("Expected key");

            println!("\n2. Analyzing bug: {}", bug_key);
            let details = client
                .call_tool(
                    "get_issue_details",
                    json!({
                        "issue_key": bug_key,
                        "include_comments": true
                    }),
                )
                .expect("Failed to get issue details");

            let details_result =
                McpTestClient::extract_tool_result(&details).expect("Failed to extract result");
            println!("   Summary: {}", details_result["summary"]);

            // 3. Add a triage comment
            println!("\n3. Adding triage comment...");
            let _comment = client
                .call_tool(
                    "add_comment",
                    json!({
                        "issue_key": bug_key,
                        "comment_body": "🤖 Triaged by AI: This bug needs investigation. Severity appears moderate based on description."
                    }),
                )
                .expect("Failed to add comment");

            println!("   ✓ Comment added");
        }
    }

    println!("\n✓ Bug triage complete!");
}

// =============================================================================
// SCENARIO 3: Sprint Planning Assistant
// =============================================================================
// An AI agent helps prepare for sprint planning by analyzing backlog

#[test]
fn scenario_sprint_planning() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== SCENARIO: Sprint Planning Assistant ===");

    // 1. Get backlog items (stories and tasks not in Done)
    println!("\n1. Analyzing backlog...");
    let backlog = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": test_project_key(),
                "issue_types": ["Story", "Task"],
                "status_filter": ["To Do", "Backlog", "Open"],
                "limit": 20
            }),
        )
        .expect("Failed to get backlog");

    let backlog_result =
        McpTestClient::extract_tool_result(&backlog).expect("Failed to extract result");
    println!(
        "   Found {} backlog items",
        backlog_result["search_result"]["total"].as_u64().unwrap()
    );

    // 2. Check for stories without acceptance criteria
    println!("\n2. Checking for stories missing acceptance criteria...");
    if let Some(issues) = backlog_result["search_result"]["issues"].as_array() {
        let mut missing_ac = 0;
        for issue in issues.iter().take(5) {
            let key = issue["key"].as_str().expect("Expected key");
            let issue_type = issue["issue_type"].as_str().unwrap_or("");

            if issue_type == "Story" {
                // Check if description mentions acceptance criteria
                if let Some(desc) = issue["description"].as_str() {
                    if !desc.to_lowercase().contains("acceptance") && !desc.contains("- [ ]") {
                        missing_ac += 1;
                        println!("   ⚠️  {} may need acceptance criteria", key);
                    }
                }
            }
        }
        println!("   {} stories may need acceptance criteria", missing_ac);
    }

    // 3. Check team capacity - who has bandwidth?
    println!("\n3. Checking team workload...");
    let my_issues = client
        .call_tool(
            "get_user_issues",
            json!({
                "status_filter": ["In Progress", "To Do", "Open"]
            }),
        )
        .expect("Failed to get my issues");

    let my_issues_result =
        McpTestClient::extract_tool_result(&my_issues).expect("Failed to extract result");
    println!(
        "   Current user has {} active issues",
        my_issues_result["search_result"]["total"].as_u64().unwrap()
    );

    println!("\n✓ Sprint planning analysis complete!");
}

// =============================================================================
// SCENARIO 4: Code Review Workflow
// =============================================================================
// An AI agent manages the code review process

#[test]
fn scenario_code_review_workflow() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    println!("\n=== SCENARIO: Code Review Workflow ===");

    // 1. Get issue details
    println!("\n1. Getting issue details for {}...", issue_key);
    let details = client
        .call_tool(
            "get_issue_details",
            json!({
                "issue_key": issue_key
            }),
        )
        .expect("Failed to get issue details");

    let details_result =
        McpTestClient::extract_tool_result(&details).expect("Failed to extract result");
    let current_status = details_result["status"].as_str().expect("Expected status");
    println!("   Current status: {}", current_status);

    // 2. Check available transitions
    println!("\n2. Checking available transitions...");
    let transitions = client
        .call_tool(
            "get_available_transitions",
            json!({
                "issue_key": issue_key
            }),
        )
        .expect("Failed to get transitions");

    let transitions_result =
        McpTestClient::extract_tool_result(&transitions).expect("Failed to extract result");
    if let Some(trans_array) = transitions_result["transitions"].as_array() {
        println!("   Available transitions:");
        for trans in trans_array {
            println!("   - {} (ID: {})", trans["name"], trans["id"]);
        }
    }

    // 3. Add code review comment
    println!("\n3. Adding code review feedback...");
    let _comment = client
        .call_tool(
            "add_comment",
            json!({
                "issue_key": issue_key,
                "comment_body": "🤖 AI Code Review:\n\n✅ Code quality looks good\n✅ Tests are passing\n✅ Documentation updated\n\nReady for merge!"
            }),
        )
        .expect("Failed to add comment");

    println!("   ✓ Review comment added");
    println!("\n✓ Code review workflow complete!");
}

// =============================================================================
// SCENARIO 5: Incident Response
// =============================================================================
// An AI agent helps respond to a production incident

#[test]
fn scenario_incident_response() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== SCENARIO: Incident Response ===");

    // 1. Create incident ticket
    println!("\n1. Creating incident ticket...");
    let create_result = client
        .call_tool(
            "create_issue",
            json!({
                "project_key": test_project_key(),
                "summary": format!("🚨 INCIDENT: Production API latency spike - {}", chrono::Utc::now().format("%Y%m%d_%H%M%S")),
                "description": "# Incident Details\n\nProduction API experiencing high latency (>5s) affecting user transactions.\n\n## Impact\n- Service: Payment API\n- Severity: High\n- Users affected: ~500\n\n## Timeline\n- Detected: Now\n- Investigating: In progress",
                "issue_type": "Task",
                "priority": "Highest",
                "assign_to_me": true
            }),
        )
        .expect("Failed to create incident");

    let create_result_data =
        McpTestClient::extract_tool_result(&create_result).expect("Failed to extract result");
    let incident_key = create_result_data["issue_key"]
        .as_str()
        .expect("Expected issue_key");
    println!("   ✓ Created incident: {}", incident_key);

    // 2. Check for similar past incidents
    println!("\n2. Searching for similar past incidents...");
    let similar = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": test_project_key(),
                "query_text": "API latency",
                "status_filter": ["Done", "Resolved", "Closed"],
                "limit": 5
            }),
        )
        .expect("Failed to search similar incidents");

    let similar_result =
        McpTestClient::extract_tool_result(&similar).expect("Failed to extract result");
    println!(
        "   Found {} similar past incidents",
        similar_result["search_result"]["total"].as_u64().unwrap()
    );

    // 3. Add investigation update
    println!("\n3. Adding investigation update...");
    let _update = client
        .call_tool(
            "add_comment",
            json!({
                "issue_key": incident_key,
                "comment_body": "## Investigation Update\n\n🔍 Checked database connection pool - utilization at 95%\n🔍 Reviewed recent deployments - no changes in last 24h\n🔍 Monitoring logs for error patterns\n\n**Next steps:**\n- Scale up connection pool\n- Review slow query logs"
            }),
        )
        .expect("Failed to add update");

    println!("   ✓ Investigation update added");

    // 4. Track time spent on incident
    println!("\n4. Logging incident response time...");
    let _todo = client
        .call_tool(
            "add_todo",
            json!({
                "issue_key": incident_key,
                "todo_text": "Root cause analysis and post-mortem"
            }),
        )
        .expect("Failed to add todo");

    println!("   ✓ Post-incident todo added");
    println!("\n✓ Incident response workflow complete!");
}

// =============================================================================
// SCENARIO 6: Dependency Analysis
// =============================================================================
// An AI agent analyzes issue dependencies before starting work

#[test]
fn scenario_dependency_analysis() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    println!("\n=== SCENARIO: Dependency Analysis ===");

    // 1. Get issue relationships
    println!("\n1. Analyzing dependencies for {}...", issue_key);
    let relationships = client
        .call_tool(
            "get_issue_relationships",
            json!({
                "root_issue_key": issue_key,
                "max_depth": 2
            }),
        )
        .expect("Failed to get relationships");

    let rel_result =
        McpTestClient::extract_tool_result(&relationships).expect("Failed to extract result");

    if let Some(relationships_array) = rel_result["relationships"].as_array() {
        println!("   Found {} related issues", relationships_array.len());

        // 2. Check if any blockers are still open
        println!("\n2. Checking for blocking issues...");
        let mut open_blockers = 0;
        for rel in relationships_array {
            if let Some(rel_type) = rel["relationship_type"].as_str() {
                if rel_type.to_lowercase().contains("block") {
                    if let Some(status) = rel["status"].as_str() {
                        if !status.eq_ignore_ascii_case("Done")
                            && !status.eq_ignore_ascii_case("Resolved")
                        {
                            open_blockers += 1;
                            println!("   ⚠️  Blocker still open: {} ({})", rel["key"], status);
                        }
                    }
                }
            }
        }

        if open_blockers == 0 {
            println!("   ✅ No open blockers found - safe to proceed!");
        } else {
            println!(
                "   ⚠️  {} blocking issues need resolution first",
                open_blockers
            );
        }
    }

    println!("\n✓ Dependency analysis complete!");
}

// =============================================================================
// SCENARIO 7: Epic Progress Tracking
// =============================================================================
// An AI agent tracks epic progress and story completion

#[test]
fn scenario_epic_progress() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== SCENARIO: Epic Progress Tracking ===");

    // 1. Search for epics
    println!("\n1. Finding active epics...");
    let epics = client
        .call_tool(
            "search_issues",
            json!({
                "project_key": test_project_key(),
                "issue_types": ["Epic"],
                "status_filter": ["In Progress", "To Do", "Open"],
                "limit": 5
            }),
        )
        .expect("Failed to search epics");

    let epics_result =
        McpTestClient::extract_tool_result(&epics).expect("Failed to extract result");
    let epic_count = epics_result["search_result"]["total"].as_u64().unwrap();
    println!("   Found {} active epics", epic_count);

    if epic_count > 0 {
        if let Some(issues) = epics_result["search_result"]["issues"].as_array() {
            if let Some(epic) = issues.first() {
                let epic_key = epic["key"].as_str().expect("Expected key");

                // 2. Get stories in this epic
                println!("\n2. Analyzing stories in epic {}...", epic_key);

                // Note: Our current implementation doesn't have epic-specific filtering
                // This is a GAP we should add!
                println!("   ⚠️  TODO: Need epic-specific story filtering");
                println!("   MISSING FEATURE: Cannot query stories by epic parent");

                // 3. Calculate epic completion percentage
                println!("\n3. Calculating epic progress...");
                println!("   ⚠️  TODO: Need story point aggregation");
                println!("   MISSING FEATURE: Cannot calculate epic completion %");
            }
        }
    }

    println!("\n✓ Epic progress check complete (with limitations)");
}

// =============================================================================
// SCENARIO 8: Automated Status Updates
// =============================================================================
// An AI agent provides automated status updates based on activity

#[test]
fn scenario_automated_status_update() {
    let mut client = McpTestClient::new().expect("Failed to create test client");

    println!("\n=== SCENARIO: Automated Status Update ===");

    // 1. Get recently updated issues
    println!("\n1. Checking recent activity...");
    let recent = client
        .call_tool(
            "get_user_issues",
            json!({
                "status_filter": ["In Progress", "In Review", "Done"],
                "updated_after": "7 days ago"
            }),
        )
        .expect("Failed to get recent issues");

    let recent_result =
        McpTestClient::extract_tool_result(&recent).expect("Failed to extract result");
    println!(
        "   {} issues updated in last 7 days",
        recent_result["search_result"]["total"].as_u64().unwrap()
    );

    // 2. Analyze velocity
    if let Some(issues) = recent_result["search_result"]["issues"].as_array() {
        println!("\n2. Analyzing velocity...");
        let done_count = issues
            .iter()
            .filter(|issue| {
                if let Some(status) = issue["status"].as_str() {
                    status.eq_ignore_ascii_case("Done") || status.eq_ignore_ascii_case("Resolved")
                } else {
                    false
                }
            })
            .count();

        println!("   Completed {} issues this week", done_count);
        println!("   Average: {:.1} issues per day", done_count as f64 / 7.0);
    }

    // 3. Identify at-risk items
    println!("\n3. Identifying at-risk items...");
    let stale = client
        .call_tool(
            "get_user_issues",
            json!({
                "status_filter": ["In Progress"],
                "updated_before": "3 days ago"
            }),
        )
        .expect("Failed to get stale issues");

    let stale_result =
        McpTestClient::extract_tool_result(&stale).expect("Failed to extract result");
    let stale_count = stale_result["search_result"]["total"].as_u64().unwrap();
    if stale_count > 0 {
        println!(
            "   ⚠️  {} issues in progress with no updates for 3+ days",
            stale_count
        );
    } else {
        println!("   ✅ All in-progress issues are being actively worked");
    }

    println!("\n✓ Status update complete!");
}

// =============================================================================
// SCENARIO 9: Quality Gate Checking
// =============================================================================
// An AI agent validates if an issue meets quality standards before completion

#[test]
fn scenario_quality_gate() {
    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    println!("\n=== SCENARIO: Quality Gate Validation ===");

    // 1. Get issue details
    println!("\n1. Validating {}...", issue_key);
    let details = client
        .call_tool(
            "get_issue_details",
            json!({
                "issue_key": issue_key,
                "include_comments": true
            }),
        )
        .expect("Failed to get issue details");

    let details_result =
        McpTestClient::extract_tool_result(&details).expect("Failed to extract result");

    // 2. Quality checks
    println!("\n2. Running quality checks...");
    let mut passed = 0;
    let mut failed = 0;

    // Check: Has description?
    if let Some(desc) = details_result["description"].as_str() {
        if desc.trim().len() > 50 {
            println!("   ✅ Description is comprehensive");
            passed += 1;
        } else {
            println!("   ❌ Description is too short");
            failed += 1;
        }
    }

    // Check: Has comments (indicating discussion)?
    if let Some(comments) = details_result["comments"].as_array() {
        if comments.len() > 0 {
            println!("   ✅ Has team discussion ({} comments)", comments.len());
            passed += 1;
        } else {
            println!("   ⚠️  No comments - may need review");
        }
    }

    // Check: Has todos for tracking work?
    println!("\n3. Checking todos...");
    let todos = client
        .call_tool(
            "list_todos",
            json!({
                "issue_key": issue_key
            }),
        )
        .expect("Failed to list todos");

    let todos_result =
        McpTestClient::extract_tool_result(&todos).expect("Failed to extract result");
    if let Some(todos_array) = todos_result["todos"].as_array() {
        let total = todos_array.len();
        let completed = todos_array
            .iter()
            .filter(|t| t["completed"].as_bool().unwrap_or(false))
            .count();

        if total > 0 {
            println!(
                "   ✅ Has work breakdown ({}/{} completed)",
                completed, total
            );
            passed += 1;

            if completed == total {
                println!("   ✅ All todos completed!");
                passed += 1;
            } else {
                println!("   ⚠️  Some todos remain incomplete");
            }
        }
    }

    println!("\n4. Quality gate result:");
    println!("   Passed: {}, Failed: {}", passed, failed);
    if failed == 0 && passed >= 3 {
        println!("   ✅ PASSED - Issue meets quality standards");
    } else {
        println!("   ⚠️  REVIEW NEEDED - Quality standards not fully met");
    }

    println!("\n✓ Quality gate check complete!");
}

// =============================================================================
// SCENARIO 10: Missing Features Report
// =============================================================================
// This test identifies gaps in our current tooling

#[test]
fn scenario_identify_gaps() {
    println!("\n=== FEATURE GAP ANALYSIS ===\n");

    println!("Based on research and scenarios, we are MISSING these key features:\n");

    println!("1. SPRINT MANAGEMENT");
    println!("   ❌ Get sprint info (sprint name, start/end dates, goals)");
    println!("   ❌ Get issues in a sprint");
    println!("   ❌ Move issues to/from sprint");
    println!("   ❌ Sprint velocity and burndown data");
    println!("   ❌ Create/start/complete sprints");

    println!("\n2. EPIC & STORY HIERARCHY");
    println!("   ❌ Get stories in an epic");
    println!("   ❌ Get epic progress/completion percentage");
    println!("   ❌ Calculate story point totals");
    println!("   ❌ Link issue as epic child");

    println!("\n3. BOARD MANAGEMENT");
    println!("   ❌ List available boards");
    println!("   ❌ Get board configuration");
    println!("   ❌ Get board backlog");
    println!("   ❌ Rank/reorder issues on board");

    println!("\n4. BULK OPERATIONS");
    println!("   ❌ Bulk update multiple issues");
    println!("   ❌ Bulk transition");
    println!("   ❌ Bulk assign");

    println!("\n5. WATCHERS & NOTIFICATIONS");
    println!("   ❌ Add/remove watchers");
    println!("   ❌ Get list of watchers");
    println!("   ❌ @mention users in comments");

    println!("\n6. LABELS & COMPONENTS");
    println!("   ❌ Add/remove labels");
    println!("   ❌ List available labels");
    println!("   ❌ Update components");
    println!("   ❌ List available components");

    println!("\n7. VERSIONS & RELEASES");
    println!("   ❌ Get project versions");
    println!("   ❌ Create release version");
    println!("   ❌ Set fix version");
    println!("   ❌ Release notes generation");

    println!("\n8. ADVANCED FILTERING");
    println!("   ❌ Save JQL filter");
    println!("   ❌ Get saved filters");
    println!("   ❌ Share filters");
    println!("   ❌ Get issues by label");
    println!("   ❌ Get issues by component");

    println!("\n9. TIME TRACKING ENHANCEMENTS");
    println!("   ❌ Get remaining estimate");
    println!("   ❌ Update original estimate");
    println!("   ❌ Get time tracking report");
    println!("   ❌ Compare estimated vs actual time");

    println!("\n10. ISSUE LINKING");
    println!("   ❌ Create issue link (blocks, relates to, etc.)");
    println!("   ❌ Delete issue link");
    println!("   ❌ Get available link types");

    println!("\n11. ADVANCED SEARCH");
    println!("   ❌ Search by epic");
    println!("   ❌ Search by sprint");
    println!("   ❌ Search by label");
    println!("   ❌ Search by component");
    println!("   ❌ Search by fix version");
    println!("   ❌ Complex JQL passthrough");

    println!("\n12. REPORTING & ANALYTICS");
    println!("   ❌ Get velocity report");
    println!("   ❌ Get burndown chart data");
    println!("   ❌ Get control chart data");
    println!("   ❌ Time in status report");

    println!("\n✓ Gap analysis complete!");
    println!("\nPRIORITY RECOMMENDATIONS:");
    println!("  1. Sprint management (critical for agile teams)");
    println!("  2. Epic/story hierarchy (for progress tracking)");
    println!("  3. Issue linking (for dependency management)");
    println!("  4. Labels and components (for organization)");
    println!("  5. Advanced search filters (for AI queries)");
}
