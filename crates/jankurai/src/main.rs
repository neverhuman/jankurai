use clap::{Args, Parser, Subcommand};
use jankurai::audit::policy::AuditMode;
use jankurai::audit::{run_audit, run_audit_timed_with_options, AuditOptions};
use jankurai::commands::copy_code::CopyCodeArgs;
use jankurai::commands::{
    adopt, agent, audit_file, badge, bench, cell, certify, conformance, context_pack, copy_code,
    coverage, doctor, exceptions, govern, history, hooks, init, kickoff, migrate, optimize, paper,
    postmortem, proof, proofbind, proofmark, publish, registry, repair, repair_plan, rules, rust,
    score, security, update, vibe, witness,
};
use jankurai::render::{render_markdown, write_json, write_markdown};
use jankurai::report::issues::IssueFormat;
use jankurai::validation::{self, ArtifactSchema};
use jankurai::versions::check_versions;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(
    name = "jankurai",
    version,
    args_conflicts_with_subcommands = true,
    subcommand_precedence_over_arg = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[command(flatten)]
    audit: AuditArgs,
}

// Clap subcommands keep argument structs inline so generated help and defaults stay direct.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum Commands {
    Audit(AuditArgs),
    AuditFile(audit_file::AuditFileArgs),
    CopyCode(CopyCodeArgs),
    Adopt(AdoptArgs),
    Badge(BadgeCliArgs),
    Init(InitArgs),
    Update(UpdateArgs),
    Upgrade(UpgradeArgs),
    Doctor(DoctorArgs),
    ContextPack(ContextPackArgs),
    Kickoff(KickoffArgs),
    Witness(WitnessArgs),
    Conformance {
        #[command(subcommand)]
        command: ConformanceCommand,
    },
    Score(ScoreArgs),
    History {
        #[command(subcommand)]
        command: HistoryCommand,
    },
    Rules {
        #[command(subcommand)]
        command: RulesCommand,
    },
    RepairPlan(RepairPlanArgs),
    Lane(ProofPlanArgs),
    Proof(ProofPlanArgs),
    Prove(ProveArgs),
    ProofVerify(ProofVerifyArgs),
    #[command(name = "proofbind")]
    ProofBind {
        #[command(subcommand)]
        command: ProofBindCommand,
    },
    #[command(name = "proofmark")]
    ProofMark {
        #[command(subcommand)]
        command: ProofMarkCommand,
    },
    Registry(RegistryArgs),
    Cell(CellArgs),
    Migrate(MigrateArgs),
    Bench(BenchArgs),
    Certify(CertifyArgs),
    Govern(GovernArgs),
    Publish(PublishArgs),
    Paper {
        #[command(subcommand)]
        command: PaperCommand,
    },
    Repair(RepairArgs),
    Optimize(OptimizeArgs),
    Version(VersionArgs),
    Rust {
        #[command(subcommand)]
        command: RustCommand,
    },
    Exceptions {
        #[command(subcommand)]
        command: ExceptionCommand,
    },
    Postmortem(PostmortemArgs),
    Adapters {
        #[command(subcommand)]
        command: AdapterCommand,
    },
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Ci {
        #[command(subcommand)]
        command: CiCommand,
    },
    Hooks {
        #[command(subcommand)]
        command: HooksCommand,
    },
    Issues {
        #[command(subcommand)]
        command: IssuesCommand,
    },
    Explain(ExplainArgs),
    Versions(VersionsArgs),
    Ux(UxArgs),
    Security {
        #[command(subcommand)]
        command: SecurityCommand,
    },
    Coverage {
        #[command(subcommand)]
        command: CoverageCommand,
    },
    Vibe {
        #[command(subcommand)]
        command: VibeCommand,
    },
    Guard {
        #[command(subcommand)]
        command: jankurai_guard::GuardCommand,
    },
}

#[derive(Args, Debug)]
struct BadgeCliArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH", default_value = badge::DEFAULT_SCORE_JSON)]
    score: String,
    #[arg(long, value_name = "PATH", default_value = badge::DEFAULT_BADGE_SVG)]
    out: String,
    #[arg(long, value_name = "PATH", default_value = badge::DEFAULT_BADGE_JSON)]
    json_out: String,
    /// Skip writing badge JSON metadata.
    #[arg(long)]
    no_json: bool,
    #[arg(long, value_name = "PATH", default_value = badge::DEFAULT_README)]
    readme: String,
    /// Skip updating the README file.
    #[arg(long)]
    no_readme: bool,
    #[arg(long, value_name = "PATH_OR_URL", default_value = badge::DEFAULT_SCORE_MD)]
    link: String,
    /// Write the badge block into the README.
    #[arg(long)]
    update_readme: bool,
    /// CI mode: verify badge is current without writing.
    #[arg(long)]
    check: bool,
    /// Print the Markdown snippet to stdout.
    #[arg(long)]
    print_markdown: bool,
    #[arg(long, default_value = "jankurai")]
    label: String,
}

#[derive(Subcommand, Debug)]
enum AdapterCommand {
    Verify(AdapterVerifyArgs),
    Sync(AdapterSyncArgs),
}

#[derive(Subcommand, Debug)]
enum CiCommand {
    Install(CiInstallArgs),
}

#[derive(Subcommand, Debug)]
enum HooksCommand {
    Install(HooksInstallArgs),
}

#[derive(Subcommand, Debug)]
enum IssuesCommand {
    Export(IssuesExportArgs),
}

#[derive(Subcommand, Debug)]
enum SecurityCommand {
    Run(SecurityRunArgs),
}

#[derive(Subcommand, Debug)]
enum CoverageCommand {
    /// Parses coverage/proof artifacts. Does not run tests or external coverage tools.
    Audit(CoverageAuditArgs),
}

#[derive(Subcommand, Debug)]
enum VibeCommand {
    Coverage(VibeCoverageArgs),
    Validate(VibeValidateArgs),
}

#[derive(Subcommand, Debug)]
enum ConformanceCommand {
    Run(ConformanceRunArgs),
}

#[derive(Subcommand, Debug)]
enum PaperCommand {
    PublicRepoScores(PublicRepoScoresArgs),
}

#[derive(Subcommand, Debug)]
enum ScoreCommand {
    Diff(ScoreDiffArgs),
    Trend(ScoreTrendArgs),
}

#[derive(Args, Debug)]
struct ScoreArgs {
    #[command(flatten)]
    audit: AuditArgs,
    #[command(subcommand)]
    command: Option<ScoreCommand>,
}

#[derive(Subcommand, Debug)]
enum HistoryCommand {
    Latest(HistoryLatestArgs),
    Export(HistoryExportArgs),
    Compact(HistoryCompactArgs),
    Restore(HistoryRestoreArgs),
}

#[derive(Subcommand, Debug)]
enum RulesCommand {
    Export(RulesExportArgs),
    Verify(RulesVerifyArgs),
}

#[derive(Subcommand, Debug)]
enum RustCommand {
    Map(RustMapArgs),
    Witness {
        #[command(subcommand)]
        command: RustWitnessCommand,
    },
    Diagnose(RustDiagnoseArgs),
}

#[derive(Subcommand, Debug)]
enum RustWitnessCommand {
    Build(RustWitnessBuildArgs),
    Diff(RustWitnessDiffArgs),
}

#[derive(Subcommand, Debug)]
enum ProofBindCommand {
    Map(ProofBindMapArgs),
    Verify(ProofBindVerifyArgs),
}

#[derive(Subcommand, Debug)]
enum ProofMarkCommand {
    Rust(ProofMarkRustArgs),
}

#[derive(Subcommand, Debug)]
enum ExceptionCommand {
    Expire(ExceptionExpireArgs),
}

#[derive(Subcommand, Debug)]
enum PostmortemCommand {
    Record(PostmortemRecordArgs),
    List(PostmortemListArgs),
    Show(PostmortemShowArgs),
    Read(PostmortemReadArgs),
}

#[derive(Subcommand, Debug)]
enum AgentCommand {
    Verify(AgentVerifyArgs),
}

