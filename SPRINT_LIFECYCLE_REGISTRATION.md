# Sprint Lifecycle Tools - Registration Guide

## Implementation Complete ✅

The three sprint lifecycle tools have been implemented in `jira-mcp-server/src/tools/sprints.rs`:

1. **CreateSprintTool** - Create new sprints
2. **StartSprintTool** - Start future sprints (change state to "active")
3. **CloseSprintTool** - Close active sprints with completion statistics

## Required Changes to Register Tools

### 1. Export New Types from `tools/mod.rs`

Add to the public exports:

```rust
pub use sprints::{
    // ... existing exports ...
    CreateSprintParams, CreateSprintResult, CreateSprintTool,
    StartSprintParams, StartSprintResult, StartSprintTool,
    CloseSprintParams, CloseSprintResult, CloseSprintTool,
};
```

### 2. Update `lib.rs` Imports (line ~35-45)

Add to the `use crate::tools::{` block:

```rust
    CloseSprintParams, CloseSprintResult, CloseSprintTool,
    CreateSprintParams, CreateSprintResult, CreateSprintTool,
    StartSprintParams, StartSprintResult, StartSprintTool,
```

### 3. Add Tool Fields to Struct (line ~122)

After `move_to_sprint_tool`:

```rust
    move_to_sprint_tool: Arc<MoveToSprintTool>,
    create_sprint_tool: Arc<CreateSprintTool>,
    start_sprint_tool: Arc<StartSprintTool>,
    close_sprint_tool: Arc<CloseSprintTool>,
```

### 4. Initialize Tools in `new()` (line ~249)

After the existing sprint tools:

```rust
        let move_to_sprint_tool = Arc::new(MoveToSprintTool::new(Arc::clone(&jira_client)));
        let create_sprint_tool = Arc::new(CreateSprintTool::new(Arc::clone(&jira_client)));
        let start_sprint_tool = Arc::new(StartSprintTool::new(Arc::clone(&jira_client)));
        let close_sprint_tool = Arc::new(CloseSprintTool::new(Arc::clone(&jira_client)));
```

### 5. Add to Struct Initialization in `new()` (line ~294)

```rust
            move_to_sprint_tool,
            create_sprint_tool,
            start_sprint_tool,
            close_sprint_tool,
```

### 6. Repeat for `with_config()` (lines ~395 and ~434)

Same additions as steps 4 and 5 above.

### 7. Update Tools Count (line ~552)

```rust
tools_count: 47, // was 44, now +3 for create_sprint, start_sprint, close_sprint
```

Also update the comment to include:

```
..., create_sprint, start_sprint, close_sprint
```

### 8. Add Public MCP Methods (after line ~1240)

```rust
    /// Create a new sprint on a board
    ///
    /// Creates a future sprint with the specified name. Dates can be set immediately
    /// or later when starting the sprint.
    ///
    /// # Examples
    /// - Create basic sprint: `{"board_id": 1, "name": "Sprint 42"}`
    /// - With dates: `{"board_id": 1, "name": "Sprint 42", "start_date": "2025-01-20T00:00:00Z", "end_date": "2025-02-03T23:59:59Z"}`
    #[instrument(skip(self))]
    pub async fn create_sprint(
        &self,
        params: CreateSprintParams,
    ) -> anyhow::Result<CreateSprintResult> {
        self.create_sprint_tool
            .execute(params)
            .await
            .map_err(|e: JiraMcpError| {
                error!("create_sprint failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Start a sprint
    ///
    /// Transitions a future sprint to active state. Requires an end date to be set
    /// either on the sprint already or provided as a parameter.
    ///
    /// Validations:
    /// - Sprint must be in "future" state (not already active or closed)
    /// - End date must be set
    /// - Warns if sprint has no issues
    ///
    /// # Examples
    /// - Start with existing dates: `{"sprint_id": 123}`
    /// - Start and set end date: `{"sprint_id": 123, "end_date": "2025-02-03T23:59:59Z"}`
    /// - Start with custom start: `{"sprint_id": 123, "start_date": "2025-01-20T08:00:00Z", "end_date": "2025-02-03T18:00:00Z"}`
    #[instrument(skip(self))]
    pub async fn start_sprint(
        &self,
        params: StartSprintParams,
    ) -> anyhow::Result<StartSprintResult> {
        self.start_sprint_tool
            .execute(params)
            .await
            .map_err(|e: JiraMcpError| {
                error!("start_sprint failed: {}", e);
                anyhow::anyhow!(e)
            })
    }

    /// Close a sprint
    ///
    /// Closes an active sprint and provides completion statistics. Optionally moves
    /// incomplete issues to another sprint for continuity.
    ///
    /// Features:
    /// - Calculates completion rate (done vs total issues)
    /// - Optionally moves incomplete issues to next sprint
    /// - Provides warnings about incomplete work
    /// - JIRA automatically sets complete date to current time
    ///
    /// # Examples
    /// - Simple close: `{"sprint_id": 123}`
    /// - Move incomplete to next: `{"sprint_id": 123, "move_incomplete_to": 124}`
    #[instrument(skip(self))]
    pub async fn close_sprint(
        &self,
        params: CloseSprintParams,
    ) -> anyhow::Result<CloseSprintResult> {
        self.close_sprint_tool
            .execute(params)
            .await
            .map_err(|e: JiraMcpError| {
                error!("close_sprint failed: {}", e);
                anyhow::anyhow!(e)
            })
    }
```

## Quick Registration Script

Run this from the project root to complete registration:

```bash
# This file has manual changes needed - see sections above
# Or run cargo build and fix compilation errors one by one

# Quick test after registration:
cargo check
cargo clippy --all-targets --all-features
cargo build
```

## Testing

After registration, test with:

```bash
# Create a sprint
# (requires board_id - get from list_sprints first)

# Start a sprint
# (requires sprint_id and end_date)

# Close a sprint
# (requires sprint_id, optionally move_incomplete_to)
```

## Features Implemented

### CreateSprintTool
- ✅ Creates sprint with name
- ✅ Validates board exists
- ✅ Returns sprint info with ID
- ✅ Error handling for invalid board

### StartSprintTool
- ✅ Validates sprint state (must be "future")
- ✅ Requires end date (param or already set)
- ✅ Defaults start date to "now"
- ✅ Counts issues in sprint
- ✅ Warns if sprint has no issues
- ✅ Prevents starting already active/closed sprints

### CloseSprintTool
- ✅ Validates sprint state (must not be "closed")
- ✅ Calculates completion statistics
- ✅ Moves incomplete issues to target sprint (optional)
- ✅ Provides completion rate warnings
- ✅ Handles up to 1000 issues in sprint
- ✅ Identifies done/closed/resolved as completed

## Next Steps

1. Export types from `tools/mod.rs`
2. Add imports to `lib.rs`
3. Add struct fields
4. Initialize tools in `new()` and `with_config()`
5. Update tools count
6. Add public MCP methods
7. Run `cargo check`
8. Fix any compilation errors
9. Run tests
10. Ready to use!
