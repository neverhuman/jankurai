# Bad PYTHON Behavior: Comprehensive Guide

This document organizes the worst PYTHON behaviors that are inexcusable in production.

## Known Best Practices

To counteract the anti-patterns listed below, ensure adherence to these core PYTHON best practices:

- **Use type hints everywhere**: Annotate function signatures and use `mypy` or `pyright` in strict mode.
- **Manage dependencies deterministically**: Use modern tools (Poetry, uv, pip-tools) with lockfiles.
- **Handle exceptions specifically**: Catch narrow exception types and avoid bare `except
- **Follow PEP 8 and use formatters**: Enforce `black`, `ruff`, and `isort` via pre-commit hooks.
- **Favor composition and pure functions**: Keep classes small and avoid deep inheritance trees.

## 1. Arbitrary code execution

1. `eval()` on anything not hardcoded and fully controlled.
2. `exec()` on anything not hardcoded and fully controlled.
3. `compile(..., "exec")` or `compile(..., "eval")` on dynamic strings.
4. Building Python source with f-strings or string concatenation, then executing it.
5. Accepting “safe because it came from the LLM” generated code and running it.
6. Letting config files be Python code when the config author is outside the trust boundary.
7. Importing user-controlled modules, plugin names, class names, or dotted paths without a strict allowlist.
8. Using `globals()[name]`, `locals()[name]`, or `getattr(module, name)` as a user-controlled router.
9. Using `__import__`, `importlib.import_module`, entry-point discovery, or plugin loading on untrusted names.
10. Using `base64.b64decode(...); exec(...)`, `marshal.loads(...); exec(...)`, or any obfuscated execute pattern.
11. Treating `ast.parse()` as a sandbox. Parsing code is not the same as safely running code.
12. Letting notebooks, markdown, README snippets, GitHub issue text, PR comments, or agent messages become executable Python.
13. Letting an agent “fix an error” by adding an `eval`, `exec`, dynamic import, or generated-code runner.
14. Using Python as a templating language for untrusted templates.
15. Shipping self-modifying code, hidden import hooks, or runtime monkeypatching that changes security behavior.
16. Creating “admin consoles,” “debug shells,” or “quick REPL endpoints” inside production apps.

## 1. Code execution and command execution sins

1. Never execute user input with `eval()`.
2. Never execute user input with `exec()`.
3. Never compile and run untrusted strings with `compile()`.
4. Never pass untrusted data into `getattr()`, `globals()`, `locals()`, or `__import__()` to select executable behavior.
5. Never dynamically import modules based on unsanitized user input.
6. Never load plugins from writable, user-controlled, temp, downloaded, or current-working-directory paths without verification.
7. Never run generated Python code in production without sandboxing and review.
8. Never run notebook cells from untrusted notebooks.
9. Never treat a string from an LLM, database, config file, queue, webhook, or API request as executable Python.
10. Never use `os.system()` with variable input.
11. Never use `subprocess(..., shell=True)` with any tainted input.
12. Never build shell commands using f-strings, `%`, `.format()`, concatenation, or template substitution with untrusted data.
13. Never pass a command string when an argument list is available.
14. Never rely on quoting as your primary defense when avoiding the shell is possible.
15. Never invoke shell metacharacters, pipes, redirects, globs, or command substitution using data that came from a user, file, database, LLM, webhook, or environment variable.
16. Never call external commands without checking exit status.
17. Never ignore `subprocess` return codes.
18. Never run commands without timeouts in services, CI, workers, or agents.
19. Never let a web request directly trigger arbitrary command execution.
20. Never let an agent choose arbitrary shell commands against a real machine without an allowlist and sandbox.
21. Never invoke package managers, cloud CLIs, Docker, Kubernetes, Terraform, SSH, or database CLIs from Python using untrusted arguments.
22. Never run shell commands as root unless the command is static, audited, and unavoidable.
23. Never let untrusted input control `PATH`, `PYTHONPATH`, working directory, interpreter path, or command name.
24. Never hide command execution inside “helper” functions that look harmless.
25. Never accept “it’s internal” as a justification for command injection.

## 10. Dependencies, packaging, and supply chain

1. Unpinned dependencies in production builds.
2. `pip install package` with no lockfile, no hashes, and no review.
3. Mutable dependency ranges for deploys.
4. Docker base images pinned to `latest`.
5. Installing hallucinated packages from LLM output.
6. Installing packages because an agent suggested them without checking provenance.
7. Typosquatted or slopsquatted package names.
8. Installing from random GitHub repos, gists, raw URLs, or HTTP indexes.
9. Runtime `pip install` based on user input.
10. Running untrusted package setup/build hooks on privileged machines.
11. Building source distributions from unknown packages in CI with secrets present.
12. Ignoring dependency advisories.
13. Ignoring `pip-audit`, Dependabot, GitLab dependency scanning, or equivalent alerts.
14. No review of transitive dependency changes.
15. Vendored code with unknown origin.
16. Vendored code with incompatible licenses.
17. Generated code with no provenance.
18. Binary blobs in the repo with no source or checksum.
19. Private package names not reserved publicly where dependency confusion is possible.
20. Mixing public and private indexes unsafely.
21. Publishing packages with secrets in metadata, wheels, sdists, notebooks, tests, or docs.
22. Trusting package import names that shadow standard library modules.
23. A local `random.py`, `email.py`, `jwt.py`, `typing.py`, `logging.py`, or `asyncio.py` that breaks imports.
24. Skipping reproducible builds for critical services.
25. No SBOM or artifact provenance for high-risk systems.
26. Disabling dependency scanning because “it’s noisy.”

## 10. File, path, temp-file, and filesystem sins

1. Never join paths with untrusted filenames and assume the result is safe.
2. Never allow `../` traversal.
3. Never trust normalized-looking paths without resolving and checking the base directory.
4. Never write files using user-supplied absolute paths.
5. Never delete files using user-supplied paths without strict allowlists and invariant checks.
6. Never recursively delete a path that was not constructed and verified by your code.
7. Never follow symlinks unexpectedly in upload, extraction, cleanup, or temp directories.
8. Never create world-writable files containing sensitive data.
9. Never create world-writable directories for application state.
10. Never use `tempfile.mktemp()`.
11. Never hand-roll temporary filenames.
12. Never write temp files into predictable locations.
13. Never assume `/tmp` is private.
14. Never leave sensitive temp files behind.
15. Never read entire untrusted files into memory without size limits.
16. Never process user uploads without quota, type validation, malware policy where relevant, and cleanup.
17. Never trust file extensions.
18. Never trust image libraries to be safe from decompression bombs without limits.
19. Never let an agent write outside the repository or workspace unless explicitly allowed.
20. Never run code that assumes the current working directory is safe.
21. Never rely on relative paths in privileged scripts.
22. Never put writable directories ahead of trusted directories in import or executable search paths.
23. Never name project files `json.py`, `typing.py`, `email.py`, `requests.py`, `logging.py`, or other stdlib/package-shadowing names.
24. Never run Python scripts from untrusted directories without considering import-path hijacking.
25. Never chmod broadly, such as `chmod -R 777`, to “fix permissions.”

## 11. Agent-hostile Python behavior

1. Import-time side effects: network calls, DB writes, filesystem mutation, cloud calls, subprocesses, logging reconfiguration, environment mutation, thread/process startup.
2. Code that behaves differently depending on the current directory.
3. Code that behaves differently depending on hidden environment variables.
4. Code that silently reads credentials from the developer’s machine.
5. Code that silently writes outside the repo/workspace.
6. Code that silently modifies global process state.
7. Monkeypatching builtins, stdlib modules, TLS verification, auth checks, serializers, or logging globally.
8. “Convenience” global clients initialized at import time with production credentials.
9. Hidden singletons with mutable state.
10. Mutable default arguments.
11. Bare module-level code that performs real work.
12. Scripts that lack `if __name__ == "__main__":`.
13. Functions that mix parsing, auth, IO, business logic, retries, logging, and mutation in one block.
14. Huge untestable files named `utils.py`, `helpers.py`, or `misc.py` containing security-critical behavior.
15. Dynamic imports for normal control flow.
16. Reflection-based routing instead of explicit allowlists.
17. Magic decorators that change authorization, serialization, or transaction boundaries invisibly.
18. Runtime patching to “make tests pass.”
19. Generated code that no human can explain.
20. Code comments that contradict behavior.
21. Dead code paths that still contain dangerous behavior.
22. Commented-out secrets, commands, credentials, or vulnerable snippets.
23. Relying on agent memory instead of explicit repo documentation.
24. Requiring a specific local machine setup that is not declared.
25. Hiding required services, credentials, or commands outside README/config.
26. Non-deterministic tests due to time, randomness, network, or external APIs.
27. No schema at boundaries.
28. No typed boundary objects for agent-edited APIs.
29. Security decisions spread across unrelated files.
30. “Temporary” bypasses without issue, owner, and expiry.

## 11. Network, HTTP, API, and SSRF sins

1. Never call `requests.get()` or similar network functions without timeouts in services, workers, CLIs, or CI.
2. Never let user input choose arbitrary outbound URLs.
3. Never fetch URLs from webhooks, uploaded files, metadata, OpenGraph tags, PDFs, XML, or LLM output without SSRF defenses.
4. Never allow server-side fetches to internal IP ranges unless explicitly intended.
5. Never allow server-side fetches to cloud metadata IPs.
6. Never follow redirects blindly for user-controlled URLs.
7. Never forward internal headers, cookies, or tokens to user-controlled URLs.
8. Never disable TLS verification.
9. Never send credentials over plaintext HTTP.
10. Never expose debug servers to public interfaces.
11. Never run Flask/Django/FastAPI debug mode in production.
12. Never expose Werkzeug/Django debug consoles.
13. Never bind development servers to `0.0.0.0` on untrusted networks.
14. Never trust inbound webhook payloads without verifying signatures.
15. Never trust IP allowlists alone for webhook security.
16. Never parse API responses without schema validation.
17. Never assume third-party APIs always return expected fields.
18. Never log full request/response bodies when they may contain secrets or PII.
19. Never implement clients without backoff, timeout, and rate-limit handling.
20. Never let an agent browse, scrape, or call APIs with privileged credentials without domain allowlists.

## 11. Operational and deployment anti-patterns

1. Run Python services as root unless there is a narrowly justified container/runtime reason and compensating controls.
2. Run apps with write access to their own source code.
3. Mount Docker socket into app containers.
4. Give app containers cloud-admin IAM roles.
5. Use one environment for dev/staging/prod.
6. Use production databases for local testing.
7. Run migrations automatically on every container start without coordination.
8. Deploy without health checks.
9. Deploy without rollback.
10. Deploy without timeouts.
11. Deploy without resource limits.
12. Deploy without dependency vulnerability visibility.
13. Store local state in containers that are expected to be ephemeral.
14. Use SQLite for multi-writer production workloads where it cannot meet concurrency/recovery requirements.
15. Ignore backups and restore tests.
16. Encrypt backups with keys stored beside the backups.
17. Keep logs forever without retention and privacy controls.
18. Disable audit trails because they cost money.
19. Expose internal services to the public internet for convenience.
20. Leave default passwords or default admin accounts.
21. Use shared admin accounts.
22. Disable MFA on admin, cloud, GitHub, GitLab, PyPI, or CI accounts.
23. Use long-lived deploy keys with broad write access.
24. Let CI/CD deploy from unprotected branches.
25. Let untrusted PRs run privileged CI.

## 12. Data integrity and privacy failures

1. Trust client timestamps, prices, balances, permissions, quotas, or IDs.
2. Trust data simply because it came from your frontend.
3. Trust data simply because it came from your queue.
4. Process messages without schema validation.
5. Ignore duplicate events.
6. Ignore idempotency for payments, webhooks, order creation, email sending, and job processing.
7. Store PII you do not need.
8. Log PII by default.
9. Put PII in metrics labels.
10. Put secrets or PII in exception tracking breadcrumbs.
11. Export customer data without authorization checks.
12. Build admin bulk actions without dry-run, confirmation, and audit logs.
13. Mix tenants in caches without tenant-scoped keys.
14. Cache personalized responses publicly.
15. Use user-controlled cache keys without normalization.
16. Use pickle-backed caches for untrusted data.
17. Use stale authorization data indefinitely.
18. Fail open when policy services are unreachable.
19. Delete data without soft-delete/recovery where business risk requires recoverability.
20. Build “temporary” data exports with no retention policy.

## 12. Data, privacy, and logging sins

1. Never log passwords, tokens, cookies, Authorization headers, API keys, private keys, or session IDs.
2. Never log full request headers by default.
3. Never log full request bodies by default.
4. Never log PII unless there is a clear need, retention policy, and redaction.
5. Never store sensitive data in debug dumps.
6. Never commit production data samples.
7. Never use real customer data in tests unless approved and protected.
8. Never paste customer data into LLM prompts without policy approval.
9. Never use production databases for local development.
10. Never export data without encryption and access controls.
11. Never store backups with weaker protection than production.
12. Never leave data dumps in object storage without access controls.
13. Never store sensitive data in notebooks committed to repos.
14. Never store raw webhook payloads forever without retention rules.
15. Never use analytics events to smuggle sensitive fields.
16. Never store more sensitive data than the application needs.
17. Never keep sensitive data longer than needed.
18. Never allow broad admin data access without audit logs.
19. Never print DataFrames containing sensitive columns in shared logs.
20. Never train, fine-tune, or prompt with private data without explicit policy.

## 13. Error handling that fails open

1. `except: pass`.
2. `except Exception: pass`.
3. Returning success after an exception.
4. Security checks that fail open.
5. Auth checks inside broad `try/except` blocks that default to allow.
6. Permission checks that default to admin, owner, or public.
7. Swallowing database write failures.
8. Swallowing audit-log failures for sensitive actions.
9. Swallowing transaction failures.
10. Retrying money-moving, email-sending, deletion, or mutation operations without idempotency.
11. Retrying after partial success without tracking state.
12. Logging only “something went wrong.”
13. Raising raw internal exceptions to users.
14. Returning stack traces in production.
15. Hiding scanner findings with blanket ignores.
16. Suppressing warnings globally.
17. `# noqa`, `# type: ignore`, `# nosec`, or `skipcq` without narrow scope and justification.
18. Catching `BaseException`.
19. Catching `KeyboardInterrupt` or `SystemExit` accidentally.
20. Finally blocks that mask original exceptions.
21. Cleanup code that can delete the wrong thing if earlier validation failed.
22. Treating failed validation as “use default.”
23. Treating failed secret retrieval as “use dev secret.”
24. Treating failed TLS validation as “disable verification.”
25. Treating failed tests as “mark flaky.”

## 13. “Never excuse this in review” quick filters

1. `eval(` or `exec(`.
2. `pickle.load` / `pickle.loads` on external data.
3. `yaml.load(` without safe loader.
4. `subprocess` with `shell=True` and variable input.
5. SQL f-strings.
6. `verify=False`.
7. `SECRET_KEY = "..."`.
8. Passwords/tokens/API keys in source.
9. `random` used for tokens.
10. `assert` used for security/runtime validation.
11. `except: pass`.
12. `# nosec` without a written justification.
13. `# type: ignore` without a written justification.
14. `CORS("*")` with credentials.
15. Debug mode in production config.
16. Public admin/debug/docs endpoints.
17. `chmod 777`.
18. `tempfile.mktemp`.
19. Raw archive extraction.
20. Raw user file paths.
21. `.env` committed.
22. Dependency versions missing or floating.
23. Unreviewed new dependency.
24. CI permissions set to write-all.
25. Tests removed or weakened.
26. Agent-generated code with no tests.
27. “Temporary” security bypass.
28. “Internal-only” used as the entire threat model.
29. “The AI said it’s secure.”
30. “It works” used as a substitute for security review.

## 14. Agent-friendly maintainability sins

1. Never write clever code where clear code would work.
2. Never hide important behavior in import-time side effects.
3. Never perform network calls at import time.
4. Never perform database mutations at import time.
5. Never perform filesystem mutations at import time.
6. Never configure logging destructively at import time in a library.
7. Never use wildcard imports in production modules.
8. Never mutate builtins.
9. Never monkeypatch global behavior outside tightly controlled tests.
10. Never use mutable default arguments for state that can leak between calls.
11. Never use global mutable state for request-specific, user-specific, tenant-specific, or security-specific data.
12. Never store request state in module globals.
13. Never rely on hidden environment variables without documenting them.
14. Never write giant functions that mix parsing, validation, authorization, business logic, I/O, and formatting.
15. Never write giant files that agents and humans cannot reason about.
16. Never duplicate security logic across many places.
17. Never encode business rules only in comments.
18. Never encode security policy only in frontend code.
19. Never hide dangerous behavior behind innocent function names.
20. Never write non-idempotent scripts without clear warnings and dry-run support.
21. Never write migration scripts that cannot be resumed or rolled back.
22. Never write CLIs that default to destructive behavior.
23. Never write scripts that operate on “all” resources by default.
24. Never leave TODOs in security-sensitive paths without an issue, owner, and deadline.
25. Never use magic strings for roles, permissions, states, or scopes without central definitions.
26. Never leave public APIs undocumented.
27. Never leave complex data shapes undocumented.
28. Never use untyped dictionaries everywhere when a dataclass, Pydantic model, TypedDict, or schema would clarify invariants.
29. Never ignore linter warnings about shadowing, unreachable code, unused variables, or broad exceptions.
30. Never optimize for fewer lines over auditability.
31. Never make code hostile to grep, tests, static analysis, or code review.
32. Never make an agent infer hidden contracts that could have been encoded in types, schemas, tests, or docs.
33. Never let generated code sprawl without refactoring around clear boundaries.
34. Never accept “the AI wrote it that way” as an architecture rationale.
35. Never make security-sensitive behavior implicit.

## 14. Logging and observability failures

1. Logging secrets.
2. Logging full request headers when auth/cookies are present.
3. Logging full request bodies by default.
4. Logging full environment variables.
5. Logging database URLs.
6. Logging private keys, tokens, session IDs, password reset links, signed URLs, or OAuth codes.
7. Logging PII without minimization and retention controls.
8. Printing secrets during local debugging and then committing logs/screenshots.
9. Sending logs to third parties without data review.
10. Agent transcripts containing secrets.
11. Agent transcripts stored indefinitely.
12. No audit log for admin actions.
13. No audit log for agent actions.
14. No audit log for permission changes.
15. No audit log for secret access.
16. No correlation IDs for security-relevant requests.
17. No alerting on suspicious auth failures, privilege changes, mass exports, or secret reads.
18. Logs forgeable via newline/control-character injection.
19. Using `print()` debugging in production services instead of structured logging.
20. Debug logs enabled in production.
21. Stack traces exposed to users.
22. Exception handlers that hide all useful forensic detail.
23. Monitoring that cannot distinguish user error from attack.
24. No retention policy.
25. No way to delete/redact sensitive logs.

## 15. Python-specific language-footgun sins

1. Never use mutable defaults like `def f(x=[]):` when mutation can persist across calls.
2. Never use `is` for value equality with strings, numbers, or containers.
3. Never rely on dict ordering in code that claims compatibility with old Python versions.
4. Never shadow builtins like `list`, `dict`, `id`, `type`, `file`, `input`, `open`, or `str`.
5. Never shadow stdlib module names with local filenames.
6. Never rely on `assert` for required runtime behavior.
7. Never ignore timezone awareness in datetime code.
8. Never store local naive datetimes for cross-timezone systems.
9. Never compare naive and aware datetimes casually.
10. Never use floating-point money.
11. Never use binary floats for exact decimal, billing, or accounting logic.
12. Never mutate a collection while iterating over it unless deliberately coded and tested.
13. Never rely on object finalizers for critical cleanup.
14. Never forget context managers for files, locks, DB sessions, temp dirs, and network clients.
15. Never use `except:` to catch `KeyboardInterrupt` and `SystemExit` accidentally.
16. Never use `from x import *` in production modules.
17. Never use hidden module-level side effects as configuration.
18. Never use circular imports as an architecture pattern.
19. Never write async code that blocks the event loop with CPU or synchronous I/O.
20. Never forget `await` and ignore coroutine warnings.
21. Never share non-thread-safe objects across threads without protection.
22. Never assume the GIL makes compound operations logically safe.
23. Never use multiprocessing without understanding serialization and process boundaries.
24. Never rely on object `__del__` for releasing security-sensitive resources.
25. Never use metaprogramming to avoid straightforward code in security-sensitive paths.

## 16. Concurrency, resource exhaustion, and DoS sins

1. Never process untrusted input without size limits.
2. Never process untrusted input without time limits where parsing can be expensive.
3. Never read unbounded request bodies into memory.
4. Never read unbounded files into memory.
5. Never build unbounded lists when streaming would work.
6. Never allow unlimited pagination.
7. Never allow unlimited concurrency.
8. Never allow unlimited queue growth.
9. Never allow unlimited retries.
10. Never allow unlimited recursion.
11. Never run user-selected regexes without complexity controls.
12. Never run attacker-influenced expensive computations synchronously in request handlers.
13. Never call external services without timeouts.
14. Never call external services without circuit breaking or failure policy in production.
15. Never block an async event loop with synchronous `requests`, filesystem, database, or CPU-heavy operations.
16. Never share mutable global caches across tenants without isolation.
17. Never use caches without invalidation for authorization-sensitive data.
18. Never use background workers without dead-letter/error handling.
19. Never allow one bad job to poison the entire worker.
20. Never let an agent create infinite loops, polling loops, or recursive crawlers without hard limits.

## 16. Input validation and boundary design

1. No schema validation at API boundaries.
2. No type validation at CLI boundaries.
3. No validation for queue messages.
4. No validation for webhook payloads.
5. No validation for config files.
6. No validation for LLM/agent output.
7. No validation for database rows read from less-trusted systems.
8. Trusting hidden form fields.
9. Trusting client-side enums.
10. Trusting client-side prices, roles, limits, or ownership claims.
11. Trusting user-supplied timestamps.
12. Trusting user-supplied filenames.
13. Trusting MIME type from the client.
14. Trusting file extensions.
15. Trusting CSV headers.
16. Trusting model output as JSON without parsing, schema validation, and semantic validation.
17. Accepting unknown fields silently in security-sensitive requests.
18. Accepting negative quantities, huge numbers, NaN, Infinity, or overflow-prone values.
19. Accepting timezone-naive datetimes in security/business logic.
20. Accepting ambiguous encodings.
21. Accepting unbounded lists, deeply nested objects, or giant strings.
22. No pagination limits.
23. No upload limits.
24. No normalization before comparison.
25. Case-sensitive security comparisons where identities are case-insensitive.
26. Unicode confusables in usernames, package names, domains, paths, or identifiers without policy.
27. Treating validation as sanitization.
28. Sanitizing for one context and using the value in another.

## 17. Database and migration sins

1. Never run raw SQL with interpolated user input.
2. Never run migrations against production without backup/rollback planning.
3. Never let an app connect as a database superuser.
4. Never use the same database credentials for app, migrations, analytics, and admin.
5. Never grant write permissions where read-only is sufficient.
6. Never skip transactions for multi-step consistency-sensitive operations.
7. Never ignore transaction rollback paths.
8. Never perform destructive migrations without checking row counts, locks, and downtime impact.
9. Never store secrets in plaintext database columns.
10. Never store passwords reversibly.
11. Never log full SQL queries with sensitive parameters.
12. Never expose database error messages to users.
13. Never trust ORM-level filters as tenant isolation unless tested.
14. Never implement soft delete without considering authorization and uniqueness implications.
15. Never use production data in local dev without approval and sanitization.
16. Never run agent-generated migrations without manual review.
17. Never let an agent “fix” a migration by dropping data.
18. Never allow schema drift between environments without detection.
19. Never leave migrations irreversible when they mutate important data.
20. Never skip indexes or query limits on user-controlled search endpoints.

## 17. Database and persistence behavior

1. String-built SQL.
2. Missing transactions around multi-step mutations.
3. Partial writes without recovery.
4. No uniqueness constraints for values that must be unique.
5. No foreign keys where referential integrity matters.
6. No optimistic/pessimistic locking where concurrent updates matter.
7. Race-prone “check then insert” logic.
8. Money or inventory changes outside transactions.
9. Background jobs that are not idempotent.
10. Migrations that delete or rewrite data without backups and review.
11. Destructive migrations generated by an agent and applied automatically.
12. Production migrations at app startup with broad credentials.
13. Unbounded queries in request paths.
14. User-controlled sort/filter fields without allowlists.
15. N+1 queries that enable denial of service.
16. No statement timeout.
17. No connection timeout.
18. No pool limits.
19. Shared SQLite connections across threads without serialization.
20. Using SQLite for concurrent write-heavy production workloads without understanding locking.
21. Storing secrets unencrypted in general-purpose tables.
22. No audit fields for sensitive records.
23. Soft-delete used as access control.
24. Backups never tested.
25. Restores never tested.
26. PII retained forever by default.
27. No migration rollback/forward plan.
28. No data-classification boundaries.

## 18. Async, concurrency, and resource exhaustion

1. Blocking network/file/subprocess calls inside an event loop.
2. Fire-and-forget `asyncio.create_task()` with no tracking, cancellation, or exception handling.
3. Ignoring task exceptions.
4. No cancellation path.
5. No timeout on locks, queues, HTTP calls, DB calls, or subprocesses.
6. Unbounded queues.
7. Unbounded thread pools.
8. Unbounded process pools.
9. Spawning threads/processes per request without limits.
10. No backpressure.
11. No rate limiting.
12. No request size limits.
13. No memory limits for parsing, decompression, dataframes, or ML inference.
14. No CPU limits for user-controlled regex, image processing, PDF parsing, or model inference.
15. Catastrophic backtracking regex on user input.
16. Shared mutable global state across threads.
17. Unsynchronized writes to files, SQLite databases, caches, shelves, or in-memory maps.
18. Multiprocessing across trust boundaries with pickle-based messages.
19. Forking after threads in unsafe runtime contexts.
20. Locks acquired in inconsistent order.
21. Locks not released in `finally`.
22. Distributed jobs with no idempotency key.
23. Cron jobs that overlap and corrupt state.
24. Retry storms.
25. Agent loops with no budget or kill switch.

## 18. Web framework sins: Flask, Django, FastAPI, etc.

1. Never run debug mode in production.
2. Never expose interactive debuggers.
3. Never leave default secret keys.
4. Never disable CSRF protection on browser forms without a specific safe reason.
5. Never use permissive CORS with credentials.
6. Never set `Access-Control-Allow-Origin: *` with sensitive APIs.
7. Never return raw exception messages to clients.
8. Never expose framework admin panels without strong protection.
9. Never trust reverse-proxy headers unless configured from trusted proxies.
10. Never accept file uploads without size/type/storage policy.
11. Never serve uploaded files from executable paths.
12. Never put uploaded files inside source directories.
13. Never render untrusted markdown/HTML without sanitization.
14. Never call `mark_safe` or equivalent on user content casually.
15. Never disable template escaping globally.
16. Never implement rate-limit-free login, reset, OTP, or invite flows.
17. Never omit security headers where relevant.
18. Never rely on frontend validation.
19. Never return sensitive fields from serializers by default.
20. Never expose internal model fields accidentally through `__dict__`, `model_to_dict`, or broad serializers.
21. Never use mass assignment for sensitive fields.
22. Never accept arbitrary filter/sort fields without allowlisting.
23. Never expose stack traces through API responses.
24. Never expose OpenAPI docs for private/admin APIs without access control.
25. Never assume localhost-only services are unreachable in container/cloud environments.

## 19. Packaging, project structure, and environment sins

1. Never develop production apps directly in global Python.
2. Never omit a reproducible environment setup.
3. Never omit `pyproject.toml`, lockfile, or equivalent dependency metadata for serious projects.
4. Never rely on undocumented manual setup steps.
5. Never require developers to guess environment variables.
6. Never let local, test, staging, and production config be indistinguishable.
7. Never make production the default environment.
8. Never default to destructive production endpoints.
9. Never let tests depend on developer-specific paths.
10. Never commit virtual environments.
11. Never commit build artifacts as source unless deliberately vendored.
12. Never commit `__pycache__`, `.pytest_cache`, coverage dumps, local DBs, or temp files.
13. Never publish packages with accidental secrets or internal files.
14. Never publish packages without checking included files.
15. Never run package build scripts from untrusted sources.
16. Never mix library import behavior with application startup side effects.
17. Never make package installation perform network calls beyond dependency resolution.
18. Never make imports require optional heavy services unless explicitly documented.
19. Never let generated files be edited manually without clear ownership.
20. Never leave dead code paths that nobody tests but production can reach.

