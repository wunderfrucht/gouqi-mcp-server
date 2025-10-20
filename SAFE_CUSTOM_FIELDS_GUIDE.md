# SAFe Implementation in Your Organization: Custom Fields Guide
## How SAFe Data is Stored and Auto-Detected

**Date:** 2025-01-17
**Context:** Understanding your SAFe setup without Jira Align

---

## Answer to Your Question: "Is this mainly driven by custom fields?"

**Short Answer:** **Yes, about 70% custom fields, 30% native JIRA fields**

**Breakdown:**

| SAFe Concept | Storage in JIRA | Type |
|--------------|-----------------|------|
| **Program Increment (PI)** | `fixVersion` OR custom field `customfield_10xxx` | Mixed (your choice!) |
| **Capability** | Custom field `customfield_10xxx` | **Custom Field** |
| **Feature** | Native `Epic` (renamed) OR custom link type | Native (workaround) |
| **Epic** | Native `Epic` OR custom field | Native OR Custom |
| **Story** | Native `Story` issue type | **Native** |
| **Sprint** | Native `Sprint` field (array) | **Native** |
| **ART/Team** | Custom field OR `Component` | Mixed |
| **Dependencies** | Native `Blocks/Blocked By` links | **Native** |
| **WSJF Score** | Custom field `customfield_10xxx` | **Custom Field** |
| **Business Value** | Custom field `customfield_10xxx` | **Custom Field** |

