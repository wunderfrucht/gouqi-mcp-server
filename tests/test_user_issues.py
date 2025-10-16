"""
Tests for get_user_issues tool.

Tests the user issue query functionality including:
- Get all user issues (default to current user)
- Get user issues with status filter
- Get user issues with project filter
- Get user issues with issue type filter
- Verify JQL syntax is correct (bug #19 fix)
"""

import pytest


@pytest.mark.asyncio
async def test_get_user_issues_default(mcp_session):
    """
    Test get_user_issues with no parameters.

    Should return issues assigned to the current user.
    After bug fix #19, this should work correctly.
    """
    result = await mcp_session.call_tool("get_user_issues", {})

    assert result is not None
    assert "content" in result

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify response structure
    assert "search_result" in data
    assert "resolved_user" in data
    assert "jql_query" in data

    # Verify JQL does NOT have invalid syntax (bug #19)
    jql = data["jql_query"]
    # ORDER BY should NOT be joined with AND
    assert " AND ORDER BY" not in jql
    # Should have proper format: conditions ORDER BY ...
    assert "ORDER BY updated DESC" in jql

    # Verify user info
    user_info = data["resolved_user"]
    assert "account_id" in user_info
    assert "display_name" in user_info
    assert user_info["is_current_user"] is True

    # Verify search results
    search_result = data["search_result"]
    assert "issues" in search_result
    assert "total" in search_result
    assert isinstance(search_result["issues"], list)


@pytest.mark.asyncio
async def test_get_user_issues_with_status_filter(mcp_session):
    """Test get_user_issues with status filter."""
    result = await mcp_session.call_tool(
        "get_user_issues", {"status_filter": ["To Do", "In Progress"]}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify JQL syntax is correct
    jql = data["jql_query"]
    assert " AND ORDER BY" not in jql
    assert "ORDER BY updated DESC" in jql

    # Verify status filter is applied
    assert "status" in jql.lower()

    # Verify applied filters
    assert "applied_filters" in data
    filters = data["applied_filters"]
    assert "status_categories" in filters or filters.get("status_categories") is not None


@pytest.mark.asyncio
async def test_get_user_issues_with_project_filter(mcp_session, test_project_key):
    """Test get_user_issues with project filter."""
    result = await mcp_session.call_tool(
        "get_user_issues", {"project_filter": [test_project_key]}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify JQL syntax
    jql = data["jql_query"]
    assert " AND ORDER BY" not in jql
    assert test_project_key in jql

    # Verify all returned issues are from the specified project
    issues = data["search_result"]["issues"]
    for issue in issues:
        assert issue["key"].startswith(test_project_key)


@pytest.mark.asyncio
async def test_get_user_issues_with_issue_types(mcp_session):
    """Test get_user_issues with issue type filter."""
    result = await mcp_session.call_tool(
        "get_user_issues", {"issue_types": ["Task", "Story"]}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify JQL syntax
    jql = data["jql_query"]
    assert " AND ORDER BY" not in jql
    assert "issuetype" in jql.lower()


@pytest.mark.asyncio
async def test_get_user_issues_with_priority_filter(mcp_session):
    """Test get_user_issues with priority filter."""
    result = await mcp_session.call_tool(
        "get_user_issues", {"priority_filter": ["High", "Critical"]}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify JQL syntax
    jql = data["jql_query"]
    assert " AND ORDER BY" not in jql
    assert "priority" in jql.lower()


@pytest.mark.asyncio
async def test_get_user_issues_with_due_date_filter(mcp_session):
    """Test get_user_issues with due date filter."""
    result = await mcp_session.call_tool(
        "get_user_issues", {"due_date_filter": "this_week"}
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify JQL syntax
    jql = data["jql_query"]
    assert " AND ORDER BY" not in jql
    assert "due" in jql.lower()


@pytest.mark.asyncio
async def test_get_user_issues_pagination(mcp_session):
    """Test get_user_issues with pagination."""
    # Get first page
    result1 = await mcp_session.call_tool("get_user_issues", {"limit": 2, "start_at": 0})

    # Get second page
    result2 = await mcp_session.call_tool("get_user_issues", {"limit": 2, "start_at": 2})

    import json

    data1 = json.loads(result1["content"][0]["text"])
    data2 = json.loads(result2["content"][0]["text"])

    # Verify both pages have valid structure
    assert "search_result" in data1
    assert "search_result" in data2

    # If there are enough issues, pages should be different
    total = data1["search_result"]["total"]
    if total > 2:
        issues1 = {issue["key"] for issue in data1["search_result"]["issues"]}
        issues2 = {issue["key"] for issue in data2["search_result"]["issues"]}
        assert issues1 != issues2


@pytest.mark.asyncio
async def test_get_user_issues_combined_filters(mcp_session, test_project_key):
    """Test get_user_issues with multiple filters combined."""
    result = await mcp_session.call_tool(
        "get_user_issues",
        {
            "project_filter": [test_project_key],
            "status_filter": ["To Do", "In Progress"],
            "issue_types": ["Task"],
            "limit": 5,
        },
    )

    assert result is not None

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify JQL syntax with multiple filters
    jql = data["jql_query"]
    assert " AND ORDER BY" not in jql

    # All filters should be joined with AND, then ORDER BY at the end
    assert "assignee" in jql
    assert "ORDER BY updated DESC" in jql

    # Multiple filters should be present
    filter_count = jql.count(" AND ")
    assert filter_count >= 2  # At least project and status


@pytest.mark.asyncio
async def test_get_user_issues_performance_metrics(mcp_session):
    """Test that get_user_issues includes performance metrics."""
    result = await mcp_session.call_tool("get_user_issues", {"limit": 5})

    import json

    content = result["content"][0]["text"]
    data = json.loads(content)

    # Verify performance metrics
    assert "performance" in data
    perf = data["performance"]
    assert "duration_ms" in perf
    assert "api_calls" in perf
    assert "user_cache_hit" in perf
    assert "query_complexity" in perf
    assert isinstance(perf["duration_ms"], (int, float))
    assert perf["duration_ms"] > 0
