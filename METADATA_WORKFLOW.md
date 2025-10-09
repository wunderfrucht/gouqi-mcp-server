# Issue Creation Workflow with Metadata Discovery

## The Problem

Before creating an issue in JIRA, you need to know:
1. ✅ What issue types are available? (Task, Bug, Story, Epic?)
2. ✅ What fields are required? (Summary, Description, Custom fields?)
3. ✅ What values are allowed? (Priorities, Components, Custom dropdowns?)

## The Solution: `get_create_metadata`

This tool discovers all the information needed to successfully create issues.

---

## Basic Usage

### 1. Discover Available Issue Types

```json
{
  "project_key": "PROJ"
}
```

**Response:**
```json
{
  "project_key": "PROJ",
  "project_name": "My Project",
  "issue_types": [
    {
      "name": "Task",
      "id": "10001",
      "is_subtask": false,
      "required_fields": ["project", "summary", "issuetype"],
      "field_summary": {
        "required_standard": ["project", "summary", "issuetype"],
        "required_custom": [],
        "optional_standard": ["description", "priority", "assignee"],
        "optional_custom": ["Story Points (customfield_10016)"]
      }
    },
    {
      "name": "Bug",
      "id": "10002",
      "is_subtask": false,
      "required_fields": ["project", "summary", "issuetype", "priority"],
      "field_summary": {
        "required_standard": ["project", "summary", "issuetype", "priority"],
        "required_custom": ["Environment (customfield_10050)"],
        "optional_standard": ["description", "assignee"],
        "optional_custom": []
      }
    }
  ],
  "common_required_fields": ["project", "summary", "issuetype"],
  "usage_hints": [
    "Use create_issue tool to create issues in this project",
    "Available issue types: Task, Bug, Story, Epic",
    "Common required fields: project, summary, issuetype",
    "For custom fields, use the field_id in custom_fields parameter"
  ]
}
```

---

### 2. Get Metadata for Specific Issue Type

```json
{
  "project_key": "PROJ",
  "issue_type": "Bug"
}
```

**Response:** Only returns metadata for Bug type

---

### 3. Get Detailed Field Schemas

```json
{
  "project_key": "PROJ",
  "issue_type": "Bug",
  "include_schemas": true
}
```

**Response includes:**
```json
{
  "issue_types": [{
    "all_fields": [
      {
        "field_id": "priority",
        "name": "Priority",
        "required": true,
        "field_type": "priority",
        "allowed_values": ["Highest", "High", "Medium", "Low", "Lowest"],
        "schema": {
          "type": "priority",
          "system": "priority"
        }
      },
      {
        "field_id": "customfield_10050",
        "name": "Environment",
        "required": true,
        "field_type": "string",
        "allowed_values": null
      }
    ]
  }]
}
```

---

## AI Agent Workflow

### Pattern 1: Safe Issue Creation

```javascript
// Step 1: Discover what's needed
const metadata = await get_create_metadata({
  project_key: "PROJ",
  issue_type: "Bug"
});

// Step 2: Check requirements
const bugType = metadata.issue_types[0];
console.log("Required fields:", bugType.required_fields);
// Output: ["project", "summary", "issuetype", "priority", "customfield_10050"]

console.log("Allowed priorities:",
  bugType.all_fields.find(f => f.field_id === "priority").allowed_values
);
// Output: ["Highest", "High", "Medium", "Low", "Lowest"]

// Step 3: Create issue with all required fields
const issue = await create_issue({
  project_key: "PROJ",
  summary: "Login button not responding",
  issue_type: "Bug",
  priority: "High",  // ✅ From allowed values
  custom_fields: {
    "customfield_10050": "Production"  // ✅ Required custom field
  }
});
```

---

### Pattern 2: User Prompt Handling

**User says:** "Create a bug for the login issue"

**AI Workflow:**
```javascript
// 1. Get metadata to understand requirements
const metadata = await get_create_metadata({
  project_key: "PROJ",
  issue_type: "Bug"
});

// 2. Check if user provided all required info
const bugMeta = metadata.issue_types[0];
const requiredCustomFields = bugMeta.field_summary.required_custom;

// 3. If missing required fields, ask user
if (requiredCustomFields.includes("Environment")) {
  // Ask: "What environment is this bug in? (required field)"
  // User: "Production"
}

// 4. Validate priority if required
const priorityField = bugMeta.all_fields.find(f => f.field_id === "priority");
if (priorityField.required) {
  // Ask: "What priority? (Highest, High, Medium, Low, Lowest)"
  // User: "High"
}

// 5. Create with validated data
const issue = await create_issue({
  project_key: "PROJ",
  summary: "Login button not responding",
  issue_type: "Bug",
  priority: "High",
  custom_fields: {
    "customfield_10050": "Production"
  }
});
```

---

### Pattern 3: Batch Issue Creation

```javascript
// Get metadata once, create many issues
const metadata = await get_create_metadata({
  project_key: "PROJ"
});

// Find Story type requirements
const storyType = metadata.issue_types.find(t => t.name === "Story");
const hasStoryPoints = storyType.all_fields.some(
  f => f.name === "Story Points"
);

// Create multiple stories
for (const feature of features) {
  const params = {
    project_key: "PROJ",
    summary: feature.title,
    issue_type: "Story",
    description: feature.description
  };

  // Add story points only if available in this project
  if (hasStoryPoints) {
    params.custom_fields = {
      "customfield_10016": feature.points
    };
  }

  await create_issue(params);
}
```

---

## Understanding the Response

### Field Categories

**Required Standard Fields:**
- Always needed (project, summary, issuetype)
- Handled automatically by create_issue tool

**Required Custom Fields:**
- Project-specific requirements
- Must be provided via `custom_fields` parameter
- Tool shows field name and ID: `"Environment (customfield_10050)"`

