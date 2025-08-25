#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

console.log('Preparing distribution files...');

// Create dist directory
const distDir = path.join(__dirname, '..', 'dist');
if (!fs.existsSync(distDir)) {
  fs.mkdirSync(distDir, { recursive: true });
}

// Detect platform and architecture
const platform = process.platform;
const arch = process.arch;

// Map to Rust target triples
const targetMap = {
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'win32-x64': 'x86_64-pc-windows-msvc',
};

const rustTarget = targetMap[`${platform}-${arch}`];
if (!rustTarget) {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  process.exit(1);
}

// Binary name (with .exe extension on Windows)
const binaryName = platform === 'win32' ? 'template-mcp-server.exe' : 'template-mcp-server';
const sourcePath = path.join(__dirname, '..', 'target', 'release', binaryName);
const destPath = path.join(distDir, binaryName);

// Check if binary exists
if (!fs.existsSync(sourcePath)) {
  console.error(`Binary not found at ${sourcePath}`);
  console.error('Please run "cargo build --release" first');
  process.exit(1);
}

// Copy binary to dist
fs.copyFileSync(sourcePath, destPath);
fs.chmodSync(destPath, 0o755);

console.log(`Copied ${binaryName} to dist/`);

// Create Node.js wrapper
const wrapperContent = `#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

const binaryName = process.platform === 'win32' ? 'template-mcp-server.exe' : 'template-mcp-server';
const binaryPath = path.join(__dirname, binaryName);

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env,
});

child.on('error', (err) => {
  console.error('Failed to start MCP server:', err);
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code);
  }
});

// Forward signals to child process
process.on('SIGINT', () => child.kill('SIGINT'));
process.on('SIGTERM', () => child.kill('SIGTERM'));
`;

fs.writeFileSync(path.join(distDir, 'index.js'), wrapperContent);
fs.chmodSync(path.join(distDir, 'index.js'), 0o755);

console.log('Created Node.js wrapper');

// Create platform-specific package info
const packageInfo = {
  platform,
  arch,
  rustTarget,
  binaryName,
  version: require('../package.json').version,
};

fs.writeFileSync(
  path.join(distDir, 'platform.json'),
  JSON.stringify(packageInfo, null, 2)
);

console.log('Distribution files prepared successfully');