use super::catalog::{
    ConfidencePolicy, Language, LanguageFinding, LanguageRule, Matcher, ProofWindow,
};
use super::common::{
    finding, is_docs_reference_tips_or_generated, is_test_fixture_or_example,
    sort_and_cap_findings, strip_comments_for_line_language,
};
use crate::audit::helpers::AuditContext;
use crate::model::FileInfo;
use once_cell::sync::Lazy;

const HLT_RULE_ID: &str = "HLT-032-DOCKER-BAD-BEHAVIOR";

const HARD_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "docker.compose.socket-mount",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["/var/run/docker.sock"]),
        proof_window: ProofWindow::None,
        problem: "Docker socket mount gives the job host-level control",
        fix: "remove the socket mount or isolate the job behind a dedicated daemon boundary",
    },
    LanguageRule {
        id: "docker.compose.privileged",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["privileged: true", "--privileged"]),
        proof_window: ProofWindow::None,
        problem: "privileged container or compose service bypasses confinement",
        fix: "remove privileged mode and keep the container within a least-privilege profile",
    },
    LanguageRule {
        id: "docker.compose.host-namespace",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "network_mode: host",
            "pid: host",
            "ipc: host",
            "userns_mode: host",
        ]),
        proof_window: ProofWindow::None,
        problem: "host namespace sharing removes container isolation",
        fix: "remove host namespace sharing or justify it with a dedicated local-only boundary",
    },
    LanguageRule {
        id: "docker.compose.dangerous-capability",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["cap_add: all", "sys_admin", "net_admin", "sys_ptrace"]),
        proof_window: ProofWindow::None,
        problem: "dangerous Linux capability weakens container confinement",
        fix: "remove the capability or scope it to a reviewed local-only exception",
    },
    LanguageRule {
        id: "docker.confinement.disabled",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "seccomp=unconfined",
            "apparmor=unconfined",
            "security.insecure",
        ]),
        proof_window: ProofWindow::None,
        problem: "container confinement is explicitly disabled",
        fix: "remove the unconfined profile and keep the default sandbox in place",
    },
    LanguageRule {
        id: "docker.secret.in-layer",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "arg ",
            "env ",
            "copy .env",
            "copy id_rsa",
            "copy .npmrc",
            "copy .pypirc",
            "copy .aws",
            "copy .kube",
            "copy .docker",
        ]),
        proof_window: ProofWindow::None,
        problem: "secret-like material is copied or baked into an image layer",
        fix: "mount the secret at runtime or replace it with a non-secret build input",
    },
    LanguageRule {
        id: "docker.install.unverified-remote",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "curl",
            "wget",
            "| sh",
            "| bash",
            "--no-check-certificate",
            "curl -k",
            "--allow-unauthenticated",
        ]),
        proof_window: ProofWindow::None,
        problem: "remote install step is not pinned or verified",
        fix: "pin the download, verify a checksum or signature, and avoid shell piping",
    },
    LanguageRule {
        id: "docker.image.mutable-tag",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[":latest"]),
        proof_window: ProofWindow::None,
        problem: "mutable image tag can move without review",
        fix: "pin the image to an immutable digest or version tag",
    },
    LanguageRule {
        id: "docker.port.public-db-admin",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["5432", "3306", "6379", "9200", "27017", "11211", "15672"]),
        proof_window: ProofWindow::None,
        problem: "database or admin port is published too broadly",
        fix: "bind the port to localhost or keep it on an internal-only network",
    },
];

const ADVISORY_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "docker.review.missing-dockerignore",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "security",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::NoActiveDetectors,
        proof_window: ProofWindow::None,
        problem: "repository has Docker build surface without a visible .dockerignore",
        fix: "add a .dockerignore that keeps secrets and build noise out of the context",
    },
    LanguageRule {
        id: "docker.review.root-user",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "security",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::NoActiveDetectors,
        proof_window: ProofWindow::None,
        problem: "Docker image appears to run as root in the final stage",
        fix: "switch to a non-root user in the final image stage",
    },
    LanguageRule {
        id: "docker.review.copy-dot-dot",
        language: Language::Docker,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "security",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::NoActiveDetectors,
        proof_window: ProofWindow::None,
        problem: "Docker build uses a broad COPY . . pattern",
        fix: "copy only the files needed for the stage or add a tighter .dockerignore",
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct DockerSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn catalog() -> &'static [LanguageRule] {
    static RULES: Lazy<Vec<LanguageRule>> = Lazy::new(|| {
        let mut rules = Vec::new();
        rules.extend_from_slice(HARD_RULES);
        rules.extend_from_slice(ADVISORY_RULES);
        rules
    });
    RULES.as_slice()
}

