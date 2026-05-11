// Root library module for the perfect-web-api-db reference backend.
//
// Layer responsibilities (from Jankurai JANKURAI_STANDARD.md):
//
//   domain      — IDs, invariants, pure decisions. No IO.
//   application — commands, authorization, idempotency, transactions. No UI.
//   adapters    — DB, queue clients, external APIs, filesystem. No domain rules.

pub mod adapters;
pub mod application;
pub mod auth_session;
pub mod background_job;
pub mod domain;
pub mod organization_team;
pub mod webhook_receiver;
pub mod notification_shell;
pub mod periodic_cron;
pub mod billing_subscription;

pub fn service_name() -> &'static str {
    "perfect-web-api-db"
}
