# Publishing to npm

This guide explains how to publish your MCP server to npm for easy distribution.

## Prerequisites

1. **npm account**: Create one at [npmjs.com](https://www.npmjs.com)
2. **GitHub repository**: Your code should be pushed to GitHub
3. **GitHub Actions secrets**: Add `NPM_TOKEN` to your repository secrets

## Initial Setup

### 1. Update Package Information

Edit `package.json`:
```json
{
  "name": "@yourusername/your-mcp-server",
  "version": "0.1.0",
  "author": "Your Name <your.email@example.com>",
  "repository": {
    "url": "https://github.com/yourusername/your-mcp-server.git"
  }
}
```

### 2. Get npm Token

```bash
npm login
npm token create --read-only=false
```

Copy the token and add it to GitHub:
1. Go to Settings → Secrets and variables → Actions
2. Add new secret named `NPM_TOKEN`
3. Paste your npm token

## Manual Publishing

### Local Build and Test

```bash
# Build the Rust binary
cargo build --release

# Test with npm structure
cd npm
npm install
npm link
npx template-mcp-server

# Test locally
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | npx template-mcp-server
```

### Publish to npm

```bash
# Update version in npm/package.json
cd npm
npm version patch  # or minor/major

# Publish main package
npm publish --access public

# Platform packages are published automatically via CI
```

## Automated Publishing with GitHub Actions

### Using Workflow Dispatch

1. Go to Actions tab in your GitHub repository
2. Select "NPM Publish" workflow
3. Click "Run workflow"
4. Enter version number (e.g., "0.1.0")
5. Click "Run workflow"

This will:
- Build binaries for all platforms
- Create a GitHub release with binaries
- Publish the main package to npm
- Publish platform-specific packages

### Using GitHub Releases

1. Create a new release on GitHub
2. Tag it with version (e.g., "v0.1.0")
3. The workflow will automatically trigger

## Platform Support

The npm package supports:
- **macOS**: x64, arm64
- **Linux**: x64, arm64
- **Windows**: x64

Users will automatically get the correct binary for their platform.

## Package Structure on npm

Your package will be available as:
```bash
# Main package (includes postinstall script)
npm install @yourusername/your-mcp-server

# Platform-specific packages (optional)
npm install @yourusername/your-mcp-server-darwin-x64
npm install @yourusername/your-mcp-server-linux-x64
# etc.
```

## Testing npm Package

### Before Publishing

```bash
# Pack the package locally
npm pack

# Install and test
npm install -g ./your-package-0.1.0.tgz
npx your-mcp-server
```

### After Publishing

```bash
# Install from npm
npm install -g @yourusername/your-mcp-server

# Test with MCP Inspector
npx @modelcontextprotocol/inspector your-mcp-server

# Use with Claude Desktop
# Add to config:
# "command": "npx"
# "args": ["@yourusername/your-mcp-server"]
```

## Version Management

### Semantic Versioning

- **Patch** (0.0.X): Bug fixes, minor updates
- **Minor** (0.X.0): New features, backward compatible
- **Major** (X.0.0): Breaking changes

### Update Version

```bash
# Patch release (0.1.0 → 0.1.1)
npm version patch

# Minor release (0.1.0 → 0.2.0)
npm version minor

# Major release (0.1.0 → 1.0.0)
npm version major
```

## Troubleshooting

### Build Failures

```bash
# Clean and rebuild
cargo clean
rm -rf target/
cargo build --release
```

### npm Publish Errors

```bash
# Check authentication
npm whoami

# Check package name availability
npm view @yourusername/your-mcp-server

# Dry run
npm publish --dry-run
```

### Binary Not Found

The postinstall script downloads platform binaries from GitHub releases.
Ensure:
1. GitHub release exists with correct binaries
2. Binary names match platform expectations
3. Release is public (not draft)

## Distribution Strategies

### 1. Simple npm Package
- Single package with postinstall script
- Downloads binary from GitHub on install
- Best for most users

### 2. Platform-Specific Packages
- Separate packages per platform
- No postinstall needed
- Larger total size but faster install

### 3. WebAssembly (Future)
- Compile to WASM
- Universal compatibility
- Some performance tradeoff

## Best Practices

1. **Test all platforms** before publishing
2. **Use GitHub Actions** for consistent builds
3. **Document installation** clearly in README
4. **Version carefully** - npm doesn't allow unpublishing easily
5. **Include examples** of client configuration
6. **Test with real MCP clients** before release

## Maintenance

### Updating Dependencies

```bash
# Update Rust dependencies
cargo update

# Update npm dependencies
npm update

# Check for outdated
npm outdated
```

### Security

```bash
# Audit npm packages
npm audit

# Audit Rust dependencies
cargo audit
```

### Deprecation

If needed:
```bash
npm deprecate @yourusername/your-mcp-server@"< 1.0.0" "Please upgrade to v1.0.0"
```

## Support

- [npm Documentation](https://docs.npmjs.com)
- [GitHub Actions Documentation](https://docs.github.com/actions)
- [MCP Specification](https://modelcontextprotocol.io)