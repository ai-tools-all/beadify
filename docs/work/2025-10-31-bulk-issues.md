use beads cli to create these issues. assume beads repository is initialised.

## beads cli 

1. beads list --json => to support json format
2. beads list =>  do not show closed by default. only show open. 
3. beads list --dep-graph => to show the dependency graph
4. beads spec edit 



## beads create 

1. allow to create the data as well -- so that the issue description can be written in data field.

## beads init 

1. beads doctor command -- this is issue. nested list indicates implemenation details
   1. check if repo is initalised properly 
   2. sanity check via beads list command 
   3. check if .gitignore has proper files .beads/beads.db


## beads search command 
1. sqlite full text search -- to find the related issue described by the user --  in the title / data    
   1. beads search "my concerned topic"



## UI polish 
1. the beads list --all flag  => mark as empty [ ] and closed as green dot 



## beads dependency command 
1. beads dep => to show if any issue blocks implementation of this issue.


## package 
1. gh workflow for building for mac + linux arm + linux amd64 
2. with version via git tags?


## beads update 
- allow for updating the --data -> see the flag during creation of the beads create -- same flags for title and data in beads update.




## beads show
beads show bd-042
ID:       bd-042
Title:    Show status circles for main issue and dependencies in dep show command
Status:   open
Kind:     feature
Priority: 2

should show all the metadta / data in kv format. 



--------------------
-- yet to create 
--------------------


create issue - with feature + dependency using beads cli. only create i9ssue. do not implement this yet. 



for example 

beads dep show bd-041
Dependencies for bd-041 - Filter closed dependencies from blocker display

Blockers (Issues this depends on):
  ↳ bd-025 [closed] p2 - Implement beads dep command to show issue dependencies and blockers


this should also show the open /closed status of bd-041 



## beads dep show 
- close and open status to be shown consistently in the app via open and close circle.



## beads ready command 
1. algorithm which shows the next issue to work on. highlights the next open issue and groups them with priority list.


## beads list - 
1. show total count of issues 
2. do not show `[open]` written.  we already have the square icon. change that to open circle.


## labels - structured metadata for filtering querying
- read the docs/work/2025-10-31-labels.md and crate sub issues for implementing this feature. add dependency between issues as needed. do not create cyclic dependencies.