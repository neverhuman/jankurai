# Dated Bibliography

Last reviewed: 2026-04-23

## Coding Agent and Repository Literature

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2023-06 | RepoBench | arXiv | Establishes repository-level completion as distinct from single-file completion |
| 2023-10 | SWE-bench | ICLR 2024 / arXiv | Establishes real GitHub issue repair as a foundational execution-based benchmark |
| 2024-05 | SWE-agent | NeurIPS 2024 / arXiv | Shows agent-computer interface design affects repository navigation, editing, and testing |
| 2024-07 | Agentless | arXiv | Supports staged localization, repair, and validation |
| 2024-10 | RepoGraph | arXiv | Supports repository-level graph navigation instead of blind file traversal |
| 2025-03 | LocAgent | arXiv | Shows graph-guided localization gains |
| 2025-04 | SWE-PolyBench | arXiv | Extends repository-level execution evaluation across multiple languages and task types |
| 2025-07 | SetupBench | arXiv | Isolates environment bootstrap as a distinct agent bottleneck |
| 2025-09 | SWE-Effi | arXiv | Adds token and time cost as first-class evaluation dimensions |
| 2025-09 | SWE-Bench Pro | arXiv | Adds harder long-horizon enterprise-grade tasks and contamination-resistant framing |
| 2025-12 | SWE-EVO | arXiv | Frames real software work as long-horizon codebase evolution |
| 2025-12 | Is Vibe Coding Safe? (SUSVIBES) | arXiv | Separates functional correctness from security-gated correctness |
| 2026-01 | RIG | arXiv | Supports deterministic architectural maps |
| 2026-01 | SWE-Pruner | arXiv | Supports task-aware context pruning and token-economy claims for coding agents |
| 2026-01 | RepoReason | arXiv | Introduces integration-width diagnostics for repository reasoning |
| 2026-02 | ContextBench | arXiv | Shows over-retrieval and under-use of explored context |
| 2026-02 | Evaluating AGENTS.md | arXiv | Shows context files can help or hurt depending on specificity |
| 2026-02 | Limits of Long-Context Reasoning | arXiv | Supports short-context decomposition over brute-force context |
| 2026-02 | OpenAI SWE-bench Verified critique | official research post | Warns about benchmark contamination and flawed tests at frontier capability levels |
| 2026-03 | SWE-Skills-Bench | arXiv | Shows skills help narrowly and can increase token cost or hurt when mismatched |
| 2026-03 | SWE-CI | arXiv | Evaluates maintainability through CI-style codebase evolution |
| 2026-03 | TDAD | arXiv | Supports test-impact mapping for regression reduction |
| 2026-03 | RACE-bench | arXiv | Adds intermediate reasoning ground truth to feature-addition tasks |
| 2026-02 | Rust-SWE-bench / RUSTFORGER | ICSE 2026 / arXiv | Direct Rust repository-level issue-resolution benchmark with Rust-specific failure modes |
| 2025-04 | Multi-SWE-bench | arXiv | Supports cross-language evaluation across Java, TypeScript, JavaScript, Go, Rust, C, and C++ |
| 2025-07 | METR early-2025 productivity RCT | arXiv | Cautions that AI tools can slow experienced developers on selected real tasks |
| 2025-12 | Professional Software Developers Don't Vibe, They Control | arXiv | Frames professional agent use as controlled workflows rather than unbounded delegation |
| 2026-01 | Where Do AI Coding Agents Fail? | arXiv | Provides failure-mode taxonomy for repository-level coding agents |
| 2026-01 | On the Impact of AGENTS.md Files | arXiv | Adds token/runtime context-file evidence that complements Evaluating AGENTS.md |
| 2026-02 | Rethinking Agent-Generated Tests | arXiv | Supports reviewing generated tests rather than treating them as automatic proof |
| 2026-03 | ReCUBE | arXiv | Supports code-understanding and intermediate-comprehension evaluation |

## Rust-Specific Agent and Migration Benchmarks

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2025-03 | RustEvo2 | arXiv | Shows Rust API evolution and after-cutoff APIs remain difficult for LLM code generation |
| 2025-04 | CRUST-Bench | COLM 2025 / arXiv | Shows safe, idiomatic, repository-level C-to-Rust transpilation remains difficult |
| 2025-04 | RustAssistant | ICSE 2025 / Microsoft Research | Shows compiler-guided LLM repair can fix many real Rust compilation errors |
| 2025-03 | RustMap | arXiv | Supports project-scale C-to-Rust migration as decomposition plus dependency reasoning |
| 2024-09 | AutoVerus | arXiv | Supports LLM-assisted proof generation for Rust/Verus as high-assurance adjacent work |

