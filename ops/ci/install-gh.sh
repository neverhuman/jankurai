#!/usr/bin/env bash
# Pin the same GitHub verifier used by the standalone release installer.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
case "$(uname -s)/$(uname -m)" in
  Linux/x86_64)
    archive=gh_2.100.0_linux_amd64.tar.gz
    digest=e4d4bb4498e8d007abe545b6568926793ace1b6447da598294a610018cb164be
    ;;
  Darwin/arm64)
    archive=gh_2.100.0_macOS_arm64.zip
    digest=45f9a62da2f6e641a7fad57e2ce39656dfd7ef331372d80a2a2aed65abb01642
    ;;
  *) echo 'unsupported GitHub verifier platform' >&2; exit 1 ;;
esac
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
curl --proto '=https' --tlsv1.2 -fsSL "https://github.com/cli/cli/releases/download/v2.100.0/$archive" -o "$stage/$archive"
(cd "$stage" && printf '%s  %s\n' "$digest" "$archive" | shasum -a 256 -c -)
case "$archive" in
  *.tar.gz) tar -xOzf "$stage/$archive" gh_2.100.0_linux_amd64/bin/gh > "$stage/gh" ;;
  *.zip) unzip -p "$stage/$archive" gh_2.100.0_macOS_arm64/bin/gh > "$stage/gh" ;;
esac
chmod 0755 "$stage/gh"
"$stage/gh" --version
destination="${RUNNER_TEMP:-$PWD/target}/jankurai-ci-tools/bin"
mkdir -p "$destination"
install -m 0755 "$stage/gh" "$destination/gh"
if [[ -n "${GITHUB_PATH:-}" ]]; then printf '%s\n' "$destination" >> "$GITHUB_PATH"; fi
