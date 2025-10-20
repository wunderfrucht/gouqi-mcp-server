# SAFe Sprint Management Analysis for JIRA MCP Server
## Comprehensive Analysis & Recommendations

**Date:** 2025-01-17
**Context:** Scaled Agile Framework (SAFe) implementation without Jira Align
**Current State:** Basic sprint management tools exist, need SAFe-specific enhancements

---

## Executive Summary

Your organization is implementing **Scaled Agile Framework (SAFe)** with a multi-level hierarchy spanning Program Increments, multiple sprints, and cross-team dependencies. **There is currently NO MCP server for SAFe management** - this is a unique opportunity!

You need tools that go far beyond simple sprint lifecycle management (create/start/close) to support:
- **Multi-level SAFe hierarchy** (Capability → Feature → Story → Subtask)
- **Program Increment (PI) tracking** across 10-12 week cycles
- **Cross-sprint dependency management** for stories spanning multiple sprints
- **Cross-team coordination** when multiple teams work on related features
- **ART (Agile Release Train) visibility** across teams and sprints
- **WIP tracking** to identify currently running work across the organization

---

## Part 1: Understanding SAFe Hierarchy in JIRA

### 1.1 SAFe Organization Levels

```
Portfolio Level
├── Value Stream
    ├── Program (ART - Agile Release Train)
        ├── Team
```

### 1.2 Work Item Hierarchy

**Standard SAFe Hierarchy:**
```
Capability (Portfolio Level)
├── Epic (Program Level)
    ├── Feature (Program Level)
        ├── Story (Team Level)
            ├── Task/Subtask (Team Level)
```

**Your Specific Hierarchy (from description):**
```
Capability (spans multiple PIs)
├── Feature (spans one PI - 10-12 weeks)
    ├── Story (spans 1-2 sprints, can be carried over)
        ├── Subtask
```

### 1.3 JIRA Limitations

**Critical Challenge:** JIRA's native hierarchy only supports:
- Initiative → Epic → Story → Subtask

**Problem:** You cannot add hierarchy levels BELOW Epic in standard JIRA!
- Epics cannot have parents (except Initiative in Premium)
- Stories can only link to ONE Epic

**Common Workarounds:**
1. **Rename approach**: Epic → Feature, Initiative → Epic (maps to SAFe Feature-Story)
2. **Custom link types**: Create "SAFe Hierarchy" links (Is Parent Of / Is Child Of)
3. **Custom fields**: Add "Capability" and "PI" custom fields to track relationships
4. **Jira Align**: Full SAFe support ($$$, separate product)
5. **Third-party apps**: Structure, Agile Hive, Easy Agile Programs

---

## Part 2: Program Increment (PI) Management

### 2.1 What is a Program Increment?

- **Duration**: 8-12 weeks (typically 10 weeks)
- **Structure**: 5 iterations (sprints) + 1 Innovation & Planning (IP) sprint
- **Cadence**: Fixed schedule for planning, execution, and synchronization
- **Scope**: Contains multiple Features that deliver value

### 2.2 PI Tracking in JIRA (Best Practices)

**Option 1: Fix Version Field (RECOMMENDED)**
```
Advantages:
✓ Semantically aligned with release cadence
✓ Native JIRA support for reporting
✓ Release burndown charts work out-of-box
✓ Can track actual vs planned

Implementation:
- Create fixVersions: "PI 2025.1", "PI 2025.2", "PI 2025.3"
- Assign Features/Stories to appropriate PI
- Use release reports for PI burndown
```

**Option 2: Custom "PI" Field**
```
Field Type: Select List (single choice) or Text
Values: "PI 2025.1", "PI 2025.2", "PI 2025.3", "PI 2025.4"

Advantages:
✓ Flexible
✓ Can add metadata (start/end dates, PI goals)
✓ Doesn't conflict with actual releases

Disadvantages:
✗ No native reporting
✗ Need custom JQL queries
✗ No automatic burndown charts
```

**Option 3: Components**
```
Create components per PI: "PI-2025-Q1", "PI-2025-Q2"

Advantages:
✓ Easy filtering
✓ Can combine with other components

Disadvantages:
✗ Components typically used for technical domains
✗ Can cause confusion
```

### 2.3 Sprint Naming in SAFe