pub fn summary(ctx: &AuditContext) -> DockerSummary {
    DockerSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_hits(ctx).len(),
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    sort_and_cap_findings(hard_hits(ctx), 50)
}

pub fn advisory_signals(ctx: &AuditContext) -> Vec<LanguageFinding> {
    sort_and_cap_findings(advisory_hits(ctx), 50)
}

fn hard_hits(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in docker_files(ctx) {
        out.extend(hard_hits_for_file(&file));
    }
    out
}

fn advisory_hits(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in docker_files(ctx) {
        out.extend(advisory_hits_for_file(ctx, &file));
    }
    out
}

fn docker_files(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|file| is_docker_surface(file))
        .cloned()
        .collect()
}

fn is_docker_surface(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    if is_docs_reference_tips_or_generated(&file.rel_path)
        || is_test_fixture_or_example(&file.rel_path)
    {
        return false;
    }
    lower == ".dockerignore"
        || lower.starts_with("dockerfile")
        || lower.ends_with(".dockerfile")
        || lower.ends_with("docker-compose.yml")
        || lower.ends_with("docker-compose.yaml")
        || lower.ends_with("compose.yml")
        || lower.ends_with("compose.yaml")
        || lower.starts_with(".github/workflows/")
            && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
}

fn hard_hits_for_file(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let kind = docker_line_kind(file);
    let lower_text = file.text.to_ascii_lowercase();
    let has_cap_add = lower_text.contains("cap_add:");

    for (idx, raw_line) in file.text.lines().enumerate() {
        let line = strip_comments_for_line_language(raw_line, kind);
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();

        if lower.contains("/var/run/docker.sock") {
            out.push(finding(
                HLT_RULE_ID,
                "docker.compose.socket-mount",
                file,
                idx + 1,
                "Docker socket mount gives the job host-level control",
                "the container can reach the host Docker daemon",
                "remove the socket mount or isolate the job behind a dedicated daemon boundary",
                ProofWindow::None,
            ));
        }

        if lower.contains("privileged: true") || lower.contains("--privileged") {
            out.push(finding(
                HLT_RULE_ID,
                "docker.compose.privileged",
                file,
                idx + 1,
                "privileged container or compose service bypasses confinement",
                "the runtime explicitly turns on privileged mode",
                "remove privileged mode and keep the container within a least-privilege profile",
                ProofWindow::None,
            ));
        }

        if lower.contains("network_mode: host")
            || lower.contains("pid: host")
            || lower.contains("ipc: host")
            || lower.contains("userns_mode: host")
        {
            out.push(finding(
                HLT_RULE_ID,
                "docker.compose.host-namespace",
                file,
                idx + 1,
                "host namespace sharing removes container isolation",
                "the container shares a host namespace",
                "remove host namespace sharing or justify it with a dedicated local-only boundary",
                ProofWindow::None,
            ));
        }

        if has_cap_add
            && (lower.contains("all")
                || lower.contains("sys_admin")
                || lower.contains("net_admin")
                || lower.contains("sys_ptrace"))
        {
            out.push(finding(
                HLT_RULE_ID,
                "docker.compose.dangerous-capability",
                file,
                idx + 1,
                "dangerous Linux capability weakens container confinement",
                "the container requests a high-risk capability",
                "remove the capability or scope it to a reviewed local-only exception",
                ProofWindow::None,
            ));
        }

        if lower.contains("seccomp=unconfined")
            || lower.contains("apparmor=unconfined")
            || lower.contains("security.insecure")
        {
            out.push(finding(
                HLT_RULE_ID,
                "docker.confinement.disabled",
                file,
                idx + 1,
                "container confinement is explicitly disabled",
                "the runtime disables a confinement profile",
                "remove the unconfined profile and keep the default sandbox in place",
                ProofWindow::None,
            ));
        }

        if is_secret_layer_hit(&lower) {
            out.push(finding(
                HLT_RULE_ID,
                "docker.secret.in-layer",
                file,
                idx + 1,
                "secret-like material is copied or baked into an image layer",
                "a secret-looking path or key lands in the build layer",
                "mount the secret at runtime or replace it with a non-secret build input",
                ProofWindow::None,
            ));
        }

        if is_unverified_remote_install(&lower) {
            out.push(finding(
                HLT_RULE_ID,
                "docker.install.unverified-remote",
                file,
                idx + 1,
                "remote install step is not pinned or verified",
                "the build downloads remote code without a checksum or signature proof",
                "pin the download, verify a checksum or signature, and avoid shell piping",
                ProofWindow::None,
            ));
        }

        if (lower.starts_with("from ") || lower.starts_with("image:") || lower.contains(" image:"))
            && lower.contains(":latest")
        {
            out.push(finding(
                HLT_RULE_ID,
                "docker.image.mutable-tag",
                file,
                idx + 1,
                "mutable image tag can move without review",
                "the build references a mutable image tag",
                "pin the image to an immutable digest or version tag",
                ProofWindow::None,
            ));
        }

        if is_public_db_port(&lower, &lower_text) {
            out.push(finding(
                HLT_RULE_ID,
                "docker.port.public-db-admin",
                file,
                idx + 1,
                "database or admin port is published too broadly",
                "the port mapping is publicly reachable without a local bind",
                "bind the port to localhost or keep it on an internal-only network",
                ProofWindow::None,
            ));
        }
    }

    out
}

