# SOC-Ready Evidence Shell

Jankurai is designed to bridge the gap between engineering artifacts and compliance audits. While **we do not claim automatic SOC 2 certification**, we generate "SOC-ready" engineering evidence that natively maps to standard audit controls.

## Control Mapping

### Change Management
- **SOC Control:** All changes must be tracked, reviewed, and approved.
- **Jankurai Evidence:** The `jankurai proof-receipt` schema maps every commit to a specific lane execution, proving that the required validations ran before merge.

### Access Control & Least Privilege
- **SOC Control:** Access to sensitive environments must be restricted.
- **Jankurai Evidence:** The `owner-map.json` and agent permission profiles define strict boundaries on who (and which AI agents) can mutate specific subsystems. Overbroad agency is flagged via `HLT-012`.

### Vulnerability Management
- **SOC Control:** Systems must be continuously scanned for known vulnerabilities.
- **Jankurai Evidence:** The `target/jankurai/security/evidence.json` envelope normalizes outputs from SAST, SCA, and secret scanners. Findings are mapped to `repo-score.json` for continuous visibility.

### Incident Response & Observability
- **SOC Control:** Systems must log operational states for anomaly detection and response.
- **Jankurai Evidence:** The `HLT-017-OPAQUE-OBSERVABILITY` rule ensures that autonomous repair loops and system state changes leave traceable trails (`repair-receipts`).

### Release Approvals & Supply Chain
- **SOC Control:** Software releases must be authorized and traceable to source code.
- **Jankurai Evidence:** SLSA provenance generation, SBOM generation (Syft), and locked CI dependencies (flagged via `HLT-020-CI-HARDENING-GAP`).

## Philosophy
We produce deterministic, machine-readable JSON files (`evidence.json`, `repo-score.json`, `proof-receipts`) so compliance teams can build automated assertions on top of the engineering workflow, eliminating the need for manual screenshot gathering.