**Recommended Convention:**
```
Format: {ART Name} - PI {Year}.{Quarter}.{Iteration}

Examples:
- "StreamAlpha - PI 2025.1.1"  (Q1, Sprint 1)
- "StreamAlpha - PI 2025.1.2"  (Q1, Sprint 2)
- "StreamAlpha - PI 2025.1.3"  (Q1, Sprint 3)
- "StreamAlpha - PI 2025.1.4"  (Q1, Sprint 4)
- "StreamAlpha - PI 2025.1.5"  (Q1, Sprint 5)
- "StreamAlpha - PI 2025.1.IP" (Q1, Innovation & Planning)

Alternative Format:
- "PI25Q1-Sprint1", "PI25Q1-Sprint2", "PI25Q1-Sprint3"
```

**Challenge with Similar Sprint Names:**
- Multiple ARTs running parallel PIs → naming collision
- Solution: Include team/ART identifier in sprint name
- Your MCP server should support **sprint search by pattern** or **date range**

---

## Part 3: Cross-Sprint & Cross-Team Dependencies

### 3.1 The Problem You're Facing

**Scenario:**
```
Feature: User Authentication (PI 2025.1)
├── Story A: OAuth Implementation (3 sprints)
│   ├── Sprint 1: Design & Setup (Team Alpha)
│   ├── Sprint 2: Core Implementation (Team Alpha)  ← Story carries over
│   ├── Sprint 3: Integration (Team Alpha)         ← Story carries over
├── Story B: Token Management (2 sprints)
│   ├── Sprint 1: Token Service (Team Beta)
│   ├── Sprint 2: Refresh Logic (Team Beta)        ← Story carries over
│   └── DEPENDS ON: Story A (Sprint 2)             ← Cross-team dependency
```

**Questions You Need to Answer:**
1. **What's currently running?** → Show all WIP issues across all active sprints
2. **What stories span multiple sprints?** → Track carryover stories
3. **What are the dependencies?** → Cross-team blockers
4. **Is Feature X on track for PI delivery?** → Roll-up status
5. **Which teams are working on Feature Y?** → Multi-team coordination

### 3.2 Current JIRA MCP Server Capabilities

**✅ What You Have:**
- `get_issue_relationships` - Can trace epic/parent links, blocks/blocked-by
- `search_issues` - Can search by sprint, status, assignee, project
- `get_sprint_issues` - Get issues in a specific sprint
- `link_issues` / `delete_issue_link` - Manage dependencies
- `get_custom_fields` - Discover PI/Feature fields

**❌ What You're Missing:**
- **No PI-based search** - Can't search "all issues in PI 2025.1"
- **No cross-sprint analysis** - Can't find "stories spanning multiple sprints"
- **No ART/team rollup** - Can't aggregate status across teams
- **No dependency visualization** - Can't map cross-team dependencies
- **No WIP tracking** - Can't identify all "In Progress" work across sprints
- **No Feature → Story rollup** - Can't see Feature progress from Stories
- **No sprint pattern matching** - Hard to find all "PI 2025.1.*" sprints

---

## Part 4: What You Actually Need

### 4.1 Critical Missing Tools (Priority 1)

#### 1. **Advanced Search with PI Context**

```rust
Tool: search_issues_by_pi
Parameters:
- pi_identifier: String (e.g., "PI 2025.1" or "2025.1")
- status: Optional<Vec<String>> (e.g., ["In Progress", "Done"])
- issue_types: Optional<Vec<String>> (e.g., ["Feature", "Story"])
- include_child_issues: bool (default: true)
- team_filter: Optional<Vec<String>> (team names or project keys)

Returns:
- Issues grouped by Feature/Epic
- Status rollup (how many stories done vs in progress)
- Team distribution
- Sprint distribution
- Dependency count
```

**Use Case:** "Show me all WIP work in current PI across all teams"

#### 2. **Cross-Sprint Story Tracking**

```rust
Tool: get_stories_spanning_sprints
Parameters:
- pi_identifier: Optional<String>
- min_sprint_count: u32 (default: 2)
- status: Optional<Vec<String>>

Returns:
- Stories that appear in multiple sprints
- Sprint history (which sprints it was in)
- Time in each sprint
- Reason for carryover (if captured in comments)
- Current status and assignee
```

**Use Case:** "Which stories have been carried over across sprints?"

#### 3. **Dependency Map for PI**

