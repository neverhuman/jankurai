#!/usr/bin/env bash
# Provision the jankurai split family to GitHub (neverhuman) + Jeryu, at parity.
#
# Run this from the hub repo root WITH your neverhuman GitHub credentials available
# (gh auth status must be green) and the Jeryu SSH remote reachable.
# It is read-mostly and idempotent: re-running skips repos/tags that already exist.
#
#   bash scripts/provision-family.sh --dry-run     # print every action, do nothing
#   bash scripts/provision-family.sh --visibility public   # create + push for real
#
# Phases:
#   1. Pre-flight   : gh auth, clean worktrees, family.lock pins == HEADs, validate-family
#   2. Create+push  : leaf repos first (kernel, guard, proof) -> core -> the rest,
#                     pushing `main` + the manifest tag to BOTH github and jeryu
#   3. Parity check : git ls-remote github == jeryu for every repo's main + tag
#   4. Release      : push the hub release tag to trigger the signed binary release
set -euo pipefail
hub="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
split_root="$(cd "${hub}/.." && pwd)"
manifest="${hub}/repos.manifest.toml"
lock="${hub}/family.lock"
owner="$(awk -F'"' '/^public_owner/{print $2}' "$manifest")"   # neverhuman
release_tag="v$(cat "${split_root}/jankurai-deploy/VERSION" 2>/dev/null || echo 1.7.0-split.0)"

dry=false; visibility=private
while [[ $# -gt 0 ]]; do case "$1" in
  --dry-run) dry=true; shift ;;
  --visibility) visibility="$2"; shift 2 ;;
  *) echo "usage: $0 [--dry-run] [--visibility public|private]" >&2; exit 2 ;;
esac; done
run() { if $dry; then echo "  + $*"; else "$@"; fi; }

# Dependency-ordered: core's Cargo.toml git-deps must exist before core builds in the release.
ORDER=(jankurai-tools-kernel jankurai-tools-guard jankurai-tools-proof jankurai-core \
       jankurai-contracts jankurai-standard jankurai-conformance jankurai-paper \
       jankurai-tools-tui jankurai-tools-ux jankurai-tools-dedup jankurai-tools-fleet \
       jankurai-tools-analyzers jankurai-deploy jankurai)

field() { awk -F'"' -v n="$1" -v f="$2" '$0=="name = \""n"\""{hit=1} hit&&$1~("^"f" = "){print $2; exit}' "$manifest"; }

echo "== Phase 1: pre-flight =="
$dry || gh auth status >/dev/null || { echo "gh not authenticated"; exit 1; }
for repo in "${ORDER[@]}"; do
  d="${split_root}/${repo}"
  [[ -d "$d/.git" ]] || { echo "missing repo $d"; exit 1; }
  [[ -z "$(git -C "$d" status --porcelain | grep -v '^?? ')" ]] || { echo "uncommitted tracked changes in $repo"; exit 1; }
done
$dry || bash "${hub}/scripts/validate-family.sh"

echo "== Phase 2: create + push (github + jeryu), leaves first =="
for repo in "${ORDER[@]}"; do
  d="${split_root}/${repo}"
  gh_url="$(field "$repo" github)"; jr_url="$(field "$repo" jeryu)"; tag="$(field "$repo" tag)"
  echo "-- ${repo}  tag=${tag}"
  $dry || gh repo view "${owner}/${repo}" >/dev/null 2>&1 || run gh repo create "${owner}/${repo}" --"${visibility}"
  run git -C "$d" remote add github "$gh_url" 2>/dev/null || run git -C "$d" remote set-url github "$gh_url"
  run git -C "$d" remote add jeryu  "$jr_url" 2>/dev/null || run git -C "$d" remote set-url jeryu  "$jr_url"
  for remote in github jeryu; do
    run git -C "$d" push "$remote" main
    run git -C "$d" push "$remote" "refs/tags/${tag}"
  done
done

echo "== Phase 3: parity check (github == jeryu) =="
for repo in "${ORDER[@]}"; do
  d="${split_root}/${repo}"; tag="$(field "$repo" tag)"
  if ! $dry; then
    gm=$(git -C "$d" ls-remote github main | cut -f1); jm=$(git -C "$d" ls-remote jeryu main | cut -f1)
    gt=$(git -C "$d" ls-remote github "refs/tags/${tag}" | cut -f1); jt=$(git -C "$d" ls-remote jeryu "refs/tags/${tag}" | cut -f1)
    [[ "$gm" == "$jm" && "$gt" == "$jt" && -n "$gm" && -n "$gt" ]] \
      && echo "  OK  ${repo}" || { echo "  PARITY MISMATCH ${repo}: github main=$gm jeryu main=$jm tag g=$gt j=$jt"; exit 1; }
  fi
done

echo "== Phase 4: first signed binary release on the hub (${owner}/jankurai) =="
# The hub's release pipeline (release.yml) builds core against the now-published
# tool tags, signs, and publishes. Trigger it by pushing the release tag.
run git -C "${split_root}/jankurai" tag -f "${release_tag}" main
run git -C "${split_root}/jankurai" push github "refs/tags/${release_tag}"
echo "Release ${release_tag} triggered on ${owner}/jankurai. Watch: gh run watch --repo ${owner}/jankurai"
echo "When green, the installer works: curl -fsSL https://github.com/${owner}/jankurai/releases/download/${release_tag}/jankurai-installer.sh | bash"
