#!/bin/bash
source .env
export JIRA_URL JIRA_AUTH_TYPE JIRA_USERNAME JIRA_PASSWORD

echo "Testing various JIRA tools..."
(
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
sleep 0.5
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_issue_details","arguments":{"issue_key":"SCRUM-1"}}}'
sleep 2
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_todos","arguments":{"issue_key":"SCRUM-1"}}}'
sleep 2
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_create_metadata","arguments":{"project_key":"SCRUM"}}}'
sleep 2
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"search_issues","arguments":{"created_after":"-30d","limit":5}}}'
sleep 2
) | ./target/release/jira-mcp-server 2>&1 | python3 << 'PYEOF'
import sys, json

for line in sys.stdin:
    line = line.strip()
    if not line.startswith('{'):
        continue
    try:
        data = json.loads(line)
        if 'error' in data:
            test_id = data.get('id', '?')
            msg = data['error']['message'][:120]
            print(f"❌ Test {test_id}: {msg}")
        elif 'result' in data and data.get('id') != 0:
            test_id = data.get('id', '?')
            print(f"✅ Test {test_id}: SUCCESS")
    except:
        pass
PYEOF
