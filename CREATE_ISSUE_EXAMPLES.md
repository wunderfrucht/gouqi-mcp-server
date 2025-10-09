# Create Issue Tool - Usage Examples

## Design Philosophy

The `create_issue` tool is designed to be:
- **Simple for basic use** - Just provide summary and project
- **Powerful for advanced use** - Support all JIRA features
- **AI-friendly** - Clear parameter names, smart defaults, good validation
- **Human-friendly** - Intuitive, well-documented, helpful error messages

## Basic Examples

### 1. Simplest Usage - Just Create a Task

```json
{
  "project_key": "PROJ",
  "summary": "Fix login button alignment"
}
```

**Creates:** A Task in project PROJ with the given summary

---

### 2. Task with Description

```json
{
  "project_key": "PROJ",
  "summary": "Implement user profile page",
  "description": "Create a user profile page that displays:\n- User avatar\n- Username and email\n- Edit profile button\n\nShould match the design in Figma."
}
```

---

### 3. Bug Report

```json
{
  "project_key": "PROJ",
  "summary": "Login fails with invalid credentials error",
  "issue_type": "Bug",
  "priority": "High",
  "description": "## Steps to Reproduce\n1. Go to login page\n2. Enter invalid credentials\n3. Click login\n\n## Expected\nShows 'Invalid credentials' message\n\n## Actual\nShows generic error",
  "labels": ["login", "security"]
}
```

---

### 4. Story with Initial Todos

```json
{
  "project_key": "PROJ",
  "summary": "Add dark mode support",
  "issue_type": "Story",
  "initial_todos": [
    "Design dark mode color palette",
    "Implement theme switching logic",
    "Update all components to support dark mode",
    "Add toggle in settings",
    "Test on all supported devices"
  ],
  "assign_to_me": true,
  "labels": ["ui", "enhancement"]
}
```

**Result:** Creates a Story with a formatted todo checklist in the description, assigned to you

---

## Advanced Examples

### 5. Create Subtask

```json
{
  "parent_issue_key": "PROJ-123",
  "summary": "Write unit tests for authentication module",
  "description": "Cover all authentication flows with unit tests",
  "assignee": "john.doe@example.com"
}
```

**Note:** When `parent_issue_key` is provided:
- `project_key` is auto-inferred from parent
- `issue_type` is automatically set to "Subtask"

---

### 6. Epic Creation

```json
{
  "project_key": "PROJ",
  "summary": "Q4 Performance Improvements",
  "issue_type": "Epic",
  "description": "Epic to track all Q4 performance improvement initiatives",
  "priority": "High",
  "assign_to_me": true
}
```

---

### 7. Story with Story Points and Epic Link

```json
{
  "project_key": "PROJ",
  "summary": "Optimize database queries for user dashboard",
  "issue_type": "Story",
  "story_points": 5,
  "epic_link": "PROJ-100",
  "priority": "Medium",
  "labels": ["performance", "backend"],
  "components": ["API", "Database"]
}
```

---

### 8. Issue with Custom Fields

```json
{
  "project_key": "PROJ",
  "summary": "Security audit for payment processing",
  "description": "Conduct comprehensive security audit",
  "custom_fields": {
    "customfield_10050": "Q4 2024",
    "customfield_10051": {"value": "Security Team"},
    "customfield_10052": ["PCI-DSS", "SOC2"]
  },
  "priority": "Highest"
}
```

**Tip:** Use `get_custom_fields` to discover field IDs for your JIRA instance

---

## AI Agent Usage Patterns

### Pattern 1: Feature Development Workflow

```json
// AI creates epic first
{
  "project_key": "DEV",
  "summary": "User Authentication System",
  "issue_type": "Epic",
  "description": "Complete authentication system with login, signup, and password reset"
}

// Then creates stories under the epic
{
  "project_key": "DEV",
  "summary": "Implement login functionality",
  "issue_type": "Story",
  "epic_link": "DEV-100",
  "initial_todos": [
    "Create login API endpoint",
    "Implement JWT token generation",
    "Add login UI components",
    "Write integration tests"
  ],
  "story_points": 8,
  "assign_to_me": true
}
```

---

### Pattern 2: Bug Triage Workflow

