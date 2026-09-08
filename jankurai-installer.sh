#!/usr/bin/env bash
# Install verified public tarballs from the immutable release workflow identity.
set -euo pipefail
fail() { printf 'installer: %s\n' "$*" >&2; exit 1; }
repo="${JANKURAI_RELEASE_REPO:-neverhuman/jankurai}"
tag="${JANKURAI_RELEASE_TAG:-v1.7.0}"
install_dir="${JANKURAI_INSTALL_DIR:-$HOME/.local/bin}"
product=jankurai
verify_only=false
print_asset=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) repo="${2:?missing repository}"; shift 2 ;;
    --tag) tag="${2:?missing tag}"; shift 2 ;;
    --product) product="${2:?missing product}"; shift 2 ;;
    --install-dir) install_dir="${2:?missing directory}"; shift 2 ;;
    --verify-only) verify_only=true; shift ;;
    --print-asset-name) print_asset=true; shift ;;
    --help|-h) printf 'usage: jankurai-installer.sh [--tag v1.7.0] [--product jankurai|tuiwright] [--repo owner/repo] [--install-dir path] [--verify-only] [--print-asset-name]\n'; exit 0 ;;
    *) fail "unknown argument: $1" ;;
  esac
done
[[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || fail 'invalid repository'
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ ]] || fail 'invalid version tag'
[[ "$product" == jankurai || "$product" == tuiwright ]] || fail 'unsupported product'
case "$(uname -s)/$(uname -m)" in
  Linux/x86_64) target=x86_64-unknown-linux-gnu ;;
  Darwin/arm64) target=aarch64-apple-darwin ;;
  *) fail 'supported platforms: Linux x86-64 and Apple Silicon macOS' ;;
esac
stem="$product-${tag#v}-$target"
asset="$stem.tar.gz"
if "$print_asset"; then printf '%s\n' "$asset"; exit 0; fi
for tool in curl gh cosign jq; do command -v "$tool" >/dev/null || fail "install $tool before running the installer"; done
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
base="https://github.com/$repo/releases/download/$tag"
for name in "$asset" "$asset.sha256" "$asset.sigstore.bundle"; do
  curl --proto '=https' --tlsv1.2 -fsSL "$base/$name" -o "$work/$name"
done
identity="https://github.com/$repo/.github/workflows/release.yml@refs/tags/$tag"
gh attestation verify "$work/$asset" --repo "$repo" \
  --cert-identity "$identity" --deny-self-hosted-runners
cosign verify-blob "$work/$asset" --bundle "$work/$asset.sigstore.bundle" \
  --certificate-identity "$identity" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
release_commit="$(gh api "repos/$repo/commits/$tag" --jq '.sha')"
sha256() {
  if command -v shasum >/dev/null; then shasum -a 256 "$1" | cut -d ' ' -f 1
  else sha256sum "$1" | cut -d ' ' -f 1
  fi
}
[[ "$(cat "$work/$asset.sha256")" == "$(sha256 "$work/$asset")  $asset" ]] || fail 'checksum mismatch'
tar -tzf "$work/$asset" | sed 's:/$::' | LC_ALL=C sort > "$work/inventory"
printf '%s\n' "$stem" "$stem/$product" "$stem/family.lock" "$stem/Cargo.lock" \
  "$stem/LICENSE" "$stem/provenance.json" | LC_ALL=C sort > "$work/expected"
cmp -s "$work/inventory" "$work/expected" || fail 'unexpected archive inventory'
tar -tvzf "$work/$asset" > "$work/details"
if LC_ALL=C grep -qv '^[-d]' "$work/details"; then fail 'unsafe archive entry'; fi
mkdir "$work/payload"
tar -xzf "$work/$asset" --no-same-owner -C "$work/payload"
payload="$work/payload/$stem"
jq -e --arg commit "$release_commit" --arg target "$target" --arg version "${tag#v}" \
  '.commit == $commit and .target == $target and .version == $version' \
  "$payload/provenance.json" >/dev/null || fail 'release provenance mismatch'
jq -e --arg family "$(sha256 "$payload/family.lock")" --arg cargo "$(sha256 "$payload/Cargo.lock")" \
  '.family_lock_sha256 == $family and .cargo_lock_sha256 == $cargo' \
  "$payload/provenance.json" >/dev/null || fail 'lock provenance mismatch'

if "$verify_only"; then printf 'Verified %s\n' "$asset"; exit 0; fi
mkdir -p "$install_dir"
install -m 0755 "$payload/$product" "$install_dir/$product"
"$install_dir/$product" --version
printf 'Installed %s/%s\n' "$install_dir" "$product"
