# JIRA MCP Server

An AI-friendly JIRA integration server using the Model Context Protocol (MCP). This server provides semantic tools for searching, retrieving, and interacting with JIRA issues without requiring knowledge of JQL or JIRA internals.

## ✨ Features

- **🤖 AI-Friendly Interface**: Uses semantic parameters instead of JQL
- **🔄 Automatic JIRA Detection**: Leverages gouqi 0.14.0 for Cloud/Server detection
- **⚡ Smart Caching**: Metadata caching with TTL for performance
- **🛠️ Comprehensive Tools**: Search, issue details, user issues
- **🚦 Error Handling**: MCP-compliant error codes and messages
- **🔐 Flexible Authentication**: Supports PAT, Basic, Bearer, and Anonymous auth

## 🚀 Quick Start

### Prerequisites

- Rust 1.75.0 or later
- Access to a JIRA instance (Cloud or Server)
- JIRA authentication credentials

### 1. Configuration

Set up your JIRA connection using environment variables:

```bash
# Required: JIRA instance URL
export JIRA_URL="https://your-company.atlassian.net"

# Required: Authentication
export JIRA_AUTH_TYPE="pat"  # or "basic", "bearer", "anonymous"
export JIRA_TOKEN="your_personal_access_token"

# Optional: Advanced settings
export JIRA_CACHE_TTL="300"        # Cache TTL in seconds (default: 300)
export JIRA_MAX_RESULTS="50"       # Max search results (default: 50, max: 200)
export JIRA_REQUEST_TIMEOUT="30"   # Request timeout in seconds (default: 30)
```

### 2. Build and Run

```bash
# Clone and build
git clone https://github.com/yourusername/gouqi-mcp-server.git
cd gouqi-mcp-server
cargo build --release

# Run the server
./target/release/jira-mcp-server
```

### 3. Test Connection

```bash
# Test tools
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | ./target/debug/jira-mcp-server

# Test connection
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"test_connection","arguments":{}}}' | ./target/debug/jira-mcp-server
```

## 🛠️ Available Tools

### `search_issues`
Search for JIRA issues using AI-friendly semantic parameters.

**Example Usage:**
```json
{
  "issue_types": ["story", "bug"],
  "assigned_to": "me",
  "status": ["open", "in_progress"],
  "project_key": "PROJ",
  "created_after": "7 days ago",
  "limit": 25
}
```

### `get_issue_details`
Get detailed information about a specific JIRA issue.

**Example Usage:**
```json
{
  "issue_key": "PROJ-123",
  "include_comments": true,
  "include_attachments": true
}
```

### `get_user_issues`
Get issues assigned to a specific user with filtering options.

**Example Usage:**
```json
{
  "username": "me",
  "status_filter": ["open", "in_progress"],
  "issue_types": ["story", "bug"],
  "due_date_filter": "overdue"
}
```

### `get_server_status`
Get server status and JIRA connection information.

### `test_connection`
Test JIRA connection and authentication.

### `clear_cache`
Clear all cached metadata.

## 📁 Project Structure

```
jira-mcp-server/
├── Cargo.toml                    # Package configuration
├── src/
│   ├── main.rs                   # Server entry point
│   ├── lib.rs                    # Main server implementation
│   ├── config.rs                 # Configuration management
│   ├── cache.rs                  # Metadata caching
│   ├── jira_client.rs            # JIRA API wrapper
│   ├── semantic_mapping.rs       # AI-friendly parameter mapping
│   ├── error.rs                  # Error types and handling
│   └── tools/                    # MCP tool implementations
│       ├── mod.rs
│       ├── search_issues.rs
│       ├── issue_details.rs
│       └── user_issues.rs
├── config/
│   └── jira-mcp-config.toml.example  # Configuration example
└── README.md
```

## ⚙️ Configuration

### Environment Variables (Recommended)

```bash
# Required
JIRA_URL="https://your-instance.atlassian.net"
JIRA_AUTH_TYPE="pat"  # "pat", "basic", "bearer", "anonymous"
JIRA_TOKEN="your_token"

# For Basic Auth
JIRA_USERNAME="your_username"
JIRA_PASSWORD="your_password"  # pragma: allowlist secret

# Optional
JIRA_CACHE_TTL="300"
JIRA_MAX_RESULTS="50"
JIRA_REQUEST_TIMEOUT="30"
JIRA_RATE_LIMIT="60"
```

### TOML Configuration File (Alternative)

Copy `config/jira-mcp-config.toml.example` to `jira-mcp-config.toml` and customize:

