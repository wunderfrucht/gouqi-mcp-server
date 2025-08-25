#!/usr/bin/env node

const { spawn } = require("child_process");
const { getBinaryPath } = require("./index.js");
const fs = require("fs");

/**
 * Runs the template-mcp-server binary with the provided arguments
 */
function runServer() {
  try {
    const binaryPath = getBinaryPath();
    
    // Check if binary exists
    if (!fs.existsSync(binaryPath)) {
      console.error("❌ Binary not found at:", binaryPath);
      console.error("");
      console.error("This might happen if:");
      console.error("1. Platform packages failed to install");
      console.error("2. GitHub release download failed");
      console.error("3. Your platform is not supported");
      console.error("");
      console.error("Try running: npm install --force");
      console.error("Or install from source: cargo install --git https://github.com/yourusername/template-mcp-server.git template-mcp-server");
      process.exit(1);
    }
    
    // Spawn the binary with all arguments passed through
    const child = spawn(binaryPath, process.argv.slice(2), {
      stdio: "inherit",
      env: process.env
    });
    
    // Handle child process events
    child.on("error", (err) => {
      console.error("❌ Failed to start template-mcp-server:", err.message);
      process.exit(1);
    });
    
    child.on("exit", (code, signal) => {
      if (signal) {
        // If killed by signal, propagate it
        process.kill(process.pid, signal);
      } else {
        // Exit with the same code
        process.exit(code || 0);
      }
    });
    
    // Forward signals to child process
    process.on("SIGINT", () => child.kill("SIGINT"));
    process.on("SIGTERM", () => child.kill("SIGTERM"));
    
  } catch (err) {
    console.error("❌ Error starting template-mcp-server:", err.message);
    process.exit(1);
  }
}

// Run the server
runServer();