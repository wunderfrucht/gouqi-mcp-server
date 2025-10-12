/// Common utilities for JIRA MCP Server integration tests
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// MCP Test Client for sending JSON-RPC requests to the server
#[allow(dead_code)]
pub struct McpTestClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

#[allow(dead_code)]
impl McpTestClient {
    /// Create a new test client by spawning the server
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Load environment variables from .env file
        // Tests run from workspace root, so .env is in current directory
        dotenv::from_filename(".env").ok();

        // Get environment variables
        let jira_url = std::env::var("JIRA_URL")?;
        let jira_auth_type = std::env::var("JIRA_AUTH_TYPE")?;
        let jira_username = std::env::var("JIRA_USERNAME")?;
        let jira_password = std::env::var("JIRA_PASSWORD")?;

        // Spawn the server process
        // Try multiple possible locations for the binary
        let binary_path = if std::path::Path::new("target/debug/jira-mcp-server").exists() {
            "target/debug/jira-mcp-server"
        } else if std::path::Path::new("target/release/jira-mcp-server").exists() {
            "target/release/jira-mcp-server"
        } else if std::path::Path::new("../target/debug/jira-mcp-server").exists() {
            "../target/debug/jira-mcp-server"
        } else if std::path::Path::new("../target/release/jira-mcp-server").exists() {
            "../target/release/jira-mcp-server"
        } else {
            // Debug: show current directory
            let cwd = std::env::current_dir().unwrap_or_default();
            return Err(format!("Server binary not found. Current dir: {:?}. Looked in target/debug and target/release", cwd).into());
        };

        let mut child = Command::new(binary_path)
            .env("JIRA_URL", jira_url)
            .env("JIRA_AUTH_TYPE", jira_auth_type)
            .env("JIRA_USERNAME", jira_username)
            .env("JIRA_PASSWORD", jira_password)
            .env("RUST_LOG", "error")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Suppress stderr for cleaner test output
            .spawn()?;

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stdout = BufReader::new(stdout);

        let mut client = Self {
            child,
            stdin,
            stdout,
        };

        // Initialize the session
        client.initialize()?;

        Ok(client)
    }

    /// Initialize the MCP session
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "rust-test-client",
                    "version": "1.0.0"
                }
            }
        });

        self.send_request(&request)?;
        let response = self.read_response()?;

        // Verify initialization was successful
        if response.get("error").is_some() {
            return Err(format!("Initialization failed: {:?}", response["error"]).into());
        }

        Ok(())
    }

    /// Call an MCP tool
    pub fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);

        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        self.send_request(&request)?;
        self.read_response()
    }

    /// Send a JSON-RPC request
    fn send_request(&mut self, request: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let request_str = serde_json::to_string(request)?;
        writeln!(self.stdin, "{}", request_str)?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Read a JSON-RPC response
    fn read_response(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        let mut line = String::new();
        self.stdout.read_line(&mut line)?;

        if line.is_empty() {
            return Err("Server closed connection".into());
        }

        let response: Value = serde_json::from_str(&line)?;
        Ok(response)
    }

    /// Extract the tool result from the response
    pub fn extract_tool_result(response: &Value) -> Result<Value, String> {
        // Debug output
        if std::env::var("RUST_TEST_DEBUG").is_ok() {
            eprintln!(
                "Full response: {}",
                serde_json::to_string_pretty(response).unwrap()
            );
        }

        if let Some(error) = response.get("error") {
            return Err(format!("Tool call failed: {:?}", error));
        }

        let result = response.get("result").ok_or("No result in response")?;

        let content = result
            .get("content")
            .ok_or("No content in result")?
            .as_array()
            .ok_or("Content is not an array")?;

        let text_content = content
            .iter()
            .find(|item| item.get("type") == Some(&Value::String("text".to_string())))
            .ok_or("No text content found")?;

        let text = text_content
            .get("text")
            .ok_or("No text field in content")?
            .as_str()
            .ok_or("Text is not a string")?;

        // Debug output
        if std::env::var("RUST_TEST_DEBUG").is_ok() {
            eprintln!("Tool result text: {}", text);
        }

        let parsed: Value = serde_json::from_str(text).map_err(|e| {
            format!(
                "Failed to parse tool result JSON: {}. Text was: {}",
                e, text
            )
        })?;

        Ok(parsed)
    }
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        // Kill the server process when the client is dropped
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Helper function to get the test project key from environment or use default
#[allow(dead_code)]
pub fn test_project_key() -> String {
    std::env::var("TEST_PROJECT_KEY").unwrap_or_else(|_| "SCRUM".to_string())
}

/// Helper function to get a test issue key
#[allow(dead_code)]
pub fn test_issue_key() -> String {
    std::env::var("TEST_ISSUE_KEY").unwrap_or_else(|_| format!("{}-1", test_project_key()))
}
