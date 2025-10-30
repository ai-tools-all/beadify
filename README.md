# Beads CLI Cheatsheet (for LLM agents)

```bash
# Initialize a repo in the current directory
beads init --prefix bd

# Create an issue (requires --title and --data as JSON)
beads create --title "Fix sync" --data '{"kind":"bug","priority":1}'
beads create --title "Add feature" --data '{"kind":"feature","priority":2}' --depends-on bd-01

# Show issue details (including dependencies)
beads show bd-01

# List cached issues (offline, from SQLite materialized view)
beads list                    # shows only open issues
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

Key behavior reminders:

- All mutations append to `.beads/events.jsonl` and update `.beads/beads.db` immediately (offline-first).
- `beads sync` is the only command touching the network; it performs pull → reconcile → push.
- The merge driver binary `beads-merge-driver` must be configured as `merge=beadslog` for `*.jsonl` files in `.gitattributes`.
