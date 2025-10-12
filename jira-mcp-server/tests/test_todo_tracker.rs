/// Integration tests for todo tracker functionality
mod common;

use common::{test_issue_key, McpTestClient};
use serde_json::json;

/// Helper function to add a todo and return its 1-based index
fn add_todo_and_get_index(client: &mut McpTestClient, issue_key: &str, todo_text: &str) -> String {
    // Add the todo
    client
        .call_tool(
            "add_todo",
            json!({
                "issue_key": issue_key,
                "todo_text": todo_text
            }),
        )
        .expect("Failed to add todo");

    // List todos to find the newly added one's index
    let list_response = client
        .call_tool("list_todos", json!({"issue_key": issue_key}))
        .expect("Failed to list todos");

    let list_result =
        McpTestClient::extract_tool_result(&list_response).expect("Failed to extract list result");

    let todos_count = list_result["total_count"].as_u64().unwrap();

    // The newly added todo should be the last one
    todos_count.to_string()
}

#[test]
fn test_set_todo_base() {
    // Test setting the base issue for todo tracking

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let response = client
        .call_tool(
            "set_todo_base",
            json!({
                "issue_key": issue_key
            }),
        )
        .expect("Failed to call set_todo_base");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("base_issue_key").is_some());
    assert_eq!(result["base_issue_key"], issue_key);
    assert!(result.get("message").is_some());
}

#[test]
fn test_list_todos_basic() {
    // Test listing todos from an issue

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let response = client
        .call_tool(
            "list_todos",
            json!({
                "issue_key": issue_key
            }),
        )
        .expect("Failed to call list_todos");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("todos").is_some());
    assert!(result.get("total_count").is_some());
    assert_eq!(result["issue_key"], issue_key);

    // Verify todos is an array (expect will panic if it's not)
    result["todos"]
        .as_array()
        .expect("todos should be an array");
}

#[test]
fn test_add_todo() {
    // Test adding a new todo to an issue

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    let todo_text = format!(
        "🧪 Test todo from integration test - {}",
        chrono::Utc::now().timestamp()
    );

    let response = client
        .call_tool(
            "add_todo",
            json!({
                "issue_key": issue_key,
                "todo_text": todo_text
            }),
        )
        .expect("Failed to call add_todo");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("todo").is_some());
    assert!(result.get("message").is_some());
    assert!(result.get("updated_description").is_some());

    let todo = &result["todo"];
    assert_eq!(todo["text"], todo_text);
    assert_eq!(todo["completed"], false);
    assert!(todo.get("id").is_some());
}

#[test]
fn test_update_todo_status() {
    // Test updating a todo's completion status

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // First add a todo
    let todo_text = format!(
        "🧪 Test todo for status update - {}",
        chrono::Utc::now().timestamp()
    );

    let add_response = client
        .call_tool(
            "add_todo",
            json!({
                "issue_key": issue_key,
                "todo_text": todo_text
            }),
        )
        .expect("Failed to add todo");

    let add_result =
        McpTestClient::extract_tool_result(&add_response).expect("Failed to extract add result");

    let todo_id = add_result["todo"]["id"]
        .as_str()
        .expect("todo should have id");

    // Now update its status
    let update_response = client
        .call_tool(
            "update_todo",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_id,
                "completed": true
            }),
        )
        .expect("Failed to call update_todo");

    let update_result = McpTestClient::extract_tool_result(&update_response)
        .expect("Failed to extract update result");

    assert!(update_result.get("todo").is_some());
    assert_eq!(update_result["todo"]["completed"], true);
    assert!(update_result.get("message").is_some());
}

#[test]
fn test_start_and_pause_work() {
    // Test starting work on a todo and pausing it (creates worklog)

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // Add a todo and get its index
    let todo_text = format!(
        "🧪 Test todo for work tracking - {}",
        chrono::Utc::now().timestamp()
    );
    let todo_index = add_todo_and_get_index(&mut client, &issue_key, &todo_text);

    // Start work
    let start_response = client
        .call_tool(
            "start_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index
            }),
        )
        .expect("Failed to start work");

    let start_result = McpTestClient::extract_tool_result(&start_response)
        .expect("Failed to extract start result");

    assert!(start_result.get("todo").is_some());
    assert!(start_result.get("started_at").is_some());
    assert!(start_result.get("message").is_some());

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Pause work (should create worklog)
    let pause_response = client
        .call_tool(
            "pause_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index,
                "worklog_comment": "Test pause from integration test"
            }),
        )
        .expect("Failed to pause work");

    let pause_result = McpTestClient::extract_tool_result(&pause_response)
        .expect("Failed to extract pause result");

    assert!(pause_result.get("todo").is_some());
    assert!(pause_result.get("time_spent_seconds").is_some());
    assert!(pause_result.get("time_spent_formatted").is_some());
    assert!(pause_result.get("worklog").is_some());

    let time_spent = pause_result["time_spent_seconds"]
        .as_u64()
        .expect("time_spent_seconds should be a number");
    assert!(time_spent >= 2, "Should have logged at least 2 seconds");

    // Verify worklog was created
    let worklog = &pause_result["worklog"];
    assert!(worklog.get("id").is_some());
    assert!(worklog.get("time_spent_seconds").is_some());
}