## 19. Web framework mistakes

1. Flask `debug=True` in production.
2. Django `DEBUG=True` in production.
3. Weak or default `SECRET_KEY`.
4. Misconfigured `ALLOWED_HOSTS`.
5. CORS wildcard with credentials.
6. CSRF disabled globally.
7. Raw SQL in framework code with string interpolation.
8. `mark_safe`, `|safe`, or disabled autoescape on user content.
9. File uploads saved under static/web root.
10. Serving uploaded HTML, SVG, or scripts without strict controls.
11. No `Content-Disposition` for user-uploaded files where needed.
12. No MIME-sniffing protections where relevant.
13. Trusting `X-Forwarded-*` headers without trusted proxy configuration.
14. Trusting `Host` headers in password reset links.
15. Open redirects through `next`, `redirect`, or `return_to`.
16. No rate limits for login/password reset.
17. No pagination limits.
18. Returning different auth errors that enable account enumeration.
19. Background tasks that skip authz because “the API already checked.”
20. Websocket handlers without authz.
21. Admin panels reachable publicly.
22. API docs exposing sensitive schemas or test credentials.
23. Health checks exposing secrets, config, versions, or dependency internals.
24. Temporary routes left in production.
25. Debug toolbars, profilers, notebooks, or dashboards exposed.

## 2. Deserialization, parsing, and archive handling sins

1. Never unpickle untrusted data.
2. Never unpickle data that could have been tampered with.
3. Never unpickle data from uploads, email, object storage, cache, queue, user home directories, artifacts, or public internet sources.
4. Never treat “it came from our S3 bucket” as proof that pickled data is safe.
5. Never use `pickle`, `dill`, `shelve`, `jsonpickle`, `marshal`, `cloudpickle`, `joblib`, `pandas.read_pickle`, or ML model loaders on untrusted artifacts.
6. Never accept a pickled model file from a ticket, Slack message, GitHub issue, Hugging Face-style model repo, or random download and load it on a real machine.
7. Never deserialize YAML with unsafe loaders.
8. Never allow object construction from untrusted serialized formats.
9. Never parse XML from untrusted sources without hardened parser settings and size limits.
10. Never parse attacker-controlled XML with entity expansion enabled.
11. Never process compressed XML or archive payloads without decompression limits.
12. Never extract tar files from untrusted sources without filters and path validation.
13. Never extract zip/tar members that can escape the target directory.
14. Never trust archive filenames.
15. Never extract archives as root.
16. Never extract archives into application source directories.
17. Never process archives without member-count, file-size, total-size, symlink, hardlink, and path traversal checks.
18. Never accept archive uploads without quotas.
19. Never parse JSON as if it were harmless when an attacker can send enormous, deeply nested, or schema-invalid payloads.
20. Never skip schema validation for external JSON, YAML, TOML, XML, CSV, or protobuf messages.
21. Never parse CSV from users and then open it in Excel without considering formula injection.
22. Never trust MIME type, filename extension, or `Content-Type` header as validation.
23. Never use serialization formats that can execute code when a data-only format would work.
24. Never store security-sensitive state in opaque serialized blobs that nobody can inspect.
25. Never allow an agent to choose a serializer/deserializer for convenience without a security review.

## 2. Secrets and credentials

1. Commit API keys, passwords, tokens, private keys, OAuth secrets, session secrets, database URLs, webhook secrets, signing keys, `.env` files, or cloud credentials.
2. Put secrets in frontend code, mobile bundles, notebooks, screenshots, logs, tracebacks, test fixtures, README examples, GitHub issues, GitLab merge requests, Slack snippets, or agent prompts.
3. Use real production secrets in local development.
4. Give AI agents broad production credentials.
5. Store long-lived production credentials on developer laptops when short-lived scoped credentials are available.
6. Use one shared service account for everything.
7. Use personal access tokens where deploy keys, OIDC, workload identity, or narrowly scoped tokens would work.
8. Give CI tokens write/admin permissions by default.
9. Print environment variables in CI logs.
10. Echo secrets during debugging.
11. Include secrets in URLs; URLs leak through logs, proxies, browser history, referrers, and metrics.
12. Store passwords in plaintext, reversible encryption, MD5, SHA1, SHA256, or “double-hashed” homegrown schemes.
13. Reuse Flask/Django `SECRET_KEY` values across environments.
14. Use default demo secrets from tutorials.
15. Continue using a secret after it has been committed, even if the commit was deleted.
16. Treat private repos as safe places for secrets.
17. Store reset tokens or invite tokens in plaintext.
18. Generate tokens with `random`, timestamps, UUIDv1, incremental IDs, or short codes without rate limits.
19. Compare secrets with normal `==` when timing attacks matter.
20. Let MCP/server config files contain broad credentials that agents can read and exfiltrate.

## 2. Unsafe deserialization and parsing

1. `pickle.load()` or `pickle.loads()` on untrusted or tamperable data.
2. `dill`, `cloudpickle`, `joblib`, `shelve`, or `marshal` on untrusted or tamperable data.
3. `multiprocessing.Connection.recv()` across a trust boundary, because it uses pickle-like semantics.
4. Loading ML/model artifacts that rely on pickle semantics from unknown origins.
5. `yaml.load()` without a safe loader.
6. YAML object constructors from untrusted files.
7. Treating JSON as “validated” just because it parsed.
8. Parsing massive JSON/YAML/XML without size, depth, field, and timeout limits.
9. Parsing untrusted XML with unsafe defaults or old vulnerable XML parsers.
10. Accepting XML external entities, entity expansion, decompression bombs, or giant-token attacks.
11. `ast.literal_eval()` as a generic parser for unbounded external input.
12. `eval(repr(x))` as storage, caching, IPC, or config.
13. Using `.py` files as user-editable configuration.
14. Loading user-controlled Python packages as plugins.
15. Loading browser cookies, session files, cache files, or SQLite files from users and trusting their content.
16. Deserializing data from Redis, queues, S3, GitHub artifacts, CI caches, or local temp files without authentication/integrity checks.
17. Trusting file extensions instead of validating content.
18. Treating “internal service” data as trusted without authentication and schema validation.

## 20. Data science, notebooks, and ML-specific Python

1. Executing untrusted notebooks.
2. Committing notebooks with outputs containing secrets, tokens, internal data, screenshots, or PII.
3. Loading unknown pickle-based model checkpoints.
4. Running arbitrary code from model repositories without isolation.
5. Installing whatever a model card or README says without verification.
6. Trusting CSV/Excel files that contain formulas.
7. Exporting CSVs that allow formula injection.
8. Using production PII in notebooks.
9. Sending proprietary datasets to external LLMs or hosted notebooks without approval.
10. No dataset provenance.
11. No model provenance.
12. No separation between training, validation, and test data.
13. Data leakage between train and evaluation.
14. Hidden random seeds that make results unreproducible.
15. Treating benchmark output as proof of safety.
16. No resource limits for pandas, XML, CSV, parquet, image, PDF, or model input.
17. Notebook code promoted to production without refactoring, tests, config, logging, and error handling.
18. Secrets in notebook metadata.
19. Long-running notebook kernels with broad cloud credentials.
20. Agent-edited notebooks executed automatically.

## 20. ML, data science, notebook, and LLM-app sins

1. Never load untrusted model artifacts that rely on pickle-like deserialization.
2. Never run downloaded model code in your main environment.
3. Never enable remote/custom model code execution without sandboxing and review.
4. Never treat notebooks as safe just because they are `.ipynb`.
5. Never commit notebooks with outputs containing secrets or private data.
6. Never commit notebooks with hidden state required for correctness.
7. Never train or evaluate on data you are not allowed to use.
8. Never leak training data into prompts, logs, or demos.
9. Never use test data for tuning and then report it as unbiased evaluation.
10. Never accept an LLM answer as ground truth for security, legal, financial, or medical logic.
11. Never let LLM output directly trigger tools without validation.
12. Never let prompts override system/security policy.
13. Never put API keys in prompt templates.
14. Never put database credentials in agent memory.
15. Never give an agent unrestricted access to production databases.
16. Never give an agent unrestricted shell access on production systems.
17. Never let an agent browse internal networks without egress controls.
18. Never let retrieved documents become instructions without prompt-injection defenses.
19. Never let LLM-generated SQL run automatically.
20. Never let LLM-generated Python run automatically.
21. Never let LLM-generated regexes run on unbounded input.
22. Never let LLM-generated file paths control reads/writes/deletes.
23. Never treat vector-store contents as trusted.
24. Never log full conversations if they may contain secrets or private data.
25. Never skip abuse testing for LLM tool-use workflows.

## 21. Privacy, compliance, and data handling

1. Production PII in local dev.
2. Production PII in tests.
3. Production PII in prompts.
4. Production PII in logs.
5. Production PII in screenshots.
6. Production PII in notebooks.
7. Production PII in support dumps without minimization.
8. Sending confidential code or data to external services without approval.
9. Training, fine-tuning, or retrieval over user/private data without permission and retention policy.
10. No data-retention policy.
11. No deletion path.
12. No export controls by tenant/user.
13. No masking in lower environments.
14. No access audit for sensitive data.
15. No purpose limitation: collecting data “just in case.”
16. Overbroad analytics events.
17. Secrets or PII in error monitoring.
18. User data in CI artifacts.
19. User data in package artifacts.
20. User data in public bug reports.
21. Copying code from GitHub/Reddit/Stack Overflow without checking license and attribution needs.
22. Scraping data in violation of platform terms.
23. Using API data for prohibited ML training or surveillance use cases.
24. Mixing tenants in caches, embeddings, vector stores, logs, or search indexes.
25. Treating embeddings as non-sensitive by default.

## 21. Production operations sins

1. Never deploy with debug settings enabled.
2. Never deploy with development credentials.
3. Never deploy with default credentials.
4. Never deploy without logs.
5. Never deploy without metrics for critical services.
6. Never deploy without health checks.
7. Never deploy without rollback strategy.
8. Never deploy without config separation between environments.
9. Never deploy without least-privilege runtime permissions.
10. Never run app containers as root unless unavoidable and justified.
11. Never mount the Docker socket into app containers casually.
12. Never mount host directories broadly into containers.
13. Never give production workloads broad cloud IAM permissions.
14. Never expose internal services publicly by accident.
15. Never ship code that requires manual SSH fixes after deploy.
16. Never mutate production data from ad hoc scripts without review and backup.
17. Never run one-off scripts without logging what they changed.
18. Never run one-off scripts without dry-run for destructive operations.
19. Never leave cron jobs undocumented.
20. Never leave background workers without monitoring.

## 22. Deployment, runtime, and infrastructure

1. Running as root when not required.
2. Writable application code directories in production.
3. World-readable secrets.
4. World-writable app directories.
5. No CPU/memory limits.
6. No request limits.
7. No upload limits.
8. No worker timeouts.
9. No graceful shutdown for jobs that mutate data.
10. No health checks.
11. No readiness checks.
12. No rollback plan.
13. No feature flag for risky changes.
14. No backup before destructive migrations.
15. No tested restore process.
16. Dev/staging/prod configs mixed.
17. Production connected to developer machines.
18. Local `.env` accidentally overriding production config.
19. Unsafe `PYTHONPATH`.
20. Running privileged scripts without Python isolated/safe-path modes where appropriate.
21. Relying on current directory imports in privileged scripts.
22. Cron jobs with broad credentials and no logs.
23. Containers with package managers, shells, and compilers when unnecessary.
24. Containers built from mutable tags.
25. Containers with secrets baked into layers.
26. CI artifacts containing secrets.
27. Deployment logs containing secrets.
28. No dependency/container scanning.
29. No SAST in CI for code paths that agents edit.
30. No secret scanning in CI.
31. No separation of deploy credentials from build credentials.
32. Production deploys from unreviewed branches.
33. Agents allowed to deploy directly.
34. No human approval for destructive infrastructure changes.
35. No incident rollback instructions.

## 22. “Convenience” excuses that are not valid

1. “It is only internal.”
2. “It is behind VPN.”
3. “It is just a script.”
4. “It is temporary.”
5. “We will fix it later.”
6. “The repo is private.”
7. “The endpoint is obscure.”
8. “The user input is probably safe.”
9. “Only admins can access it.”
10. “The AI wrote it.”
11. “The scanner is noisy.”
12. “The test was flaky.”
13. “The dependency is popular.”
14. “The package has lots of stars.”
15. “The secret is base64 encoded.”
16. “The token is read-only.”
17. “The command is escaped.”
18. “The pickle comes from our bucket.”
19. “The webhook comes from a trusted service.”
20. “The frontend already validates it.”
21. “The error never happens.”
22. “The data is anonymized.”
23. “The model probably knows best.”
24. “It passed once locally.”
25. “No one will notice.”

## 24. Socially common Python sins that are still not excusable

1. “It’s internal.”
2. “Only admins can reach it.”
3. “The agent wrote it.”
4. “The LLM said it’s safe.”
5. “It passed once locally.”
6. “We’ll clean it up later.”
7. “It’s just a script.”
8. “It’s just a notebook.”
9. “It’s just a migration.”
10. “It’s just a temporary endpoint.”
11. “The repo is private.”
12. “The token is read-only.”
13. “The data is not sensitive.”
14. “The shell command is simple.”
15. “The pickle file comes from S3.”
16. “The URL comes from our database.”
17. “The filename comes from the user but we sanitize it later.”
18. “The dependency is popular.”
19. “The warning is noisy.”
20. “The failing test is flaky.”
21. “The scanner is wrong.”
22. “The secret was deleted from the latest commit.”
23. “The agent needs broad access to be useful.”
24. “The debug route is unlinked.”
25. “We trust our users.”

## 3. Secrets and credential-handling sins

1. Never hardcode API keys.
2. Never hardcode passwords.
3. Never hardcode database URLs with credentials.
4. Never hardcode private keys.
5. Never hardcode JWT signing secrets.
6. Never hardcode OAuth client secrets.
7. Never hardcode cloud access keys.
8. Never commit `.env` files.
9. Never commit `.pem`, `.key`, service-account JSON, kubeconfigs, SSH keys, or Terraform state containing secrets.
10. Never print secrets.
11. Never log secrets.
12. Never include secrets in exception messages.
13. Never include secrets in telemetry, analytics, crash reports, or traces.
14. Never send secrets to an LLM prompt.
15. Never store production secrets in notebooks.
16. Never store secrets in test fixtures committed to the repo.
17. Never store secrets in Docker images.
18. Never store secrets in frontend code.
19. Never expose secrets through `/debug`, `/health`, `/metrics`, OpenAPI docs, admin pages, or stack traces.
20. Never use base64 encoding as “encryption.”
21. Never use reversible “obfuscation” as secret protection.
22. Never reuse the same secret across dev, staging, and production.
23. Never share a personal access token between developers, CI, and production.
24. Never use long-lived credentials where short-lived/OIDC credentials are available.
25. Never keep a secret after it has been exposed; revoke and rotate it.
26. Never assume deleting a secret from the latest commit removes it from Git history.
27. Never put credentials in command-line arguments where process listings can expose them.
28. Never put secrets in URLs.
29. Never put secrets in query parameters.
30. Never store secrets in world-readable files.
31. Never let an agent create, print, copy, rotate, or exfiltrate secrets without strict tool boundaries.
32. Never accept “private repo” as sufficient secret protection.

## 3. Shell, subprocess, and OS command abuse

1. `subprocess.run(..., shell=True)` with any variable input.
2. `os.system()`, `os.popen()`, or shell-string subprocess calls with dynamic content.
3. `bash -c`, `sh -c`, `cmd.exe /c`, or PowerShell commands built from strings.
4. Concatenating command strings instead of passing an argument list.
5. Using shell quoting as the primary security boundary for complex commands.
6. Running LLM-generated shell commands automatically.
7. Passing untrusted filenames as command options without `--` and strict validation.
8. Allowing option injection, for example a filename beginning with `-`.
9. Executing downloaded scripts, including “curl pipe shell” patterns.
10. Calling package managers, Git hooks, build scripts, or test runners from untrusted repos without isolation.
11. Running subprocesses without timeouts in request handlers, tools, CI agents, or services.
12. Running subprocesses with inherited full environment when secrets are present.
13. Invoking relative executables in privileged contexts, enabling PATH hijacking.
14. Letting agent-written scripts run outside a sandbox.
15. Using `sudo`, root, admin shells, or broad OS permissions to “make it work.”
16. `chmod 777` as a fix.
17. Running commands in directories controlled by an attacker.
18. Treating “the filename came from our app” as safe without validating the full path and arguments.

## 4. Cryptography, tokens, passwords, and TLS sins

1. Never use `random` for passwords, tokens, reset links, session IDs, API keys, salts, nonces, or cryptographic material.
2. Never use predictable token generation.
3. Never seed a PRNG and call the output “secure.”
4. Never use timestamps, user IDs, emails, UUIDv1, incremental IDs, or hashes of public data as secrets.
5. Never use MD5 for security.
6. Never use SHA-1 for security.
7. Never use weak hashes for password storage.
8. Never store plaintext passwords.
9. Never store reversibly encrypted passwords when hashing is required.
10. Never roll your own password hashing.
11. Never omit salts from password hashing.
12. Never use fast general-purpose hashes for passwords.
13. Never roll your own encryption scheme.
14. Never roll your own signing scheme.
15. Never roll your own JWT implementation.
16. Never accept JWTs without verifying signature, issuer, audience, expiry, and algorithm constraints.
17. Never allow `alg=none` or algorithm confusion in token handling.
18. Never use the same key for unrelated purposes.
19. Never disable TLS certificate verification with `verify=False` in production code.
20. Never use `_create_unverified_context()` to “fix” TLS failures.
21. Never set `check_hostname = False` for real HTTPS connections.
22. Never send credentials over HTTP.
23. Never accept self-signed certificates silently.
24. Never compare secrets with normal equality where timing attacks matter.
25. Never log cryptographic keys or raw tokens.
26. Never invent “encryption” with XOR, base64, ROT13, Fernet misuse, or homegrown transforms.
27. Never store encryption keys beside encrypted data.
28. Never use one global signing secret forever.
29. Never skip key rotation planning.
30. Never use production crypto code that lacks tests against known-good vectors.

## 4. Secrets and credentials

1. Hardcoded API keys, tokens, passwords, private keys, OAuth secrets, webhook secrets, database URLs, or signing keys.
2. Committed `.env` files.
3. Committed SSH keys, AWS/GCP/Azure credentials, service-account JSON, TLS private keys, or kubeconfigs.
4. “Fake” test secrets that are actually valid somewhere.
5. Secrets in notebooks, test fixtures, README files, issue templates, Dockerfiles, shell history, or copied stack traces.
6. Secrets in command-line arguments where process lists can expose them.
7. Secrets in URLs, query strings, Git remotes, redirects, or analytics events.
8. Logging authorization headers, cookies, JWTs, session IDs, password reset tokens, or full environment variables.
9. Printing exception objects that include secrets.
10. Sending secrets to an LLM prompt or agent memory.
11. Giving agents access to the user’s full shell environment.
12. Sharing one broad secret across development, staging, production, CI, and local agents.
13. Long-lived tokens with no rotation plan.
14. Production credentials in local development.
15. Production credentials in CI jobs that run untrusted PRs.
16. Secrets available to forks.
17. Secrets available to all repository collaborators by default.
18. Secrets stored in plaintext databases or config stores.
19. Passwords stored recoverably.
20. Missing secret scanning, or ignoring secret-scanning alerts.
21. Failing to rotate after a leak.
22. Trying to “delete the commit” without revoking the leaked secret.
23. Passing secrets through untrusted MCP servers, browser automation, plugins, or shell tools.
24. Using broad cloud credentials where a narrow per-agent credential would work.
25. Treating environment variables as automatically safe; they are only safer than source code, not safe from all tools.

## 4. Web application behavior

1. Ship Flask/Django/FastAPI apps with debug mode enabled.
2. Expose interactive debuggers, stack traces, admin panels, OpenAPI docs, metrics, Celery flower, Jupyter, MLflow, or internal dashboards publicly without authentication.
3. Bind dev servers to `0.0.0.0` on real networks.
4. Use `python -m http.server` or `http.server` as a production server.
5. Disable CSRF protection for cookie-authenticated state-changing requests.
6. Use `CORS_ALLOW_ALL_ORIGINS=True` or `allow_origins=["*"]` with credentials.
7. Reflect arbitrary origins into CORS headers.
8. Trust `X-Forwarded-For`, `X-Forwarded-Host`, or `X-Forwarded-Proto` without a trusted proxy configuration.
9. Leave session cookies without `Secure`, `HttpOnly`, and an appropriate `SameSite`.
10. Set cookie domains broadly, such as `.example.com`, without a real subdomain trust model.
11. Put session IDs or tokens in query strings.
12. Store sensitive tokens in browser localStorage for apps exposed to XSS risk.
13. Trust client-supplied `user_id`, `role`, `tenant_id`, `is_admin`, price, discount, or permission fields.
14. Enforce authorization only in the frontend.
15. Check authentication but not object-level authorization.
16. Use sequential IDs without authorization checks and call it fine because IDs are “hard to guess.”
17. Skip rate limits on login, password reset, invite, OTP, scraping, exports, webhooks, expensive searches, and file uploads.
18. Accept webhooks without signature verification and replay windows.
19. Accept file uploads into web-served executable paths.
20. Serve uploaded content from the same origin as the application if it can execute scripts.
21. Fail open on auth provider errors.
22. Use production data in staging without equivalent controls.

## 5. Authentication, authorization, and session sins

1. Never trust the client to tell you who the user is.
2. Never trust a client-provided `user_id`, `role`, `is_admin`, `tenant_id`, `plan`, `price`, or `subscription_status`.
3. Never authorize based only on hidden form fields.
4. Never authorize based only on frontend route guards.
5. Never check authentication but skip authorization.
6. Never rely on “security by obscurity” URLs.
7. Never expose object IDs without ownership checks.
8. Never let users access records by changing an ID in the URL.
9. Never skip tenant isolation checks.
10. Never cache authorization decisions without invalidation.
11. Never use weak session cookies.
12. Never store raw session data in client-side cookies unless properly signed/encrypted and size-limited.
13. Never omit `HttpOnly`, `Secure`, and appropriate `SameSite` cookie settings for sensitive cookies.
14. Never put access tokens in local storage for high-risk apps without a deliberate threat model.
15. Never allow unlimited login attempts.
16. Never leak whether an email exists during login/reset flows unless explicitly accepted.
17. Never implement password reset without single-use, expiring, unpredictable tokens.
18. Never allow password reset tokens to remain valid after use.
19. Never skip CSRF protection on state-changing browser endpoints.
20. Never disable framework auth protections to get tests passing.
21. Never use debug/admin endpoints without strong auth.
22. Never expose internal admin APIs to the public internet.
23. Never implement webhook handlers without signature verification.
24. Never trust webhook payloads because they “come from Stripe/GitHub/Slack/etc.”
25. Never treat authentication as authorization.
26. Never make admin checks in templates only.
27. Never allow mass assignment of sensitive model fields.
28. Never ship default admin credentials.
29. Never keep dormant accounts, tokens, or service credentials active forever.
30. Never let an agent edit access-control code without dedicated tests for privilege escalation.

## 5. Authentication, authorization, and tenant isolation

1. Unauthenticated admin endpoints.
2. Debug endpoints reachable outside localhost.
3. “Internal only” endpoints with no authentication.
4. Hardcoded admin credentials.
5. Default credentials left enabled.
6. Backdoor passwords, magic headers, or emergency bypasses.
7. Client-side authorization only.
8. Role checks in the UI but not the server.
9. Missing object-level authorization.
10. Missing tenant isolation in queries.
11. User IDs, org IDs, file IDs, invoice IDs, or project IDs accepted without ownership checks.
12. Service accounts with admin permissions when narrow permissions would work.
13. Agents using human admin credentials.
14. Production agents able to mutate infrastructure without explicit authorization.
15. JWTs accepted without validating signature, expiration, issuer, audience, and algorithm.
16. Accepting `alg=none` or trusting token headers to choose algorithms.
17. Long-lived bearer tokens with no revocation.
18. Password reset tokens that are guessable, reusable, or never expire.
19. Session cookies without `HttpOnly`, `Secure`, and appropriate `SameSite`.
20. CSRF disabled on browser-based state-changing actions.
21. CORS `*` with credentials.
22. Rate-limit-free login, signup, password reset, token minting, or expensive endpoints.
23. State-changing GET endpoints.
24. Relying on obscurity, odd URLs, robots.txt, or “not linked anywhere.”
25. Assuming one user cannot see another user’s data because the frontend hides it.

## 5. Input validation, path traversal, and file system safety

1. Use raw user input as a filename, directory, glob, URL, import path, module path, class name, shell arg, SQL identifier, queue name, S3 key, Redis key, or template name.
2. Validate paths with simple string checks like “does not contain `../`.”
3. Assume `os.path.join(base, user_path)` prevents traversal.
4. Follow symlinks in writable directories without checking.
5. Use `tempfile.mktemp()`.
6. Write temp files in shared directories with predictable names.
7. Use `chmod 777`, world-writable files, or broad umasks to “fix permissions.”
8. Delete paths supplied by users, configs, or agents with `shutil.rmtree()` without canonical containment checks.
9. Let archive extraction, upload handling, or exports overwrite existing files.
10. Use relative paths in cron, CI, privileged services, or long-running daemons.
11. Store sensitive data under `/tmp` without permissions and cleanup discipline.
12. Trust filenames from `Content-Disposition`.
13. Use user-controlled glob patterns.
14. Generate download responses from raw paths instead of opaque file IDs.
15. Mix tenant files in the same namespace without tenant isolation.

## 6. Injection and untrusted-input sins

1. Never concatenate SQL strings with user input.
2. Never use f-strings to build SQL from user input.
3. Never use `%` formatting or `.format()` to build SQL from user input.
4. Never quote SQL manually when parameterized queries are available.
5. Never trust ORM escape hatches like `.raw()`, `.extra()`, or text queries without parameterization.
6. Never build NoSQL queries directly from request JSON without allowlisting fields/operators.
7. Never pass user input into shell commands.
8. Never pass user input into LDAP filters without escaping/parameterization.
9. Never pass user input into XPath/XQuery unsafely.
10. Never render user input as HTML without escaping.
11. Never mark user content as safe unless it has been sanitized with a trusted sanitizer and policy.
12. Never disable template autoescaping globally.
13. Never let users control template source.
14. Never let users control redirect targets without allowlisting.
15. Never let users control file paths without canonicalization and base-directory enforcement.
16. Never let users control URLs fetched by the server without SSRF defenses.
17. Never fetch arbitrary internal URLs on behalf of a user.
18. Never allow requests to cloud metadata services from user-controlled URLs.
19. Never use regular expressions from users without bounding complexity.
20. Never run catastrophic-backtracking regexes on untrusted input.
21. Never accept unbounded request bodies.
22. Never accept unbounded uploads.
23. Never accept unbounded JSON nesting.
24. Never accept unbounded pagination limits.
25. Never accept unbounded search queries.
26. Never trust request headers like `X-Forwarded-For` unless set by trusted infrastructure.
27. Never trust `Host` headers without validation.
28. Never trust `Content-Type` for security decisions.
29. Never trust filenames from uploads.
30. Never trust image metadata.
31. Never trust EXIF.
32. Never trust CSV/Excel files from users.
33. Never trust email addresses, domains, or URLs without validation.
34. Never treat “internal API” as safe from malicious input.
35. Never let LLM output flow into SQL, shell, file paths, network calls, or code execution without validation.

## 6. Injection bugs

