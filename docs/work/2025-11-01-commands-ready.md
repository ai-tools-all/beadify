Ready Work Command
Purpose
The bd ready command identifies issues that are available for work by filtering out any issues that have open blocking dependencies. This is the primary command for finding actionable work in the system.

Usage
bd ready [flags]
Command Flags
Flag	Type	Description	Default
--limit	int	Maximum number of issues to return	Unlimited
--priority	int	Filter by priority (0-4)	All priorities
--assignee	string	Filter by assignee	All assignees
--json	bool	Output in JSON format	false
Examples
# Get all ready work
bd ready

# Get the top 5 ready issues
bd ready --limit 5

# Get ready work for a specific assignee
bd ready --assignee alice

# Get high-priority ready work
bd ready --priority 0

# Get ready work in JSON format (for AI agents)
bd ready --json --limit 1
The Ready Work Algorithm
An issue is considered "ready" if and only if:

Its status is open
It has no dependencies where:
The dependency type is blocks
The blocking issue's status is open, in_progress, or blocked


Implementation
The ready work query is implemented in GetReadyWork() using a SQL subquery with a NOT EXISTS clause to efficiently exclude issues with open blockers:

SELECT i.*
FROM issues i
WHERE i.status = ?
  AND NOT EXISTS (
    SELECT 1 FROM dependencies d
    JOIN issues blocked ON d.depends_on_id = blocked.id
    WHERE d.issue_id = i.id
      AND d.type = 'blocks'
      AND blocked.status IN ('open', 'in_progress', 'blocked')
  )
ORDER BY i.priority ASC, i.created_at DESC
The query accepts a WorkFilter parameter that allows filtering by:

Status (defaults to open if not specified)
Priority
Assignee
Limit
Sources: 
internal/storage/sqlite/ready.go
13-70
 
internal/types/filters.go

Dependency Type Significance
Only dependencies with type = 'blocks' affect ready work calculation. The other dependency types have no impact:

Dependency Type	Affects Ready Work?	Purpose
blocks	✅ Yes	Hard blocker - issue cannot start
related	❌ No	Informational relationship
parent-child	❌ No	Epic/subtask hierarchy
discovered-from	❌ No	Work discovery tracking