#[test]
fn test_start_and_complete_work() {
    // Test starting work and completing it (creates worklog and marks todo as done)

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // Add a todo and get its index
    let todo_text = format!(
        "🧪 Test todo for completion - {}",
        chrono::Utc::now().timestamp()
    );
    let todo_index = add_todo_and_get_index(&mut client, &issue_key, &todo_text);

    // Start work
    let start_response = client
        .call_tool(
            "start_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index
            }),
        )
        .expect("Failed to start work");

    let start_result = McpTestClient::extract_tool_result(&start_response)
        .expect("Failed to extract start result");

    assert!(start_result.get("started_at").is_some());

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Complete work (should create worklog and mark todo as completed)
    let complete_response = client
        .call_tool(
            "complete_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index,
                "worklog_comment": "Test completion from integration test",
                "mark_completed": true
            }),
        )
        .expect("Failed to complete work");

    let complete_result = McpTestClient::extract_tool_result(&complete_response)
        .expect("Failed to extract complete result");

    assert!(complete_result.get("todo").is_some());
    assert!(complete_result.get("time_spent_seconds").is_some());
    assert!(complete_result.get("time_spent_formatted").is_some());
    assert!(complete_result.get("worklog").is_some());

    // Verify todo was marked as completed
    let todo = &complete_result["todo"];
    assert_eq!(todo["completed"], true);

    let time_spent = complete_result["time_spent_seconds"]
        .as_u64()
        .expect("time_spent_seconds should be a number");
    assert!(time_spent >= 2, "Should have logged at least 2 seconds");

    // Verify worklog was created
    let worklog = &complete_result["worklog"];
    assert!(worklog.get("id").is_some());
    assert!(worklog.get("time_spent_seconds").is_some());
}

#[test]
fn test_checkpoint_work() {
    // Test checkpointing work (logs time but doesn't end session)

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // Add a todo and get its index
    let todo_text = format!(
        "🧪 Test todo for checkpoint - {}",
        chrono::Utc::now().timestamp()
    );
    let todo_index = add_todo_and_get_index(&mut client, &issue_key, &todo_text);

    // Start work
    client
        .call_tool(
            "start_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index
            }),
        )
        .expect("Failed to start work");

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Checkpoint (logs time but keeps session active)
    let checkpoint_response = client
        .call_tool(
            "checkpoint_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index,
                "worklog_comment": "Test checkpoint from integration test"
            }),
        )
        .expect("Failed to checkpoint work");

    let checkpoint_result = McpTestClient::extract_tool_result(&checkpoint_response)
        .expect("Failed to extract checkpoint result");

    assert!(checkpoint_result.get("todo").is_some());
    assert!(checkpoint_result.get("checkpoint_time_seconds").is_some());
    assert!(checkpoint_result.get("total_accumulated_seconds").is_some());
    assert!(checkpoint_result.get("worklog").is_some());

    let checkpoint_time = checkpoint_result["checkpoint_time_seconds"]
        .as_u64()
        .expect("checkpoint_time_seconds should be a number");
    assert!(
        checkpoint_time >= 2,
        "Should have checkpointed at least 2 seconds"
    );

    // Wait a bit more
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Complete work (should log additional time)
    let complete_response = client
        .call_tool(
            "complete_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index,
                "mark_completed": true
            }),
        )
        .expect("Failed to complete work");

    let complete_result = McpTestClient::extract_tool_result(&complete_response)
        .expect("Failed to extract complete result");

    let total_time = complete_result["time_spent_seconds"]
        .as_u64()
        .expect("time_spent_seconds should be a number");

    // Total time should be checkpoint + additional time
    assert!(
        total_time >= 4,
        "Total time should be at least 4 seconds (checkpoint + additional)"
    );
}

