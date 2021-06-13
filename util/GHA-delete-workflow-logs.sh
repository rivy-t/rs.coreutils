#! /bin/bash
# ref: <https://stackoverflow.com/questions/57927115/anyone-know-a-way-to-delete-a-workflow-from-github-actions>

user=GH_USERNAME
repo=REPO_NAME
gh api repos/$user/$repo/actions/runs | jq -r '.workflow_runs[] | select(.head_branch != "master") | "\(.id)"' | xargs -n1 -I % gh api repos/$user/$repo/actions/runs/% -X DELETE
