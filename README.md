# Beads CLI Cheatsheet (for LLM agents)

```bash
# Initialize a repo in the current directory
beads init --prefix bd

# Create an issue (requires --title and --data as JSON)
beads create --title "Fix sync" --data '{"kind":"bug","priority":1}'
beads create --title "Add feature" --data '{"kind":"feature","priority":2}' --depends-on bd-01

# Show issue details (includes "Blocked By" dependencies)
beads show bd-01

# Manage dependencies
beads dep show bd-02               # show blockers and dependents
beads dep add bd-02 bd-01          # bd-02 depends on bd-01
beads dep remove bd-02 bd-01       # remove dependency

# List cached issues (offline, from SQLite materialized view)
beads list                    # shows only open issues (with blockers indented)
beads list --all              # shows all issues including closed
beads list --status in_progress  # filter by status

# Search issues by text query
beads search "sync"                              # search in all fields
beads search "bug" --kind bug --status open      # search with filters
beads search "fix" --priority 1 --title-only     # search only in titles

# Show next issue to work on (grouped by priority)
beads ready

# Update fields on an existing issue
beads update bd-01 --status in_progress --priority 3 --title "Refine sync driver"

# Delete issues (soft delete with status="deleted")
beads delete bd-01                              # preview mode (shows impact)
beads delete bd-01 --force                      # confirm deletion
beads delete bd-01 bd-02 bd-03 --force          # batch deletion
beads delete --from-file deletions.txt --force  # delete from file
beads delete bd-01 --cascade --force            # delete with dependents

# Reconcile with git remote and apply new events
beads sync          # runs git pull → apply incremental log events → git push
beads sync --full   # full log replay into a fresh cache
```

## Output Format

### List Format
```
   ID       Kind       Prio Title
────────────────────────────────────────────────────────────────────────────────
☐ bd-001   task       p1   Implement show command
☐ bd-025   feature    p2   Implement beads dep command...
    ↳ bd-001   task       p1 - Implement show command
    ↳ bd-002   task       p2 - Update merge logic
● bd-032   feature    p2   Implement beads dep add/remove commands
```

- `☐` = open/in-progress, `●` = closed
- Dependencies shown indented with `↳` arrow

### Show Format
```
ID:       bd-025
Title:    Implement beads dep command...
Status:   open
Kind:     feature
Priority: 2

Blocked By:
  ↳ bd-001 [open] p1 - Implement show command
  ↳ bd-002 [open] p2 - Update merge logic
```

```bash
# 1. Pick next issue
beads ready

# 2. Start working
beads update bd-041 --status in_progress

# 3. View issue & blockers
beads show bd-041

# 4. Check dependencies
beads dep show bd-041

# 5. When done, commit
git add src/file.rs
git commit -m "fix(bd-041): your description"

# 6. Mark as done
beads update bd-041 --status closed
```

## Document Management

Beads supports attaching documents to issues using a blob store with an ephemeral workspace for editing.

### Attach documents when creating an issue
```bash
beads create --title "API Design" \
  --data '{"kind":"feature","priority":1}' \
  --doc "spec:./docs/api-spec.md" \
  --doc "notes:./notes.txt"
```

### Add documents to existing issues
```bash
beads doc add bd-042 ./research/findings.md
```

### Edit-Sync Workflow

Documents are stored in an immutable blob store (`.beads/blobs/`). To edit:

**1. Check out document to workspace:**
```bash
beads doc edit bd-042 findings.md
# Exports to .beads/docs/bd-042/findings.md
```

**2. Edit the file** in `.beads/docs/bd-042/findings.md` with your editor

**3. Check in changes:**
```bash
beads doc sync bd-042 findings.md
# Prompts: Clean up workspace file? [y/N]
```

The sync command:
- Calculates new hash and stores updated content in blob store
- Updates issue metadata to point to new hash
- Optionally cleans up the workspace file

**List all documents on an issue:**
```bash
beads doc list bd-042
```

**Notes:**
- `.beads/blobs/` is permanent and tracked in Git (source of truth)
- `.beads/docs/` is temporary and gitignored (workspace only)
- Same content = same hash = automatic deduplication
- Document history preserved through event log

## Issue Deletion (Soft Delete)

Beads implements soft deletion - issues are marked with `status="deleted"` in events.jsonl and excluded from the database.

### Preview Mode (Default)
```bash
beads delete bd-042
# Shows impact: issues to delete, dependents, text references
# Requires --force to confirm
```

### Single Issue Deletion
```bash
beads delete bd-042 --force
# Sets status="deleted" in events.jsonl
# Removes from SQLite cache
# Updates text references: bd-042 → [deleted:bd-042]
```

### Batch Deletion
```bash
# Delete multiple issues
beads delete bd-001 bd-002 bd-003 --force

# Delete from file (one ID per line, # for comments)
echo "bd-001" > to-delete.txt
echo "bd-002" >> to-delete.txt
beads delete --from-file to-delete.txt --force
```

### Cascade Deletion
```bash
# Recursively delete all dependent issues
beads delete bd-042 --cascade --force
# Prompts for confirmation
# Deletes in topological order (leaves first)
```

**How it works:**
- Issues remain in `events.jsonl` with `status="deleted"` (audit trail)
- Removed from SQLite during event replay
- Text references updated to `[deleted:ID]` format
- Dependencies and labels automatically cleaned up
- Blobs remain in storage (same content might be reused)

**Future recovery:**
- Can implement `beads undelete` to restore issues
- Compaction can physically remove after retention period

## Git Commit Guidelines

When committing changes for an issue:

```bash
# 1. Only add files changed for THIS specific issue (never use git add .)
git add path/to/file1.rs path/to/file2.rs

# 2. Commit with issue number in title (conventional commit format)
git commit -m "fix(bd-015): resolve sync driver race condition"
git commit -m "feat(bd-023): add search filter by priority"
git commit -m "refactor(bd-018): simplify event parsing logic"
```

**Commit format:** `<type>(<issue-id>): <short description>`
- **Types:** `fix`, `feat`, `refactor`, `docs`, `test`, `chore`
- **Issue ID:** Always include (e.g., `bd-015`)
- **Description:** 50-72 chars recommended

**Rules:**
- ✅ Always include issue number in commit title
- ✅ Only commit files changed for current issue (never `git add .`)
- ✅ Use conventional commit format
- ✅ Keep titles short and focused

## Key Reminders

- All mutations append to `.beads/events.jsonl` and update `.beads/beads.db` immediately (offline-first).
- `beads sync` is the only command touching the network; it performs pull → reconcile → push.
- The merge driver binary `beads-merge-driver` must be configured as `merge=beadslog` for `*.jsonl` files in `.gitattributes`.
- **Closed dependencies are filtered** from all blocker displays (`beads show`, `beads dep show`, `beads list`) since closed issues don't block progress.
