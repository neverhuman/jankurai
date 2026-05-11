#!/bin/sh
git config core.hooksPath /dev/null
cp hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