#[derive(Args, Debug, Clone)]
struct AuditArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "agent/repo-score.json")]
    json: String,
    #[arg(long, value_name = "PATH", default_value = "agent/repo-score.md")]
    md: String,
    #[arg(long)]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(long)]
    changed_fast: bool,
    #[arg(long, default_value = "standard")]
    mode: String,
    #[arg(long, value_name = "PATH")]
    timings_json: Option<String>,
    #[arg(long, value_name = "PATH")]
    sarif: Option<String>,
    #[arg(long, value_name = "PATH")]
    junit: Option<String>,
    #[arg(long, value_name = "PATH")]
    github_step_summary: Option<String>,
    #[arg(long, value_name = "PATH")]
    repair_queue_jsonl: Option<String>,
    #[arg(long, value_name = "PATH")]
    proof_receipts: Option<String>,
    #[arg(long, value_name = "PATH")]
    proof_evidence: Option<String>,
    #[arg(long, value_name = "PATH")]
    baseline: Option<String>,
    #[arg(long, value_name = "PATH")]
    policy: Option<String>,
    #[arg(long)]
    self_audit: bool,
    #[arg(long)]
    fail_under: Option<i32>,
    #[arg(long, value_delimiter = ',')]
    fail_on: Vec<String>,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/score-history.jsonl"
    )]
    score_history: String,
    #[arg(long, value_name = "PATH")]
    score_history_csv: Option<String>,
    #[arg(long, value_name = "PATH")]
    score_history_mirror: Option<String>,
    #[arg(long)]
    score_history_mirror_required: bool,
    #[arg(long, default_value_t = 500)]
    score_history_max_rows: usize,
    #[arg(long, default_value_t = 1_048_576)]
    score_history_max_bytes: usize,
    #[arg(long)]
    no_score_history: bool,
    #[arg(long)]
    full: bool,
    #[arg(long, value_name = "SECS")]
    smart_interval: Option<u64>,
    #[arg(long, value_name = "FLOAT")]
    smart_rate: Option<f64>,
}

#[derive(Args, Debug)]
struct InitArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    apply: bool,
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, default_value = "rust-ts-vite-react-postgres-bounded-python")]
    profile: String,
    /// Init profile manifest JSON (`schemas/init-profile.schema.json`). When set, bundled `--profile` is not used to resolve the manifest.
    #[arg(long, value_name = "PATH")]
    profile_file: Option<PathBuf>,
    #[arg(long, default_value = "full", value_parser = ["agents", "score", "ci", "full"])]
    level: String,
    #[arg(long, default_value = "all")]
    ide: String,
    #[arg(long, default_value = "advisory")]
    mode: String,
    #[arg(long)]
    diff: bool,
    #[arg(long, default_value = "github")]
    ci: String,
    #[arg(long, default_value = "jsonl")]
    issue_backend: String,
    #[arg(long)]
    ux_qa: bool,
    #[arg(long, value_name = "PATH")]
    plan_json: Option<String>,
    #[arg(long)]
    force_generated_adapters: bool,
    /// Bootstrap adoption commit: apply the full scaffold, install observe CI, score, append tracked history, stage everything, and commit.
    #[arg(long)]
    bootstrap_commit: bool,
    #[arg(long, default_value = "Adopt Jankurai control plane")]
    bootstrap_message: String,
}

#[derive(Args, Debug)]
struct UpdateArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    check: bool,
    #[arg(long)]
    apply: bool,
    #[arg(long)]
    yes: bool,
    #[arg(long = "self", alias = "self-update")]
    self_update: bool,
    #[arg(long, hide = true)]
    skip_self: bool,
    #[arg(long)]
    client_start: bool,
    #[arg(long)]
    quiet: bool,
    #[arg(long, default_value = "stable", value_parser = ["stable", "beta", "draft", "lts"])]
    channel: String,
    #[arg(long, default_value = "auto", value_parser = ["auto", "crates-io", "git", "github", "local"])]
    source: String,
    #[arg(long)]
    offline: bool,
    #[arg(long)]
    fail_if_outdated: bool,
    #[arg(long)]
    install_missing: bool,
    #[arg(long, default_value = "rust-ts-postgres")]
    profile: String,
    #[arg(long, default_value = "full", value_parser = ["agents", "score", "ci", "full"])]
    level: String,
    #[arg(long, default_value = "all")]
    ide: String,
    #[arg(long)]
    score: bool,
    #[arg(long, default_value = "standard", value_parser = ["standard", "advisory"])]
    score_mode: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/repo-score.json"
    )]
    score_json: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/repo-score.md"
    )]
    score_md: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/update/update-plan.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/update/update-plan.md"
    )]
    md: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/update/state.json"
    )]
    state: String,
}

#[derive(Args, Debug)]
struct UpgradeArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    offline: bool,
    #[arg(long)]
    quiet: bool,
    #[arg(long, default_value = "stable", value_parser = ["stable", "beta", "draft", "lts"])]
    channel: String,
    #[arg(long, default_value = "auto", value_parser = ["auto", "crates-io", "git", "github", "local"])]
    source: String,
    #[arg(long, default_value = "rust-ts-postgres")]
    profile: String,
    #[arg(long, default_value = "full", value_parser = ["agents", "score", "ci", "full"])]
    level: String,
    #[arg(long, default_value = "all")]
    ide: String,
    #[arg(long)]
    score: bool,
    #[arg(long, default_value = "standard", value_parser = ["standard", "advisory"])]
    score_mode: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/repo-score.json"
    )]
    score_json: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/repo-score.md"
    )]
    score_md: String,
}

#[derive(Args, Debug)]
struct AdoptArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "auto", value_parser = [
        "auto",
        "migration-target",
        "rust-api",
        "react-web",
        "rust-ts-postgres",
        "b2b-saas",
        "ai-product",
        "regulated-saas"
    ])]
    profile: String,
    #[arg(long, default_value = "observe", value_parser = ["observe", "advisory", "ratchet"])]
    mode: String,
    #[arg(long, value_name = "PATH", default_value = adopt::DEFAULT_OUT)]
    out: String,
    #[arg(long, value_name = "PATH", default_value = adopt::DEFAULT_MD)]
    md: String,
}

#[derive(Args, Debug)]
struct DoctorArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "high")]
    fail_on: String,
    #[arg(long, value_name = "PATH")]
    json: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct ContextPackArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "merge")]
    task: String,
    #[arg(long, value_name = "PATH")]
    changed: Vec<PathBuf>,
    #[arg(long, default_value_t = 6000)]
    max_tokens: usize,
    #[arg(long, default_value = "generic", value_parser = ["codex", "claude", "cursor", "generic"])]
    agent: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct KickoffArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "TEXT")]
    intent: String,
    #[arg(long, value_name = "PATH")]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(long, default_value = "generic", value_parser = ["codex", "claude", "cursor", "generic"])]
    agent: String,
    #[arg(long, default_value_t = 6000)]
    max_tokens: usize,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/kickoff.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/kickoff.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct PublicRepoScoresArgs {
    #[arg(long, value_name = "PATH")]
    source: PathBuf,
    #[arg(long, value_name = "PATH")]
    out: PathBuf,
}

#[derive(Args, Debug)]
struct WitnessArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(long, value_name = "PATH")]
    baseline: Option<String>,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proof-receipts"
    )]
    proof_receipts: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/merge-witness.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/merge-witness.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct ConformanceRunArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    workspace: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "conformance/fixtures")]
    fixtures: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "conformance/expected")]
    expected: PathBuf,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/conformance-results.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/conformance-results.md"
    )]
    md: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "paper/tex/generated/conformance_results_table.tex"
    )]
    tex: String,
}

#[derive(Args, Debug)]
struct RepairPlanArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    from: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct ProofPlanArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct ProveArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    plan: Option<String>,
    #[arg(long)]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proof-plan.json"
    )]
    plan_out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proof-plan.md"
    )]
    plan_md: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proof-receipts"
    )]
    out_dir: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/evidence-index.json"
    )]
    evidence_index: String,
    #[arg(long)]
    continue_on_error: bool,
    /// Allow commands not listed in agent/proof-lanes.toml or agent/test-map.json.
    /// Requires `JANKURAI_ALLOW_UNSIGNED_PROOF_COMMANDS=1` in the environment.
    #[arg(long = "allow-unsigned-commands")]
    allow_unsigned_commands: bool,
}