```rust
Tool: get_pi_dependency_map
Parameters:
- pi_identifier: String
- dependency_types: Optional<Vec<String>> (["blocks", "depends on", "relates to"])
- team_scope: Optional<Vec<String>> (filter to specific teams)
- include_external_dependencies: bool (dependencies outside PI)

Returns:
- Dependency graph (from_issue, to_issue, type)
- Critical path analysis
- Blocked items with blockers
- Cross-team dependencies highlighted
- Circular dependencies detected
```

**Use Case:** "What are the cross-team dependencies in PI 2025.1?"

#### 4. **Feature Progress Rollup**

```rust
Tool: get_feature_progress
Parameters:
- feature_key: String (or epic_key)
- include_subtasks: bool
- calculate_story_points: bool

Returns:
- Feature metadata (summary, PI, status)
- Child stories with their status
- Rollup metrics:
  - Total stories: X
  - Done: Y
  - In Progress: Z
  - To Do: N
  - Story points (planned vs completed)
- Team distribution (which teams working on it)
- Sprint distribution (which sprints contain work)
- Dependency status (any blockers)
- Risk indicators (behind schedule, dependencies, etc.)
```

**Use Case:** "Is Feature X on track for PI delivery?"

#### 5. **Active Work (WIP) Tracker**

```rust
Tool: get_active_work
Parameters:
- scope: Enum ["current_pi", "current_sprint", "all_active_sprints", "team"]
- team_filter: Optional<Vec<String>>
- assignee_filter: Optional<String>
- group_by: Enum ["team", "feature", "sprint", "assignee"]

Returns:
- All issues with status "In Progress"
- Grouped by requested grouping
- Time in current status
- Sprint information
- Assignee information
- Parent Feature/Epic
- Dependency status (blocked or blocking others)
```

**Use Case:** "What is everyone currently working on right now?"

#### 6. **Sprint Pattern Search**

```rust
Tool: search_sprints
Parameters:
- name_pattern: String (regex or glob pattern)
- state: Optional<Enum["future", "active", "closed"]>
- date_range: Optional<DateRange>
- board_ids: Optional<Vec<u64>>

Returns:
- Matching sprints with metadata
- Issue counts per sprint
- Team/board information

Example Usage:
- search_sprints(name_pattern: "PI 2025.1.*")  → All sprints in PI 2025.1
- search_sprints(name_pattern: "*IP*")         → All IP sprints
- search_sprints(state: "active")              → All active sprints
```

**Use Case:** "Find all sprints for PI 2025.1 across all teams"

### 4.2 Enhanced Sprint Lifecycle Tools (Priority 2)

Beyond basic create/start/close, you need:

#### 7. **Bulk Sprint Operations**

```rust
Tool: bulk_create_sprints_for_pi
Parameters:
- pi_identifier: String
- board_ids: Vec<u64>
- start_date: Date
- sprint_duration_weeks: u32 (default: 2)
- sprint_count: u32 (default: 5)
- ip_sprint: bool (create Innovation & Planning sprint)
- naming_pattern: String (template for sprint names)

Returns:
- Created sprints per board
- Validation warnings (date overlaps, etc.)
```

**Use Case:** "Create 5 sprints for PI 2025.2 across 8 teams"

#### 8. **Sprint Alignment Checker**

```rust
Tool: check_sprint_alignment
Parameters:
- pi_identifier: String
- check_types: Vec<Enum["dates", "naming", "capacity", "dependencies"]>

Returns:
- Misaligned sprints (wrong dates, naming)
- Capacity issues (team overcommitted)
- Dependency risks (dependent work in wrong sprints)
- Recommendations for fixes
```

**Use Case:** "Are all team sprints aligned for PI 2025.1?"

#### 9. **Sprint Carry-Over Tool**

```rust
Tool: handle_sprint_carryover
Parameters:
- from_sprint_id: u64
- to_sprint_id: u64
- carryover_criteria: Enum["incomplete", "in_progress", "specific_issues"]
- issue_keys: Optional<Vec<String>> (if specific issues)
- move_or_copy: Enum["move", "copy"]
- add_comment: bool (add carryover note)

Returns:
- Issues moved/copied
- Remaining capacity in target sprint
- Warnings (dependencies, capacity issues)
```

**Use Case:** "Move all incomplete work from Sprint 3 to Sprint 4"

### 4.3 SAFe Hierarchy Support (Priority 3)