1. SQL built with f-strings.
2. SQL built with `%`, `.format()`, `+`, or manual escaping.
3. ORM raw SQL with string interpolation.
4. User-controlled table, column, sort, or direction names without allowlists.
5. NoSQL query dicts built directly from request bodies.
6. Mongo-style operator injection through `$ne`, `$gt`, `$where`, or equivalent.
7. LDAP queries built from strings.
8. XPath queries built from strings.
9. GraphQL queries built by concatenating user input.
10. Jinja2 templates created from user strings.
11. Django `mark_safe()` on user content.
12. Jinja `|safe` on user content.
13. Disabling autoescape globally.
14. Header injection through untrusted header values.
15. CRLF injection in logs, emails, redirects, or HTTP responses.
16. Path traversal through `../`, absolute paths, symlinks, encoded separators, or Windows drive paths.
17. SSRF through user-controlled URLs.
18. Calling `requests.get(user_url)` without URL scheme, host, IP range, redirect, and DNS controls.
19. Template injection in alert rules, emails, reports, or admin dashboards.
20. Shell injection disguised as “just a filename.”
21. Python format-string abuse when untrusted strings become templates.
22. HTML rendered from markdown without sanitization.
23. CSV/Excel formula injection in exported spreadsheets.
24. Open redirects via unchecked `next` or `return_to`.
25. Log injection that forges audit records.

## 6. Supply-chain, dependencies, packaging, and CI/CD

1. Use unpinned production dependencies.
2. Use `latest` tags for production images or tools.
3. Install dependencies from random Git branches, gists, pastebins, URLs, or unreviewed forks.
4. Use `--extra-index-url` with private package names without dependency-confusion controls.
5. Use `--trusted-host` or disable TLS to make `pip` work.
6. Run `curl | python`, `curl | bash`, or copied installer scripts in CI without pinning and verification.
7. Run `sudo pip install` into system Python for app deployments.
8. Let CI install dependencies without a lockfile or constraints.
9. Ignore transitive dependencies.
10. Ignore CVEs because the vulnerable package is “only a dev dependency.”
11. Merge dependency bumps without tests.
12. Disable Dependabot/GitLab dependency scanning/security alerts because they are noisy.
13. Use abandoned packages for authentication, crypto, parsing, cloud access, or web security.
14. Vendor code from StackOverflow, GitHub, Reddit, or LLM output without license and security review.
15. Publish packages with broad PyPI tokens.
16. Store PyPI/npm/Docker/cloud publishing tokens in repo secrets available to every branch or PR.
17. Let pull requests from forks access deployment secrets.
18. Use mutable build artifacts without provenance.
19. Build releases on developer laptops.
20. Skip SBOMs or dependency inventories in serious systems.
21. Ignore typosquatting and package-name confusion.
22. Disable build isolation without understanding why.
23. Let setup/build scripts from untrusted packages run in privileged environments.
24. Leave GitHub Actions or GitLab CI permissions broad by default.
25. Use self-hosted runners with production network access for untrusted PRs.

## 7. Dependency and supply-chain sins

1. Never `pip install` a package suggested by an agent without checking it exists, is reputable, and is the intended package.
2. Never ignore typo-squatting risk.
3. Never ignore dependency-confusion risk.
4. Never install from random GitHub repositories in production builds.
5. Never install from mutable branches like `main` for production.
6. Never depend on `latest`.
7. Never leave production dependencies unpinned.
8. Never pin only top-level dependencies while allowing transitive dependencies to float unpredictably.
9. Never ship without a lockfile or reproducible install process.
10. Never ignore hashes for high-assurance deployments.
11. Never use `setup.py install`, `easy_install`, or legacy install paths in modern production workflows.
12. Never run package install commands as root in a broad environment when isolation is possible.
13. Never install into system Python for app deployments.
14. Never let CI install dependencies from uncontrolled indexes.
15. Never mix public and private package indexes without dependency-confusion defenses.
16. Never ignore known vulnerabilities in dependencies.
17. Never treat `pip-audit` or SCA as optional for production services.
18. Never treat SCA as proof that a package is not malicious.
19. Never vendor code without tracking origin, license, version, and update path.
20. Never copy code from Stack Overflow, GitHub, Reddit, or an LLM into production without license and security review.
21. Never use abandoned packages for security-sensitive functionality.
22. Never keep using unsupported Python versions.
23. Never keep using unsupported framework versions.
24. Never use broad version ranges for critical security libraries without lockfile control.
25. Never suppress dependency vulnerability reports without a documented risk acceptance.
26. Never allow an agent to downgrade dependencies to make code work.
27. Never allow an agent to add giant frameworks for tiny problems without review.
28. Never add a dependency for trivial standard-library functionality unless there is a clear reason.
29. Never skip license review for generated or copied code.
30. Never build production containers with unverified, unpinned base images.

## 7. Error handling, logging, and observability

1. Use bare `except:`.
2. Catch `BaseException` except at a process boundary where you immediately re-raise termination signals.
3. Catch `Exception` and silently continue.
4. Write `except Exception: pass`.
5. Hide failures under “best effort” unless the business logic explicitly allows loss.
6. Return success after partial failure without recording what failed.
7. Use `assert` for runtime validation, authorization, authentication, data integrity, or security checks.
8. Put `return`, `break`, or `continue` in `finally` where it can swallow exceptions.
9. Log secrets, auth headers, cookies, reset links, full DB URLs, OAuth codes, private keys, or raw environment dumps.
10. Log raw user input into structured logs without neutralizing newlines/control characters where log injection matters.
11. Show internal exception messages to end users.
12. Swallow task exceptions in background jobs.
13. Create fire-and-forget async tasks with no supervision.
14. Ignore failed audit-log writes for security-sensitive events.
15. Use `print()` debugging in services instead of structured logging.
16. Disable warnings globally.
17. Set logging to DEBUG in production where it exposes data.
18. Treat absence of logs as proof that nothing happened.

## 7. Filesystem, paths, archives, and temporary files

1. Joining a base directory and user input without normalizing and checking containment.
2. Allowing absolute paths.
3. Allowing `..` traversal.
4. Ignoring symlinks.
5. Writing uploads into executable or web-served directories.
6. Deleting paths derived from user input.
7. `shutil.rmtree(user_path)`.
8. Globs derived from user input for delete, move, copy, or chmod.
9. Overwriting files outside a dedicated workspace.
10. Letting agents write outside their workspace.
11. Letting agents write dotfiles such as `.bashrc`, `.zshrc`, `.gitconfig`, `.curlrc`, SSH config, shell profile files, editor config, or agent config.
12. Letting agents modify Git hooks.
13. Letting agents modify MCP, Cursor, Claude, or other agent/tool configuration.
14. Trusting archive member paths during extraction.
15. Extracting tar files from untrusted sources.
16. Extracting zip files without path traversal and size checks.
17. Accepting archive symlinks, hardlinks, device files, or absolute paths.
18. Decompressing untrusted archives without quotas.
19. Ignoring zip bombs, tar bombs, and decompression bombs.
20. Using `tempfile.mktemp()`.
21. Creating predictable temp filenames.
22. Writing secrets to temp files without cleanup and permissions.
23. Setting world-writable permissions as a fix.
24. Creating files with unsafe default permissions.
25. Assuming `/tmp` is private.
26. Storing unencrypted sensitive exports on local disk.
27. Relying on cleanup in `__del__`.
28. Non-atomic writes for security-critical or money-critical state.
29. File writes that can be interrupted and leave corrupted state.
30. TOCTOU checks: “check path, then later open path” where an attacker can swap it.

## 8. Git, GitHub, GitLab, and CI/CD sins

1. Never commit secrets.
2. Never rely on `.gitignore` after a secret has already been committed.
3. Never leave secret scanning disabled.
4. Never leave dependency scanning disabled for production repos.
5. Never leave SAST disabled for security-sensitive repos.
6. Never make CI secrets available to untrusted fork pull requests.
7. Never use `pull_request_target` carelessly with untrusted code.
8. Never run untrusted PR code with write tokens.
9. Never give `GITHUB_TOKEN` or GitLab job tokens broad permissions by default.
10. Never use unpinned third-party GitHub Actions in sensitive workflows.
11. Never pin Actions only to mutable tags when integrity matters.
12. Never run random third-party CI actions without reviewing their source and permissions.
13. Never run `curl | bash` in CI.
14. Never echo environment variables in CI logs.
15. Never store CI artifacts containing secrets, `.env`, coverage dumps with tokens, or raw request fixtures.
16. Never run tests against production databases.
17. Never run migrations against production from unreviewed CI jobs.
18. Never deploy from unprotected branches.
19. Never deploy without required checks.
20. Never bypass branch protection because a change is “urgent” without incident documentation.
21. Never let an agent edit workflow files without review from someone who understands CI security.
22. Never let CI jobs write to package registries, cloud infrastructure, or production unless scoped to the job’s purpose.
23. Never use long-lived cloud keys in CI if OIDC/short-lived credentials are available.
24. Never leave CI variables unmasked, unprotected, or broadly available.
25. Never accept a pipeline that can be modified by the same untrusted code it executes.
26. Never cache dependency directories without considering cache poisoning.
27. Never build release artifacts from dirty working trees.
28. Never deploy artifacts that cannot be traced back to a commit, lockfile, and build.
29. Never skip code review on generated diffs because “the bot wrote it.”
30. Never leave repository permissions broader than needed.

## 8. Networking, HTTP, TLS, and API clients

1. `verify=False` in `requests`, `httpx`, `urllib3`, or equivalent.
2. Suppressing TLS warnings instead of fixing TLS.
3. Plain HTTP for credentials, tokens, sessions, or private data.
4. No timeout on outbound HTTP calls.
5. Infinite retries.
6. Retrying non-idempotent actions without idempotency keys.
7. Blindly following redirects with sensitive headers.
8. Sending authorization headers to redirected hosts.
9. User-controlled webhooks with no SSRF protection.
10. Allowing requests to link-local, loopback, private, metadata, or internal IP ranges unless explicitly intended and authorized.
11. Letting agents have unrestricted outbound network access.
12. Letting agents post to pastebins, gists, arbitrary APIs, or unknown MCP servers.
13. Binding development servers or debug tools to `0.0.0.0`.
14. Using `python -m http.server` or `http.server` as production infrastructure.
15. Publicly exposing stack traces.
16. Publicly exposing profiler, metrics, docs, admin, Celery Flower, Jupyter, Streamlit, Gradio, or debug dashboards without auth.
17. No request body limits.
18. No upload size limits.
19. No rate limits for expensive endpoints.
20. No backoff for third-party APIs.
21. Ignoring platform API rules.
22. Scraping, browser automation, or training/surveillance uses that violate a platform’s developer terms.
23. Storing third-party API tokens in code.
24. Using personal tokens for shared services.
25. Treating “works locally” API behavior as permission to automate at scale.

## 8. Python-specific maintainability footguns

1. Use mutable default arguments like `def f(x=[]):` in production code.
2. Use global mutable state for request, user, tenant, security, or transaction context.
3. Use module-level singletons for things that need lifecycle, config, auth context, or test isolation.
4. Use wildcard imports except the narrow public-API re-export case.
5. Shadow standard library or dependency names with files like `json.py`, `typing.py`, `requests.py`, `jwt.py`, or `email.py`.
6. Name variables `list`, `dict`, `id`, `file`, `type`, `input`, or `open` in ways that break readability/tooling.
7. Use `is` for equality checks on strings, ints, or other values.
8. Depend on CPython implementation details where portability matters.
9. Mutate function arguments unexpectedly.
10. Return multiple inconsistent types from one function because “Python allows it.”
11. Use `None`, `False`, empty string, `0`, and empty list interchangeably.
12. Use magic strings for states, roles, permissions, and event types when enums/literals would prevent mistakes.
13. Hide business logic in decorators, metaclasses, monkeypatches, import hooks, or dynamic attributes.
14. Monkeypatch builtins, standard library modules, or third-party library globals in app code.
15. Perform network calls, database migrations, file writes, secret reads, or subprocess calls at import time.
16. Let imports depend on current working directory.
17. Modify `sys.path` dynamically to make imports work.
18. Leave circular imports that only work by accident.
19. Use relative imports that break under tests or packaging.
20. Depend on hidden environment variables with no schema.
21. Use broad `Any` everywhere and pretend the code is typed.
22. Add `# type: ignore`, `# noqa`, `# nosec`, or `# pragma: no cover` without a specific justification.
23. Let generated code introduce massive functions, duplicate logic, and ambiguous names.
24. Use comments to contradict code instead of making code clear.
25. Use clever one-liners where explicit code would be safer.
26. Use `datetime.now()` naive timestamps for security, expiry, audit, billing, or distributed systems.
27. Use local time for tokens, signatures, expiry, or cross-region events.
28. Ignore encoding in file I/O for portable code.
29. Leave resources unclosed instead of using context managers.
30. Block inside `async def` with synchronous I/O or CPU-heavy work.
31. Forget timeouts and cancellation handling in async code.
32. Use threads/processes without clear ownership, shutdown, and exception handling.

## 9. Cryptography, randomness, and passwords

1. `random` for passwords, tokens, reset links, session IDs, API keys, salts, nonces, or cryptographic keys.
2. `uuid4()` treated as a universal secret token without considering threat model and entropy needs.
3. MD5 for passwords, signatures, or security integrity.
4. SHA-1 for passwords, signatures, or security integrity.
5. Unsalted password hashes.
6. Fast hashes for passwords.
7. Reversible password storage.
8. Hardcoded salts, peppers, keys, IVs, nonces, or signing secrets.
9. Reusing nonces with stream ciphers or AEAD modes.
10. AES-ECB.
11. Encryption without authentication.
12. “Encrypting” by base64, XOR, rot13, compression, or obfuscation.
13. Homegrown JWT, OAuth, HMAC, password reset, or session schemes.
14. Comparing tokens with `==` where timing can matter.
15. Ignoring token length and entropy.
16. Long-lived reset tokens.
17. Reset tokens that do not become invalid after use.
18. Passwords or secrets logged during validation.
19. Storing encryption keys next to encrypted data.
20. Using production signing keys in tests.
21. Continuing to use compromised keys.
22. Using outdated TLS versions or ciphers by manually weakening SSL contexts.
23. Disabling hostname verification.
24. Trusting self-signed certs globally.
25. Treating checksums as security.

## 9. Error-handling sins

1. Never use bare `except:` to hide failures.
2. Never write `except Exception: pass`.
3. Never wrap an entire program in a broad try/except and continue.
4. Never swallow exceptions without logging, metrics, retry policy, or explicit recovery.
5. Never turn security failures into success responses.
6. Never ignore failed auth checks.
7. Never ignore failed payment checks.
8. Never ignore failed webhook verification.
9. Never ignore failed file writes.
10. Never ignore failed subprocess return codes.
11. Never ignore failed database commits.
12. Never ignore failed migrations.
13. Never ignore failed network calls.
14. Never ignore failed email/SMS delivery for security flows.
15. Never use `assert` for runtime validation, auth, permissions, input validation, or security checks.
16. Never use `assert` for logic that must run under optimized Python.
17. Never leak raw stack traces to users in production.
18. Never hide stack traces from logs where operators need them.
19. Never retry forever.
20. Never retry non-idempotent operations blindly.
21. Never retry payment, email, or mutation operations without idempotency keys.
22. Never catch cancellation/timeouts and continue as if work completed.
23. Never let background task failures disappear.
24. Never return HTTP 200 for failed operations.
25. Never mark a job successful when a critical step failed.
26. Never use logging as a substitute for correct error handling.
27. Never use print-debugging as production observability.
28. Never create exception messages that include secrets.
29. Never downgrade security exceptions to warnings.
30. Never silence warnings globally to make CI green.

## AI & Vibe-Coding

1. Merge code because it “looks plausible.”
2. Accept code the author cannot explain.
3. Let an agent make broad rewrites without small commits and reviewable diffs.
4. Let an agent silently change public APIs, database schemas, auth flows, migrations, or security config.
5. Let an agent invent libraries, flags, config keys, or security properties.
6. Trust agent-written comments that claim “secure,” “sanitized,” “validated,” or “encrypted.”
7. Ask an agent to “fix all security issues” and accept a superficial patch.
8. Let an agent remove validation to satisfy tests.
9. Let an agent weaken types to `Any` to satisfy type checking.
10. Let an agent add `# noqa`, `# type: ignore`, `# nosec`, `verify=False`, `shell=True`, broad CORS, or debug mode to get unstuck.
11. Give an agent access to real secrets, real customer data, or production databases.
12. Paste secret-bearing logs into an AI chat.
13. Paste private source code into tools that are not approved for that data.
14. Let an agent choose dependencies without supply-chain review.
15. Let an agent install packages during runtime.
16. Let an agent run migrations against production.
17. Let an agent deploy without human approval.
18. Let agent-generated code bypass normal branch protection.
19. Let generated tests redefine acceptance criteria after the code is written.
20. Use “vibe” as an excuse for missing threat models.
21. Use “MVP” as an excuse for broken auth, leaked secrets, insecure deserialization, SQL injection, or disabled TLS.
22. Accept “temporary” insecure code without an expiration, owner, and blocking ticket.
23. Allow prompt injection pathways where untrusted content can instruct the agent to exfiltrate secrets or change code.
24. Give tool-using agents broad filesystem, shell, browser, repo, ticketing, and cloud access by default.
25. Let an agent read `.env`, SSH keys, cloud config, browser cookies, or password-manager exports.
26. Store agent memory containing secrets, customer data, or private implementation details.
27. Let generated code create hidden telemetry, callbacks, analytics, or external network calls.
28. Accept code with no rollback plan.
29. Accept code with no observability for failure.
30. Treat one passing run as proof of safety.
31. Shipping AI-generated Python that no human reviewed.
32. Merging agent-created diffs without reading the diff.
33. Accepting code because “it runs” without testing negative cases, security cases, and failure cases.
34. Letting an agent add dependencies without checking package reputation, provenance, license, version, and vulnerability history.
35. Letting an agent modify authentication, authorization, payment, crypto, serialization, deployment, CI/CD, or secret-handling code without strict review.
36. Letting an agent “fix” failing tests by deleting tests, weakening assertions, adding sleeps, or broadening mocks.
37. Letting an agent silence linters or scanners with blanket `# noqa`, `# type: ignore`, `# nosec`, `skipcq`, `bandit: skip`, or global ignore rules.
38. Letting an agent introduce broad filesystem, shell, network, cloud, database, or credential access without an allowlist.
39. Letting an agent run destructive commands without a dry-run mode, preview, or explicit bounded target.
40. Treating an LLM’s “security review” as a substitute for SAST, dependency audit, secret scanning, tests, and human review.
41. Pasting secrets, production logs, customer data, private stack traces, tokens, cookies, database dumps, or proprietary code into an LLM prompt without an approved data policy.
42. Asking an agent to “make it work” on security-sensitive code without specifying security invariants.
43. Accepting agent code that has no clear ownership, no tests, no rollback path, and no observability.
44. Letting agent-generated code mutate global state, environment variables, config files, databases, or cloud resources implicitly.
45. Letting an agent install packages globally on a developer machine or CI runner.
46. Letting an agent run `curl | bash`, `pip install` from a random source, or clone-and-run unknown repositories.
47. Letting an agent change lockfiles or dependency constraints without explaining why.
48. Letting an agent create clever abstractions that humans cannot quickly audit.
49. Letting an agent write “temporary” insecure code paths that become permanent.
50. Treating generated code as low-risk because it is “just Python.”
51. Human diff review.
52. Tests for happy path, failure path, and malicious/invalid inputs.
53. No committed secrets.
54. No unsafe deserialization of untrusted data.
55. No command execution with tainted input.
56. No raw SQL interpolation.
57. No disabled TLS verification.
58. No hardcoded credentials.
59. No broad exception swallowing.
60. No blanket scanner/linter suppressions.
61. Pinned/reproducible dependencies.
62. Dependency vulnerability scan.
63. Secret scan.
64. Static/security lint scan.
65. CI with required checks.
66. Least-privilege CI and runtime credentials.
67. Clear ownership of generated code.
68. Clear rollback path for migrations/deployments.
69. Logging without leaking secrets or PII.
70. A reviewer who understands the security boundary being changed.
71. “It works on my machine” with no reproducible setup.
72. No `pyproject.toml`, lockfile, or dependency manifest.
73. No README for running tests and services.
74. No documented environment variables.
75. No sample config that excludes secrets.
76. No explicit trust boundaries.
77. No error model.
78. No threat model for auth, secrets, files, network, and agents.
79. No tests for the code the agent just changed.
80. No security scanner run after AI-generated code.
81. No explanation for complex generated code.
82. Giant diffs with unrelated changes.
83. Generated code that adds dependencies for trivial tasks.
84. Generated code that adds shell commands for things Python can do safely.
85. Generated code that handles errors by swallowing them.
86. Generated code that handles auth by bypassing it.
87. Generated code that handles parsing by using `eval`.
88. Generated code that handles serialization by using pickle.
89. Generated code that handles speed by removing validation.
90. Generated code that handles test failures by weakening tests.
91. Generated code that handles linter failures by blanket ignoring rules.
92. Generated code that handles type errors by using `Any` everywhere.
93. Generated code that handles secrets by adding them to env examples with real values.
94. Generated code that adds “temporary” debug endpoints.
95. Generated code that changes deployment, auth, IAM, networking, or migrations without focused review.
96. Generated code that nobody can confidently delete, test, or explain.

## Agent permissions

1. Give an agent unrestricted shell access.
2. Give an agent unrestricted filesystem read/write.
3. Give an agent production credentials.
4. Give an agent access to `.env`, SSH keys, cloud profiles, browser cookies, password vault exports, private repos, customer data, prod DBs, or billing systems by default.
5. Give an agent write/delete/deploy/email/payment/admin tools without explicit per-action approval.
6. Let an agent install packages, run migrations, delete files, push commits, open PRs, close incidents, rotate secrets, or change IAM without guardrails.
7. Give one agent both sensitive read access and external-send capabilities without a source/sink control.
8. Let agent tools accept arbitrary paths, URLs, shell commands, SQL, Python code, or JSON blobs.
9. Use denylist-only controls for dangerous tools.
10. Let agent loops run without budget, time, token, network, retry, and cost limits.
11. Let agents run on developer laptops with broad ambient credentials.

## Agent-generated code quality

1. Accept code because “the AI said it works.”
2. Accept imports that were not verified.
3. Accept code with no tests.
4. Accept code that cannot be explained by the committer.
5. Accept code that changes behavior outside the requested scope.
6. Accept huge agent PRs with unrelated rewrites.
7. Accept generated regex, crypto, auth, SQL, concurrency, parser, or permission code without expert review.
8. Accept generated migrations without checking data loss and rollback.
9. Accept generated code that hides side effects at import time.
10. Accept generated code that silently catches errors.
11. Accept generated code that weakens validation “to make tests pass.”
12. Accept generated code that removes security checks, logging, or monitoring.
13. Accept generated dependencies without provenance review.
14. Accept generated tests that only assert mocks were called and not behavior.
15. Accept generated docs that do not match code.
16. Let an agent mark its own task complete without running the project’s actual checks.

## Archives and decompression

1. Extract ZIP/TAR files from users or external systems without path traversal checks, file count limits, total uncompressed size limits, compression-ratio limits, symlink/hardlink checks, and destination isolation.
2. Trust archive member names.
3. Let archives overwrite existing files.
4. Extract archives as root or into application code directories.
5. Process compressed XML/ZIP/GZIP/LZMA streams without decompression-bomb protections.
6. Load Python packages, plugins, notebooks, model artifacts, or config directly from uploaded archives.
7. Assume `zipfile.extractall()` alone is your security boundary.

## Async and concurrency

1. Call blocking network/file/database/subprocess code directly inside an event loop.
2. Forget `await`.
3. Create fire-and-forget tasks without tracking, cancellation, and exception handling.
4. Share mutable state across threads/tasks without locks or immutability.
5. Use global request state in async services.
6. Use thread-local assumptions in async code.
7. Mix sync and async DB sessions incorrectly.
8. Ignore cancellation.
9. Hold locks across awaits unless intentionally designed.
10. Use `time.sleep()` in async code.
11. Assume the GIL makes compound operations safe.
12. Write non-idempotent background jobs without dedupe, locking, or transaction boundaries.
13. Run app code as database superuser/root.
14. Use one DB user for all services and migrations.
15. Give read/write/delete/schema privileges to code that only needs read.
16. Make destructive migrations without backups, dry runs, rollback plans, and explicit review.
17. Auto-run destructive migrations at application startup.
18. Drop/recreate tables in production because “the model changed.”
19. Store PII without data classification and retention rules.
20. Copy production data to dev laptops, notebooks, demos, or agent contexts without anonymization and approval.
21. Put production DB dumps in object storage, CI artifacts, Slack, Git, notebooks, or issue attachments.
22. Log query results containing sensitive data.
23. Cache tenant/user-specific data without tenant/user keys.
24. Trust database data as safe just because it is “internal.”
25. Build second-order injection by storing untrusted input and later using it in SQL/shell/templates.
26. Skip transactions for multi-step writes that must be atomic.
27. Ignore isolation, locking, idempotency, and retry behavior.
28. Use timestamps or incremental IDs as authorization boundaries.
29. Expose raw internal IDs where authorization is missing.
30. Implement soft delete without access rules and retention cleanup.
31. Merge behavior changes without tests.
32. Merge security fixes without regression tests.
33. Let generated tests replace human review.
34. Skip tests to make CI green.
35. Use `pytest.mark.skip`, `xfail`, or broad mocks without a reason and owner.
36. Write tests that only verify implementation details.
37. Write tests that pass even if the feature is broken.
38. Depend on test order.
39. Depend on real time, local timezone, random order, network availability, external APIs, or production credentials in unit tests.
40. Use `sleep()` to “fix” flaky tests.
41. Use shared mutable fixtures without cleanup.
42. Leave test data in production systems.
43. Run destructive tests against production-like credentials.
44. Let tests hit paid APIs by default.
45. Use real secrets in tests.
46. Generate golden files from current broken output and call it done.
47. Ignore coverage gaps in auth, permissions, parsing, validation, error handling, migrations, and billing.
48. Mock security checks away.
49. Test only happy paths.
50. Avoid property/fuzz tests for parsers, serializers, validators, and boundary-heavy code.
51. Have behavior depend silently on undeclared environment variables.
52. Fail open when config is missing.
53. Use production defaults in development templates.
54. Use development defaults in production.
55. Use `DEBUG=True` by default.
56. Put secrets in config files committed to Git.
57. Mix dev/staging/prod credentials.
58. Let local config override security controls silently.
59. Use broad `ALLOWED_HOSTS = ["*"]`, wildcard CORS with credentials, or permissive origins by default.
60. Rely on current working directory, relative paths, or developer-specific absolute paths.
61. Hide config in code instead of typed, validated settings.
62. Start services successfully with invalid or partial config.
63. Let an agent rewrite config without showing a diff.
64. Keep feature flags undocumented, permanent, or unsafe by default.
65. Use environment variables as an unvalidated control plane for dangerous behavior.
66. Ship a Python app without a clear dependency file/lock strategy.
67. Leave the supported Python version ambiguous.
68. Leave no `README`, run command, test command, lint command, or setup instructions.
69. Require undocumented manual steps.
70. Depend on developer machine state.
71. Commit virtualenvs, `__pycache__`, `.pytest_cache`, `.mypy_cache`, build outputs, local databases, notebook checkpoints, or generated secrets.
72. Put large binaries/model files in Git without a storage/provenance strategy.
73. Use notebooks as the only production source of truth.
74. Have import cycles that require side-effect imports to work.
75. Hide CLI behavior in module import side effects.
76. Publish packages with missing license/provenance.
77. Publish internal packages accidentally to public registries.
78. Name internal packages in ways that collide with public packages.
79. Trust package names hallucinated by AI tools.
80. Leave dead code, unused dependencies, stale scripts, and obsolete config in place.
81. Run Python services as root unless there is a tightly justified and isolated reason.
82. Run containers privileged by default.
83. Mount host Docker socket into app containers.
84. Give app containers broad cloud IAM.
85. Put production secrets in images.
86. Expose metrics/admin/debug endpoints publicly.
87. Expose Jupyter, Streamlit, Gradio, Flower, Celery dashboards, Redis, RabbitMQ, Postgres, or admin UIs without auth/network isolation.
88. Deploy without health checks.
89. Deploy without rollback.
90. Deploy without logs/metrics/traces for critical paths.
91. Deploy without rate limits and resource limits.
92. Deploy without backups for stateful systems.
93. Deploy without monitoring failed auth, permission failures, data exports, deletes, billing changes, and admin actions.
94. Let one failed dependency hang all workers indefinitely.
95. Let background jobs retry forever and duplicate side effects.
96. Let queues grow without bounds.
97. Store user data in local container disk without persistence strategy.
98. Use local SQLite for multi-worker production writes unless explicitly designed.
99. Ignore OS/package/security updates.
100. Leave old endpoints, old secrets, old tokens, and old service accounts alive.
101. Merge security-sensitive code without a second reviewer.
102. Merge auth/crypto/payment/data-deletion/permission/migration code based only on AI output.
103. Skip threat modeling for new external input, file parsing, webhooks, agent tools, uploads, auth flows, or data exports.
104. Let the author decide alone that a finding is a false positive.
105. Suppress SAST/lint/type/security warnings without an inline reason and owner.
106. Accept “temporary” bypasses without expiry.
107. Accept TODOs in security-critical code without a tracking issue.
108. Make huge PRs that mix refactor, behavior, formatting, dependency, and security changes.
109. Review only the diff when the change affects trust boundaries.
110. Ignore abuse cases.
111. Ignore privacy/legal constraints.
112. Ignore backwards compatibility in libraries.
113. Ignore migration/rollback plans.
114. Ignore operational runbooks.
115. Let undocumented tribal knowledge be required to run or secure the system.
116. Does it execute strings as code?
117. Does it deserialize untrusted objects?
118. Does it shell out with user/model-controlled input?
119. Does it build SQL/queries/templates/paths from strings?
120. Does it commit, print, log, or prompt with secrets?
121. Does it disable TLS, auth, CSRF, validation, or security checks?
122. Does it load packages suggested by AI without verification?
123. Does it add dependencies without pinning/review?
124. Does it let an agent use broad tools or ambient credentials?
125. Does it place untrusted content into privileged prompts/messages?
126. Does it lack tests for changed behavior?
127. Does it catch and suppress errors?
128. Does it lack timeouts/resource limits on I/O?
129. Does it trust filenames, URLs, headers, content types, cookies, JWTs, or client-side checks?
130. Does it change migrations, permissions, billing, auth, crypto, data export/delete, deployment, or CI without expert review?