## Harness, Context, Skills, and Tool Guidance

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2024-12-19 | Anthropic Building Effective Agents | official engineering post | Supports simple, composable agent systems before complex frameworks |
| 2025-09-11 | Anthropic Writing Effective Tools for Agents | official engineering post | Treats tools as contracts for nondeterministic agents and stresses token-efficient outputs |
| 2025-09-29 | Anthropic Effective Context Engineering | official engineering post | Anchors the smallest high-signal context principle and progressive disclosure |
| 2025-11-26 | Anthropic Effective Harnesses for Long-Running Agents | official engineering post | Supports feature ledgers, progress files, init scripts, and E2E verification |
| 2026-02 | OpenAI Harness Engineering | official engineering post | Shows agent-first repos need mechanical rules, cleanup loops, and local feedback surfaces |
| 2026-03-24 | Anthropic Harness Design for Long-Running Application Development | official engineering post | Updates long-running harness design with decomposition, handoff artifacts, and evaluator roles |

## Rust and Cargo Primary Sources

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2026-04-22 access | Cargo Book: workspaces, metadata, tree, check, test, features, targets | official docs | Defines attachment points and validation semantics for agent-oriented Rust repos |
| 2026-04-23 access | Cargo Book: build cache and timings | official docs | Supports build-loop acceleration, stable target directories, and bottleneck diagnosis |
| 2026-04-22 access | Rust Book: performance in loops vs iterators | official docs | Supports zero-cost abstraction claims |
| 2026-04-22 access | Rust Book: fearless concurrency | official docs | Supports compile-time memory and concurrency safety claims |
| 2026-04-22 access | Rust API Guidelines | official guidelines | Supports predictable, type-safe, future-proof public APIs |
| 2026-04-22 access | Clippy docs | official docs | Supports lint-backed fast-lane verification |
| 2026-04-23 access | Rustonomicon | official docs | Supports explicit unsafe-invariant and unsafe-boundary treatment |
| 2026-04-23 access | Google Engineering Practices: Small CLs | official guidance | Supports small, reviewable change guidance without claiming exact Rust-agent thresholds |

## Rust Tooling and Framework Sources

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2026-04-22 access | cargo-nextest | project docs | Fast, isolated Rust test execution for the inner verification lane |
| 2026-04-22 access | proptest, insta, trybuild | project docs | Property, snapshot, and compile-fail proof modes |
| 2026-04-22 access | cargo-fuzz, Miri, Loom, sanitizers | project docs | Deep validation for parsers, unsafe code, and concurrency |
| 2026-04-23 access | Kani Rust Verifier | project docs | Supports model-checking in deep lanes for high-risk Rust code |
| 2026-04-23 access | cargo-geiger | project docs | Supports unsafe-surface discovery for security review |
| 2026-04-22 access | cargo-llvm-cov, cargo-mutants, Criterion | project docs | Coverage, mutation, and performance regression proof |
| 2026-04-22 access | cargo-deny, cargo-udeps, cargo-machete, cargo-hack, cargo-semver-checks | project docs | Dependency, feature, and API-surface hygiene |
| 2026-04-23 access | cargo-audit and cargo-vet | project docs | Advisory and supply-chain review for the security lane |
| 2026-04-23 access | cargo-public-api, cargo-about, cargo-msrv, cargo-hakari, cargo-auditable | project docs | Public API, license, MSRV, workspace unification, and binary-audit surfaces |
| 2026-04-23 access | sccache, cargo-chef, bacon, cargo-binstall, cargo-limit, rust-analyzer-mcp, just, ast-grep, cross | project docs | Build-loop acceleration, command routing, diagnostic shaping, semantic search, and cross-target workflows |
| 2026-04-22 access | Axum, Tokio, Tower, SQLx, Serde, tracing | official or project docs | Recommended backend substrate for agent-legible Rust services |
| 2026-04-22 access | thiserror, anyhow, miette | project docs | Typed library errors and rich application diagnostics |
| 2026-04-22 access | Schemars, Utoipa, ts-rs, Specta, Buf, tonic, prost, OpenAPI TypeScript, Zod, PyO3, maturin, wasm-pack | project docs | Generated schemas, protobuf boundaries, TS contracts, Python bridges, and wasm packaging |
| 2026-04-22 access | React, Tauri, Ratatui, Leptos, Dioxus, Biome, Vitest, Playwright | official or project docs | UI, desktop, TUI, and non-Rust proof-surface defaults discussed as doctrine |