**The Reality:**
- **Native JIRA provides:** Issue types (Story, Epic, Task), Sprint field, Issue links, fixVersion
- **Custom fields needed for:** PI tracking, Capability, ART assignment, SAFe metrics (WSJF, Business Value)
- **Workarounds needed for:** Feature hierarchy (can't add levels below Epic)

---

## How It Works in Practice

### Option 1: You Use fixVersion for PI (Recommended)

**Setup:**
```
Project Settings → Releases → Create Versions:
- Name: "PI 2025.1"
- Start Date: 2025-01-13
- Release Date: 2025-03-24
- Description: "Q1 2025 Program Increment"

Repeat for: PI 2025.2, PI 2025.3, PI 2025.4
```

**JQL Queries:**
```jql
# All work in current PI
fixVersion = "PI 2025.1"

# All work in unreleased PIs
fixVersion in unreleasedVersions()

# Feature X in current PI
"Epic Link" = FEAT-123 AND fixVersion = "PI 2025.1"

# Cross-team work in PI
fixVersion = "PI 2025.1" AND project in (TEAM1, TEAM2, TEAM3)
```

**Advantages:**
✓ Native JIRA field - no custom field needed
✓ Release burndown charts work automatically
✓ JQL functions like `unreleasedVersions()` available
✓ Release Hub shows PI status

**Disadvantages:**
✗ Must create fixVersion in EVERY project (each team)
✗ Naming must be consistent across teams
✗ May conflict with actual product releases

### Option 2: You Use Custom Field for PI

**Setup:**
```
Project Settings → Custom Fields → Create Field:
- Name: "Program Increment"
- Type: Select List (single choice)
- Values: "PI 2025.1", "PI 2025.2", "PI 2025.3", "PI 2025.4"
- Field ID: customfield_10100 (assigned by JIRA)
```

**JQL Queries:**
```jql
# All work in current PI
"Program Increment" = "PI 2025.1"

# All WIP in current PI
"Program Increment" = "PI 2025.1" AND status = "In Progress"

# Stories in PI
"Program Increment" = "PI 2025.1" AND type = Story
```

**Advantages:**
✓ Flexible - can add metadata
✓ Doesn't conflict with releases
✓ Single configuration (shared across projects)
✓ Can use labels: "2025.1", "Q1-2025", etc.

**Disadvantages:**
✗ No native release reports
✗ Must build custom dashboards
✗ No `unreleasedVersions()` equivalent

---

## Your MCP Server's Auto-Detection Strategy

### Phase 1: Discovery on First Use

When you run any SAFe tool for the first time, the MCP server will:

**Step 1: Inspect a Sample Issue**
```rust
Tool: get_custom_fields
Parameters: { "issue_key": "PROJ-123" }

Returns:
{
  "custom_fields": [
    {
      "field_id": "customfield_10100",
      "field_type": "string",
      "value": "PI 2025.1",
      "value_display": "PI 2025.1"
    },
    {
      "field_id": "customfield_10101",
      "field_type": "string",
      "value": "CAP-456",
      "value_display": "CAP-456"
    },
    {
      "field_id": "customfield_10102",
      "field_type": "object",
      "value": {"name": "Stream Alpha"},
      "value_display": "Object: Stream Alpha"
    }
  ],
  "detected_mappings": {
    "sprint_field": "customfield_10020",
    "story_points_field": "customfield_10016"
  }
}
```

**Step 2: Pattern Matching**
```rust
// MCP server analyzes field values to detect SAFe fields

if value matches "PI \d{4}\.\d+" → Likely PI field
if value matches "CAP-\d+" → Likely Capability field
if value contains "Stream" or "ART" → Likely ART/Team field
if value is numeric (1-100) → Could be WSJF score
if field_type == "number" → Could be story points
```

**Step 3: Cache the Mapping**
```rust
pub struct SAFeFieldMapping {
    pub pi_field: Option<String>,         // "customfield_10100" or "fixVersion"
    pub pi_field_type: String,            // "custom" or "native"
    pub capability_field: Option<String>, // "customfield_10101"
    pub art_field: Option<String>,        // "customfield_10102"
    pub wsjf_field: Option<String>,       // "customfield_10103"
    pub business_value_field: Option<String>,
}

// Cached for 60 minutes, or until user runs clear_cache
```

### Phase 2: Validation Through Search

**Try fixVersion first:**
```jql
fixVersion in unreleasedVersions()
```

**Check if results look like PIs:**
- Version names match pattern: "PI 2025.X" or "2025.Q1"
- Multiple versions spanning quarters
- Start/end dates aligned to 10-12 week cycles

**If fixVersion doesn't look like PIs:**
```jql
# Search for custom field
"customfield_10100" IS NOT EMPTY
```

**Analyze results:**
- Consistent naming pattern
- Multiple values for different quarters
- Issues span sprints within PI range

### Phase 3: User Confirmation (Optional)

```rust
Tool: configure_safe_fields
Parameters: {
  "pi_field": "customfield_10100",  // or "fixVersion"
  "capability_field": "customfield_10101",
  "art_field": "customfield_10102"
}

// Manually override auto-detection if needed
```

---

## How Each Tool Will Work With Your Setup

### Example 1: search_issues_by_pi

**Your Setup Detected:**
- PI Field: `customfield_10100` (custom field)
- Capability Field: `customfield_10101`
- ART Field: `customfield_10102`

**User Request:**
```
"Show me all WIP work in PI 2025.1"
```

**MCP Server Flow:**
```rust
1. Auto-detect PI field → customfield_10100
2. Build JQL:
   "customfield_10100" = "PI 2025.1" AND status = "In Progress"
3. Execute search
4. Group by:
   - customfield_10102 (ART/Team)
   - "Epic Link" (Feature)
   - Sprint
```

**Response:**
```json
{
  "pi": "PI 2025.1",
  "total_wip": 47,
  "by_team": {
    "Stream Alpha": {
      "stories": 15,
      "features": 3,
      "sprints": ["PI 2025.1.1", "PI 2025.1.2"]
    },
    "Stream Beta": {
      "stories": 32,
      "features": 5,
      "sprints": ["PI 2025.1.2"]
    }
  }
}
```

### Example 2: get_feature_progress

**Your Setup Detected:**
- Feature = Epic (native)
- PI Field: `customfield_10100`
- Story Points: `customfield_10016` (detected from numeric field)

**User Request:**
```
"Check Feature FEAT-456 progress"
```

**MCP Server Flow:**
```rust
1. Get Epic FEAT-456
2. Find all child Stories:
   JQL: "Epic Link" = FEAT-456
3. For each story, extract:
   - Status
   - customfield_10016 (story points)
   - Sprint (customfield_10020 or native)
   - customfield_10100 (PI)
4. Calculate rollup:
   - Total stories: 10
   - Done: 4 (16 points)
   - In Progress: 3 (12 points)
   - To Do: 3 (15 points)
   - Completion: 37% (16 of 43 points)
```

**Response:**
```json
{
  "feature_key": "FEAT-456",
  "feature_summary": "User Authentication System",
  "pi": "PI 2025.1",
  "status": "In Progress",
  "story_rollup": {
    "total": 10,
    "done": 4,
    "in_progress": 3,
    "todo": 3
  },
  "story_points": {
    "total": 43,
    "completed": 16,
    "remaining": 27,
    "completion_percentage": 37
  },
  "risk_indicators": [
    "Behind schedule (expected 50% by Sprint 2)",
    "2 blocked stories"
  ]
}
```

---

## Configuration: Setting Up Your JIRA for SAFe MCP

### Step 1: Choose Your PI Strategy

**Option A: Use fixVersion (Recommended if not already using for releases)**

```bash
# For each team project, create releases:
Project TEAM1 → Releases:
  - PI 2025.1 (Start: 2025-01-13, End: 2025-03-24)
  - PI 2025.2 (Start: 2025-03-25, End: 2025-06-09)
  - PI 2025.3 (Start: 2025-06-10, End: 2025-08-25)
  - PI 2025.4 (Start: 2025-08-26, End: 2025-11-10)

Repeat for: TEAM2, TEAM3, TEAM4...
```

**Option B: Use Custom Field**

```bash
# Create once, share across all projects
Custom Field:
  Name: "Program Increment"
  Type: Select List (single choice)
  Context: Global (all projects)
  Values:
    - PI 2025.1
    - PI 2025.2
    - PI 2025.3
    - PI 2025.4
```

### Step 2: Create Capability Field (Custom Field Required)

```bash
Custom Field:
  Name: "Capability"
  Type: Text Field (single line) OR Issue Picker
  Context: Issues of type Epic, Feature

  # Stores: "CAP-123" or capability issue key
```

**Alternative: Use Epic of Epic approach**
- Epic at Capability level
- Sub-Epic (custom issue type) at Feature level
- Requires Advanced Roadmaps (Premium)

### Step 3: Create ART/Team Field (Optional)

```bash
Custom Field:
  Name: "Agile Release Train"
  Type: Select List (single choice)
  Values:
    - Stream Alpha
    - Stream Beta
    - Stream Gamma
```

**Alternative: Use Component field**
```bash
# Per project
Components:
  - Stream Alpha
  - Stream Beta
```

### Step 4: Create SAFe Metrics Fields (Optional)

```bash
Custom Field: "WSJF Score"
  Type: Number Field
  Min: 0, Max: 100

Custom Field: "Business Value"
  Type: Number Field
  Min: 0, Max: 100

Custom Field: "Time Criticality"
  Type: Select List
  Values: Low, Medium, High, Urgent
```

### Step 5: Configure Issue Types and Hierarchy

**Option A: Rename Approach (Simplest)**
```bash
# In your JIRA scheme, rename:
Initiative → Capability
Epic → Feature
Story → Story (unchanged)

Hierarchy: Capability → Feature → Story → Subtask
```

**Option B: Custom Link Type**
```bash
# Create custom link type
Link Type: "SAFe Hierarchy"
  Outward: "Is Parent Of"
  Inward: "Is Child Of"

# Use for: Capability → Epic → Feature → Story
```

---

## Auto-Detection Algorithm (Implementation Details)

### Algorithm for PI Field Detection

```rust
pub async fn detect_pi_field(&self) -> JiraMcpResult<PIFieldInfo> {
    // Step 1: Try fixVersion first (most common)
    let versions = self.get_unreleased_versions().await?;

    if self.looks_like_pi_versions(&versions) {
        return Ok(PIFieldInfo {
            field_id: "fixVersion".to_string(),
            field_type: FieldType::Native,
            detected_pis: versions,
            confidence: Confidence::High,
        });
    }

    // Step 2: Search custom fields
    let sample_issue = self.get_sample_issue_with_custom_fields().await?;

    for (field_id, value) in sample_issue.custom_fields {
        // Pattern match for PI values
        if let Some(pi_value) = value.as_str() {
            if PI_PATTERN.is_match(pi_value) {
                return Ok(PIFieldInfo {
                    field_id: field_id.clone(),
                    field_type: FieldType::Custom,
                    detected_pis: vec![pi_value.to_string()],
                    confidence: Confidence::Medium,
                });
            }
        }
    }

    // Step 3: Ask user or search by field name
    let field_metadata = self.get_all_field_metadata().await?;

    for field in field_metadata {
        if field.name.to_lowercase().contains("program increment")
           || field.name.to_lowercase().contains("pi") {
            return Ok(PIFieldInfo {
                field_id: field.id,
                field_type: FieldType::Custom,
                detected_pis: vec![],
                confidence: Confidence::Low,
            });
        }
    }

    Err(JiraMcpError::not_found("PI field", "No PI field detected"))
}

fn looks_like_pi_versions(&self, versions: &[Version]) -> bool {
    // Check if version names match PI patterns
    let pi_count = versions.iter()
        .filter(|v| {
            v.name.starts_with("PI ") ||
            v.name.matches(r"^\d{4}\.[1-4]$").is_some() ||
            v.name.contains("Q") && v.name.contains("2025")
        })
        .count();

    // Need at least 2 PIs to be confident
    pi_count >= 2 && pi_count as f32 / versions.len() as f32 > 0.5
}

lazy_static! {
    static ref PI_PATTERN: Regex = Regex::new(
        r"^(PI\s*)?(\d{4})[.\-]([1-4]|Q[1-4])$"
    ).unwrap();
}
```

### Algorithm for Capability Field Detection

```rust
pub async fn detect_capability_field(&self) -> JiraMcpResult<String> {
    // Step 1: Check for field name containing "capability"
    let fields = self.get_all_field_metadata().await?;

    for field in fields {
        if field.name.to_lowercase().contains("capability") {
            return Ok(field.id);
        }
    }

    // Step 2: Check for custom fields with issue key patterns
    let sample_epic = self.get_sample_epic().await?;

    for (field_id, value) in sample_epic.custom_fields {
        if let Some(val_str) = value.as_str() {
            // Pattern: CAP-123 or similar
            if val_str.matches(r"^[A-Z]{2,5}-\d+$") {
                return Ok(field_id);
            }
        }
    }

    Err(JiraMcpError::not_found("Capability field", "No capability field detected"))
}
```

---

## What Your MCP Server Needs From You

### Discovery Phase (One-Time Setup)

**Option 1: Fully Automatic (Recommended)**
```
# First time you use any SAFe tool:
User: "Show me all work in current PI"

MCP Server:
1. Inspects your JIRA setup automatically
2. Detects: fixVersion or custom field for PI
3. Caches the configuration
4. Returns results
```

**Option 2: Manual Configuration**
```
# Run once to configure:
Tool: configure_safe_fields
Parameters: {
  "pi_field": "fixVersion",  # or "customfield_10100"
  "capability_field": "customfield_10101",
  "art_field": "Component",  # or "customfield_10102"
  "story_points_field": "customfield_10016"
}

# Saves to cache, used for all future queries
```

### Required Information (If Auto-Detection Fails)

1. **Sample Issue Key**
   - Provide: "TEAM1-123" (any issue with SAFe fields populated)
   - MCP server inspects this to detect fields

2. **PI Naming Convention**
   - Format: "PI 2025.1" or "2025-Q1" or "2025.1"?
   - Used for pattern matching

3. **Current PI**
   - What PI are you in now? (for "current PI" queries)
   - MCP server can detect from dates if using fixVersion

4. **Hierarchy Approach**
   - Using renamed Epics (Epic=Feature)?
   - Or custom link types?
   - Or custom fields?

---

## Migration Path: What If You Change Your Mind?

### Scenario: You Start With Custom Field, Later Switch to fixVersion

**No Problem!** The MCP server adapts:

```rust
// Old configuration (cached)
pi_field: "customfield_10100"

// You create fixVersions and populate them
// User runs: clear_cache

// Next query auto-detects new setup
pi_field: "fixVersion"  // Auto-switched!

// All tools continue to work
```

### Scenario: You Add Capability Field Later

```rust
// Initially: No capability field detected
capability_field: None

// You create capability field and populate it
// Run: clear_cache

// Next query detects it
capability_field: Some("customfield_10101")

// Tools now support capability queries
```

---

## Summary: How It Works in YOUR Organization

### The Data Flow

```
1. You structure data in JIRA:
   ├── PI: fixVersion "PI 2025.1" (native)
   ├── Capability: customfield_10101 "CAP-456" (custom)
   ├── Feature: Epic (renamed, native)
   ├── Story: Story (native)
   └── Sprint: Sprint field (native)

2. MCP Server auto-detects on first use:
   ├── Scans fixVersion → Finds "PI 2025.1" pattern
   ├── Scans custom fields → Finds "CAP-456" pattern
   ├── Caches mapping for 60 minutes
   └── Ready for queries!

3. You ask questions:
   "What's running in PI 2025.1?"

4. MCP Server translates to JQL:
   fixVersion = "PI 2025.1" AND status = "In Progress"

5. Results grouped by your structure:
   ├── By ART/Team
   ├── By Feature (Epic)
   ├── By Sprint
   └── With story points, status, dependencies
```

### The Key Insight

**You don't change how you work in JIRA!**

The MCP server adapts to YOUR setup:
- Whether you use fixVersion or custom fields
- Whether you renamed Epics or use custom links
- Whether you track ART via Components or custom fields

**All it needs:**
- Sample issue to inspect (one time)
- Consistent naming patterns
- Clear cache when you change structure

---

## Next Steps

1. **Tell me about YOUR setup:**
   - Do you use fixVersion or custom field for PI?
   - What's a sample issue key I can reference?
   - What's your current PI identifier?

2. **I'll verify auto-detection works:**
   - Test the detection algorithm
   - Confirm all SAFe fields are found
   - Document your specific configuration

3. **We implement the tools:**
   - With YOUR field IDs hardcoded initially
   - Auto-detection added later
   - You get immediate value!

**The beauty:** Even if we hardcode fields now, auto-detection means the tools work for OTHER organizations too!
