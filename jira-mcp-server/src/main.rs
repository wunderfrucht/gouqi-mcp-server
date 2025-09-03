//! JIRA MCP Server - AI-friendly JIRA integration via MCP
//!
//! This server provides semantic tools for interacting with JIRA without
//! requiring knowledge of JQL or JIRA internals.

use jira_mcp_server::JiraMcpServer;
use pulseengine_mcp_server::McpServerBuilder;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure logging for STDIO transport
    JiraMcpServer::configure_stdio_logging();

    info!("Starting JIRA MCP Server...");

    // Create the JIRA MCP server instance
    let jira_server = match JiraMcpServer::new().await {
        Ok(server) => {
            info!("JIRA MCP Server created successfully");
            server
        }
        Err(e) => {
            error!("Failed to create JIRA MCP Server: {}", e);
            eprintln!("❌ Failed to start JIRA MCP Server: {}", e);
            eprintln!("\nPlease check:");
            eprintln!("  - JIRA_URL environment variable is set");
            eprintln!("  - JIRA authentication is configured (JIRA_AUTH_TYPE, JIRA_TOKEN, etc.)");
            eprintln!("  - JIRA instance is accessible");
            eprintln!("\nFor help, see the README.md file.");
            std::process::exit(1);
        }
    };

    info!("Starting MCP server with STDIO transport...");

    // Start the server using the macro-generated infrastructure
    let mut server = jira_server.serve_stdio().await?;

    info!("🚀 JIRA MCP Server is running and ready to serve requests");

    server.run().await?;

    Ok(())
}
