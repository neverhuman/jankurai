<!-- jankurai-badge:start -->
[![Jankurai score: 97/100](agent/jankurai-badge.svg)](agent/baselines/main.repo-score.json)
<!-- jankurai-badge:end -->

<p align="center">
  <img src="assets/jankurai_github_header_transparent.png" alt="Jankurai: agent-native repository control plane" width="100%">
</p>

# Jankurai

[![jankurai CI](https://github.com/neverhuman/jankurai/actions/workflows/jankurai.yml/badge.svg)](https://github.com/neverhuman/jankurai/actions/workflows/jankurai.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Jankurai is an anti-vibe coding standard and local audit CLI for auditable AI-assisted merge. Its public loop is simple: find vibe artifacts, prove the merge, repair the repo.

- Turns ownership maps, proof lanes, generated zones, security boundaries, rolling scores, merge witnesses, and repair queues into files agents and humans can both read.
- Checks 40 stable HLT rule families and maps 260 vibe-coding failure rows into auditable controls, including release readiness, web security hazards, repo rot, bad CI/Git/tooling behavior, secret sprawl, generated drift, false-green tests, UX proof gaps, and missing evidence.
- Starts with read-only reports, then lets teams adopt guidance, CI, hooks, and ratchets only when they choose.
- Leaves receipts: JSON/Markdown reports, score history, proof artifacts, and command evidence under predictable paths.

Jankurai is not a model, hosted AI service, or "open source AI" system. It is repository infrastructure for making merge decisions reproducible. In Jankurai, "proof" means repository-local evidence receipts, not formal proof of full program semantics.

## Install

Prerequisites: `git` and a Rust toolchain with `cargo` on `PATH`.

```bash
git clone https://github.com/neverhuman/jankurai.git
cd jankurai
cargo install --path crates/jankurai --locked
jankurai version
jankurai versions
```

For demos or CI logs, force rich terminal output:

```bash
export JANKURAI_COLOR=always
export JANKURAI_PROGRESS=always
```

## Try Safely In 5 Minutes

Run the first pass from the repository you want to inspect. These commands read source files and write only the report paths you name under `target/jankurai/`.

```bash
mkdir -p target/jankurai

jankurai adopt . \
  --profile auto \
  --mode observe \
  --out target/jankurai/adoption-plan.json \
  --md target/jankurai/adoption-plan.md

jankurai audit . \
  --mode advisory \
  --json target/jankurai/repo-score.json \
  --md target/jankurai/repo-score.md
```

Expected artifacts:

| Artifact | Purpose |
| --- | --- |
| `target/jankurai/adoption-plan.md` | Recommended next steps and adoption level. |
| `target/jankurai/repo-score.md` | Advisory score, findings, hard caps, and repair queue. |
| `target/jankurai/score-history.jsonl` | One score-history row per audit run. |

## Adopt Levels

| Level | Command | Writes | Use When |
| --- | --- | --- | --- |
| Observe | `jankurai adopt . --mode observe` | Named report files only. | You want an inventory before Jankurai changes tracked files. |
| Agents | `jankurai init . --level agents --dry-run` | Agent guidance after you apply with `--yes`. | You want Codex, Claude, Cursor, Copilot, or another agent to follow the same local rules. |
| Full | `jankurai init . --level full --dry-run` | Full scaffold after review and `--yes`. | You want owner maps, proof lanes, generated-zone policy, docs, contracts/db placeholders, CI, and hooks. |
| Ratchet | `jankurai ci install . --github --mode ratchet --baseline <file>` | CI gate. | The team has accepted a baseline and wants to block regression. |

Ratchet mode is impossible without an accepted baseline. Start in observe or advisory mode, generate reports under `target/jankurai/`, then copy a reviewed clean report to `agent/baselines/main.repo-score.json` in a dedicated baseline update. Ignored `agent/repo-score.*` files are local generated outputs, not trusted ratchet inputs.

## Fresh Agent Kickoff

When a new task arrives, start with the no-write intake command:

```bash
jankurai kickoff . \
  --intent "<change request>" \
  --out target/jankurai/kickoff.json \
  --md target/jankurai/kickoff.md
```

`kickoff` is the first-hour handoff. It turns intent into a bounded plan with read-first files, ownership boundaries, proof lanes, generated-zone and forbidden-path constraints, clarifying questions, and expected receipts before any mutable command runs. If the task is still too broad, keep the response planning-safe and refine the intent before moving to `context-pack`.

## Daily Loop

```bash
jankurai kickoff . --intent "<change request>" --out target/jankurai/kickoff.json --md target/jankurai/kickoff.md
jankurai context-pack . --changed <path> --max-tokens 6000 --out target/jankurai/context-pack.json --md target/jankurai/context-pack.md
jankurai prove . --changed <path> --plan-out target/jankurai/proof-plan.json --plan-md target/jankurai/proof-plan.md
jankurai audit . --changed-fast --changed-from origin/main --json target/jankurai/audit-fast.json --md target/jankurai/audit-fast.md --timings-json target/jankurai/audit-timings.json
jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
jankurai witness . --changed-from origin/main --baseline agent/baselines/main.repo-score.json --out target/jankurai/merge-witness.json --md target/jankurai/merge-witness.md
```

`--changed-fast` is an advisory inner-loop scan. It inventories changed files plus required control files, skips score-history writes, and must be followed by the full audit before merge or release.

Preview before tracked writes:

```bash
jankurai init . \
  --profile rust-ts-postgres \
  --level agents \
  --dry-run \
  --plan-json target/jankurai/init-agents.json
```

Apply only after reviewing the plan:

```bash
jankurai init . --profile rust-ts-postgres --level agents --yes
jankurai adapters verify .
```

Then open your coding agent from the same repo root and ask it to:

```text
Read AGENTS.md, follow the jankurai standard, then run the proof lane for my change.
```

## Upgrade Jankurai

Audits check for available Jankurai upgrades automatically. The check is advisory only: audit never auto-applies an upgrade, never changes score reports with live version data, and silently continues when the network is unavailable.

When audit reports an available upgrade, run:

```bash
jankurai upgrade
```

`jankurai upgrade` is the write-capable refresh path. Use `jankurai upgrade
--score` to run the post-upgrade scoring lane after the install refresh. `jankurai
score` is the main scoring command; with no subcommand it runs the audit lane,
and `diff` and `trend` remain available.

When run from a Jankurai source checkout, `jankurai upgrade` automatically
prefers the local `crates/jankurai` package if it is newer than the installed
binary. This covers the common case where `jankurai version` still reports the
older installed client after pulling or updating the repository.

For advanced review-only checks, preview what would change:

```bash
jankurai update . \
  --check \
  --out target/jankurai/update/update-plan.json \
  --md target/jankurai/update/update-plan.md
```

Set `JANKURAI_NO_UPDATE_CHECK=1` to disable audit-time upgrade checks. Use `jankurai update . --offline` in environments where explicit update checks must avoid network-backed version lookups.

## How Jankurai Handles AI-Agent Risk

Jankurai treats agent behavior as repository policy, not chat convention.

| Risk | Jankurai Control |
| --- | --- |
| Broad or surprising writes | `agent/owner-map.json`, generated-zone manifests, dry-run plans, and explicit apply flags. |
| Weak proof | `agent/test-map.json`, proof lanes, `jankurai lane`, `jankurai prove`, and receipt paths under `target/jankurai/`. |
| Prompt injection | Root `AGENTS.md`, thin provider adapters, and rules that keep untrusted context from changing trusted policy or tool permissions. |
| Generated drift | `agent/generated-zones.toml` identifies generated/read-only outputs and the source command that owns them. |
| Security regressions | Security policy artifacts, `jankurai security run`, dependency/secret checks, and private reporting guidance. |
| Lost context | JSON/Markdown reports, score history, repair queues, and final handoff receipts. |

The project does not send repository contents to a hosted Jankurai service. The CLI inspects local files and writes local artifacts. Any external tools you run through your coding agent remain governed by that agent and your environment.

## GitHub Action

Run Jankurai in GitHub Actions with the Marketplace action tag:

```yaml
name: Jankurai Audit

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: neverhuman/jankurai@v1.1.0
        with:
          mode: advisory
      - uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
        with:
          name: jankurai-audit
          path: |
            target/jankurai/repo-score.json
            target/jankurai/repo-score.md
            target/jankurai/jankurai.sarif
            target/jankurai/summary.md
            target/jankurai/repair-queue.jsonl
```

Inputs:

| Input | Default | Values | Purpose |
| --- | --- | --- | --- |
| `mode` | `advisory` | `observe`, `advisory`, `ratchet` | Selects audit strictness. |
| `baseline` | `agent/baselines/main.repo-score.json` | Any repository-relative JSON path | Accepted baseline score file used by `ratchet` mode. |

Audit path exclusions live in `agent/audit-policy.toml`. New scaffolds exclude `tips/` by default; add repository-relative folder prefixes to keep local planning notes, scratch directories, or generated side inputs out of the audit inventory:

```toml
[scan]
excluded_paths = ["tips/", "scratch/"]
```

The action emits `target/jankurai/repo-score.json`, `target/jankurai/repo-score.md`,
`target/jankurai/jankurai.sarif`, `target/jankurai/summary.md`, and
`target/jankurai/repair-queue.jsonl`. No secrets are required. The CLI installs
from the action checkout and runs locally on the GitHub-hosted runner.

## Public Repository Advisory Scores

The May 6, 2026 paper scan is advisory posture evidence, not certification or defect attribution. Source data is tracked at [`paper/data/public-repo-scores-20260506T014156Z.json`](paper/data/public-repo-scores-20260506T014156Z.json).

| Rank | Repository | Score | Issues |
| ---: | --- | ---: | ---: |
| 1 | [neverhuman/jankurai](https://github.com/neverhuman/jankurai) | 100 | 0 |
| 2 | [zed-industries/zed](https://github.com/zed-industries/zed) | 47 | 3,328 |
| 3 | [astral-sh/ruff](https://github.com/astral-sh/ruff) | 44 | 2,842 |
| 4 | [denoland/deno](https://github.com/denoland/deno) | 42 | 3,130 |
| 5 | [RightNow-AI/openfang](https://github.com/RightNow-AI/openfang) | 42 | 695 |
| 6 | [nearai/ironclaw](https://github.com/nearai/ironclaw) | 42 | 1,968 |
| 7 | [meilisearch/meilisearch](https://github.com/meilisearch/meilisearch) | 41 | 793 |
| 8 | [googleworkspace/cli](https://github.com/googleworkspace/cli) | 40 | 114 |
| 9 | [zeroclaw-labs/zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) | 40 | 1,317 |
| 10 | [kevinpiao1025/tauri-react-typescript-tailwind](https://github.com/kevinpiao1025/tauri-react-typescript-tailwind) | 39 | 21 |
| 11 | [kvnxiao/tauri-tanstack-start-react-template](https://github.com/kvnxiao/tauri-tanstack-start-react-template) | 39 | 24 |
| 12 | [astral-sh/uv](https://github.com/astral-sh/uv) | 38 | 1,258 |
| 13 | [tauri-apps/tauri](https://github.com/tauri-apps/tauri) | 38 | 485 |
| 14 | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | 38 | 226 |
| 15 | [AlexsJones/llmfit](https://github.com/AlexsJones/llmfit) | 35 | 110 |
| 16 | [alacritty/alacritty](https://github.com/alacritty/alacritty) | 33 | 231 |
| 17 | [exit-zero-labs/threat-forge](https://github.com/exit-zero-labs/threat-forge) | 32 | 142 |
| 18 | [typst/typst](https://github.com/typst/typst) | 32 | 398 |
| 19 | [microsoft/RustTraining](https://github.com/microsoft/RustTraining) | 31 | 34 |
| 20 | [BurntSushi/ripgrep](https://github.com/BurntSushi/ripgrep) | 30 | 80 |
| 21 | [octasoft-ltd/wsl-ui](https://github.com/octasoft-ltd/wsl-ui) | 30 | 321 |
| 22 | [rustdesk/rustdesk](https://github.com/rustdesk/rustdesk) | 29 | 1,345 |
| 23 | [Duri686/RustQuantLab](https://github.com/Duri686/RustQuantLab) | 26 | 45 |
| 24 | [MarkShawn2020/lovtauri](https://github.com/MarkShawn2020/lovtauri) | 26 | 30 |
| 25 | [h4ckf0r0day/obscura](https://github.com/h4ckf0r0day/obscura) | 25 | 63 |
| 26 | [rtk-ai/rtk](https://github.com/rtk-ai/rtk) | 25 | 451 |
| 27 | [xai-org/x-algorithm](https://github.com/xai-org/x-algorithm) | 24 | 38 |
| 28 | [fudanglp/tauri-fastapi-full-stack-template](https://github.com/fudanglp/tauri-fastapi-full-stack-template) | 23 | 87 |
| 29 | [lostf1sh/rustune](https://github.com/lostf1sh/rustune) | 23 | 38 |
| 30 | [sergioadevita/notemac-plus-plus](https://github.com/sergioadevita/notemac-plus-plus) | 23 | 344 |
| 31 | [ianho7/maptoposter-online](https://github.com/ianho7/maptoposter-online) | 14 | 66 |

## Control-Plane Surfaces

Jankurai works as a local control plane over a few repeatable surfaces:

| Surface | Commands |
| --- | --- |
| Adoption and drift | `adopt`, `init`, `update`, `doctor` |
| Intent intake | `kickoff` (no-write handoff, read-first files, ownership boundaries, proof lanes, stop conditions, and next commands) |
| Bounded agent context | `context-pack`, `adapters verify`, `adapters sync`, `agent verify`, `hooks install` |
| Proof and evidence | `lane`, `proof`, `prove`, `proof-verify` |
| Audit and routing | `audit`, `witness`, `score diff`, `score trend`, `rules verify`, `issues export`, score history, repair queues |
| Security and UX evidence | `security run`, `ux ...` |
| Repair and expiry | `repair-plan`, `repair`, `optimize`, `waivers expire` |
| Reusable/public evidence | `registry`, `cell`, `bench`, `certify`, `govern`, `publish` |

The loop is intentionally ordinary: changed paths map to owners and proof lanes, commands leave receipts, audit turns evidence into findings, and repair plans keep follow-up bounded.

## Toolkit

Jankurai ships as a Rust workspace of focused crates. Install the core CLI with `cargo install --path crates/jankurai --locked`; companion crates are available as library dependencies or standalone binaries.

### Core Crates

| Crate | Purpose |
| --- | --- |
| [`jankurai`](crates/jankurai) | Audit CLI and standard enforcement engine. Scores repositories, generates findings, routes proof obligations, and writes JSON/Markdown evidence. |
| [`jankurai-proofbind`](crates/jankurai-proofbind) | Semantic surface routing and proof obligation binding. Maps changed paths to owners, proof lanes, and generated-zone policies. |
| [`jankurai-proofmark`](crates/jankurai-proofmark) | Changed-behavior proof receipt engine. Validates that proof plans produce runnable commands and writes audit-ready receipts. |

### Companion Tools

#### Tuiwright — Playwright-Style TUI Testing

[Tuiwright](docs/tuiwright.md) is a Rust-native, black-box testing framework for terminal user interfaces. It spawns real TUI applications in a real pseudo-terminal, drives keyboard/mouse/paste/resize input, maintains an accurate virtual terminal model, and provides Playwright-grade ergonomics.

| Crate | Purpose |
| --- | --- |
| [`tuiwright`](crates/tuiwright) | Core library: PTY driver, vt100 screen model, locators, auto-waiting assertions, PNG screenshot renderer, GIF recorder, JSONL trace writer. |
| [`tuiwright-cli`](crates/tuiwright-cli) | CLI binary for headless `tuiwright screenshot` and `tuiwright record` commands. |
| [`tuiwright-demo`](examples/tuiwright-demo) | Minimal crossterm counter app used as the integration test target. |

```rust
use tuiwright::{Key, Page, SpawnConfig};
use std::time::Duration;

let page = Page::spawn(SpawnConfig::new("my-tui").size(80, 24))?;
page.wait_for_text("Ready", Duration::from_secs(5))?;
page.press(Key::Enter)?;
page.screenshot("target/tuiwright/home.png")?;
```

Run the Tuiwright test suite:

```bash
just tuiwright-test
```

#### Bad-Behavior Reference Docs

The `docs/` directory includes anti-pattern catalogs covering common vibe-coding failure modes. These are curated from real agent sessions and referenced by Jankurai's audit rules:

| Doc | Scope |
| --- | --- |
| [BAD_RUST.md](docs/BAD_RUST.md) | Rust anti-patterns: unsafe misuse, error swallowing, mega-functions, trait misuse |
| [BAD_SQL.md](docs/BAD_SQL.md) | SQL anti-patterns: destructive migrations, missing rollbacks, lock contention |
| [BAD_PYTHON.md](docs/BAD_PYTHON.md) | Python anti-patterns: scope creep, product truth leaks, missing typed contracts |
| [BAD_CI.md](docs/BAD_CI.md) | CI anti-patterns: flaky tests, no gates, artifact gaps |
| [BAD_GIT.md](docs/BAD_GIT.md) | Git anti-patterns: force push, broad commits, missing context |
| [BAD_DOCKER.md](docs/BAD_DOCKER.md) | Docker anti-patterns: root execution, unbounded layers, missing health checks |
| [BAD_TYPE.md](docs/BAD_TYPE.md) | Type system anti-patterns: handwritten DTOs, missing generated clients |
| [BAD_release.md](docs/BAD_release.md) | Release anti-patterns: mutable tags/assets, skipped proof, missing provenance, no rollback |

### Registered Tools

Jankurai's tool adoption catalog ([`agent/tool-adoption.toml`](agent/tool-adoption.toml)) tracks which tools are active and their enforcement mode. Each tool produces evidence that feeds the audit loop:

| Tool ID | Mode | Purpose |
| --- | --- | --- |
| `audit-ci` | auto | CI audit integration and score gating |
| `proof-routing` | auto | Changed-path proof obligation routing |
| `proofbind` | advisory | Semantic surface binding validation |
| `proofmark-rust` | advisory | Rust-specific proof receipt engine |
| `security` | auto | Dependency, secret, and provenance scanning |
| `ux-qa` | auto | Playwright UX evidence and accessibility |
| `db-migration-analyze` | auto | Migration safety analysis |
| `contract-drift` | auto | Generated contract drift detection |
| `rust-witness` | auto | Rust build witness graph |
| `vibe-coverage` | auto | Vibe-coding coverage analysis |
| `tui-testing` | advisory | TUI black-box testing via Tuiwright |
| `release-bad-behavior` | advisory | Release tag, artifact, provenance, and rollback bad-behavior checks |
| `web-security-bad-behavior` | auto | Vite exposure, client secret, browser token storage, and credentialed wildcard CORS checks |
| `repo-rot-bad-behavior` | advisory | Active-source old, backup, copy, archive, and hard-disabled-code checks |

## Test Surface

Jankurai's correctness is gated by a wide suite of Rust unit and integration tests plus
Playwright UX checks. Counts below are regenerated from source by
`scripts/render-test-surface.sh` (also exposed as `just test-surface`) and verified in
CI — the build fails if the chart drifts from reality.

<!-- TEST_SURFACE_START -->
_Generated by `scripts/render-test-surface.sh` — do not edit by hand._

- **Total `#[test]` functions:** 581 across the Rust workspace
- **Integration test files:** 65
- **Playwright tests (`@jankurai/ux-qa`):** 20

```
rust           █                        3
python         ██                       9
typescript     █                        3
docker         █                        3
sql            ████                     18
comments       ███████                  29
security       █████                    20
boundaries     ████████                 30
ci             █                        4
git            ██                       8
release        █                        5
migration      ██████████████           56
ux-qa          ██████████               38
proof          █████                    20
audit          ████████████████████████ 90
conformance    ████                     18
vibe           ███                      12
phases         ███████████████████      73
```
<!-- TEST_SURFACE_END -->

## Project Status

Jankurai is early but usable as a local Rust CLI and standard workspace. The current source tree includes audit, init, update, proof, repair planning, migration analysis, security evidence, UX QA, TUI testing, publication evidence, and the paper source for *Jankurai: Merge Witnesses for Evidence-Carrying AI-Assisted Pull Requests*.

Paper framing:

- The standard is stack-neutral.
- The CLI is a reference implementation.
- The Rust/TypeScript/React/Vite/PostgreSQL/exception-only-Python profile is non-normative.
- This workspace is Rust-first: agents must not add Python for repo tools, proof lanes, product truth, product services, authorization, direct PostgreSQL writes, or general backend glue. Python is allowed only for rare advanced ML/data library work with a dated exception under `python/ai-service`.
- Full audit remains the merge and release gate.
- Score is posture; the merge witness is the decision; conformance is pass/fail.

Compatibility posture:

- Public report schemas should remain compatible or receive explicit migration notes.
- Ratchet enforcement should be opt-in and baseline-backed.
- New agent-facing guidance should be deterministic, local, and reviewable.

Known open-source gaps:

- deeper conformance runner with observed per-fixture witness decisions
- accessible HTML or tagged PDF edition
- durable JEP/RFC governance docs and independent implementation path
- public evidence registry, badge policy, and release checklist

## Docs

- [Install guide](docs/install.md)
- [Adoption guide](docs/adoption.md)
- [Branch protection policy](docs/branch-protection.md)
- [Running CI locally](docs/ci-local.md)
- [Agent-native standard](docs/agent-native-standard.md)
- [Architecture](docs/architecture.md)
- [Testing and proof lanes](docs/testing.md)
- [Tuiwright TUI testing](docs/tuiwright.md)
- [Merge witness](docs/merge-witness.md)
- [Rolling score](docs/rolling-score.md)
- [Security tool matrix](docs/security-tool-matrix.md)
- [Audit rubric](docs/audit-rubric.md)
- [Language bad-behavior catalogs](docs/language-bad-behavior.md)
- [Release bad-behavior catalog](docs/BAD_release.md)
- [Migration engine](docs/migration-engine.md)
- [Mission](docs/mission.md)

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. The short version:

```bash
cargo fmt --all
cargo test -p jankurai
just fast
just score
git diff --check
```

Keep `reference/` read-only, do not hand-edit generated artifacts, and route changed paths through `agent/owner-map.json` and `agent/test-map.json`.

## Security

Do not open public issues for suspected vulnerabilities. Use GitHub private vulnerability reporting for this repository:

https://github.com/neverhuman/jankurai/security/advisories/new

See [SECURITY.md](SECURITY.md) for supported versions, evidence lanes, and advisory handling.

## Support

Use [GitHub issues](https://github.com/neverhuman/jankurai/issues) for reproducible bugs, documentation gaps, and feature proposals. See [SUPPORT.md](SUPPORT.md) for what to include.

## License

Jankurai is licensed under the [MIT License](LICENSE).

## Citation And Paper

This repository is the working source for the paper *Jankurai: Merge Witnesses for Evidence-Carrying AI-Assisted Pull Requests*.

Current release: standard `0.8.0`, auditor/action `1.1.0`, schema `1.7.0`, paper edition `2026.05-ed8`.

Public thesis line: *Find the vibe. Prove the merge. Repair the repo.*

- Paper PDF: [paper/jankurai.pdf](paper/jankurai.pdf)
- Paper source: [paper/jankurai.tex](paper/jankurai.tex)
- Agent-readable companion: [paper/jankurai.md](paper/jankurai.md)
- Mission: [docs/mission.md](docs/mission.md)
