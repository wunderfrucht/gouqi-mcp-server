#!/bin/bash

# Comprehensive test script for JIRA MCP Server
# Tests all major functionality

set -e
source .env

echo "🧪 COMPREHENSIVE JIRA MCP SERVER TESTS"
echo "=========================================="
echo ""

cargo build --release --quiet 2>/dev/null

SERVER="./jira-mcp-server/target/release/jira-mcp-server"

{
    # Initialize
    echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
    sleep 0.3

    # Test 1: Connection Test
    echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"test_connection","arguments":{}}}'
    sleep 0.5

    # Test 2: Search Issues (basic)
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_issues","arguments":{"limit":5}}}'
    sleep 0.5

    # Test 3: Search Issues (with filters)
    echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_issues","arguments":{"project_key":"SCRUM","limit":3}}}'
    sleep 0.5

    # Test 4: Get User Issues (my issues)
    echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_user_issues","arguments":{}}}'
    sleep 0.5

    # Test 5: Get User Issues (with status filter)
    echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_user_issues","arguments":{"status_filter":["To Do","In Progress"]}}}'
    sleep 0.5

    # Test 6: Get Issue Details
    echo '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"get_issue_details","arguments":{"issue_key":"SCRUM-1"}}}'
    sleep 0.5

    # Test 7: Get Issue Details with comments
    echo '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"get_issue_details","arguments":{"issue_key":"SCRUM-1","include_comments":true}}}'
    sleep 0.5

    # Test 8: Get Create Metadata
    echo '{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"get_create_metadata","arguments":{"project_key":"SCRUM"}}}'
    sleep 0.5

    # Test 9: List Todos
    echo '{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"list_todos","arguments":{"issue_key":"SCRUM-1"}}}'
    sleep 0.5

    # Test 10: Get Active Work Sessions
    echo '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"get_active_work_sessions","arguments":{}}}'
    sleep 0.5

} | env JIRA_URL="$JIRA_URL" JIRA_AUTH_TYPE="$JIRA_AUTH_TYPE" JIRA_USERNAME="$JIRA_USERNAME" JIRA_PASSWORD="$JIRA_PASSWORD" $SERVER 2>&1 | python3 << 'PYTHON_EOF'
import sys, json

