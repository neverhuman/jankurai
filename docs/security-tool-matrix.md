# Security Tool Matrix

Jankurai orchestrates different security tools based on the environment to balance developer velocity and security posture.

## Tool Profiles

### Local "Fast" Lane
The local fast lane optimizes for speed and quick feedback. It runs pre-commit and lightweight scanners.
- **Required:**
  - `gitleaks` (Secret scanning)
- **Advisory (Optional):**
  - `cargo audit` (Rust advisories)
  - `npm audit` (JS dependencies)

### Continuous Integration (CI)
CI acts as the primary gatekeeper before code merges to main. It runs a comprehensive suite.
- **Required:**
  - `gitleaks`
  - `cargo audit`
  - `npm audit`
  - `zizmor` (GitHub Actions hardening)
- **Recommended:**
  - `Semgrep` / `CodeQL` (SAST)
  - `cargo deny` (License and dependency constraints)

### Release (Nightly / CD)
The release pipeline generates immutable provenance and supply chain artifacts.
- **Required:**
  - `Syft` (SBOM generation)
  - `Grype` / `Trivy` (Container and artifact vulnerability scanning)
  - `OpenSSF Scorecard` (Posture scanning)
  - `SLSA/Cosign` (Release provenance signing)

## Configuration
The tool matrix is configured in `agent/security-policy.toml`. 
Tools missing from the local environment but listed as "advisory" will not fail the build unless the `--strict` flag is explicitly passed to `jankurai security run`.
