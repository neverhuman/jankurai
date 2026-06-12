# jankurai Agent Instructions

Read `SPLIT.md` first. This repository is one member of the Jankurai split family.

- Canonical local Jeryu repo: `root/jankurai`.
- Public mirror target: `github.com/neverhuman/jankurai`.
- Do not add committed cross-repo `path = "../..."` dependencies. Use the hub fusion workspace for local path patches.
- Do not hand-edit generated artifacts listed in `agent/generated-zones.toml`.
- Run `bash scripts/ci-local.sh required` before handing off changes.
