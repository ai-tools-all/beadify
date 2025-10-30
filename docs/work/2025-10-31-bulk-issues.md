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



-- yet to create 


## beads ready command 
1. algorithm which shows the next issue to work on. highlights the next open issue and groups them with priority list.


## beads list - 
1. list based on task filters  - task status or task type 
2. show total count of issues 


## package 
1. gh workflow for building for mac + linux arm + linux amd64 
2. with version via git tags?


## labels - structured metadata for filtering querying
- read the docs/work/2025-10-31-labels.md and crate sub issues for implementing this feature.