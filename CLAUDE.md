
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

# 6. Commit only the files changed for this issue with the issue name in the commit message
git add path/to/file1.rs path/to/file2.rs
git commit -m "fix(bd-015): resolve sync driver race condition"
```

**Note:** This workflow is **mandatory** for issues with `kind=feature`.

## Git Commit Guidelines

**IMPORTANT:** When committing changes for an issue:

```bash
# 1. Only add files changed for THIS specific issue (never use git add .)
git add path/to/file1.rs path/to/file2.rs

# 2. Commit with issue number in title (conventional commit format)
git commit -m "fix(bd-015): resolve sync driver race condition"
git commit -m "feat(bd-023): add search filter by priority"
git commit -m "refactor(bd-018): simplify event parsing logic"
```

**Commit format:**
- **Format:** `<type>(<issue-id>): <short description>`
- **Types:** `fix`, `feat`, `refactor`, `docs`, `test`, `chore`
- **Issue ID:** Always include the issue number (e.g., `bd-015`)
- **Description:** Keep it short and descriptive (50-72 chars recommended)

**Rules:**
- ✅ **Always include issue number** in commit title
- ✅ **Only commit files changed** for the current issue (never use `git add .`)
- ✅ **Use conventional commit format** (`type(issue-id): description`)
- ✅ **Keep titles short** and focused

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