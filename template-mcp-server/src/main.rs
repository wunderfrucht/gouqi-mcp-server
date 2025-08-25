//! Template MCP Server - A starting point for building MCP servers
//!
//! This template demonstrates the basic structure for creating an MCP server
//! using the PulseEngine MCP framework with automatic tool discovery.

use template_mcp_server::TemplateMcpServer;
use pulseengine_mcp_server::McpServerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure logging for STDIO transport
    TemplateMcpServer::configure_stdio_logging();

    // Start the server using the macro-generated infrastructure
    let mut server = TemplateMcpServer::with_defaults().serve_stdio().await?;
    server.run().await?;

    Ok(())
}