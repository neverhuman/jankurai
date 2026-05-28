# jankurai

`jankurai` is the Rust CLI for the Jankurai agent-native repository control plane.

Install from the release installer first, then fall back to source builds if
you need to work from the workspace directly:

```bash
curl -fsSL https://github.com/neverhuman/jankurai/releases/download/v1.6.0/jankurai-installer.sh \
  | JANKURAI_RELEASE_TAG=v1.6.0 bash
jankurai --version
```

Fallback source install:

```bash
cargo install --path crates/jankurai --locked
jankurai --version
```

Run a read-only first pass in a target repository:

```bash
jankurai adopt . --mode observe --out target/jankurai/adoption-plan.json --md target/jankurai/adoption-plan.md
jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
```

See the repository README at https://github.com/neverhuman/jankurai for adoption levels, proof lanes, security reporting, and contribution guidance.
