set shell := ["bash", "-lc"]

default: check

fast:
    mkdir -p target/jankurai/coverage
    cargo run -p jankurai -- coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md
    cargo check -p jankurai
    cargo run -p jankurai -- . --json target/jankurai/fast-score.json --md target/jankurai/fast-score.md

setup:
    npm ci

versions:
    cargo run -p jankurai -- versions

ux-qa:
    npm --workspace @jankurai/ux-qa run build
    npm --workspace @jankurai/ux-qa run test

quality:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo test --workspace --all-targets --all-features --locked
    npm --workspace @jankurai/ux-qa run build
    npm --workspace @jankurai/ux-qa run test

check: quality security-strict conformance score paper

validate: check

paper:
    latexmk -pdf -interaction=nonstopmode -halt-on-error -outdir=paper paper/jankurai.tex

score:
    mkdir -p target/jankurai/coverage
    cargo run -p jankurai -- coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md
    cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md --score-history agent/score-history.jsonl --score-history-csv agent/score-history.csv

audit-fast base="origin/main":
    cargo run -p jankurai -- audit . --changed-fast --changed-from {{base}} --json target/jankurai/audit-fast.json --md target/jankurai/audit-fast.md --timings-json target/jankurai/audit-timings.json

compat:
    cargo test -p jankurai --test report_compatibility_guard

conformance:
    test -f conformance/README.md
    test "$(find conformance/fixtures -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')" = "10"
    test "$(find conformance/expected -type f -name '*.json' | wc -l | tr -d ' ')" = "12"
    cargo run -p jankurai -- conformance run --fixtures conformance/fixtures --expected conformance/expected --out target/jankurai/conformance-results.json --md target/jankurai/conformance-results.md --tex paper/tex/generated/conformance_results_table.tex
    cargo test -p jankurai conformance

self-audit:
    cargo run -p jankurai -- audit . --self-audit --json target/jankurai/self-audit.json --md target/jankurai/self-audit.md

security:
    cargo run -p jankurai -- security run . --out target/jankurai/security/evidence.json

security-strict:
    cargo run -p jankurai -- security run . --strict --profile ci --out target/jankurai/security/evidence.json

security-bash:
    bash tools/security-lane.sh

phase12:
    mkdir -p target/jankurai/public
    cargo run -p jankurai -- bench . --out target/jankurai/p12-benchmark-report.json --md target/jankurai/p12-benchmark-report.md
    cargo run -p jankurai -- certify . --out target/jankurai/p12-certification.json --md target/jankurai/p12-certification.md
    cargo run -p jankurai -- govern . --out target/jankurai/p12-governance-policy.json --md target/jankurai/p12-governance-policy.md
    cargo run -p jankurai -- publish . --certification target/jankurai/p12-certification.json --benchmark target/jankurai/p12-benchmark-report.json --governance target/jankurai/p12-governance-policy.json --out target/jankurai/public/p12-public-evidence.json --md target/jankurai/public/p12-public-evidence.md --badge-json target/jankurai/public/jankurai-badge.json --badge-svg target/jankurai/public/jankurai-badge.svg

phase13:
    mkdir -p target/jankurai
    cargo run -p jankurai -- optimize . --mode all --out target/jankurai/p13-optimization-report.json --md target/jankurai/p13-optimization-report.md
    cargo run -p jankurai -- exceptions expire . --warning-days 7 --strict --out target/jankurai/p13-exception-expiry.json --md target/jankurai/p13-exception-expiry.md

cov:
    mkdir -p target/coverage
    cargo llvm-cov --workspace --all-features --locked --lcov --output-path target/coverage/lcov.info
    cargo llvm-cov report --json --output-path target/coverage/coverage.json
    cargo llvm-cov report --summary-only | tee target/coverage/summary.txt

test-surface:
    bash scripts/render-test-surface.sh

test-surface-check:
    bash scripts/render-test-surface.sh --check

# Local mirror of CI. Each recipe reproduces a GitHub Actions job so
# breakage is caught before push, never first on GitHub.
ci-doctor:
    bash scripts/ci-doctor.sh

ci-quick:
    bash scripts/ci-local.sh quick

ci-coverage:
    bash scripts/ci-local.sh coverage

ci-audit:
    bash scripts/ci-local.sh audit

ci-release:
    bash scripts/ci-local.sh release

ci:
    bash scripts/ci-local.sh all

ci-container:
    bash ops/ci/run-in-container.sh "bash ops/ci/audit.sh"

zizmor:
    zizmor .github/workflows

tuiwright-test:
    cargo test -p tuiwright --lib
    cargo test -p tuiwright --test smoke -- --test-threads=1
    cargo test -p tuiwright-cli

tuiwright-demo:
    cargo run -p tuiwright-demo
