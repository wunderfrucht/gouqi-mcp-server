# Pull Request

## Description
Brief description of changes made in this PR.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)
- [ ] Performance improvement

## Changes Made
- Change 1
- Change 2
- Change 3

## Testing
- [ ] I have tested these changes locally
- [ ] I have tested with MCP Inspector
- [ ] I have tested the tools/list endpoint
- [ ] I have tested tool execution

## Test Commands
```bash
# List tools
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | ./target/debug/your-server

# Test specific tool
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tool_name","arguments":{}}}' | ./target/debug/your-server
```

## Documentation
- [ ] I have updated the README if needed
- [ ] I have added/updated code comments
- [ ] I have updated tool descriptions

## Checklist
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my own code
- [ ] My changes generate no new warnings
- [ ] Any dependent changes have been merged and published