## Industry and Systems Case Studies

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2022-12 | Memory Safe Languages in Android 13 | Google official blog | Anchors Android's memory-safe language shift |
| 2023-02 | Hardening firmware across the Android ecosystem | Google official blog | Shows memory-safe language adoption in privileged firmware |
| 2025-11-13 | Rust in Android: move fast and fix things | Google official blog | Provides review, rollback, and vulnerability-density metrics |
| 2025-09 | Cloudflare just got faster and more secure, powered by Rust | Cloudflare official blog | Provides CPU, memory, and latency case-study data |
| 2020-02 | Why Discord is switching from Go to Rust | Discord official engineering blog | Provides hot-path service rewrite and latency-spike elimination case study |
| 2020-03 | Rewriting the heart of our sync engine | Dropbox official engineering blog | Provides Rust rewrite, invariant, determinism, and simulation-testing case study |
| 2022 | Why AWS is the Best Place to Run Rust | AWS official blog | Adoption and operational support context |
| 2020 | Bottlerocket GA | AWS official blog | Thread safety and memory-safety rationale in a production OS |
| 2026-04-22 access | Firecracker official site | official project site | Startup time and memory overhead metrics |
| 2025-04 | RustAssistant publication page | Microsoft Research | Compiler-guided repair performance for Rust |
| 2026-04-22 access | Hugging Face Tokenizers docs | official docs | Rust's relevance to AI infrastructure and tokenization performance |

## Security and Supply Chain Sources

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2022-02 | NIST SP 800-218 SSDF | government standard | Provides auditable secure software development practices |
| 2026-04-23 access | OWASP Top 10 for LLM Applications | official OWASP project | Frames prompt injection, insecure outputs, supply chain, and plugin/tool risks |
| 2026-04-23 access | OWASP AI Agent Security Cheat Sheet | official OWASP guidance | Frames least privilege, human-in-the-loop controls, monitoring, memory/context security, and cost risks |
| 2026-04-23 access | OpenSSF Scorecard | OpenSSF project | Provides automated project-risk and dependency-risk framing |
| 2026-04-23 access | gitleaks, detect-secrets, Syft, Grype, actionlint, zizmor | project docs | Secret scanning, SBOM generation, vulnerability scanning, and GitHub Actions hardening |

## Token Economy Sources

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2026-04-23 access | RTK repository | project repository | Rust CLI proxy for command-output filtering, token analytics, raw-output recovery, and project-reported token reduction |
| 2026-04-23 access | RTK Architecture | project documentation | Documents filtering pipeline, Bash-hook limitation, tee/raw-output behavior, and command rules |
| 2026-04-23 access | Cargo external tools docs | official docs | Supports machine-readable Cargo output and external tool integration |

## Productivity and Unsafe-Rust Research

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2021 | SPACE framework | ACM Queue | Supports treating productivity/readiness as multidimensional rather than a single score |
| 2018 | RustBelt | POPL / PACMPL | Supports formal reasoning about Rust safety foundations and unsafe abstractions |
| 2021 | Rudra | SOSP | Supports explicit unsafe/ecosystem-scale bug analysis for Rust security review |
| 2008 | A Metric for Software Readability | ISSTA | Supports treating readability as a review and comprehension concern |
| 1976 | A Complexity Measure | IEEE TSE | Classic control-flow complexity reference for structured review concerns |
| 2021 access | Cognitive Complexity | SonarSource white paper | Supports treating understandability as separate from raw path count |

## V7 Flagship Future-Work Concepts

| Date | Source | Type | Why it matters |
| --- | --- | --- | --- |
| 2026-04-23 | `paper/artifacts/cargo-mss-concept.md` | proposed artifact | Defines the semantic-surface compiler concept, outputs, metrics, and falsification criteria |
| 2026-04-23 | `paper/artifacts/prooflens-concept.md` | proposed artifact | Defines proof-preserving Rust output compression and the required raw-evidence guardrails |
| 2026-04-23 | InsForge repository | project repo | Practitioner evidence for semantic-layer and structured-state shaping |
| 2026-04-23 | InsForge introduction docs | product docs | Project-reported explanation of structured domain context and MCP surface |
| 2026-04-23 | InsForge deployment post | engineering blog | Project-reported deployment-state shaping example |
| 2026-03-02 | InsForge MCPMark v2 benchmark post | engineering blog | Project-reported benchmark context for token and latency claims |
| 2026-04-23 | `paper/artifacts/cargo-obligation-cache-concept.md` | proposed artifact | Defines conservative proof-obligation reuse, invalidation logic, and evaluation metrics for repeated-proof savings |
