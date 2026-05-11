#!/bin/sh
git reset --hard HEAD~1
git clean -fdx
git stash -u
git add .
git commit -am "wip"
git push --force
git branch -D feature
git remote set-url origin https://token@example.com/org/repo.git
git worktree remove --force ../tmp
