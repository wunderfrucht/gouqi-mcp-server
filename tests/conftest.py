"""
Pytest configuration and fixtures for JIRA MCP Server testing.
"""

import os
import pytest
from pathlib import Path
from dotenv import load_dotenv
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

# Load environment variables from .env file
env_path = Path(__file__).parent.parent / ".env"
load_dotenv(env_path, override=True)


@pytest.fixture(scope="session")
def server_path():
    """Path to the JIRA MCP server binary."""
    repo_root = Path(__file__).parent.parent
    server_binary = repo_root / "target" / "release" / "jira-mcp-server"

    if not server_binary.exists():
        pytest.fail(
            f"Server binary not found at {server_binary}. "
            "Run 'cargo build --release' first."
        )

    return str(server_binary)


@pytest.fixture(scope="session")
def server_env():
    """Environment variables for the MCP server."""
    required_vars = ["JIRA_URL", "JIRA_AUTH_TYPE", "JIRA_USERNAME", "JIRA_PASSWORD"]

    env = {}
    for var in required_vars:
        value = os.getenv(var)
        if not value:
            pytest.fail(f"Required environment variable {var} not set")
        env[var] = value

    # Add RUST_LOG for better debugging
    env["RUST_LOG"] = os.getenv("RUST_LOG", "error")

    return env


@pytest.fixture
async def mcp_session(server_path, server_env):
    """
    Create an MCP client session connected to the JIRA MCP server.

    This fixture:
    - Starts the MCP server as a subprocess
    - Establishes a stdio connection
    - Initializes the session
    - Yields the session for testing
    - Cleans up on teardown
    """
    server_params = StdioServerParameters(
        command=server_path,
        args=[],
        env=server_env,
    )

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            # Initialize the session
            await session.initialize()

            # Yield session to the test
            yield session


@pytest.fixture
async def test_project_key():
    """The JIRA project key to use for testing."""
    return os.getenv("TEST_PROJECT_KEY", "SCRUM")


@pytest.fixture
async def test_issue_key(test_project_key):
    """A known test issue key for detailed testing."""
    return os.getenv("TEST_ISSUE_KEY", f"{test_project_key}-1")
