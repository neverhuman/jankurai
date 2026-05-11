#!/bin/sh
set -eu
git status --porcelain
git diff --quiet
git add src/main.rs
git commit -m "update docs"
