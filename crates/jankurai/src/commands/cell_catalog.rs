use crate::commands::context_data::RepoCatalog;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct CellManifest {
    pub cell_id: String,
    pub version: String,
    pub category: String,
    pub lifecycle: String,
    pub supported_profiles: Vec<String>,
    pub dependencies: Vec<String>,
    pub source_paths: Vec<String>,
    pub generated_paths: Vec<String>,
    pub contract_paths: Vec<String>,
    pub migration_paths: Vec<String>,
    pub ui_routes: Vec<String>,
    pub proof_lanes: Vec<String>,
    pub proof_commands: Vec<String>,
    pub security_assumptions: Vec<String>,
    pub observability_events: Vec<String>,
    pub docs: Vec<String>,
    pub upgrade_notes: Vec<String>,
    pub rollback_notes: Vec<String>,
    pub certification_status: String,
    pub certification_evidence: Vec<CellEvidence>,
    pub install_strategy: String,
    pub conflict_policy: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CellEvidence {
    pub kind: String,
    pub path: String,
    pub required: bool,
    pub status: String,
}

#[derive(Debug, Clone, Copy)]
pub struct EvidenceCounts {
    pub present: usize,
    pub missing: usize,
    pub review_required: usize,
}

pub fn built_in_manifests(repo: &Path, catalog: &RepoCatalog) -> Vec<CellManifest> {
    vec![
        audit_log_manifest(repo, catalog),
        crud_resource_manifest(repo, catalog),
        rbac_manifest(repo, catalog),
        auth_session_manifest(repo, catalog),
        organization_team_manifest(repo, catalog),
        background_job_manifest(repo, catalog),
        webhook_receiver_manifest(repo, catalog),
        notification_shell_manifest(repo, catalog),
        periodic_cron_manifest(repo, catalog),
        billing_subscription_manifest(repo, catalog),
    ]
}

pub fn manifest_for_cell(repo: &Path, catalog: &RepoCatalog, cell_id: &str) -> CellManifest {
    built_in_manifests(repo, catalog)
        .into_iter()
        .find(|manifest| manifest.cell_id == cell_id)
        .unwrap_or_else(|| fallback_manifest(catalog, cell_id))
}

pub fn evidence_counts(manifest: &CellManifest) -> EvidenceCounts {
    let mut counts = EvidenceCounts {
        present: 0,
        missing: 0,
        review_required: 0,
    };
    for evidence in &manifest.certification_evidence {
        match evidence.status.as_str() {
            "present" => counts.present += 1,
            "missing" => counts.missing += 1,
            "review-required" => counts.review_required += 1,
            _ => {}
        }
    }
    counts
}

pub fn owner_for_cell(catalog: &RepoCatalog, cell_id: &str) -> String {
    if let Some(owner) = cell_id.split_once('-').map(|(owner, _)| owner.to_string()) {
        if catalog.owners.values().any(|candidate| candidate == &owner) {
            return owner;
        }
    }
    "workspace".to_string()
}

pub fn category_for_owner(owner: &str) -> &'static str {
    match owner {
        "agent" => "agent-surface",
        "paper" => "documentation",
        "ops" => "governance",
        "standard" => "standard",
        "tools" => "tooling",
        _ => "engineering",
    }
}

pub fn source_paths_for_owner(catalog: &RepoCatalog, owner: &str) -> Vec<String> {
    let mut paths = catalog.prefixes_for_owner(owner);
    if paths.is_empty() {
        paths.push("crates/jankurai/src/commands/".to_string());
    }
    paths
}