```json
{
  "project_key": "BUG",
  "summary": "Memory leak in image upload component",
  "issue_type": "Bug",
  "priority": "High",
  "description": "## Problem\nMemory usage increases continuously when uploading multiple images\n\n## Environment\n- Browser: Chrome 120\n- OS: macOS\n- Version: 2.3.1\n\n## Logs\n```\nMemory: 150MB -> 450MB over 5 uploads\n```",
  "labels": ["memory-leak", "image-upload"],
  "assign_to_me": true,
  "initial_todos": [
    "Reproduce the issue",
    "Profile memory usage",
    "Identify leak source",
    "Implement fix",
    "Add memory usage tests"
  ]
}
```

---

### Pattern 3: Quick Task Creation from Chat

AI can parse natural language and create issues:

**User:** "Create a task to fix the broken search on the homepage, make it high priority"

**AI creates:**
```json
{
  "project_key": "WEB",
  "summary": "Fix broken search on homepage",
  "issue_type": "Task",
  "priority": "High",
  "assign_to_me": true
}
```

---

## Smart Defaults & Conveniences

### 1. Auto-Subtask Detection
```json
{
  "parent_issue_key": "PROJ-123",
  "summary": "Update documentation"
}
```
✅ Automatically sets `issue_type: "Subtask"` and infers project

### 2. Assignee Shortcuts
```json
{
  "assignee": "me"  // or "self"
}
// OR
{
  "assign_to_me": true
}
```
Both automatically assign to the current user

### 3. Initial Todos
```json
{
  "initial_todos": ["Task 1", "Task 2", "Task 3"]
}
```
Automatically formats as markdown checklist and integrates with todo tracker

### 4. Common Custom Fields
```json
{
  "story_points": 5,
  "epic_link": "PROJ-100"
}
```
Tries common field IDs automatically (customfield_10016, customfield_10014)

---

## Error Handling

The tool provides helpful errors:

### Invalid Project
```json
{
  "project_key": "INVALID",
  "summary": "Test"
}
```
**Error:** `Invalid project: INVALID`

### Invalid Issue Type
```json
{
  "project_key": "PROJ",
  "summary": "Test",
  "issue_type": "InvalidType"
}
```
**Error:** `Invalid issue type: InvalidType`

### Missing Required Fields
```json
{
  "summary": "Test"
  // Missing project_key and parent_issue_key
}
```
**Error:** `Either project_key or parent_issue_key must be provided`

---

## Response Format

```json
{
  "issue_key": "PROJ-456",
  "issue_id": "10123",
  "issue_url": "https://your-domain.atlassian.net/browse/PROJ-456",
  "summary": "Fix login button alignment",
  "issue_type": "Task",
  "project_key": "PROJ",
  "message": "Successfully created task 'Fix login button alignment' in project PROJ. View at: https://..."
}
```

---

## Best Practices

### For AI Agents:

1. **Start Simple** - Use minimal parameters first, add complexity as needed
2. **Use initial_todos** - Great for breaking down work immediately
3. **Leverage assign_to_me** - Auto-assign issues you create
4. **Chain Operations** - Create epic → create stories → create subtasks
5. **Include Context** - Put relevant info in description (logs, screenshots, steps)

### For Users:

1. **Be Descriptive** - Clear summaries help everyone
2. **Use Markdown** - Format descriptions with headers, lists, code blocks
3. **Tag Properly** - Use labels and components for organization
4. **Set Priority** - Help others understand urgency
5. **Link Related Issues** - Use epic_link and parent_issue_key

---

## Integration with Other Tools

### Combined Workflow Example:

```javascript
// 1. Create issue with todos
const issue = await create_issue({
  project_key: "PROJ",
  summary: "Implement new feature X",
  initial_todos: [
    "Design API endpoints",
    "Implement backend logic",
    "Create frontend components",
    "Write tests"
  ],
  assign_to_me: true,
  story_points: 13
});

// 2. Start work on first todo
await start_todo_work({
  issue_key: issue.issue_key,
  todo_id_or_index: 1
});

// 3. Checkpoint progress
await checkpoint_todo_work({
  issue_key: issue.issue_key,
  todo_id_or_index: 1,
  worklog_comment: "Completed API design"
});

// 4. Complete and move to next
await complete_todo_work({
  issue_key: issue.issue_key,
  todo_id_or_index: 1,
  mark_completed: true
});
```

---

## Summary

The `create_issue` tool is designed to be:
- ✅ **Easy to use** - Simple params for simple cases
- ✅ **Powerful** - Full JIRA feature support
- ✅ **Smart** - Auto-detection and sensible defaults
- ✅ **Flexible** - Works for bugs, tasks, stories, epics, subtasks
- ✅ **Integrated** - Works seamlessly with todo tracker and other tools
- ✅ **Well-documented** - Clear examples and error messages
