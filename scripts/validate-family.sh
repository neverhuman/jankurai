#!/usr/bin/env bash
set -euo pipefail

hub="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
split_root="$(cd "${hub}/.." && pwd)"
manifest="${hub}/repos.manifest.toml"
lock="${hub}/family.lock"

fail() {
  printf 'validate-family: %s\n' "$*" >&2
  exit 1
}

need_file() {
  local path="$1"
  [[ -f "$path" ]] || fail "missing file: $path"
}

need_dir() {
  local path="$1"
  [[ -d "$path" ]] || fail "missing directory: $path"
}

repo_names() {
  awk -F '"' '/^name = "/ { print $2 }' "$manifest"
}

manifest_tags() {
  awk -F '"' '
    /^name = "/ { name=$2 }
    /^tag = "/ { print name " " $2 }
  ' "$manifest"
}

need_file "$manifest"

while read -r repo; do
  [[ -n "$repo" ]] || continue
  dir="${split_root}/${repo}"
  need_dir "$dir"
  need_file "${dir}/AGENTS.md"
  need_file "${dir}/SPLIT.md"
  need_file "${dir}/agent/owner-map.json"
  need_file "${dir}/agent/test-map.json"
  need_file "${dir}/agent/generated-zones.toml"
  need_file "${dir}/agent/standard-version.toml"
  need_file "${dir}/agent/split-member.toml"
  need_file "${dir}/.jeryu/repo.toml"
  need_file "${dir}/scripts/ci-local.sh"
  need_file "${dir}/ops/ci/required.sh"
  jq empty "${dir}/agent/owner-map.json" >/dev/null
  jq empty "${dir}/agent/test-map.json" >/dev/null
  grep -q "repo = \"root/${repo}\"" "${dir}/.jeryu/repo.toml" \
    || fail "${repo}: .jeryu/repo.toml does not declare root/${repo}"
  grep -q "git@github.com:neverhuman/${repo}.git" "${dir}/.jeryu/repo.toml" \
    || fail "${repo}: .jeryu/repo.toml does not declare GitHub mirror"
done < <(repo_names)

if rg -n 'branch\s*=' "${split_root}" --glob 'Cargo.toml' --glob 'package.json' --glob '!**/.git/**'; then
  fail "branch dependencies are forbidden in committed manifests"
fi

while IFS= read -r cargo_toml; do
  repo_root="$(dirname "$cargo_toml")"
  while [[ "$repo_root" != "$split_root" && "$(dirname "$repo_root")" != "$split_root" ]]; do
    repo_root="$(dirname "$repo_root")"
  done
  while IFS= read -r line; do
    dep_path="$(printf '%s\n' "$line" | sed -n 's/.*path[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p')"
    [[ -n "$dep_path" ]] || continue
    [[ "$dep_path" == ../* || "$dep_path" == /* ]] || continue
    target="$(realpath -m "$(dirname "$cargo_toml")/${dep_path}")"
    case "$target" in
      "${repo_root}"/*) ;;
      *) fail "committed cross-repo Cargo path dependency in ${cargo_toml}: ${dep_path}" ;;
    esac
  done < <(grep -nE 'path[[:space:]]*=' "$cargo_toml" || true)
done < <(find "${split_root}" -path '*/.git' -prune -o -name Cargo.toml -print)

while IFS= read -r manifest_file; do
  repo_dir="$(cd "$(dirname "$manifest_file")" && pwd)"
  if [[ ! -f "${repo_dir}/Cargo.lock" ]]; then
    fail "missing Cargo.lock next to ${manifest_file}"
  fi
done < <(find "${split_root}" -mindepth 2 -maxdepth 2 -name Cargo.toml -print)

while IFS= read -r package_file; do
  repo_dir="$(cd "$(dirname "$package_file")" && pwd)"
  if [[ ! -f "${repo_dir}/package-lock.json" ]]; then
    fail "missing package-lock.json next to ${package_file}"
  fi
done < <(find "${split_root}" -mindepth 2 -maxdepth 2 -name package.json -print)

if find "${split_root}" -mindepth 3 -maxdepth 4 -path '*/.github/workflows/*' -type f | grep -q .; then
  while IFS= read -r workflow; do
    if grep -Eq 'uses: [^@]+@[A-Za-z0-9_.-]+$' "$workflow" \
      && ! grep -Eq 'uses: [^@]+@[0-9a-f]{40}$' "$workflow"; then
      fail "workflow action is not pinned to a 40-character SHA: ${workflow}"
    fi
  done < <(find "${split_root}" -mindepth 3 -maxdepth 4 -path '*/.github/workflows/*' -type f)
fi

if [[ -f "$lock" ]]; then
  while read -r repo tag; do
    [[ -n "${repo:-}" && -n "${tag:-}" ]] || continue
    [[ "$repo" == "jankurai" ]] && continue
    grep -q "repo = \"${repo}\"" "$lock" || fail "family.lock missing ${repo}"
    grep -q "tag = \"${tag}\"" "$lock" || fail "family.lock tag drift for ${repo}"
    if [[ -d "${split_root}/${repo}/.git" ]]; then
      head="$(git -C "${split_root}/${repo}" rev-parse HEAD)"
      grep -q "commit = \"${head}\"" "$lock" || fail "family.lock SHA mismatch for ${repo}"
    fi
  done < <(manifest_tags)
fi

printf 'validate-family: ok\n'