## Code execution and dynamic evaluation

1. Use `eval()`, `exec()`, `compile()`, `execfile`-style behavior, dynamic imports, or `ast` tricks on untrusted or semi-trusted input.
2. “Sandbox” `eval()` with `{"__builtins__": None}` and pretend it is safe.
3. Let users, files, environment variables, prompts, model outputs, database rows, URLs, queue messages, YAML, JSON, CSV, Markdown, notebooks, or config files become Python code.
4. Generate Python code with an LLM and run it automatically.
5. Store business rules as Python snippets in a database and execute them.
6. Pass model/tool output into `exec`, `eval`, Jinja templates, shell commands, SQL strings, or Python import paths.
7. Use `logging.config.listen()` or config formats that can evaluate attacker-controlled logging config.
8. Hide dynamic execution behind helper names like `safe_eval`, `run_formula`, `execute_rule`, `load_plugin`, or `dynamic_transform`.

## Code execution and unsafe deserialization

1. Use `eval()`, `exec()`, `compile()`, dynamic `__import__()`, or `getattr()` dispatch on untrusted or semi-trusted input.
2. Use `pickle.loads()`, `pickle.load()`, `dill`, `cloudpickle`, `joblib.load()`, `shelve`, or `multiprocessing.Connection.recv()` on untrusted data.
3. Use `yaml.load()` without `SafeLoader` / `safe_load()` on data you did not author and control.
4. Treat “internal input” as safe just because it came from another service, queue, cache, notebook, agent, or CI job.
5. Build a plugin system by importing arbitrary module names supplied by users, config, LLM output, webhooks, or database rows.
6. Run generated Python code because “the agent wrote it” or “it passed one smoke test.”
7. Use `ast.literal_eval()` as a general parser for untrusted large input; it is safer than `eval`, but still not a substitute for bounded structured parsing.
8. Use `marshal` for external or persistent data formats.
9. Let notebooks, “temporary scripts,” or agent scratch files execute production credentials.
10. Accept Python expressions as configuration.

## Cryptography and randomness

1. Use `random` for passwords, tokens, reset links, API keys, nonces, salts, session IDs, verification codes, invite links, CSRF tokens, or anything security-related.
2. Use MD5, SHA-1, bare SHA-256, or fast hashes for password storage.
3. Store passwords in plaintext, reversible encryption, unsalted hashes, or homegrown password schemes.
4. Roll your own crypto protocol, token format, password reset flow, key exchange, signature format, or encryption mode.
5. Reuse IVs/nonces with encryption modes that require uniqueness.
6. Use ECB mode.
7. Disable certificate validation to “fix” TLS errors.
8. Use hardcoded JWT signing keys or accept `alg=none`.
9. Accept unsigned, unverified, expired, wrong-audience, wrong-issuer, or wrong-tenant tokens.
10. Use comparison operators for secrets where timing leaks matter; use constant-time comparison.

## Deserialization

1. `pickle.load()` or `pickle.loads()` anything that could have come from a user, network, queue, file upload, model artifact, cache, database, Git repo, CI artifact, object store, email, notebook, or vendor.
2. Use `shelve`, `dill`, `cloudpickle`, `joblib`, `jsonpickle`, or ML model files as if they were safe data formats.
3. Use PyYAML unsafe loaders, `yaml.load()` with unsafe/default loaders, or object constructors from untrusted YAML.
4. Accept “it’s signed” as a reason to deserialize if the signing key can be reused, leaked, shared broadly, or bypassed.
5. Treat internal queues as trusted if any external service, webhook, test environment, or agent can write to them.
6. Deserialize before authentication, authorization, schema validation, tenant isolation, and size limits.
7. Load pickled ML models from random GitHub repos, Kaggle notebooks, Hugging Face uploads, build artifacts, or agent-suggested URLs without a hardened trust process.

## Django

1. Ship with `DEBUG=True`.
2. Ship with weak or committed `SECRET_KEY`.
3. Use `ALLOWED_HOSTS = ["*"]` casually.
4. Disable CSRF globally.
5. Disable secure cookie settings in production.
6. Use raw SQL with interpolated input.
7. Use unsafe password hashers.
8. Trust `request.GET`, `POST`, or JSON without form/schema validation.
9. Use pickle-based serializers for untrusted data.
10. Expose admin publicly without additional protections where risk requires them.

## Exceptions and error handling

1. Use bare `except:`.
2. Use `except Exception: pass`.
3. Catch everything and continue as if the operation succeeded.
4. Hide partial failure in data writes, payments, auth, security checks, migrations, file operations, or external API calls.
5. Convert all exceptions to `None`/`False` without preserving context.
6. Retry non-idempotent operations blindly.
7. Retry forever.
8. Retry without backoff/jitter.
9. Log the same exception repeatedly in tight loops.
10. Raise generic `Exception` for domain errors when callers need to respond differently.
11. Use exceptions for normal loop control in hot paths where simple branching is clearer.
12. Suppress `CancelledError` or task cancellation in async code.
13. Ignore cleanup after failed transactions.

## FastAPI / Starlette / Pydantic

1. Treat Pydantic validation as authorization.
2. Accept raw `dict` for complex untrusted bodies when a model is needed.
3. Disable response-model validation in security-sensitive endpoints without reason.
4. Return internal ORM objects directly if they include secrets/private fields.
5. Block the event loop in async endpoints.
6. Trust dependency-injected user objects without checking resource-level permissions.
7. Expose Swagger/OpenAPI publicly for private APIs without review.
8. Accept file uploads without size/type/content scanning policy.

## File path handling

1. Join user input directly into paths for reading, writing, deleting, serving, importing, or archiving.
2. Trust filenames from uploads, ZIP/TAR entries, HTTP headers, emails, S3 keys, Git repos, or model output.
3. Allow `../`, absolute paths, Windows drive paths, UNC paths, symlinks, null bytes, Unicode confusables, or hidden dotfiles to escape intended directories.
4. Use user-controlled filenames for templates, config, imports, logs, cache keys, or output files.
5. Delete paths derived from user input without canonicalization, base-directory enforcement, and authorization.
6. Serve files by direct path instead of stable IDs mapped server-side.
7. Put sensitive config inside web roots or static directories.
8. Treat “we sanitized the string” as sufficient for path safety.

## File uploads

1. Trust `Content-Type`.
2. Trust the file extension.
3. Use the original filename as the stored filename.
4. Store uploads inside the webroot.
5. Let uploaded files be executable or importable.
6. Accept unlimited size, dimensions, pages, rows, sheets, nested archives, or compression ratios.
7. Allow uploads from unauthenticated users unless the whole design is abuse-hardened.
8. Skip malware/sandbox/CDR checks where file risk matters.
9. Serve uploaded content under your primary app domain without isolation.
10. Parse uploaded Office/PDF/image/XML/CSV/archive files in privileged workers without resource limits.
11. Return uploaded files inline when they should be attachment-only.
12. Run Flask/Django/FastAPI/debug servers in production with debug mode, auto-reload, interactive debugger, or traceback exposure.
13. Use `python -m http.server` or `http.server` as a production server.
14. Bind admin/debug/dev tools to `0.0.0.0`.
15. Use default `SECRET_KEY`, weak Flask/Django secrets, default admin credentials, or tutorial config.
16. Disable CSRF protection for browser-authenticated state-changing endpoints.
17. Use GET for destructive actions.
18. Trust hidden fields, disabled fields, client-side validation, frontend route guards, or UI visibility for authorization.
19. Fail to check authorization on every object access.
20. Implement access control only in templates or React/Vue routes.
21. Use wildcard CORS with credentials.
22. Set cookies without appropriate `Secure`, `HttpOnly`, and `SameSite` flags.
23. Put JWTs or session IDs in query strings.
24. Return raw exceptions, stack traces, SQL errors, environment values, or dependency versions to users.
25. Disable rate limiting on login, password reset, invite, OTP, expensive search, file upload, webhook, or agent endpoints.
26. Accept arbitrary redirect URLs.
27. Fetch arbitrary user-provided URLs server-side without SSRF controls.
28. Allow SSRF to `localhost`, cloud metadata IPs, private ranges, link-local addresses, file schemes, gopher/data/dict schemes, redirects to internal addresses, or DNS rebinding targets.
29. Treat internal services as trusted just because they are internal.
30. Use template `|safe`, `Markup`, disabled autoescaping, or raw HTML rendering on user/model content.
31. Render Markdown/HTML from untrusted sources without a sanitizer and strict allowlist.
32. Return sensitive data in autocomplete, search, error messages, analytics, or logs.
33. Log passwords, tokens, cookies, Authorization headers, API keys, private keys, reset links, magic links, OTPs, database URLs, `.env` values, session IDs, raw JWTs, or cloud credentials.
34. Log full request/response bodies by default.
35. Log PII, health data, government IDs, payment data, or confidential business data unless legally justified, minimized, masked, access-controlled, and retention-bound.
36. Use `print()` debugging for production services.
37. Let libraries configure root logging globally.
38. Swallow exceptions silently.
39. Use `except Exception: pass`.
40. Log an error and continue after data corruption, failed auth checks, failed payment checks, failed signature checks, failed migration, or partial write.
41. Emit stack traces to clients.
42. Emit logs without request IDs/correlation IDs for sensitive operations.
43. Let logs be writable/deletable by the application user without tamper controls.
44. Put logs in public buckets or user-visible build artifacts.
45. Disable audit logs for admin, auth, permission, export, delete, billing, deployment, and secret-access events.
46. Let agent actions happen without audit trails.
47. `pip install` a package suggested by an LLM without verifying it exists, is the intended project, has acceptable maintainership, provenance, version history, license, and security posture.
48. Install typo-squatted, hallucinated, abandoned, or lookalike packages because “the import worked.”
49. Use unpinned dependencies in deployable applications.
50. Use broad version ranges in production deploys without a lockfile or repeatable build process.
51. Deploy without vulnerability scanning.
52. Ignore known-critical dependency vulnerabilities.
53. Pull from `main`, random Git SHAs, unreviewed forks, pastebins, gists, or arbitrary archives in production builds.
54. Use `curl | python`, `curl | bash`, or agent-generated installer commands.
55. Allow dependency installation at runtime.
56. Let application startup mutate dependencies.
57. Install packages globally on shared hosts.
58. Mix system Python packages with app packages.
59. Use `--extra-index-url` carelessly in a way that enables dependency confusion.
60. Publish packages with compromised PyPI tokens, no MFA, shared credentials, or broad maintainers.
61. Ship vendored code without provenance and update process.
62. Skip hash verification for high-integrity deployment paths.
63. Let CI install dev dependencies with production secrets available.
64. Merge Python code that fails tests, lint, type checks, SAST, dependency scan, or secret scan.
65. Disable branch protection to “just get it in.”
66. Let AI-generated code bypass human review.
67. Approve your own security-sensitive PR.
68. Use broad CI tokens by default.
69. Expose secrets to forked PRs.
70. Use `pull_request_target`-style workflows unsafely.
71. Run untrusted PR code with write tokens or secrets.
72. Let CI jobs deploy from unreviewed branches.
73. Allow arbitrary workflow dispatch inputs to become shell commands.
74. Cache virtualenvs/build artifacts without key isolation.
75. Upload build artifacts containing `.env`, logs, test DBs, coverage with source secrets, or generated config.
76. Put production deploy credentials in repo variables available to all jobs.
77. Skip CODEOWNERS/security review for auth, crypto, payments, permissions, migrations, data export/delete, agent tools, or deployment changes.
78. Use CI as a remote shell for agents.
79. Let agents auto-commit, auto-push, auto-merge, auto-release, or auto-deploy without review gates.
80. Ignore GitHub/GitLab secret or code scanning alerts.
81. Treat “private repo” as a secrets manager.

## Flask

1. Ship with `debug=True`.
2. Use a placeholder `SECRET_KEY`.
3. Use unsigned or weakly signed cookies.
4. Disable CSRF where cookie auth is used.
5. Trust `request.json` without schema validation.
6. Return raw exception text to users.
7. Use Jinja `autoescape=False`.
8. Mark user content safe.
9. Bind the dev server publicly.
10. Use Flask’s dev server as production.

## Input validation and data contracts

1. Validate only on the client.
2. Trust type hints as runtime validation.
3. Trust Pydantic/dataclasses/TypedDict without checking boundaries and constraints.
4. Accept arbitrary dictionaries as config, request, message, or tool parameters.
5. Accept unknown fields silently in security-sensitive inputs.
6. Validate with a denylist when a strict allowlist is possible.
7. Fail open when validation fails.
8. Treat `None`, empty string, `0`, missing field, and false as interchangeable in business logic.
9. Use regex for complex parsers when a real parser exists.
10. Parse dates, currencies, URLs, IPs, emails, and hostnames with ad hoc string splitting.
11. Ignore Unicode normalization, case folding, confusables, and locale issues for usernames, filenames, domains, identifiers, and allowlists.

## Minimum non-negotiable enforcement stack

1. Formatting/linting: `ruff format` and `ruff check`.
2. Type checking: `mypy` or `pyright` for important code paths.
3. Tests: `pytest`, with security and failure-path tests.
4. Security static analysis: Bandit and Semgrep.
5. Dependency audit: `pip-audit` or equivalent.
6. Secret scanning: GitHub/GitLab secret scanning, Gitleaks, detect-secrets, or equivalent.
7. Lockfiles and hashes for production builds.
8. CI required before merge.
9. Human review for auth, crypto, secrets, subprocess, deserialization, dependency, deployment, migration, and agent-permission changes.
10. Sandboxed agent execution with least privilege, no ambient secrets, egress controls, workspace write boundaries, and auditable actions.

## Never accept agent output without human-grade review

1. No diff review.
2. No threat-model review for auth, parsing, subprocess, file, network, dependency, or secrets changes.
3. No tests run locally/CI.
4. No dependency review.
5. No migration review.
6. No rollback plan.
7. No observability for new behavior.
8. No documentation for changed interfaces.
9. No owner for generated code.
10. “The AI said it is secure” as evidence.
11. Fix TLS errors with `verify=False`.
12. Fix auth errors by removing auth.
13. Fix CSRF errors by disabling CSRF.
14. Fix CORS by allowing all origins with credentials.
15. Fix SQL errors by using string concatenation.
16. Fix pickle/YAML errors by using more permissive loaders.
17. Fix path errors by using absolute user-provided paths.
18. Fix permission errors with `chmod -R 777`.
19. Fix import errors by mutating `sys.path` randomly.
20. Fix race conditions with `time.sleep`.
21. Fix flaky tests by increasing sleeps.
22. Fix timeouts by removing timeouts.
23. Fix dependency conflicts by unpinning everything.
24. Fix scanner noise by disabling scanners.
25. Fix failing tests by deleting tests.
26. Fix type errors with blanket `Any`.
27. Fix lint errors with blanket `# noqa`.
28. Fix exceptions with `except Exception: pass`.
29. Fix production debug need by turning debug on.
30. Fix performance by caching sensitive data forever.
31. Fix “not found” by broadening access.
32. Fix “invalid input” by accepting anything.
33. Executes strings as code.
34. Runs shell commands from uncontrolled input.
35. Builds queries by string interpolation.
36. Deserializes untrusted objects.
37. Parses untrusted XML/YAML/archives without safe loaders and limits.
38. Commits, logs, prints, prompts, or exposes secrets.
39. Disables TLS verification, auth, CSRF, validation, scanners, tests, or logging.
40. Uses weak crypto or non-crypto randomness for security.
41. Trusts the client for identity, role, tenant, ownership, or path.
42. Handles files/uploads/archives without containment and limits.
43. Fetches arbitrary URLs server-side.
44. Runs debug/dev servers in production.
45. Uses broad exception swallowing or `assert` for production checks.
46. Ships without tests for new behavior and failure modes.
47. Adds dependencies without review, pinning/locking strategy, and vulnerability scanning.
48. Runs CI workflows with secrets/write permissions on untrusted code.
49. Lets an agent change broad/unrelated code or weaken gates to get green.
50. Produces code that cannot be read, reviewed, reproduced, monitored, or rolled back.

## Never build SQL, NoSQL, LDAP, GraphQL, or search queries by string concatenation

1. `f"SELECT * FROM users WHERE id = {user_id}"`.
2. `"WHERE name = '%s'" % name`.
3. `.format()` or f-string SQL, even when “it’s just an integer.”
4. SQLAlchemy `text()` with untrusted interpolated text.
5. Mongo/NoSQL queries built from raw request JSON without schema validation and allow-lists.
6. Dynamically choosing table, column, sort, filter, operator, or index names from user strings.
7. Letting agents “fix” SQL errors by making queries more permissive.
8. Falling back to raw SQL because the ORM query was inconvenient.
9. Swallowing SQL errors and retrying with broader queries.

## Never construct shell commands with untrusted data

1. `subprocess.run("cmd " + user_input, shell=True)`.
2. `os.system`, `os.popen`, `commands.*`, `pexpect`, `sh -c`, `bash -c`, or `subprocess(..., shell=True)` with any untrusted component.
3. Passing a single command string when an argv list is possible.
4. Manually quoting shell strings as the main defense.
5. Concatenating file paths, branch names, URLs, package names, issue titles, PR titles, or model-generated text into shell commands.
6. Running package manager commands generated by an agent without review.
7. Invoking shell pipelines for simple operations that Python APIs can do safely.
8. Letting user input influence `cwd`, `env`, executable path, or `PATH`.

## Never create hidden shared state accidentally

1. Mutable default arguments for state: `def f(x=[]): ...`.
2. Global mutable registries without lifecycle/reset.
3. Module-level work that opens sockets, reads env, starts threads, writes files, or calls services at import time.
4. Singleton clients initialized before config is loaded.
5. Monkeypatching builtins, stdlib modules, third-party APIs, or global settings outside isolated tests.
6. `from module import *`.
7. Shadowing builtins: `list`, `dict`, `id`, `file`, `input`, `type`, `open`.
8. Circular imports “fixed” by moving imports into random functions without design.
9. Runtime mutation of `sys.path` to make imports work.
10. Importing from parent directories or relying on cwd.
11. “Works when run from my IDE” path logic.

## Never deserialize untrusted Python objects

1. `pickle.load(s)`, `pickle.loads(s)`, `dill`, `cloudpickle`, `shelve`, `jsonpickle`, `marshal`, `joblib.load`, `pandas.read_pickle`, `torch.load`, or similar on files from users, uploads, URLs, caches, queues, model outputs, public buckets, emails, CI artifacts, or “trusted” internal services without provenance guarantees.
2. Treating signed pickle as safe when the signing key is broadly accessible.
3. Using pickle for API payloads.
4. Accepting `.pkl`, `.pickle`, `.pt`, `.pth`, `.joblib`, `.sav`, `.dill`, or model artifacts from users and loading them in-process.
5. Allowing plugins/extensions to ship as pickled Python objects.
6. Using `marshal` for application data.
7. “It came from S3” or “it came from our DB” as a trust argument after any user or compromised service can write there.

## Never execute dynamic code from untrusted or uncontrolled input

1. Using `eval()`, `exec()`, `compile()`, `execfile`-style helpers, dynamic `import`, `__import__`, `getattr` dispatch, or plugin loaders on strings influenced by users, files, tickets, prompts, webpages, issue comments, environment variables, config, database rows, or model output.
2. “Sandboxing” `eval` with `{}` and pretending it is safe.
3. Letting an agent generate Python and immediately run it against real data, production credentials, or a developer machine with broad filesystem access.
4. Building Python expressions with f-strings and evaluating them.
5. Using `ast.literal_eval` as a magic security blanket without bounding input size and type.
6. Loading “rules,” “formulas,” “templates,” “workflows,” or “business logic” from editable text and interpreting it as Python.
7. Accepting user-supplied module names, class names, function names, or dotted paths and importing/calling them directly.
8. Using dynamic deserialization or metaprogramming to “make the tests pass” instead of designing a real interface.

## Never fetch arbitrary URLs from user input

1. Server-side `requests.get(user_url)` without allow-listing scheme, host, port, DNS behavior, redirect behavior, IP ranges, and response limits.
2. Allowing access to `localhost`, RFC1918/private ranges, link-local metadata services, Kubernetes service IPs, cloud metadata endpoints, Unix sockets, internal DNS, or admin panels.
3. Following redirects from allowed external hosts to internal hosts.
4. Fetching files from `file://`, `ftp://`, `gopher://`, custom schemes, or internal services.
5. Returning fetched content blindly to users.
6. No timeout.
7. No max response size.
8. No content-type validation.
9. No retry limits.
10. No rate limiting.
11. Using user-controlled webhook URLs without validation.

## Never hide failures

1. Bare `except:`.
2. `except Exception: pass`.
3. `try: ... except: return None`.
4. Catching everything and continuing.
5. Catching `BaseException`.
6. Swallowing `KeyboardInterrupt`, `SystemExit`, cancellation, or timeout errors.
7. Logging an error and pretending success.
8. Replacing specific exceptions with generic falsey values.
9. Broad retry loops that hide systemic failure.
10. “Best effort” security checks.
11. Ignoring return codes from subprocesses.
12. Ignoring failed writes, failed deletes, failed auth, failed encryption, failed validation, failed migrations, failed dependency scans, or failed tests.
13. Catching exceptions around a large block instead of the precise operation.
14. Losing exception context with `raise NewError()` instead of `raise NewError(...) from exc` when appropriate.
15. Printing exceptions instead of structured logging and error propagation.

## Never ignore known vulnerable dependencies

1. Merging with open high/critical dependency alerts without documented risk acceptance.
2. “It’s transitive so it’s not our problem.”
3. “It’s only a dev dependency” without proving it cannot affect build/release/CI.
4. Pinning a vulnerable version forever.
5. Using unmaintained packages for authentication, crypto, parsing, XML, YAML, markdown, templating, HTTP, JWT, SAML, OAuth, or file processing.
6. No Dependabot/pip-audit/Safety/GitLab dependency scanning/equivalent.
7. No owner assigned to dependency alerts.
8. No update tests.

## Never install or upgrade dependencies casually

1. `pip install package` from an agent suggestion without checking name, source, maintainer, release history, license, vulnerabilities, and whether it is actually needed.
2. Copying install commands from random READMEs, Stack Overflow, Reddit, X, issue comments, or model output.
3. Installing typo-squatted, dependency-confusion, or hallucinated packages.
4. Allowing unpinned broad ranges in deployable apps without lockfiles.
5. No lockfile for applications.
6. No hash checking for high-integrity builds.
7. Installing from `main`, random Git SHAs, forks, gists, pastebins, or URLs without review.
8. `curl | python`, `curl | bash`, or running arbitrary setup scripts.
9. Ignoring package install-time code execution risk.
10. Depending on abandoned packages with known CVEs.
11. Vendoring code without provenance.
12. Removing dependency pins to “fix conflicts.”
13. Editing lockfiles by hand.
14. Disabling dependency scanners because they are noisy.
15. Failing to separate dev/test/prod dependencies.
16. Installing build tools/compilers in production images unnecessarily.
17. Not reviewing transitive dependency changes in PRs.
18. Not using private index controls for internal package names.

## Never invent cryptography

1. Custom encryption algorithms, custom MACs, custom token formats, custom password hashing, custom signing, custom key exchange.
2. MD5, SHA1, DES, RC4, ECB mode, static IVs/nonces, reused nonces, predictable IVs, or unauthenticated encryption for security.
3. Using `random` for tokens, passwords, reset links, invite codes, CSRF tokens, session IDs, API keys, salts, nonces, or crypto keys.
4. Hardcoded crypto keys.
5. Reusing keys across environments or purposes.
6. Failing to rotate keys.
7. Disabling certificate validation.
8. `requests.get(..., verify=False)`.
9. `ssl._create_unverified_context()`.
10. Turning off hostname checks.
11. Allowing TLS fallback to insecure connections.
12. Building “encryption” with base64, XOR, rot13, Fernet keys in code, or “obfuscation.”
13. Signing without binding audience, issuer, expiration, and purpose.
14. Comparing HMACs/tokens with `==` when constant-time comparison is needed.

## Never let CI run untrusted code with secrets or write permissions

1. `pull_request_target` checking out and running fork code.
2. Workflows triggered by untrusted PRs with write-scoped `GITHUB_TOKEN`.
3. Passing PR title/body/branch name/commit message into shell commands.
4. Running untrusted tests on self-hosted runners with network/secrets.
5. Sharing caches across trust boundaries.
6. Exposing secrets to forks.
7. Printing secrets in CI.
8. Long-lived cloud credentials in CI instead of short-lived OIDC.
9. Default write-all permissions.
10. Release jobs triggered by unreviewed code.
11. Deployment from branches without protection.
12. CI that modifies source code and commits back without review.
13. “Fixing” CI by skipping tests, scanners, or permission checks.
14. No branch protection, CODEOWNERS, required reviews, or required checks for protected branches.

## Never let an agent make broad, hidden, or unrelated changes

1. Reformatting entire repositories during a feature change.
2. Renaming public APIs without migration.
3. Touching lockfiles without dependency intent.
4. Editing generated files manually.
5. Changing config defaults silently.
6. Changing tests unrelated to the task.
7. Moving files to satisfy imports instead of fixing package structure.
8. Adding new frameworks/libraries for small tasks.
9. Replacing stable code with a rewrite because it is easier for the model.
10. Adding abstraction layers not required by the problem.
11. Copy-pasting large chunks from old code and creating drift.

## Never let an agent operate with unnecessary authority

1. Agent has production secrets.
2. Agent can push to protected branches.
3. Agent can deploy.
4. Agent can publish packages.
5. Agent can approve its own PR.
6. Agent can run arbitrary shell commands on a developer laptop with SSH keys/cloud tokens.
7. Agent can access private data unrelated to the task.
8. Agent can browse issue comments/webpages and execute instructions found there.
9. Agent can modify CI permissions.
10. Agent can write to `$HOME`, global package caches, shell startup files, SSH config, or cloud config.
11. Agent can install packages globally.

## Never let an agent optimize for “green” over “correct”

1. Deleting tests.
2. Weakening assertions.
3. Broadening mocks.
4. Skipping security checks.
5. Disabling type/lint/security tools.
6. Making code less strict to pass tests.
7. Catching exceptions instead of fixing the cause.
8. Returning dummy data on failure.
9. Replacing real authorization with TODOs.
10. Making migrations optional because they fail.
11. “Temporary” bypasses with no ticket/expiry.
12. Fixing flakes by removing code paths.
13. Suppressing warnings without understanding them.

