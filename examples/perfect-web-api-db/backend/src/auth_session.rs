// Auth/session certified-cell shell.
//
// Boundary contract:
// - Owns session identity, token-hash invariants, expiry/revocation decisions,
//   session port traits, and application-level create/revoke orchestration.
// - Does not own OAuth/SAML/passkey providers, HTTP extraction, raw credentials,
//   DB SQL, environment variables, random generation, or wall-clock reads.
//
// Production split recommendation:
// - Move pure value objects to domain/auth_session.rs.
// - Move command functions and port traits to application/auth_session.rs.
// - Keep provider integrations and SQL in adapters.

use crate::domain::{Account, AccountId, DomainError};
use std::fmt;

// ---------------------------------------------------------------------------
// Session value objects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Hash or digest of a bearer token. Raw tokens never cross durable boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionTokenHash(pub String);

// ---------------------------------------------------------------------------
// Session policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPolicy {
    pub ttl_seconds: u64,
    pub max_active_sessions_per_account: usize,
}

impl SessionPolicy {
    pub fn standard() -> Self {
        Self {
            ttl_seconds: 60 * 60 * 8,
            max_active_sessions_per_account: 5,
        }
    }

    pub fn validate(&self) -> Result<(), SessionError> {
        if self.ttl_seconds == 0 {
            return Err(SessionError::InvalidPolicy {
                reason: "ttl_seconds must be greater than zero".to_string(),
            });
        }
        if self.max_active_sessions_per_account == 0 {
            return Err(SessionError::InvalidPolicy {
                reason: "max_active_sessions_per_account must be greater than zero".to_string(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Session aggregate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,
    pub account_id: AccountId,
    pub token_hash: SessionTokenHash,
    pub created_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
    pub revoked_at_epoch_seconds: Option<u64>,
}

impl Session {
    pub fn new(
        id: impl Into<String>,
        account_id: AccountId,
        token_hash: impl Into<String>,
        now_epoch_seconds: u64,
        policy: &SessionPolicy,
    ) -> Result<Self, SessionError> {
        policy.validate()?;

        let token_hash = token_hash.into();
        if token_hash.trim().is_empty() {
            return Err(SessionError::EmptyTokenHash);
        }
        if token_hash.len() < 32 {
            return Err(SessionError::WeakTokenHash);
        }

        Ok(Self {
            id: SessionId(id.into()),
            account_id,
            token_hash: SessionTokenHash(token_hash),
            created_at_epoch_seconds: now_epoch_seconds,
            expires_at_epoch_seconds: now_epoch_seconds.saturating_add(policy.ttl_seconds),
            revoked_at_epoch_seconds: None,
        })
    }

    pub fn is_active(&self, now_epoch_seconds: u64) -> bool {
        self.revoked_at_epoch_seconds.is_none() && now_epoch_seconds < self.expires_at_epoch_seconds
    }

    pub fn revoke(&mut self, now_epoch_seconds: u64) {
        if self.revoked_at_epoch_seconds.is_none() {
            self.revoked_at_epoch_seconds = Some(now_epoch_seconds);
        }
    }
}

// ---------------------------------------------------------------------------
// Session errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    EmptyTokenHash,
    WeakTokenHash,
    AccountInactive,
    PermissionDenied { action: String },
    TooManyActiveSessions { account_id: AccountId, limit: usize },
    SessionNotFound { id: SessionId },
    SessionExpired { id: SessionId },
    SessionRevoked { id: SessionId },
    InvalidPolicy { reason: String },
    AdapterFailure { reason: String },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTokenHash => f.write_str("session token hash must not be empty"),
            Self::WeakTokenHash => {
                f.write_str("session token hash must be at least 32 characters")
            }
            Self::AccountInactive => f.write_str("inactive account cannot hold a session"),
            Self::PermissionDenied { action } => write!(f, "session action denied: {action}"),
            Self::TooManyActiveSessions { account_id, limit } => {
                write!(
                    f,
                    "account {account_id} exceeds active session limit {limit}"
                )
            }
            Self::SessionNotFound { id } => write!(f, "session {id} not found"),
            Self::SessionExpired { id } => write!(f, "session {id} expired"),
            Self::SessionRevoked { id } => write!(f, "session {id} revoked"),
            Self::InvalidPolicy { reason } => write!(f, "invalid session policy: {reason}"),
            Self::AdapterFailure { reason } => write!(f, "session adapter failed: {reason}"),
        }
    }
}

impl From<DomainError> for SessionError {
    fn from(value: DomainError) -> Self {
        match value {
            DomainError::AccountInactive => Self::AccountInactive,
            DomainError::PermissionDenied { action, .. } => Self::PermissionDenied { action },
            other => Self::AdapterFailure {
                reason: other.to_string(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Port traits (adapter contracts)
// ---------------------------------------------------------------------------

pub trait SessionRepository {
    fn count_active_for_account(
        &self,
        account_id: &AccountId,
        now_epoch_seconds: u64,
    ) -> Result<usize, String>;

    fn save(&self, session: &Session) -> Result<(), String>;

    fn find_by_id(&self, id: &SessionId) -> Result<Option<Session>, String>;
}

pub trait SessionAuditLog {
    fn record_session_event(&self, event: SessionEvent) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// Session audit events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEvent {
    pub actor_id: AccountId,
    pub session_id: SessionId,
    pub action: SessionEventAction,
    pub outcome: SessionEventOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEventAction {
    Created,
    Revoked,
    Expired,
    AuthenticationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEventOutcome {
    Success,
    Denied,
    Error(String),
}

// ---------------------------------------------------------------------------
// Application commands
// ---------------------------------------------------------------------------

pub fn create_session(
    actor: &Account,
    session_id: impl Into<String>,
    token_hash: impl Into<String>,
    now_epoch_seconds: u64,
    policy: &SessionPolicy,
    sessions: &impl SessionRepository,
    audit_log: &impl SessionAuditLog,
) -> Result<Session, SessionError> {
    actor.authorize("read")?;

    let active_count = sessions
        .count_active_for_account(&actor.id, now_epoch_seconds)
        .map_err(|reason| SessionError::AdapterFailure { reason })?;

    if active_count >= policy.max_active_sessions_per_account {
        let _ = audit_log.record_session_event(SessionEvent {
            actor_id: actor.id.clone(),
            session_id: SessionId("pending".to_string()),
            action: SessionEventAction::AuthenticationFailed,
            outcome: SessionEventOutcome::Denied,
        });

        return Err(SessionError::TooManyActiveSessions {
            account_id: actor.id.clone(),
            limit: policy.max_active_sessions_per_account,
        });
    }

    let session = Session::new(session_id, actor.id.clone(), token_hash, now_epoch_seconds, policy)?;

    sessions
        .save(&session)
        .map_err(|reason| SessionError::AdapterFailure { reason })?;

    let _ = audit_log.record_session_event(SessionEvent {
        actor_id: actor.id.clone(),
        session_id: session.id.clone(),
        action: SessionEventAction::Created,
        outcome: SessionEventOutcome::Success,
    });

    Ok(session)
}

pub fn revoke_session(
    actor: &Account,
    session_id: &SessionId,
    now_epoch_seconds: u64,
    sessions: &impl SessionRepository,
    audit_log: &impl SessionAuditLog,
) -> Result<Session, SessionError> {
    actor.authorize("delete")?;

    let mut session = sessions
        .find_by_id(session_id)
        .map_err(|reason| SessionError::AdapterFailure { reason })?
        .ok_or_else(|| SessionError::SessionNotFound {
            id: session_id.clone(),
        })?;

    if !session.is_active(now_epoch_seconds) {
        if session.revoked_at_epoch_seconds.is_some() {
            return Err(SessionError::SessionRevoked {
                id: session_id.clone(),
            });
        }
        return Err(SessionError::SessionExpired {
            id: session_id.clone(),
        });
    }

    session.revoke(now_epoch_seconds);

    sessions
        .save(&session)
        .map_err(|reason| SessionError::AdapterFailure { reason })?;

    let _ = audit_log.record_session_event(SessionEvent {
        actor_id: actor.id.clone(),
        session_id: session.id.clone(),
        action: SessionEventAction::Revoked,
        outcome: SessionEventOutcome::Success,
    });

    Ok(session)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Role;
    use std::cell::RefCell;

    struct FakeSessions {
        items: RefCell<Vec<Session>>,
    }

    impl FakeSessions {
        fn new() -> Self {
            Self {
                items: RefCell::new(Vec::new()),
            }
        }
    }

    impl SessionRepository for FakeSessions {
        fn count_active_for_account(
            &self,
            account_id: &AccountId,
            now_epoch_seconds: u64,
        ) -> Result<usize, String> {
            Ok(self
                .items
                .borrow()
                .iter()
                .filter(|s| s.account_id == *account_id && s.is_active(now_epoch_seconds))
                .count())
        }

        fn save(&self, session: &Session) -> Result<(), String> {
            let mut items = self.items.borrow_mut();
            if let Some(existing) = items.iter_mut().find(|s| s.id == session.id) {
                *existing = session.clone();
            } else {
                items.push(session.clone());
            }
            Ok(())
        }

        fn find_by_id(&self, id: &SessionId) -> Result<Option<Session>, String> {
            Ok(self.items.borrow().iter().find(|s| s.id == *id).cloned())
        }
    }

    struct FakeSessionAudit {
        events: RefCell<Vec<SessionEvent>>,
    }

    impl FakeSessionAudit {
        fn new() -> Self {
            Self {
                events: RefCell::new(Vec::new()),
            }
        }
    }

    impl SessionAuditLog for FakeSessionAudit {
        fn record_session_event(&self, event: SessionEvent) -> Result<(), String> {
            self.events.borrow_mut().push(event);
            Ok(())
        }
    }

    fn strong_hash() -> &'static str {
        "0123456789abcdef0123456789abcdef"
    }

    #[test]
    fn session_requires_strong_non_empty_hash() {
        let account = Account::new("a1", "alice@example.com", Role::Member).unwrap();
        let policy = SessionPolicy::standard();

        assert!(matches!(
            Session::new("s1", account.id.clone(), "", 0, &policy),
            Err(SessionError::EmptyTokenHash)
        ));
        assert!(matches!(
            Session::new("s1", account.id, "short", 0, &policy),
            Err(SessionError::WeakTokenHash)
        ));
    }

    #[test]
    fn create_session_emits_success_event() {
        let account = Account::new("a1", "alice@example.com", Role::Member).unwrap();
        let sessions = FakeSessions::new();
        let audit = FakeSessionAudit::new();
        let policy = SessionPolicy::standard();

        let session =
            create_session(&account, "s1", strong_hash(), 100, &policy, &sessions, &audit)
                .unwrap();

        assert_eq!(session.account_id, account.id);
        assert_eq!(audit.events.borrow().len(), 1);
        assert_eq!(
            audit.events.borrow()[0].action,
            SessionEventAction::Created
        );
    }

    #[test]
    fn create_session_enforces_active_session_limit() {
        let account = Account::new("a1", "alice@example.com", Role::Member).unwrap();
        let sessions = FakeSessions::new();
        let audit = FakeSessionAudit::new();
        let policy = SessionPolicy {
            ttl_seconds: 100,
            max_active_sessions_per_account: 1,
        };

        create_session(&account, "s1", strong_hash(), 100, &policy, &sessions, &audit).unwrap();

        let err = create_session(
            &account,
            "s2",
            "fedcba9876543210fedcba9876543210",
            101,
            &policy,
            &sessions,
            &audit,
        )
        .unwrap_err();

        assert!(matches!(err, SessionError::TooManyActiveSessions { .. }));
    }

    #[test]
    fn revoke_session_requires_privileged_actor() {
        let account = Account::new("a1", "alice@example.com", Role::Member).unwrap();
        let viewer = Account::new("v1", "viewer@example.com", Role::Viewer).unwrap();
        let sessions = FakeSessions::new();
        let audit = FakeSessionAudit::new();
        let policy = SessionPolicy::standard();

        create_session(&account, "s1", strong_hash(), 100, &policy, &sessions, &audit).unwrap();

        let err = revoke_session(
            &viewer,
            &SessionId("s1".to_string()),
            101,
            &sessions,
            &audit,
        )
        .unwrap_err();

        assert!(matches!(err, SessionError::PermissionDenied { .. }));
    }
}
