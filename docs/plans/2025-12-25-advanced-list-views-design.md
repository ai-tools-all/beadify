# bd-078: Advanced List Views Design

## Summary

Replace `--dep-graph` with unified `--view` flag supporting three view modes:
- `epic` (default) - issues grouped under parent epics
- `flat` - current table view
- `tree` - full dependency hierarchy

## CLI Changes

```bash
# New behavior
beads list                    # epic view (default), open issues only
beads list --view flat        # current table view
beads list --view tree        # dependency tree
beads list --view epic        # explicit epic view

# Removed
beads list --dep-graph        # DEPRECATED, use --view tree
```

## View Specifications

### Epic View (default)

Groups issues by their parent epic. An issue belongs to an epic if it `depends_on` that epic.

```
── bd-078 (p1) Implement advanced list views ──────────────
☐ bd-079   feature    p2   Update CLI arguments...
☐ bd-080   feature    p2   Add database helper functions...
☐ bd-081   refactor   p3   Refactor list.rs formatting...
☐ bd-082   feature    p2   Implement tree view rendering...
☐ bd-083   feature    p2   Implement group-by epic view...

── bd-071 (p1) Implement dual configuration system ────────
☐ bd-072   feature    p2   Add config crate dependencies...
☐ bd-073   feature    p2   Create config module...
☐ bd-074   feature    p2   Integrate Settings with CLI...
☐ bd-075   feature    p2   Add database functions...
☐ bd-076   feature    p2   Define bd config CLI subcommands...
☐ bd-077   feature    p2   Implement bd config command...

── Ungrouped ──────────────────────────────────────────────
☐ bd-002   task       p2   Update merge logic
☐ bd-006   task       p1   Implement beads-merge-driver
☐ bd-013   feature    p3   beads init - beads doctor command
```

**Rules:**
- Epic sections sorted by priority (p1 first), then by ID
- Issues within epic sorted by priority, then by ID
- Issues depending on multiple epics: show under first epic (by ID sort)
- Ungrouped section at bottom for issues not depending on any epic

### Flat View

Current table view, no changes except flag rename.

```
   ID       Kind       Prio Title
──────────────────────────────────────────────────────────────────────
☐ bd-002   task       p2   Update merge logic
☐ bd-006   task       p1   Implement beads-merge-driver
    ↳ bd-071   epic       p1 - Implement dual configuration system
☐ bd-013   feature    p3   beads init - beads doctor command
...
```

### Tree View

Full dependency hierarchy. Shows parent→child relationships.

```
☐ bd-078   epic    p1   Implement advanced list views
  ├─ ☐ bd-079   feature  p2   Update CLI arguments...
  │    └─ ☐ bd-082   feature  p2   Implement tree view...
  ├─ ☐ bd-080   feature  p2   Add database helper...
  │    └─ ☐ bd-083   feature  p2   Implement group-by epic...
  └─ ☐ bd-081   refactor p3   Refactor list.rs...
       ├─ ☐ bd-082   feature  p2   Implement tree view... (→)
       └─ ☐ bd-083   feature  p2   Implement group-by... (→)
```

**Rules:**
- Root nodes: issues with no open dependencies (or all deps closed)
- Children: issues that `depend_on` the parent
- Multi-parent issues: show fully under first parent, show `(→)` reference under others
- Sorted by priority within each level

## Implementation Plan

### Phase 1: CLI & Data Structures

**File: `crates/beads/src/main.rs`**
- Add `ViewMode` enum: `Epic`, `Flat`, `Tree`
- Replace `--dep-graph: bool` with `--view: Option<ViewMode>`
- Default to `ViewMode::Epic` when `--view` not specified

### Phase 2: Database Helpers

**File: `crates/beads-core/src/db.rs`**
- Add `get_epics()` - returns all issues where `kind = 'epic'`
- Add `get_children()` - returns issues that depend_on a given issue (inverse of get_dependencies)
- Rename existing `get_dependents()` for clarity if needed

### Phase 3: List Command Refactor

**File: `crates/beads/src/commands/list.rs`**

```rust
pub fn run(
    repo: BeadsRepo,
    show_all: bool,
    status_filter: Option<String>,
    view_mode: ViewMode,        // NEW: replaces dep_graph
    label_filter: Option<String>,
    label_any_filter: Option<String>,
    json_output: bool,
    show_labels: bool,
) -> Result<()>
```

**New functions:**
- `render_epic_view()` - group and display by epic
- `render_flat_view()` - current table logic (extract from existing)
- `render_tree_view()` - recursive tree rendering

### Phase 4: Rendering Utilities

**File: `crates/beads/src/commands/list.rs`** (or new `render.rs`)

```rust
struct TreeNode {
    issue: Issue,
    children: Vec<TreeNode>,
    is_reference: bool,  // true if shown elsewhere as primary
}

fn build_dependency_tree(issues: &[Issue], deps: &HashMap<String, Vec<String>>) -> Vec<TreeNode>;
fn render_tree_node(node: &TreeNode, prefix: &str, is_last: bool);
```

## Migration

- `--dep-graph` flag: Keep as hidden alias for `--view tree` for one release cycle
- Document change in README/changelog

## Testing

1. Epic view with multiple epics
2. Epic view with ungrouped issues
3. Tree view with multi-parent issues (verify `→` reference)
4. Tree view cycle detection (shouldn't happen, but guard)
5. All views with `--status`, `--label` filters
6. JSON output compatibility with all views
