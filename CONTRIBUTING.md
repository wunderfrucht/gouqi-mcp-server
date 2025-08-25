# Contributing to Template MCP Server

Thank you for your interest in contributing! This document provides guidelines for contributing to this MCP server project.

## Development Setup

1. **Prerequisites**
   - Rust 1.75.0 or later
   - Git
   - Node.js (for MCP Inspector testing)

2. **Clone and Setup**
   ```bash
   git clone https://github.com/yourusername/your-mcp-server.git
   cd your-mcp-server
   cargo build
   ```

3. **Install MCP Inspector** (for testing)
   ```bash
   npm install -g @modelcontextprotocol/inspector
   ```

## Development Workflow

### Making Changes

1. **Create a branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Follow Rust conventions
   - Add documentation for new tools
   - Update README if needed

3. **Test your changes**
   ```bash
   # Build
   cargo build
   
   # Test tools list
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | ./target/debug/template-mcp-server
   
   # Test with MCP Inspector
   npx @modelcontextprotocol/inspector ./target/debug/template-mcp-server
   ```

4. **Commit and push**
   ```bash
   git add .
   git commit -m "feat: add new awesome tool"
   git push origin feature/your-feature-name
   ```

5. **Create a Pull Request**

### Code Style

- Follow standard Rust formatting (use `cargo fmt`)
- Use meaningful variable and function names
- Add documentation comments for all public tools
- Handle errors appropriately (prefer `anyhow::Result`)

### Tool Development Guidelines

When adding new MCP tools:

1. **Tool Method Signature**
   ```rust
   /// Clear description of what the tool does
   ///
   /// # Parameters
   /// - param1: Description of parameter
   /// - param2: Optional parameter description
   pub async fn your_tool(
       &self,
       required_param: String,
       optional_param: Option<i32>,
   ) -> anyhow::Result<YourReturnType> {
       // Implementation
   }
   ```

2. **Parameter Types**
   - Use standard Rust types (String, i32, f64, bool, Vec<T>)
   - Wrap optional parameters in `Option<T>`
   - Use custom structs for complex parameters

3. **Error Handling**
   - Return `anyhow::Result<T>` for all tools
   - Provide meaningful error messages
   - Use `anyhow::anyhow!("message")` for custom errors

4. **Documentation**
   - Add doc comments with `///`
   - Describe what the tool does
   - Document all parameters
   - Provide usage examples if complex

### Testing

Test all changes with:

1. **Direct JSON-RPC testing**
   ```bash
   # Test tool discovery
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | ./target/debug/template-mcp-server
   
   # Test tool execution
   echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"your_tool","arguments":{"param":"value"}}}' | ./target/debug/template-mcp-server
   ```

2. **MCP Inspector testing**
   ```bash
   npx @modelcontextprotocol/inspector ./target/debug/template-mcp-server
   ```

3. **Unit tests** (if applicable)
   ```bash
   cargo test
   ```

## Commit Message Format

Use conventional commits:
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Adding tests
- `chore:` - Maintenance tasks

Examples:
- `feat: add weather lookup tool`
- `fix: handle empty parameter arrays`
- `docs: update tool usage examples`

## Pull Request Process

1. **Update documentation** if you've added new tools
2. **Test thoroughly** with both direct calls and MCP Inspector
3. **Fill out the PR template** completely
4. **Link any related issues**
5. **Request review** from maintainers

## Getting Help

- **Questions?** Open an [issue](https://github.com/yourusername/your-mcp-server/issues)
- **Bugs?** Use the bug report template
- **Feature ideas?** Use the feature request template

## Code of Conduct

- Be respectful and constructive
- Help others learn and grow
- Focus on the technical merit of contributions
- Welcome newcomers and different perspectives

Thank you for contributing! 🎉