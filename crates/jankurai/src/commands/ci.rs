use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct CiInstallArgs {
    pub repo: PathBuf,
    pub github: bool,
    pub mode: String,
    pub min_score: i32,
    pub baseline: Option<String>,
    pub dry_run: bool,
}

pub fn install(args: CiInstallArgs) -> Result<()> {
    let progress = crate::ui::CliProgress::new("installing CI workflow", 4);
    progress.tick("validate options");
    if !args.github {
        bail!("only `jankurai ci install --github` is currently supported");
    }
    if !matches!(args.mode.as_str(), "observe" | "advisory" | "ratchet") {
        bail!(
            "unknown CI mode `{}`; expected observe, advisory, or ratchet",
            args.mode
        );
    }
    if args.mode == "ratchet" && args.baseline.is_none() {
        bail!("ratchet CI requires --baseline PATH; use agent/baselines/main.repo-score.json after an accepted baseline exists");
    }
    progress.tick("render workflow");
    let path = args.repo.join(".github/workflows/jankurai.yml");
    let rendered = workflow(&args.mode, args.min_score, args.baseline.as_deref());
    if args.dry_run {
        progress.finish("dry-run workflow rendered");
        println!(
            "{}",
            crate::ui::paint(
                crate::ui::Style::Accent,
                format!("# would write {}", path.display()),
                crate::ui::stdout_color_enabled()
            )
        );
        print!("{rendered}");
        return Ok(());
    }
    progress.tick("prepare workflow directory");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    if path.exists() {
        progress.finish("existing workflow preserved");
        println!(
            "{}",
            crate::ui::paint(
                crate::ui::Style::Warn,
                format!("{}: exists; leaving user content unchanged", path.display()),
                crate::ui::stdout_color_enabled()
            )
        );
        return Ok(());
    }
    progress.tick("write workflow");
    fs::write(&path, rendered).with_context(|| format!("write {}", path.display()))?;
    progress.finish("CI workflow installed");
    println!(
        "{}",
        crate::ui::paint(
            crate::ui::Style::Good,
            format!("wrote {}", path.display()),
            crate::ui::stdout_color_enabled()
        )
    );
    Ok(())
}

fn workflow(mode: &str, _min_score: i32, baseline: Option<&str>) -> String {
    let audit_mode = if mode == "ratchet" {
        "ratchet"
    } else {
        "advisory"
    };
    let baseline = baseline.unwrap_or("agent/baselines/main.repo-score.json");
    let baseline_arg = if mode == "ratchet" {
        " --baseline target/jankurai/accepted-baseline.json"
    } else {
        ""
    };
    format!(
        r#"name: jankurai

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

concurrency:
  group: jankurai-${{{{ github.workflow }}}}-${{{{ github.ref }}}}
  cancel-in-progress: true

jobs:
  audit:
    runs-on: ubuntu-latest
    timeout-minutes: 45
    permissions:
      contents: read
      security-events: write
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd
        with:
          fetch-depth: 0
      - name: Install Rust toolchain
        run: rustup toolchain install stable --profile minimal --component rustfmt,clippy
      - name: Prepare accepted baseline
        run: |
          set -euo pipefail
          mkdir -p target/jankurai
          if [ "${{{{ github.event_name }}}}" = "pull_request" ]; then
            git fetch origin main --depth=1
            git show origin/main:{baseline} > target/jankurai/accepted-baseline.json
          elif [ -f {baseline} ]; then
            cp {baseline} target/jankurai/accepted-baseline.json
          else
            echo "missing accepted baseline {baseline}" >&2
            exit 1
          fi
      - name: Install jankurai
        run: cargo install jankurai --locked
      - run: jankurai --version
      - name: Proofbind verify
        run: jankurai proofbind verify . --changed-from origin/main --mode required
      - name: Proofmark rust
        run: jankurai proofmark rust . --mode required --obligations target/jankurai/proofbind/obligations.json
      - name: Rust witness build
        run: jankurai rust witness build .
      - name: Security lane
        run: jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json
      - name: UX QA smoke
        run: jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json
      - name: jankurai audit
        run: jankurai audit . --mode {audit_mode}{baseline_arg} --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --sarif target/jankurai/jankurai.sarif --github-step-summary target/jankurai/summary.md --repair-queue-jsonl target/jankurai/repair-queue.jsonl
      - name: Upload SARIF
        if: always()
        uses: github/codeql-action/upload-sarif@53e96ec3b35fce51c141c0d6f0e31028a448722d
        with:
          sarif_file: target/jankurai/jankurai.sarif
      - uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
        if: always()
        with:
          name: jankurai-adoption-evidence
          if-no-files-found: ignore
          path: |
            target/jankurai/repo-score.json
            target/jankurai/repo-score.md
            target/jankurai/jankurai.sarif
            target/jankurai/repair-queue.jsonl
            target/jankurai/proofbind/obligations.json
            target/jankurai/proofbind/surface-witness.json
            target/jankurai/proofmark/proofmark-receipt.json
            target/jankurai/proofmark/proof-receipt.json
            target/jankurai/rust/witness-graph.json
            target/jankurai/ux-qa.json
            target/jankurai/merge-witness.json
            target/jankurai/merge-witness.md
            target/jankurai/score-diff.json
            target/jankurai/score-trend.json
            target/jankurai/security/evidence.json
            target/jankurai/migration-report.json
            agent/jankurai-badge.svg
            agent/jankurai-badge.json
"#
    )
}

#[cfg(test)]
mod tests {
    use super::workflow;

    #[test]
    fn ratchet_workflow_uses_installed_jankurai_and_score_gate() {
        let rendered = workflow("ratchet", 85, Some(".jankurai/repo-score.json"));
        assert!(rendered.contains("cargo install jankurai --locked"));
        assert!(rendered.contains("--mode ratchet"));
        assert!(rendered.contains("target/jankurai/accepted-baseline.json"));
        assert!(rendered.contains("jankurai security run . --strict --profile ci"));
        assert!(rendered.contains(
            "github/codeql-action/upload-sarif@53e96ec3b35fce51c141c0d6f0e31028a448722d"
        ));
        assert!(!rendered.contains("target/jankurai/baseline-score.json"));
    }

    #[test]
    fn observe_workflow_has_no_score_gate() {
        let rendered = workflow("observe", 85, None);
        assert!(rendered.contains("--mode advisory"));
        assert!(!rendered.contains("Enforce score floor"));
        assert!(!rendered.contains("-ge 85"));
        assert!(rendered.contains("timeout-minutes"));
        assert!(rendered.contains("concurrency:"));
    }

    #[test]
    fn ratchet_workflow_can_use_baseline() {
        let rendered = workflow("ratchet", 85, Some("target/jankurai/baseline.json"));
        assert!(rendered.contains("git show origin/main:target/jankurai/baseline.json"));
        assert!(rendered.contains("--baseline target/jankurai/accepted-baseline.json"));
        assert!(rendered.contains("target/jankurai/repo-score.json"));
    }
}
