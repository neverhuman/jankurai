# Jankurai Guard

Jankurai Guard makes a failed audit impossible for an AI agent to miss. The
moment an agent writes or changes a file, that single file is audited and, if it
fails, the agent is forced to see the failure and fix it before moving on.

It is provider-neutral: it works for Claude, Codex, Gemini, OpenCode, an editor,
or a shell script, because it intercepts at the filesystem and process layer
rather than through any agent's hook API. `AGENTS.md`-style guidance is advisory
and agents skip it; the guard is not advisory.

## Two pieces

### 1. `jankurai audit-file` — the single-file save-gate

Audits one candidate file change without writing it to disk, and returns a
**delta-based** decision: pre-existing repo debt does not block a save; only
findings the candidate *newly introduces*, *worsens*, or leaves in place at
`critical` severity do.

```
jankurai audit-file <repo> --path <rel> [--candidate <file|->] \
  [--op create|modify|delete|rename] [--rename-from <rel>] \
  [--baseline <file>] [--mode save-gate|advisory] \
  [--format agent|json] [--json-out <path>] [--self-audit]
```

Exit codes: `0` pass, `2` advisory, `3` block, `4` internal error. The JSON form
(`--format json`) is the `jankurai-save-gate/1` schema in
`schemas/save-gate.schema.json`; the `agent` form is a human/agent-readable
block listing each blocking finding, its line, and the fix.

The candidate bytes can come from a file, from stdin (`--candidate -`), or — when
`--candidate` is omitted — from the file currently on disk at `--path`. The
baseline for the delta is the caller-supplied `--baseline` file, or the on-disk
file when that flag is omitted, or empty for a brand-new path.

`--self-audit` is only needed when the target repo is the jankurai repo itself.

### 2. `jankurai guard` — the enforcement runtime

`jankurai guard` runs the audit on every file change and enforces the result.
It has two backends that share one audit engine, one enforcement core, and one
CLI:

| Backend | Platforms | When the audit runs | What it guarantees |
|---|---|---|---|
| **Watcher** | macOS + Linux | Just after the write lands | Detects the change in milliseconds, then reverts to last-good / quarantines / poisons. Cannot block the write itself. |
| **FUSE** | Linux | Before the bytes reach the real repo | The write is buffered; on a block the real repo is never touched. True pre-save blocking. |

```
jankurai guard watch <repo>      # cross-platform: detect-and-react
jankurai guard mount <repo>      # Linux: guarded FUSE mount, block-before-write
jankurai guard run -- <agent>    # mount-or-watch, launch the agent, inject feedback
jankurai guard status            # mount liveness, mode, blocked files
jankurai guard doctor            # check libfuse/macFUSE, backing perms, hooks
jankurai guard install <repo>    # one-time layout + policy scaffold
```

On a block the guard:

1. **Keeps the real repo unchanged** — the last passing version stands.
2. **Quarantines** the rejected candidate under `.jankurai/guard/rejected/<ts>/`.
3. **Poisons** the file with a language-aware, un-ignorable error header
   (`compile_error!` for Rust, `throw` for TypeScript, `raise` for Python, an
   invalid sentinel for JSON/TOML, and so on) so the agent's next compile, test,
   or read hits a wall. The agent's original rejected content is preserved below
   the header between `JANKURAI-POISON-BEGIN` / `JANKURAI-POISON-END` markers.
4. **Writes a failure report** to `.jankurai/guard/failures/<ts>.{md,json}` and
   `.jankurai/guard/LAST_FAILURE.md`.
5. **Injects a failure banner** into the agent's terminal when the agent was
   launched through `jankurai guard run`.

## Modes

Set in `agent/guard-policy.toml` or with `--mode`; resolution order is
flag > policy file > `enforce`.

- `observe` — audit and report only; never alters agent writes.
- `enforce` — the default: revert / quarantine / poison on a block.
- `strict` — `enforce` plus a persisted strict marker so enforcement stays on
  even if the policy file is later relaxed, and the offending path stays locked
  for writes until the failure report is read.

## Platforms

- **Linux** uses `libfuse` for the FUSE backend. Install `libfuse3-dev` (CI does
  this automatically via `ops/ci/lib.sh`'s `ensure_fuse_dev`).
- **macOS** uses the watcher backend by default. The FUSE backend would require
  macFUSE, a kernel extension that needs administrator approval, so it is not
  the default and is not exercised in CI. macOS users get full guard semantics
  (intercept, audit, revert, poison) through the watcher backend; they do not
  get kernel-level pre-save blocking.

The `fuse` Cargo feature gates the FUSE backend, and the `fuser` dependency is
scoped to `cfg(target_os = "linux")`, so `cargo build --all-features` stays
green on macOS without macFUSE.

## Honest limits

- The watcher backend is **post-write**. The bytes briefly land before they are
  reverted or poisoned. It is detect-and-react, not block-before.
- A FUSE `release`/`close` error is not reliably surfaced to the caller, so
  enforcement never *depends* on the errno reaching the agent — the real
  guarantees are "the backing repo is not modified", the poison overlay, the
  failure report, and the PTY banner.
- A process with direct same-user write access to the backing directory can
  bypass FUSE. `jankurai guard doctor` warns when the backing directory is
  agent-writable. Closing that gap fully needs OS-level hardening (Linux
  Landlock/fanotify, macOS Endpoint Security), which is feature-gated and off by
  default.
- The existing Git hooks (`jankurai hooks install`) remain as the commit-time
  backstop; the guard is the realtime layer, not a replacement for them.
