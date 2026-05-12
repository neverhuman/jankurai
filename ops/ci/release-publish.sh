#!/usr/bin/env bash
# Release publish: extract CHANGELOG slice and run `gh release create`.
# Requires GH_TOKEN in env (the workflow injects ${{ github.token }}).
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [[ -z "${RELEASE_TAG:-}" ]]; then
  fail "RELEASE_TAG must be set, e.g. RELEASE_TAG=v1.1.0"
fi
if [[ -z "${GH_TOKEN:-}" ]]; then
  fail "GH_TOKEN must be exported for gh release create"
fi

version="${RELEASE_TAG#v}"
dist="${CI_ROOT}/dist"
notes_file="${dist}/RELEASE_NOTES.md"
ensure_dir "${dist}"

step "Extract release notes from CHANGELOG"
awk -v ver="$version" '
  BEGIN { capture = 0 }
  /^## / {
    if (capture) exit
    if (index($0, ver) > 0) { capture = 1; next }
  }
  capture { print }
' "${CI_ROOT}/CHANGELOG.md" > "${notes_file}"
if [[ ! -s "${notes_file}" ]]; then
  printf "Release %s\n\nSee CHANGELOG.md for details.\n" "$version" > "${notes_file}"
fi
assert_nonempty "${notes_file}"

step "Gather release assets"
shopt -s nullglob
assets=()
for f in "${dist}"/*.tar.gz "${dist}"/*.tar.gz.sha256 "${dist}/audit"/repo-score.json "${dist}/audit"/repo-score.md; do
  [[ -e "$f" ]] && assets+=("$f")
done
if [[ ${#assets[@]} -eq 0 ]]; then
  fail "no release assets found under ${dist}/"
fi

prerelease=""
if [[ "${RELEASE_TAG}" == *-* ]]; then
  prerelease="--prerelease"
fi

step "gh release create ${RELEASE_TAG}"
gh release create "${RELEASE_TAG}" \
  --title "${RELEASE_TAG}" \
  --notes-file "${notes_file}" \
  --verify-tag \
  ${prerelease} \
  "${assets[@]}"