**Optional Standard Fields:**
- Common fields like description, priority, assignee
- Can be provided directly to create_issue

**Optional Custom Fields:**
- Nice-to-have custom fields
- Use `custom_fields` parameter if needed

---

### Field Info Details

```json
{
  "field_id": "customfield_10050",
  "name": "Environment",
  "required": true,
  "field_type": "string",
  "allowed_values": ["Development", "Staging", "Production"],
  "has_default_value": false
}
```

- **field_id**: Use this in `custom_fields` parameter
- **name**: Human-readable name
- **required**: Must provide or creation fails
- **field_type**: Data type (string, array, option, etc.)
- **allowed_values**: Valid options (if restricted)
- **has_default_value**: Will auto-fill if not provided

---

## Common Scenarios

### Scenario 1: Different Projects, Different Rules

```javascript
// Project A requires priority for bugs
const metadataA = await get_create_metadata({
  project_key: "PROJ-A",
  issue_type: "Bug"
});
// Required: ["project", "summary", "issuetype", "priority"]

// Project B doesn't require priority
const metadataB = await get_create_metadata({
  project_key: "PROJ-B",
  issue_type: "Bug"
});
// Required: ["project", "summary", "issuetype"]

// AI adapts to each project's requirements
```

---

### Scenario 2: Custom Workflows

```javascript
const metadata = await get_create_metadata({
  project_key: "SECURITY",
  issue_type: "Bug"
});

// Security project requires:
// - Severity (customfield_10100)
// - Security Classification (customfield_10101)
// - Affected Versions (customfield_10102)

const securityBug = await create_issue({
  project_key: "SECURITY",
  summary: "SQL Injection in login form",
  issue_type: "Bug",
  priority: "Highest",
  custom_fields: {
    "customfield_10100": "Critical",
    "customfield_10101": "Confidential",
    "customfield_10102": ["2.1.0", "2.0.5"]
  }
});
```

---

### Scenario 3: Component Validation

```javascript
const metadata = await get_create_metadata({
  project_key: "PROJ",
  issue_type: "Task"
});

// Find components field
const componentField = metadata.issue_types[0].all_fields
  .find(f => f.field_id === "components");

console.log("Available components:", componentField.allowed_values);
// ["Backend", "Frontend", "Database", "API"]

// Create with valid component
await create_issue({
  project_key: "PROJ",
  summary: "Optimize query performance",
  components: ["Database"]  // ✅ Valid
});
```

---

## Error Prevention

### Before Metadata Tool:
```javascript
// ❌ This might fail with cryptic error
await create_issue({
  project_key: "PROJ",
  summary: "Fix bug",
  issue_type: "Bug"
  // Missing required priority!
  // Missing required custom field!
});
// Error: "Field 'priority' is required"
```

### With Metadata Tool:
```javascript
// ✅ Discover requirements first
const meta = await get_create_metadata({
  project_key: "PROJ",
  issue_type: "Bug"
});

// Check what's required
console.log("Required:", meta.issue_types[0].required_fields);

// Create with all requirements
await create_issue({
  project_key: "PROJ",
  summary: "Fix bug",
  issue_type: "Bug",
  priority: "High",  // ✅ Required
  custom_fields: {
    "customfield_10050": "Production"  // ✅ Required
  }
});
```

---

## Best Practices

### For AI Agents:

1. **Cache Metadata** - Same project + issue type → reuse metadata
2. **Validate Before Create** - Check required fields are provided
3. **Use Allowed Values** - Validate against allowed_values when present
4. **Handle Defaults** - Skip fields with has_default_value: true
5. **Show Field Names** - Use human names when asking users for input

### For Users:

1. **Check Once** - Run metadata tool once per project
2. **Bookmark Results** - Save commonly used field IDs
3. **Use with Create** - Reference metadata when creating issues
4. **Update Periodically** - Re-fetch if project configuration changes

---

## Integration Example

### Complete AI Conversation Flow:

**User:** "Create a bug for the payment processing error in production"

**AI:**
```javascript
// 1. Get metadata
const meta = await get_create_metadata({
  project_key: "PAY",
  issue_type: "Bug"
});

// 2. Extract requirements
const bug = meta.issue_types[0];
const needsPriority = bug.required_fields.includes("priority");
const needsEnv = bug.field_summary.required_custom
  .some(f => f.includes("Environment"));

// 3. Has enough info?
// ✅ Summary: "payment processing error"
// ✅ Environment: "production" (from user message)
// ❌ Priority: not mentioned

// 4. Ask for missing required field
// AI: "What priority? (Highest, High, Medium, Low, Lowest)"
// User: "Highest"

// 5. Create with all requirements
const issue = await create_issue({
  project_key: "PAY",
  summary: "Payment processing error in production",
  issue_type: "Bug",
  priority: "Highest",
  custom_fields: {
    "customfield_10050": "production"
  },
  assign_to_me: true
});

// 6. Confirm to user
// AI: "Created bug PAY-456: Payment processing error in production"
// AI: "View at: https://your-domain.atlassian.net/browse/PAY-456"
```

---

## Summary

### Why This Tool Is Essential:

✅ **Prevents Errors** - Know requirements before creating
✅ **Project-Agnostic** - Works with any JIRA project configuration
✅ **AI-Friendly** - Structured data for automated decision-making
✅ **User-Friendly** - Clear field names and helpful hints
✅ **Validation Ready** - Allowed values for field validation
✅ **Future-Proof** - Adapts to project configuration changes

### The Workflow:

```
1. get_create_metadata()
   ↓
2. Validate user input
   ↓
3. Ask for missing required fields
   ↓
4. create_issue() with complete data
   ↓
5. Success! ✅
```