#[derive(Args, Debug)]
struct ProofVerifyArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    plan: String,
    #[arg(long, value_name = "PATH")]
    evidence_index: String,
    #[arg(long, value_name = "PATH")]
    out: String,
    #[arg(long, value_name = "PATH")]
    md: String,
}

#[derive(Args, Debug)]
struct ProofBindMapArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(long, default_value = "advisory", value_parser = ["advisory", "required"])]
    mode: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proof-receipts"
    )]
    proof_receipts: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofbind/surface-witness.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofbind/obligations.json"
    )]
    obligations_out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofbind/proofbind.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct ProofBindVerifyArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(long, default_value = "advisory", value_parser = ["advisory", "required"])]
    mode: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proof-receipts"
    )]
    proof_receipts: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofbind/surface-witness.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofbind/obligations.json"
    )]
    obligations_out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofbind/proofbind.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct ProofMarkRustArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    changed: Vec<PathBuf>,
    #[arg(long, value_name = "REF")]
    changed_from: Option<String>,
    #[arg(long, default_value = "advisory", value_parser = ["advisory", "required"])]
    mode: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofbind/obligations.json"
    )]
    obligations: String,
    #[arg(long, value_name = "PATH")]
    coverage: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    mutation: Option<PathBuf>,
    #[arg(long)]
    negative_proof: Vec<String>,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofmark/proofmark-receipt.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofmark/proof-receipt.json"
    )]
    proof_receipt: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/proofmark/proofmark.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct RegistryArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct CellArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "workspace-cell")]
    cell_id: String,
    #[arg(long, default_value = "install-ready", value_parser = ["install-ready", "prove", "upgrade-plan", "deprecate-plan"])]
    mode: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct MigrateArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
    /// Run analyze mode (report only). Default is plan mode.
    #[arg(long)]
    analyze: bool,
    /// Target stack for migration (default: rust-ts-postgres)
    #[arg(long, default_value = "rust-ts-postgres")]
    target: String,
    #[command(subcommand)]
    command: Option<MigrateCommand>,
}

#[derive(Subcommand, Debug)]
enum MigrateCommand {
    VerifyPrompt(MigrateVerifyPromptArgs),
    SliceRisk(MigrateSliceRiskArgs),
}

#[derive(Args, Debug)]
struct MigrateVerifyPromptArgs {
    #[arg(value_name = "DOC")]
    document: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/migration-prompt-verification.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/migration-prompt-verification.md"
    )]
    md: String,
    #[arg(long)]
    strict: bool,
}

#[derive(Args, Debug)]
struct MigrateSliceRiskArgs {
    #[arg(long, value_name = "PATH")]
    plan: String,
    #[arg(long, value_name = "SLICE_ID")]
    slice_id: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/migration-slice-risk.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/migration-slice-risk.md"
    )]
    md: String,
    #[arg(long)]
    check_env: bool,
}

#[derive(Args, Debug)]
struct BenchArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct CertifyArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct GovernArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct PublishArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/p12-certification.json"
    )]
    certification: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/p12-benchmark-report.json"
    )]
    benchmark: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/p12-governance-policy.json"
    )]
    governance: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
    #[arg(long, value_name = "PATH")]
    badge_json: Option<String>,
    #[arg(long, value_name = "PATH")]
    badge_svg: Option<String>,
}

#[derive(Args, Debug)]
struct RepairArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH")]
    plan: String,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    fixture_apply: bool,
    /// Apply a bounded repair to a real git repository. Requires
    /// JANKURAI_ALLOW_REPAIR_APPLY=1 in the environment.
    #[arg(long)]
    apply: bool,
    #[arg(long)]
    auto_pr: bool,
    /// With --apply, commit repair changes and optionally push. Requires
    /// JANKURAI_ALLOW_GIT_MUTATION=1 in the environment.
    #[arg(long)]
    git_commit: bool,
    /// With --apply --git-commit --auto-pr, push and create a draft GitHub PR
    /// via gh. Requires JANKURAI_ALLOW_GITHUB_PR=1 in the environment.
    #[arg(long)]
    github_pr: bool,
    #[arg(long, default_value = "origin")]
    remote: String,
    #[arg(long, default_value = "main")]
    base: String,
    #[arg(long, value_name = "PATH")]
    pr_draft_out: Option<String>,
    #[arg(long, value_name = "PATH")]
    pr_draft_md: Option<String>,
    #[arg(long, default_value = "low")]
    max_risk: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct OptimizeArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "all", value_parser = ["all", "token", "performance", "dependency", "dead-code"])]
    mode: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct ExceptionExpireArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value_t = 7)]
    warning_days: i64,
    /// Exit with failure when the report status is blocked (expired or invalid exceptions). Expiring-soon remains status complete.
    #[arg(long)]
    strict: bool,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct PostmortemArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[command(subcommand)]
    command: Option<PostmortemCommand>,
}

#[derive(Args, Debug)]
struct PostmortemRecordArgs {
    #[arg(value_name = "PATH")]
    input: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct PostmortemListArgs {
    #[arg(long, value_name = "PATH", default_value = ".jankurai/postmortems")]
    root: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct PostmortemShowArgs {
    #[arg(long, default_value = ".jankurai/postmortems")]
    root: String,
    #[arg(long, value_name = "POSTMORTEM_ID")]
    postmortem_id: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct PostmortemReadArgs {
    #[arg(value_name = "PATH")]
    path: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct VersionsArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
}

#[derive(Args, Debug)]
struct VersionArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
}

#[derive(Args, Debug)]
struct IssuesExportArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "markdown")]
    format: String,
    #[arg(long, value_name = "PATH")]
    out: Option<String>,
}

#[derive(Args, Debug)]
struct CiInstallArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    github: bool,
    #[arg(long, default_value = "observe", value_parser = ["observe", "advisory", "ratchet"])]
    mode: String,
    #[arg(long, default_value_t = 85)]
    min_score: i32,
    #[arg(long, value_name = "PATH")]
    baseline: Option<String>,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Debug)]
struct ScoreDiffArgs {
    #[arg(long, value_name = "PATH")]
    base: PathBuf,
    #[arg(long, value_name = "PATH")]
    head: PathBuf,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/score-diff.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/score-diff.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct ScoreTrendArgs {
    #[arg(long, value_name = "PATH", default_value = "agent/score-history.jsonl")]
    history: PathBuf,
    #[arg(long, default_value_t = 30)]
    window: usize,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/score-trend.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/score-trend.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct HistoryLatestArgs {
    #[arg(long, value_name = "PATH")]
    history: PathBuf,
    #[arg(long, default_value = "auto", value_parser = ["auto", "local", "mirror"])]
    source: String,
    #[arg(long, value_name = "PATH", default_value = "-")]
    out: String,
}

#[derive(Args, Debug)]
struct HistoryExportArgs {
    #[arg(long, value_name = "PATH")]
    history: PathBuf,
    #[arg(long, default_value_t = 30)]
    window: usize,
    #[arg(long, default_value = "auto", value_parser = ["auto", "local", "mirror"])]
    source: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/history-export.json"
    )]
    out: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/history-export.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct HistoryCompactArgs {
    #[arg(long, value_name = "PATH")]
    history: PathBuf,
    #[arg(long, default_value_t = 500)]
    max_rows: usize,
    #[arg(long, default_value_t = 1_048_576)]
    max_bytes: usize,
    #[arg(long, default_value = "auto", value_parser = ["auto", "local", "mirror"])]
    source: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/history-compact.json"
    )]
    json: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/history-compact.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct HistoryRestoreArgs {
    #[arg(long, value_name = "PATH")]
    mirror: PathBuf,
    #[arg(long, default_value = "auto")]
    repo_id: String,
    #[arg(long, value_name = "PATH")]
    out: String,
    #[arg(long, default_value_t = 500)]
    max_rows: usize,
    #[arg(long, default_value_t = 1_048_576)]
    max_bytes: usize,
    #[arg(long, default_value = "auto", value_parser = ["auto", "local", "mirror"])]
    source: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/history-restore.json"
    )]
    json: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/history-restore.md"
    )]
    md: String,
}