## Never log or expose sensitive data

1. Logging passwords, tokens, session IDs, cookies, Authorization headers, PII, PHI, card data, reset links, signed URLs, private prompts, embeddings of sensitive text, full request bodies, or full database records.
2. Returning stack traces, SQL errors, file paths, environment variables, dependency versions, or secrets to users.
3. Logging untrusted strings without preventing log injection/control characters.
4. Keeping debug logs enabled in production.
5. Sending secrets to analytics, crash reporting, LLM telemetry, or third-party observability without explicit review.
6. Treating logs as low-sensitivity.
7. Using print debugging with secrets and forgetting it.

## Never make code unreviewable

1. Giant functions.
2. Multi-thousand-line files.
3. Deeply nested conditionals.
4. Copy-paste branches with tiny differences.
5. Dead code kept “just in case.”
6. Commented-out code.
7. Unused imports, unused variables, unreachable branches.
8. Boolean parameters that radically change behavior.
9. Magic strings/numbers scattered everywhere.
10. Clever one-liners for critical logic.
11. Metaclasses/decorators/descriptors/dynamic attributes where simple code works.
12. Framework magic without tests.
13. Overloaded functions that accept anything.
14. `Any` everywhere to silence type errors.
15. No docstring/comment for non-obvious security-sensitive logic.
16. Comments that lie or describe old behavior.
17. No tests for new behavior.
18. Deleting or weakening tests to pass.
19. Marking failing tests as skipped/xfail without a tracked reason.
20. Changing assertions to match broken behavior.
21. Snapshot-updating without inspecting diffs.
22. Mocking the code under test.
23. Only testing happy paths.
24. No tests for authz, validation, boundaries, malformed input, concurrency, timeouts, and failure modes.
25. Tests that depend on order, wall-clock time, timezone, network, external APIs, local files, or developer machine state.
26. Tests that use production services or production credentials.
27. Tests that pass only when run individually.
28. Fixtures with real secrets or real customer data.
29. No regression test for a bug fix.
30. No migration tests for database changes.
31. No property/fuzz tests for parsers and validators where applicable.
32. “Manual tested” as a substitute for automated tests.
33. CI allowed to go red for long periods.
34. `# type: ignore` without a precise code and justification.
35. `# noqa` without a precise rule and justification.
36. `# nosec` / `bandit: skip` without a security ticket or documented false positive.
37. Disabling mypy/pyright/ruff/bandit/semgrep/CodeQL for whole files because fixing is hard.
38. Lowering scanner severity thresholds after findings appear.
39. Turning off dependency scanning because alerts are noisy.
40. Treating type hints as documentation only while runtime data is unvalidated.
41. Hiding `Any` in public interfaces.
42. Blanket `cast()` to silence type failures.
43. `object`/`dict[str, Any]` for everything.
44. Ignoring nullability/optional handling.
45. Letting generated code bypass lint/security gates.
46. Destructive migrations without backup/rollback plan.
47. Data migrations that cannot be rerun safely.
48. Non-idempotent scripts.
49. No transaction around multi-step data changes.
50. Long locks on production tables without planning.
51. Adding nullable/constraint/index changes blindly on large tables.
52. Dropping columns/tables in the same deploy that stops writing them.
53. Using admin DB credentials in the app.
54. App connects as schema owner/root.
55. No separate read/write/admin users.
56. Hardcoded connection strings.
57. Logging full queries with sensitive parameters.
58. No connection timeout.
59. No pool limits.
60. Leaking connections/cursors.
61. Creating N+1 queries that become DoS.
62. “Fixing” authorization by filtering in Python after fetching all rows.
63. Shared mutable state across threads/tasks without locks or immutability.
64. Assuming the GIL makes compound operations safe.
65. Blocking calls inside async request handlers.
66. Fire-and-forget tasks with no ownership, cancellation, logging, or error handling.
67. Race-prone check-then-act filesystem or DB logic.
68. No idempotency keys for retried external operations.
69. No transaction isolation thought.
70. Global caches with no invalidation or bounds.
71. Background workers that can process the same job twice unsafely.
72. Distributed locks without expiry/fencing.
73. Unbounded task queues.
74. Swallowing cancellation.
75. Doing CPU-bound work in event loop.
76. Using multiprocessing with pickled untrusted objects.
77. No `pyproject.toml`/clear dependency declaration.
78. App depends on globally installed packages.
79. Works only with the developer’s venv.
80. No Python version pin/constraint.
81. No reproducible install instructions.
82. Mixing app code, scripts, notebooks, generated files, and secrets in one directory.
83. Running modules by path hacks instead of package entry points.
84. Shipping dev dependencies/tools in production images unnecessarily.
85. Running as root in containers when not required.
86. Writable application code directory in production.
87. No health checks.
88. No graceful shutdown.
89. No config validation at startup.
90. Silent fallback to insecure defaults when config is missing.
91. Auto-creating admin users/passwords in production.
92. Running migrations implicitly on app startup without controls.

## Never make outbound or inbound HTTP unsafe by default

1. No timeouts on `requests`, `httpx`, DB, Redis, S3, SMTP, or RPC calls.
2. Infinite retries.
3. Retrying non-idempotent operations blindly.
4. No circuit breaker/backoff.
5. No request body size limit.
6. No upload size limit.
7. No concurrency limit.
8. No rate limit on expensive endpoints.
9. No authentication on internal APIs because “only internal services call it.”
10. Trusting `X-Forwarded-For`, `Host`, or proxy headers without trusted proxy config.
11. Putting tokens in query strings.
12. CORS `*` with credentials.
13. CSRF disabled for cookie-authenticated state-changing requests.
14. Returning overly broad error details.

## Never parse untrusted XML casually

1. Parsing untrusted XML without understanding entity expansion, decompression bombs, external resources, parser version, and size limits.
2. Accepting XML uploads without strict schema, depth, and size limits.
3. Enabling DTDs or external entity resolution unless explicitly required and isolated.
4. Treating SAML, SOAP, SVG, DOCX/XLSX, or XML-in-zip as harmless.
5. Feeding XML from emails, webhooks, uploaded documents, or agents directly into business logic.

## Never process untrusted data without validation

1. Trusting request JSON shape.
2. Trusting CSV/Excel headers/types.
3. Trusting webhook payloads without signature validation.
4. Trusting queue messages because “internal.”
5. Trusting database values that originally came from users.
6. Trusting LLM/agent output as structured data without schema validation.
7. Using deny-lists for dangerous characters as the only defense.
8. Accepting unlimited string lengths, list sizes, nesting depth, numeric ranges, regex complexity, file counts, or object graphs.
9. Allowing unknown fields silently in security-sensitive APIs.
10. Mixing validation, coercion, authorization, and persistence in one blob of code.
11. Validating only in the frontend.
12. “Sanitizing” by stripping quotes.
13. Treating type hints as runtime validation.
14. No timeout on network calls, subprocesses, DB queries, locks, queues, or futures.
15. No request body limit.
16. No upload limit.
17. No decompressed-size limit.
18. No pagination.
19. Returning unbounded query results.
20. Loading entire files into memory unnecessarily.
21. Unbounded recursion.
22. Unbounded regex on attacker-controlled input.
23. Unbounded task/thread/process spawning.
24. Unbounded async gather over user-controlled lists.
25. Infinite retries or retry storms.
26. No backpressure on queues/streams.
27. No cancellation handling.
28. No rate limiting on expensive endpoints.
29. No cache limits/TTL.
30. User-controlled image/video/PDF processing without CPU/memory/time sandboxing.
31. Running model inference or data analysis on user files without quotas.

## Never run development/debug servers in production

1. Flask `debug=True` in production.
2. Django `DEBUG=True` on any public server.
3. `python -m http.server` for production.
4. Werkzeug dev server as production.
5. Auto-reloaders in production.
6. Public interactive debuggers.
7. Stack traces visible to users.
8. Django `ALLOWED_HOSTS = ["*"]` without explicit host validation.
9. Committed Django `SECRET_KEY`.
10. Missing secure cookie flags for session/auth cookies.
11. Missing HTTPS enforcement where auth/sensitive data is involved.
12. Disabling CSRF middleware to “make frontend work.”
13. Broad CORS to unblock local development and forgetting it.

## Never store passwords incorrectly

1. Plaintext passwords.
2. Reversible encrypted passwords for normal login.
3. Fast hashes like MD5/SHA1/SHA256 alone.
4. Unsalted hashes.
5. One global salt.
6. Homegrown password hashing.
7. Storing reset tokens as plaintext.
8. No rate limiting or lockout/backoff on login/reset.
9. Password reset links that never expire.
10. Logging password reset tokens.
11. Comparing secrets with normal equality where timing attacks matter.

## Never trust paths from users, archives, repos, or model output

1. Joining `base_dir / user_supplied_path` without resolving and verifying containment.
2. Allowing `../`, absolute paths, drive letters, UNC paths, symlinks, hardlinks, URL-encoded traversal, null bytes, Unicode confusables, or alternate separators.
3. Letting users choose filenames used on disk.
4. Serving files by raw path.
5. Deleting paths from user input.
6. Running cleanup scripts over paths generated by an agent without dry-run and containment checks.
7. Writing uploads into executable or served directories.
8. Trusting file extensions or MIME types.
9. Using uploaded filename as storage key.
10. Failing to scan or quarantine risky uploaded files.
11. Making files or directories `777`.
12. Relying on current working directory.

## Never trust the client for identity, role, ownership, or authorization

1. Taking `user_id`, `is_admin`, `role`, `tenant_id`, `organization_id`, or `owner_id` from a request body/query/header and trusting it.
2. Checking authentication but not object-level authorization.
3. “Admins only” enforced only in frontend JavaScript.
4. Hidden routes, obscured IDs, UUIDs, or “unguessable” URLs used as authorization.
5. Missing tenant isolation in queries.
6. Allowing users to access files by path or key without ownership checks.
7. Trusting JWT claims without verifying signature, issuer, audience, algorithm, expiration, and key rotation.
8. Accepting `alg=none` or algorithm confusion in JWT-like systems.
9. Using one service account for all users and skipping per-user authorization.
10. Letting an agent “simplify” auth by bypassing decorators/middleware/tests.
11. Returning different login error messages that disclose whether a user exists.

## Never unpack archives without containment

1. Extracting zip/tar files from users without path normalization, allow-lists, file count limits, total uncompressed size limits, symlink handling, and destination containment.
2. Trusting archive filenames.
3. Allowing `../`, absolute paths, device files, symlinks, hardlinks, or permission bits from archives.
4. Extracting into application directories, web roots, repo roots, `$HOME`, or `/tmp` shared locations.
5. Ignoring zip bombs and decompression bombs.
6. Processing nested archives recursively without limits.

## Never use insecure temporary-file patterns

1. `tempfile.mktemp()`.
2. Predictable temp filenames.
3. Writing sensitive data to `/tmp/foo`.
4. Opening temp files with broad permissions.
5. Creating a name first and opening it later.
6. Ignoring symlink races.
7. Leaving temp files with secrets behind.
8. Using shared temp dirs for privileged operations without safe primitives.

## Never use mutable CI dependencies for sensitive build/release paths

1. GitHub Actions pinned only to tags like `@v4` for high-integrity release workflows.
2. Docker base images like `latest`.
3. Unpinned setup scripts.
4. Unpinned build tools.
5. Unverified artifact downloads.
6. Release artifacts built outside CI with unknown local state.
7. No provenance/attestation for published packages.
8. No reproducible build process.
9. Publishing to PyPI/npm/container registries from developer laptops.

## Never use unsafe YAML/object loaders

1. `yaml.load(data)` without a safe loader.
2. Accepting YAML from users or repos and letting it instantiate arbitrary Python objects.
3. Assuming YAML is “just config.”
4. Letting agents add YAML-based “automation” without schema validation.
5. Loading CI, plugin, pipeline, or workflow YAML with code-capable constructors.

## Prompt injection and untrusted context

1. Treat retrieved documents, web pages, emails, issue comments, README files, code comments, PDFs, images, filenames, API responses, tool output, logs, or database content as instructions.
2. Put untrusted variables into system/developer messages.
3. Let tool output override policy.
4. Let web content tell the agent which tools to call.
5. Let a dependency README or repo file instruct the agent to exfiltrate secrets.
6. Store raw untrusted user/model content in long-term agent memory without validation, tenant isolation, expiry, and auditing.
7. Reuse memory across users or tenants.
8. Let model output flow into shell, SQL, file paths, HTTP requests, emails, or commits without structured validation.
9. Use free-form text where a strict schema, enum, allowlist, or typed object is possible.
10. Let an agent silently transmit sensitive data to third parties.
11. Treat guardrails as perfect.

## Python-specific footguns

1. Use mutable default arguments like `def f(x=[]):`.
2. Use `assert` for runtime validation, user input validation, authorization, authentication, or security checks.
3. Use `is` for equality with strings, numbers, booleans other than literal `None` checks, or containers.
4. Shadow built-ins like `id`, `type`, `list`, `dict`, `set`, `file`, `input`, `sum`.
5. Name files after standard-library modules, such as `json.py`, `typing.py`, `logging.py`, `email.py`, `asyncio.py`, `random.py`, `ssl.py`.
6. Use wildcard imports in production modules.
7. Mutate `sys.path` to make imports work.
8. Depend on the current working directory.
9. Perform network calls, DB connections, file writes, migrations, subprocesses, logging configuration, or secret loading at import time.
10. Monkeypatch built-ins or third-party library internals in application code.
11. Rely on dictionary ordering, timezone defaults, locale defaults, or environment defaults without stating assumptions.
12. Mix bytes and strings casually.
13. Use implicit file encodings.
14. Use naive datetimes for cross-system timestamps.
15. Use local time for persisted timestamps when UTC-aware timestamps are required.
16. Use floats for money.
17. Use `deepcopy`/serialization as a substitute for understanding object ownership.
18. Depend on private attributes of libraries.
19. Ignore deprecation warnings forever.

## Resource management

1. Open files, sockets, DB connections, HTTP sessions, temp dirs, locks, or subprocess pipes without deterministic cleanup.
2. Skip context managers for resources that need closing.
3. Make HTTP requests without timeouts.
4. Read unbounded files into memory.
5. Accept unbounded request bodies.
6. Accept unbounded CSV rows, JSON depth, XML entities, archive entries, image dimensions, or regex input.
7. Use regexes vulnerable to catastrophic backtracking on uncontrolled input.
8. Spawn unbounded threads, processes, tasks, greenlets, or subprocesses.
9. Use queues without max sizes.
10. Use caches without size/TTL/tenant isolation.
11. Use recursion on attacker-controlled depth.
12. Build infinite agent/retry/poll loops.
13. Ignore OS signals and graceful shutdown for services.
14. Leave zombie subprocesses.

## SQL and data-store injection

1. Build SQL with f-strings, `%`, `.format()`, concatenation, or agent-generated query strings.
2. Use string interpolation for table names, column names, sort keys, filters, or `WHERE` clauses.
3. Assume ORM usage automatically prevents injection when using raw SQL/text fragments.
4. Pass untrusted values into `ORDER BY`, `LIMIT`, `OFFSET`, JSON path, SQL functions, or `LIKE` patterns without validation.
5. Build MongoDB, Redis, Elasticsearch, LDAP, XPath, GraphQL, or Cypher queries by string concatenation.
6. Disable ORM escaping or bind parameters to “make debugging easier.”
7. Let an LLM “optimize” a parameterized query into raw string SQL.
8. Treat admin-only query boxes as safe if they run in production.
9. Log full SQL with secrets, tokens, PII, or credentials.

## SQL, NoSQL, LDAP, template, and query injection

1. Build SQL with f-strings, concatenation, `.format()`, `%`, or templating.
2. Put user input into `WHERE`, `ORDER BY`, `LIMIT`, table names, column names, JSON paths, raw ORM clauses, or stored-procedure calls without strict allowlisting/parameterization.
3. Use “escaping” as the primary defense.
4. Let an LLM write raw SQL that runs against production or privileged data.
5. Trust ORM filters constructed directly from request dictionaries.
6. Build Mongo/NoSQL queries from unvalidated dictionaries that can contain operators like `$where`, `$ne`, `$regex`, `$gt`.
7. Build LDAP filters, XPath, GraphQL, Elasticsearch, Redis, or PromQL queries with string concatenation.
8. Treat “admin-only” input as safe.

## Secrets and credentials

1. Hardcode API keys, passwords, private keys, JWT secrets, OAuth secrets, database URLs, signing keys, SSH keys, cloud credentials, service-account tokens, or webhook secrets.
2. Commit `.env`, `.pypirc`, `.netrc`, kubeconfig, Terraform state, AWS/GCP/Azure credential files, `.pem`, `.key`, `.p12`, or local config with secrets.
3. Put secrets in prompts, model context, issue comments, PR descriptions, code snippets, screenshots, notebooks, fixtures, test data, or logs.
4. Print secrets during debugging.
5. Store secrets in Docker images, baked config files, frontend bundles, mobile apps, or public package metadata.
6. Use one shared secret across dev/staging/prod.
7. Use long-lived production credentials for local development or agents.
8. Give CI/CD jobs broad secrets by default.
9. Ignore secret-scanning alerts.
10. Remove leaked secrets from Git history but fail to rotate/revoke them.
11. Let an agent read `.env`, key files, browser cookies, SSH config, cloud profiles, password-manager exports, or clipboard contents unless that is explicitly scoped and audited.

## Security

1. Disable TLS verification with `verify=False`, `_create_unverified_context()`, `CERT_NONE`, or `check_hostname=False`.
2. Suppress TLS warnings instead of fixing TLS.
3. Use HTTP for credentials, sessions, API calls, webhooks, package downloads, or admin traffic.
4. Use MD5 or SHA1 for security-sensitive integrity, signatures, password storage, or deduplication where collision resistance matters.
5. Invent encryption, signing, token, or password-reset schemes.
6. Use AES-ECB.
7. Reuse nonces/IVs with modes that require uniqueness.
8. Encrypt without authentication.
9. Sign without covering all security-critical fields.
10. Accept JWTs with `alg=none`, disabled signature verification, missing `exp`, missing `aud`/`iss` checks, or confused symmetric/asymmetric algorithms.
11. Store JWTs forever with no revocation strategy for sensitive systems.
12. Use predictable password reset, invite, email verification, magic link, or MFA codes.
13. Ignore replay protection for webhooks and signed requests.
14. Hardcode cryptographic keys in source or config.
15. Roll your own certificate validation.
16. Treat base64 as encryption.
17. Use the `cryptography.hazmat` layer casually because an agent suggested it.
18. Use weak randomness for salts, nonces, or session IDs.
19. Use `verify=False` in `requests`/HTTP clients outside a one-off local diagnostic.
20. Set `ssl.CERT_NONE` or disable hostname verification for real traffic.
21. Suppress TLS warnings instead of fixing trust configuration.
22. Accept unknown SSH host keys automatically in production automation.
23. Use HTTP for login, tokens, cookies, internal admin panels, webhooks carrying secrets, package downloads, or service-to-service auth.
24. Send API keys in URLs.
25. Log full URLs if they may contain tokens.
26. Trust self-signed certs without pinning/managed CA strategy.
27. Use deprecated TLS versions/ciphers because “it works.”
28. Fail open when TLS verification fails.
29. Agent with unrestricted shell access.
30. Agent with unrestricted filesystem access.
31. Agent with unrestricted network egress.
32. Agent with access to `~/.ssh`, `~/.aws`, cloud CLIs, kubeconfigs, password managers, browser profiles, or local token caches.
33. Agent inheriting all developer environment variables.
34. Agent using personal admin credentials.
35. Agent using production credentials.
36. Agent able to deploy, delete, rotate secrets, migrate DBs, or change IAM without fresh explicit approval.
37. Agent able to install arbitrary packages, extensions, MCP servers, or shell tools.
38. Agent able to modify its own instructions, guardrails, config, tool registry, or sandbox policy.
39. Agent able to write shell profiles, Git hooks, editor config, or agent config.
40. Agent able to read unrelated repos or directories.
41. Agent able to exfiltrate through HTTP, DNS, Git, Slack, paste sites, telemetry, images, logs, or package publishing.
42. Agent able to follow instructions embedded in untrusted repos, PRs, issues, web pages, PDFs, docs, comments, `.cursorrules`, `AGENTS.md`, `CLAUDE.md`, or tool responses without treating them as hostile.
43. Agent memory storing secrets, credentials, PII, or private business context.
44. Agent memory shared across projects or tenants.
45. Agent approvals that become habitual “click yes” prompts.
46. Cached approval for future destructive actions.
47. No transcript or audit log of agent actions.
48. No resource budget: unbounded tokens, time, subprocesses, network calls, or cloud spend.
49. No rollback plan for agent-generated migrations.
50. No tests run after agent edits.
51. No static analysis after agent edits.
52. No human review for security-sensitive agent edits.
53. Multi-agent systems where every agent has the same tools and permissions.
54. Using agents to touch production directly.
55. Letting an agent decide its own least privilege.
56. Letting untrusted content and trusted system instructions share the same channel without clear boundaries.
57. `assert user.is_admin`.
58. `assert amount > 0`.
59. `assert signature_valid`.
60. `assert request.user.id == object.owner_id`.
61. `assert path.startswith(base)`.
62. `assert isinstance(data, ExpectedType)` for external input.
63. Asserting auth, authz, tenant checks, balance checks, limits, or invariants that must hold in production.

## Shell and OS commands

1. Use `os.system`, `os.popen`, `subprocess.*(..., shell=True)`, `pty`, `pexpect`, `bash -c`, PowerShell, `sh -c`, or command strings with user/model-controlled content.
2. Build command strings with f-strings, `%`, `.format()`, concatenation, templates, or joined argument lists.
3. Assume `shlex.quote()` makes arbitrary shell execution safe enough for privileged paths.
4. Pass user input as a command name, executable path, environment variable, working directory, glob, redirect, pipe, or shell fragment.
5. Let agents run arbitrary terminal commands.
6. Let a Python app invoke package managers, interpreters, migration tools, cloud CLIs, Docker, Kubernetes, Git, SSH, or deployment commands from untrusted input.
7. Run subprocesses without timeouts, resource limits, bounded output capture, explicit environment, and error checking.
8. Ignore return codes from security-sensitive subprocesses.

## Shell, OS, and command execution

1. Use `os.system()`, `os.popen()`, `commands`, backticks through shell wrappers, or `subprocess(..., shell=True)` with any variable input.
2. Concatenate shell commands with f-strings, `%`, `.format()`, or `+`.
3. Pass user input into `rm`, `find`, `tar`, `ssh`, `scp`, `curl`, `git`, `pip`, `docker`, `kubectl`, `terraform`, or cloud CLIs.
4. Trust `PATH` for privileged subprocesses.
5. Ignore subprocess return codes.
6. Hide subprocess stderr because it is noisy.
7. Use relative executable names in privileged or production automation.
8. Use `preexec_fn` for security-sensitive setup.
9. Drop only `user=` in `subprocess` while forgetting groups, supplementary groups, `cwd`, `PATH`, environment, umask, and file descriptors.
10. Let agents run package installation, shell commands, migration scripts, or deployment commands without a sandbox and allowlist.

## Template, HTML, and web injection

1. Disable Jinja2/Django autoescaping for user-controlled output.
2. Mark user input as `safe`, `Markup`, `|safe`, or equivalent.
3. Use `render_template_string()` with user-controlled template content.
4. Let users or agents supply template expressions.
5. Concatenate HTML with unescaped variables.
6. Insert untrusted values into JavaScript, CSS, URLs, or HTML attributes without context-specific encoding.
7. Trust Markdown rendering without sanitization.
8. Return user-controlled error messages as HTML.
9. Expose stack traces to users.
10. Treat Content Security Policy as a replacement for output encoding.
11. Build XML/HTML with string concatenation when safer builders/templates exist.

## Templates generally

