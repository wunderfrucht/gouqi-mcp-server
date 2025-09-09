#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

/**
 * Cleanup downloaded binaries on package uninstall
 */
function cleanupBinaries() {
  const binDir = path.join(__dirname, "bin");

  if (fs.existsSync(binDir)) {
    console.log("🧹 Cleaning up downloaded binaries...");
    try {
      fs.rmSync(binDir, { recursive: true, force: true });
      console.log("✅ Cleanup completed");
    } catch (err) {
      console.error("⚠️ Failed to cleanup binaries:", err.message);
      // Don't fail the uninstall process
    }
  }
}

// Run cleanup
cleanupBinaries();
