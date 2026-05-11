# Source Notes

Last reviewed: 2026-04-23

## V7 Claim Discipline

- The main manuscript is a review and operating standard. It intentionally does not claim original local empirical proof of the standard.
- Private/local artifacts and unpublished token deltas are excluded from the public manuscript, citation index, executive brief, and public claim ledger.
- Quantitative statements must be one of four things: external evidence, official documentation, project-reported tooling evidence, or proposed doctrine.
- Project README claims can motivate tooling hypotheses, but they are not peer-reviewed evidence unless independently evaluated.
- Exact function, file, trait, patch, instruction, and token budgets are review triggers, not universal scientific laws.

## Agent Literature Notes

- SWE-bench and SWE-agent are foundational because they frame repository repair as an interactive systems problem rather than a pure code-generation problem.
- RepoBench, RepoExec, Agentless, LocAgent, RepoGraph, RIG, and RepoReason motivate locality, repository maps, graph guidance, staged localization, and low integration width.
- Rust-SWE-bench/RUSTFORGER is the central Rust-specific repository-agent source. Use it for Rust repair difficulty, reproduction, tracing, and type/trait-aware evaluation claims.
- RustEvo2 supports version-aware and retrieval-aware Rust API claims. Use it for API drift and after-cutoff API usage, not broad repository architecture claims.
- CRUST-Bench supports safe-migration difficulty and repository-level Rust generation difficulty. Do not use it as evidence that ordinary greenfield Rust development is impossible.
- RustMap and AutoVerus are adjacent Rust evidence. RustMap supports migration/decomposition reasoning; AutoVerus supports proof-generation direction. Neither validates the whole MSS standard.
- Where Do AI Coding Agents Fail? strengthens the case for failure taxonomies and intermediate failure analysis instead of pass-rate-only evaluation.
- ContextBench, AGENTS.md evaluations, long-context bug-fixing, and Anthropic context engineering are the strongest support for "minimum high-signal context" rather than "dump more text."
- On the Impact of AGENTS.md Files should be paired with Evaluating AGENTS.md because instruction files can affect runtime, output tokens, and success differently.
- OpenAI's SWE-bench Verified critique prevents the paper from leaning on saturated or contaminated benchmark narratives.
- SWE-Bench Pro, SWE-EVO, SWE-CI, Multi-SWE-bench, RACE-bench, and SWE-PolyBench justify treating long-horizon evolution, maintainability, intermediate reasoning, language choice, and multi-language boundaries as evaluation axes.
- SWE-Effi and SUSVIBES justify keeping cost and security-gated correctness separate from visible pass rate.
- SWE-Pruner strengthens the token section because it supports task-aware pruning rather than blind compression, and it reinforces the claim that the biggest savings come from reducing wasted read context.
- METR's productivity trial and Professional Software Developers Don't Vibe, They Control justify keeping human review burden and workflow control in the thesis.
- SWE-Skills-Bench justifies narrow skills and discourages giant general-purpose skill packs.
- Rethinking Agent-Generated Tests justifies reviewing generated tests as artifacts, not treating them as proof by default.
- ReCUBE supports evaluating code understanding and intermediate comprehension, not only final patch pass rate.

## Harness and Tool Notes

- Anthropic Effective Context Engineering is the cleanest source for the "smallest high-signal token set" language.
- Anthropic Effective Harnesses is the cleanest source for feature ledgers, progress files, init scripts, and end-to-end verification across context windows.
- Anthropic Writing Effective Tools for Agents supports treating tools as contracts with nondeterministic agents and optimizing outputs for token efficiency.
- OpenAI Harness Engineering is useful because it makes cleanup loops, local observability, and repository-embedded rules explicit instead of treating them as prompt folklore.
- Tool output compression belongs in the paper only when it preserves exit codes, raw-output recovery, decisive evidence, redaction status, and human-review quality.
- Low-token narration belongs in the paper only as a harness policy. It should never be presented as permission to compress commands, code, paths, error codes, advisories, or other decisive evidence.

## Why Rust Notes