1. Disable escaping globally.
2. Mark user input safe.
3. Allow user-controlled template names without allowlists.
4. Render user-provided templates.
5. Render Markdown/HTML without sanitization.
6. Use server-side template engines as user-facing customization engines without sandboxing.
7. Run untrusted notebooks.
8. Run notebooks with production secrets.
9. Commit notebooks containing outputs with secrets or PII.
10. Commit notebooks with hidden state required for correctness.
11. Treat a notebook as production code without conversion, review, tests, and dependency locking.
12. Load random `.pkl`, `.joblib`, `.pt`, `.pth`, `.npy`, `.npz`, `.h5`, or model artifacts.
13. Use `torch.load` on untrusted checkpoints.
14. Use `joblib.load` on untrusted model files.
15. Use scikit-learn pickled models from untrusted sources.
16. Use `numpy.load(..., allow_pickle=True)` on untrusted files.
17. Use `pandas.eval()` or `DataFrame.query()` on user-provided expressions.
18. Trust CSV/Excel formulas from uploads.
19. Export CSVs that can trigger spreadsheet formula injection without neutralization.
20. Load huge datasets without quotas.
21. Download datasets from generated URLs without provenance checks.
22. Train on production PII without governance.
23. Send private datasets to external LLMs or APIs without approval.
24. Log sample rows containing PII.
25. Use synthetic-looking data that is actually copied from production.
26. Store model prompts/completions containing secrets.
27. Give agents notebook kernels connected to real credentials.
28. Let agents install arbitrary ML packages in environments with secrets.
29. Treat model output as facts, code, SQL, file paths, or commands without validation.
30. Parse untrusted XML with unsafe defaults.
31. Allow external entities.
32. Allow DTDs unless explicitly required and safely configured.
33. Ignore “billion laughs” and entity expansion attacks.
34. Treat HTML sanitization as simple string replacement.
35. Strip `<script>` and call it safe.
36. Trust Markdown renderers to sanitize HTML by default.
37. Trust uploaded SVGs as safe images.
38. Trust uploaded Office/PDF files as inert.
39. Trust image metadata.
40. Trust EXIF GPS metadata not to leak privacy.
41. Trust CSV cells not to execute formulas in spreadsheets.
42. Generate CSV/Excel exports without neutralizing formula injection where files may be opened by users.
43. Parse user-supplied regex, glob, XPath, CSS selectors, or template expressions without constraints.
44. Treat environment variables as trusted when an attacker can influence runtime.
45. Put secrets in default config files.
46. Use insecure defaults.
47. Fail open when config is missing.
48. Silently ignore malformed config.
49. Merge config from multiple sources without precedence rules.
50. Let user-controlled config choose classes, modules, functions, commands, or file paths without allowlists.
51. Let config enable debug mode in production.
52. Let config disable auth.
53. Let config disable TLS verification.
54. Let config disable logging/audit.
55. Let config change database targets without environment guardrails.
56. Let config point to arbitrary plugin paths.
57. Let config load YAML unsafely.
58. Let config use Python expressions.
59. Let config interpolate shell commands.
60. Print full config if it may contain secrets.
61. Store per-environment secrets in repo.
62. Use the same config for local, CI, staging, and production without explicit separation.
63. Run generated destructive migrations without review.
64. Run migrations against production from a developer laptop.
65. Run migrations without backups/rollback strategy where needed.
66. Run data migrations without idempotency.
67. Run data migrations without batching for large tables.
68. Run data migrations that lock critical tables without review.
69. Log full rows containing PII during migrations.
70. Use production snapshots locally without privacy controls.
71. Build migration SQL from untrusted strings.
72. Use `DROP`, `TRUNCATE`, or broad `DELETE` without explicit targeting and review.
73. Omit `WHERE` safeguards in generated scripts.
74. Use autocommit carelessly for multi-step mutations.
75. Ignore transaction boundaries.
76. Reuse one database user with admin privileges for all app operations.
77. Give read-only code write credentials.
78. Give background jobs broader DB permissions than needed.
79. Give agents direct write access to production databases.
80. Store credentials in migration files.
81. Mix tenants in migration scripts without tenant scoping.
82. Backfill sensitive data into logs, queues, or analytics accidentally.
83. Run containers as root by default when not required.
84. Bake secrets into images.
85. Pass secrets as Docker build args.
86. Leave secrets in image layers.
87. Use `latest` tags for production base images.
88. Ignore base-image vulnerabilities.
89. Install compilers, shells, package managers, curl, or debugging tools in final runtime images when unnecessary.
90. Disable certificate stores.
91. Disable OS package signature checks.
92. Use `chmod -R 777`.
93. Mount the Docker socket into app containers casually.
94. Mount host root directories into containers.
95. Run privileged containers without a specific reviewed need.
96. Use host networking casually.
97. Give app pods broad Kubernetes service-account permissions.
98. Give every pod access to every secret.
99. Use the same service account for app, CI, and deploy.
100. Let agents run Docker with host mounts and secrets.
101. Let generated Dockerfiles download and execute arbitrary scripts.
102. Run migrations automatically on every container start without locking and safety controls.
103. Treat container isolation as a complete security boundary without understanding its limits.
104. Use world-writable files for config, logs, sockets, caches, or secrets.
105. Use world-readable private keys.
106. Use `chmod 777`.
107. Use `chmod 666`.
108. Use broad recursive chmod/chown from generated code.
109. Create files without considering umask.
110. Create secret files without restrictive permissions.
111. Run services as root unnecessarily.
112. Use setuid/setgid behavior casually.
113. Trust `$PATH`.
114. Trust `$PYTHONPATH`.
115. Trust current directory executables.
116. Trust writable plugin directories.
117. Trust files in `/tmp` by name.
118. Trust lock files in world-writable directories without safe creation semantics.
119. Follow symlinks during privileged file operations.
120. Write logs to locations where lower-privileged users can inject content.
121. Read files based on user-supplied paths with elevated privileges.
122. Surprises callers with network calls.
123. Surprises callers with file writes.
124. Surprises callers with subprocesses.
125. Mutates inputs unexpectedly.
126. Returns different types for normal cases.
127. Uses `None` ambiguously for “missing,” “denied,” “failed,” and “empty.”
128. Uses booleans to mean many different things.
129. Hides errors instead of returning structured results or raising clear exceptions.
130. Requires callers to remember security-sensitive ordering.
131. Defaults to insecure behavior.
132. Makes secure behavior optional and inconvenient.
133. Exposes foot-gun parameters like `verify=False`, `unsafe=True`, `shell=True`, `skip_auth=True`, or `allow_pickle=True` without strong isolation.
134. Names dangerous options innocently.
135. Lets callers pass arbitrary callables that run in privileged contexts.
136. Lets callers pass arbitrary classes/modules by string.
137. Uses callbacks without documenting execution context and trust boundary.
138. Leaks implementation details into public interfaces.
139. Breaks backward compatibility silently in security-sensitive behavior.
140. Collect data you do not need.
141. Store data you do not need.
142. Retain data forever by default.
143. Use production PII in local development.
144. Use production PII in demos.
145. Use production PII in tests.
146. Use production PII in generated prompts.
147. Send private data to third-party APIs without approval.
148. Send private data to LLMs without approval.
149. Include private data in traces.
150. Include private data in metrics labels.
151. Include private data in analytics events without minimization.
152. Include private data in crash reports.
153. Include private data in support bundles.
154. Include private data in sample fixtures.
155. Let agents browse private data broadly.
156. Let agents summarize private data into persistent memory.
157. Export user data without authentication, authorization, rate limits, and audit logs.
158. Build admin bulk export tools without auditability.
159. Assume anonymization is easy.
160. Hash emails/phones and call them anonymous without threat modeling.
161. Store consent, deletion, or retention state in an unreliable side channel.
162. “It is just internal.”
163. “It is behind VPN.”
164. “Only admins can use it.”
165. “The URL is unguessable.”
166. “The bucket is private.”
167. “The repo is private.”
168. “The model generated it.”
169. “The agent tested it.”
170. “It is temporary.”
171. “We will rotate the secret later.”
172. “We will add auth later.”
173. “We will add validation later.”
174. “We will add tests later.”
175. “We need to move fast.”
176. “The scanner is noisy.”
177. “The user input is probably safe.”
178. “No one knows this endpoint exists.”
179. “It is only staging.”
180. “The data is not that sensitive.”
181. “The shell command is simple.”
182. “The pickle file came from our team.”
183. “The certificate failed, so I set `verify=False`.”
184. “The dependency is popular, so it is safe.”
185. “The code is too generated to review.”
186. “The diff is too large to review.”
187. “The agent needs broad permissions to be useful.”
188. Does it use `eval`, `exec`, dynamic imports, or dynamic attribute dispatch on untrusted names?
189. Does it use `pickle`, `dill`, `cloudpickle`, `shelve`, `marshal`, `joblib.load`, `torch.load`, or `numpy.load(..., allow_pickle=True)` on untrusted artifacts?
190. Does it use `shell=True`, `os.system`, or string-built subprocess commands?
191. Does it disable TLS/cert/hostname/SSH host-key verification?
192. Does it use `random` for security tokens?
193. Does it use MD5/SHA-1/password hashing incorrectly?
194. Does it hardcode or log secrets?
195. Does it accept user paths, URLs, templates, regexes, SQL, expressions, or filenames without allowlists and validation?
196. Does it extract archives without traversal, symlink, size, and overwrite protections?
197. Does it parse untrusted XML/YAML/HTML/Markdown unsafely?
198. Does it miss network timeouts or resource limits?
199. Does it swallow exceptions or fail open?
200. Does it use `assert` for security/runtime validation?
201. Does it ship debug mode or dev servers?
202. Does it weaken auth, CSRF, CORS, session, or tenant isolation?
203. Does it add dependencies without pinning, provenance, and vulnerability review?
204. Does it suppress linters/security scanners/type checkers broadly?
205. Does it remove or skip tests to pass CI?
206. Does it expose secrets to CI, tests, notebooks, logs, or agents?
207. Does it let an agent run with broad shell/filesystem/network/credential access?
208. Does it perform import-time side effects?
209. Does it rely on hidden local state or the developer’s machine?
210. Does it make destructive changes without dry-run, review, idempotency, and rollback?
211. Does it make a giant generated diff no human can understand?
212. Does it contain “temporary” insecure behavior?

## Temporary files

1. Use `tempfile.mktemp()`.
2. Generate predictable temp names.
3. Create temp files in world-writable directories without atomic creation.
4. Close/delete a secure temp file and then reopen the same path.
5. Pass temp filenames to external programs in a way that creates race windows.
6. Use temp files for secrets unless permissions, lifecycle, cleanup, and crash behavior are controlled.
7. Leave temp directories/files behind with sensitive contents.

## Testing

1. **Using `eval`, `exec`, `compile`, dynamic imports, or reflection on untrusted strings.** Python’s own docs warn that dynamic execution can execute arbitrary code.
2. **Unpickling untrusted data** with `pickle`, `dill`, `shelve`, `joblib`, `pandas.read_pickle`, model checkpoints, or pickle-backed loaders. Python explicitly says pickle is not secure and malicious pickle data can execute arbitrary code.
3. **Unsafe YAML loading** such as `yaml.load()` without a safe loader, because it can construct arbitrary Python objects.
4. **Building SQL, shell commands, HTML, templates, LDAP, XPath, or NoSQL queries with string concatenation or f-strings.** OWASP’s primary SQL injection defense is parameterized queries, and its command-injection guidance says to avoid OS command execution where possible.
5. **Using `subprocess(..., shell=True)`, `os.system`, or shell pipelines with variables or untrusted input.** Python’s subprocess docs direct readers to security considerations before using `shell=True`; community discussions repeatedly flag `shell=True` as a common avoidable risk.
6. **Hardcoding secrets**: passwords, API keys, OAuth tokens, private keys, database URLs, signing keys, cloud credentials, JWT secrets. OWASP calls hardcoded credentials a persistent risk, and GitHub/GitLab both treat exposed secrets as needing immediate revocation.
7. **Logging secrets or sensitive data**: auth headers, cookies, tokens, passwords, private keys, PII, raw request bodies, environment dumps, config dumps, source code, stack traces with credentials. OWASP’s logging guidance says private or secret information should never be logged.
8. **Disabling TLS verification** with `verify=False`, `_create_unverified_context`, ignored certificate warnings, or custom “trust all” SSL logic. Bandit flags unverified SSL context creation as a security issue.
9. **Using weak or homegrown cryptography**: MD5/SHA1 for security, DES, RC4, Blowfish, ECB mode, predictable IVs, homegrown encryption, custom password hashing. Bandit explicitly flags weak hashes, ciphers, and ECB mode.
10. **Using `random` for secrets, tokens, passwords, reset links, session IDs, or crypto keys.** Python provides `secrets` for cryptographically strong randomness; Bandit flags standard pseudo-random generators as unsuitable for security-sensitive use.
11. **Storing passwords in plaintext, reversible encryption, unsalted hashes, or weak password hashing.** OWASP recommends modern password hashing such as Argon2id, bcrypt, or PBKDF2 with salts.
12. **Running Flask/Django debug tooling in production.** Flask warns not to run the development server or debugger in production because the debugger can execute arbitrary Python code from the browser; Django’s deployment checks flag `DEBUG=True` and weak security settings.
13. **Trusting client-side validation, client-supplied roles, hidden fields, cookies, user IDs, or JWT claims without server-side authorization.**
14. **Failing open**: letting users in when auth, validation, network calls, policy checks, crypto checks, or permission checks error.
15. **Swallowing exceptions with `except: pass` around security, payment, auth, file, network, or data-integrity logic.**
16. **Using `assert` for runtime validation, auth checks, permission checks, or input validation.** Assertions can be stripped with optimized execution.
17. **Installing hallucinated, unpinned, unknown, typosquatted, or random internet packages.** PyPA recommends repeatable installs and pinning for reproducibility, and tools like `pip-audit` check known vulnerable dependencies.
18. **Running code fetched from the internet**, including `curl | python`, copied README snippets, unknown GitHub gists, setup scripts, model loaders, or notebooks.
19. **Ignoring failing security/lint/type/test/secret-scan CI** with “it works on my machine” or blanket suppressions.
20. **Letting an AI agent modify, run, deploy, migrate, or delete things without sandboxing, least privilege, tests, diffs, and human review for high-impact actions.** OWASP’s AI agent and LLM risks include prompt injection, tool abuse, data exfiltration, insecure output handling, supply-chain issues, excessive agency, and overreliance.
21. `eval(user_input)`
22. `exec(user_input)`
23. `compile(user_input, ...)`
24. `exec(open(path).read())`
25. `runpy.run_path()` on untrusted or mutable paths
26. exposing a Python REPL, notebook kernel, debugger, or admin console to users
27. dynamically importing modules based on user/LLM/config input without a strict allowlist
28. `globals()[name]`, `locals()[name]`, or `getattr(obj, name)` dispatch from untrusted strings
29. plugin loading from the current working directory, upload directories, temp directories, or writable paths
30. adding untrusted directories to `sys.path` or `PYTHONPATH`
31. executing generated Python code because “the agent wrote it”
32. using `ast.literal_eval` as a general safe parser for untrusted data; Python notes it can still cause memory or C-stack exhaustion on small malicious inputs.
33. `pickle.load()` or `pickle.loads()` on files, uploads, cache entries, cookies, messages, database blobs, ML artifacts, or network data
34. `dill`, `cloudpickle`, `shelve`, `marshal`, `joblib`, or pickle-backed model/data loading from untrusted sources
35. `pandas.read_pickle()` on user-supplied or downloaded files
36. loading ML checkpoints, model files, or artifacts that can execute Python without provenance and isolation
37. `yaml.load()` without `SafeLoader`
38. custom JSON `object_hook` or decoder logic that instantiates arbitrary classes
39. accepting serialized class names, module names, function names, or import paths from users
40. trusting signed serialized data merely because it is signed; signing proves source/integrity, not safety
41. parsing untrusted XML with unsafe defaults, entity expansion, external entities, or DTDs
42. using unsafe template rendering from user-supplied template strings
43. loading config from writable locations without ownership and permission checks
44. treating `.env`, YAML, TOML, JSON, notebook, or CLI config as trusted when it comes from an agent, user, PR, uploaded file, or third-party repo
45. SQL via f-strings:
46. SQL via string concatenation:
47. shell commands built from strings:
48. NoSQL filters containing raw user-controlled dictionaries without schema checks
49. Mongo-style operator injection such as accepting `{"$ne": null}` from users
50. LDAP filters built from strings
51. XPath expressions built from strings
52. GraphQL queries built by concatenating user input
53. Jinja/Django template strings from users
54. regex patterns from users without allowlists, length limits, and ReDoS protections
55. logging raw user input where line breaks or delimiters can forge log entries
56. HTML assembled manually instead of escaped templating
57. JSON embedded into HTML/script contexts without context-aware escaping
58. command flags assembled from user strings
59. filenames, URLs, or headers inserted into shell commands
60. “sanitizing” with ad hoc `.replace()` calls instead of parameterization or context-aware escaping
61. HTTP requests
62. CLI arguments
63. environment variables
64. config files
65. database records written by another service
66. queue messages
67. webhooks
68. uploaded files
69. JWTs
70. hidden form fields
71. LLM outputs
72. agent tool results
73. scraped web pages
74. GitHub/GitLab issue comments
75. PR descriptions
76. Slack/Discord/email content
77. model outputs
78. cached values
79. feature flags editable by non-admins
80. only validating in the browser
81. using denylist validation as the only defense
82. accepting arbitrary keys in JSON request bodies
83. accepting arbitrary nested structures with no schema
84. failing to bound string length, list length, file size, recursion depth, or request body size
85. accepting unknown enum values and silently defaulting
86. coercing invalid input into something “close enough”
87. accepting dates, time zones, money, paths, IDs, or permissions without strict parsing
88. trusting MIME type, extension, or `Content-Type` alone
89. trusting a user-supplied `user_id`, `account_id`, `tenant_id`, `role`, `is_admin`, or `organization_id`
90. trusting LLM-generated JSON without validation
91. continuing after validation errors
92. checking only “is logged in” and not “is allowed to access this object”
93. trusting `user_id` from the request instead of deriving it from the authenticated principal
94. direct object reference bugs: `/invoice/123` accessible to another user
95. tenant isolation based only on client-supplied tenant ID
96. admin routes protected only by hidden URLs
97. role checks performed only in frontend code
98. missing authorization on background jobs, export endpoints, internal APIs, GraphQL resolvers, or batch endpoints
99. using predictable reset tokens, invite links, API keys, or session IDs
100. no expiration on tokens
101. no rotation on compromised credentials
102. no revocation path
103. storing session state in unsigned or weakly signed cookies
104. weak JWT secrets
105. accepting `alg=none` or failing to verify JWT issuer, audience, expiry, and signature
106. storing passwords, tokens, or private keys in localStorage for browser apps
107. mixing production and development auth secrets
108. using default credentials
109. using shared admin accounts
110. bypass flags such as `DISABLE_AUTH=true` reachable in production
111. fail-open behavior when auth provider, database, or permission service is unavailable
112. hardcoded API keys
113. hardcoded passwords
114. hardcoded database URLs
115. hardcoded JWT signing keys
116. private keys committed to repos
117. secrets in notebooks
118. secrets in tests
119. secrets in fixtures
120. secrets in `.env.example`
121. secrets in screenshots
122. secrets in comments
123. secrets in docs
124. secrets in stack traces
125. secrets in CI YAML
126. secrets in Dockerfiles
127. secrets baked into container images
128. secrets passed as command-line arguments
129. secrets stored in shell history
130. secrets in cache/artifact uploads
131. secrets in frontend bundles
132. secrets in mobile/desktop client code
133. secrets copied into LLM prompts
134. production secrets used in local dev
135. long-lived PyPI/API/cloud tokens when short-lived or OIDC/trusted publishing is available
136. merely deleting a leaked secret instead of revoking and rotating it
137. logging passwords
138. logging API keys
139. logging bearer tokens
140. logging cookies
141. logging session IDs
142. logging private keys
143. logging OAuth authorization codes
144. logging password reset links
145. logging full request/response bodies for sensitive endpoints
146. logging raw authorization headers
147. logging config dumps
148. logging database connection strings
149. logging PII by default
150. logging payment data
151. logging medical, legal, financial, or personal data without explicit policy
152. logging user-uploaded file contents
153. logging entire objects whose `repr()` contains secrets
154. returning raw tracebacks to users
155. exposing stack traces in API errors
156. using user input directly in logs without neutralizing newlines/control characters
157. shipping debug logs to third-party tools without data classification
158. storing logs forever
159. letting agents read logs containing secrets or user data without need-to-know access
160. custom encryption algorithms
161. custom token signing
162. custom JWT implementation
163. MD5/SHA1 for signatures, password storage, integrity in adversarial contexts, or security decisions
164. DES, RC4, Blowfish, or obsolete algorithms
165. AES-ECB
166. static IVs/nonces
167. nonce reuse
168. predictable salts
169. using `random`, timestamps, incremental counters, process IDs, or UUID1 for secrets
170. comparing secrets with `==` where timing attacks matter instead of constant-time comparison
171. storing encryption keys next to encrypted data
172. using the same key for dev, staging, and prod
173. suppressing crypto warnings
174. accepting any certificate
175. silently falling back to plaintext
176. using expired certificates knowingly
177. implementing “encryption” with base64, ROT13, XOR, or compression
178. claiming hashing is anonymization for sensitive data
179. using reversible encryption for passwords
180. `os.system()`
181. `subprocess.run("...", shell=True)` with variables
182. shelling out for things the standard library can do
183. passing one big string instead of an argv list
184. interpolating filenames, URLs, headers, branches, commit messages, or user input into commands
185. no timeout on subprocess calls
186. no maximum output size
187. no working-directory control
188. inheriting the full environment into child processes
189. passing secrets as command-line arguments
190. running commands as root when not required
191. using `sudo` in application code
192. relying on shell aliases, globbing, `$PATH`, or current directory
193. executing scripts from a writable directory
194. letting agents run destructive commands without explicit allowlisting
195. letting user input choose the executable path
196. ignoring return codes
197. hiding stderr/stdout when diagnosing failures
198. continuing after failed commands
199. using shell pipelines for archive extraction, file conversion, or parsing untrusted files
200. using `rm -rf`, `chmod -R 777`, or broad recursive operations in application paths
201. path traversal: `../../etc/passwd`
202. joining user input directly into paths
203. trusting `Path.resolve()` without checking containment inside an allowed base directory
204. writing files with user-controlled names into executable/importable directories
205. reading arbitrary paths from query parameters
206. deleting arbitrary paths from query parameters
207. archive extraction without checking for traversal, absolute paths, symlinks, and file count/size limits
208. trusting file extensions
209. trusting MIME type from the client
210. storing uploads under web-executable paths
211. using predictable temp names
212. using `tempfile.mktemp()`
213. creating temp files in shared directories insecurely
214. world-readable secret files
215. world-writable application directories
216. `chmod 777`
217. following symlinks unintentionally
218. race-prone “check then open” logic
219. writing credentials to disk without permissions checks
220. importing Python modules from directories writable by users or build steps
221. destructive operations at import time
222. destructive operations based on relative paths
223. accepting a user-provided URL and calling `requests.get(url)` without allowlists
224. allowing internal IPs, localhost, metadata services, private ranges, link-local addresses, or cloud metadata endpoints
225. allowing dangerous schemes such as `file://`, `ftp://`, `gopher://`, or unexpected redirects
226. following redirects from allowed domains to forbidden destinations
227. DNS rebinding exposure
228. no maximum response size
229. no content-type validation
230. no decompression bomb protection
231. no rate limits
232. no retries/backoff policy
233. retry storms
234. no circuit breaker for downstream outages
235. sending credentials to user-controlled domains
236. forwarding internal headers to external URLs
237. using GET for state-changing actions
238. ignoring TLS failures
239. disabling certificate verification
240. trusting IP allowlists from user-controlled headers like `X-Forwarded-For`
241. webhook signatures checked after parsing huge bodies
242. exposing local dev servers publicly
243. exposing admin/debug endpoints publicly
244. Flask debug mode in production
245. Flask development server exposed publicly
246. Werkzeug debugger reachable from a browser
247. Django `DEBUG=True` in production
248. weak or default `SECRET_KEY`
249. wildcard `ALLOWED_HOSTS` in deployed Django apps without a carefully justified architecture
250. missing CSRF protection for cookie-authenticated state changes
251. insecure cookies: missing `Secure`, `HttpOnly`, or appropriate `SameSite`
252. session cookies over HTTP
253. wildcard CORS with credentials
254. returning raw exception pages
255. exposing admin panels without strong authentication
256. missing rate limits on login, reset, invite, OTP, and token endpoints
257. template autoescaping disabled
258. using `mark_safe` or equivalent on untrusted content
259. open redirects
260. file upload endpoints without validation
261. mass assignment from request JSON into models
262. object-level authorization missing from class-based views, serializers, resolvers, or route handlers
263. no security headers where appropriate
264. no request size limits
265. no timeout limits around slow backends
266. trusting proxy headers without trusted proxy configuration
267. unpinned dependencies in deployed apps
268. no lockfile or reproducible install process
269. broad version ranges with no testing
270. installing dependencies directly from random Git URLs
271. installing packages because an AI hallucinated an import name
272. ignoring typosquatting risk
273. ignoring dependency confusion risk
274. using `--extra-index-url` carelessly with private package names
275. running `setup.py`, install hooks, or build scripts from unknown packages
276. vendoring random code without provenance
277. no dependency vulnerability scanning
278. no SBOM or dependency inventory for serious systems
279. no review of transitive dependencies
280. leaving known-vulnerable dependencies because upgrading is annoying
281. pinning to abandoned packages forever
282. disabling hash checking where the project requires hashes
283. using package maintainers’ branch names instead of immutable versions/commits
284. running package manager commands as root in mutable production environments
285. shipping dev/test dependencies into production images
286. installing from the network during application startup
287. importing optional dependencies dynamically based on user input
288. copying StackOverflow/GitHub snippets into packages without license or security review
289. CI without tests
290. CI without linting
291. CI without security scanning
292. CI without secret scanning
293. CI without dependency scanning
294. ignoring failing CI
295. `# noqa`, `# nosec`, `# type: ignore`, or disabled checks with no explanation
296. unpinned GitHub Actions
297. third-party CI actions with broad permissions
298. workflows with default write-all permissions
299. secrets exposed to forked PRs
300. unsafe use of `pull_request_target`
301. self-hosted runners exposed to untrusted PRs
302. writing secrets into workflow files
303. printing secrets in CI logs
304. storing secrets in artifacts or caches
305. uploading `.env`, coverage, test DBs, logs, or build folders that contain secrets
306. deployment from unreviewed branches
307. agent-authored PRs auto-merged without human review
308. auto-fixing security findings without test evidence
309. release jobs that run arbitrary scripts from the repo without protection
310. publishing packages with long-lived tokens when trusted publishing is available
311. no provenance for release artifacts
312. no rollback path
313. no branch protection
314. no required review for security-sensitive paths
315. no CODEOWNERS or equivalent for auth/crypto/deploy/infrastructure paths
316. shipping AI-generated code without reading the diff
317. shipping AI-generated code without tests
318. shipping AI-generated code without security scanning
319. letting the agent delete tests to make CI pass
320. letting the agent weaken assertions to make tests pass
321. letting the agent silence linters instead of fixing issues
322. letting the agent add dependencies without justification
323. letting the agent add network calls without justification
324. letting the agent add shell commands without justification
325. letting the agent add dynamic execution
326. letting the agent add broad exception swallowing
327. letting the agent invent APIs and commit them unchecked
328. letting the agent invent config variables and defaults
329. letting the agent invent database columns/migrations without schema review
330. letting the agent run migrations against shared or production data
331. letting the agent run destructive filesystem commands
332. letting the agent access production credentials
333. letting the agent access secrets it does not need
334. letting the agent browse arbitrary web pages and obey instructions found inside them
335. letting external text override system/repo instructions
336. letting the agent exfiltrate code, secrets, logs, data, or prompts to external tools
337. letting the agent self-approve PRs
338. letting the agent deploy
339. letting the agent change CI policy to pass its own work
340. letting the agent remove security checks
341. letting the agent modify lockfiles without dependency review
342. letting the agent produce “clever” code that future agents cannot inspect
343. letting the agent create hidden side effects at import time
344. letting the agent rely on global mutable state
345. letting the agent write code that requires a human to infer undocumented invariants
346. letting the agent use production data in prompts
347. letting the agent store user data in comments, fixtures, logs, or examples
348. accepting agent output that has no acceptance criteria
349. accepting agent output that does not explain security-relevant assumptions
350. `except Exception: pass`
351. catching exceptions and returning success
352. treating auth-system failure as allow
353. treating permission-check failure as allow
354. treating validation failure as default value
355. treating payment failure as paid
356. treating signature-verification failure as trusted
357. treating TLS failure as retry-without-TLS
358. treating database failure as empty permissions
359. treating missing config as insecure defaults
360. ignoring subprocess return codes
361. ignoring failed writes
362. ignoring failed deletes
363. ignoring partial uploads
364. ignoring failed migrations
365. ignoring JSON/YAML parse errors and using defaults
366. swallowing `CancelledError` incorrectly in async code
367. catching broad exceptions around large blocks instead of narrow operations
368. losing original stack traces
369. raising generic errors that remove diagnostic context
370. returning raw errors to users
371. retrying forever
372. retrying non-idempotent operations blindly
373. no timeout, no cancellation, no backoff, no idempotency key
374. public functions with ambiguous untyped inputs
375. security-sensitive functions accepting `dict[str, Any]` with no schema
376. returning inconsistent shapes
377. using `None`, `False`, `0`, `""`, and `{}` interchangeably
378. using strings for roles, permissions, states, currencies, or statuses when enums/literals are appropriate
379. no validation at service boundaries
380. no database constraints for invariants that matter
381. using comments as the only source of truth
382. using dynamic attributes for domain objects
383. changing function behavior based on hidden globals
384. ignoring type-checker errors
385. using blanket `Any`
386. adding `# type: ignore` without reason
387. monkeypatching core behavior in production
388. mutating input arguments unexpectedly
389. returning mutable internal state
390. relying on dict key presence instead of typed models or schemas
391. accepting arbitrary JSON and passing it deeper into the system
392. inconsistent timezone handling
393. naive datetimes for externally meaningful times
394. floats for money
395. no tests
396. only happy-path tests
397. tests that do not assert anything meaningful
398. tests that merely snapshot broken behavior
399. tests that mock away the risky part
400. no negative tests
401. no permission tests
402. no authz boundary tests
403. no malicious input tests
404. no regression test for fixed vulnerabilities
405. tests requiring production credentials
406. tests hitting production services
407. tests depending on global machine state
408. flaky tests accepted as normal
409. `sleep()`-based timing tests without deterministic control
410. nondeterministic random tests without seeding or property controls
411. deleting tests to satisfy agent/codegen output
412. weakening tests to match bad implementation
413. not running tests in CI
414. marking tests skipped forever
415. `xfail` with no issue, owner, or expiry
416. no coverage of security-critical code paths
417. no formatter
418. no linter
419. no security linter
420. no dependency scanner
421. no secret scanner
422. no type checker for serious application code
423. no pre-commit hooks
424. no CI enforcement
425. blanket disabling Ruff/Bandit/mypy/pyright/pytest warnings
426. adding `# noqa` everywhere
427. adding `# nosec` without issue link and review
428. using `ruff --fix` blindly without reviewing diffs
429. using auto-fixes that change semantics without tests
430. pinning old linter versions forever to avoid new findings
431. treating style-only cleanup as a substitute for security review
432. network calls at import time
433. database writes at import time
434. migrations at import time
435. file deletion at import time
436. cloud API calls at import time
437. logging configuration that hijacks host applications
438. reading secrets at import time unnecessarily
439. starting threads/processes at import time
440. registering destructive signal handlers at import time
441. performing expensive computation at import time
442. parsing CLI args at import time
443. writing files at import time
444. modifying `sys.path` at import time
445. modifying global state at import time
446. monkeypatching libraries at import time
447. top-level script behavior without `if __name__ == "__main__":`
448. modules that cannot be imported safely by tests, linters, type checkers, documentation tools, or agents
449. mutable default arguments for stateful values:
450. global caches with no invalidation
451. global clients initialized with production credentials
452. global config mutated by tests or requests
453. global “current user”
454. thread-local state used as an authorization source without strict lifecycle control
455. context variables not reset
456. hidden singleton dependencies
457. monkeypatching production behavior
458. modifying builtins
459. modifying library globals
460. relying on import order
461. relying on current working directory
462. hidden environment-variable behavior
463. shared mutable class attributes for per-request state
464. storing request-specific data in module globals
465. using globals to pass data between functions
466. action-at-a-distance side effects
467. no clear ownership of state
468. no timeouts on network calls
469. no timeouts on database calls
470. no timeouts on subprocesses
471. no timeouts on locks
472. no timeout around agent tools
473. blocking I/O inside async request handlers
474. CPU-heavy work inside an event loop
475. unbounded task creation
476. unbounded queues
477. unbounded thread pools
478. unbounded process pools
479. unbounded recursion
480. unbounded regex processing
481. unbounded JSON/YAML/XML parsing
482. unbounded file reads
483. unbounded response reads
484. unbounded uploads
485. unbounded retries
486. no backpressure
487. no cancellation handling
488. swallowing cancellations
489. deadlocks from mixed sync/async locks
490. shared mutable state across threads without synchronization
491. database transactions held across network calls
492. locks held across awaits
493. missing idempotency for retried jobs
494. cron jobs that can overlap destructively
495. background workers that process poison messages forever
496. resource cleanup dependent on happy paths only
497. no database constraints for unique, foreign-key, non-null, or range invariants
498. no transactions around multi-step changes
499. long transactions around network calls
500. autocommit surprises
501. ignoring isolation/race conditions
502. read-check-write without locking or constraints
503. migrations with destructive changes and no backup/rollback plan
504. irreversible migrations casually shipped
505. data migrations with no dry run
506. schema drift between environments
507. raw SQL without parameterization
508. ORM filters based on user-supplied dicts without allowlists
509. mass assignment into ORM models
510. exposing internal IDs where access control is missing
511. returning soft-deleted or cross-tenant records
512. no audit trail for sensitive changes
513. deleting instead of tombstoning when retention/audit matters
514. using production data in tests, demos, or prompts
515. copying production data to laptops without controls
516. logging query results that contain sensitive data
517. treating backups as optional
518. never testing restore
519. endpoints with hidden side effects
520. GET requests that mutate state
521. no idempotency keys for payment/order/job creation
522. no pagination limits
523. no body-size limits
524. no schema validation
525. no versioning strategy
526. inconsistent error formats
527. leaking internals in errors
528. returning more fields than needed
529. returning secrets or internal flags
530. accepting more fields than needed
531. silently ignoring unknown input fields in security-sensitive APIs
532. mixing public and internal APIs without auth separation
533. undocumented permission requirements
534. no audit logs for admin actions
535. no request signing where needed
536. weak CORS
537. weak CSRF
538. no timeout budgets
539. no deprecation policy
540. collecting data not needed
541. retaining data forever
542. no classification of sensitive fields
543. no deletion path where required
544. no access controls around exports
545. no purpose limitation
546. no encryption at rest where required
547. no encryption in transit
548. using real user data in test fixtures
549. using real user data in prompts to coding agents
550. copying PII into tickets, comments, logs, screenshots, notebooks, or docs
551. exporting full tables for debugging
552. sending sensitive data to third-party APIs without review
553. storing raw documents when only derived features are needed
554. using hashes as “anonymous” data without threat modeling
555. no audit trail for reads of sensitive records
556. no redaction in admin tools
557. no retention policy for logs and backups
558. committing notebooks with outputs containing secrets or data
559. committing notebooks with hidden state that changes results
560. relying on cell execution order instead of scripts/pipelines
561. loading pickled datasets from unknown sources
562. loading model checkpoints from unknown sources
563. running downloaded notebooks
564. using `exec`/`eval` in notebooks to generate code
565. no train/test split discipline
566. data leakage from test to train
567. training on sensitive data without governance
568. no reproducibility for important results
569. no dependency pinning for experiments that become production
570. no model/data provenance
571. no validation of input ranges at inference time
572. no guardrails on generated outputs used as code, queries, or commands
573. no monitoring for model drift where decisions matter
574. no review of prompt templates that can trigger tools/actions
575. treating model output as truth instead of untrusted input
576. no `pyproject.toml` or clear build metadata
577. ambiguous package layout
578. scripts depending on the current working directory
579. modules shadowing standard library names: `json.py`, `typing.py`, `email.py`, `test.py`
580. circular imports caused by poor boundaries
581. imports with side effects
582. no separation between library code and CLI entrypoints
583. no pinned runtime dependencies for apps
584. no supported Python-version policy
585. relying on system Python packages in production
586. mixing dev/prod dependencies
587. editable installs as a production deployment mechanism
588. package import requires credentials
589. package import starts services
590. package installation runs unexpected network calls
591. no license/provenance review for vendored code
592. no release checklist
593. no artifact signing/provenance where required
594. default `DEBUG=True`
595. default auth disabled
596. default password `admin`
597. default secret key
598. default bind to public interface
599. default CORS permissive
600. default TLS verification off
601. default “allow all”
602. default local filesystem paths in production
603. default production database in local dev
604. default destructive mode
605. no config schema
606. no startup config validation
607. silent fallback when required config is missing
608. reading config from untrusted writable paths
609. mixing dev/staging/prod config
610. environment-variable names that are ambiguous
611. secrets and non-secrets stored together with the same access controls
612. no config diff review
613. no safe mode for dry-run operations
614. running app containers as root without need
615. broad filesystem write access
616. broad cloud IAM roles
617. shared cloud credentials
618. long-lived cloud credentials
619. production credentials on developer machines without controls
620. no network egress controls for sensitive services
621. no inbound firewalling
622. public debug/admin ports
623. SSH keys baked into images
624. secrets baked into images
625. writable application code directories
626. world-writable mounted volumes
627. cron jobs with broad privileges
628. deploying from a developer laptop as the release process
629. no health checks
630. no graceful shutdown
631. no backup/restore test
632. no monitoring on critical jobs
633. no alerting for auth, payment, data, or job failures
634. giant functions
635. giant files
636. unclear names
637. magic constants
638. copy-paste logic
639. dead code
640. commented-out code
641. misleading comments
642. comments that contradict behavior
643. no docstrings for public APIs
644. no examples for tricky APIs
645. no threat model for security-sensitive features
646. no migration notes
647. no operational runbook
648. no ownership
649. no issue link for risky changes
650. no changelog for breaking behavior
651. “temporary” bypasses with no expiry
652. TODOs that disable security
653. code paths nobody can explain
654. cleverness over clarity
655. accepting generated code whose assumptions are not documented
656. no human review on high-risk areas
657. `# nosec` without a ticket, owner, and expiration
658. `# noqa` on whole files
659. `# type: ignore` without explanation
660. disabling Bandit/Ruff/mypy/pytest warnings globally
661. lowering CI standards to merge a change
662. marking security tests skipped
663. “temporarily” allowing debug mode
664. “temporarily” hardcoding a credential
665. “temporarily” disabling auth
666. “temporarily” disabling TLS verification
667. “temporarily” opening CORS
668. “temporarily” making a bucket public
669. accepting “we’ll fix it later” for an exploitable issue
670. accepting “the AI wrote it” as a justification
671. accepting “the user won’t do that” as a defense
672. accepting “it’s internal” as a defense
673. accepting “it’s behind a VPN” as the only defense
674. accepting “we trust our users” as the only defense
675. **Formatting/linting:** Ruff or equivalent enforced in CI.
676. **Security static analysis:** Bandit/Ruff security rules enabled; suppressions reviewed.
677. **Types:** mypy or Pyright for application/library code where correctness matters.
678. **Tests:** pytest or equivalent with negative/security tests for trust boundaries.
679. **Secrets:** GitHub/GitLab secret scanning or equivalent enabled; leaked secrets revoked, not merely deleted.
680. **Dependencies:** pinned/repeatable installs, vulnerability scanning, reviewed lockfile changes.
681. **CI permissions:** least privilege tokens, no secrets exposed to untrusted PRs, no agent self-approval.
682. **Release:** reproducible builds, reviewed artifacts, rollback path.
683. **Agent rules:** documented rules forbidding dynamic execution, unsafe deserialization, shell shortcuts, secret exposure, dependency additions without review, and destructive commands without approval.
684. Ship agent-generated code without tests.
685. Let the same agent write code and tests with no independent review.
686. Generate tests that merely assert the current implementation.
687. Delete failing tests to make CI green.
688. Mark flaky tests as skipped without an owner and issue.
689. Mock the thing you are trying to test.
690. Test only happy paths.
691. Skip authorization, multi-tenant, malicious input, boundary, timeout, and failure-mode tests.
692. Use production services in unit tests.
693. Use real customer data in tests.
694. Depend on test execution order.
695. Leave tests nondeterministic through time, randomness, network, or shared state.
696. Ignore coverage holes in security-critical code.
697. Treat coverage percentage as proof of correctness.
698. Merge when lint/type/security scans fail.
699. Disable Bandit/Ruff/mypy/CodeQL/secret scanning because “the AI knows what it’s doing.”
700. Run tests only locally.
701. Skip regression tests for security fixes.
702. Skip negative tests for injection, traversal, auth bypass, and malformed input.
703. Treat “manual QA passed” as enough for critical code.
704. Never merge production Python without automated tests.
705. Never test only the happy path.
706. Never skip regression tests for a security bug.
707. Never skip tests because the change was “AI-generated.”
708. Never accept tests that merely assert mocks were called while core behavior is untested.
709. Never let tests pass by weakening the specification.
710. Never delete failing tests without explaining why the test is wrong.
711. Never mark flaky tests as acceptable indefinitely.
712. Never let tests depend on run order.
713. Never let tests mutate production services.
714. Never let tests require production credentials.
715. Never use real payment, email, or SMS side effects in normal test runs.
716. Never rely only on manual clicking for a web app.
717. Never skip linting for production code.
718. Never skip formatting in a shared codebase.
719. Never skip type checking for complex interfaces where types would prevent misuse.
720. Never skip dependency vulnerability scanning.
721. Never skip secret scanning.
722. Never skip SAST/security linting for services exposed to users.
723. Never ignore Bandit/Ruff/security-tool findings without documented triage.
724. Never blanket-ignore an entire rule category because it is noisy.
725. Never rely on coverage percentage as proof of correctness.
726. Never accept generated code without tests that encode the intended behavior.
727. Never use property-based testing results as a substitute for threat modeling, or vice versa.
728. No regression test for a fixed vulnerability.
729. Tests that depend on execution order.
730. Tests that hit production.
731. Tests that use production credentials.
732. Tests that mutate production-like shared resources.
733. Flaky tests ignored instead of fixed.
734. Blanket `xfail` without owner and expiry.
735. Disabling tests to merge.
736. Disabling linting to merge.
737. Disabling type checking to merge.
738. Disabling security scans to merge.
739. Ignoring Bandit/Semgrep high-severity findings without documented review.
740. Ignoring dependency-audit findings without documented risk acceptance.
741. Ignoring secret-scanning findings.
742. No CI for pull requests.
743. CI that does not run on agent-generated changes.
744. No pre-commit checks for formatting, linting, secret detection, and basic static analysis.
745. No code owner review for security-sensitive files.
746. Huge AI-generated PRs that are impossible to review.
747. Generated tests that merely assert the generated implementation’s current behavior.
748. Mocking away all real security behavior.
749. Snapshot tests full of secrets or PII.
750. Not testing failure paths.
751. Not testing malformed input.
752. Not testing concurrent calls.
753. Not testing authorization on background jobs, websockets, CLI tools, and admin paths.
754. Hardcoding API keys, passwords, tokens, private keys, OAuth secrets, database URLs, cloud credentials, JWT signing keys, encryption keys, webhook secrets, SSH keys, service account JSON, or basic-auth strings.
755. Committing `.env`, `.pem`, `.key`, `.p12`, `.kube/config`, `service-account.json`, SQLite prod dumps, test fixtures with real data, or notebook outputs containing credentials.
756. “Removing the secret in the next commit” and treating it as fixed.
757. Sharing one token across dev, CI, staging, and prod.
758. Long-lived human tokens for automation.
759. Storing secrets in client-side code, mobile apps, browser JS, or downloadable config.
760. Default credentials, sample credentials, `changeme`, `password`, or copied tutorial keys.
761. Printing `os.environ`, full config objects, request headers, or database URLs.

