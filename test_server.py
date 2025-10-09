#!/usr/bin/env python3
"""
Test script for JIRA MCP Server
Tests all new features: custom fields, checkpointing, and ID stability
"""

import json
import subprocess
import sys
from typing import Any, Dict


class MCPClient:
    def __init__(self, server_path: str):
        import os
        # Pass current environment to subprocess
        env = os.environ.copy()
        self.process = subprocess.Popen(
            [server_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env
        )
        self.request_id = 0
        self._initialize()

    def _initialize(self):
        """Initialize the MCP connection"""
        init_request = {
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "jira-test-client",
                    "version": "1.0.0"
                }
            }
        }

        print(">>> Initializing MCP connection...")
        self.process.stdin.write(json.dumps(init_request) + "\n")
        self.process.stdin.flush()

        # Read initialize response
        response_str = self.process.stdout.readline()
        if not response_str:
            # Check stderr for errors
            import select
            import sys
            if select.select([self.process.stderr], [], [], 0.1)[0]:
                stderr = self.process.stderr.read()
                raise Exception(f"Server error during init: {stderr}")
            raise Exception("No response from server during initialization")

        response = json.loads(response_str)
        if "error" in response:
            raise Exception(f"Initialize failed: {response['error']}")

        print(f"<<< Server initialized: {response.get('result', {}).get('serverInfo', {}).get('name', 'Unknown')}")

        # Send initialized notification
        initialized = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        self.process.stdin.write(json.dumps(initialized) + "\n")
        self.process.stdin.flush()

    def send_request(self, method: str, params: Dict[str, Any] = None) -> Dict[str, Any]:
        """Send a JSON-RPC request to the MCP server"""
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params or {}
        }

        # Send request
        request_str = json.dumps(request) + "\n"
        print(f"\n>>> Sending: {method}")
        print(f"    Params: {json.dumps(params, indent=2) if params else '{}'}")

        self.process.stdin.write(request_str)
        self.process.stdin.flush()

        # Read response
        response_str = self.process.stdout.readline()
        if not response_str:
            stderr = self.process.stderr.read()
            raise Exception(f"No response from server. Stderr: {stderr}")

        response = json.loads(response_str)

        if "error" in response:
            print(f"<<< Error: {response['error']}")
            return response

        print(f"<<< Success")
        return response

    def close(self):
        """Close the connection to the server"""
        self.process.stdin.close()
        self.process.wait(timeout=5)


def test_connection(client: MCPClient):
    """Test 1: Connection and Authentication"""
    print("\n" + "="*60)
    print("TEST 1: Connection and Authentication")
    print("="*60)

    result = client.send_request("tools/call", {
        "name": "test_connection",
        "arguments": {}
    })

    if "result" in result:
        print(f"✅ Connection successful")
        content = result["result"].get("content", [])
        for item in content:
            if item.get("type") == "text":
                print(f"   {item.get('text', '')}")
    else:
        print(f"❌ Connection failed")

    return result


def test_get_custom_fields(client: MCPClient, issue_key: str):
    """Test 2: Get Custom Fields"""
    print("\n" + "="*60)
    print(f"TEST 2: Get Custom Fields from {issue_key}")
    print("="*60)

    result = client.send_request("tools/call", {
        "name": "get_custom_fields",
        "arguments": {"issue_key": issue_key}
    })

    if "result" in result:
        print(f"✅ Retrieved custom fields")
        content = result["result"].get("content", [])
        for item in content:
            if item.get("type") == "text":
                data = json.loads(item.get("text", "{}"))
                print(f"   Total fields: {data.get('total_count', 0)}")

                # Show detected mappings
                mappings = data.get('detected_mappings', {})
                print(f"   Detected mappings:")
                for key, value in mappings.items():
                    if value:
                        print(f"     - {key}: {value}")

                # Show first 3 fields
                fields = data.get('custom_fields', [])[:3]
                print(f"   First 3 fields:")
                for field in fields:
                    print(f"     - {field['field_id']}: {field['value_display']} ({field['field_type']})")
    else:
        print(f"❌ Failed to get custom fields")

    return result


