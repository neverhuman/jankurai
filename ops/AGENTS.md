# ops Agent Instructions

This cell owns the hub's CI and operational surface. Read the root
[`AGENTS.md`](../AGENTS.md) and [`SPLIT.md`](../SPLIT.md) first.

## Owns

- `ops/ci/*.sh` — thin per-lane CI scripts (`fast`, `security`, `audit`,
  `tool-adoption`, `required`, `quality-gates`) sourced by both
  `scripts/ci-local.sh` and `.github/workflows/ci.yml`.
- `ops/ci/lib.sh` — shared tool-version pins and artifact assertions; the single
  source of truth so local runs and CI execute the same commands.
- `ops/git-hooks/pre-push` — the mandatory pre-push gate; wire it with
  `git config core.hooksPath ops/git-hooks`.

## Forbidden

- Do not put lane logic in `.github/workflows/ci.yml`; jobs only call
  `bash ops/ci/<lane>.sh` so CI and local stay identical.
- Do not unpin a third-party GitHub Action. Every `uses:` is pinned to a
  40-character commit SHA.
- Do not hand-edit generated zones listed in
  [`agent/generated-zones.toml`](../agent/generated-zones.toml).

## Proof lane

Run the security lane and family validation before handing off ops changes:

```bash
bash ops/ci/security.sh
bash scripts/validate-family.sh
```

The narrowest gate is `just fast`; the full gate is `just check`.