## The top-level rule

1. Executes untrusted text as code.
2. Deserializes untrusted data into live Python objects.
3. Runs shell commands built from strings.
4. Disables TLS, certificate, hostname, or host-key verification.
5. Handles secrets carelessly.
6. Uses weak randomness or crypto for security.
7. Trusts user input, model output, files, URLs, archives, environment variables, or repo contents without validation.
8. Gives an AI/agent broad shell, filesystem, network, cloud, or credential access.
9. Skips tests, linting, security review, or human review because the code was generated quickly.
10. Hides failure.
11. Never use `eval()` on user input, file contents, network data, model output, environment variables, CLI arguments, database content, spreadsheet cells, YAML/JSON/TOML values, or config from a repo.
12. Never use `exec()` on anything not fully hardcoded and reviewed.
13. Never use `compile()` as a back door to dynamic execution.
14. Never build Python code as strings and run it.
15. Never let an LLM generate Python and immediately execute it against real files, secrets, or networks.
16. Never execute code copied from README files, GitHub issues, Stack Overflow, Reddit, X, generated notebooks, or model responses without review.
17. Never use `input()` followed by `eval()`. This is the classic Python security failure.
18. Never dynamically import arbitrary user-provided module names with `__import__`, `importlib.import_module`, plugin paths, or dotted strings unless the value is from a strict allowlist.
19. Never call arbitrary attributes or functions based on user-controlled strings without an allowlist.
20. Never use `globals()[name]`, `locals()[name]`, or `getattr(obj, name)` as a command router for untrusted names.
21. Never accept “Python expression” as a feature unless it is a real sandbox or a purpose-built parser. A regex is not a sandbox.
22. Never assume `ast.literal_eval()` makes arbitrary input safe at scale. It avoids code execution, but it can still be abused for memory/CPU exhaustion if unbounded.
23. Never hide dynamic execution behind “plugin,” “rule engine,” “template,” “formula,” “workflow,” “calculator,” or “custom logic” without isolation and allowlists.
24. Never execute generated migration scripts, data-cleanup scripts, or cloud scripts without review and dry-run protections.
25. Never execute test files from an untrusted repository while secrets are present in the environment.
26. Never use `shell=True` with user input, file names, repo content, URLs, environment variables, model output, branch names, commit messages, package names, or cloud resource names.
27. Never build shell commands with f-strings, `%`, `.format()`, `+`, or joins.
28. Never pass a single command string when a list of arguments is expected.
29. Never use `os.system`, `os.popen`, `commands`, or shell backticks in Python code.
30. Never shell out when a safe Python API exists.
31. Never use partial executable paths like `"git"` or `"python"` in sensitive contexts where PATH hijacking is possible; use fully resolved paths.
32. Never trust the current working directory for executable resolution.
33. Never use wildcards with untrusted input: `rm *.txt`, `tar *`, `chmod *`, `chown *`.
34. Never pass untrusted input to `sh -c`, `bash -c`, PowerShell, `cmd.exe`, `make`, `xargs`, `find -exec`, `ssh`, `scp`, `rsync`, `git`, `docker`, `kubectl`, `helm`, `aws`, `gcloud`, `az`, or database CLIs.
35. Never let an agent decide arbitrary shell commands in an environment containing secrets.
36. Never let an agent run package-manager lifecycle scripts from an untrusted repo with network access and credentials.
37. Never use `curl | sh`, `wget | bash`, or Python equivalents in build/deploy scripts.
38. Never suppress subprocess failures with `check=False` unless the failure is explicitly handled.
39. Never ignore `stderr` or return codes for commands that mutate state.
40. Never run destructive commands without path validation, dry-run support, and human-visible diffs.
41. Never run shell commands as root unless the code is explicitly designed and reviewed for privileged execution.
42. Never call shell commands from import-time code.
43. `pickle.load`
44. `pickle.loads`
45. `dill.load`
46. `dill.loads`
47. `cloudpickle.load`
48. `shelve.open` on untrusted files
49. `marshal.load` on untrusted data
50. `joblib.load` on untrusted files
51. `torch.load` on untrusted models/checkpoints
52. scikit-learn pickle/joblib model files from untrusted sources
53. `numpy.load(..., allow_pickle=True)` on untrusted `.npy` / `.npz`
54. arbitrary Keras/TensorFlow model formats with custom objects from untrusted sources
55. untrusted Python wheels, eggs, source distributions, or notebooks as “data”
56. Use `yaml.load()` without `SafeLoader`.
57. Accept YAML configs from users, repos, tickets, or model output and load them unsafely.
58. Treat `.pkl`, `.pickle`, `.pt`, `.pth`, `.joblib`, `.npy`, `.npz`, `.onnx`, `.h5`, `.keras`, `.safetensors`, `.parquet`, `.feather`, or `.arrow` as automatically safe.
59. Load model artifacts from random GitHub repos, Hugging Face repos, Google Drive links, Slack uploads, S3 buckets, or CI artifacts without provenance checks.
60. Store sessions, auth tokens, permissions, or user preferences as pickled blobs.
61. Accept pickled cookies.
62. Use pickle for cross-service communication.
63. Use pickle for cache values if the cache can be written by another process, tenant, user, or compromised service.
64. Trust “internal” artifacts simply because they came from an internal bucket.
65. Deserialize before authenticating integrity.
66. Deserialize before checking file size, type, source, and expected schema.
67. Let agents “inspect” unknown binary artifacts by loading them into Python.
68. Build SQL with f-strings.
69. Build SQL with string concatenation.
70. Build SQL with `%` formatting.
71. Build SQL with `.format()`.
72. Pass user-controlled column names, table names, sort keys, or filter clauses without strict allowlists.
73. Use ORM “raw SQL” escape hatches with untrusted input.
74. Use Django `.raw()`, `.extra()`, `RawSQL`, or manual cursor execution with untrusted interpolated strings.
75. Use SQLAlchemy `text()` with interpolated user data.
76. Treat “admin-only” SQL construction as safe.
77. Trust LLM-generated SQL against production databases.
78. Allow an agent to generate and execute database mutations without review.
79. Use MongoDB/NoSQL query dictionaries built directly from request bodies if operators like `$where`, `$ne`, `$regex`, `$gt`, or `$lookup` can be smuggled in.
80. Build LDAP filters by string interpolation.
81. Build XPath expressions from untrusted input.
82. Build GraphQL queries by string interpolation.
83. Build Redis commands from strings with untrusted pieces.
84. Build PromQL, Lucene, Elasticsearch DSL, or search queries from raw strings without validation.
85. Disable Jinja2 autoescaping around user input.
86. Use `mark_safe`, `Markup`, or “safe HTML” wrappers on untrusted content.
87. Render Markdown to HTML without sanitization.
88. Trust filenames, image metadata, CSV cells, spreadsheet formulas, or uploaded document text as safe display content.
89. Feed user text into Pandas `DataFrame.query()` or `DataFrame.eval()` as an expression language. Pandas warns these APIs can run arbitrary code if passed user input.
90. Hardcode API keys.
91. Hardcode passwords.
92. Hardcode database URLs with credentials.
93. Hardcode private keys.
94. Hardcode JWT secrets.
95. Hardcode OAuth client secrets.
96. Hardcode cloud credentials.
97. Hardcode PyPI tokens.
98. Hardcode GitHub tokens.
99. Hardcode Slack/Discord/webhook URLs.
100. Hardcode test credentials that work anywhere real.
101. Commit `.env`.
102. Commit `.pem`, `.key`, `.p12`, `.pfx`, `id_rsa`, `id_ed25519`, kubeconfigs, service-account JSON, or cloud credential files.
103. Put secrets in notebooks.
104. Put secrets in screenshots.
105. Put secrets in docstrings or comments.
106. Put secrets in sample config that looks real.
107. Put secrets in Dockerfiles, image layers, build args, or package metadata.
108. Print secrets.
109. Log secrets.
110. Include secrets in exception messages.
111. Include secrets in telemetry.
112. Include secrets in metrics labels.
113. Include secrets in Sentry/OpenTelemetry attributes.
114. Include secrets in CLI arguments where process listings can expose them.
115. Include secrets in URLs.
116. Include secrets in GitHub Actions workflow output.
117. Include secrets in model prompts or agent context.
118. Let an agent read secret stores unless the task explicitly requires it and the tool is scoped.
119. Let an agent write secrets to files.
120. Store secrets in browser-readable frontend config.
121. Store secrets in SQLite files, pickle files, local JSON, or cache directories without encryption and access control.
122. Use the same secret across dev, staging, and production.
123. Use placeholder secrets like `"changeme"`, `"secret"`, `"password"`, `"dev"`, or `"test"` in deployable code.
124. Mask secrets only after logging them. The log event already happened.
125. Depend on automatic redaction as your only protection.
126. Keep long-lived publishing tokens in CI when short-lived/OIDC-based publishing is available.
127. Use `random` for passwords, reset tokens, API keys, session IDs, CSRF tokens, invite codes, verification codes, OAuth state, nonce values, salts, or cryptographic keys.
128. Use `uuid.uuid4()` as a security token without understanding token-length and secrecy requirements.
129. Use MD5 for passwords, signatures, integrity checks in adversarial contexts, cache-busting security, or file authenticity.
130. Use SHA-1 for security-sensitive integrity or signatures.
131. Use unsalted password hashes.
132. Store plaintext passwords.
133. Store reversibly encrypted passwords.
134. Roll your own password hashing.
135. Roll your own encryption.
136. Roll your own signing scheme.
137. Roll your own JWT implementation.
138. Use ECB mode.
139. Reuse nonces with stream ciphers or AEAD modes.
140. Hardcode crypto keys.
141. Reuse dev keys in production.
142. Use short RSA keys.
143. Disable certificate verification.
144. Disable hostname verification.
145. Use `requests(..., verify=False)`.
146. Use `ssl._create_unverified_context()`.
147. Use `CERT_NONE` for production clients.
148. Use self-signed certificates by “just turning off verification.”
149. Accept any SSH host key automatically.
150. Use Paramiko auto-add host-key policies in production.
151. Use SSLv2, SSLv3, obsolete TLS versions, export ciphers, null ciphers, or anonymous ciphers.
152. Ignore certificate expiration.
153. Ignore certificate pinning or CA trust requirements where the system design requires them.
154. Log private keys, JWT signing keys, or decrypted secrets.
155. Store encryption keys next to encrypted data with the same access controls.
156. Trust user-supplied paths.
157. Concatenate paths with strings when crossing trust boundaries.
158. Allow `../` path traversal.
159. Allow absolute paths from untrusted input.
160. Write to paths derived from user input without resolving and checking they remain inside an allowed directory.
161. Follow symlinks in untrusted directories unless explicitly safe.
162. Use filenames from uploads directly.
163. Trust file extensions.
164. Trust MIME types from clients.
165. Trust archive member paths.
166. Extract tar files from untrusted sources without a safe extraction filter and inspection.
167. Extract zip files from untrusted sources without path, size, and member validation.
168. Ignore symlinks, hardlinks, device files, file permissions, and ownership inside archives.
169. Extract archives as root.
170. Extract archives into sensitive directories.
171. Extract archives over existing files.
172. Read entire untrusted files into memory without size limits.
173. Accept zip bombs, decompression bombs, deeply nested archives, or huge sparse files.
174. Use `tempfile.mktemp()`. It is deprecated and insecure because of race conditions.
175. Create temp files with predictable names.
176. Write secret material to world-readable temp files.
177. Leave temp files containing secrets behind.
178. Use `/tmp/some_fixed_name`.
179. Use `chmod 777`.
180. Use `chmod -R 777`.
181. Use world-readable private keys.
182. Use world-writable config directories.
183. Use user-controlled file names in logs without sanitization if logs are parsed downstream.
184. Delete recursively based on untrusted or unchecked paths.
185. Run cleanup code that can turn an empty string, `/`, `.`, or `$HOME` into a deletion target.
186. Call external URLs supplied by users without allowlists and SSRF protections.
187. Fetch arbitrary URLs from an agent’s plan.
188. Let users provide internal hostnames or IPs to fetch.
189. Allow requests to `localhost`, link-local addresses, private RFC1918 ranges, metadata services, Kubernetes service IPs, cloud metadata endpoints, or internal admin panels.
190. Follow redirects blindly when fetching user URLs.
191. Ignore DNS rebinding.
192. Ignore IPv6 private/link-local addresses.
193. Forget timeouts on `requests`, `httpx`, `urllib`, database calls, cloud SDK calls, or subprocesses.
194. Use infinite retries.
195. Retry non-idempotent operations blindly.
196. Retry payment, email, mutation, or deletion operations without idempotency keys.
197. Accept webhooks without signature verification.
198. Trust `X-Forwarded-For`, `Host`, `X-Real-IP`, or scheme headers unless your proxy chain is explicitly configured.
199. Trust request JSON shape without validation.
200. Trust client-side validation.
201. Trust frontend role/user ID/tenant ID claims.
202. Trust CORS as authentication.
203. Use `Access-Control-Allow-Origin: *` with credentials.
204. Disable CSRF protection on cookie-authenticated state-changing endpoints.
205. Return stack traces to users.
206. Serve Python’s `http.server` in production.
207. Bind dev servers to `0.0.0.0` by default.
208. Ship debug endpoints, admin endpoints, profiling endpoints, or test fixtures.
209. Expose `/metrics`, `/debug`, `/docs`, `/openapi.json`, `/admin`, `/actuator`, or internal health endpoints publicly without review.
210. Use Flask `debug=True` in any exposed environment.
211. Use Django `DEBUG=True` in any exposed environment.
212. Treat internal APIs as trusted simply because they are internal.
213. Let an LLM choose arbitrary API calls with production credentials.
214. Authenticate without authorization.
215. Check authorization only in the UI.
216. Trust `user_id`, `tenant_id`, `role`, `is_admin`, or `permissions` from request bodies.
217. Trust JWT claims without verifying signature, issuer, audience, expiry, and algorithm policy.
218. Accept `alg=none`.
219. Let clients choose JWT algorithms.
220. Reuse JWT signing keys across unrelated systems.
221. Store session IDs in logs.
222. Put sensitive session tokens in URLs.
223. Store auth tokens in localStorage for high-risk apps when safer cookie/session designs are required.
224. Disable CSRF on cookie-authenticated apps.
225. Use predictable session IDs.
226. Use long-lived bearer tokens without rotation/revocation.
227. Fail open when an auth service is unavailable.
228. Cache authorization decisions without including user, tenant, resource, action, policy version, and revocation implications.
229. Use global mutable permission state.
230. Use `lru_cache` on user-specific access-control checks without complete keys.
231. Mix tenants in the same cache key namespace.
232. Use object IDs as authorization.
233. Allow direct object references without ownership checks.
234. Use “admin route” names as access control.
235. Trust email domains without verification where verification matters.
236. Implement password reset without single-use, expiry, rate limits, and audit logging.
237. Implement MFA bypass “temporarily.”
238. Log in users automatically after email change without verification.
239. Change account email, password, MFA, payout details, or API keys without re-authentication where risk requires it.
240. Let an agent perform account/security mutations without explicit human approval.
241. Use bare `except:`.
242. Use `except Exception: pass`.
243. Swallow exceptions without logging, metrics, or caller-visible failure.
244. Convert all exceptions into `None`.
245. Convert all exceptions into `False`.
246. Catch everything and continue with partial state.
247. Hide data-loss errors.
248. Hide permission errors.
249. Hide network errors in background jobs.
250. Hide failed security checks.
251. Hide failed audit logging.
252. Hide failed notification or payment operations.
253. Use `finally` to `return`, `break`, or `continue` in ways that suppress exceptions.
254. Use `assert` for runtime input validation, authorization, authentication, or security checks.
255. Replace specific exceptions with a vague `Exception("failed")` while losing cause/context.
256. Raise strings or non-exceptions.
257. Log an exception and then pretend the operation succeeded.
258. Log and rethrow in a way that creates duplicate noisy alerts without adding context.
259. Catch `BaseException` unless handling process shutdown at a top-level boundary.
260. Catch `KeyboardInterrupt` or `SystemExit` in normal application logic.
261. Retry forever after exceptions.
262. Retry without jitter/backoff.
263. Retry on authentication/authorization failures.
264. Fail open when a security dependency fails.
265. Treat “unknown” as “allowed.”
266. Treat parser failure as “accept default.”
267. Treat config load failure as “use insecure defaults.”
268. Log passwords.
269. Log tokens.
270. Log API keys.
271. Log cookies.
272. Log `Authorization` headers.
273. Log private keys.
274. Log full credit card numbers.
275. Log full SSNs or national IDs.
276. Log medical records, legal records, or sensitive personal data unless explicitly required and protected.
277. Log raw request bodies by default.
278. Log raw response bodies by default.
279. Log query strings when they may contain secrets.
280. Log signed URLs.
281. Log presigned S3/GCS/Azure URLs.
282. Log database DSNs with credentials.
283. Log environment variables wholesale.
284. Log model prompts containing secrets or user-private data.
285. Send sensitive data to third-party observability tools without policy approval.
286. Put user-controlled values into logs without considering log injection.
287. Disable security logs to reduce noise.
288. Disable audit logs for destructive actions.
289. Fail to log permission changes, account recovery, MFA changes, key creation, key deletion, data export, bulk deletion, payout changes, or admin impersonation.
290. Store logs forever without retention policy.
291. Store logs without access control.
292. Let agents read unrestricted logs containing secrets or PII.
293. Let agents summarize sensitive logs into long-term memory.
294. Use debug logging in production by default.
295. Ignore alerting for repeated auth failures, SSRF attempts, injection attempts, privilege changes, and unusual data export.
296. Use unpinned production dependencies.
297. Use `pip install package` in production without a lockfile or equivalent reproducibility mechanism.
298. Use `pip install -U` in CI/deploy without review.
299. Use `latest` as a production dependency strategy.
300. Use random packages suggested by an LLM without checking name, maintainer, downloads, release history, source, license, and vulnerabilities.
301. Install packages with typosquatted names.
302. Install packages from arbitrary Git URLs in production.
303. Install from arbitrary branches.
304. Install from mutable refs instead of immutable commits/tags where reproducibility matters.
305. Install directly from a user’s fork without review.
306. Use `--extra-index-url` in a way that enables dependency confusion for private packages.
307. Mix public and private indexes carelessly.
308. Disable TLS verification for package installs.
309. Ignore dependency vulnerability reports.
310. Ignore transitive dependencies.
311. Use abandoned crypto libraries like `pycrypto`.
312. Vendor code copied from gists/blogs/issues without license and security review.
313. Allow dependency install scripts to run in an environment with production secrets.
314. Let an agent run `pip install` from an untrusted repo while secrets are present.
315. Let CI publish packages from pull requests.
316. Publish to PyPI with long-lived tokens stored broadly in CI when Trusted Publishing/OIDC is available.
317. Reuse PyPI tokens across projects.
318. Build wheels/sdists on a developer laptop with untracked local files.
319. Publish artifacts that were not built from a clean, reproducible source state.
320. Ignore lockfile diffs.
321. Hide dependency changes in huge generated commits.
322. Commit virtualenvs.
323. Commit site-packages.
324. Commit wheels of unknown provenance.
325. Depend on `setup.py` side effects.
326. Import the package inside `setup.py` if import-time code has side effects.
327. Use post-install hooks to download or execute code.
328. Run untrusted pull-request code with write tokens.
329. Run untrusted pull-request code with cloud credentials.
330. Run untrusted pull-request code with PyPI publishing tokens.
331. Run untrusted pull-request code with production database access.
332. Run untrusted pull-request code with access to internal package indexes unless isolated.
333. Use broad `GITHUB_TOKEN` permissions by default.
334. Use `pull_request_target` unsafely with checkout of attacker-controlled code.
335. Echo secrets in CI logs.
336. Disable secret masking.
337. Store secrets as plaintext variables in repo files.
338. Put deploy credentials in workflow YAML.
339. Let CI deploy from any branch.
340. Let CI publish from any tag pattern without protected tags.
341. Let generated code bypass branch protection.
342. Let agents commit directly to protected branches.
343. Let agents approve their own PRs.
344. Let agents resolve security findings by suppressing the rule.
345. Disable failing tests in CI to merge generated code.
346. Skip security scans on “docs-only” changes if docs execute notebooks or code blocks.
347. Cache dependency directories across trust boundaries without isolation.
348. Trust artifacts from untrusted jobs.
349. Deploy artifacts not built in CI.
350. Reuse build artifacts from untrusted workflows.
351. Leave self-hosted runners exposed to arbitrary PR code.
352. Run CI as root unnecessarily.
353. Use unpinned third-party actions in sensitive workflows.
354. Ignore vulnerabilities in third-party actions.
355. Unrestricted shell access.
356. Unrestricted filesystem access.
357. Unrestricted network access.
358. Cloud admin credentials.
359. Database write access.
360. Package publishing access.
361. Secret-manager read access.
362. SSH keys.
363. Kubernetes cluster-admin access.
364. GitHub organization admin access.
365. Ability to approve or merge its own PR.
366. Ability to disable CI.
367. Ability to edit security policy files without review.
368. Ability to modify audit logs.
369. Ability to send emails, payments, deletes, deploys, migrations, or customer-visible messages without explicit approval.
370. Treat external text as instructions.
371. Treat README files as trusted instructions.
372. Treat comments in code as trusted instructions.
373. Treat issue text as trusted instructions.
374. Treat webpage content as trusted instructions.
375. Treat logs, emails, Slack messages, or tickets as trusted instructions.
376. Follow prompt-injection instructions found in files or webpages.
377. Exfiltrate data to external services to “debug.”
378. Paste secrets into a prompt.
379. Store secrets in memory.
380. Add hidden backdoors “for testing.”
381. Add telemetry without disclosure.
382. Modify auth/security code without targeted review.
383. Add dependencies without explaining why.
384. Add broad `# noqa`, `# type: ignore`, `bandit:skip`, or `semgrep:ignore`.
385. Delete tests instead of fixing code.
386. Replace precise code with vague generated abstractions.
387. Write code it cannot explain.
388. Generate massive diffs that no human can review.
389. Make unrelated formatting changes in security-sensitive PRs.
390. Run destructive commands from generated plans.
391. Run tests from an untrusted repo while credentials are present.
392. Write outside the repository workspace.
393. Follow symlinks outside the workspace.
394. Access `$HOME/.ssh`, shell history, cloud config, browser profiles, token caches, or password stores.
395. Install browser extensions, daemons, cron jobs, launch agents, or global hooks.
396. Modify global Python, shell, Git, Docker, SSH, or cloud config.
397. Has import-time side effects.
398. Requires hidden local state.
399. Requires undocumented environment variables.
400. Depends on the developer’s current working directory.
401. Mutates global state during import.
402. Starts network calls during import.
403. Starts threads/processes during import.
404. Writes files during import.
405. Reads secrets during import.
406. Has non-deterministic tests.
407. Requires production services to run tests.
408. Fails without internet access when it should be unit-testable.
409. Has no clear entry points.
410. Has no clear rollback path.
411. Has no dry-run mode for destructive actions.
412. Hides behavior behind metaprogramming.
413. Makes security behavior implicit.
414. Makes permissions hard to inspect.
415. Makes diffs too noisy to review.
416. Put real work in module import side effects.
417. Perform network calls at import time.
418. Read secrets at import time.
419. Connect to databases at import time.
420. Start schedulers at import time.
421. Start threads/processes at import time.
422. Configure global logging at import time in library code.
423. Monkey-patch builtins globally.
424. Monkey-patch standard library modules globally.
425. Monkey-patch third-party libraries globally without an isolated compatibility layer.
426. Mutate `sys.path` casually.
427. Depend on the current working directory for imports.
428. Shadow standard-library modules with files like `json.py`, `email.py`, `types.py`, `typing.py`, `logging.py`, `asyncio.py`, or `secrets.py`.
429. Use wildcard imports in production code.
430. Hide circular imports with runtime hacks.
431. Use dynamic imports to avoid clear dependencies.
432. Put environment-specific behavior in import paths.
433. Use package import as a migration/deployment trigger.
434. Let a library call `sys.exit()` during import.
435. Let a library configure process-wide signal handlers without explicit opt-in.
436. Let a library change global event loops without explicit opt-in.
437. Use mutable default arguments:
438. Use `datetime.now()` or `time.time()` as a default argument.
439. Use open files, DB connections, sessions, clients, or locks as default arguments.
440. Use global mutable state for request/user/tenant/security context.
441. Store per-request state in module globals.
442. Store per-user state in class variables.
443. Store tenant-specific data in shared caches without tenant keys.
444. Use `lru_cache` on functions whose results depend on hidden state, permissions, time, environment, tenant, user, or locale.
445. Mutate input arguments unexpectedly.
446. Return internal mutable objects that callers can mutate.
447. Use shallow copies when deep copies are required for isolation.
448. Use `dict.setdefault(key, [])` or `defaultdict(list)` in shared/global contexts without lifecycle control.
449. Hide mutation inside properties.
450. Hide mutation inside `__repr__`, `__str__`, `__eq__`, or hashing.
451. Make objects hashable while mutable.
452. Use mutable dataclass defaults instead of `default_factory`.
453. Use global registries that tests cannot reset.
454. Use singleton clients with stale credentials.
455. Use global sessions across forks/threads when unsafe.
456. Trust untyped `dict` input at boundaries.
457. Accept `Any` everywhere to silence type checkers.
458. Turn off type checking because generated code is messy.
459. Use `# type: ignore` broadly.
460. Use `cast()` to lie to the type checker.
461. Treat Pydantic/dataclass validation as authorization.
462. Validate only frontend inputs.
463. Accept unknown fields silently in security-sensitive schemas.
464. Coerce types in dangerous ways: `"false"` becoming truthy, `"0"` becoming truthy, empty string becoming default admin values.
465. Treat missing values as allowed.
466. Treat parser failure as default allow.
467. Treat unknown enum values as safe.
468. Ignore timezone fields in timestamps.
469. Accept naive datetimes for audit, auth, expiration, billing, legal, or distributed-system logic.
470. Use floats for money.
471. Use floats for exact IDs, counters, or security-sensitive comparisons.
472. Use `is` for value equality except with `None` and true singletons.
473. Compare booleans to `True` or `False` instead of using clear truth checks.
474. Use `type(x) == SomeClass` when `isinstance` is intended.
475. Accept arbitrary JSON without size and depth limits.
476. Accept arbitrary regexes from users.
477. Accept arbitrary format strings from users.
478. Accept arbitrary Python format specs from users.
479. Accept arbitrary pickle/YAML/CSV/Excel formulas as data without sanitization.
480. Use naive local datetimes for tokens, billing, audit logs, legal deadlines, schedules, or distributed systems.
481. Compare naive and aware datetimes.
482. Store local time without timezone.
483. Use `datetime.now()` instead of explicit timezone-aware UTC where correctness matters.
484. Use wall-clock time for measuring durations; use monotonic time.
485. Use floats for currency.
486. Round money with binary floating-point.
487. Use locale-dependent parsing for security/business logic.
488. Accept ambiguous dates without explicit format.
489. Parse user dates without timezone policy.
490. Use string sorting for dates unless ISO format and validated.
491. Use cron-like schedules without documenting timezone and DST behavior.
492. Expire tokens based on client-provided time.
493. Generate IDs from timestamps when secrecy or unpredictability is required.
494. Use sequential IDs as proof of authorization.
495. Merge generated Python without human review.
496. Merge generated Python without tests.
497. Merge security-sensitive changes without negative tests.
498. Merge auth changes without authorization tests.
499. Merge parser changes without malformed-input tests.
500. Merge file-upload changes without malicious-file tests.
501. Merge archive extraction without traversal/symlink/size tests.
502. Merge subprocess changes without injection tests.
503. Merge dependency changes without lockfile review.
504. Merge database migrations without rollback or backup consideration.
505. Merge code with failing tests.
506. Delete tests to make generated code pass.
507. Mark tests `skip` or `xfail` permanently to make CI green.
508. Lower coverage thresholds to merge a vibe-coded PR.
509. Use `pragma: no cover` on security logic.
510. Disable lint rules globally to accept generated code.
511. Suppress Ruff/Bandit/Semgrep findings without narrow justification.
512. Turn off type checking for whole files because the generated code is hard to type.
513. Accept “manually tested” as proof for security-sensitive code.
514. Use production data in tests.
515. Use real customer PII in tests.
516. Use real payment methods in tests.
517. Use real credentials in tests.
518. Write tests that depend on public internet unless explicitly marked integration.
519. Write tests that mutate production resources.
520. Write flaky tests and fix them with retries only.
521. Use sleep-based tests when deterministic synchronization is possible.
522. Use current time directly in tests without freezing/injection.
523. Use random data without seeding or property-test reporting.
524. Snapshot secrets.
525. Snapshot huge generated outputs no one reviews.
526. Let an agent update snapshots blindly.
527. Add `# noqa` without specifying the rule and reason.
528. Add `# type: ignore` without specifying the reason.
529. Add `# nosec` casually.
530. Add `bandit:skip` casually.
531. Add `semgrep:ignore` casually.
532. Disable entire lint categories to merge generated code.
533. Suppress import-order, dead-code, complexity, or security warnings in bulk.
534. Leave unused imports, unused variables, dead branches, and commented-out code in serious code paths.
535. Leave generated TODOs like “handle errors later.”
536. Leave “temporary” insecure code without an owner, ticket, and expiry.
537. Turn off dependency scanning because it is noisy.
538. Turn off secret scanning because it caught test keys.
539. Ignore pre-commit hooks.
540. Bypass CI with admin privileges.
541. Rewrite history to hide secrets without rotating them. Rotation is still required.
542. Treat scanner silence as proof of safety.
543. Leave comments that contradict the code.
544. Leave comments that claim security properties not enforced by code.
545. Leave “safe because internal” comments.
546. Leave “TODO: validate input” in shipped code.
547. Leave “TODO: auth” in shipped code.
548. Leave “temporary bypass” in shipped code.
549. Leave generated explanations that do not match the implementation.
550. Leave misleading docstrings.
551. Hide important side effects from docs.
552. Hide required environment variables.
553. Hide required permissions.
554. Hide destructive behavior.
555. Hide network calls.
556. Hide data retention behavior.
557. Hide telemetry.
558. Hide security assumptions.
559. Write clever code no maintainer can debug.
560. Over-abstract generated code into unreadable frameworks.
561. Make simple code metaprogrammed.
562. Make security logic generic and magical.
563. Use single-letter names outside tiny scopes.
564. Use misleading names like `safe_eval`, `trusted_input`, or `sanitize` unless they truly do what they claim.
565. Call a function `validate_*` if it only parses.
566. Call a function `sanitize_*` if it only strips whitespace.
567. Call a function `encrypt_*` if it only base64-encodes.
568. Call a function `hash_*` if it is not cryptographic but used as if it were.
569. Block the event loop with `time.sleep()` in async code.
570. Run blocking network/file/database calls in async handlers without appropriate executors or async clients.
571. Perform CPU-heavy work on the event loop.
572. Create unbounded tasks.
573. Create fire-and-forget tasks without error handling.
574. Ignore cancellation.
575. Swallow `CancelledError`.
576. Leave tasks running after request cancellation.
577. Share non-thread-safe clients across threads.
578. Share DB sessions across threads/tasks when not safe.
579. Share mutable global state across workers without locks or process-safe design.
580. Use multiprocessing with pickled objects from untrusted sources.
581. Fork after threads are running unless the runtime is designed for it.
582. Use unbounded queues.
583. Use unbounded caches.
584. Use unbounded connection pools.
585. Use unbounded recursion.
586. Use unbounded regex on untrusted input.
587. Use catastrophic-backtracking regexes on untrusted input.
588. Read entire request bodies into memory without limits.
589. Read entire files into memory without limits.
590. Parse huge JSON/XML/YAML without limits.
591. Decompress untrusted data without size limits.
592. Spawn subprocesses per request without quotas.
593. Launch background jobs without idempotency.
594. Launch background jobs without retries/dead-letter behavior.
595. Launch background jobs without ownership and cancellation policy.
596. Ignore backpressure.
597. Ignore rate limits.
598. Ignore file descriptor exhaustion.
599. Ignore memory exhaustion.
600. Ignore disk exhaustion.
601. Ignore denial-of-wallet risk in agent or LLM loops.

