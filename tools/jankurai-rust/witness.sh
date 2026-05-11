#!/usr/bin/env bash
set -euo pipefail
repo_root="${1:-.}"
exec jankurai rust witness build "$repo_root"
