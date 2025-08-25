---
name: Bug report
about: Create a report to help us improve
title: '[BUG] '
labels: bug
assignees: ''
---

## Bug Description
A clear and concise description of what the bug is.

## To Reproduce
Steps to reproduce the behavior:
1. Build the server with '...'
2. Run command '...'
3. Send request '...'
4. See error

## Expected Behavior
A clear and concise description of what you expected to happen.

## Actual Behavior
What actually happened.

## Error Output
```
Paste any error messages or logs here
```

## Environment
- OS: [e.g. macOS, Ubuntu, Windows]
- Rust version: [e.g. 1.75.0]
- Framework version: [e.g. 0.8.2]
- MCP Client: [e.g. Claude Desktop, MCP Inspector]

## Request/Response Examples
```json
// Request that caused the issue
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{...}}

// Response received
{"jsonrpc":"2.0","id":1,"error":{...}}
```

## Additional Context
Add any other context about the problem here.