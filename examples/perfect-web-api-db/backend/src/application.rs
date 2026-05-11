// Application layer: commands, authorization, idempotency, audit events.
// Owns: use-case orchestration, authorization enforcement, transactional boundaries.
// Never owns: UI, external protocol details, raw SQL, domain rules.

use crate::domain::{
    Account, AccountId, AuditEvent, AuditOutcome, DomainError, Resource, ResourceId,
};

// ---------------------------------------------------------------------------
// Port traits (adapter contracts — implemented in adapters layer, not here)
// ---------------------------------------------------------------------------

/// Repository port for Account persistence.
pub trait AccountRepository {
    fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, String>;
    fn save(&self, account: &Account) -> Result<(), String>;
}

/// Repository port for Resource persistence.
pub trait ResourceRepository {
    fn find_by_id(&self, id: &ResourceId) -> Result<Option<Resource>, String>;
    fn save(&self, resource: &Resource) -> Result<(), String>;
    fn delete(&self, id: &ResourceId) -> Result<(), String>;
    fn list_by_owner(&self, owner_id: &AccountId) -> Result<Vec<Resource>, String>;
}

/// Port for emitting audit events.
pub trait AuditLog {
    fn record(&self, event: AuditEvent) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Create a new resource, enforcing authorization and emitting an audit event.
pub fn create_resource(
    actor: &Account,
    resource_id: impl Into<String>,
    title: impl Into<String>,
    body: impl Into<String>,
    resources: &impl ResourceRepository,
    audit_log: &impl AuditLog,
) -> Result<Resource, DomainError> {
    // Authorization check
    actor.authorize("write")?;

    let resource = Resource::new(resource_id, title, body, actor.id.clone());

    // Idempotency: check for existing resource
    if let Ok(Some(_)) = resources.find_by_id(&resource.id) {
        return Err(DomainError::DuplicateResource {
            id: resource.id.clone(),
        });
    }

    // Persist (adapter boundary — we call through the port, not raw SQL)
    resources
        .save(&resource)
        .map_err(|e| DomainError::ResourceNotFound {
            id: ResourceId(e),
        })?;

    // Audit event
    let _ = audit_log.record(AuditEvent {
        actor_id: actor.id.clone(),
        action: "create_resource".to_string(),
        target_kind: "Resource".to_string(),
        target_id: resource.id.0.clone(),
        outcome: AuditOutcome::Success,
    });

    Ok(resource)
}

/// Delete a resource, enforcing delete authorization and emitting an audit event.
pub fn delete_resource(
    actor: &Account,
    resource_id: &ResourceId,
    resources: &impl ResourceRepository,
    audit_log: &impl AuditLog,
) -> Result<(), DomainError> {
    // Authorization check
    actor.authorize("delete")?;

    // Existence check
    match resources.find_by_id(resource_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err(DomainError::ResourceNotFound {
                id: resource_id.clone(),
            })
        }
        Err(e) => {
            return Err(DomainError::ResourceNotFound {
                id: ResourceId(e),
            })
        }
    }

    resources
        .delete(resource_id)
        .map_err(|e| DomainError::ResourceNotFound {
            id: ResourceId(e),
        })?;

    let _ = audit_log.record(AuditEvent {
        actor_id: actor.id.clone(),
        action: "delete_resource".to_string(),
        target_kind: "Resource".to_string(),
        target_id: resource_id.0.clone(),
        outcome: AuditOutcome::Success,
    });

    Ok(())
}

/// Check admin dashboard access — pure authorization, no side effects.
pub fn can_view_admin_dashboard(account: &Account) -> bool {
    account.authorize("view_admin").is_ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Role;
    use std::cell::RefCell;

    // Minimal in-memory test doubles — adapters layer owns the real implementations.

    struct FakeResources {
        items: RefCell<Vec<Resource>>,
    }

    impl FakeResources {
        fn new() -> Self {
            Self {
                items: RefCell::new(Vec::new()),
            }
        }
    }

    impl ResourceRepository for FakeResources {
        fn find_by_id(&self, id: &ResourceId) -> Result<Option<Resource>, String> {
            Ok(self.items.borrow().iter().find(|r| r.id == *id).cloned())
        }
        fn save(&self, resource: &Resource) -> Result<(), String> {
            self.items.borrow_mut().push(resource.clone());
            Ok(())
        }
        fn delete(&self, id: &ResourceId) -> Result<(), String> {
            self.items.borrow_mut().retain(|r| r.id != *id);
            Ok(())
        }
        fn list_by_owner(&self, owner_id: &AccountId) -> Result<Vec<Resource>, String> {
            Ok(self
                .items
                .borrow()
                .iter()
                .filter(|r| r.owner_id == *owner_id)
                .cloned()
                .collect())
        }
    }

    struct FakeAuditLog {
        events: RefCell<Vec<AuditEvent>>,
    }

    impl FakeAuditLog {
        fn new() -> Self {
            Self {
                events: RefCell::new(Vec::new()),
            }
        }
    }

    impl AuditLog for FakeAuditLog {
        fn record(&self, event: AuditEvent) -> Result<(), String> {
            self.events.borrow_mut().push(event);
            Ok(())
        }
    }

    #[test]
    fn create_resource_success() {
        let actor = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let repo = FakeResources::new();
        let log = FakeAuditLog::new();

        let result = create_resource(&actor, "r1", "Title", "Body", &repo, &log);
        assert!(result.is_ok());
        assert_eq!(repo.items.borrow().len(), 1);
        assert_eq!(log.events.borrow().len(), 1);
        assert_eq!(log.events.borrow()[0].action, "create_resource");
    }

    #[test]
    fn create_resource_denied_for_viewer() {
        let viewer = Account::new("v1", "viewer@example.com", Role::Viewer).unwrap();
        let repo = FakeResources::new();
        let log = FakeAuditLog::new();

        let result = create_resource(&viewer, "r1", "Title", "Body", &repo, &log);
        assert!(matches!(result, Err(DomainError::PermissionDenied { .. })));
    }

    #[test]
    fn create_resource_rejects_duplicate() {
        let actor = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let repo = FakeResources::new();
        let log = FakeAuditLog::new();

        let _ = create_resource(&actor, "r1", "Title", "Body", &repo, &log);
        let result = create_resource(&actor, "r1", "Title 2", "Body 2", &repo, &log);
        assert!(matches!(result, Err(DomainError::DuplicateResource { .. })));
    }

    #[test]
    fn delete_resource_denied_for_viewer() {
        let viewer = Account::new("v1", "viewer@example.com", Role::Viewer).unwrap();
        let repo = FakeResources::new();
        let log = FakeAuditLog::new();

        let result = delete_resource(&viewer, &ResourceId("r1".to_string()), &repo, &log);
        assert!(matches!(result, Err(DomainError::PermissionDenied { .. })));
    }

    #[test]
    fn delete_resource_not_found() {
        let admin = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let repo = FakeResources::new();
        let log = FakeAuditLog::new();

        let result = delete_resource(&admin, &ResourceId("r1".to_string()), &repo, &log);
        assert!(matches!(result, Err(DomainError::ResourceNotFound { .. })));
    }

    #[test]
    fn admin_dashboard_access() {
        let admin = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let viewer = Account::new("v1", "viewer@example.com", Role::Viewer).unwrap();
        let mut inactive = Account::new("i1", "inactive@example.com", Role::Admin).unwrap();
        inactive.deactivate();

        assert!(can_view_admin_dashboard(&admin));
        assert!(!can_view_admin_dashboard(&viewer));
        assert!(!can_view_admin_dashboard(&inactive));
    }
}