#### 10. **Custom Field Management for SAFe**

```rust
Tool: get_safe_hierarchy_fields
Parameters:
- project_key: String

Returns:
- Detected SAFe custom fields:
  - PI field (customfield_xxxxx)
  - Capability field
  - ART/Team field
  - WSJF score field
  - Business value field
- Validation of field types
- Usage recommendations
```

#### 11. **Capability/Feature Hierarchy Navigator**

```rust
Tool: get_capability_hierarchy
Parameters:
- root_key: String (Capability, Feature, or Epic key)
- max_depth: u32
- include_metrics: bool (story points, status rollup)

Returns:
- Full hierarchy tree
- Metrics at each level
- Status propagation
- Team ownership at each level
- PI alignment information
```

**Use Case:** "Show me the full hierarchy for Capability CAP-123"

### 4.4 Team & ART Coordination (Priority 3)

#### 12. **ART Dashboard View**

```rust
Tool: get_art_status
Parameters:
- art_name: String (or team filter)
- pi_identifier: String
- include_risks: bool
- include_dependencies: bool

Returns:
- Teams in ART
- Features in progress per team
- Objectives status (PI objectives)
- Dependency status
- Risk indicators (delays, blockers, capacity issues)
- Velocity trends
```

**Use Case:** "ART status report for current PI"

---

## Part 5: Implementation Recommendations

### 5.1 Phase 1: Foundation (Weeks 1-2)

**Focus:** Enable PI-based search and cross-sprint visibility

1. **Add PI Field Detection**
   - Enhance `get_custom_fields` to detect PI fields
   - Add PI field to `search_issues` parameters
   - Cache PI field ID for performance

2. **Implement `search_issues_by_pi`**
   - Build JQL query: `fixVersion = "PI 2025.1" OR "PI Field" = "PI 2025.1"`
   - Support status filtering
   - Add grouping by Feature/Epic

3. **Implement `get_active_work`**
   - Search: `status = "In Progress" AND sprint in openSprints()`
   - Group by team/feature/sprint
   - Add time-in-status calculation

4. **Implement `search_sprints` with pattern matching**
   - Extend current `list_sprints` tool
   - Add regex/glob pattern filtering
   - Support multiple boards

**Deliverable:** Can answer "What's running in current PI?"

### 5.2 Phase 2: Dependencies & Relationships (Weeks 3-4)

1. **Enhance `get_issue_relationships` for SAFe**
   - Add PI context to relationship graph
   - Highlight cross-team dependencies
   - Flag blocking dependencies

2. **Implement `get_pi_dependency_map`**
   - Build dependency graph for entire PI
   - Detect cross-team dependencies
   - Identify critical path

3. **Implement `get_stories_spanning_sprints`**
   - Query sprint history using `Sprint in (X, Y, Z)` JQL
   - Track story movement across sprints
   - Identify carryover patterns

**Deliverable:** Dependency visibility and carryover tracking

### 5.3 Phase 3: Feature Progress & Rollup (Weeks 5-6)

1. **Implement `get_feature_progress`**
   - Fetch Feature/Epic
   - Get all child Stories
   - Calculate rollup metrics (done/in-progress/todo)
   - Story point aggregation
   - Sprint distribution

2. **Implement SAFe hierarchy navigation**
   - Support Capability → Feature → Story traversal
   - Custom field-based hierarchy links
   - Metrics rollup at each level

**Deliverable:** Feature progress tracking

### 5.4 Phase 4: Sprint Lifecycle Enhancement (Weeks 7-8)

