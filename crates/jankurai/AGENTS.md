# Jankurai Auditor Crate

Read the root `AGENTS.md` and `agent/JANKURAI_STANDARD.md` first.

This crate owns the Rust auditor, CLI commands, report schemas, renderers, and
conformance checks. Keep proof and repo tooling Rust-first; do not add Python
helpers for auditor behavior.

For scoring or report changes, add focused Rust tests under
`crates/jankurai/tests/`, then run:

```bash
cargo test -p jankurai
cargo run -p jankurai -- versions
```

Do not hand-edit generated report artifacts. Regenerate score outputs with the
root `just score` lane.
