// Domain layer: pure logic, typed IDs, invariants, no IO.
// Owns: identity types, business rules, authorization decisions, error shapes.
// Never owns: database access, HTTP, env, time, random, filesystem.

use std::fmt;

// ---------------------------------------------------------------------------
// Typed identifiers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId(pub String);

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for OrganizationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    EmptyEmail,
    InvalidEmail,
    AccountInactive,
    PermissionDenied { actor: AccountId, action: String },
    ResourceNotFound { id: ResourceId },
    DuplicateResource { id: ResourceId },
    OrganizationLimitReached { org_id: OrganizationId, limit: usize },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEmail => write!(f, "email must not be empty"),
            Self::InvalidEmail => write!(f, "email must contain @"),
            Self::AccountInactive => write!(f, "account is inactive"),
            Self::PermissionDenied { actor, action } => {
                write!(f, "account {} denied action: {}", actor, action)
            }
            Self::ResourceNotFound { id } => write!(f, "resource {} not found", id),
            Self::DuplicateResource { id } => write!(f, "resource {} already exists", id),
            Self::OrganizationLimitReached { org_id, limit } => {
                write!(f, "organization {} reached member limit {}", org_id, limit)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Role-based access control
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl Role {
    /// Returns true if this role can perform write operations.
    pub fn can_write(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin | Role::Member)
    }

    /// Returns true if this role can manage organization membership.
    pub fn can_manage_members(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    /// Returns true if this role can view the admin dashboard.
    pub fn can_view_admin(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    /// Returns true if this role can delete resources.
    pub fn can_delete(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }
}

// ---------------------------------------------------------------------------
// Account aggregate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: AccountId,
    pub email: String,
    pub active: bool,
    pub role: Role,
    pub organization_id: Option<OrganizationId>,
}

impl Account {
    /// Create a new account with validated email.
    pub fn new(
        id: impl Into<String>,
        email: impl Into<String>,
        role: Role,
    ) -> Result<Self, DomainError> {
        let email = email.into();
        if email.is_empty() {
            return Err(DomainError::EmptyEmail);
        }
        if !email.contains('@') {
            return Err(DomainError::InvalidEmail);
        }
        Ok(Self {
            id: AccountId(id.into()),
            email,
            active: true,
            role,
            organization_id: None,
        })
    }

    /// Deactivate the account. Idempotent.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Pure authorization check: can this account perform the named action?
    pub fn authorize(&self, action: &str) -> Result<(), DomainError> {
        if !self.active {
            return Err(DomainError::AccountInactive);
        }
        let allowed = match action {
            "view_admin" => self.role.can_view_admin(),
            "manage_members" => self.role.can_manage_members(),
            "write" => self.role.can_write(),
            "delete" => self.role.can_delete(),
            "read" => true, // all active accounts can read
            _ => false,
        };
        if allowed {
            Ok(())
        } else {
            Err(DomainError::PermissionDenied {
                actor: self.id.clone(),
                action: action.to_string(),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Organization aggregate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub member_limit: usize,
    pub member_count: usize,
}

impl Organization {
    pub fn new(id: impl Into<String>, name: impl Into<String>, member_limit: usize) -> Self {
        Self {
            id: OrganizationId(id.into()),
            name: name.into(),
            member_limit,
            member_count: 0,
        }
    }

    /// Check whether a new member can be added. Pure invariant.
    pub fn can_add_member(&self) -> Result<(), DomainError> {
        if self.member_count >= self.member_limit {
            Err(DomainError::OrganizationLimitReached {
                org_id: self.id.clone(),
                limit: self.member_limit,
            })
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Resource (CRUD example entity)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub id: ResourceId,
    pub title: String,
    pub body: String,
    pub owner_id: AccountId,
}

impl Resource {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
        owner_id: AccountId,
    ) -> Self {
        Self {
            id: ResourceId(id.into()),
            title: title.into(),
            body: body.into(),
            owner_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Audit log event (domain-level, no IO)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub actor_id: AccountId,
    pub action: String,
    pub target_kind: String,
    pub target_id: String,
    pub outcome: AuditOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditOutcome {
    Success,
    Denied,
    Error(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_account_validates_email() {
        assert!(Account::new("a1", "", Role::Member).is_err());
        assert!(Account::new("a1", "nope", Role::Member).is_err());
        assert!(Account::new("a1", "alice@example.com", Role::Member).is_ok());
    }

    #[test]
    fn authorization_respects_role() {
        let admin = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let viewer = Account::new("a2", "viewer@example.com", Role::Viewer).unwrap();

        assert!(admin.authorize("view_admin").is_ok());
        assert!(admin.authorize("write").is_ok());
        assert!(admin.authorize("delete").is_ok());

        assert!(viewer.authorize("read").is_ok());
        assert!(viewer.authorize("view_admin").is_err());
        assert!(viewer.authorize("write").is_err());
        assert!(viewer.authorize("delete").is_err());
    }

    #[test]
    fn inactive_account_denied_all_actions() {
        let mut account = Account::new("a1", "user@example.com", Role::Owner).unwrap();
        account.deactivate();
        assert_eq!(
            account.authorize("read"),
            Err(DomainError::AccountInactive)
        );
    }

    #[test]
    fn organization_member_limit_enforced() {
        let mut org = Organization::new("org1", "Acme", 2);
        assert!(org.can_add_member().is_ok());
        org.member_count = 2;
        assert!(org.can_add_member().is_err());
    }
}
