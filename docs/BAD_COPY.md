# Bad Copy-Code Behavior

This document covers the copy-code failure mode that Jankurai treats as a
high-confidence audit signal when it appears in active product source.

## Known Best Practices

Use these practices to avoid copy-code drift:

- Keep one owner for one behavior.
- Move shared logic behind a named boundary before the third copy appears.
- Test behavior before extracting helpers.
- Prefer explicit module boundaries over vague utility dumping.
- Keep generated, vendor, build, cache, dependency, and snapshot output out of
  the hard-evidence path.

## Never Excused Hard Classes

The `copy-code` lane treats these as hard duplicate classes when they appear in
active source:

1. Exact duplicate active source files.
2. Exact same-name function, method, or component copies across different
   active source files.

These classes affect score and hard caps. They are reserved for high-confidence
evidence only.

## Advisory Classes

The lane also emits warning-only classes for duplicated code that is useful to
review but not strong enough to hard-fail:

1. Exact same-body different-name units.
2. Strict token or block duplication that clears the configured thresholds.
3. Duplicates in warning-only scopes such as tests, fixtures, stories, config,
   Docker, and migrations when `--include-tests` is set.

These warnings are ranked by duplicated volume. They do not reduce score.

## False-Positive Policy

The scanner is precision-first:

- It preserves identifiers and literals for hard duplicate checks.
- It only normalizes line endings, trailing whitespace, blank-line noise, and
  indentation where needed to compare code shape.
- It excludes generated, vendor, build, cache, lockfile, docs, reference, and
  tips paths from hard evidence.
- It does not compare unrelated languages.

If a warning looks noisy, tighten the threshold or exclusion scope before
loosening the detector.

## Command

Run the dedicated lane with:

```bash
cargo run -p jankurai -- copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md
```

Use `--include-tests` only when you want warning-only scopes such as tests,
fixtures, stories, config, Docker, and migrations included in the scan.
Use `--strict` when you want the command to exit nonzero on hard classes.

## Tools Matrix

| Tool | Role in jankurai | Hard-fail? | Status in v1.3 |
|---|---|---|---|
| Built-in Rust scanner | Primary; SHA-256 exact + token-window + regex unit extraction | Yes, narrow | Required |
| `jscpd` | Optional cross-check, Rabin-Karp polyglot scan | No, advisory | Optional via `--cross-check jscpd` |
| PMD CPD | Cross-language token scan, strict precision | No | Deferred (post-v1.3) |
| tree-sitter | AST-aware unit extraction (replaces regex eventually) | No | Deferred (post-v1.3) |
| SonarQube | Trend dashboards | No | Not in audit path |

## Allowlist

`agent/copy-code-allowlist.toml` lets owners suppress noisy fingerprints with a
written reason and (optionally) an expiry. Suppressed classes still appear in
the report with `suppressed.by = "allowlist"` for visibility but do not affect
score. Expired entries are ignored (the finding rehydrates to its scanner
severity).

Example:

```toml
[[entries]]
fingerprint = "a1b2c3d4e5f60718"
owner       = "@alice"
reason      = "Adapter boilerplate; extraction tracked in JANK-123"
expires     = "2026-08-01"
```

## Stack-Rank Quick Use

```bash
jankurai copy-code rank --top 20                  # all classes by total redundant lines
jankurai copy-code rank --kind hard-only          # focus on inexcusable findings
jankurai copy-code rank --by tokens               # sort by redundant token volume
```

## Repair Guidance

When hard copy-code appears:

- Consolidate ownership into one file or one semantic unit.
- Keep the other call sites thin and obvious.
- Add a behavior test before or with the extraction.
- Do not dump the shared code into a catch-all helper module.
- If the duplication is intentional, write down the owner and the reason in the
  relevant docs or exception record.

The goal is not to eliminate all resemblance. The goal is to stop copied active
behavior from silently diverging.

## Scoring Impact

Copy-code hard classes reduce the **Code shape** dimension by 10 points and apply
the `severe-duplication-in-product-code` score cap (maximum score = 70) while
any hard class is present. This is intentionally narrow:

- The cap applies only to `ExactFile` and `ExactUnitSameName` hard classes.
- `ExactUnitDifferentName` and `TokenBlock` findings are advisory; they never
  reduce score or apply a cap.
- Allowlist-suppressed classes are demoted to warning; they do not trigger
  the cap even if the underlying fingerprint would otherwise be hard.

To clear the cap: remove the duplication or suppress it via
`agent/copy-code-allowlist.toml` with a documented owner and reason.

## Workspace Carve-Outs

The scanner automatically demotes these file types to warning-only regardless
of content, because duplication is expected by convention:

- `Cargo.toml`, `package.json`, `tsconfig.json`, `pyproject.toml`, `setup.cfg`
  — workspace member manifests share boilerplate by design.
- `tests/`, `fixtures/`, `stories/`, `config/`, `docker/`, `migrations/`
  — these scopes are always warning-only unless `--include-tests` is set.
- Derive macro lines (`#[derive(...)]`, `#[serde(...)]`) and bare `impl` headers
  are stripped from token-window matching to reduce idiomatic boilerplate FPs.
