# 2025-10-31 Beads Gitignore Issue Notes

- 2025-10-31 01:41:19 Start session to track beads issues about ensuring `.beads/` database files are ignored.
- 2025-10-31 01:42:30 Reviewed `beads` CLI help for create/list commands; additional request to add JSON mode to `beads list` noted.
- 2025-10-31 01:42:56 Created issue `bd-01K8VC0M8VB52SGZCG2XJMB8HP` for ensuring `beads init` adds `.beads/.db` to `.gitignore`.
- 2025-10-31 01:42:56 Created issue `bd-01K8VC0T25Z9JZNSCX1AE3PBV5` to add JSON output mode to `beads list`.
- 2025-10-31 01:51:43 Implemented `.gitignore` enforcement during `beads init`, added regression tests, and ran `cargo test`.
- 2025-10-31 01:52:20 Closed issue `bd-01K8VC0M8VB52SGZCG2XJMB8HP` via `beads update --status closed`.
- 2025-10-31 01:53:56 Need follow-up issues: implement show command, refresh merge logic, support editing spec via `$EDITOR`.

## Tasks

- [x] Review beads CLI usage for issue creation
- [x] Create issue covering `.beads/` ignore requirements
- [x] Create issue for JSON mode on `beads list`
- [x] Ensure `beads init` adds `.beads/beads.db` to `.gitignore`
- [x] Run `cargo test`
- [x] Close issue `bd-01K8VC0M8VB52SGZCG2XJMB8HP`
- [ ] Create issue for show command implementation
- [ ] Create issue for merge logic update
- [ ] Create issue for editing spec in `$EDITOR`
