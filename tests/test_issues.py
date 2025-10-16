"""
Tests for issue CRUD operations.

Tests include:
- get_issue_details
- create_issue
- update_issue (if implemented)
- get_create_metadata
"""

import pytest


@pytest.mark.asyncio
async def test_get_issue_details(mcp_session, test_issue_key):
    """Test get_issue_details for a known issue."""
    result = await mcp_session.call_tool(
        "get_issue_details", {"issue_key": test_issue_key}
    )

    assert result is not None
    assert "content" in result

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify issue_info structure
    assert "issue_info" in data
    issue = data["issue_info"]

    # Verify key fields
    assert "key" in issue
    assert issue["key"] == test_issue_key
    assert "id" in issue
    assert "summary" in issue
    assert "status" in issue
    assert "issue_type" in issue


@pytest.mark.asyncio
async def test_get_issue_details_with_comments(mcp_session, test_issue_key):
    """Test get_issue_details with include_comments flag."""
    result = await mcp_session.call_tool(
        "get_issue_details", {"issue_key": test_issue_key, "include_comments": True}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Should include comments (even if empty array)
    # The structure depends on implementation
    assert "issue_info" in data


@pytest.mark.asyncio
async def test_get_issue_details_with_relationships(mcp_session, test_issue_key):
    """Test get_issue_details with include_relationships flag."""
    result = await mcp_session.call_tool(
        "get_issue_details",
        {"issue_key": test_issue_key, "include_relationships": True},
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issue_info" in data
    # Relationships might be empty, but structure should be there


@pytest.mark.asyncio
async def test_get_issue_details_invalid_key(mcp_session):
    """Test get_issue_details with invalid issue key."""
    # This should raise an error or return an error response
    with pytest.raises(Exception) as exc_info:
        await mcp_session.call_tool(
            "get_issue_details", {"issue_key": "INVALID-999999"}
        )

    # Verify it's an appropriate error
    assert "INVALID-999999" in str(exc_info.value) or "not found" in str(
        exc_info.value
    ).lower()


@pytest.mark.asyncio
async def test_get_create_metadata(mcp_session, test_project_key):
    """Test get_create_metadata for a project."""
    result = await mcp_session.call_tool(
        "get_create_metadata", {"project_key": test_project_key}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify metadata structure
    assert "project_name" in data
    assert "issue_types" in data
    assert isinstance(data["issue_types"], list)

    # Verify issue type metadata
    if len(data["issue_types"]) > 0:
        issue_type = data["issue_types"][0]
        assert "name" in issue_type
        assert "required_fields" in issue_type
        assert isinstance(issue_type["required_fields"], list)


@pytest.mark.asyncio
async def test_get_create_metadata_with_issue_type(mcp_session, test_project_key):
    """Test get_create_metadata filtered by issue type."""
    result = await mcp_session.call_tool(
        "get_create_metadata", {"project_key": test_project_key, "issue_type": "Task"}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issue_types" in data
    # If filtered by type, should have fewer results or specific type
    issue_types = data["issue_types"]
    if len(issue_types) > 0:
        # At least one should be Task
        task_types = [it for it in issue_types if it["name"] == "Task"]
        assert len(task_types) > 0


@pytest.mark.asyncio
async def test_create_issue_basic(mcp_session, test_project_key):
    """Test creating a basic issue."""
    result = await mcp_session.call_tool(
        "create_issue",
        {
            "project_key": test_project_key,
            "summary": f"🧪 Test issue from pytest-mcp - DELETE ME",
            "description": "This is an automated test issue created by pytest-mcp",
            "labels": ["automated-test", "pytest"],
        },
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify issue was created
    assert "issue_key" in data
    assert "issue_id" in data
    assert "issue_url" in data

    # Verify the key is from the correct project
    assert data["issue_key"].startswith(test_project_key)

    # Store for potential cleanup
    created_issue_key = data["issue_key"]
    print(f"\n✅ Created test issue: {created_issue_key}")


@pytest.mark.asyncio
async def test_create_issue_with_assignee(mcp_session, test_project_key):
    """Test creating an issue and assigning to current user."""
    result = await mcp_session.call_tool(
        "create_issue",
        {
            "project_key": test_project_key,
            "summary": f"🧪 Test assigned issue - DELETE ME",
            "description": "Test issue assigned to me",
            "assign_to_me": True,
            "labels": ["automated-test"],
        },
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issue_key" in data
    assert data["issue_key"].startswith(test_project_key)

    print(f"\n✅ Created assigned test issue: {data['issue_key']}")


@pytest.mark.asyncio
async def test_create_issue_with_todos(mcp_session, test_project_key):
    """Test creating an issue with initial todos."""
    result = await mcp_session.call_tool(
        "create_issue",
        {
            "project_key": test_project_key,
            "summary": f"🧪 Test issue with todos - DELETE ME",
            "description": "Test issue with initial checklist",
            "initial_todos": [
                "Verify issue creation",
                "Check todos are created",
                "Delete this test issue",
            ],
            "labels": ["automated-test"],
        },
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issue_key" in data
    # Verify todos info if returned
    if "todos_created" in data:
        assert data["todos_created"] == 3

    print(f"\n✅ Created test issue with todos: {data['issue_key']}")
