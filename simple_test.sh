#!/bin/bash

# Simple test script for JIRA MCP Server
# Usage: ./simple_test.sh ISSUE-KEY

set -e

# Load environment
source .env

ISSUE_KEY="${1:-}"

if [ -z "$ISSUE_KEY" ]; then
    echo "Usage: $0 ISSUE-KEY"
    echo "Example: $0 PROJ-123"
    exit 1
fi

echo "🧪 Testing JIRA MCP Server with issue: $ISSUE_KEY"
echo ""

# Build the server
echo "📦 Building server..."
cargo build --release --quiet 2>/dev/null

SERVER="./target/release/jira-mcp-server"

# Helper function to send MCP request
send_request() {
    local method=$1
    local params=$2
    local id=$((RANDOM))

    echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"method\":\"$method\",\"params\":$params}"
}

# Start server and test
{
    # Initialize
    echo "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1.0\"}}}"
    sleep 0.2

    # Initialized notification
    echo "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}"
    sleep 0.2

    # Test 1: Connection
    echo ""
    echo "============================================================"
    echo "TEST 1: Connection Test"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"test_connection\",\"arguments\":{}}}"
    sleep 0.5

    # Test 2: Assign Issue to Self
    echo ""
    echo "============================================================"
    echo "TEST 2: Assign Issue to Self (NEW TOOL!)"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"assign_issue\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\",\"assignee\":\"me\"}}}"
    sleep 0.5

    # Test 3: Get Custom Fields
    echo ""
    echo "============================================================"
    echo "TEST 3: Get Custom Fields from $ISSUE_KEY"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"get_custom_fields\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\"}}}"
    sleep 0.5

    # Test 4: Add Todo
    echo ""
    echo "============================================================"
    echo "TEST 4: Add Test Todo to $ISSUE_KEY"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"add_todo\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\",\"todo_text\":\"🧪 Test checkpoint - PLEASE DELETE\"}}}"
    sleep 0.5

    # Test 5: List Todos
    echo ""
    echo "============================================================"
    echo "TEST 5: List Todos from $ISSUE_KEY"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"list_todos\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\"}}}"
    sleep 0.5

    # Test 6: Start Work (using index 1 - first todo)
    echo ""
    echo "============================================================"
    echo "TEST 6: Start Work on First Todo"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"tools/call\",\"params\":{\"name\":\"start_todo_work\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\",\"todo_id_or_index\":\"1\"}}}"
    sleep 3

    # Test 7: Checkpoint
    echo ""
    echo "============================================================"
    echo "TEST 7: Checkpoint Work (New Feature!)"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"tools/call\",\"params\":{\"name\":\"checkpoint_todo_work\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\",\"todo_id_or_index\":\"1\",\"worklog_comment\":\"Test checkpoint\"}}}"
    sleep 2

    # Test 8: Complete Work
    echo ""
    echo "============================================================"
    echo "TEST 8: Complete Work (Test ID Stability Fix)"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"tools/call\",\"params\":{\"name\":\"complete_todo_work\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\",\"todo_id_or_index\":\"1\",\"mark_completed\":true}}}"
    sleep 0.5

    # Test 9: Verify Worklogs
    echo ""
    echo "============================================================"
    echo "TEST 9: Verify Worklogs (Check Time Logging)"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/call\",\"params\":{\"name\":\"get_issue_details\",\"arguments\":{\"issue_key\":\"$ISSUE_KEY\",\"include_comments\":false,\"include_attachments\":false,\"include_worklogs\":true}}}"
    sleep 0.5

    # Extract project key from issue key (e.g., "PROJ-123" -> "PROJ")
    PROJECT_KEY=$(echo "$ISSUE_KEY" | cut -d'-' -f1)

    # Test 10: Get Create Metadata
    echo ""
    echo "============================================================"
    echo "TEST 10: Get Create Metadata for Project $PROJECT_KEY (NEW TOOL!)"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"tools/call\",\"params\":{\"name\":\"get_create_metadata\",\"arguments\":{\"project_key\":\"$PROJECT_KEY\",\"issue_type\":\"Task\"}}}"
    sleep 0.5

    # Test 11: Create a Test Issue
    echo ""
    echo "============================================================"
    echo "TEST 11: Create Test Issue (NEW TOOL!)"
    echo "============================================================"
    echo "{\"jsonrpc\":\"2.0\",\"id\":11,\"method\":\"tools/call\",\"params\":{\"name\":\"create_issue\",\"arguments\":{\"project_key\":\"$PROJECT_KEY\",\"summary\":\"🧪 Test issue from simple_test.sh - PLEASE DELETE\",\"description\":\"This is a test issue created by the automated test script.\",\"initial_todos\":[\"Verify issue creation\",\"Delete this test issue\"],\"assign_to_me\":true,\"labels\":[\"test\",\"automated\"]}}}"
    sleep 0.5

} | env JIRA_URL="$JIRA_URL" JIRA_AUTH_TYPE="$JIRA_AUTH_TYPE" JIRA_USERNAME="$JIRA_USERNAME" JIRA_PASSWORD="$JIRA_PASSWORD" $SERVER 2>&1 | python3 -c '
import sys, json

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        data = json.loads(line)
        if "result" in data:
            req_id = data.get("id", "?")
            print(f"✅ Success (id={req_id})")
            # Pretty print result content
            if "content" in data["result"]:
                for item in data["result"]["content"]:
                    if item.get("type") == "text":
                        text = item.get("text", "")
                        # Try to parse as JSON for pretty print
                        try:
                            parsed = json.loads(text)
                            # Special handling for worklog verification (test 9)
                            if req_id == 9 and "worklogs" in parsed:
                                print("\n📋 WORKLOG VERIFICATION:")
                                worklogs = parsed.get("worklogs", [])
                                total_seconds = 0
                                for wl in worklogs[-5:]:  # Show last 5 worklogs
                                    author = wl.get("author", "Unknown")
                                    comment = wl.get("comment", "No comment")
                                    time_spent = wl.get("time_spent", "0s")
                                    time_secs = wl.get("time_spent_seconds", 0)
                                    created = wl.get("created", "")[:19]  # Trim to datetime
                                    total_seconds += time_secs
                                    print(f"  • [{created}] {time_spent} - {comment[:50]}")
                                print(f"\n  📊 Total time from last 5 entries: {total_seconds}s")
                                print(f"  ✅ Expected ~5s (3s first segment + 2s second segment)")
                            # Special handling for create metadata (test 10)
                            elif req_id == 10 and "issue_types" in parsed:
                                print("\n📋 CREATE METADATA:")
                                issue_types = parsed.get("issue_types", [])
                                for it in issue_types:
                                    name = it.get("name", "Unknown")
                                    required = ", ".join(it.get("required_fields", [])[:5])
                                    print(f"  • {name}: Required fields: {required}")
                            # Special handling for create issue (test 11)
                            elif req_id == 11 and "issue_key" in parsed:
                                print("\n🎉 ISSUE CREATED:")
                                issue_key = parsed.get("issue_key", "")
                                issue_url = parsed.get("issue_url", "")
                                summary = parsed.get("summary", "")
                                print(f"  📝 Issue: {issue_key}")
                                print(f"  📌 Summary: {summary}")
                                print(f"  🔗 URL: {issue_url}")
                                print(f"\n  ⚠️  REMINDER: Please delete this test issue!")
                            else:
                                print(json.dumps(parsed, indent=2))
                        except:
                            print(text)
        elif "error" in data:
            req_id = data.get("id", "?")
            msg = data["error"].get("message", "Unknown error")
            print(f"❌ Error (id={req_id}): {msg}")
        else:
            print(f"📢 {line}")
    except json.JSONDecodeError:
        # Not JSON, probably log output
        if "ERROR" in line or "WARN" in line:
            print(f"⚠️  {line}")
        elif line.startswith("{"):
            print(f"?? {line}")
'

echo ""
echo "============================================================"
echo "✅ TEST COMPLETE"
echo "============================================================"
