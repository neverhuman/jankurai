# ARYA Fork Study

Date: 2026-05-11

## Objective

Review the ARYA-Labs-PBC/Jankurai fork for ideas that improve migration evidence, false-positive discipline, and repair feedback without importing the Python implementation wholesale.

## Accepted Ideas

- Keep migration verification advisory and read-only by default.
- Treat ambiguity as `review` rather than forcing a pass/fail answer.
- Harden prompt verification against refutation text, code fences, and extension-like references.
- Add slice-risk signals for checkpoint paths, GPU/CUDA, multiprocessing, thread-count env vars, and prior-failure hooks.
- Add a durable postmortem surface with read-only inspect commands and explicit record mode.
- Add severity-discipline checks that require nearby justification trailers instead of flagging every use of strong language.

## Deferred Ideas

- Do not import the fork's Python tooling.
- Do not add `jankurai ai audit` in this pass.
- Do not add external cloud probes, tag mutation, or secret-manager checks.
- Do not let detectors fall back to whole-repo scans when the selected slice is unavailable.

## Policy

- Default to no-write, advisory behavior unless a command explicitly records an artifact.
- Prefer canonical repo-local paths and invalidate symlink escapes.
- Ignore comments and strings when they are not reliable evidence.
- Skip TOML postmortem records in prose-only scans.
- Require explicit evidence for severe claims; bare adjectives are not enough.

## Validation Notes

- Targeted tests passed for prompt verification, slice-risk, postmortem smoke, severity-discipline, and schema contracts.
- The repo quick lane also passed via `just fast`.

