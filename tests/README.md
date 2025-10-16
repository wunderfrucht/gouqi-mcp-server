# JIRA MCP Server Tests

Automated test suite for the JIRA MCP Server using pytest-mcp framework.

## Setup

### Prerequisites

1. Build the JIRA MCP server:
   ```bash
   cargo build --release
   ```

2. Configure environment variables in `.env`:
   ```bash
   JIRA_URL=https://your-instance.atlassian.net
   JIRA_AUTH_TYPE=basic
   JIRA_USERNAME=your.email@example.com
   JIRA_PASSWORD=your_api_token

   # Optional test configuration
   TEST_PROJECT_KEY=SCRUM
   TEST_ISSUE_KEY=SCRUM-1
   ```

3. Install Python dependencies:
   ```bash
   pip install -r tests/requirements.txt
   ```

## Running Tests

### Run all tests
```bash
pytest
```

### Run specific test file
```bash
pytest tests/test_search.py
pytest tests/test_user_issues.py
pytest tests/test_issues.py
```

### Run specific test
```bash
pytest tests/test_search.py::test_search_issues_no_params
```

### Run with verbose output
```bash
pytest -v
```

### Run with coverage
```bash
pytest --cov=. --cov-report=html
```

## Test Structure

```
tests/
├── conftest.py          # Pytest fixtures and configuration
├── test_search.py       # Tests for search_issues tool
├── test_user_issues.py  # Tests for get_user_issues tool
├── test_issues.py       # Tests for issue CRUD operations
└── requirements.txt     # Python dependencies
```

## Test Fixtures

### `mcp_session`
Provides an initialized MCP client session connected to the JIRA MCP server.

Example usage:
```python
@pytest.mark.asyncio
async def test_example(mcp_session):
    result = await mcp_session.call_tool("search_issues", {"limit": 5})
    assert result is not None
```

### `test_project_key`
Returns the test project key (default: "SCRUM").

### `test_issue_key`
Returns a known test issue key for detailed testing (default: "SCRUM-1").

## Writing Tests

Example test:
```python
import pytest

@pytest.mark.asyncio
async def test_my_tool(mcp_session):
    """Test description."""
    result = await mcp_session.call_tool("tool_name", {"param": "value"})

    assert result is not None

    import json
    content = result["content"][0]["text"]
    data = json.loads(content)

    assert "expected_field" in data
```

## Continuous Integration

Tests run automatically on:
- Pull requests
- Pushes to main branch

See `.github/workflows/test.yml` for CI configuration.

## Troubleshooting

### Server binary not found
```
cargo build --release
```

### Environment variables not set
Check that `.env` file exists and contains required variables.

### Connection timeouts
Verify JIRA credentials and network connectivity.

### Tests creating issues
Some tests create issues in JIRA with "🧪" emoji and "DELETE ME" in the summary.
These can be safely deleted after test runs.
