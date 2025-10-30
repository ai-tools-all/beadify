
## Workflow for Working on Issues

### Standard Issue Workflow (Required for `kind=feature`)

When working on an issue, follow this lifecycle (example):

```bash
# 1. Mark issue as in_progress before starting work
beads update bd-015 --status in_progress

# 2. Write a short approach/plan (before writing code)
# Document the approach: what needs to be done, how, and any key considerations

# 3. Implement the solution
# ... make code changes, implement features, etc ...

# 4. When implementation is complete, mark for review
beads update bd-015 --status review

# 5. After user testing/validation, mark as closed
beads update bd-015 --status closed
```

**Note:** This workflow is **mandatory** for issues with `kind=feature`.

### Handling Discovered Issues During Implementation

If you discover a bug or improvement while working on an issue:

```bash
# Create a new issue dependent on the current issue
beads create --title "Fix edge case in sync" --kind bug --priority 2 --depends-on bd-015

# Continue working on the original issue, or switch to the new one
beads update bd-015 --status in_progress  # continue original work
# OR
beads update bd-016 --status in_progress  # switch to new issue
```