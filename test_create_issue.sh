#!/bin/bash

# Quick test for the new create_issue and get_create_metadata tools
# Usage: ./test_create_issue.sh PROJECT-KEY

set -e

source .env

PROJECT_KEY="${1:-SCRUM}"

echo "🧪 Testing new issue creation tools for project: $PROJECT_KEY"
echo ""

cargo build --release --quiet 2>/dev/null

SERVER="./jira-mcp-server/target/release/jira-mcp-server"

{
    # Initialize
    echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
    sleep 0.3

    # Test 1: Get Create Metadata
    echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_create_metadata","arguments":{"project_key":"'$PROJECT_KEY'","issue_type":"Task"}}}'
    sleep 1

    # Test 2: Create Issue
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_issue","arguments":{"project_key":"'$PROJECT_KEY'","summary":"🧪 Test from test_create_issue.sh - DELETE ME","description":"This is a test issue","initial_todos":["Verify creation","Delete this issue"],"assign_to_me":true,"labels":["test"]}}}'
    sleep 1

} | env JIRA_URL="$JIRA_URL" JIRA_AUTH_TYPE="$JIRA_AUTH_TYPE" JIRA_USERNAME="$JIRA_USERNAME" JIRA_PASSWORD="$JIRA_PASSWORD" $SERVER 2>&1 | python3 << 'PYTHON_EOF'
import sys, json

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        data = json.loads(line)
        if "result" in data:
            req_id = data.get("id", "?")
            print(f"\n✅ Test {req_id} Success")
            if "content" in data["result"]:
                for item in data["result"]["content"]:
                    if item.get("type") == "text":
                        text = item.get("text", "")
                        try:
                            parsed = json.loads(text)
                            if req_id == 1 and "issue_types" in parsed:
                                print("📋 CREATE METADATA:")
                                for it in parsed.get("issue_types", []):
                                    name = it.get("name", "Unknown")
                                    req_fields = it.get("required_fields", [])
                                    print(f"  Type: {name}")
                                    print(f"  Required: {req_fields[:3]}...")
                            elif req_id == 2 and "issue_key" in parsed:
                                print("🎉 ISSUE CREATED:")
                                key = parsed.get("issue_key", "")
                                url = parsed.get("issue_url", "")
                                print(f"  Key: {key}")
                                print(f"  URL: {url}")
                            else:
                                print(json.dumps(parsed, indent=2)[:500])
                        except:
                            print(text[:500])
        elif "error" in data:
            req_id = data.get("id", "?")
            msg = data["error"].get("message", "Unknown")
            print(f"\n❌ Test {req_id} Error: {msg}")
    except:
        pass
PYTHON_EOF

echo ""
echo "✅ Tests complete"