#[derive(Args, Debug)]
struct RulesExportArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/rule-registry.json"
    )]
    out: String,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct RulesVerifyArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/rules-verify.json"
    )]
    out: String,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
}

#[derive(Args, Debug)]
struct HooksInstallArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    force: bool,
}

#[derive(Args, Debug)]
struct AdapterVerifyArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
}

#[derive(Args, Debug)]
struct AdapterSyncArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "all")]
    ide: String,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    yes: bool,
}

#[derive(Args, Debug)]
struct AgentVerifyArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
}

#[derive(Args, Debug)]
struct ExplainArgs {
    rule_id: String,
}

#[derive(Args, Debug)]
struct UxArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<OsString>,
}

#[derive(Args, Debug)]
struct SecurityRunArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "tools/security-lane.sh")]
    script: String,
    #[arg(
        long,
        value_name = "PATH",
        default_value = "target/jankurai/security/evidence.json"
    )]
    out: String,
    #[arg(long)]
    strict: bool,
    #[arg(long, default_value = "local", value_parser = ["local", "ci", "release"])]
    profile: String,
}

#[derive(Args, Debug)]
struct CoverageAuditArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH", default_value = coverage::DEFAULT_CONFIG_PATH)]
    config: String,
    #[arg(long, value_name = "PATH", default_value = coverage::DEFAULT_JSON_PATH)]
    json: String,
    #[arg(long, value_name = "PATH", default_value = coverage::DEFAULT_MD_PATH)]
    md: String,
    #[arg(long, value_name = "REV")]
    changed_from: Option<String>,
    #[arg(long)]
    strict: bool,
    #[arg(long, default_value_t = coverage::DEFAULT_MAX_ARTIFACT_BYTES)]
    max_artifact_bytes: u64,
    #[arg(long, default_value_t = coverage::DEFAULT_MAX_FINDINGS)]
    max_findings: usize,
}

#[derive(Args, Debug)]
struct VibeCoverageArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "agent/vibe-coverage.toml")]
    source: String,
    #[arg(long, value_name = "PATH", default_value = "tips/vibe_coding")]
    tips: String,
    #[arg(long, value_name = "PATH")]
    json: Option<String>,
    #[arg(long, value_name = "PATH")]
    md: Option<String>,
    #[arg(long, value_name = "PATH")]
    tex: Option<String>,
}

#[derive(Args, Debug)]
struct VibeValidateArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "agent/vibe-coverage.toml")]
    source: String,
    #[arg(long, value_name = "PATH", default_value = "tips/vibe_coding")]
    tips: String,
}

#[derive(Args, Debug)]
struct RustMapArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "target/jankurai/rust")]
    out_dir: String,
}

#[derive(Args, Debug)]
struct RustWitnessBuildArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "target/jankurai/rust/witness-graph.json")]
    out: String,
}

#[derive(Args, Debug)]
struct RustWitnessDiffArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, value_name = "FILE")]
    old: PathBuf,
    #[arg(long, value_name = "FILE")]
    new: PathBuf,
    #[arg(long, default_value = "target/jankurai/rust/witness-diff.json")]
    out: String,
    #[arg(long, default_value = "target/jankurai/rust/witness-diff.md")]
    md: String,
}

