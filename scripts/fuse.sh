#!/usr/bin/env bash
set -euo pipefail

hub="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
split_root="$(cd "${hub}/.." && pwd)"
fusion="${hub}/.fusion"
repos_dir="${fusion}/repos"
source_kind="github"
all=false

usage() {
  cat >&2 <<'EOF_USAGE'
usage: scripts/fuse.sh --source {github|jeryu|local} --all

Creates a generated local fusion workspace under .fusion/ using family.lock.
The fusion workspace is ignored by git and is the only place local path patches
are written.
EOF_USAGE
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) source_kind="${2:-}"; shift 2 ;;
    --all) all=true; shift ;;
    -h|--help) usage ;;
    *) usage ;;
  esac
done

[[ "$all" == true ]] || usage
case "$source_kind" in
  github|jeryu|local) ;;
  *) usage ;;
esac

manifest="${hub}/repos.manifest.toml"
lock="${hub}/family.lock"
[[ -f "$manifest" ]] || { echo "missing ${manifest}" >&2; exit 1; }
[[ -f "$lock" ]] || { echo "missing ${lock}" >&2; exit 1; }

mkdir -p "$repos_dir"

lock_rows() {
  awk -F '"' '
    /^\[\[repo\]\]/ { if (repo != "") print repo " " tag " " commit; repo=""; tag=""; commit="" }
    /^repo = "/ { repo=$2 }
    /^tag = "/ { tag=$2 }
    /^commit = "/ { commit=$2 }
    END { if (repo != "") print repo " " tag " " commit }
  ' "$lock"
}

repo_url() {
  local repo="$1"
  case "$source_kind" in
    github) printf 'https://github.com/neverhuman/%s.git\n' "$repo" ;;
    jeryu) printf 'ssh://git@127.0.0.1:2224/root/%s.git\n' "$repo" ;;
    local) printf '%s/%s\n' "$split_root" "$repo" ;;
  esac
}

checkout_repo() {
  local repo="$1"
  local tag="$2"
  local commit="$3"
  local dest="${repos_dir}/${repo}"
  local url
  url="$(repo_url "$repo")"

  if [[ ! -d "$dest/.git" ]]; then
    git clone "$url" "$dest"
  else
    git -C "$dest" remote set-url origin "$url" || true
    git -C "$dest" fetch --force --tags origin
  fi

  if [[ -n "$(git -C "$dest" status --porcelain)" ]]; then
    current="$(git -C "$dest" rev-parse HEAD)"
    if [[ "$current" != "$commit" ]]; then
      echo "refuse dirty checkout ref change for ${repo}: ${current} -> ${commit}" >&2
      exit 1
    fi
  fi

  git -C "$dest" fetch --force --tags origin || true
  if [[ -n "$commit" && "$commit" != "pending" ]]; then
    git -C "$dest" checkout --detach "$commit"
  else
    git -C "$dest" checkout --detach "$tag"
  fi
}

while read -r repo tag commit; do
  [[ -n "${repo:-}" ]] || continue
  checkout_repo "$repo" "$tag" "$commit"
done < <(lock_rows)

cat > "${fusion}/Cargo.toml" <<'EOF_CARGO'
[workspace]
members = [
  "repos/jankurai-core/crates/jankurai",
  "repos/jankurai-tools-guard/crates/jankurai-guard",
  "repos/jankurai-tools-proof/crates/jankurai-proofbind",
  "repos/jankurai-tools-proof/crates/jankurai-proofmark",
  "repos/jankurai-tools-tui/crates/tuiwright",
  "repos/jankurai-tools-tui/crates/tuiwright-cli",
  "repos/jankurai-tools-tui/examples/tuiwright-demo",
]
resolver = "2"

[patch."https://github.com/neverhuman/jankurai-tools-guard.git"]
jankurai-guard = { path = "repos/jankurai-tools-guard/crates/jankurai-guard" }

[patch."https://github.com/neverhuman/jankurai-tools-proof.git"]
jankurai-proofbind = { path = "repos/jankurai-tools-proof/crates/jankurai-proofbind" }
jankurai-proofmark = { path = "repos/jankurai-tools-proof/crates/jankurai-proofmark" }
EOF_CARGO

if [[ -f "${repos_dir}/jankurai-core/Cargo.lock" ]]; then
  cp "${repos_dir}/jankurai-core/Cargo.lock" "${fusion}/Cargo.lock"
fi

cat > "${fusion}/package.json" <<'EOF_PACKAGE'
{
  "name": "jankurai-fusion",
  "private": true,
  "type": "module",
  "workspaces": [
    "repos/jankurai-tools-ux/packages/*"
  ],
  "scripts": {
    "ux-qa:build": "npm --workspace @jankurai/ux-qa run build",
    "ux-qa:test": "npm --workspace @jankurai/ux-qa run test"
  }
}
EOF_PACKAGE

cat > "${fusion}/dev.sh" <<'EOF_DEV'
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

case "${1:-build}" in
  build)
    cargo build -p jankurai --locked
    (cd repos/jankurai-tools-ux && npm ci && npm run build)
    ;;
  test)
    cargo test --workspace --locked
    (cd repos/jankurai-tools-ux && npm ci && npm run build && npm test)
    ;;
  version)
    cargo run -p jankurai -- version
    ;;
  *)
    echo "usage: .fusion/dev.sh {build|test|version}" >&2
    exit 2
    ;;
esac
EOF_DEV
chmod +x "${fusion}/dev.sh"

printf 'fusion workspace ready: %s\n' "$fusion"
