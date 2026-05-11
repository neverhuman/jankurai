// Adapters layer: database access, external API clients, queue/streaming clients.
// Owns: SQL queries, connection pools, HTTP client calls, filesystem IO.
// Never owns: domain invariants, authorization rules, business decisions.
//
// Production implementations of AccountRepository, ResourceRepository, and
// AuditLog (from the application layer) belong here.
//
// This scaffold does not include a running database. The adapters are
// documented to show correct boundary placement. Real implementations
// would use sqlx, diesel, or equivalent Rust DB crates.

// ---------------------------------------------------------------------------
// Example: PostgreSQL adapter for AccountRepository
// ---------------------------------------------------------------------------
//
// ```rust
// use sqlx::PgPool;
// use crate::application::AccountRepository;
// use crate::domain::{Account, AccountId};
//
// pub struct PgAccountRepository {
//     pool: PgPool,
// }
//
// impl AccountRepository for PgAccountRepository {
//     fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, String> {
//         // SELECT id, email, active, role, organization_id
//         // FROM accounts WHERE id = $1
//         todo!("implement with sqlx")
//     }
//
//     fn save(&self, account: &Account) -> Result<(), String> {
//         // INSERT INTO accounts (id, email, active, role, organization_id)
//         // VALUES ($1, $2, $3, $4, $5)
//         // ON CONFLICT (id) DO UPDATE SET ...
//         todo!("implement with sqlx")
//     }
// }
// ```
//
// The key boundary rules:
//
// 1. Raw SQL lives here, not in domain or application.
// 2. The adapter maps between DB rows and domain types.
// 3. Transaction boundaries are managed by the application layer
//    through a unit-of-work or transaction port, not by raw
//    adapter-level commit/rollback.
// 4. The adapter never makes authorization decisions.
// 5. The adapter never invents domain invariants.

// ---------------------------------------------------------------------------
// Example: Audit log adapter for append-only event storage
// ---------------------------------------------------------------------------
//
// ```rust
// use sqlx::PgPool;
// use crate::application::AuditLog;
// use crate::domain::AuditEvent;
//
// pub struct PgAuditLog {
//     pool: PgPool,
// }
//
// impl AuditLog for PgAuditLog {
//     fn record(&self, event: AuditEvent) -> Result<(), String> {
//         // INSERT INTO audit_events (actor_id, action, target_kind,
//         //   target_id, outcome, created_at)
//         // VALUES ($1, $2, $3, $4, $5, now())
//         todo!("implement with sqlx")
//     }
// }
// ```