```toml
jira_url = "https://your-company.atlassian.net"
cache_ttl_seconds = 300
max_search_results = 50

[auth]
type = "personal_access_token"
token = "your_token_here"

[issue_type_mappings]
story = ["Story", "User Story"]
bug = ["Bug", "Defect"]
feature = ["Feature", "Enhancement"]
```

## 🔌 Integration with MCP Clients

### Claude Desktop

Add to your MCP configuration:

```json
{
  "servers": {
    "jira": {
      "command": "/path/to/jira-mcp-server",
      "env": {
        "JIRA_URL": "https://your-company.atlassian.net",
        "JIRA_AUTH_TYPE": "pat",
        "JIRA_TOKEN": "your_token"
      }
    }
  }
}
```

### Continue.dev

```json
{
  "mcpServers": {
    "jira": {
      "command": "/path/to/jira-mcp-server",
      "env": {
        "JIRA_URL": "https://your-company.atlassian.net",
        "JIRA_AUTH_TYPE": "pat",
        "JIRA_TOKEN": "your_token"
      }
    }
  }
}
```

## 🎯 Semantic Parameters

The server translates AI-friendly parameters to JIRA concepts:

### Issue Types
- `"story"` → Story, User Story
- `"bug"` → Bug, Defect
- `"feature"` → Feature, Enhancement
- `"task"` → Task, Sub-task
- `"capability"` → Capability, Epic

### Status Categories
- `"open"` → Open, To Do, Backlog, New
- `"in_progress"` → In Progress, In Development, In Review
- `"done"` → Done, Closed, Resolved, Complete
- `"blocked"` → Blocked, On Hold, Waiting

### User References
- `"me"` or `"current_user"` → Authenticated user
- `"unassigned"` → Unassigned issues
- Any username or account ID

## 🔧 Development

### Building
```bash
cargo build
```

### Running with Debug Logs
```bash
RUST_LOG=debug cargo run
```

### Testing with MCP Inspector
```bash
# Install MCP Inspector
npm install -g @modelcontextprotocol/inspector

# Test your server
npx @modelcontextprotocol/inspector ./target/debug/jira-mcp-server
```

### Running Tests
```bash
cargo test
```

## 🌟 Example AI Interactions

**Find my open stories:**
```
AI: "Show me all the stories assigned to me that are currently open or in progress"
→ Uses: search_issues with {"issue_types": ["story"], "assigned_to": "me", "status": ["open", "in_progress"]}
```

**Get issue details:**
```
AI: "What's the current status and description of PROJ-123?"
→ Uses: get_issue_details with {"issue_key": "PROJ-123"}
```

**Find overdue bugs:**
```
AI: "Show me all bugs that are overdue"
→ Uses: search_issues with {"issue_types": ["bug"], "created_after": "30 days ago", "status": ["open", "in_progress"]}
```

## 🚨 Troubleshooting

### Connection Issues
1. Verify JIRA_URL is correct and accessible
2. Check authentication credentials
3. Test with `test_connection` tool
4. Check firewall/network restrictions

### Authentication Issues
- **Jira Cloud**: Use Personal Access Token (PAT)
- **Jira Server**: Use username/password or API token
- Verify token permissions and expiration

### Performance Issues
- Check cache TTL settings
- Monitor API rate limits
- Use more specific search filters
- Consider increasing `JIRA_REQUEST_TIMEOUT`

## 🔒 Security

- Never commit credentials to version control
- Use environment variables for sensitive data
- Rotate tokens regularly
- Use least-privilege access tokens
- Monitor API usage and access logs

## 📊 Monitoring & Debugging

Set log levels for debugging:
```bash
RUST_LOG=debug ./target/debug/jira-mcp-server     # Debug level
RUST_LOG=trace ./target/debug/jira-mcp-server     # Verbose trace level
```

Log output includes:
- Tool invocations and parameters
- JIRA API calls and responses
- Cache operations and hit/miss rates
- Performance timing information
- Error details and stack traces

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## 🆘 Support & Resources

- [Model Context Protocol Specification](https://modelcontextprotocol.io/specification/)
- [Gouqi JIRA Client Documentation](https://docs.rs/gouqi)
- [PulseEngine MCP Framework](https://docs.rs/pulseengine-mcp-protocol)
- [JIRA REST API Documentation](https://developer.atlassian.com/cloud/jira/platform/rest/)
- [GitHub Issues](https://github.com/yourusername/gouqi-mcp-server/issues)

---

**🎉 Happy JIRA automation with AI!**
