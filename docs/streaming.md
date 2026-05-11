# Streaming Infrastructure

jankurai treats streaming as infrastructure behind contracts, not as stack identity.

Kafka is valid brownfield infrastructure when a system already depends on its durable log, ecosystem, Connect, Streams, client support, and operational proof. It must live behind generated event contracts and observable Rust queue adapters.

Rust-native direction:

- Tansu is the strongest Kafka-compatible Rust candidate to watch and evaluate.
- Apache Iggy and Fluvio are greenfield Rust-native streaming candidates, not Kafka-wire-compatible drop-ins.
- RobustMQ is useful evaluation material, but Kafka protocol support is not yet complete enough to treat as replacement proof.

Reference-platform rule:

- broker clients stay behind declared queue adapters and generated contracts
- streaming proof is about boundary control and repairability, not about adopting Kafka as identity
- `HLT-019-STREAMING-RUNTIME-DRIFT` fires when broker code escapes adapter boundaries

The evaluation pack should measure protocol compatibility, produce/fetch correctness, crash recovery, consumer groups, replication, retention, compaction, transactions, ACLs, quotas, observability, migration tooling, and chaos behavior.

Rule `HLT-019-STREAMING-RUNTIME-DRIFT` fires when broker clients appear outside queue adapters or when Kafka is treated as stack identity without a brownfield exception and migration path.
