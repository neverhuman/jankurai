#!/usr/bin/env bash
# Release macOS packaging: codesign the CLI binary, assemble a Developer ID
# installer pkg, notarize it, and staple the ticket.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

usage() {
  cat <<'EOF'
usage: release-macos-sign.sh --binary <path> --pkg-out <path> --version <version> --target <target>
EOF
}

decode_base64() {
  if base64 --decode </dev/null >/dev/null 2>&1; then
    base64 --decode
  else
    base64 -D
  fi
}

binary=""
pkg_out=""
version=""
target=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary) binary="${2:-}"; shift 2 ;;
    --pkg-out) pkg_out="${2:-}"; shift 2 ;;
    --version) version="${2:-}"; shift 2 ;;
    --target) target="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown argument: $1" ;;
  esac
done

[[ -n "${binary}" && -n "${pkg_out}" && -n "${version}" && -n "${target}" ]] || {
  usage
  fail "missing required arguments"
}

for required in \
  APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64 \
  APPLE_DEVELOPER_ID_APPLICATION_PASSWORD \
  APPLE_DEVELOPER_ID_APPLICATION_IDENTITY \
  APPLE_DEVELOPER_ID_INSTALLER_P12_BASE64 \
  APPLE_DEVELOPER_ID_INSTALLER_PASSWORD \
  APPLE_DEVELOPER_ID_INSTALLER_IDENTITY \
  APPLE_NOTARYTOOL_APPLE_ID \
  APPLE_NOTARYTOOL_PASSWORD \
  APPLE_NOTARYTOOL_TEAM_ID
do
  [[ -n "${!required:-}" ]] || fail "missing required release signing secret: ${required}"
done

if ! command -v security >/dev/null 2>&1 || ! command -v xcrun >/dev/null 2>&1; then
  fail "macOS signing requires security and xcrun"
fi
if ! command -v pkgbuild >/dev/null 2>&1 || ! command -v productsign >/dev/null 2>&1; then
  fail "macOS signing requires pkgbuild and productsign"
fi

workdir="$(mktemp -d "${ARTIFACT_ROOT}/release-macos.XXXXXX")"
keychain="${workdir}/jankurai-release.keychain-db"
cleanup() {
  security default-keychain -s "${HOME}/Library/Keychains/login.keychain-db" >/dev/null 2>&1 || true
  rm -rf "${workdir}"
}
trap cleanup EXIT

ensure_dir "$(dirname "${pkg_out}")"

step "Create temporary keychain"
security create-keychain -p "${APPLE_KEYCHAIN_PASSWORD:-jankurai-release}" "${keychain}"
security set-keychain-settings -lut 21600 "${keychain}"
security unlock-keychain -p "${APPLE_KEYCHAIN_PASSWORD:-jankurai-release}" "${keychain}"
security list-keychains -d user -s "${keychain}" "${HOME}/Library/Keychains/login.keychain-db"
security default-keychain -s "${keychain}"

step "Import Developer ID certificates"
app_p12="${workdir}/developer-id-application.p12"
installer_p12="${workdir}/developer-id-installer.p12"
printf '%s' "${APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64}" | decode_base64 > "${app_p12}"
printf '%s' "${APPLE_DEVELOPER_ID_INSTALLER_P12_BASE64}" | decode_base64 > "${installer_p12}"
security import "${app_p12}" -k "${keychain}" -P "${APPLE_DEVELOPER_ID_APPLICATION_PASSWORD}" -A -T /usr/bin/codesign
security import "${installer_p12}" -k "${keychain}" -P "${APPLE_DEVELOPER_ID_INSTALLER_PASSWORD}" -A -T /usr/bin/productsign

step "codesign jankurai"
codesign --force --timestamp --options runtime --sign "${APPLE_DEVELOPER_ID_APPLICATION_IDENTITY}" "${binary}"
codesign --verify --deep --strict --verbose=2 "${binary}"

step "Assemble pkg payload"
payload="${workdir}/payload"
ensure_dir "${payload}/usr/local/bin"
install -m 0755 "${binary}" "${payload}/usr/local/bin/jankurai"

unsigned_pkg="${workdir}/jankurai-${version}-${target}-unsigned.pkg"
signed_pkg="${workdir}/jankurai-${version}-${target}.pkg"

step "pkgbuild"
pkgbuild \
  --root "${payload}" \
  --identifier "dev.neverhuman.jankurai" \
  --version "${version}" \
  --install-location "/" \
  "${unsigned_pkg}"

step "productsign"
productsign \
  --sign "${APPLE_DEVELOPER_ID_INSTALLER_IDENTITY}" \
  "${unsigned_pkg}" \
  "${signed_pkg}"

step "notarytool submit"
xcrun notarytool submit "${signed_pkg}" \
  --wait \
  --apple-id "${APPLE_NOTARYTOOL_APPLE_ID}" \
  --password "${APPLE_NOTARYTOOL_PASSWORD}" \
  --team-id "${APPLE_NOTARYTOOL_TEAM_ID}"

step "stapler + verification"
xcrun stapler staple "${signed_pkg}"
pkgutil --check-signature "${signed_pkg}"

cp "${signed_pkg}" "${pkg_out}"
assert_nonempty "${pkg_out}"
