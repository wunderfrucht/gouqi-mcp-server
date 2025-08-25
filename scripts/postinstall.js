#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const { pipeline } = require('stream');
const { promisify } = require('util');
const streamPipeline = promisify(pipeline);

const GITHUB_RELEASES_URL = 'https://api.github.com/repos/yourusername/template-mcp-server/releases/latest';

async function downloadBinary() {
  // Only download in production installs, not during development
  if (process.env.NODE_ENV === 'development' || fs.existsSync(path.join(__dirname, '..', 'Cargo.toml'))) {
    console.log('Development environment detected, skipping binary download');
    return;
  }

  const platform = process.platform;
  const arch = process.arch;
  
  // Map to expected binary names in GitHub releases
  const binaryMap = {
    'darwin-x64': 'template-mcp-server-darwin-x64',
    'darwin-arm64': 'template-mcp-server-darwin-arm64',
    'linux-x64': 'template-mcp-server-linux-x64',
    'linux-arm64': 'template-mcp-server-linux-arm64',
    'win32-x64': 'template-mcp-server-windows-x64.exe',
  };

  const binaryName = binaryMap[`${platform}-${arch}`];
  if (!binaryName) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    console.error('You may need to build from source');
    process.exit(1);
  }

  console.log(`Downloading binary for ${platform}-${arch}...`);

  try {
    // Get latest release info
    const releaseInfo = await fetchJson(GITHUB_RELEASES_URL);
    
    // Find the asset URL for our platform
    const asset = releaseInfo.assets.find(a => a.name === binaryName);
    if (!asset) {
      throw new Error(`No binary found for ${platform}-${arch}`);
    }

    // Download the binary
    const distDir = path.join(__dirname, '..', 'dist');
    if (!fs.existsSync(distDir)) {
      fs.mkdirSync(distDir, { recursive: true });
    }

    const localBinaryName = platform === 'win32' ? 'template-mcp-server.exe' : 'template-mcp-server';
    const binaryPath = path.join(distDir, localBinaryName);

    await downloadFile(asset.browser_download_url, binaryPath);
    fs.chmodSync(binaryPath, 0o755);

    console.log('Binary downloaded successfully');
  } catch (error) {
    console.error('Failed to download binary:', error.message);
    console.error('You may need to build from source using "cargo build --release"');
    // Don't exit with error - allow npm install to complete
  }
}

function fetchJson(url) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { 'User-Agent': 'template-mcp-server' } }, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          reject(e);
        }
      });
    }).on('error', reject);
  });
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, {
      headers: { 'User-Agent': 'template-mcp-server' },
      followRedirect: true,
    }, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        downloadFile(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      
      response.pipe(file);
      file.on('finish', () => {
        file.close(resolve);
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

// Run if this is a postinstall script
if (require.main === module) {
  downloadBinary().catch(console.error);
}