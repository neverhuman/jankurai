#!/usr/bin/env bash
set -euo pipefail

hub="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
split_root="$(cd "${hub}/.." && pwd)"
manifest="${hub}/repos.manifest.toml"
lock="${hub}/family.lock"
projection_tool="${hub}/scripts/project-family-manifest.sh"

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

manifest_rows() {
  awk -F '"' '
    function emit() {
      if (name != "") {
        print name "\t" path "\t" slug "\t" default_branch "\t" required_check \
          "\t" hosted "\t" mirror_github "\t" tag "\t" jeryu "\t" github
      }
    }
    /^\[\[repo\]\]$/ { emit(); name=""; path=""; slug=""; default_branch=""; required_check=""; hosted=""; mirror_github=""; tag=""; jeryu=""; github=""; next }
    /^name = "/ { name=$2 }
    /^path = "/ { path=$2 }
    /^slug = "/ { slug=$2 }
    /^default_branch = "/ { default_branch=$2 }
    /^required_check = "/ { required_check=$2 }
    /^hosted = "/ { hosted=$2 }
    /^mirror_github = / { split($0, fields, " = "); mirror_github=fields[2] }
    /^tag = "/ { tag=$2 }
    /^jeryu = "/ { jeryu=$2 }
    /^github = "/ { github=$2 }
    END { emit() }
  ' "$manifest"
}

repo_names() {
  manifest_rows | cut -f1
}

manifest_tags() {
  manifest_rows | awk -F '\t' '{ print $1 " " $8 }'
}

manifest_string() {
  local key="$1"
  awk -F '"' -v key="$key" '$0 ~ ("^" key " = ") { print $2; exit }' "$manifest"
}

required_repo_names() {
  awk -F '"' '
    /^required_repos = \[$/ { inside=1; next }
    inside && /^\]$/ { exit }
    inside && /^[[:space:]]+"/ { print $2 }
  ' "$manifest"
}

need_file "$manifest"
need_file "$projection_tool"
[[ ! -L "$manifest" && "$(stat -c '%h' "$manifest")" == "1" ]] \
  || fail "tracked authority manifest must be a regular one-link non-symlink file"
[[ "$(manifest_string schema_version)" == "1.2.0" ]] \
  || fail "unsupported family manifest schema"
[[ "$(manifest_string repo_family)" == "jankurai-split" ]] \
  || fail "unexpected repo_family"
[[ "$(manifest_string split_root)" == "$split_root" ]] \
  || fail "split_root does not bind the canonical family root"
[[ "$(manifest_string authority_manifest)" == "$manifest" ]] \
  || fail "authority_manifest does not bind the tracked hub manifest"
[[ "$(manifest_string runtime_projection)" == "${split_root}/repos.manifest.toml" ]] \
  || fail "runtime_projection does not bind the container-root projection"
[[ "$(manifest_string authority_forge)" == "local_transition" ]] \
  || fail "authority_forge must remain local_transition before the protected cutover"
[[ "$(manifest_string hosted_base_url)" == "https://git.neverhuman.org" ]] \
  || fail "hosted_base_url must be the pinned TLS endpoint"
[[ "$(manifest_string hosted_git_url_template)" == "https://git.neverhuman.org/git/{owner}/{repo}.git" ]] \
  || fail "hosted_git_url_template must be the canonical hosted template"
[[ "$(awk -F ' = ' '/^expected_repo_count = / { print $2; exit }' "$manifest")" == "15" ]] \
  || fail "expected_repo_count must be 15"

rows="$(manifest_rows)"
[[ "$(printf '%s\n' "$rows" | sed '/^$/d' | wc -l)" == "15" ]] \
  || fail "family manifest must contain exactly 15 repository rows"
for field in 1 2 3 5; do
  duplicate="$(printf '%s\n' "$rows" | cut -f"$field" | sort | uniq -d)"
  [[ -z "$duplicate" ]] || fail "duplicate repository identity in manifest: ${duplicate}"
