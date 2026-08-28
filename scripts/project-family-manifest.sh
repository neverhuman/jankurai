#!/usr/bin/env bash
set -euo pipefail

hub="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
split_root="$(cd "${hub}/.." && pwd -P)"
authority="${hub}/repos.manifest.toml"
projection="${split_root}/repos.manifest.toml"
bundles_root="${split_root}/.bundles"
custody_root="${bundles_root}/manifest-projection-custody"

fail() {
  printf 'project-family-manifest: %s\n' "$*" >&2
  exit 1
}

usage() {
  printf 'usage: %s {--check|--apply}\n' "$0" >&2
  exit 2
}

require_owned_physical_dir() {
  local path="$1"
  local label="$2"
  [[ -d "$path" && ! -L "$path" && "$(realpath -e "$path")" == "$path" ]] \
    || fail "${label} must be a physical canonical directory"
  [[ "$(stat -c '%u' "$path")" == "$(id -u)" ]] \
    || fail "${label} must be owned by the current user"
}

[[ $# -eq 1 ]] || usage
case "$1" in
  --check|--apply) mode="$1" ;;
  *) usage ;;
esac

[[ "$hub" == "/home/ubuntu/jankurai-split/jankurai" ]] \
  || fail "unexpected authority checkout: ${hub}"
[[ "$split_root" == "/home/ubuntu/jankurai-split" ]] \
  || fail "unexpected split root: ${split_root}"
require_owned_physical_dir "$split_root" "split root"
require_owned_physical_dir "$hub" "authority checkout"
[[ -f "$authority" && ! -L "$authority" ]] \
  || fail "authority manifest must be a regular non-symlink file"
[[ "$(stat -c '%h' "$authority")" == "1" ]] \
  || fail "authority manifest must have exactly one hard link"
[[ "$(stat -c '%u' "$authority")" == "$(id -u)" ]] \
  || fail "authority manifest must be owned by the current user"

authority_sha256="$(sha256sum "$authority" | awk '{print $1}')"
stage="$(mktemp "${split_root}/.repos.manifest.toml.generated.XXXXXX")"
cleanup() {
  [[ -z "$stage" ]] || rm -f -- "$stage"
}
trap cleanup EXIT

{
  printf '# GENERATED FILE — DO NOT EDIT\n'
  printf '# Authority: /home/ubuntu/jankurai-split/jankurai/repos.manifest.toml\n'
  printf '# Authority SHA-256: %s\n\n' "$authority_sha256"
  sed -e '/^# GENERATED FILE — DO NOT EDIT$/d' \
      -e '/^# Authority: \/home\/ubuntu\/jankurai-split\/jankurai\/repos\.manifest\.toml$/d' \
      -e '/^# Authority SHA-256: [0-9a-f]\{64\}$/d' \
      "$authority"
} >"$stage"
chmod 0664 "$stage"
expected_sha256="$(sha256sum "$stage" | awk '{print $1}')"

if [[ "$mode" == "--check" ]]; then
  [[ -f "$projection" && ! -L "$projection" ]] \
    || fail "runtime projection is missing, non-regular, or a symlink"
  [[ "$(stat -c '%h' "$projection")" == "1" ]] \
    || fail "runtime projection must have exactly one hard link"
  [[ "$(stat -c '%u' "$projection")" == "$(id -u)" ]] \
    || fail "runtime projection must be owned by the current user"
  cmp -s "$stage" "$projection" \
    || fail "runtime projection differs from the tracked authority"
  printf 'project-family-manifest: ok authority_sha256=%s projection_sha256=%s\n' \
    "$authority_sha256" "$expected_sha256"
  exit 0
fi

if [[ -e "$projection" || -L "$projection" ]]; then
  [[ -f "$projection" && ! -L "$projection" ]] \
    || fail "refusing to replace a non-regular or symlink projection"
  [[ "$(stat -c '%h' "$projection")" == "1" ]] \
    || fail "refusing to replace a multiply linked projection"
  [[ "$(stat -c '%u' "$projection")" == "$(id -u)" ]] \
    || fail "refusing to replace a projection owned by another user"
  projection_before="$(stat -c '%d:%i:%s:%Y:%Z:%u:%g:%a:%h' "$projection")"

  if ! cmp -s "$stage" "$projection"; then
    old_sha256="$(sha256sum "$projection" | awk '{print $1}')"
    stamp="$(date -u +%Y%m%dt%H%M%Sz)"
    backup="${custody_root}/${stamp}-${old_sha256}.toml"
    require_owned_physical_dir "$bundles_root" "bundle custody root"
    if [[ ! -e "$custody_root" && ! -L "$custody_root" ]]; then
      mkdir -- "$custody_root"
    fi
    require_owned_physical_dir "$custody_root" "manifest projection custody"
    chmod 0700 "$custody_root"
    [[ ! -e "$backup" && ! -L "$backup" ]] \
      || fail "projection custody target already exists"
    install -m 0600 -- "$projection" "$backup"
    [[ "$(sha256sum "$backup" | awk '{print $1}')" == "$old_sha256" ]] \
      || fail "projection custody verification failed"
  fi
  [[ "$(stat -c '%d:%i:%s:%Y:%Z:%u:%g:%a:%h' "$projection")" == "$projection_before" ]] \
    || fail "runtime projection changed during generation"
else
  projection_before="absent"
fi

if [[ "$projection_before" == "absent" ]]; then
  [[ ! -e "$projection" && ! -L "$projection" ]] \
    || fail "runtime projection appeared during generation"
fi

mv -T -- "$stage" "$projection"
stage=""
sync -f "$projection"
[[ "$(sha256sum "$projection" | awk '{print $1}')" == "$expected_sha256" ]] \
  || fail "runtime projection post-write verification failed"

printf 'project-family-manifest: applied authority_sha256=%s projection_sha256=%s\n' \
  "$authority_sha256" "$expected_sha256"