def test_todo_operations(client: MCPClient, issue_key: str):
    """Test 3: Todo Operations with ID Stability"""
    print("\n" + "="*60)
    print(f"TEST 3: Todo Operations and ID Stability on {issue_key}")
    print("="*60)

    # Add a test todo
    print("\n>>> Step 1: Add test todo")
    result = client.send_request("tools/call", {
        "name": "add_todo",
        "arguments": {
            "issue_key": issue_key,
            "todo_text": "🧪 Test checkpoint feature - DO NOT COMPLETE MANUALLY"
        }
    })

    if "error" in result:
        print(f"❌ Failed to add todo: {result['error']}")
        return result

    print(f"✅ Added test todo")

    # List todos to get ID
    print("\n>>> Step 2: List todos to get ID")
    result = client.send_request("tools/call", {
        "name": "list_todos",
        "arguments": {"issue_key": issue_key}
    })

    if "error" in result:
        print(f"❌ Failed to list todos")
        return result

    content = result["result"].get("content", [])
    todos_data = None
    for item in content:
        if item.get("type") == "text":
            todos_data = json.loads(item.get("text", "{}"))
            break

    if not todos_data:
        print(f"❌ No todo data found")
        return result

    # Find our test todo
    test_todo = None
    for todo in todos_data.get("todos", []):
        if "🧪 Test checkpoint" in todo.get("text", ""):
            test_todo = todo
            break

    if not test_todo:
        print(f"❌ Test todo not found")
        return result

    todo_id = test_todo["id"]
    print(f"✅ Found test todo with ID: {todo_id}")

    # Start work on the todo
    print("\n>>> Step 3: Start work on todo")
    result = client.send_request("tools/call", {
        "name": "start_todo_work",
        "arguments": {
            "issue_key": issue_key,
            "todo_id_or_index": todo_id
        }
    })

    if "error" in result:
        print(f"❌ Failed to start work: {result['error']}")
        return result

    print(f"✅ Started work tracking")

    # Wait a moment
    import time
    print("   Waiting 3 seconds...")
    time.sleep(3)

    # Checkpoint the work
    print("\n>>> Step 4: Checkpoint work (new feature!)")
    result = client.send_request("tools/call", {
        "name": "checkpoint_todo_work",
        "arguments": {
            "issue_key": issue_key,
            "todo_id_or_index": todo_id,
            "worklog_comment": "Checkpoint: Testing new checkpoint feature"
        }
    })

    if "error" in result:
        print(f"❌ Failed to checkpoint: {result['error']}")
        return result

    content = result["result"].get("content", [])
    for item in content:
        if item.get("type") == "text":
            checkpoint_data = json.loads(item.get("text", "{}"))
            print(f"✅ Checkpoint successful!")
            print(f"   Time logged: {checkpoint_data.get('checkpoint_time_formatted', 'N/A')}")
            print(f"   Total accumulated: {checkpoint_data.get('total_accumulated_seconds', 0)}s")

    # Wait again
    print("   Waiting 2 more seconds...")
    time.sleep(2)

    # Complete the work (tests ID stability - ID should still work!)
    print("\n>>> Step 5: Complete work (test ID stability)")
    result = client.send_request("tools/call", {
        "name": "complete_todo_work",
        "arguments": {
            "issue_key": issue_key,
            "todo_id_or_index": todo_id,
            "mark_completed": True
        }
    })

    if "error" in result:
        print(f"❌ Failed to complete work: {result['error']}")
        # This would have failed before the ID fix!
        print(f"   (This used to fail due to ID instability bug)")
        return result

    content = result["result"].get("content", [])
    for item in content:
        if item.get("type") == "text":
            complete_data = json.loads(item.get("text", "{}"))
            print(f"✅ Work completed successfully!")
            print(f"   Total time: {complete_data.get('time_spent_formatted', 'N/A')}")
            print(f"   Total seconds: {complete_data.get('time_spent_seconds', 0)}s")
            print(f"   (Includes checkpoint + final segment)")

    return result


def load_env_file(filepath: str = ".env"):
    """Load environment variables from .env file"""
    import os
    if not os.path.exists(filepath):
        print(f"Warning: {filepath} not found")
        return

    with open(filepath) as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith('#'):
                key, value = line.split('=', 1)
                os.environ[key] = value
    print(f"✅ Loaded environment from {filepath}")


def find_test_issue(client: MCPClient) -> str:
    """Find an existing issue to test with"""
    print("\n🔍 Searching for a test issue...")

    result = client.send_request("tools/call", {
        "name": "search_issues",
        "arguments": {
            "assigned_to": "me",
            "limit": 1
        }
    })

    if "result" in result:
        content = result["result"].get("content", [])
        print(f"   Content items: {len(content)}")
        for item in content:
            print(f"   Item type: {item.get('type')}")
            if item.get("type") == "text":
                text_content = item.get("text", "{}")
                print(f"   Text (first 200 chars): {text_content[:200]}")
                try:
                    data = json.loads(text_content)
                    issues = data.get("issues", [])
                    if issues:
                        issue_key = issues[0]["key"]
                        print(f"✅ Found test issue: {issue_key}")
                        return issue_key
                except json.JSONDecodeError as e:
                    print(f"   JSON decode error: {e}")
                    print(f"   Raw text: {text_content}")

    raise Exception("Could not find any issues to test with")


def main():
    """Run all tests"""
    # Load environment variables
    load_env_file(".env")

    server_path = "./target/release/jira-mcp-server"

    # Allow override via command line or find automatically
    import sys
    if len(sys.argv) > 1:
        test_issue_key = sys.argv[1]
        print(f"\n🚀 Using provided issue key: {test_issue_key}")
    else:
        # Find a test issue automatically
        print("\n🚀 Starting JIRA MCP Server Tests")
        print(f"   Server: {server_path}")

        # Initialize client first to find an issue
        client = MCPClient(server_path)
        try:
            test_issue_key = find_test_issue(client)
        except Exception as e:
            print(f"❌ Failed to find test issue: {e}")
            client.close()
            return

        print(f"   Test Issue: {test_issue_key}")

        # Now run the actual tests with this issue
        test_connection(client)
        test_get_custom_fields(client, test_issue_key)
        test_todo_operations(client, test_issue_key)

        print("\n" + "="*60)
        print("✅ ALL TESTS COMPLETED!")
        print("="*60)

        client.close()
        return

    print(f"\n🚀 Starting JIRA MCP Server Tests")
    print(f"   Server: {server_path}")
    print(f"   Test Issue: {test_issue_key}")

    try:
        client = MCPClient(server_path)

        # Run tests
        test_connection(client)
        test_get_custom_fields(client, test_issue_key)
        test_todo_operations(client, test_issue_key)

        print("\n" + "="*60)
        print("✅ ALL TESTS COMPLETED!")
        print("="*60)

        client.close()

    except Exception as e:
        print(f"\n❌ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