done
required_diff="$(comm -3 \
  <(repo_names | sort) \
  <(required_repo_names | sort))"
[[ -z "$required_diff" ]] || fail "required_repos differs from repository rows: ${required_diff}"

while IFS=$'\t' read -r repo path slug default_branch required_check hosted mirror_github tag jeryu github; do
  [[ -n "$repo" ]] || continue
  [[ "$path" == "${split_root}/${repo}" ]] \
    || fail "${repo}: unexpected canonical path ${path}"
  [[ "$slug" == "root/${repo}" ]] \
    || fail "${repo}: unexpected repository slug ${slug}"
  [[ "$default_branch" == "main" ]] \
    || fail "${repo}: default branch must be main"
  [[ "$required_check" == "${repo}/required" ]] \
    || fail "${repo}: missing or unexpected required check"
  [[ "$hosted" == "https://git.neverhuman.org/git/root/${repo}.git" ]] \
    || fail "${repo}: unexpected hosted URL"
  [[ "$mirror_github" == "false" ]] \
    || fail "${repo}: outbound GitHub mirroring must be disabled"
  [[ "$jeryu" == "ssh://git@127.0.0.1:2224/root/${repo}.git" ]] \
    || fail "${repo}: unexpected local-transition URL"
  [[ "$github" == "https://github.com/neverhuman/${repo}.git" ]] \
    || fail "${repo}: unexpected read-only GitHub history URL"
  [[ -n "$tag" ]] || fail "${repo}: missing immutable tag"
  [[ -d "$path" && ! -L "$path" && "$(realpath -e "$path")" == "$path" ]] \
    || fail "${repo}: canonical checkout path is missing, a symlink, or aliased"
  [[ -d "${path}/.git" && ! -L "${path}/.git" ]] \
    || fail "${repo}: canonical checkout is not a primary non-worktree repository"
  worktree_count="$(git -C "$path" worktree list --porcelain | awk '/^worktree / { count++ } END { print count+0 }')"
  [[ "$worktree_count" == "1" ]] \
    || fail "${repo}: expected exactly one registered checkout"
done <<<"$rows"

bash "$projection_tool" --check
provision_output="$(bash "${hub}/scripts/provision-family.sh" --dry-run 2>&1 || true)"
[[ "$provision_output" == "provision-family: disabled; use the protected manifest-selected forge lifecycle" ]] \
  || fail "legacy outbound provisioner is not fail-closed"
if rg -n 'git[[:space:]]+-C.*push|gh[[:space:]]+repo[[:space:]]+create' "${hub}/scripts/provision-family.sh" >/dev/null; then
  fail "legacy outbound provisioner still contains a push path"
fi
hub_mirror_states="$(awk -F ' = ' '
  /^\[shadow_main\]$/ { section="shadow_main"; next }
  /^\[tag_mirror\]$/ { section="tag_mirror"; next }
  /^enabled = / && section != "" { print section "=" $2; section="" }
' "${hub}/.jeryu/repo.toml")"
[[ "$hub_mirror_states" == $'shadow_main=false\ntag_mirror=false' ]] \
  || fail "hub outbound GitHub mirror routes must be disabled"

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
    repo_path="${split_root}/${repo}"
    lock_commit="$(awk -F '"' -v wanted="$repo" '
      /^\[\[repo\]\]$/ { repo=""; commit="" }
      /^repo = "/ { repo=$2 }
      /^commit = "/ && repo == wanted { print $2; exit }
    ' "$lock")"
    [[ "$lock_commit" =~ ^[0-9a-f]{40}$ ]] \
      || fail "family.lock commit missing or malformed for ${repo}"
    tag_commit="$(git -C "$repo_path" rev-parse --verify "refs/tags/${tag}^{commit}" 2>/dev/null)" \
      || fail "${repo}: immutable tag is missing locally"
    [[ "$tag_commit" == "$lock_commit" ]] \
      || fail "family.lock commit does not match immutable tag for ${repo}"
  done < <(manifest_tags)
fi

printf 'validate-family: ok\n'