- The Rust Book's performance chapter is the cleanest official source for zero-cost abstraction framing.
- The Rust Book's concurrency chapter is the cleanest official source for compile-time memory and concurrency safety framing.
- Cargo docs matter because agent productivity depends on machine-readable repo structure, additive feature policy, target boundaries, build caching, timings, and deterministic validation loops.
- Rust API Guidelines matter because they convert "boring good APIs" into named qualities: predictability, type safety, dependability, debuggability, and future-proofing.
- PyO3, maturin, Buf, tonic/prost, OpenAPI TypeScript, Zod, and wasm-pack matter because the paper's language-pairing guidance depends on typed, generated, or packaged boundaries rather than ad hoc glue.
- Google Android, Cloudflare, AWS, Bottlerocket, Firecracker, Discord, Dropbox, and Hugging Face Tokenizers provide production viability evidence. They should support Rust maturity claims, not direct proof of agent efficiency.
- RustAssistant is useful Rust-specific repair evidence because it focuses on compiler-error repair, but it should not be used to claim full repository-agent readiness.
- Stack Overflow survey data supports popularity and admiration framing, but it should not be used as a proxy for production adoption or agent readiness.
- RustBelt and Rudra support the safety section by showing that Rust's safety story depends on formal reasoning and careful unsafe usage, not slogans.

## Testing, Caching, and Tooling Notes

- cargo-nextest belongs in the fast and medium lanes because it is designed for fast and reliable Rust test execution.
- proptest, insta, and trybuild map cleanly to state-space, output-surface, and API-misuse proof modes.
- cargo-fuzz, Miri, Loom, sanitizers, cargo-mutants, cargo-llvm-cov, Criterion, cargo-deny, cargo-hack, cargo-semver-checks, cargo-audit, cargo-vet, and cargo-geiger belong in medium, deep, security, or release lanes depending on risk and runtime cost.
- cargo-msrv, cargo-hakari, cargo-public-api, cargo-auditable, just, ast-grep, cross, gitleaks, detect-secrets, Syft, Grype, actionlint, zizmor, Biome, Vitest, and Playwright belong in the supplement tool catalog because they reduce proof cost, ambiguity, setup friction, or security blind spots in specific roles.
- thiserror, anyhow, miette, and tracing support diagnostic-first failure surfaces.
- SQLx, Schemars, Utoipa, ts-rs, Specta, and Serde support generated contracts and should be framed as ways to reduce duplicated semantic surface, not as a crate catalog.
- Build-loop acceleration should cite Cargo build cache, Cargo timings, sccache, cargo-chef, bacon, targeted package checks, default-members, and stable target directories.
- Whole-workspace validation should be reserved for public-contract, feature, security, dependency, or cross-crate changes; local pure-logic edits should start with narrower proof loops.
- Google small-change guidance and SPACE productivity framing support patch-width and review-burden discussion, but the paper's numeric budgets remain proposed review triggers.
- Readability and complexity sources support treating code shape as a review and comprehension concern, but they do not directly justify the paper's exact numeric thresholds.

## Token Economy Notes

- Token reduction is useful only when correctness, security-gated validation, auditability, and human review are preserved.
- V7 uses validated token savings: gross compression is raw-output reduction, net run saving is total model-visible reduction, and validated saving is lower expected tokens to security-gated hidden pass without review/audit regression.
- ETTS claims must name per-attempt token cost `C_i`, first hidden-pass attempt `S`, first security-gated hidden-pass attempt `S_sec`, retry cap `R`, and which token classes are included.
- RTK: Rust Token Killer is project-reported Rust tooling evidence for command-output filtering. Its reduction and overhead claims should be labeled project-reported.
- Every compressed command summary must preserve exit code, raw-output path, raw-output hash or equivalent audit handle, redaction status, and decisive failure facts.
- Rust-specific token-saving methods should emphasize Cargo JSON diagnostics, Cargo metadata, feature-matrix pruning, dependency summaries, tracing span summaries, generated contract diffs, and failure-only test output.
- Token-saving tricks should be organized as a hierarchy: structural savings first, proof-lane savings second, safe output shaping third, terse narration last.
- Never let token filtering hide security failures, panics, denied dependencies, unsafe findings, fuzz seeds, advisory IDs, or secret exposure.

## V8 Future-Work Notes

