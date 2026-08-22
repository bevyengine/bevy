#!/bin/bash


echo "Checking PRs with S-Merge-Conflicts if they are mergeable"

gh pr list --repo bevyengine/bevy --json number,mergeable,title --label S-Merge-Conflicts -L 10000 > yes_conflict_label.json

YES_LABEL_UNKNOWN=$(jq -r 'map(select(.mergeable == "UNKNOWN")) | length' yes_conflict_label.json)

jq -r '.[] | select(.mergeable == "MERGEABLE") | .number' yes_conflict_label.json | while read -r number; do
    echo "Removing S-Merge-Conflicts from $number"
    gh pr edit $number --repo bevyengine/bevy --remove-label S-Merge-Conflicts
done



echo "Checking PRs without S-Merge-Conflicts if they have conflicts"

gh pr list --repo bevyengine/bevy --json number,mergeable,title --search "-label:S-Merge-Conflicts" -L 10000 > no_conflict_label.json

NO_LABEL_UNKNOWN=$(jq -r 'map(select(.mergeable == "UNKNOWN")) | length' no_conflict_label.json)

jq -r '.[] | select(.mergeable == "CONFLICTING") | .number' no_conflict_label.json | while read -r number; do
    echo "Adding S-Merge-Conflicts to $number"
    gh pr edit $number --repo bevyengine/bevy --add-label S-Merge-Conflicts
done



if [[ "$YES_LABEL_UNKNOWN" -gt 0 ]]; then
    echo "Found $YES_LABEL_UNKNOWN with mergeable=UNKNOWN, please re-run"
fi

if [[ "$NO_LABEL_UNKNOWN" -gt 0 ]]; then
    echo "Found $NO_LABEL_UNKNOWN with mergeable=UNKNOWN, please re-run"
fi

echo "Done"
