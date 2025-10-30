Statistics Command
Purpose
The bd stats command provides aggregate statistics about the issue tracker, giving a high-level view of issue distribution, workflow health, and system activity.

Usage
bd stats [flags]
Command Flags
Flag	Type	Description	Default
--json	bool	Output in JSON format	false
Metrics Provided
The statistics command calculates and displays:

Metric	Description	Query Method
Total Issues	Count of all issues	COUNT(*)
Open Issues	Issues with status='open'	SUM(CASE WHEN status='open')
In Progress Issues	Issues with status='in_progress'	SUM(CASE WHEN status='in_progress')
Closed Issues	Issues with status='closed'	SUM(CASE WHEN status='closed')
Blocked Issues	Issues with open blockers	Complex join with dependencies
Ready Issues	Issues with no open blockers	Subquery with NOT EXISTS
Average Lead Time	Average hours from creation to closure	Date arithmetic on closed issues


Example Output
Issue Statistics:
  Total Issues:      47
  Open:              12
  In Progress:       8
  Closed:            27
  Blocked:           5
  Ready:             7

Workflow Metrics:
  Average Lead Time: 18.5 hours
Examples
# Display statistics
bd stats

# Get statistics in JSON format
bd stats --json


-----------------


materialsed views 
SQL Views for Work Management
The schema defines two materialized views that support work management queries:

ready_issues View
Defines the ready work calculation as a reusable view:

CREATE VIEW IF NOT EXISTS ready_issues AS
SELECT i.*
FROM issues i
WHERE i.status = 'open'
  AND NOT EXISTS (
    SELECT 1 FROM dependencies d
    JOIN issues blocked ON d.depends_on_id = blocked.id
    WHERE d.issue_id = i.id
      AND d.type = 'blocks'
      AND blocked.status IN ('open', 'in_progress', 'blocked')
  );
Sources: 
internal/storage/sqlite/schema.go
74-85

blocked_issues View
Defines the blocked issues calculation with blocker counts:

CREATE VIEW IF NOT EXISTS blocked_issues AS
SELECT
    i.*,
    COUNT(d.depends_on_id) as blocked_by_count
FROM issues i
JOIN dependencies d ON i.id = d.issue_id
JOIN issues blocker ON d.depends_on_id = blocker.id
WHERE i.status IN ('open', 'in_progress', 'blocked')
  AND d.type = 'blocks'
  AND blocker.status IN ('open', 'in_progress', 'blocked')
GROUP BY i.id;
Sources: 
internal/storage/sqlite/schema.go
88-98