1. **Implement basic lifecycle (Issue #29)**
   - `create_sprint`
   - `start_sprint`
   - `close_sprint`

2. **Add bulk sprint operations**
   - `bulk_create_sprints_for_pi`
   - `check_sprint_alignment`
   - `handle_sprint_carryover`

**Deliverable:** Full sprint lifecycle management

### 5.5 Phase 5: ART Coordination (Weeks 9-10)

1. **Implement `get_art_status`**
   - Multi-team view
   - PI objective tracking
   - Risk dashboard

2. **Add capacity planning tools**
   - Team velocity tracking
   - Capacity vs commitment
   - Burndown/burnup charts

**Deliverable:** ART-level visibility

---

## Part 6: Technical Architecture

### 6.1 New Modules Needed

```
jira-mcp-server/src/tools/
├── pi_management.rs           (NEW)
├── safe_hierarchy.rs          (NEW)
├── cross_sprint_analysis.rs   (NEW)
├── dependency_tracking.rs     (NEW - or enhance existing)
├── art_coordination.rs        (NEW)
├── sprint_lifecycle.rs        (ENHANCE existing sprints.rs)
└── wip_tracking.rs            (NEW)
```

### 6.2 Data Model Extensions

**Cache Enhancements:**
```rust
// Add to cache.rs
pub struct PIInfo {
    pub pi_identifier: String,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub state: String, // "planning", "active", "completed"
    pub sprints: Vec<SprintInfo>,
    pub features: Vec<String>, // issue keys
}

pub struct ARTInfo {
    pub art_name: String,
    pub teams: Vec<String>,
    pub board_ids: Vec<u64>,
    pub current_pi: Option<String>,
}
```

**Custom Field Mapping:**
```rust
// Add to cache.rs or new safe_fields.rs
pub struct SAFeFieldMapping {
    pub pi_field: Option<String>,         // customfield_10100
    pub capability_field: Option<String>, // customfield_10101
    pub art_field: Option<String>,        // customfield_10102
    pub wsjf_field: Option<String>,       // customfield_10103
    pub business_value_field: Option<String>,
}
```

### 6.3 JQL Query Patterns

**PI-based queries:**
```jql
# All work in PI
fixVersion = "PI 2025.1" OR "PI Field" = "PI 2025.1"

# WIP in current PI
status = "In Progress" AND fixVersion in unreleasedVersions()

# Stories spanning sprints
Sprint in (X, Y, Z) AND type = Story

# Cross-team dependencies
project = TEAM1 AND issueFunction in linkedIssuesOf("project = TEAM2")

# Feature rollup
"Epic Link" = FEAT-123 AND status in ("To Do", "In Progress", "Done")
```

### 6.4 Performance Considerations

**Caching Strategy:**
- Cache PI information (30-minute TTL)
- Cache sprint patterns (list of sprints matching "PI 2025.1.*")
- Cache ART/team mappings (60-minute TTL)
- Cache dependency graphs (15-minute TTL)

**Query Optimization:**
- Use pagination for large result sets
- Implement parallel queries for multi-team searches
- Leverage bulk API where possible
- Add query result caching for common patterns

---

## Part 7: Example Workflows

### Workflow 1: "What's everyone working on in current PI?"

```
User: "Show me all active work in PI 2025.1"

MCP Flow:
1. detect_current_pi() → "PI 2025.1"
2. get_active_work(scope="current_pi", group_by="team")
3. Returns:
   - Team Alpha:
     - STORY-123: OAuth Implementation (Sprint 2, Jane)
     - STORY-456: Token Service (Sprint 2, Bob)
   - Team Beta:
     - STORY-789: User Profile API (Sprint 1, Alice)
     - STORY-234: Email Notifications (Sprint 3, Carol)
```

### Workflow 2: "Is Feature X on track?"

```
User: "Check Feature FEAT-456 progress"

MCP Flow:
1. get_feature_progress(feature_key="FEAT-456")
2. Fetches:
   - Feature metadata (PI 2025.1, Status: In Progress)
   - Child stories (10 total)
     - 4 Done
     - 3 In Progress
     - 3 To Do
   - Story points: 32 / 50 completed (64%)
   - Teams: Alpha (6 stories), Beta (4 stories)
   - Sprints: Sprint 1-3
   - Dependencies: 2 blocked items
3. Risk Analysis:
   - Behind schedule (expected 70% done by Sprint 3)
   - 2 blockers need attention
   - Team Beta at capacity
```

### Workflow 3: "Dependencies blocking PI delivery"

```
User: "Show PI 2025.1 dependency risks"

MCP Flow:
1. get_pi_dependency_map(pi="PI 2025.1", critical_only=true)
2. Returns:
   - STORY-123 blocks STORY-456 (cross-team)
     - Story-123: In Progress (Sprint 2, Team Alpha)
     - Story-456: To Do (Sprint 3, Team Beta) ← at risk!
   - STORY-789 blocks STORY-234 (same team)
     - Story-789: In Progress
     - Story-234: Waiting
3. Critical Path:
   - FEAT-100 → STORY-123 → STORY-456 → FEAT-200
   - Delay in STORY-123 impacts entire chain
```

---

## Part 8: Comparison with Jira Align

### What Jira Align Provides (That You'd Be Replicating)

| Feature | Jira Align | Your MCP Server (Proposed) |
|---------|------------|----------------------------|
| **PI Planning** | Full digital board, drag-drop | Search + bulk operations |
| **Hierarchy** | Native Capability → Feature → Story | Custom fields + links + queries |
| **Dependency Visualization** | Interactive graph | JSON dependency map (UI in Claude) |
| **ART Dashboard** | Real-time dashboards | On-demand queries |
| **Capacity Planning** | Built-in capacity mgmt | Query-based analysis |
| **Progress Tracking** | Automated rollups | Calculated rollups on-demand |
| **Cost** | $$$$ (enterprise licensing) | Free (self-hosted MCP) |
| **AI Integration** | None | Native AI conversation interface! |

**Your Advantage:** AI-native conversational interface
- "What stories are at risk in current PI?"
- "Show me cross-team dependencies for Feature X"
- "Which teams are overcommitted?"

---

## Part 9: Recommended Next Steps

### Immediate Actions (This Week)

1. **Analyze your JIRA setup**
   - Run `get_custom_fields` on sample issues
   - Identify PI field ID
   - Identify Capability/Feature hierarchy approach
   - Document sprint naming convention

2. **Create new GitHub issue**
   - Title: "SAFe Program Increment & Cross-Sprint Management"
   - Reference this analysis document
   - Break into sub-issues for each tool

3. **Implement Phase 1 foundation**
   - Priority: `search_issues_by_pi`
   - Priority: `get_active_work`
   - Priority: `search_sprints` with patterns

### Short Term (Next 2-4 Weeks)

1. Implement Phase 1 & 2 tools
2. Test with your real JIRA data
3. Gather feedback from teams

### Medium Term (Next 2-3 Months)

1. Complete all 5 implementation phases
2. Create documentation and examples
3. Share as reference MCP server for SAFe

### Long Term Vision

1. **Open source** this as "SAFe MCP Server"
2. **Community contributions** from other SAFe orgs
3. **Integration** with other SAFe tools
4. **AI agent workflows** for PI planning, dependency resolution, etc.

---

## Part 10: Questions to Answer

Before implementing, clarify:

1. **PI Field Configuration**
   - Do you use fixVersion or custom field?
   - What's the field ID?
   - Format: "PI 2025.1" or "2025-Q1-PI1"?

2. **Hierarchy Approach**
   - Do you use Initiative → Epic → Story (rename approach)?
   - Or Epic → Feature → Story (custom links)?
   - Custom field: "Capability"?

3. **Sprint Naming**
   - What's your current convention?
   - Do multiple ARTs have overlapping sprint names?
   - Example: "StreamAlpha - PI 2025.1.1" or "PI1-Sprint1"?

4. **Team Structure**
   - How many ARTs?
   - How many teams per ART?
   - Do teams share sprints or have independent sprints?

5. **Dependency Tracking**
   - Do you use standard "Blocks/Blocked By" links?
   - Any custom link types?
   - How do you mark cross-team vs same-team dependencies?

---

## Conclusion

**Issue #29 (Add advanced sprint lifecycle operations) is too narrow** for your needs.

**What you really need:** A comprehensive **SAFe Program Increment Management Suite** that includes:
1. Sprint lifecycle (create/start/close) - the easy part
2. PI-based search and filtering
3. Cross-sprint story tracking
4. Dependency mapping
5. Feature progress rollup
6. Active work (WIP) tracking
7. ART coordination tools
8. Bulk sprint operations

**This is a significant undertaking** - estimate 8-10 weeks for full implementation.

**Recommendation:** Start with Phase 1 (Foundation) to get immediate value, then iterate based on real usage.

---

## Appendix: Related Resources

- [Jira Align Documentation](https://help.jiraalign.com/)
- [SAFe 6.0 Framework](https://scaledagileframework.com/)
- [PI Planning Guide](https://www.atlassian.com/agile/agile-at-scale/pi-planning)
- [JIRA Advanced Roadmaps for SAFe](https://www.atlassian.com/software/jira/guides/roadmaps)
- [Creating SAFe Hierarchy in JIRA](https://www.forty8fiftylabs.com/tech-tip/creating-a-scaled-agile-hierarchy-within-jira/)
