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
for tool in curl gh cosign python3; do command -v "$tool" >/dev/null || fail "install $tool before running the installer"; done
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
python3 - "$work" "$asset" "$stem" "$product" "$release_commit" "$target" "${tag#v}" <<'PY_VERIFY'
import hashlib, json, pathlib, sys, tarfile
work, asset, stem, product, commit, target, version = sys.argv[1:]
work = pathlib.Path(work)
archive = work / asset
expected = hashlib.sha256(archive.read_bytes()).hexdigest() + '  ' + asset + '\n'
if (work / (asset + '.sha256')).read_text() != expected:
    sys.exit('installer: checksum mismatch')
with tarfile.open(archive) as tar:
    names = (product, 'family.lock', 'Cargo.lock', 'LICENSE', 'provenance.json')
    allowed = {stem, *(stem + '/' + n for n in names)}
    members = tar.getmembers()
    if len(members) != len(allowed) or {m.name for m in members} != allowed:
        sys.exit('installer: unexpected archive inventory')
    for member in members:
        if member.name == stem and member.isdir():
            continue
        if not member.isfile() or member.size > 256 * 1024 * 1024:
            sys.exit('installer: unsafe archive entry')
    payload = {name: tar.extractfile(stem + '/' + name).read() for name in names}
    provenance = json.loads(payload['provenance.json'])
    if (provenance['commit'], provenance['target'], provenance['version']) != (commit, target, version):
        sys.exit('installer: release provenance mismatch')
    for name, key in [('family.lock', 'family_lock_sha256'), ('Cargo.lock', 'cargo_lock_sha256')]:
        if hashlib.sha256(payload[name]).hexdigest() != provenance[key]:
            sys.exit('installer: lock provenance mismatch')
    (work / product).write_bytes(payload[product])
PY_VERIFY
if "$verify_only"; then printf 'Verified %s\n' "$asset"; exit 0; fi
mkdir -p "$install_dir"
install -m 0755 "$work/$product" "$install_dir/$product"
"$install_dir/$product" --version
printf 'Installed %s/%s\n' "$install_dir" "$product"