- The V7 concept list collapses into three families, not nine flagship systems: negative-context compilers, capsule/shadow-workspace materialization, and active distinguishing probes.
- `cargo-mss` remains the strongest untested flagship concept, but it should now be framed as a negative-context compiler rather than a static manifest emitter. It should emit repair capsules, ignore certificates, edit grammars, widening predicates, and optional shadow workspaces.
- Capsule or shadow-workspace projection is a materialization mode of `cargo-mss`, not a separate flagship. `cargo-needle`-style distinguishing probes are a query mode of `cargo-mss`, not a separate flagship.
- ProofLens is the strongest untested token-efficiency concept. It should be proof-preserving compression, not generic summarization, and it must preserve raw-output path, hash, exit code, spans, failing tests, security evidence, and redaction status.
- ProofLens should also prefer structured proof or operational-state packets over raw-log summaries whenever the underlying system can expose them directly.
- `cargo-obligation-cache` is the strongest third flagship concept because repeated proof work remains a large avoidable token sink after owner routing and proof-preserving compression. It should cache proof obligations conservatively by public API, contract hash, feature set, target, tool version, unsafe-ledger state, dependency set, and relevant tests.
- `AgentDiagnostic` remains a strong secondary recovery and review concept. It should combine typed failure routing with patch receipts that state owners changed, contracts touched, commands run, raw-output hashes, generated files, unsafe/dependency changes, and residual risk.
- Former loose lists of proposed `cargo-*` tools should not appear as if those tools exist. They should be folded into the three research concepts or treated as possible implementation components.
- Each future-work concept needs a falsification criterion. A concept is not successful if it merely reduces tokens while worsening hidden pass, security-gated pass, illegal touch rate, or human review.

## InsForge Notes

- InsForge should appear only as project-reported practitioner evidence, not as peer-reviewed proof of token savings.
- Its main value to the paper is conceptual, not branding: it shows semantic-layer and structured-state shaping as a complement to post-hoc output filtering.
- The March 2, 2026 MCPMark v2 benchmark post is usable only as dated, project-reported benchmark context. Do not convert it into a universal token-savings claim.

## WarpOS and CrateAtlas Notes

- WarpOS belongs in the paper only as a bounded runtime companion layer to MSS. It shows what a live mediation plane can observe and shape on the wire; it does not prove the global thesis of the paper.
- The engineering A--I taxonomy is too detailed for the main manuscript. In the main paper it should collapse into five runtime stages: Suppress, Compress, Reuse, Steer, and Guard.
- Any WarpOS token number in the main paper must be labeled as one of: observed corpus value, directly derived local estimate, or projected intervention opportunity.
- Suppress and Compress are the highest-trust runtime stages because they can be grounded in duplicated requests, inert control-plane traffic, repeated schemas, repeated environment context, and structured tool-output surfaces already visible in the corpus.
- Reuse, Steer, and Guard should be discussed as conditional stages with stronger caveats. Reuse depends on conservative state equivalence; Steer depends on routing and hazard models; Guard is mixed, with rule-based interrupts higher trust than learned interventions.
- The ISSUE-05 offline result of 18/34 invalid runs prevented is a ceiling for online steering, not a deployment claim. Any online expectation must be phrased as projected and shadow-mode-first.
- CrateAtlas should be treated as graph/localization substrate, not as a sixth runtime stage or a flagship concept. Its value is feeding owner ranking, dependency and reference slicing, top-K path ranking for shell-packet summaries, symbol/path existence checks, and repair-capsule or ignore-certificate generation.
- The paper should prefer WarpOS local artifact families over vague one-off references: telemetry architecture, five-agent telemetry corpus, intervention catalog, and supplement derivation tables.

## Security Notes

- SUSVIBES is the core benchmark support for separating functional correctness from security-gated correctness.
- OWASP LLM Top 10 and OWASP AI Agent Security Cheat Sheet justify treating prompt injection, tool abuse, data exfiltration, memory poisoning, excessive autonomy, denial of wallet, and supply-chain risk as agent-specific security concerns.
- NIST SSDF and OpenSSF Scorecard are not Rust-specific, but they provide audit vocabulary for secure development and dependency risk.
- Rustonomicon, cargo-geiger, Miri, sanitizers, Kani, cargo-deny, cargo-audit, cargo-vet, and fuzzing should be used together. Unsafe-count and advisory tooling are evidence surfaces, not proof of safety by themselves.

## What the Paper Must Keep Honest

- Rust is not the right language for every layer.
- Rust's learning curve, compile times, `unsafe`, FFI, macro expansion, and async complexity remain real costs.
- Minimum semantic surface is a proposed operating doctrine informed by evidence, not a proven universal optimum.
- Token minimization can be unsafe when it removes evidence needed for debugging, security, reproducibility, or review.
- Functional correctness does not imply security-gated correctness.
- RTK should remain a project/tooling example until independent measurements exist.
- Rust adoption case studies should be framed as substrate maturity evidence, not direct proof of agent efficiency.
- Tool catalogs should stay role-based: include a tool only when it reduces semantic surface, proof cost, security ambiguity, or boundary ambiguity.
