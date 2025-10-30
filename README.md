# Beads CLI Cheatsheet (for LLM agents)

```bash
# initialize a repo in the current directory
beads init

# create an issue (defaults: kind=task, priority=2)
beads create --title "Fix sync" --kind bug --priority 1

# update fields on an existing issue
beads update bd-01 --status in_progress --priority 3 --title "Refine sync driver"

# list cached issues (offline, from SQLite materialized view)
beads list

# reconcile with git remote and apply new events (use --full for full rebuild)
beads sync          # runs git pull → apply incremental log events → git push
beads sync --full   # full log replay into a fresh cache
```

Key behavior reminders:

- All mutations append to `.beads/events.jsonl` and update `.beads/beads.db` immediately (offline-first).
- `beads sync` is the only command touching the network; it performs pull → reconcile → push.
- The merge driver binary `beads-merge-driver` must be configured as `merge=beadslog` for `*.jsonl` files in `.gitattributes`.