#[derive(Args, Debug)]
struct RustDiagnoseArgs {
    #[arg(default_value = ".", value_parser = parse_repo_arg)]
    repo: PathBuf,
    #[arg(long, default_value = "target/jankurai/rust/compile-packets.json")]
    out: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse_from(normalize_cli_args(std::env::args_os()));
    match cli.command {
        Some(Commands::Versions(args)) => {
            check_versions(&args.repo)?;
        }
        Some(Commands::Version(args)) => {
            jankurai::versions::print_version(&args.repo)?;
        }
        Some(Commands::Audit(args)) => {
            run_audit_and_write(args)?;
        }
        Some(Commands::AuditFile(args)) => {
            let code = audit_file::run(args)?;
            use std::io::Write;
            std::io::stdout().flush().ok();
            std::io::stderr().flush().ok();
            std::process::exit(code);
        }
        Some(Commands::CopyCode(args)) => {
            copy_code::run(args)?;
        }
        Some(Commands::Adopt(args)) => {
            adopt::run(adopt::AdoptArgs {
                repo: args.repo,
                profile: args.profile,
                mode: args.mode,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Badge(args)) => {
            badge::run(badge::BadgeArgs {
                repo: args.repo,
                score: args.score,
                out: args.out,
                json_out: if args.no_json {
                    None
                } else {
                    Some(args.json_out)
                },
                readme: if args.no_readme {
                    None
                } else {
                    Some(args.readme)
                },
                link: args.link,
                update_readme: args.update_readme,
                check: args.check,
                print_markdown: args.print_markdown,
                label: args.label,
            })?;
        }
        Some(Commands::Init(args)) => {
            if args.bootstrap_commit {
                run_init_bootstrap_commit(args)?;
                return Ok(());
            }
            init::run(init::InitArgs {
                repo: args.repo,
                apply: args.apply,
                dry_run: args.dry_run,
                yes: args.yes,
                profile: args.profile,
                profile_file: args.profile_file,
                level: args.level,
                ide: args.ide,
                mode: args.mode,
                diff: args.diff,
                ci: args.ci,
                issue_backend: args.issue_backend,
                ux_qa: args.ux_qa,
                plan_json: args.plan_json,
                force_generated_adapters: args.force_generated_adapters,
            })?;
        }
        Some(Commands::Update(args)) => {
            update::run(update::UpdateArgs {
                repo: args.repo,
                check: args.check,
                apply: args.apply,
                yes: args.yes,
                self_update: args.self_update,
                skip_self: args.skip_self,
                client_start: args.client_start,
                quiet: args.quiet,
                channel: args.channel,
                source: args.source,
                offline: args.offline,
                fail_if_outdated: args.fail_if_outdated,
                install_missing: args.install_missing,
                profile: args.profile,
                level: args.level,
                ide: args.ide,
                score: args.score,
                score_mode: args.score_mode,
                score_json: args.score_json,
                score_md: args.score_md,
                out: args.out,
                md: args.md,
                state: args.state,
            })?;
        }
        Some(Commands::Upgrade(args)) => {
            update::run(update::UpdateArgs {
                repo: args.repo,
                check: false,
                apply: true,
                yes: true,
                self_update: true,
                skip_self: false,
                client_start: false,
                quiet: args.quiet,
                channel: args.channel,
                source: args.source,
                offline: args.offline,
                fail_if_outdated: false,
                install_missing: false,
                profile: args.profile,
                level: args.level,
                ide: args.ide,
                score: args.score,
                score_mode: args.score_mode,
                score_json: args.score_json,
                score_md: args.score_md,
                out: "target/jankurai/update/update-plan.json".into(),
                md: "target/jankurai/update/update-plan.md".into(),
                state: "target/jankurai/update/state.json".into(),
            })?;
        }
        Some(Commands::Doctor(args)) => {
            doctor::run(doctor::DoctorArgs {
                repo: args.repo,
                fail_on: args.fail_on,
                json: args.json,
                md: args.md,
            })?;
        }
        Some(Commands::ContextPack(args)) => {
            context_pack::run(context_pack::ContextPackArgs {
                repo: args.repo,
                task: args.task,
                changed: args.changed,
                max_tokens: args.max_tokens,
                agent: args.agent,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Kickoff(args)) => {
            kickoff::run(kickoff::KickoffArgs {
                repo: args.repo,
                intent: args.intent,
                changed: args.changed,
                changed_from: args.changed_from,
                agent: args.agent,
                max_tokens: args.max_tokens,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Witness(args)) => {
            witness::run(witness::WitnessArgs {
                repo: args.repo,
                changed: args.changed,
                changed_from: args.changed_from,
                baseline: args.baseline,
                proof_receipts: Some(args.proof_receipts),
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Conformance { command }) => match command {
            ConformanceCommand::Run(args) => {
                conformance::run(conformance::ConformanceRunArgs {
                    workspace: args.workspace,
                    fixtures: args.fixtures,
                    expected: args.expected,
                    out: args.out,
                    md: args.md,
                    tex: args.tex,
                })?;
            }
        },
        Some(Commands::Score(args)) => match args.command {
            Some(ScoreCommand::Diff(args)) => {
                score::run_diff(score::DiffArgs {
                    base: args.base,
                    head: args.head,
                    out: args.out,
                    md: args.md,
                })?;
            }
            Some(ScoreCommand::Trend(args)) => {
                score::run_trend(score::TrendArgs {
                    history: args.history,
                    window: args.window,
                    out: args.out,
                    md: args.md,
                })?;
            }
            None => {
                run_audit_and_write(args.audit)?;
            }
        },
        Some(Commands::History { command }) => match command {
            HistoryCommand::Latest(args) => {
                history::run_latest(history::LatestArgs {
                    history: args.history,
                    source: args.source,
                    out: args.out,
                })?;
            }
            HistoryCommand::Export(args) => {
                history::run_export(history::ExportArgs {
                    history: args.history,
                    window: args.window,
                    source: args.source,
                    out: args.out,
                    md: args.md,
                })?;
            }
            HistoryCommand::Compact(args) => {
                history::run_compact(history::CompactArgs {
                    history: args.history,
                    max_rows: args.max_rows,
                    max_bytes: args.max_bytes,
                    source: args.source,
                    json: args.json,
                    md: args.md,
                })?;
            }
            HistoryCommand::Restore(args) => {
                history::run_restore(history::RestoreArgs {
                    mirror: args.mirror,
                    repo_id: args.repo_id,
                    out: args.out,
                    max_rows: args.max_rows,
                    max_bytes: args.max_bytes,
                    source: args.source,
                    json: args.json,
                    md: args.md,
                })?;
            }
        },
        Some(Commands::Rules { command }) => match command {
            RulesCommand::Export(args) => {
                rules::run_export(rules::ExportArgs {
                    repo: args.repo,
                    out: args.out,
                    md: args.md,
                })?;
            }
            RulesCommand::Verify(args) => {
                rules::run_verify(rules::VerifyArgs {
                    repo: args.repo,
                    out: args.out,
                    md: args.md,
                })?;
            }
        },
        Some(Commands::RepairPlan(args)) => {
            repair_plan::run(repair_plan::RepairPlanArgs {
                repo: args.repo,
                from: args.from,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Lane(args)) => {
            proof::run_lane(proof::ProofPlanArgs {
                repo: args.repo,
                changed: args.changed,
                changed_from: args.changed_from,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Proof(args)) => {
            proof::run_proof(proof::ProofPlanArgs {
                repo: args.repo,
                changed: args.changed,
                changed_from: args.changed_from,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Prove(args)) => {
            proof::run_prove(proof::ProveArgs {
                repo: args.repo,
                plan: args.plan,
                changed: args.changed,
                changed_from: args.changed_from,
                plan_out: args.plan_out,
                plan_md: args.plan_md,
                out_dir: args.out_dir,
                evidence_index: args.evidence_index,
                continue_on_error: args.continue_on_error,
                allow_unsigned_commands: args.allow_unsigned_commands,
            })?;
        }
        Some(Commands::ProofVerify(args)) => {
            proof::run_proof_verify(proof::ProofVerifyArgs {
                repo: args.repo,
                plan: args.plan,
                evidence_index: args.evidence_index,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::ProofBind { command }) => match command {
            ProofBindCommand::Map(args) => {
                proofbind::run_map(proofbind::ProofBindMapArgs {
                    repo: args.repo,
                    changed: args.changed,
                    changed_from: args.changed_from,
                    mode: args.mode,
                    proof_receipts: args.proof_receipts,
                    out: args.out,
                    obligations_out: args.obligations_out,
                    md: args.md,
                })?;
            }
            ProofBindCommand::Verify(args) => {
                proofbind::run_verify(proofbind::ProofBindVerifyArgs {
                    repo: args.repo,
                    changed: args.changed,
                    changed_from: args.changed_from,
                    mode: args.mode,
                    proof_receipts: args.proof_receipts,
                    out: args.out,
                    obligations_out: args.obligations_out,
                    md: args.md,
                })?;
            }
        },
        Some(Commands::ProofMark { command }) => match command {
            ProofMarkCommand::Rust(args) => {
                proofmark::run_rust(proofmark::ProofMarkRustArgs {
                    repo: args.repo,
                    changed: args.changed,
                    changed_from: args.changed_from,
                    mode: args.mode,
                    obligations: args.obligations,
                    coverage: args.coverage,
                    mutation: args.mutation,
                    negative_proof: args.negative_proof,
                    out: args.out,
                    proof_receipt: args.proof_receipt,
                    md: args.md,
                })?;
            }
        },
        Some(Commands::Registry(args)) => {
            registry::run(registry::RegistryArgs {
                repo: args.repo,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Cell(args)) => {
            cell::run(cell::CellArgs {
                repo: args.repo,
                cell_id: args.cell_id,
                mode: args.mode,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Migrate(args)) => match args.command {
            Some(MigrateCommand::VerifyPrompt(command)) => {
                migrate::run_prompt_verify(migrate::PromptVerifyArgs {
                    repo: args.repo,
                    document: command.document,
                    out: Some(command.out),
                    md: Some(command.md),
                    strict: command.strict,
                })?;
            }
            Some(MigrateCommand::SliceRisk(command)) => {
                migrate::run_slice_risk(migrate::SliceRiskArgs {
                    repo: args.repo,
                    plan: command.plan,
                    slice_id: command.slice_id,
                    out: Some(command.out),
                    md: Some(command.md),
                    check_env: command.check_env,
                })?;
            }
            None => {
                let mode = if args.analyze {
                    migrate::MigrateMode::Analyze
                } else {
                    migrate::MigrateMode::Plan
                };
                migrate::run(migrate::MigrateArgs {
                    repo: args.repo,
                    out: args.out,
                    md: args.md,
                    mode,
                    target: args.target,
                })?;
            }
        },
        Some(Commands::Bench(args)) => {
            bench::run(bench::BenchArgs {
                repo: args.repo,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Certify(args)) => {
            certify::run(certify::CertifyArgs {
                repo: args.repo,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Govern(args)) => {
            govern::run(govern::GovernArgs {
                repo: args.repo,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Publish(args)) => {
            publish::run(publish::PublishArgs {
                repo: args.repo,
                certification: args.certification,
                benchmark: args.benchmark,
                governance: args.governance,
                out: args.out,
                md: args.md,
                badge_json: args.badge_json,
                badge_svg: args.badge_svg,
            })?;
        }
        Some(Commands::Paper { command }) => match command {
            PaperCommand::PublicRepoScores(args) => {
                paper::run_public_repo_scores(paper::PublicRepoScoresArgs {
                    source: args.source,
                    out: args.out,
                })?;
            }
        },
        Some(Commands::Repair(args)) => {
            repair::run(repair::RepairArgs {
                repo: args.repo,
                plan: args.plan,
                dry_run: args.dry_run,
                fixture_apply: args.fixture_apply,
                apply: args.apply,
                auto_pr: args.auto_pr,
                git_commit: args.git_commit,
                github_pr: args.github_pr,
                remote: args.remote,
                base: args.base,
                pr_draft_out: args.pr_draft_out,
                pr_draft_md: args.pr_draft_md,
                max_risk: args.max_risk,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Optimize(args)) => {
            optimize::run(optimize::OptimizeArgs {
                repo: args.repo,
                mode: args.mode,
                out: args.out,
                md: args.md,
            })?;
        }
        Some(Commands::Rust { command }) => match command {
            RustCommand::Map(args) => {
                rust::run_map(rust::RustMapArgs {
                    repo: args.repo,
                    out_dir: args.out_dir,
                })?;
            }
            RustCommand::Witness { command } => match command {
                RustWitnessCommand::Build(args) => {
                    rust::run_witness_build(rust::RustWitnessBuildArgs {
                        repo: args.repo,
                        out: args.out,
                    })?;
                }
                RustWitnessCommand::Diff(args) => {
                    rust::run_witness_diff(rust::RustWitnessDiffArgs {
                        repo: args.repo,
                        old: args.old,
                        new: args.new,
                        out: args.out,
                        md: args.md,
                    })?;
                }
            },
            RustCommand::Diagnose(args) => {
                rust::run_diagnose(rust::RustDiagnoseArgs {
                    repo: args.repo,
                    out: args.out,
                })?;
            }
        },
        Some(Commands::Exceptions { command }) => match command {
            ExceptionCommand::Expire(args) => {
                exceptions::run_expire(exceptions::ExceptionExpireArgs {
                    repo: args.repo,
                    warning_days: args.warning_days,
                    strict: args.strict,
                    out: args.out,
                    md: args.md,
                })?;
            }
        },
        Some(Commands::Postmortem(args)) => {
            let repo = args.repo;
            match args.command {
                Some(PostmortemCommand::Record(command)) => {
                    postmortem::run_record(postmortem::PostmortemRecordArgs {
                        repo: repo.clone(),
                        input: command.input,
                        out: command.out,
                        md: command.md,
                    })?;
                }
                Some(PostmortemCommand::List(command)) => {
                    postmortem::run_list(postmortem::PostmortemListArgs {
                        repo: repo.clone(),
                        root: command.root,
                        out: command.out,
                        md: command.md,
                    })?;
                }
                Some(PostmortemCommand::Show(command)) => {
                    postmortem::run_show(postmortem::PostmortemShowArgs {
                        repo: repo.clone(),
                        root: command.root,
                        postmortem_id: command.postmortem_id,
                        out: command.out,
                        md: command.md,
                    })?;
                }
                Some(PostmortemCommand::Read(command)) => {
                    postmortem::run_read(postmortem::PostmortemReadArgs {
                        repo: repo.clone(),
                        path: command.path,
                        out: command.out,
                        md: command.md,
                    })?;
                }
                None => {}
            }
        }
        Some(Commands::Adapters { command }) => match command {
            AdapterCommand::Verify(args) => run_adapters_verify(args)?,
            AdapterCommand::Sync(args) => run_adapters_sync(args)?,
        },
        Some(Commands::Agent { command }) => match command {
            AgentCommand::Verify(args) => {
                agent::verify(agent::AgentVerifyArgs { repo: args.repo })?
            }
        },
        Some(Commands::Issues { command }) => match command {
            IssuesCommand::Export(args) => run_issues_export(args)?,
        },
        Some(Commands::Ci { command }) => match command {
            CiCommand::Install(args) => {
                jankurai::commands::ci::install(jankurai::commands::ci::CiInstallArgs {
                    repo: args.repo,
                    github: args.github,
                    mode: args.mode,
                    min_score: args.min_score,
                    baseline: args.baseline,
                    dry_run: args.dry_run,
                })?;
            }
        },
        Some(Commands::Hooks { command }) => match command {
            HooksCommand::Install(args) => hooks::install(hooks::HooksInstallArgs {
                repo: args.repo,
                yes: args.yes,
                dry_run: args.dry_run,
                force: args.force,
            })?,
        },
        Some(Commands::Explain(args)) => run_explain(&args.rule_id)?,
        Some(Commands::Ux(_args)) => run_ux_passthrough()?,
        Some(Commands::Security { command }) => match command {
            SecurityCommand::Run(args) => {
                security::run(security::SecurityRunArgs {
                    repo: args.repo,
                    script: args.script,
                    out: args.out,
                    strict: args.strict,
                    profile: args.profile,
                })?;
            }
        },
        Some(Commands::Coverage { command }) => match command {
            CoverageCommand::Audit(args) => {
                coverage::run_audit(coverage::CoverageAuditArgs {
                    repo: args.repo,
                    config: args.config,
                    json: args.json,
                    md: args.md,
                    changed_from: args.changed_from,
                    strict: args.strict,
                    max_artifact_bytes: args.max_artifact_bytes,
                    max_findings: args.max_findings,
                })?;
            }
        },
        Some(Commands::Vibe { command }) => match command {
            VibeCommand::Coverage(args) => {
                vibe::run_coverage(vibe::VibeCoverageArgs {
                    repo: args.repo,
                    source: args.source,
                    tips: args.tips,
                    json: args.json,
                    md: args.md,
                    tex: args.tex,
                })?;
            }
            VibeCommand::Validate(args) => {
                vibe::run_validate(vibe::VibeValidateArgs {
                    repo: args.repo,
                    source: args.source,
                    tips: args.tips,
                })?;
            }
        },
        Some(Commands::Guard { command }) => {
            jankurai_guard::run(command)?;
        }
        None => {
            run_audit_and_write(cli.audit)?;
        }
    }
    Ok(())
}

fn parse_repo_arg(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.exists()
        || value == "."
        || value == ".."
        || value.starts_with('/')
        || value.starts_with("./")
        || value.starts_with("../")
        || value.contains(std::path::MAIN_SEPARATOR)
    {
        Ok(path)
    } else {
        Err(format!(
            "`{value}` is not a known command or an existing/path-like repository path"
        ))
    }
}

fn normalize_cli_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args: Vec<OsString> = args.into_iter().collect();
    if args.len() < 2 {
        return args;
    }
    let alias = args[1].to_string_lossy();
    if alias != "--update" && alias != "-update" {
        return args;
    }
    let mut normalized = vec![args.remove(0), OsString::from("update")];
    if args.len() == 1 || args[1].to_string_lossy().starts_with('-') {
        normalized.push(OsString::from("."));
    }
    normalized.extend(args.into_iter().skip(1));
    normalized
}

fn run_init_bootstrap_commit(args: InitArgs) -> anyhow::Result<()> {
    if args.dry_run || args.diff {
        anyhow::bail!(
            "--bootstrap-commit commits changes; omit --dry-run/--diff or run normal init first"
        );
    }
    if !args.yes {
        eprintln!(
            "{}",
            jankurai::ui::epaint(
                jankurai::ui::Style::Warn,
                "--bootstrap-commit implies --yes, --level full, observe CI, local hooks, score history, score trailers, git add -A, and git commit"
            )
        );
    }

    ensure_git_repo(&args.repo)?;
    let repo = args.repo.clone();
    let adoption_json = repo.join("agent/adoption-plan.json");
    let adoption_md = repo.join("agent/adoption-plan.md");
    let score_json = repo.join("target/jankurai/hooks/pre-commit-score.json");
    let score_md = repo.join("target/jankurai/hooks/pre-commit-score.md");
    let history_jsonl = repo.join("target/jankurai/hooks/pre-commit-score-history.jsonl");
    let history_csv = repo.join("target/jankurai/hooks/pre-commit-score-history.csv");
    let doctor_json = repo.join("target/jankurai/doctor.json");
    let doctor_md = repo.join("target/jankurai/doctor.md");

    adopt::run(adopt::AdoptArgs {
        repo: repo.clone(),
        profile: args.profile.clone(),
        mode: "observe".into(),
        out: adoption_json.display().to_string(),
        md: adoption_md.display().to_string(),
    })?;

    init::run(init::InitArgs {
        repo: repo.clone(),
        apply: false,
        dry_run: false,
        yes: true,
        profile: args.profile,
        profile_file: args.profile_file,
        level: "full".into(),
        ide: args.ide,
        mode: args.mode,
        diff: false,
        ci: args.ci,
        issue_backend: args.issue_backend,
        ux_qa: args.ux_qa,
        plan_json: args.plan_json,
        force_generated_adapters: args.force_generated_adapters,
    })?;

    jankurai::commands::ci::install(jankurai::commands::ci::CiInstallArgs {
        repo: repo.clone(),
        github: true,
        mode: "observe".into(),
        min_score: 85,
        baseline: None,
        dry_run: false,
    })?;

    hooks::install(hooks::HooksInstallArgs {
        repo: repo.clone(),
        yes: true,
        dry_run: false,
        force: false,
    })?;

    run_audit_and_write(AuditArgs {
        repo: repo.clone(),
        json: score_json.display().to_string(),
        md: score_md.display().to_string(),
        changed: vec![],
        changed_from: None,
        mode: "advisory".into(),
        sarif: None,
        junit: None,
        github_step_summary: None,
        repair_queue_jsonl: None,
        proof_receipts: None,
        proof_evidence: None,
        baseline: None,
        policy: None,
        self_audit: false,
        fail_under: None,
        fail_on: vec![],
        score_history: history_jsonl.display().to_string(),
        score_history_csv: Some(history_csv.display().to_string()),
        score_history_mirror: None,
        score_history_mirror_required: false,
        score_history_max_rows: 500,
        score_history_max_bytes: 1_048_576,
        no_score_history: false,
        changed_fast: false,
        timings_json: None,
        full: true,
        smart_interval: None,
        smart_rate: None,
    })?;
    let score_trailers = score_trailers_from_report(&repo, &score_json)?;

    doctor::run(doctor::DoctorArgs {
        repo: repo.clone(),
        fail_on: "critical".into(),
        json: Some(doctor_json.display().to_string()),
        md: Some(doctor_md.display().to_string()),
    })?;

    run_git(&repo, &["add", "-A"])?;
    let staged = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(&repo)
        .status()?;
    if staged.success() {
        println!("--bootstrap-commit found no staged changes to commit");
        return Ok(());
    }
    let commit_message = &args.bootstrap_message;
    run_git_env(
        &repo,
        &["commit", "-m", commit_message, "-m", &score_trailers],
        &[("JANKURAI_SKIP_HOOKS", "1")],
    )?;
    if let Some(commit) = git_stdout(&repo, &["rev-parse", "--short", "HEAD"])? {
        println!(
            "{}",
            jankurai::ui::paint(
                jankurai::ui::Style::Good,
                format!("--bootstrap-commit committed {commit}"),
                jankurai::ui::stdout_color_enabled()
            )
        );
    }
    print_bootstrap_commit_next_steps(&repo);
    Ok(())
}

fn print_bootstrap_commit_next_steps(repo: &std::path::Path) {
    let color = jankurai::ui::stdout_color_enabled();
    println!(
        "{}",
        jankurai::ui::paint(
            jankurai::ui::Style::Heading,
            "Bootstrap commit complete. Next:",
            color
        )
    );
    println!("  1. Push the adoption commit when ready: `git push -u origin HEAD`.");
    println!(
        "  2. Start Codex, OpenCode, Claude, Cursor, or another agent from `{}`.",
        repo.display()
    );
    println!(
        "  3. Tell it: `{}`",
        jankurai::ui::paint(
            jankurai::ui::Style::Accent,
            "Read AGENTS.md, follow the jankurai standard, improve the score, and commit small steps. Local hooks now auto-score each commit.",
            color
        )
    );
    println!(
        "- score history: `{}` and `{}`",
        repo.join("target/jankurai/hooks/pre-commit-score-history.jsonl")
            .display(),
        repo.join("target/jankurai/hooks/pre-commit-score-history.csv")
            .display()
    );
}

fn ensure_git_repo(repo: &std::path::Path) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("--bootstrap-commit requires an existing git repository");
    }
    Ok(())
}

fn run_git(repo: &std::path::Path, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("git").args(args).current_dir(repo).status()?;
    if !status.success() {
        anyhow::bail!("git {} failed with status {}", args.join(" "), status);
    }
    Ok(())
}

fn run_git_env(repo: &std::path::Path, args: &[&str], envs: &[(&str, &str)]) -> anyhow::Result<()> {
    let mut command = Command::new("git");
    command.args(args).current_dir(repo);
    for (key, value) in envs {
        command.env(key, value);
    }
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("git {} failed with status {}", args.join(" "), status);
    }
    Ok(())
}

fn git_stdout(repo: &std::path::Path, args: &[&str]) -> anyhow::Result<Option<String>> {
    let output = Command::new("git").args(args).current_dir(repo).output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()).filter(|s| !s.is_empty()))
}

fn score_trailers_from_report(
    repo: &std::path::Path,
    score_json: &std::path::Path,
) -> anyhow::Result<String> {
    let text = std::fs::read_to_string(score_json)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let score = value.get("score").and_then(|v| v.as_i64()).unwrap_or(0);
    let raw_score = value
        .get("raw_score")
        .and_then(|v| v.as_i64())
        .unwrap_or(score);
    let finding_count = value
        .get("findings")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    let decision = value.get("decision");
    let hard_findings = decision
        .and_then(|v| v.get("hard_findings"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let minimum_score = decision
        .and_then(|v| v.get("minimum_score"))
        .and_then(|v| v.as_i64())
        .unwrap_or(85);
    let status = if hard_findings > 0 || score < minimum_score {
        "fail"
    } else {
        "pass"
    };
    let report = score_json
        .strip_prefix(repo)
        .unwrap_or(score_json)
        .to_string_lossy()
        .replace('\\', "/");
    Ok(format!(
        "Jankurai-Score: {score}\nJankurai-Raw-Score: {raw_score}\nJankurai-Findings: {finding_count}\nJankurai-Hard-Findings: {hard_findings}\nJankurai-Decision: {status}\nJankurai-Report: {report}"
    ))
}

fn run_audit_and_write(args: AuditArgs) -> anyhow::Result<()> {
    let command_started = std::time::Instant::now();
    if args.json == "-" && args.md == "-" {
        anyhow::bail!("use at most one stdout target; JSON and Markdown may not share stdout");
    }
    let progress = jankurai::ui::CliProgress::new("scoring repository", 8);
    progress.tick("resolve changed paths");
    let (changed, changed_fast_effective, save_smart_state) =
        if let Some(base) = args.changed_from.as_deref() {
            (jankurai::audit::changed_paths_from_git(&args.repo, base)?, args.changed_fast, false)
        } else if !args.changed.is_empty() || args.changed_fast {
            if args.changed_fast && args.changed.is_empty() {
                anyhow::bail!("--changed-fast requires --changed PATH or --changed-from REF");
            }
            (args.changed.clone(), args.changed_fast, false)
        } else {
            use jankurai::audit::smart_scan::{decide, SmartScanConfig, SmartScanDecision};
            let config = SmartScanConfig {
                enabled: !args.full,
                interval_secs: args.smart_interval.unwrap_or(3600),
                roulette_rate: args.smart_rate.unwrap_or(0.10),
            };
            match decide(&args.repo, &config)? {
                SmartScanDecision::Full { reason } => {
                    eprintln!("[smart] full scan — {reason}");
                    (vec![], false, true)
                }
                SmartScanDecision::Fast { paths } => {
                    eprintln!("[smart] fast scan — {} changed files", paths.len());
                    (paths, true, false)
                }
                SmartScanDecision::Skip => {
                    eprintln!("[smart] no changes — last clean full scan still valid");
                    return Ok(());
                }
            }
        };
    progress.tick("load audit mode");
    let mode = AuditMode::parse(&args.mode)?;
    if matches!(mode, AuditMode::Ratchet) && args.baseline.is_none() {
        anyhow::bail!(
            "ratchet mode requires --baseline PATH; first run advisory mode and commit an accepted baseline"
        );
    }
    progress.tick("scan repository");
    let (mut report, mut timings) = run_audit_timed_with_options(
        &args.repo,
        &changed,
        AuditOptions {
            self_audit: args.self_audit,
            proof_receipts: args.proof_receipts.clone(),
            changed_fast: changed_fast_effective,
        },
    )?;
    if changed_fast_effective {
        if let Some(git) = report.git.as_mut() {
            git.mode = "changed-fast".into();
        }
    }
    progress.tick("apply score policy");
    if let Some(minimum_score) = args.fail_under {
        if let Some(policy) = report.policy.as_mut() {
            policy.minimum_score = minimum_score;
        }
    }
    if !args.fail_on.is_empty() {
        if let Some(policy) = report.policy.as_mut() {
            policy.fail_on = args.fail_on.clone();
        }
    }
    recompute_report_decision(&mut report);
    progress.tick("apply mode and baseline");
    apply_mode_and_baseline(&mut report, mode, args.baseline.as_deref())?;
    if matches!(mode, AuditMode::Release) {
        let proof_findings = jankurai::audit::release_proof_findings(
            &args.repo,
            args.proof_receipts.as_deref(),
            args.proof_evidence.as_deref(),
        )?;
        if !proof_findings.is_empty() {
            report.findings.extend(proof_findings);
            jankurai::audit::rebuild_agent_fix_queue(&mut report);
            recompute_report_decision(&mut report);
            apply_mode_and_baseline(&mut report, mode, args.baseline.as_deref())?;
        }
    }
    progress.tick("render artifacts");
    report.report_fingerprint = jankurai::audit::report_fingerprint(&report);
    let md_text = render_markdown(&report);
    progress.tick("write JSON and Markdown");
    let report_write_started = std::time::Instant::now();
    validation::write_json(&args.repo, ArtifactSchema::RepoScore, &args.json, &report)?;
    write_markdown(&args.md, &md_text)?;
    if let Some(path) = args.sarif.as_deref() {
        write_json(path, &jankurai::report::sarif::render_sarif(&report))?;
    }
    if let Some(path) = args.junit.as_deref() {
        write_json(path, &jankurai::report::junit::render_junit(&report))?;
    }
    if let Some(path) = args.github_step_summary.as_deref() {
        write_markdown(
            path,
            &jankurai::report::github::render_step_summary(&report),
        )?;
    }
    if let Some(path) = args.repair_queue_jsonl.as_deref() {
        write_json(path, &jankurai::report::issues::repair_queue_jsonl(&report))?;
    }
    timings.record_duration("report_write", report_write_started.elapsed());
    let write_history = !args.no_score_history && !changed_fast_effective;
    if write_history {
        let history_started = std::time::Instant::now();
        let policy = jankurai::score_history::ScoreHistoryPolicy::from_repo(&args.repo)
            .with_overrides(
                Some(args.score_history_max_rows),
                Some(args.score_history_max_bytes),
                None,
            );
        let mirror_path = args
            .score_history_mirror
            .clone()
            .or_else(|| jankurai::score_history::history_mirror_path_from_env(&policy));
        let history_path = jankurai::score_history::append_score_history_with_options(
            &args.repo,
            &report,
            &args.json,
            &args.md,
            jankurai::score_history::ScoreHistoryAppendOptions {
                history_path: args.score_history.clone(),
                csv_path: args.score_history_csv.clone(),
                mirror_path,
                mirror_required: args.score_history_mirror_required,
                policy,
            },
        )?;
        timings.record_duration("history_write", history_started.elapsed());
        if let Some(history_path) = history_path {
            eprintln!("score history appended {}", history_path.display());
        }
    } else {
        timings.record_ms("history_write", 0);
    }
    if let Some(path) = args.timings_json.as_deref() {
        timings.total_ms = command_started.elapsed().as_millis();
        write_json(path, &serde_json::to_string_pretty(&timings)?)?;
    }
    if let Some(notice) = update::audit_upgrade_notice(&args.repo) {
        eprintln!(
            "{}",
            jankurai::ui::epaint(
                jankurai::ui::Style::Warn,
                format!(
                    "upgrade available: jankurai {} -> {}; run {}",
                    notice.current_version, notice.latest_version, notice.manual_command
                )
            )
        );
    }
    progress.finish(format!(
        "score {} raw {} findings {}",
        report.score,
        report.raw_score,
        report.findings.len()
    ));
    eprintln!(
        "{}",
        jankurai::ui::epaint(
            jankurai::ui::Style::Good,
            format!(
                "score={} raw={} caps={} findings={}",
                report.score,
                report.raw_score,
                report.caps_applied.len(),
                report.findings.len()
            )
        )
    );
    // Auto-update badge if agent/badge.toml is present and this is a full audit.
    if !changed_fast_effective {
        if let Err(e) = badge::run_from_config_after_audit(&args.repo, &args.json, &args.md) {
            eprintln!(
                "{}",
                jankurai::ui::epaint(
                    jankurai::ui::Style::Warn,
                    format!("badge update skipped: {e}")
                )
            );
        }
    }
    if save_smart_state {
        let _ = jankurai::audit::smart_scan::save_state(&args.repo, &report);
    }
    enforce_audit_decision(&report, mode)?;
    Ok(())
}

fn run_adapters_verify(args: AdapterVerifyArgs) -> anyhow::Result<()> {
    let failures = jankurai::init::adapters::verify_adapters(&args.repo)?;
    if failures.is_empty() {
        println!(
            "{}",
            jankurai::ui::paint(
                jankurai::ui::Style::Good,
                "adapters verified",
                jankurai::ui::stdout_color_enabled()
            )
        );
        return Ok(());
    }
    for failure in failures {
        eprintln!("{}: {}", failure.path, failure.problem);
    }
    anyhow::bail!("adapter verification failed")
}

fn run_adapters_sync(args: AdapterSyncArgs) -> anyhow::Result<()> {
    if !args.dry_run && !args.yes {
        anyhow::bail!("refusing to write adapters without --dry-run or --yes");
    }
    let plan = jankurai::init::adapters::adapter_plan(&args.repo, &args.ide);
    for action in &plan {
        println!("{} {}", action.action, action.path);
    }
    if args.yes {
        jankurai::init::adapters::write_adapters(&args.repo, &args.ide, false)?;
    }
    Ok(())
}

fn run_explain(rule_id: &str) -> anyhow::Result<()> {
    let Some(doc) = jankurai::audit::docs_for_rule_id(rule_id) else {
        anyhow::bail!("unknown rule id `{rule_id}`");
    };
    println!("{rule_id}: {doc}");
    Ok(())
}

fn run_issues_export(args: IssuesExportArgs) -> anyhow::Result<()> {
    let format = IssueFormat::parse(&args.format)?;
    let report = run_audit(&args.repo, &[])?;
    let text = jankurai::report::issues::render_issues(&report, format);
    if let Some(out) = args.out.as_deref() {
        write_markdown(out, &text)?;
    } else {
        print!("{text}");
    }
    Ok(())
}

fn apply_mode_and_baseline(
    report: &mut jankurai::model::Report,
    mode: AuditMode,
    baseline: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(policy) = report.policy.as_mut() {
        policy.mode = Some(mode.as_str().into());
    }
    let ratchet = baseline
        .map(|path| {
            jankurai::audit::baseline::compare_report_to_baseline(
                report,
                &std::path::PathBuf::from(path),
            )
        })
        .transpose()?;
    if let Some(decision) = report.decision.as_mut() {
        if let Some(ratchet) = ratchet {
            let ratchet_passed = ratchet.passed;
            decision.ratchet = Some(ratchet);
            if matches!(mode, AuditMode::Ratchet | AuditMode::Release) && !ratchet_passed {
                decision.status = "fail".into();
                decision.passed = false;
            }
        }
        if mode == AuditMode::Advisory {
            decision.status = "advisory".into();
            decision.passed = true;
        }
    }
    Ok(())
}

fn recompute_report_decision(report: &mut jankurai::model::Report) {
    if let Some(policy) = report.policy.as_ref() {
        report.decision = Some(jankurai::audit::report_decision(
            report.score,
            &report.findings,
            policy,
        ));
    }
}

fn enforce_audit_decision(report: &jankurai::model::Report, mode: AuditMode) -> anyhow::Result<()> {
    if matches!(mode, AuditMode::Advisory) {
        return Ok(());
    }
    let Some(decision) = report.decision.as_ref() else {
        anyhow::bail!("non-advisory audit produced no decision");
    };
    if !decision.passed {
        anyhow::bail!(
            "audit decision failed in {} mode: status={} score={} minimum_score={} hard_findings={}",
            mode.as_str(),
            decision.status,
            report.score,
            decision.minimum_score,
            decision.hard_findings
        );
    }
    Ok(())
}

fn run_ux_passthrough() -> anyhow::Result<()> {
    let status = Command::new("node")
        .arg("packages/ux-qa/dist/cli.js")
        .args(std::env::args_os().skip(2))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("jankurai ux exited with {}", status)
    }
}
