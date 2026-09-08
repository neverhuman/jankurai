# Copilot / agent instructions

Thin adapter for IDE coding agents working in the jankurai hub. The canonical
guidance lives in [`AGENTS.md`](../AGENTS.md), [`SPLIT.md`](../SPLIT.md), and
[`docs/`](../docs/); this file only routes you there.

- Read `AGENTS.md` and `SPLIT.md` before changing anything.
- Prefer `agent/owner-map.json` and `agent/test-map.json` to find the owner and
  proof command for a path; route to the narrowest proof lane.
- The narrowest proof loop is `just fast` (runs `bash scripts/validate-family.sh`).
  The full gate is `just check` (fast + security + audit).
- Do not commit cross-repo `path = "../..."` dependencies; local path patches go
  only into the generated `.fusion/` workspace.
- Do not hand-edit generated zones listed in `agent/generated-zones.toml`.
- Every third-party GitHub Action is pinned to a 40-character commit SHA.
