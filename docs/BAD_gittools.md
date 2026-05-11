# Bad Git Tools Behavior

## Core Thesis

Local Git hooks are fast feedback, not authority. They help contributors catch
formatting, lint, secret, conflict-marker, and commit-message mistakes before CI
does, but they are clone-local, install-dependent, and often bypassable.

Authoritative enforcement belongs in CI, protected branches, repository
rulesets, or server-side hooks. Hooks should be deterministic, quick, visible,
versioned, and mirrored remotely. Slow, surprising, hidden, unversioned, or
bypass-normalizing hook behavior is bad tooling.

## Hard Findings vs Advisory Signals

Hard findings require high-confidence evidence that checked-in automation can
damage local state, bypass checks as routine policy, disable hooks, or stage
unreviewed work. Ambiguous best-practice gaps stay advisory until there is
repository-local proof that the behavior is intentionally unsafe.

Advisory signals include missing CI parity, slow local suites, unpinned hook
versions, missing onboarding docs, personal hook paths, and missing repository
hygiene files. These are useful review prompts, not merge-blocking findings by
themselves.

## Native Hooks

Native hooks are executable scripts under Git's hook path. They are useful for
fast checks against staged content, commit messages, and push intent. They are
bad when they become hidden policy, run destructive Git commands, mutate broad
worktree state, depend on undeclared local tools, or block commits with opaque
errors.

Good hook scripts are small wrappers around normal project commands. They should
avoid network calls, full slow suites, broad filesystem mutation, and implicit
restaging. Important policy must be repeated in CI or server-side enforcement.

## Hook Managers and Distribution

Hook managers such as pre-commit, Husky, Lefthook, Overcommit, CaptainHook, and
lint-staged make hook policy versioned and repeatable. They are bad when they
hide behavior in lifecycle scripts, leave tool versions floating, assume every
contributor has one ecosystem installed, or make bypassing checks part of the
documented workflow.

Checked-in `.githooks/` directories are acceptable only with explicit install
steps, executable scripts, and CI mirrors. Raw writes into `.git/hooks/*` are not
team policy; they mutate per-clone state that is invisible to review.

## Commit-Quality Tooling

Formatters, linters, type checks, secret scanners, large-file checks, conflict
marker checks, and commit-message rules are good local feedback when they are
fast and deterministic. They become bad when pre-commit hooks run full project
suites, inspect unrelated unstaged files, or modify files beyond the staged set.

lint-staged commands should not run `git add .` or `git add -A`. The tool owns
safe staged-file handling; manual project-wide restaging defeats review.

## Repository Policy Files

Repository policy files should make expected behavior visible:

- `.pre-commit-config.yaml` should pin hook revisions and have CI parity.
- `.husky/` scripts should be small, POSIX-friendly wrappers.
- `lefthook.yml` should avoid racing file-mutating jobs.
- `.lintstagedrc*` should run tools against staged file lists.
- `commitlint.config.*` should enforce documented release/review rules.
- `CONTRIBUTING.md` should document setup, hooks, checks, and bypass policy.

Missing hygiene files are advisory only. Their absence is not a hard finding
without additional proof that policy is being bypassed or hidden.

## Git Config Hardening

`core.hooksPath` is useful for committed hook directories or documented
organization hook paths. Checked-in automation that sets it to `/dev/null` is a
hard finding because it disables hooks as policy. Absolute personal hook paths
are advisory unless they are used to bypass required checks.

Prefer explicit push defaults, CRLF/whitespace safety, commit templates, and
blame-ignore files where they match the project. These reduce friction but do
not replace review, CI, or server policy.

## CI and Server Enforcement

CI should rerun the important checks that local hooks provide and add the slower
tests that do not belong in commit hooks. Protected branches, required status
checks, repository rulesets, and server-side hooks are the enforcement layer for
shared history.

Server-side hooks should be fast, deterministic, auditable, and specific. They
should reject pushes for hard repository policy such as forbidden refs, unsigned
commits where required, invalid release messages, known secret patterns, or
regulated file paths.

## Highest-Confidence Audit Detectors

Hard detectors should stay narrow:

- Routine `git commit --no-verify`, `git push --no-verify`, or bare
  `--no-verify` inside hook-manager or release automation.
- Checked-in `core.hooksPath /dev/null` or equivalent hook disabling.
- Team installers that write, copy, symlink, chmod, or delete `.git/hooks/*`.
- Hook or hook-manager commands that run destructive Git operations such as
  hard reset, broad restore, clean, stash-all, ref deletion, history rewrite,
  `.git` deletion, or force push.
- Hook or hook-manager commands that run `git add .`, `git add -A`, or
  `git commit -am`.
- lint-staged commands that manually run project-wide restaging.

## Good Default Policy

Use local hooks for fast staged-file checks, commit-message checks, and obvious
secret/conflict/file-size prevention. Use pre-push or explicit local commands
for medium-cost checks. Use CI for full lint, tests, build, docs, dependency
scans, and security scans. Use branch protection, rulesets, and server hooks for
policy that must not be bypassed.

Document the install command, update command, bypass policy, and CI mirror. Make
emergency bypasses explicit, rare, logged, and followed by remediation.

## Repair Guidance

Remove destructive Git commands from hooks first. Replace broad staging with
explicit file lists or hook-manager staged-file support. Remove routine
`--no-verify` use from package scripts, hook configs, CI, and release scripts.
Replace raw `.git/hooks` installers with a committed hook directory or a hook
manager.

After repair, run the focused language bad-behavior tests and the audit lane.
If the repo intentionally needs a risky hook, add a narrow, expiring
`jankurai:allow <detector-id> reason=... expires=YYYY-MM-DD` comment next to the
specific command and preserve CI or server enforcement as the authority.