fn audit_log_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/ops/observability.md",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&["examples/perfect-web-api-db/contracts/openapi.json"]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
    ]);
    let proof_lanes = strings(&["test-cli", "audit", "db-migration-analyze", "security"]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "audit-log".to_string(),
            version: "0.1.0".to_string(),
            category: "observability".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: Vec::new(),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes: Vec::new(),
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "audit events are append-only and written through the application port",
                "actors and targets are traceable without embedding secrets",
            ]),
            observability_events: strings(&[
                "audit_events_total",
                "resource.created",
                "resource.deleted",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/ops/observability.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
            ]),
            upgrade_notes: strings(&[
                "extend the AuditLog port before adding provider-specific sinks",
                "regenerate client contracts when audit event APIs become public",
            ]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "for applied templates, remove audit table additions only through a reviewed migration",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn crud_resource_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/frontend/src/App.tsx",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&["examples/perfect-web-api-db/contracts/openapi.json"]);
    let migration_paths = strings(&["examples/perfect-web-api-db/db/migrations/001_init.sql"]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "crud-resource".to_string(),
            version: "0.1.0".to_string(),
            category: "product-ui".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "CRUD authorization is enforced in the Rust application layer",
                "frontend uses contract-shaped data and does not own durable truth",
            ]),
            observability_events: strings(&["resource.created", "resource.deleted"]),
            docs: strings(&[
                "examples/perfect-web-api-db/ux/routes.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
            ]),
            upgrade_notes: strings(&[
                "add generated clients before exposing more resource endpoints",
                "expand route states in ux/routes.md with every new CRUD surface",
            ]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "for applied templates, reverse resource tables through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn rbac_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/docs/exceptions.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&["examples/perfect-web-api-db/contracts/openapi.json"]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "rbac".to_string(),
            version: "0.1.0".to_string(),
            category: "authorization".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["crud-resource"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "roles and permissions are enforced in the Rust application layer before any command runs",
                "API security schemes in OpenAPI align with session or token checks at the edge",
                "dangerous role changes emit audit events through the audit-log cell",
            ]),
            observability_events: strings(&["authorization.denied", "authorization.allowed"]),
            docs: strings(&[
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&[
                "model new roles in domain.rs before exposing them in OpenAPI",
                "expand ux/routes.md with permission-denied coverage for each protected surface",
            ]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "reverse RBAC table or policy changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn auth_session_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/auth_session.rs",
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&[
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/contracts/auth-session.openapi.json",
    ]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/migrations/002_auth_sessions.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
        "examples/perfect-web-api-db/db/constraints/002_auth_sessions.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/auth-session-routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "auth-session".to_string(),
            version: "0.1.0".to_string(),
            category: "identity".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log", "rbac"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "session identity resolves to an active Account before application commands run",
                "bearer tokens are API-edge credentials only; durable authorization remains in the Rust application layer",
                "session/token material is never committed to source, fixtures, logs, or proof artifacts",
                "auth/session events that affect access are observable through audit-log evidence",
                "permission-denied UI states are covered as user-facing auth evidence",
            ]),
            observability_events: strings(&[
                "session.created",
                "session.revoked",
                "session.expired",
                "authentication.failed",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/docs/auth-session-cell.md",
                "examples/perfect-web-api-db/ops/auth-session-security.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&[
                "add provider-backed login only after the API edge can prove bearer token validation with explicit receipts",
                "introduce a dedicated sessions table only through reviewed migrations and db-migration-analyze proof",
                "keep token parsing at the API edge and pass an Account/SessionPrincipal into application commands",
                "add revocation and rotation contracts before moving beyond shell certification",
            ]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "remove provider config only after revoking issued credentials",
                "reverse session table or token-state changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn organization_team_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/organization_team.rs",
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&[
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/contracts/organization-team.openapi.json",
    ]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/migrations/003_organization_team.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
        "examples/perfect-web-api-db/db/constraints/003_organization_team.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/organization-team-routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "organization-team".to_string(),
            version: "0.1.0".to_string(),
            category: "organization".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log", "rbac", "auth-session"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "organization membership changes require an active authenticated account plus RBAC manage_members authorization",
                "team membership changes emit audit-log evidence and never bypass application-layer policy",
                "tenant identifiers stay explicit on teams and memberships so cross-organization access is repairable",
                "membership invitations and provider-backed directory sync remain deferred until proved by dedicated provider contracts",
            ]),
            observability_events: strings(&[
                "organization.team.created",
                "organization.team.archived",
                "organization.team.member_added",
                "organization.team.member_removed",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/docs/organization-team-cell.md",
                "examples/perfect-web-api-db/ops/organization-team-security.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&[
                "add invitation and SCIM/provider sync only after identities and membership proof commands are explicit",
                "extend organization-team.openapi.json before exposing new membership routes to the frontend",
                "add tenant-isolation tests before allowing any cross-organization membership move",
                "keep membership table changes behind db-migration-analyze and reviewed rollback notes",
            ]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "archive teams before deleting membership data so audit history remains explainable",
                "reverse membership table or role enum changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn background_job_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/background_job.rs",
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&[
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/contracts/background-job.openapi.json",
    ]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/migrations/004_background_jobs.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
        "examples/perfect-web-api-db/db/constraints/004_background_jobs.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/background-job-routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "background-job".to_string(),
            version: "0.1.0".to_string(),
            category: "workflow".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log", "rbac", "auth-session", "organization-team"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "background work is claimed by authenticated service or admin principals before execution",
                "payload bodies are referenced by opaque payload_ref values and are not embedded in source, logs, or proof artifacts",
                "retry, exhaustion, and completion decisions are durable and observable through audit-log evidence",
                "provider-specific queue backends, cron triggers, and webhook dispatch remain adapter concerns until separately certified",
            ]),
            observability_events: strings(&[
                "background_job.enqueued",
                "background_job.claimed",
                "background_job.completed",
                "background_job.failed",
                "background_job.retried",
                "background_job.exhausted",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/docs/background-job-cell.md",
                "examples/perfect-web-api-db/ops/background-job-security.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&[
                "add provider-backed queue adapters only after idempotency keys, visibility timeout, and dead-letter behavior are proven",
                "extend background-job.openapi.json before exposing queue operations to the frontend or external workers",
                "add destructive retry/backfill proof before enabling mutating install behavior",
                "keep queue storage changes behind db-migration-analyze and reviewed rollback notes",
            ]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "pause workers before rolling back queue schema or retry policy changes",
                "drain or dead-letter queued jobs before removing provider-specific queue adapters",
                "reverse background job table changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn webhook_receiver_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/webhook_receiver.rs",
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&[
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/contracts/webhook-receiver.openapi.json",
    ]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/migrations/005_webhook_receipts.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
        "examples/perfect-web-api-db/db/constraints/005_webhook_receipts.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/webhook-receiver-routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "webhook-receiver".to_string(),
            version: "0.1.0".to_string(),
            category: "integration".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log", "background-job"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "webhook signatures are verified at the application edge before parsing the payload",
                "webhook receipts are durably stored to ensure idempotency",
                "long-running processing is deferred to background-job",
            ]),
            observability_events: strings(&[
                "webhook.received",
                "webhook.signature_failed",
                "webhook.processed",
                "webhook.duplicate",
                "webhook.failed",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/docs/webhook-receiver-cell.md",
                "examples/perfect-web-api-db/ops/webhook-receiver-security.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&[
                "extend the webhook-receiver.openapi.json before exposing new webhook providers",
                "add provider-specific signature verification logic inside the edge layer",
            ]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "reverse webhook receipt table changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn notification_shell_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/notification_shell.rs",
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&[
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/contracts/notification-shell.openapi.json",
    ]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/migrations/006_notifications.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
        "examples/perfect-web-api-db/db/constraints/006_notifications.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/notification-shell-routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "notification-shell".to_string(),
            version: "0.1.0".to_string(),
            category: "integration".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log", "background-job"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "PII is scrubbed from logs before external dispatch",
                "notification delivery relies on background-job for retries",
            ]),
            observability_events: strings(&[
                "notification.queued",
                "notification.delivered",
                "notification.failed",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/docs/notification-shell-cell.md",
                "examples/perfect-web-api-db/ops/notification-shell-security.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&["add new delivery methods via adapter implementations"]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "reverse notification table changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn periodic_cron_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/periodic_cron.rs",
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&[
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/contracts/periodic-cron.openapi.json",
    ]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/migrations/007_periodic_cron.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
        "examples/perfect-web-api-db/db/constraints/007_periodic_cron.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/periodic-cron-routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "periodic-cron".to_string(),
            version: "0.1.0".to_string(),
            category: "workflow".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log", "background-job"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "cron execution evaluation operates independently from worker execution (uses background-job)",
                "schedules are durably tracked with last_run_at and next_run_at semantics",
                "leader election or distinct worker scheduling prevents duplicate schedule evaluation",
            ]),
            observability_events: strings(&[
                "cron.schedule_created",
                "cron.schedule_paused",
                "cron.schedule_resumed",
                "cron.triggered",
                "cron.missed",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/docs/periodic-cron-cell.md",
                "examples/perfect-web-api-db/ops/periodic-cron-security.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&["modify OpenAPI specs before adding new cron schedules to the API edge"]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "pause schedules before dropping the periodic_cron_schedules table during a rollback",
                "reverse periodic cron table changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn billing_subscription_manifest(repo: &Path, catalog: &RepoCatalog) -> CellManifest {
    let source_paths = strings(&[
        "examples/perfect-web-api-db/backend/src/billing_subscription.rs",
        "examples/perfect-web-api-db/backend/src/domain.rs",
        "examples/perfect-web-api-db/backend/src/application.rs",
        "examples/perfect-web-api-db/backend/src/adapters.rs",
        "examples/perfect-web-api-db/docs/architecture.md",
        "examples/perfect-web-api-db/README.md",
    ]);
    let contract_paths = strings(&[
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/contracts/billing-subscription.openapi.json",
    ]);
    let migration_paths = strings(&[
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/migrations/008_billing_subscriptions.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
        "examples/perfect-web-api-db/db/constraints/008_billing_subscriptions.sql",
    ]);
    let ui_routes = strings(&["examples/perfect-web-api-db/ux/billing-subscription-routes.md"]);
    let proof_lanes = strings(&[
        "test-cli",
        "audit",
        "db-migration-analyze",
        "ux-qa",
        "security",
    ]);
    certified_manifest(
        repo,
        catalog,
        CellManifest {
            cell_id: "billing-subscription".to_string(),
            version: "0.1.0".to_string(),
            category: "commerce".to_string(),
            lifecycle: "certified".to_string(),
            supported_profiles: strings(&["perfect-web-api-db"]),
            dependencies: strings(&["audit-log", "organization-team", "webhook-receiver"]),
            source_paths,
            generated_paths: Vec::new(),
            contract_paths,
            migration_paths,
            ui_routes,
            proof_lanes,
            proof_commands: Vec::new(),
            security_assumptions: strings(&[
                "billing and subscription states are durable in the local database and updated deterministically via webhooks",
                "feature gating queries local state rather than reaching out to the provider synchronously",
                "provider tokens and secrets are restricted to the edge layer (adapters)",
            ]),
            observability_events: strings(&[
                "billing.subscription_created",
                "billing.subscription_updated",
                "billing.subscription_canceled",
                "billing.invoice_paid",
                "billing.payment_failed",
            ]),
            docs: strings(&[
                "examples/perfect-web-api-db/docs/billing-subscription-cell.md",
                "examples/perfect-web-api-db/ops/billing-subscription-security.md",
                "examples/perfect-web-api-db/ops/security.md",
                "examples/perfect-web-api-db/docs/architecture.md",
                "examples/perfect-web-api-db/docs/exceptions.md",
            ]),
            upgrade_notes: strings(&["add provider-specific SDK implementation only after the shell domain logic is thoroughly tested"]),
            rollback_notes: strings(&[
                "dry-run install writes no files",
                "downgrade subscriptions gracefully if rolling back a plan tier",
                "reverse billing table changes only through reviewed migrations",
            ]),
            certification_status: "candidate".to_string(),
            certification_evidence: Vec::new(),
            install_strategy: "dry-run-plan".to_string(),
            conflict_policy: "never-overwrite".to_string(),
        },
    )
}

fn certified_manifest(
    repo: &Path,
    catalog: &RepoCatalog,
    mut manifest: CellManifest,
) -> CellManifest {
    let mut evidence = Vec::new();
    for path in manifest
        .source_paths
        .iter()
        .chain(manifest.contract_paths.iter())
        .chain(manifest.migration_paths.iter())
        .chain(manifest.ui_routes.iter())
        .chain(manifest.docs.iter())
    {
        evidence.push(path_evidence(repo, path));
    }
    for lane in &manifest.proof_lanes {
        evidence.push(lane_evidence(catalog, lane));
    }
    // Dependency-bound certification: check each upstream dependency is certified.
    let all_manifests = lazy_built_in_ids();
    for dep_id in &manifest.dependencies {
        let dep_certified = all_manifests.contains(&dep_id.as_str());
        evidence.push(CellEvidence {
            kind: "dependency".to_string(),
            path: dep_id.clone(),
            required: true,
            status: if dep_certified {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    // Content-marker evidence for key domain invariants.
    if manifest.cell_id == "auth-session" {
        let marker_path = "examples/perfect-web-api-db/backend/src/auth_session.rs";
        let has_marker = repo.join(marker_path).exists()
            && std::fs::read_to_string(repo.join(marker_path))
                .unwrap_or_default()
                .contains("SessionTokenHash");
        evidence.push(CellEvidence {
            kind: "content-marker".to_string(),
            path: "domain-session-token-hash".to_string(),
            required: true,
            status: if has_marker {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    if manifest.cell_id == "organization-team" {
        let marker_path = "examples/perfect-web-api-db/backend/src/organization_team.rs";
        let has_marker = repo.join(marker_path).exists()
            && std::fs::read_to_string(repo.join(marker_path))
                .unwrap_or_default()
                .contains("TeamMembershipPolicy");
        evidence.push(CellEvidence {
            kind: "content-marker".to_string(),
            path: "domain-team-membership-policy".to_string(),
            required: true,
            status: if has_marker {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    if manifest.cell_id == "background-job" {
        let marker_path = "examples/perfect-web-api-db/backend/src/background_job.rs";
        let has_marker = repo.join(marker_path).exists()
            && std::fs::read_to_string(repo.join(marker_path))
                .unwrap_or_default()
                .contains("BackgroundJobRetryPolicy");
        evidence.push(CellEvidence {
            kind: "content-marker".to_string(),
            path: "domain-background-job-retry-policy".to_string(),
            required: true,
            status: if has_marker {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    if manifest.cell_id == "webhook-receiver" {
        let marker_path = "examples/perfect-web-api-db/backend/src/webhook_receiver.rs";
        let has_marker = repo.join(marker_path).exists()
            && std::fs::read_to_string(repo.join(marker_path))
                .unwrap_or_default()
                .contains("WebhookSignaturePolicy");
        evidence.push(CellEvidence {
            kind: "content-marker".to_string(),
            path: "domain-webhook-signature-policy".to_string(),
            required: true,
            status: if has_marker {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    if manifest.cell_id == "notification-shell" {
        let marker_path = "examples/perfect-web-api-db/backend/src/notification_shell.rs";
        let has_marker = repo.join(marker_path).exists()
            && std::fs::read_to_string(repo.join(marker_path))
                .unwrap_or_default()
                .contains("NotificationDeliveryPolicy");
        evidence.push(CellEvidence {
            kind: "content-marker".to_string(),
            path: "domain-notification-delivery-policy".to_string(),
            required: true,
            status: if has_marker {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    if manifest.cell_id == "periodic-cron" {
        let marker_path = "examples/perfect-web-api-db/backend/src/periodic_cron.rs";
        let has_marker = repo.join(marker_path).exists()
            && std::fs::read_to_string(repo.join(marker_path))
                .unwrap_or_default()
                .contains("PeriodicCronSchedulePolicy");
        evidence.push(CellEvidence {
            kind: "content-marker".to_string(),
            path: "domain-periodic-cron-schedule-policy".to_string(),
            required: true,
            status: if has_marker {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    if manifest.cell_id == "billing-subscription" {
        let marker_path = "examples/perfect-web-api-db/backend/src/billing_subscription.rs";
        let has_marker = repo.join(marker_path).exists()
            && std::fs::read_to_string(repo.join(marker_path))
                .unwrap_or_default()
                .contains("BillingSubscriptionStatePolicy");
        evidence.push(CellEvidence {
            kind: "content-marker".to_string(),
            path: "domain-billing-subscription-state-policy".to_string(),
            required: true,
            status: if has_marker {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        });
    }
    manifest.proof_commands = proof_commands(catalog, &manifest.proof_lanes);
    let is_certified = evidence
        .iter()
        .all(|item| !item.required || item.status == "present");
    manifest.certification_status = if is_certified {
        "certified".to_string()
    } else {
        "candidate".to_string()
    };
    if !is_certified && manifest.lifecycle == "certified" {
        manifest.lifecycle = "experimental".to_string();
    }
    manifest.certification_evidence = evidence;
    manifest
}

/// Returns the set of built-in certified cell IDs for dependency-bound checks.
fn lazy_built_in_ids() -> Vec<&'static str> {
    vec![
        "audit-log",
        "crud-resource",
        "rbac",
        "auth-session",
        "organization-team",
        "background-job",
        "webhook-receiver",
        "notification-shell",
        "periodic-cron",
        "billing-subscription",
    ]
}

fn fallback_manifest(catalog: &RepoCatalog, cell_id: &str) -> CellManifest {
    let owner = owner_for_cell(catalog, cell_id);
    let source_paths = source_paths_for_owner(catalog, &owner);
    let proof_lanes = if catalog.proof_lane_names().is_empty() {
        strings(&["fast", "audit"])
    } else {
        catalog.proof_lane_names()
    };
    CellManifest {
        cell_id: cell_id.to_string(),
        version: "0.0.0".to_string(),
        category: category_for_owner(&owner).to_string(),
        lifecycle: "draft".to_string(),
        supported_profiles: strings(&["workspace"]),
        dependencies: Vec::new(),
        source_paths,
        generated_paths: Vec::new(),
        contract_paths: Vec::new(),
        migration_paths: Vec::new(),
        ui_routes: Vec::new(),
        proof_lanes: proof_lanes.clone(),
        proof_commands: proof_commands(catalog, &proof_lanes),
        security_assumptions: Vec::new(),
        observability_events: Vec::new(),
        docs: Vec::new(),
        upgrade_notes: strings(&[
            "add local contracts, tests, and proof lanes before certification",
        ]),
        rollback_notes: strings(&["dry-run install writes no files"]),
        certification_status: "candidate".to_string(),
        certification_evidence: vec![CellEvidence {
            kind: "review".to_string(),
            path: cell_id.to_string(),
            required: true,
            status: "review-required".to_string(),
        }],
        install_strategy: "manual".to_string(),
        conflict_policy: "review-required".to_string(),
    }
}

fn path_evidence(repo: &Path, path: &str) -> CellEvidence {
    CellEvidence {
        kind: "path".to_string(),
        path: path.to_string(),
        required: true,
        status: if repo.join(path).exists() {
            "present".to_string()
        } else {
            "missing".to_string()
        },
    }
}

fn lane_evidence(catalog: &RepoCatalog, lane: &str) -> CellEvidence {
    CellEvidence {
        kind: "proof-lane".to_string(),
        path: lane.to_string(),
        required: true,
        status: if catalog.proof_lanes.iter().any(|item| item.name == lane) {
            "present".to_string()
        } else {
            "missing".to_string()
        },
    }
}

fn proof_commands(catalog: &RepoCatalog, lanes: &[String]) -> Vec<String> {
    let mut commands = Vec::new();
    for lane_name in lanes {
        for lane in &catalog.proof_lanes {
            if lane.name == *lane_name && !commands.contains(&lane.command) {
                commands.push(lane.command.clone());
            }
        }
    }
    commands
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}
