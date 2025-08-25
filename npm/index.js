#!/usr/bin/env node

const os = require("os");
const path = require("path");

/**
 * Returns the appropriate platform package name for the current system
 */
function getPlatformPackageName() {
  const platform = os.platform();
  const arch = os.arch();
  
  let platformName;
  let archName;
  
  // Map Node.js platform names to our package naming convention
  switch (platform) {
    case "darwin":
      platformName = "darwin";
      break;
    case "linux":
      platformName = "linux";
      break;
    case "win32":
      platformName = "win32";
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }
  
  // Map Node.js architecture names to our package naming convention
  switch (arch) {
    case "x64":
      archName = "x64";
      break;
    case "arm64":
      archName = "arm64";
      break;
    default:
      throw new Error(`Unsupported architecture: ${arch}`);
  }
  
  return `@yourusername/template-mcp-server-${platformName}-${archName}`;
}

/**
 * Returns the path to the binary for the current platform
 */
function getBinaryPath() {
  const platform = os.platform();
  const binaryName = platform === "win32" ? "template-mcp-server.exe" : "template-mcp-server";
  
  try {
    // Try to get the binary from the platform package
    const platformPackage = getPlatformPackageName();
    const platformPackagePath = require.resolve(platformPackage);
    const platformPackageDir = path.dirname(platformPackagePath);
    return path.join(platformPackageDir, binaryName);
  } catch (err) {
    // Fallback to local bin directory (for GitHub releases download)
    return path.join(__dirname, "bin", binaryName);
  }
}

/**
 * Returns information about the current platform and binary location
 */
function getPlatformInfo() {
  return {
    platform: os.platform(),
    arch: os.arch(),
    platformPackage: getPlatformPackageName(),
    binaryPath: getBinaryPath(),
    binaryName: os.platform() === "win32" ? "template-mcp-server.exe" : "template-mcp-server"
  };
}

module.exports = {
  getPlatformPackageName,
  getBinaryPath,
  getPlatformInfo
};

// If this script is run directly, print platform info
if (require.main === module) {
  try {
    const info = getPlatformInfo();
    console.log("Platform Information:");
    console.log(`  Platform: ${info.platform}`);
    console.log(`  Architecture: ${info.arch}`);
    console.log(`  Platform Package: ${info.platformPackage}`);
    console.log(`  Binary Name: ${info.binaryName}`);
    console.log(`  Binary Path: ${info.binaryPath}`);
  } catch (err) {
    console.error("Error:", err.message);
    process.exit(1);
  }
}