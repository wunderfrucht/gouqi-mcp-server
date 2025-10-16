#!/bin/bash
source .env
export JIRA_URL JIRA_AUTH_TYPE JIRA_USERNAME JIRA_PASSWORD

echo "Testing get_user_issues..."
(
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
sleep 0.5
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_user_issues","arguments":{}}}'
sleep 2
) | ./target/release/jira-mcp-server 2>&1 &

PID=$!
sleep 5
kill $PID 2>/dev/null
wait $PID 2>/dev/null

echo ""
echo "Testing search_issues with project_key..."
(
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
sleep 0.5
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_issues","arguments":{"project_key":"SCRUM","limit":5}}}'
sleep 2
) | ./target/release/jira-mcp-server 2>&1 &

PID=$!
sleep 5
kill $PID 2>/dev/null
wait $PID 2>/dev/null