fn advisory_hits_for_file(ctx: &AuditContext, file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let kind = docker_line_kind(file);
    let text_lower = file.text.to_ascii_lowercase();

    if (file.rel_path.to_ascii_lowercase().starts_with("dockerfile")
        || file.rel_path.to_ascii_lowercase().ends_with(".dockerfile"))
        && !text_lower.contains("healthcheck")
    {
        out.push(finding(
            HLT_RULE_ID,
            "docker.review.missing-healthcheck",
            file,
            1,
            "Docker image has no visible healthcheck",
            "the final image does not advertise a healthcheck",
            "add a small healthcheck or document why the image stays healthcheck-free",
            ProofWindow::None,
        ));
    }

    if (file.rel_path.to_ascii_lowercase().starts_with("dockerfile")
        || file.rel_path.to_ascii_lowercase().ends_with(".dockerfile")
        || file
            .rel_path
            .to_ascii_lowercase()
            .contains("docker-compose")
        || file.rel_path.to_ascii_lowercase().contains("compose"))
        && !ctx_has_dockerignore(ctx)
    {
        out.push(finding(
            HLT_RULE_ID,
            "docker.review.missing-dockerignore",
            file,
            1,
            "repository has Docker build surface without a visible .dockerignore",
            "the build context is broader than it needs to be",
            "add a .dockerignore that keeps secrets and build noise out of the context",
            ProofWindow::None,
        ));
    }

    for (idx, raw_line) in file.text.lines().enumerate() {
        let line = strip_comments_for_line_language(raw_line, kind);
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("user root") {
            out.push(finding(
                HLT_RULE_ID,
                "docker.review.root-user",
                file,
                idx + 1,
                "Docker image appears to run as root in the final stage",
                "the final stage keeps the root user",
                "switch to a non-root user in the final image stage",
                ProofWindow::None,
            ));
        }
        if lower.contains("copy . .") || lower.contains("add . .") {
            out.push(finding(
                HLT_RULE_ID,
                "docker.review.copy-dot-dot",
                file,
                idx + 1,
                "Docker build uses a broad COPY . . pattern",
                "the build context is copied wholesale",
                "copy only the files needed for the stage or add a tighter .dockerignore",
                ProofWindow::None,
            ));
        }
    }

    out
}

fn ctx_has_dockerignore(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        file.rel_path
            .to_ascii_lowercase()
            .ends_with(".dockerignore")
    })
}

fn is_secret_layer_hit(lower: &str) -> bool {
    (lower.starts_with("arg ")
        || lower.starts_with("env ")
        || lower.starts_with("copy ")
        || lower.starts_with("add ")
        || lower.contains(" copy ")
        || lower.contains(" add "))
        && (lower.contains(".env")
            || lower.contains(".ssh")
            || lower.contains(".aws")
            || lower.contains(".docker")
            || lower.contains(".npmrc")
            || lower.contains(".pypirc")
            || lower.contains(".kube")
            || lower.contains("private_key")
            || lower.contains("password")
            || lower.contains("secret")
            || lower.contains("token"))
}

fn is_unverified_remote_install(lower: &str) -> bool {
    (lower.contains("curl") || lower.contains("wget"))
        && (lower.contains("| sh")
            || lower.contains("| bash")
            || lower.contains("curl -k")
            || lower.contains("--no-check-certificate")
            || lower.contains("--allow-unauthenticated"))
}

fn is_public_db_port(lower: &str, full_text: &str) -> bool {
    let db_ports = [
        ":5432", ":3306", ":6379", ":9200", ":27017", ":11211", ":15672", ":8080", ":8443",
    ];
    if !db_ports.iter().any(|needle| lower.contains(needle)) {
        return false;
    }
    if lower.contains("127.0.0.1")
        || lower.contains("localhost")
        || lower.contains("host.docker.internal")
        || lower.contains("internal")
    {
        return false;
    }
    full_text.contains("ports:")
}

fn docker_line_kind(file: &FileInfo) -> &'static str {
    let lower = file.rel_path.to_ascii_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        "yaml"
    } else {
        "shell"
    }
}
