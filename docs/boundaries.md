# jankurai Boundaries

The jankurai standard rejects ambiguous ownership. Each layer must own one
kind of truth and must not leak into adjacent layers.

| Layer | Owns | Must not own |
| --- | --- | --- |
| TypeScript web | UI, forms, route state, local validation, generated clients | secrets, durable truth, direct DB writes, core authz, handwritten DTO mirrors |
| Rust API | transport edge, request normalization, response mapping | domain rules hidden in handlers, scattered SQL |
| Rust domain | IDs, invariants, state machines, pure decisions | I/O, env, time, random, DB, framework types |
| Rust application | commands, authz, idempotency, transactions | UI concerns, provider-specific adapter details |
| Rust adapters | PostgreSQL, queue/streaming clients, external APIs, filesystem, env | domain rules, event schema ownership |
| PostgreSQL | constraints, migrations, indexes, transactional truth | app orchestration |
| Python AI/data exception | advanced ML/data library work behind typed boundaries | product truth, authz, repo tools, proof lanes, general backend glue, direct production DB writes |
| Ops/security | CI, OTel, SBOM, SCA, secret scanning, provenance | hidden product logic |

Boundary exceptions belong in `docs/exceptions/` with owner, reason,
expiration, proof lane, and repair guidance.

The blessed default stack is Rust core, TypeScript/React/Vite product surface, PostgreSQL durable truth, generated contracts, and exception-only Python AI/data service. The repo should treat that stack as the default control plane, not as a stylistic suggestion. New repository tools, proof lanes, core behavior, authorization, and durable writes must be Rust-first. Agents must not add Python unless a rare dated advanced-ML/data exception explicitly approves it and keeps it boxed under `python/ai-service`.

## Queue And Streaming Boundary

Event schemas belong in `contracts/`. Generated event types belong in declared generated zones. Broker clients belong in `crates/adapters/queues` or another path explicitly declared in `agent/boundaries.toml`.

Kafka is allowed as brownfield infrastructure behind adapters. It should not become stack identity. Tansu is the leading Kafka-compatible Rust candidate to evaluate; Apache Iggy and Fluvio are stronger greenfield Rust-native candidates, not Kafka-wire-compatible drop-ins. A Kafka exception needs owner, expiry, brownfield reason, and migration path.
