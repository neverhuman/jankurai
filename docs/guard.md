# Jankurai Guard

Jankurai Guard makes a failed audit impossible for an AI agent to miss. The
moment an agent writes or changes a file, that single file is audited and, if it
fails, the agent is forced to see the failure and fix it before moving on.

It is provider-neutral: it works for Claude, Codex, Gemini, OpenCode, an editor,
or a shell script, because it intercepts at the filesystem and process layer
rather than through any agent's hook API. `AGENTS.md`-style guidance is advisory
and agents skip it; the guard is not advisory.

The recommended workflow does not require a long-running daemon. Use
`jankurai audit-file` for hooks and editor integrations, `jankurai guard run --
<agent>` for one supervised agent session, or `jankurai guard watch <repo>` as a
foreground terminal process. Stop foreground guard sessions with `Ctrl-C`.

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
| **audit-file** | macOS + Linux + Windows | Caller-provided candidate bytes | One candidate in, one save-gate decision out. No resident process. |
| **Watcher** | macOS + Linux | Just after the write lands | Detects the change in milliseconds, then reverts to last-good / quarantines / poisons. Cannot block the write itself. |
| **FUSE** | Linux | Before the bytes reach the real repo | Foreground guarded mount. The write is buffered; on a block the real repo is never touched. True pre-save blocking. |

```
jankurai guard watch <repo>      # cross-platform: detect-and-react
jankurai guard mount <repo>      # Linux: foreground FUSE mount, block-before-write
jankurai guard run -- <agent>    # foreground watcher, launch the agent, inject feedback
jankurai guard status            # mount liveness, mode, blocked files
jankurai guard doctor            # check backend, backing perms, hooks, session
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
  this automatically via `ops/ci/lib.sh`'s `ensure_fuse_dev`), then build with
  `cargo install --path crates/jankurai --locked --features guard-fuse`.
  `jankurai guard mount . --mount-point /tmp/jankurai-guard` runs in the
  foreground; point the agent/editor at the mount and stop it with `Ctrl-C`.
- **macOS** uses `audit-file`, `guard run`, and the watcher backend. This
  release does not link a macFUSE backend, so installing macFUSE is not required
  and will not make `guard mount` available. This avoids kernel-extension
  approval and crash-prone resident process expectations until a macOS mount
  backend is implemented and tested directly.

The `fuse` Cargo feature gates the FUSE backend, and the `fuser` dependency is
scoped to `cfg(target_os = "linux")`, so `cargo build --all-features` stays
green on macOS without macFUSE.

## macOS Setup

Use the no-daemon paths:

```
jankurai audit-file . --path src/main.rs --candidate src/main.rs --op modify
jankurai guard run -- claude
jankurai guard watch .
jankurai guard doctor .
```

For editor integrations, call `audit-file` with the candidate buffer before the
editor commits the save. For agents, prefer `guard run -- <agent>` so the
watcher lifetime is tied to that agent process.

## Linux FUSE Setup

Use FUSE only when you need true pre-write blocking:

```
sudo apt-get install libfuse3-dev pkg-config
cargo install --path crates/jankurai --locked --features guard-fuse
jankurai guard doctor .
jankurai guard mount . --mount-point /tmp/jankurai-guard
```

Keep the mount command in the foreground. Edit through
`/tmp/jankurai-guard`; the backing repo is updated only after the save-gate
passes. Stop with `Ctrl-C`. If the process is interrupted and the mount remains,
run `jankurai guard unmount .` or the platform `fusermount3 -u
/tmp/jankurai-guard`.

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


## Relationship to smart scan mode

`jankurai audit .` and `jankurai guard` are complementary layers. The guard
fires per-write in realtime; `jankurai audit .` runs the full 10-dimension score
on demand or in CI.

After a clean full audit (`hard_findings = 0`, `caps_applied = []`), `jankurai
audit .` switches to **smart scan mode**: only the files reported by `git status`
are re-audited. A full rescan is forced hourly (or on 10% of sessions) even in
smart mode, so drift never accumulates. Use `--full` to force a complete scan at
any time.

The guard watcher does not replace smart scan — it runs on every single write,
before `git status` can even accumulate a diff.