#[test]
fn test_cancel_work() {
    // Test canceling work (discards time without logging)

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // Add a todo and get its index
    let todo_text = format!(
        "🧪 Test todo for cancel - {}",
        chrono::Utc::now().timestamp()
    );
    let todo_index = add_todo_and_get_index(&mut client, &issue_key, &todo_text);

    // Start work
    client
        .call_tool(
            "start_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index
            }),
        )
        .expect("Failed to start work");

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Cancel work (should not create worklog)
    let cancel_response = client
        .call_tool(
            "cancel_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index
            }),
        )
        .expect("Failed to cancel work");

    let cancel_result = McpTestClient::extract_tool_result(&cancel_response)
        .expect("Failed to extract cancel result");

    assert!(cancel_result.get("todo").is_some());
    assert!(cancel_result.get("discarded_time_seconds").is_some());
    assert!(cancel_result.get("message").is_some());

    let discarded_time = cancel_result["discarded_time_seconds"]
        .as_u64()
        .expect("discarded_time_seconds should be a number");
    assert!(
        discarded_time >= 2,
        "Should have discarded at least 2 seconds"
    );
}

#[test]
fn test_get_active_work_sessions() {
    // Test getting active work sessions

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // Add a todo and get its index
    let todo_text = format!(
        "🧪 Test todo for active session - {}",
        chrono::Utc::now().timestamp()
    );
    let todo_index = add_todo_and_get_index(&mut client, &issue_key, &todo_text);

    // Start work
    client
        .call_tool(
            "start_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index
            }),
        )
        .expect("Failed to start work");

    // Get active sessions
    let sessions_response = client
        .call_tool("get_active_work_sessions", json!({}))
        .expect("Failed to get active sessions");

    let sessions_result = McpTestClient::extract_tool_result(&sessions_response)
        .expect("Failed to extract sessions result");

    assert!(sessions_result.get("sessions").is_some());
    assert!(sessions_result.get("total_count").is_some());

    let sessions = sessions_result["sessions"]
        .as_array()
        .expect("sessions should be an array");
    let total_count = sessions_result["total_count"]
        .as_u64()
        .expect("total_count should be a number");

    assert!(total_count >= 1, "Should have at least one active session");
    assert!(
        !sessions.is_empty(),
        "Should have at least one active session"
    );

    // Verify session structure
    let session = &sessions[0];
    assert!(session.get("issue_key").is_some());
    assert!(session.get("todo_id").is_some());
    assert!(session.get("todo_text").is_some());
    assert!(session.get("started_at").is_some());
    assert!(session.get("duration_seconds").is_some());
    assert!(session.get("duration_formatted").is_some());

    // Clean up - complete the work
    client
        .call_tool(
            "complete_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index,
                "mark_completed": true
            }),
        )
        .expect("Failed to complete work");
}

#[test]
#[should_panic(expected = "Tool call failed")]
fn test_complete_work_without_starting() {
    // Test that completing work without starting it fails

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // Add a todo and get its index
    let todo_text = format!(
        "🧪 Test todo for error case - {}",
        chrono::Utc::now().timestamp()
    );
    let todo_index = add_todo_and_get_index(&mut client, &issue_key, &todo_text);

    // Try to complete work without starting - should fail
    let complete_response = client
        .call_tool(
            "complete_todo_work",
            json!({
                "issue_key": issue_key,
                "todo_id_or_index": todo_index,
                "mark_completed": true
            }),
        )
        .expect("Failed to call complete_todo_work");

    // This should fail
    McpTestClient::extract_tool_result(&complete_response)
        .expect("Should fail - no active work session");
}

#[test]
fn test_list_todos_with_status_filter() {
    // Test listing todos with status filter

    let mut client = McpTestClient::new().expect("Failed to create test client");
    let issue_key = test_issue_key();

    // List completed todos
    let response = client
        .call_tool(
            "list_todos",
            json!({
                "issue_key": issue_key,
                "status_filter": ["completed"]
            }),
        )
        .expect("Failed to call list_todos");

    let result =
        McpTestClient::extract_tool_result(&response).expect("Failed to extract tool result");

    assert!(result.get("todos").is_some());

    let todos = result["todos"]
        .as_array()
        .expect("todos should be an array");

    // Verify all returned todos are completed
    for todo in todos {
        assert_eq!(todo["completed"], true, "All todos should be completed");
        assert_eq!(todo["status"], "completed", "Status should be completed");
    }
}
