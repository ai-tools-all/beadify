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
