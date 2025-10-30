implement sync command as git  hooks

The Git Hooks Approach (Recommended)
This involves using standard Git hooks to trigger beads sync at specific, predictable moments in the Git workflow.
How it would work:
You would provide two small scripts for the user to place in their .git/hooks/ directory. beads init could even offer to install them automatically.
post-merge hook: This hook runs automatically after a successful git merge. Since git pull is effectively a git fetch followed by a git merge, this is the perfect time to process incoming events. The hook would simply contain the line beads sync --no-push.
pre-push hook: This hook runs before git push. It's a good place to ensure your local events are reconciled and ready to be sent. It could run the full beads sync to push any merged changes.
Pros:
Simplicity and Low Overhead: It's just a couple of simple scripts. There's no long-running process to manage.
Context-Aware and Predictable: Syncing happens at the exact moments it's needed: after pulling remote changes and before pushing local ones. The user understands the "why."
Immediate Feedback: The output of beads sync (or any errors) appears directly in the terminal as part of the git pull or git push command they just ran. This is incredibly valuable.
Safe: It doesn't run at unexpected times. It's tied directly to the user's intent to interact with the remote.
Cons:
User Setup Required: Git hooks are not cloned with a repository for security reasons. Each user must install them. A helper command (beads init --install-hooks) is almost mandatory to lower this barrier.
Can be Bypassed: A user can run git pull --no-verify or git push --no-verify to skip hooks, potentially leaving their local DB out of sync until the next manual or hook-triggered sync.
GUI Clients: Some Git GUI clients have inconsistent or poor support for running and displaying output from hooks.