test_results = {}
errors = []

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue

    # Skip non-JSON log lines
    if not line.startswith("{"):
        if "ERROR" in line:
            print(f"⚠️  {line}")
        continue

    try:
        data = json.loads(line)

        if "result" in data:
            req_id = data.get("id", "?")
            test_results[req_id] = "SUCCESS"

            print(f"\n{'='*60}")
            print(f"✅ TEST {req_id} SUCCESS")
            print(f"{'='*60}")

            if "content" in data["result"]:
                for item in data["result"]["content"]:
                    if item.get("type") == "text":
                        text = item.get("text", "")
                        try:
                            parsed = json.loads(text)

                            # Test 1: Connection
                            if req_id == 1:
                                print("📡 CONNECTION TEST:")
                                print(text[:200])

                            # Test 2: Search Issues (basic)
                            elif req_id == 2 and "issues" in parsed:
                                print("🔍 SEARCH ISSUES (basic):")
                                print(f"  Total: {parsed.get('total', 0)}")
                                print(f"  Returned: {len(parsed.get('issues', []))}")
                                for issue in parsed.get('issues', [])[:3]:
                                    print(f"  • {issue.get('key')}: {issue.get('summary')[:50]}")

                            # Test 3: Search Issues (filtered)
                            elif req_id == 3 and "issues" in parsed:
                                print("🔍 SEARCH ISSUES (project filter):")
                                print(f"  Total: {parsed.get('total', 0)}")
                                for issue in parsed.get('issues', []):
                                    print(f"  • {issue.get('key')}: {issue.get('summary')[:50]}")

                            # Test 4: Get User Issues
                            elif req_id == 4 and "issues" in parsed:
                                print("👤 GET USER ISSUES:")
                                print(f"  Total: {parsed.get('total', 0)}")
                                for issue in parsed.get('issues', [])[:5]:
                                    status = issue.get('status', 'Unknown')
                                    print(f"  • {issue.get('key')} [{status}]: {issue.get('summary')[:40]}")

                            # Test 5: Get User Issues (filtered)
                            elif req_id == 5 and "issues" in parsed:
                                print("👤 GET USER ISSUES (status filter):")
                                print(f"  Total: {parsed.get('total', 0)}")
                                for issue in parsed.get('issues', []):
                                    print(f"  • {issue.get('key')}: {issue.get('summary')[:50]}")

                            # Test 6: Get Issue Details
                            elif req_id == 6:
                                print("📋 GET ISSUE DETAILS:")
                                if "issue_info" in parsed:
                                    info = parsed["issue_info"]
                                    print(f"  Key: {info.get('key')}")
                                    print(f"  Summary: {info.get('summary')}")
                                    print(f"  Status: {info.get('status')}")
                                    print(f"  Type: {info.get('issue_type')}")
                                else:
                                    print(json.dumps(parsed, indent=2)[:300])

                            # Test 7: Get Issue Details with comments
                            elif req_id == 7:
                                print("📋 GET ISSUE DETAILS (with comments):")
                                if "comments" in parsed:
                                    print(f"  Comments: {len(parsed.get('comments', []))}")
                                else:
                                    print("  No comments field found")

                            # Test 8: Get Create Metadata
                            elif req_id == 8 and "issue_types" in parsed:
                                print("📝 GET CREATE METADATA:")
                                print(f"  Project: {parsed.get('project_name')}")
                                for it in parsed.get('issue_types', []):
                                    print(f"  Type: {it.get('name')}")
                                    print(f"    Required: {it.get('required_fields', [])[:5]}")

                            # Test 9: List Todos
                            elif req_id == 9 and "todos" in parsed:
                                print("✅ LIST TODOS:")
                                print(f"  Total: {len(parsed.get('todos', []))}")
                                for todo in parsed.get('todos', [])[:3]:
                                    status = "✓" if todo.get('completed') else "○"
                                    print(f"  {status} {todo.get('text')[:50]}")

                            # Test 10: Get Active Work Sessions
                            elif req_id == 10:
                                print("⏱️  GET ACTIVE WORK SESSIONS:")
                                sessions = parsed.get('sessions', [])
                                print(f"  Active sessions: {len(sessions)}")
                                for session in sessions[:3]:
                                    print(f"  • {session.get('issue_key')}: {session.get('todo_text')[:40]}")

                            else:
                                # Generic output for other tests
                                print(json.dumps(parsed, indent=2)[:500])

                        except json.JSONDecodeError:
                            print(text[:300])

        elif "error" in data:
            req_id = data.get("id", "?")
            test_results[req_id] = "ERROR"
            msg = data["error"].get("message", "Unknown error")
            code = data["error"].get("code", "?")

            print(f"\n{'='*60}")
            print(f"❌ TEST {req_id} ERROR")
            print(f"{'='*60}")
            print(f"Code: {code}")
            print(f"Message: {msg}")

            errors.append({
                "test": req_id,
                "code": code,
                "message": msg
            })

    except json.JSONDecodeError:
        pass

print("\n" + "="*60)
print("SUMMARY")
print("="*60)

success_count = sum(1 for r in test_results.values() if r == "SUCCESS")
error_count = sum(1 for r in test_results.values() if r == "ERROR")

print(f"✅ Successful: {success_count}")
print(f"❌ Failed: {error_count}")
print(f"📊 Total: {len(test_results)}")

if errors:
    print("\n🔴 ERRORS FOUND:")
    for err in errors:
        print(f"  Test {err['test']}: {err['message']}")

PYTHON_EOF

echo ""
echo "✅ Tests complete"
