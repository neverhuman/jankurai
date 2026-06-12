#!/usr/bin/env bash
# Release publish: extract CHANGELOG slice, stage release metadata, and run
# `gh release create` on immutable tag assets.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [[ -z "${RELEASE_TAG:-}" ]]; then
  fail "RELEASE_TAG must be set, e.g. RELEASE_TAG=v1.6.0"
fi
if [[ -z "${GH_TOKEN:-}" ]]; then
  fail "GH_TOKEN must be exported for gh release create"
fi

version="${RELEASE_TAG#v}"
expected_version="$(read_version)"
dist="${CI_ROOT}/dist"
notes_file="${dist}/RELEASE_NOTES.md"
installer_src="${CI_ROOT}/jankurai-installer.sh"
installer_dst="${dist}/jankurai-installer.sh"
formula_src="${CI_ROOT}/ops/homebrew/jankurai.rb"
formula_dst="${dist}/jankurai-homebrew.rb"

if [[ "${version}" != "${expected_version}" ]]; then
  fail "RELEASE_TAG (${RELEASE_TAG} -> ${version}) does not match VERSION (${expected_version})"
fi

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

step "Stage installer and Homebrew formula metadata"
cp "${installer_src}" "${installer_dst}"
sed "s/__RELEASE_TAG__/${RELEASE_TAG}/g" "${formula_src}" > "${formula_dst}"

step "sha256 for installer metadata"
(
  cd "${dist}" && shasum -a 256 "jankurai-installer.sh" > "jankurai-installer.sh.sha256"
)
(
  cd "${dist}" && shasum -a 256 "jankurai-homebrew.rb" > "jankurai-homebrew.rb.sha256"
)

step "Gather release assets"
shopt -s nullglob
assets=()
required_assets=(
  "${dist}/audit/repo-score.json"
  "${dist}/audit/repo-score.md"
  "${installer_dst}"
  "${installer_dst}.sha256"
  "${formula_dst}"
  "${formula_dst}.sha256"
)
for f in "${required_assets[@]}"; do
  assert_nonempty "$f"
  assets+=("$f")
done
for f in \
  "${dist}"/*.tar.gz \
  "${dist}"/*.tar.gz.sha256 \
  "${dist}"/*.tar.gz.sigstore.bundle \
  "${dist}"/*.pkg \
  "${dist}"/*.pkg.sha256 \
  "${dist}"/*.pkg.sigstore.bundle
do
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

step "gh release verify ${RELEASE_TAG}"
gh release verify "${RELEASE_TAG}" -R "${GITHUB_REPOSITORY:-neverhuman/jankurai}"
