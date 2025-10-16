#!/bin/bash

# Simple direct test of JIRA MCP Server
source .env
export JIRA_URL
export JIRA_AUTH_TYPE
export JIRA_USERNAME
export JIRA_PASSWORD

echo "🧪 Testing JIRA MCP Server"
echo "==========================
"
echo "JIRA URL: ${JIRA_URL:0:30}..."
echo ""

# Test 1: Search Issues
echo "Test 1: Search Issues"
echo "---"
(
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
sleep 1
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_issues","arguments":{"limit":5}}}'
sleep 2
) | timeout 10 ./target/release/jira-mcp-server 2>&1 | grep -A 200 '"id":1' | python3 << 'PYEOF'
import sys, json
for line in sys.stdin:
    if not line.startswith("{"):
        continue
    try:
        data = json.loads(line)
        if data.get("id") == 1 and "result" in data:
            for item in data["result"].get("content", []):
                if item.get("type") == "text":
                    parsed = json.loads(item["text"])
                    if "issues" in parsed:
                        total = parsed.get("total", 0)
                        issues = parsed.get("issues", [])
                        print(f"✅ Found {total} total issues")
                        print(f"   Returned {len(issues)} issues:")
                        for issue in issues[:3]:
                            key = issue.get("key", "")
                            summary = issue.get("summary", "")[:50]
                            print(f"   - {key}: {summary}")
                    break
        elif data.get("id") == 1 and "error" in data:
            msg = data["error"].get("message", "")
            print(f"❌ Error: {msg}")
    except: pass
PYEOF
echo ""

# Test 2: Get User Issues
echo "Test 2: Get User Issues"
echo "---"
(
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
sleep 1
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_user_issues","arguments":{}}}'
sleep 2
) | timeout 10 ./target/release/jira-mcp-server 2>&1 | grep -A 200 '"id":2' | python3 << 'PYEOF'
import sys, json
for line in sys.stdin:
    if not line.startswith("{"):
        continue
    try:
        data = json.loads(line)
        if data.get("id") == 2 and "result" in data:
            for item in data["result"].get("content", []):
                if item.get("type") == "text":
                    parsed = json.loads(item["text"])
                    if "issues" in parsed:
                        total = parsed.get("total", 0)
                        issues = parsed.get("issues", [])
                        print(f"✅ Found {total} issues assigned to me")
                        for issue in issues[:5]:
                            key = issue.get("key", "")
                            status = issue.get("status", "")
                            summary = issue.get("summary", "")[:40]
                            print(f"   - {key} [{status}]: {summary}")
                    break
        elif data.get("id") == 2 and "error" in data:
            msg = data["error"].get("message", "")
            print(f"❌ Error: {msg}")
    except: pass
PYEOF
echo ""

# Test 3: Get Issue Details
echo "Test 3: Get Issue Details (SCRUM-1)"
echo "---"
(
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
sleep 1
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_issue_details","arguments":{"issue_key":"SCRUM-1"}}}'
sleep 2
) | timeout 10 ./target/release/jira-mcp-server 2>&1 | grep -A 200 '"id":3' | python3 << 'PYEOF'
import sys, json
for line in sys.stdin:
    if not line.startswith("{"):
        continue
    try:
        data = json.loads(line)
        if data.get("id") == 3 and "result" in data:
            for item in data["result"].get("content", []):
                if item.get("type") == "text":
                    parsed = json.loads(item["text"])
                    if "issue_info" in parsed:
                        info = parsed["issue_info"]
                        print("✅ Issue Details:")
                        print(f"   Key: {info.get('key', '')}")
                        print(f"   Summary: {info.get('summary', '')}")
                        print(f"   Status: {info.get('status', '')}")
                        print(f"   Type: {info.get('issue_type', '')}")
                        print(f"   Assignee: {info.get('assignee', 'Unassigned')}")
                    break
        elif data.get("id") == 3 and "error" in data:
            msg = data["error"].get("message", "")
            print(f"❌ Error: {msg}")
    except: pass
PYEOF
echo ""

# Test 4: Get Create Metadata
echo "Test 4: Get Create Metadata"
echo "---"
(
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
sleep 1
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_create_metadata","arguments":{"project_key":"SCRUM"}}}'
sleep 2
) | timeout 10 ./target/release/jira-mcp-server 2>&1 | grep -A 200 '"id":4' | python3 << 'PYEOF'
import sys, json
for line in sys.stdin:
    if not line.startswith("{"):
        continue
    try:
        data = json.loads(line)
        if data.get("id") == 4 and "result" in data:
            for item in data["result"].get("content", []):
                if item.get("type") == "text":
                    parsed = json.loads(item["text"])
                    if "issue_types" in parsed:
                        project_name = parsed.get("project_name", "")
                        print(f"✅ Project: {project_name}")
                        print("   Available issue types:")
                        for it in parsed.get("issue_types", []):
                            name = it.get("name", "")
                            req_count = len(it.get("required_fields", []))
                            print(f"   - {name}: {req_count} required fields")
                    break
        elif data.get("id") == 4 and "error" in data:
            msg = data["error"].get("message", "")
            print(f"❌ Error: {msg}")
    except: pass
PYEOF
echo ""

echo "✅ Tests complete"
