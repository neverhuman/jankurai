use crate::audit::helpers::*;
use crate::model::DimensionResult;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let mut score = 20;
    let mut evidence = vec![];
    let mut notes = vec![];
    let surface_text = command_surface_text(ctx);
    if [
        "cargo check",
        "cargo nextest",
        "cargo build --timings",
        "sccache",
        "rust-cache",
        "bacon",
        "vitest",
        "pytest",
        "go test",
        "dotnet test",
        "pnpm",
        "bun test",
        "turbo",
        "nx",
        "tsc -p",
        "playwright test",
    ]
    .iter()
    .any(|n| surface_text.contains(n))
    {
        score += 20;
        evidence.push("build acceleration markers found".into());
    }
    if [
        "cargo check",
        "nextest",
        "vitest",
        "pytest",
        "go test",
        "dotnet test",
        "tsc -p",
        "playwright test",
    ]
    .iter()
    .any(|n| surface_text.contains(n))
    {
        score += 10;
        evidence.push("targeted test/build commands found".into());
    }
    if ctx.all_files.iter().any(|f| {
        [
            "Cargo.lock",
            "pnpm-lock.yaml",
            "package-lock.json",
            "yarn.lock",
            "uv.lock",
            "poetry.lock",
        ]
        .contains(&f.name.as_str())
    }) {
        score += 10;
        evidence.push("locked dependency graph present".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path.starts_with(".github/workflows") && f.text.contains("cache"))
    {
        score += 10;
        evidence.push("CI cache hint found".into());
    }
    if !has_one_command(ctx) {
        score -= 10;
        notes.push("missing one-command setup/validation".into());
    }
    if !has_fast_lane(ctx) {
        score -= 10;
        notes.push("missing deterministic fast lane".into());
    }
    if real_command_surface_contains(ctx, &["cargo check"])
        && real_command_surface_contains(ctx, &["npm --workspace @jankurai/ux-qa run build"])
        && real_command_surface_contains(ctx, &["npm --workspace @jankurai/ux-qa run test"])
    {
        score += 15;
        evidence.push("focused Rust and UX QA build/test lanes are available".into());
    }
    if has_fast_lane(ctx)
        && real_command_surface_contains(ctx, &["cargo check -p jankurai"])
        && surface_text.contains("target/jankurai/fast-score.json")
        && surface_text.contains("--changed-fast")
        && surface_text.contains("target/jankurai/audit-fast.json")
    {
        score += 15;
        evidence.push("fast lane uses targeted commands and target-only audit artifacts".into());
    }
    // J1g: HLT-018 raises the score above the cap when the Justfile (or other
    // command surface) shows both an explicit cache marker AND at least one
    // narrow target. The previous `+20`/`+10` bonuses match too liberally; this
    // bonus rewards repos that demonstrate evidence of both.
    let has_cache_marker = [
        "turbo",
        "nextest",
        "just-cache",
        "cargo --cached",
        "sccache",
    ]
    .iter()
    .any(|m| surface_text.contains(m));
    let has_narrow_target = [
        "cargo check -p",
        "cargo test -p",
        "cargo nextest run -p",
        "vitest run",
        "pytest -k",
        "go test -run",
    ]
    .iter()
    .any(|m| surface_text.contains(m));
    if has_cache_marker && has_narrow_target {
        score += 10;
        evidence.push("explicit cache marker plus narrow per-package target found".into());
    }
    make_dim("Build speed signals", score, evidence, notes)
}
