# Language Bad-Behavior Audit

This document defines the language-specific and repository-shape bad-behavior audit family.
The active detector pack covers Rust, SQL, TypeScript, Docker, Python, CI, Git,
GitTools, Release, WebSecurity, and RepoRot so the scanner can grow without
changing the report shape again.

## Stable Rule IDs

| Rule | Meaning |
| --- | --- |
| `HLT-029-RUST-BAD-BEHAVIOR` | Rust code uses unsafe, unchecked, or dishonest APIs without local proof |
| `HLT-030-SQL-BAD-BEHAVIOR` | SQL code or migrations use unsafe string assembly or unchecked execution without proof |
| `HLT-031-TYPESCRIPT-BAD-BEHAVIOR` | TypeScript code uses unchecked boundary or runtime shortcuts without proof |
| `HLT-032-DOCKER-BAD-BEHAVIOR` | Docker or container build behavior hides unsafe or unreviewed execution steps |
| `HLT-033-PYTHON-BAD-BEHAVIOR` | Python code owns runtime behavior or unchecked data paths without an approved exception |
| `HLT-034-CI-BAD-BEHAVIOR` | CI workflows hide unsafe, unpinned, or nonblocking security and proof behavior |
| `HLT-035-GIT-BAD-BEHAVIOR` | Git automation or hooks use destructive, hidden-state, or unreviewed mutation behavior |
| `HLT-036-GITTOOLS-BAD-BEHAVIOR` | Git hook managers or policy tooling normalize bypass, destructive mutation, or broad staging |
| `HLT-037-RELEASE-BAD-BEHAVIOR` | Release automation mutates tags/artifacts, skips proof, ships mutable latest-only outputs, or publishes without integrity evidence |
| `HLT-039-WEB-SECURITY-BAD-BEHAVIOR` | Web app security hazards appear in explicit source, config, env, or CORS surfaces |
| `HLT-040-REPO-ROT-BAD-BEHAVIOR` | Active source contains old, backup, copied, parked, or disabled code without a bounded exception |

## Detector Tiers

### Rust, hard-scored today

Rust findings are emitted only when the scanner sees a high-confidence bad
behavior and the nearby proof is missing.

Hard findings currently include:

| Family | Examples |
| --- | --- |
| Unsafe and soundness | undocumented `unsafe` blocks, public `unsafe fn` without `# Safety`, unsafe trait impls without proof, `transmute`, `MaybeUninit::assume_init`, `mem::zeroed`, `get_unchecked`, `unwrap_unchecked`, `unreachable_unchecked`, `from_utf8_unchecked`, `from_raw_parts`, `Box::from_raw`, `Vec::from_raw_parts`, `CString::from_raw` |
| Layout and ABI | `static mut`, `repr(packed)`, unsafe FFI boundaries, unsafe linking or export behavior |
| Panic and lint abuse | `#![allow(warnings)]`, `#![allow(clippy::all)]`, `#![allow(clippy::correctness)]`, `RUSTFLAGS=-A warnings` |
| Async and concurrency | `block_on` in async context, unbounded channel use in production paths, other hard concurrency shortcuts without proof |
| Shell execution | shell invocation with dynamic command text, especially `Command::new("sh"|"bash").arg("-c")` |

### Rust, advisory only for now

These are review signals, not hard findings yet:

| Family | Examples |
| --- | --- |
| Ownership shortcuts | raw `clone`, `Arc<Mutex<_>>`, `Rc<RefCell<_>>`, broad `as` casts |
| API honesty | public `repr(C)` without the rest of the contract, `Pin` where the proof is still human-only |
| Concurrency hints | atomics without an explicit ordering story, generic `unwrap` in non-test code |

### SQL, TypeScript, Docker, Python, CI, Git, GitTools, Release

These modules use deterministic, repository-local detectors and stable rule
IDs. They are designed to stay narrow, high-confidence, and proof-gated rather
than broad heuristics over arbitrary text.

| Language | Current state |
| --- | --- |
| SQL | detector-backed |
| TypeScript | detector-backed |
| Docker | detector-backed |
| Python | detector-backed |
| CI | detector-backed |
| Git | detector-backed |
| GitTools | detector-backed |
| Release | detector-backed |

### Web Security And Repo Rot Detectors

`HLT-039-WEB-SECURITY-BAD-BEHAVIOR` is hard-scored only for high-confidence
web hazards:

| Detector | Hard-scored examples |
| --- | --- |
| `websec.vite.public-dev-server` | `allowedHosts: true`, `host: "0.0.0.0"`, `cors: true`, or `server.fs.strict: false` in `vite.config.*` |
| `websec.env.client-secret` | `VITE_*` names that contain secret, password, private, database, AWS, client-secret, or non-public token terms |
| `websec.storage.token` | token, JWT, session, password, or authorization material in `localStorage` or `sessionStorage` |
| `websec.cors.credential-wildcard` | wildcard/arbitrary CORS origin in the same local window as enabled credentials |

Web security review signals that are not hard caps yet include unsafe CSP
script sources, unvalidated redirect-like navigation, and public sourcemaps
without private upload evidence.

`HLT-040-REPO-ROT-BAD-BEHAVIOR` flags active source paths that look like old,
backup, copied, parked, or fake-versioned implementations. Commented-out code
blocks, hard-disabled branches, and checked-in archive snapshots are emitted as
soft findings or advisory signals. Valid API versions, versioned contracts, DB
migrations, docs, tests, fixtures, examples, generated output, `tips/`, and
`reference/` are excluded.

## False-Positive Policy

Hard findings require both a risky construct and missing proof.

Treat as no finding when:

| Case | Reason |
| --- | --- |
| File is under `tests/`, `docs/`, `tips/`, `reference/`, or generated zones | these are not runtime proof targets for the Rust detector |
| Unsafe block has a nearby meaningful `SAFETY:` comment | the proof is local and reviewable |
| Public `unsafe fn` has a nearby `# Safety` section | caller obligations are documented |
| `Command::new("git")`, `Command::new("cargo")`, or another fixed executable is used without a shell | no shell interpretation occurs |
| `unwrap` appears only in tests or in an obviously local invariant with a clear message | the failure is intentionally bounded |
| `mpsc::unbounded_channel` appears in tests | test-only code is not production runtime behavior |
| `jankurai:allow <detector-id> reason=... expires=YYYY-MM-DD` is present nearby | the exception is explicit, scoped, and expiring |
| Web security signal is absence-based, such as missing CSP/CSRF by itself | absence-only checks need runtime proof and should not be hard static findings |
| Repo path is a DB migration, public API version, test fixture, docs, generated output, `tips/`, or `reference/` | old-looking names are legitimate in those scopes |

## Proof Lane

Rust language bad-behavior proof is routed through:

| Lane | Purpose |
| --- | --- |
| `proofmark-rust` | changed Rust line coverage, focused mutation, and negative proof receipt evidence |

For detector-focused checks, run:

```bash
cargo test -p jankurai --test language_bad_behavior
cargo test -p jankurai --test web_security_and_repo_rot
```

The detector pack must never rely on runtime reads of `tips/`; the corpus is
source material only. The scanner should use repo text that already exists in
the audit inventory.

## Reporting Shape

Language findings should carry:

| Field | Meaning |
| --- | --- |
| `rule_id` | stable `HLT-029` through `HLT-040` bad-behavior rule IDs |
| `matched_term` | the specific subrule or detector id |
| `reason` | why the proof is insufficient |
| `evidence` | path, line, snippet, detector id, and proof-window result |

The goal is to keep the report deterministic and narrow enough for agents to
repair without broad grep work.
