#!/usr/bin/env python3

import subprocess
import json
import time
import os

def test_mcp_server():
    """Test the JIRA MCP server with various queries"""

    # Start the server
    env = os.environ.copy()
    proc = subprocess.Popen(
        ['./target/release/jira-mcp-server'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        env=env
    )

    def send_request(id, method, params):
        request = {
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }
        proc.stdin.write(json.dumps(request) + '\n')
        proc.stdin.flush()

    # Initialize
    send_request(0, "initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "test", "version": "1.0"}
    })
    time.sleep(0.5)

    tests = [
        (1, "test_connection", {}),
        (2, "search_issues", {"limit": 5}),
        (3, "search_issues", {"project_key": "SCRUM", "limit": 3}),
        (4, "get_user_issues", {}),
        (5, "get_user_issues", {"status_filter": ["To Do", "In Progress"]}),
        (6, "get_issue_details", {"issue_key": "SCRUM-1"}),
        (7, "get_create_metadata", {"project_key": "SCRUM"}),
    ]

    # Send all test requests
    for test_id, tool_name, args in tests:
        send_request(test_id, "tools/call", {
            "name": tool_name,
            "arguments": args
        })
        time.sleep(0.3)

    # Give time for responses
    time.sleep(2)

    # Read responses
    results = {}
    errors = []

    proc.stdin.close()

    for line in proc.stdout:
        line = line.strip()
        if not line or not line.startswith('{'):
            continue

        try:
            data = json.loads(line)
            req_id = data.get("id")

            if req_id is None:
                continue

            if "result" in data:
                results[req_id] = "SUCCESS"
                print(f"\n{'='*60}")
                print(f"✅ TEST {req_id} SUCCESS")
                print(f"{'='*60}")

                # Extract and display relevant info
                if "content" in data["result"]:
                    for item in data["result"]["content"]:
                        if item.get("type") == "text":
                            text = item.get("text", "")
                            try:
                                parsed = json.loads(text)

                                # Test 1: Connection
                                if req_id == 1:
                                    print("📡 CONNECTION TEST:")
                                    print(text[:150])

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

                                # Test 7: Get Create Metadata
                                elif req_id == 7 and "issue_types" in parsed:
                                    print("📝 GET CREATE METADATA:")
                                    print(f"  Project: {parsed.get('project_name')}")
                                    for it in parsed.get('issue_types', []):
                                        print(f"  Type: {it.get('name')}")
                                        print(f"    Required: {it.get('required_fields', [])[:5]}")

                                else:
                                    print(json.dumps(parsed, indent=2)[:400])

                            except json.JSONDecodeError:
                                print(text[:300])

            elif "error" in data:
                results[req_id] = "ERROR"
                msg = data["error"].get("message", "Unknown error")
                code = data["error"].get("code", "?")

                print(f"\n{'='*60}")
                print(f"❌ TEST {req_id} ERROR")
                print(f"{'='*60}")
                print(f"Code: {code}")
                print(f"Message: {msg}")

                errors.append({"test": req_id, "code": code, "message": msg})

        except json.JSONDecodeError:
            pass

    proc.terminate()
    proc.wait()

    # Print summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)

    success_count = sum(1 for r in results.values() if r == "SUCCESS")
    error_count = sum(1 for r in results.values() if r == "ERROR")

    print(f"✅ Successful: {success_count}")
    print(f"❌ Failed: {error_count}")
    print(f"📊 Total: {len(results)}")

    if errors:
        print("\n🔴 ERRORS FOUND:")
        for err in errors:
            print(f"  Test {err['test']}: {err['message']}")

    return len(errors) == 0

if __name__ == "__main__":
    success = test_mcp_server()
    exit(0 if success else 1)
