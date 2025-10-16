"""
Tests for search_issues tool.

Tests the search functionality including:
- Search with no parameters (should work with default 30-day filter)
- Search with project filter
- Search with status filter
- Search with date filters
- Search pagination
"""

import pytest


@pytest.mark.asyncio
async def test_search_issues_no_params(mcp_session):
    """
    Test search_issues with no parameters.

    After bug fix #18, this should work with a default 30-day constraint.
    """
    result = await mcp_session.call_tool("search_issues", {"limit": 5})

    # Verify response structure
    assert result is not None
    assert "content" in result

    # Parse the JSON response
    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify we got issues
    assert "issues" in data
    assert "total" in data
    assert isinstance(data["issues"], list)
    assert len(data["issues"]) <= 5

    # Verify JQL query includes the 30-day default
    assert "jql_query" in data
    assert "created >= -30d" in data["jql_query"]


@pytest.mark.asyncio
async def test_search_issues_with_project(mcp_session, test_project_key):
    """Test search_issues with project filter."""
    result = await mcp_session.call_tool(
        "search_issues", {"project_key": test_project_key, "limit": 10}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issues" in data
    assert "total" in data
    assert len(data["issues"]) <= 10

    # Verify project filter is in JQL
    assert "jql_query" in data
    assert test_project_key in data["jql_query"]

    # Verify all issues belong to the project
    for issue in data["issues"]:
        assert issue["key"].startswith(test_project_key)


@pytest.mark.asyncio
async def test_search_issues_with_status_filter(mcp_session, test_project_key):
    """Test search_issues with status filter."""
    result = await mcp_session.call_tool(
        "search_issues",
        {"project_key": test_project_key, "status": ["To Do", "In Progress"], "limit": 5},
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issues" in data
    assert isinstance(data["issues"], list)

    # Verify status filter is in JQL
    assert "jql_query" in data
    assert "status" in data["jql_query"].lower()


@pytest.mark.asyncio
async def test_search_issues_with_date_filter(mcp_session, test_project_key):
    """Test search_issues with created_after filter."""
    result = await mcp_session.call_tool(
        "search_issues",
        {"project_key": test_project_key, "created_after": "-7d", "limit": 5},
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issues" in data
    assert "jql_query" in data
    # Should include the date filter
    assert "-7d" in data["jql_query"] or "created >=" in data["jql_query"]


@pytest.mark.asyncio
async def test_search_issues_pagination(mcp_session, test_project_key):
    """Test search_issues pagination."""
    # First page
    result1 = await mcp_session.call_tool(
        "search_issues", {"project_key": test_project_key, "limit": 2, "start_at": 0}
    )

    # Second page
    result2 = await mcp_session.call_tool(
        "search_issues", {"project_key": test_project_key, "limit": 2, "start_at": 2}
    )

    import json

    data1 = json.loads(result1["content"][0]["text"])
    data2 = json.loads(result2["content"][0]["text"])

    assert "issues" in data1
    assert "issues" in data2

    # If there are enough issues, pages should be different
    if data1["total"] > 2:
        issue_keys_1 = {issue["key"] for issue in data1["issues"]}
        issue_keys_2 = {issue["key"] for issue in data2["issues"]}
        # Pages should not overlap
        assert issue_keys_1 != issue_keys_2


@pytest.mark.asyncio
async def test_search_issues_with_issue_types(mcp_session, test_project_key):
    """Test search_issues with issue_types filter."""
    result = await mcp_session.call_tool(
        "search_issues",
        {"project_key": test_project_key, "issue_types": ["Task", "Story"], "limit": 5},
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issues" in data
    assert "jql_query" in data
    # JQL should include issue type filter
    assert "issuetype" in data["jql_query"].lower()


@pytest.mark.asyncio
async def test_search_issues_with_labels(mcp_session, test_project_key):
    """Test search_issues with labels filter."""
    result = await mcp_session.call_tool(
        "search_issues", {"project_key": test_project_key, "labels": ["test"], "limit": 5}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "issues" in data
    assert "jql_query" in data


@pytest.mark.asyncio
async def test_search_issues_performance(mcp_session, test_project_key):
    """Test that search_issues includes performance metrics."""
    result = await mcp_session.call_tool(
        "search_issues", {"project_key": test_project_key, "limit": 5}
    )

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify performance metrics are included
    assert "performance" in data
    perf = data["performance"]
    assert "duration_ms" in perf
    assert "api_calls" in perf
    assert isinstance(perf["duration_ms"], (int, float))
    assert perf["duration_ms"] > 0
