# DateTime Query Support

The beads CLI supports filtering issues by creation date using relative or absolute date expressions. Timestamps are stored in UTC but automatically converted to your local timezone for queries and display.

## Quick Start

```bash
# Issues created in the past week
beads issue list --created-after "1 week ago"

# Issues created in January
beads issue list --created-after "2026-01-01" --created-before "2026-02-01"

# Use your timezone
beads issue list --created-after "1 week ago" --timezone "America/New_York"
```

## Syntax

### Relative Dates (from now)

```bash
beads issue list --created-after "1 day ago"
beads issue list --created-after "2 weeks ago"
beads issue list --created-after "3 months ago"
```

Supported units: days, weeks, months, years

### Absolute Dates (ISO 8601 or YYYY-MM-DD)

```bash
beads issue list --created-after "2026-01-20"
beads issue list --created-after "2026-01-20T10:00:00Z"
```

### Combined Filters

```bash
beads issue list --created-after "2026-01-01" --created-before "2026-01-31"
```

## Timezone Support

Timestamps are stored in UTC internally, but queries respect your local timezone.

### Auto-Detection (priority order)

1. `--timezone` flag: Explicit override
2. `TZ` environment variable: System-wide setting
3. System timezone: Auto-detected from `/etc/timezone` (Linux)
4. Fallback: UTC

### Examples

```bash
# User in EST (America/New_York)
# "1 week ago" means 1 week from NOW (in your timezone)
beads issue list --created-after "1 week ago"

# Explicit timezone override
beads issue list --created-after "1 week ago" --timezone "Europe/London"

# Using environment variable
export TZ=Asia/Tokyo
beads issue list --created-after "3 days ago"
```

## Storage & Display

- **Internal:** UTC timestamps in ISO 8601 format (e.g., 2026-01-31T15:30:00Z)
- **JSON output:** Both UTC and local timezone shown
  ```json
  {
    "created_at_utc": "2024-01-01T00:00:00Z",
    "created_at_local": "2024-01-01 05:30 Asia/Kolkata"
  }
  ```
- **Database:** Stored in `created_at` column (TEXT, UTC)

## Automatic Migration

**No user action required.** When you upgrade and run `beads sync`:

1. Detects old database schema (no `created_at` column)
2. Reads event timestamps from `events.jsonl`
3. Populates `created_at` with first event timestamp for each issue
4. Updates schema version in metadata

Progress is shown:
```
Detected old database schema. Migrating timestamps from events.jsonl...
Applied 150 events
```

### Manual Rebuild

If you need to rebuild the cache for any reason:

```bash
beads sync --force
```

This rebuilds the entire SQLite cache from `events.jsonl` and recalculates all timestamps.

## CLI Reference

### `beads issue list` flags

| Flag | Description |
|------|-------------|
| `--created-after <DATE>` | Filter issues created after this date |
| `--created-before <DATE>` | Filter issues created before this date |
| `--timezone <TZ>` | Timezone for date parsing (e.g., "America/New_York") |

### Examples

```bash
# List all open issues from the past 7 days
beads issue list --created-after "1 week ago"

# List issues created in a specific date range
beads issue list --created-after "2026-01-01" --created-before "2026-01-31"

# List urgent issues from the past month
beads issue list --priority urgent --created-after "1 month ago"

# List features created this year (in Pacific timezone)
beads issue list --kind feature --created-after "2026-01-01" --timezone "America/Los_Angeles"
```

## Implementation Details

The datetime query feature consists of:

- **`crates/beads-core/src/tz.rs`**: Timezone detection and conversion
- **`crates/beads-core/src/query.rs`**: Date parsing helpers (`parse_date`, `created_after`, `created_before`)
- **`crates/beads-core/src/db.rs`**: Database query functions (`get_issues_created_after`, `get_issues_created_between`)
- **`crates/beads/src/commands/issue/list.rs`**: CLI integration with `--created-after`, `--created-before`, and `--timezone` flags
