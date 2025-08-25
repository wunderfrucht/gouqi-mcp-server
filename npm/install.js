#!/usr/bin/env node

const os = require("os");
const path = require("path");
const fs = require("fs");
const https = require("https");
const { execSync } = require("child_process");

// Check if platform packages are available first
const { getPlatformPackageName } = require("./index.js");

try {
  const platformPackage = getPlatformPackageName();
  try {
    require.resolve(platformPackage);
    console.log(`✅ Platform package ${platformPackage} is already available`);
    process.exit(0);
  } catch (err) {
    console.log(`⚠️ Platform package ${platformPackage} not found, falling back to GitHub release download`);
  }
} catch (err) {
  console.log("⚠️ Platform not supported for platform packages, using GitHub release download");
}

// Fallback to GitHub releases download
function getPlatform() {
  const type = os.type();
  const arch = os.arch();

  let platform;
  let archSuffix;

  // Determine platform
  if (type === "Windows_NT") {
    platform = "pc-windows-msvc";
  } else if (type === "Linux") {
    platform = "unknown-linux-gnu";
  } else if (type === "Darwin") {
    platform = "apple-darwin";
  } else {
    throw new Error(`Unsupported platform: ${type}`);
  }

  // Determine architecture
  if (arch === "x64") {
    archSuffix = "x86_64";
  } else if (arch === "arm64") {
    archSuffix = "aarch64";
  } else {
    throw new Error(`Unsupported architecture: ${arch}. Supported architectures: x64 (Intel), arm64 (Apple Silicon). Please install from source: cargo install --git https://github.com/yourusername/template-mcp-server.git template-mcp-server`);
  }

  return `${archSuffix}-${platform}`;
}

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "template-mcp-server.exe" : "template-mcp-server";
}

function getDownloadUrl() {
  const version = require("./package.json").version;
  const platform = getPlatform();
  
  // Determine archive format based on platform
  const archiveExtension = os.type() === "Windows_NT" ? "zip" : "tar.gz";
  
  // Use GitHub releases for binary distribution
  return `https://github.com/yourusername/template-mcp-server/releases/download/v${version}/template-mcp-server-v${version}-${platform}.${archiveExtension}`;
}

function downloadFile(url, destination) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destination);
    
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Handle redirect
        return downloadFile(response.headers.location, destination)
          .then(resolve)
          .catch(reject);
      }
      
      if (response.statusCode !== 200) {
        reject(new Error(`HTTP ${response.statusCode}: ${response.statusMessage}`));
        return;
      }
      
      response.pipe(file);
      
      file.on('finish', () => {
        file.close();
        resolve();
      });
      
      file.on('error', (err) => {
        fs.unlink(destination, () => {});
        reject(err);
      });
    }).on('error', (err) => {
      reject(err);
    });
  });
}

async function installBinary() {
  const binaryName = getBinaryName();
  const downloadUrl = getDownloadUrl();
  const platform = getPlatform();
  
  console.log(`Platform detected: ${os.type()} ${os.arch()}`);
  console.log(`Target: ${platform}`);
  console.log(`Binary: ${binaryName}`);
  console.log(`Download URL: ${downloadUrl}`);
  
  // Create bin directory
  const binDir = path.join(__dirname, "bin");
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  // Download archive
  const archiveExtension = os.type() === "Windows_NT" ? "zip" : "tar.gz";
  const archivePath = path.join(binDir, `template-mcp-server.${archiveExtension}`);
  
  console.log("📥 Downloading binary...");
  try {
    await downloadFile(downloadUrl, archivePath);
    console.log("✅ Download completed");
  } catch (err) {
    throw new Error(`Download failed: ${err.message}`);
  }
  
  // Extract archive
  console.log("📦 Extracting binary...");
  try {
    if (archiveExtension === "zip") {
      // Extract zip (Windows)
      execSync(`cd "${binDir}" && powershell -command "Expand-Archive -Path '${archivePath}' -DestinationPath '.' -Force"`, { stdio: 'inherit' });
    } else {
      // Extract tar.gz (Unix)
      execSync(`cd "${binDir}" && tar -xzf "${path.basename(archivePath)}"`, { stdio: 'inherit' });
    }
    
    // Clean up archive
    fs.unlinkSync(archivePath);
    
    // Make binary executable on Unix
    if (os.type() !== "Windows_NT") {
      const binaryPath = path.join(binDir, binaryName);
      if (fs.existsSync(binaryPath)) {
        fs.chmodSync(binaryPath, 0o755);
      }
    }
    
    console.log("✅ Binary installed successfully");
  } catch (err) {
    throw new Error(`Extraction failed: ${err.message}`);
  }
}

// Run installation
installBinary().catch(err => {
  console.error("❌ Failed to install template-mcp-server binary:", err.message);
  
  // Provide helpful error message
  console.error("\n📋 Installation failed. You can:");
  console.error("1. Install Rust and build from source:");
  console.error("   git clone https://github.com/yourusername/template-mcp-server.git");
  console.error("   cd template-mcp-server");
  console.error("   cargo install --path template-mcp-server");
  console.error("");
  console.error("2. Download binary manually from:");
  console.error("   https://github.com/yourusername/template-mcp-server/releases");
  
  process.exit(1);
});