## Types, interfaces, and readability

1. Leave public functions in shared code with ambiguous argument/return types when types are knowable.
2. Use `Any` to silence type problems.
3. Use `# type: ignore` without a precise reason.
4. Ignore type-checker errors in changed code.
5. Use misleading names like `data`, `obj`, `thing`, `tmp`, `res`, `handle`, or `manager` for important concepts.
6. Use one-letter variables outside tiny local scopes.
7. Hide domain concepts in nested dicts instead of named types/classes.
8. Return multiple unrelated shapes from the same function.
9. Return `None` for failure without documenting it.
10. Make functions depend on ambient globals instead of explicit parameters.
11. Use mutable global state for request/user/session/tenant data.
12. Write giant functions, giant classes, or “god modules.”
13. Mix I/O, parsing, validation, authorization, business logic, and persistence in one function.
14. Make agent-unfriendly code where behavior is impossible to inspect locally.

## XML, archives, and file parsing

1. Parse untrusted XML with unsafe defaults when entity expansion, external entities, or decompression bombs are possible.
2. Extract `.tar`, `.tar.gz`, `.zip`, wheels, model files, or uploaded archives directly into app directories.
3. Call `tarfile.extract()` or `extractall()` on untrusted archives without inspection, sandboxing, explicit filters, and path validation.
4. Trust archive member names.
5. Trust file extensions, MIME types, or `Content-Type` headers alone.
6. Decompress user uploads without size, file-count, nesting, and ratio limits.
7. Open uploaded files with libraries that may execute macros, scripts, formulas, or external references.
8. Process PDFs/images/media without resource limits.
9. Accept serialized ML model files from users and load them directly.
10. Treat “it’s just a CSV” as safe; CSV formula injection is real when files are opened in spreadsheets.

## References

[1] https://www.veracode.com/blog/genai-code-security-report/ "Insights from 2025 GenAI Code Security Report"
[2] https://docs.python.org/3/library/functions.html?utm_source=chatgpt.com "Built-in Functions"
[3] https://docs.python.org/3/library/pickle.html "pickle — Python object serialization — Python 3.14.5rc1 documentation"
[4] https://bandit.readthedocs.io/en/latest/plugins/b506_yaml_load.html?utm_source=chatgpt.com "B506: yaml_load - Bandit documentation"
[5] https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html "SQL Injection Prevention - OWASP Cheat Sheet Series"
[6] https://docs.python.org/3/library/subprocess.html "subprocess — Subprocess management — Python 3.14.5rc1 documentation"
[7] https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html "Secrets Management - OWASP Cheat Sheet Series"
[8] https://github.com/OWASP/CheatSheetSeries/blob/master/cheatsheets/Logging_Vocabulary_Cheat_Sheet.md?utm_source=chatgpt.com "Application Logging Vocabulary Cheat Sheet"
[9] https://bandit.readthedocs.io/en/latest/blacklists/blacklist_calls.html "blacklist_calls — Bandit  documentation"
[10] https://docs.python.org/3/library/secrets.html?utm_source=chatgpt.com "Generate secure random numbers for managing secrets"
[11] https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html?utm_source=chatgpt.com "Password Storage Cheat Sheet"
[12] https://flask.palletsprojects.com/en/stable/debugging/?utm_source=chatgpt.com "Debugging Application Errors — Flask Documentation (3.1.x)"
[13] https://pip.pypa.io/en/stable/topics/repeatable-installs/ "Repeatable Installs - pip documentation v26.1.1"
[14] https://cheatsheetseries.owasp.org/cheatsheets/AI_Agent_Security_Cheat_Sheet.html "AI Agent Security - OWASP Cheat Sheet Series"
[15] https://docs.python.org/3/library/ast.html?utm_source=chatgpt.com "ast — Abstract syntax trees"
[16] https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html?utm_source=chatgpt.com "Input Validation Cheat Sheet"
[17] https://docs.github.com/en/code-security/tutorials/remediate-leaked-secrets/remediating-a-leaked-secret "Remediating a leaked secret in your repository - GitHub Docs"
[18] https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html?utm_source=chatgpt.com "Logging Cheat Sheet"
[19] https://docs.python.org/3/library/tempfile.html "tempfile — Generate temporary files and directories — Python 3.14.5rc1 documentation"
[20] https://docs.github.com/actions/security-guides/using-secrets-in-github-actions "Using secrets in GitHub Actions - GitHub Docs"
[21] https://mypy.readthedocs.io/?utm_source=chatgpt.com "mypy 1.20.2 documentation"
[22] https://www.reddit.com/r/Python/comments/o2pcj1/what_are_best_practices_with_pytest/ "What are best practices with Pytest? : r/Python"
[23] https://docs.astral.sh/ruff/ "Ruff"
[24] https://arxiv.org/abs/2512.03262?utm_source=chatgpt.com "Is Vibe Coding Safe? Benchmarking Vulnerability of Agent-Generated Code in Real-World Tasks"
[25] https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html?utm_source=chatgpt.com "SQL Injection Prevention Cheat Sheet"
[26] https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html?utm_source=chatgpt.com "Cross Site Scripting Prevention Cheat Sheet"
[27] https://docs.python.org/3/library/tarfile.html "tarfile — Read and write tar archive files — Python 3.14.5rc1 documentation"
[28] https://docs.github.com/code-security/secret-scanning/about-secret-scanning?utm_source=chatgpt.com "About secret scanning"
[29] https://docs.python.org/3/library/ssl.html "ssl — TLS/SSL wrapper for socket objects — Python 3.14.5rc1 documentation"
[30] https://flask.palletsprojects.com/en/stable/web-security/?utm_source=chatgpt.com "Security Considerations — Flask Documentation (3.1.x)"
[31] https://owasp.org/www-community/attacks/Path_Traversal?utm_source=chatgpt.com "Path Traversal"
[32] https://pip.pypa.io/en/stable/topics/secure-installs/?utm_source=chatgpt.com "Secure installs - pip documentation v26.1"
[33] https://bandit.readthedocs.io/en/latest/plugins/b101_assert_used.html?utm_source=chatgpt.com "B101: assert_used - Bandit documentation - Read the Docs"
[34] https://peps.python.org/pep-0008/?utm_source=chatgpt.com "PEP 8 – Style Guide for Python Code"
[35] https://docs.python.org/3/library/unittest.html?utm_source=chatgpt.com "unittest — Unit testing framework"
[36] https://www.reddit.com/r/vibecoding/comments/1s9gj4g/vibe_coding_security/?utm_source=chatgpt.com "Vibe coding security. : r/vibecoding"
[37] https://docs.github.com/code-security/code-scanning/introduction-to-code-scanning/about-code-scanning-with-codeql?utm_source=chatgpt.com "About code scanning with CodeQL"
[38] https://docs.python.org/3/library/security_warnings.html "Security Considerations — Python 3.14.5rc1 documentation"
[39] https://semgrep.dev/docs/cheat-sheets/python-code-injection "Code Injection in Python | Semgrep"
[40] https://semgrep.dev/docs/cheat-sheets/python-command-injection "Command Injection in Python | Semgrep"
[41] https://docs.github.com/code-security/secret-scanning/about-secret-scanning "About secret scanning - GitHub Docs"
[42] https://docs.python.org/3/library/random.html "random — Generate pseudo-random numbers — Python 3.14.5rc1 documentation"
[43] https://owasp.org/www-community/attacks/Path_Traversal "Path Traversal | OWASP Foundation"
[44] https://docs.python.org/3/library/zipfile.html "zipfile — Work with ZIP archives — Python 3.14.5rc1 documentation"
[45] https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html "File Upload - OWASP Cheat Sheet Series"
[46] https://docs.python.org/3/library/http.server.html "http.server — HTTP servers — Python 3.14.5rc1 documentation"
[47] https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html "Logging - OWASP Cheat Sheet Series"
[48] https://docs.gitlab.com/user/application_security/sast/ "Static application security testing (SAST) | GitLab Docs"
[49] https://developers.openai.com/api/docs/guides/agent-builder-safety "Safety in building agents | OpenAI API"
[50] https://openai.com/index/designing-agents-to-resist-prompt-injection/ "Designing AI agents to resist prompt injection | OpenAI"
[51] https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html "Input Validation - OWASP Cheat Sheet Series"
[52] https://cheatsheetseries.owasp.org/cheatsheets/Secure_Code_Review_Cheat_Sheet.html "Secure Code Review - OWASP Cheat Sheet Series"
[53] https://www.reddit.com/r/learnpython/comments/doalzw/can_using_subprocesscall_with_shelltrue_be_any/ "Can using subprocess.call with shell=True be any *worse* than using a shell script? : r/learnpython"
[54] https://arxiv.org/abs/2510.26103 "[2510.26103] Security Vulnerabilities in AI-Generated Code: A Large-Scale Analysis of Public GitHub Repositories"
[55] https://docs.python.org/3/library/secrets.html "secrets — Generate secure random numbers for managing secrets — Python 3.14.5rc1 documentation"
[56] https://cheatsheetseries.owasp.org/cheatsheets/Injection_Prevention_Cheat_Sheet.html "Injection Prevention - OWASP Cheat Sheet Series"
[57] https://pip.pypa.io/en/stable/topics/secure-installs/ "Secure installs - pip documentation v26.1.1"
[58] https://docs.github.com/en/actions/reference/security/secure-use "Secure use reference - GitHub Docs"
[59] https://peps.python.org/pep-0020/ "PEP 20 – The Zen of Python | peps.python.org"
[60] https://docs.astral.sh/ruff/rules/ "Rules | Ruff"
[61] https://peps.python.org/pep-0008/ "PEP 8 – Style Guide for Python Code | peps.python.org"
[62] https://docs.python.org/3/library/sqlite3.html "sqlite3 — DB-API 2.0 interface for SQLite databases — Python 3.14.5rc1 documentation"
[63] https://www.reddit.com/r/learnpython/comments/1edtxdv/why_is_exec_and_eval_not_considered_good_practice/ "Why is exec() and eval() not considered good practice? : r/learnpython"
[64] https://docs.github.com/en/copilot/get-started/best-practices "Best practices for using GitHub Copilot - GitHub Docs"
[65] https://pandas.pydata.org/docs/reference/api/pandas.DataFrame.query.html?utm_source=chatgpt.com "pandas.DataFrame.query — pandas 3.0.2 documentation"
[66] https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html "Password Storage - OWASP Cheat Sheet Series"
[67] https://owasp.org/Top10/2021/ "OWASP Top 10:2021"
[68] https://docs.python.org/3/tutorial/errors.html "8. Errors and Exceptions — Python 3.14.5rc1 documentation"
[69] https://github.com/semgrep/semgrep "GitHub - semgrep/semgrep: Lightweight static analysis for many languages. Find bug variants with patterns that look like source code. · GitHub"
[70] https://arxiv.org/html/2310.02059v2 "Security Weaknesses of Copilot Generated Code in GitHub"
[71] https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/stable-en/02-checklist/05-checklist "OWASP Secure Coding Practices - Quick Reference Guide | Secure Coding Practices | OWASP Foundation"
[72] https://docs.python.org/3/library/xml.html "XML Processing Modules — Python 3.14.5rc1 documentation"
[73] https://docs.github.com/code-security/dependabot/dependabot-alerts/about-dependabot-alerts "About Dependabot alerts - GitHub Docs"
[74] https://openssf.org/blog/2024/08/12/mitigating-attack-vectors-in-github-workflows/ "Mitigating Attack Vectors in GitHub Workflows – Open Source Security Foundation"
[75] https://github.blog/changelog/2025-08-15-github-actions-policy-now-supports-blocking-and-sha-pinning-actions/ "GitHub Actions policy now supports blocking and SHA pinning actions - GitHub Changelog"
[76] https://docs.python.org/3/reference/simple_stmts.html?utm_source=chatgpt.com "7. Simple statements"
[77] https://docs.github.com/code-security/code-scanning/introduction-to-code-scanning/about-code-scanning-with-codeql "About code scanning with CodeQL - GitHub Docs"
[78] https://research.gatech.edu/bad-vibes-ai-generated-code-vulnerable-researchers-warn "Bad Vibes: AI-Generated Code is Vulnerable, Researchers Warn | Research"
