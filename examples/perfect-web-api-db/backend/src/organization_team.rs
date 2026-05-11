// Organization/team certified-cell shell.
//
// Boundary contract:
// - Owns tenant team value objects, membership policy, pure membership
//   invariants, application-level create/add/archive orchestration, and audit
//   event shapes.
// - Does not own provider directory sync, billing seats, HTTP extraction, raw
//   SQL, environment variables, or generated client code.
//
// Production split recommendation:
// - Move pure value objects to domain/organization_team.rs.
// - Move command functions and port traits to application/organization_team.rs.
// - Keep provider sync, SQL, billing-seat checks, and webhooks in adapters.

use crate::domain::{Account, AccountId, AuditOutcome, DomainError, Organization, OrganizationId};
use std::fmt;

// ---------------------------------------------------------------------------
// Organization/team value objects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TeamId(pub String);

impl fmt::Display for TeamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    pub id: TeamId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub archived: bool,
}

impl Team {
    pub fn new(
        id: impl Into<String>,
        organization_id: OrganizationId,
        name: impl Into<String>,
    ) -> Result<Self, OrganizationTeamError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(OrganizationTeamError::EmptyTeamName);
        }
        Ok(Self {
            id: TeamId(id.into()),
            organization_id,
            name,
            archived: false,
        })
    }

    pub fn archive(&mut self) {
        self.archived = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamRole {
    Manager,
    Contributor,
    Viewer,
}

impl TeamRole {
    pub fn can_manage_members(&self) -> bool {
        matches!(self, TeamRole::Manager)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMembership {
    pub team_id: TeamId,
    pub account_id: AccountId,
    pub role: TeamRole,
}

// ---------------------------------------------------------------------------
// Membership policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMembershipPolicy {
    pub max_teams_per_organization: usize,
    pub max_members_per_team: usize,
}

impl TeamMembershipPolicy {
    pub fn standard() -> Self {
        Self {
            max_teams_per_organization: 25,
            max_members_per_team: 100,
        }
    }

    pub fn validate(&self) -> Result<(), OrganizationTeamError> {
        if self.max_teams_per_organization == 0 {
            return Err(OrganizationTeamError::InvalidPolicy {
                reason: "max_teams_per_organization must be greater than zero".to_string(),
            });
        }
        if self.max_members_per_team == 0 {
            return Err(OrganizationTeamError::InvalidPolicy {
                reason: "max_members_per_team must be greater than zero".to_string(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrganizationTeamError {
    EmptyTeamName,
    AccountInactive,
    PermissionDenied { action: String },
    OrganizationTeamLimitReached { org_id: OrganizationId, limit: usize },
    TeamMemberLimitReached { team_id: TeamId, limit: usize },
    TeamNotFound { id: TeamId },
    TeamArchived { id: TeamId },
    DuplicateMembership { team_id: TeamId, account_id: AccountId },
    InvalidPolicy { reason: String },
    AdapterFailure { reason: String },
}

impl fmt::Display for OrganizationTeamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTeamName => f.write_str("team name must not be empty"),
            Self::AccountInactive => f.write_str("inactive account cannot manage teams"),
            Self::PermissionDenied { action } => write!(f, "team action denied: {action}"),
            Self::OrganizationTeamLimitReached { org_id, limit } => {
                write!(f, "organization {org_id} reached team limit {limit}")
            }
            Self::TeamMemberLimitReached { team_id, limit } => {
                write!(f, "team {team_id} reached member limit {limit}")
            }
            Self::TeamNotFound { id } => write!(f, "team {id} not found"),
            Self::TeamArchived { id } => write!(f, "team {id} is archived"),
            Self::DuplicateMembership { team_id, account_id } => {
                write!(f, "account {account_id} is already a member of team {team_id}")
            }
            Self::InvalidPolicy { reason } => write!(f, "invalid team policy: {reason}"),
            Self::AdapterFailure { reason } => write!(f, "organization/team adapter failed: {reason}"),
        }
    }
}

impl From<DomainError> for OrganizationTeamError {
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

pub trait OrganizationTeamRepository {
    fn count_teams_for_org(&self, org_id: &OrganizationId) -> Result<usize, String>;
    fn count_members_for_team(&self, team_id: &TeamId) -> Result<usize, String>;
    fn find_team(&self, team_id: &TeamId) -> Result<Option<Team>, String>;
    fn save_team(&self, team: &Team) -> Result<(), String>;
    fn find_membership(
        &self,
        team_id: &TeamId,
        account_id: &AccountId,
    ) -> Result<Option<TeamMembership>, String>;
    fn save_membership(&self, membership: &TeamMembership) -> Result<(), String>;
}

pub trait OrganizationTeamAuditLog {
    fn record_organization_team_event(&self, event: OrganizationTeamEvent) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// Audit event contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationTeamEvent {
    pub actor_id: AccountId,
    pub organization_id: OrganizationId,
    pub team_id: TeamId,
    pub action: OrganizationTeamEventAction,
    pub outcome: AuditOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrganizationTeamEventAction {
    TeamCreated,
    TeamArchived,
    MemberAdded,
    MemberRemoved,
}

// ---------------------------------------------------------------------------
// Application commands
// ---------------------------------------------------------------------------

pub fn create_team(
    actor: &Account,
    organization: &Organization,
    team_id: impl Into<String>,
    name: impl Into<String>,
    policy: &TeamMembershipPolicy,
    teams: &impl OrganizationTeamRepository,
    audit_log: &impl OrganizationTeamAuditLog,
) -> Result<Team, OrganizationTeamError> {
    actor.authorize("manage_members")?;
    policy.validate()?;

    let team_count = teams
        .count_teams_for_org(&organization.id)
        .map_err(|reason| OrganizationTeamError::AdapterFailure { reason })?;
    if team_count >= policy.max_teams_per_organization {
        return Err(OrganizationTeamError::OrganizationTeamLimitReached {
            org_id: organization.id.clone(),
            limit: policy.max_teams_per_organization,
        });
    }

    let team = Team::new(team_id, organization.id.clone(), name)?;

    teams
        .save_team(&team)
        .map_err(|reason| OrganizationTeamError::AdapterFailure { reason })?;

    let _ = audit_log.record_organization_team_event(OrganizationTeamEvent {
        actor_id: actor.id.clone(),
        organization_id: organization.id.clone(),
        team_id: team.id.clone(),
        action: OrganizationTeamEventAction::TeamCreated,
        outcome: AuditOutcome::Success,
    });

    Ok(team)
}

pub fn add_team_member(
    actor: &Account,
    team_id: &TeamId,
    account_id: AccountId,
    role: TeamRole,
    policy: &TeamMembershipPolicy,
    teams: &impl OrganizationTeamRepository,
    audit_log: &impl OrganizationTeamAuditLog,
) -> Result<TeamMembership, OrganizationTeamError> {
    actor.authorize("manage_members")?;
    policy.validate()?;

    let team = teams
        .find_team(team_id)
        .map_err(|reason| OrganizationTeamError::AdapterFailure { reason })?
        .ok_or_else(|| OrganizationTeamError::TeamNotFound { id: team_id.clone() })?;
    if team.archived {
        return Err(OrganizationTeamError::TeamArchived { id: team.id });
    }

    if teams
        .find_membership(team_id, &account_id)
        .map_err(|reason| OrganizationTeamError::AdapterFailure { reason })?
        .is_some()
    {
        return Err(OrganizationTeamError::DuplicateMembership {
            team_id: team_id.clone(),
            account_id,
        });
    }

    let member_count = teams
        .count_members_for_team(team_id)
        .map_err(|reason| OrganizationTeamError::AdapterFailure { reason })?;
    if member_count >= policy.max_members_per_team {
        return Err(OrganizationTeamError::TeamMemberLimitReached {
            team_id: team_id.clone(),
            limit: policy.max_members_per_team,
        });
    }

    let membership = TeamMembership {
        team_id: team_id.clone(),
        account_id: account_id.clone(),
        role,
    };

    teams
        .save_membership(&membership)
        .map_err(|reason| OrganizationTeamError::AdapterFailure { reason })?;

    let _ = audit_log.record_organization_team_event(OrganizationTeamEvent {
        actor_id: actor.id.clone(),
        organization_id: team.organization_id,
        team_id: team_id.clone(),
        action: OrganizationTeamEventAction::MemberAdded,
        outcome: AuditOutcome::Success,
    });

    Ok(membership)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Role;
    use std::cell::RefCell;

    struct FakeTeams {
        teams: RefCell<Vec<Team>>,
        memberships: RefCell<Vec<TeamMembership>>,
    }

    impl FakeTeams {
        fn new() -> Self {
            Self {
                teams: RefCell::new(Vec::new()),
                memberships: RefCell::new(Vec::new()),
            }
        }
    }

    impl OrganizationTeamRepository for FakeTeams {
        fn count_teams_for_org(&self, org_id: &OrganizationId) -> Result<usize, String> {
            Ok(self
                .teams
                .borrow()
                .iter()
                .filter(|team| team.organization_id == *org_id && !team.archived)
                .count())
        }

        fn count_members_for_team(&self, team_id: &TeamId) -> Result<usize, String> {
            Ok(self
                .memberships
                .borrow()
                .iter()
                .filter(|member| member.team_id == *team_id)
                .count())
        }

        fn find_team(&self, team_id: &TeamId) -> Result<Option<Team>, String> {
            Ok(self.teams.borrow().iter().find(|team| team.id == *team_id).cloned())
        }

        fn save_team(&self, team: &Team) -> Result<(), String> {
            self.teams.borrow_mut().push(team.clone());
            Ok(())
        }

        fn find_membership(
            &self,
            team_id: &TeamId,
            account_id: &AccountId,
        ) -> Result<Option<TeamMembership>, String> {
            Ok(self
                .memberships
                .borrow()
                .iter()
                .find(|member| member.team_id == *team_id && member.account_id == *account_id)
                .cloned())
        }

        fn save_membership(&self, membership: &TeamMembership) -> Result<(), String> {
            self.memberships.borrow_mut().push(membership.clone());
            Ok(())
        }
    }

    struct FakeAuditLog {
        events: RefCell<Vec<OrganizationTeamEvent>>,
    }

    impl FakeAuditLog {
        fn new() -> Self {
            Self {
                events: RefCell::new(Vec::new()),
            }
        }
    }

    impl OrganizationTeamAuditLog for FakeAuditLog {
        fn record_organization_team_event(&self, event: OrganizationTeamEvent) -> Result<(), String> {
            self.events.borrow_mut().push(event);
            Ok(())
        }
    }

    #[test]
    fn create_team_requires_manage_members_authority() {
        let viewer = Account::new("v1", "viewer@example.com", Role::Viewer).unwrap();
        let org = Organization::new("org1", "Acme", 10);
        let teams = FakeTeams::new();
        let audit = FakeAuditLog::new();
        let policy = TeamMembershipPolicy::standard();

        let err = create_team(&viewer, &org, "team1", "Platform", &policy, &teams, &audit)
            .unwrap_err();
        assert!(matches!(err, OrganizationTeamError::PermissionDenied { .. }));
    }

    #[test]
    fn create_team_emits_audit_event() {
        let admin = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let org = Organization::new("org1", "Acme", 10);
        let teams = FakeTeams::new();
        let audit = FakeAuditLog::new();
        let policy = TeamMembershipPolicy::standard();

        let team = create_team(&admin, &org, "team1", "Platform", &policy, &teams, &audit)
            .unwrap();

        assert_eq!(team.organization_id, org.id);
        assert_eq!(audit.events.borrow().len(), 1);
        assert_eq!(
            audit.events.borrow()[0].action,
            OrganizationTeamEventAction::TeamCreated
        );
    }

    #[test]
    fn create_team_enforces_org_team_limit() {
        let admin = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let org = Organization::new("org1", "Acme", 10);
        let teams = FakeTeams::new();
        let audit = FakeAuditLog::new();
        let policy = TeamMembershipPolicy {
            max_teams_per_organization: 1,
            max_members_per_team: 5,
        };

        create_team(&admin, &org, "team1", "Platform", &policy, &teams, &audit).unwrap();
        let err = create_team(&admin, &org, "team2", "Security", &policy, &teams, &audit)
            .unwrap_err();

        assert!(matches!(err, OrganizationTeamError::OrganizationTeamLimitReached { .. }));
    }

    #[test]
    fn add_team_member_rejects_duplicate_membership() {
        let admin = Account::new("a1", "admin@example.com", Role::Admin).unwrap();
        let member = Account::new("m1", "member@example.com", Role::Member).unwrap();
        let org = Organization::new("org1", "Acme", 10);
        let teams = FakeTeams::new();
        let audit = FakeAuditLog::new();
        let policy = TeamMembershipPolicy::standard();
        let team = create_team(&admin, &org, "team1", "Platform", &policy, &teams, &audit)
            .unwrap();

        add_team_member(
            &admin,
            &team.id,
            member.id.clone(),
            TeamRole::Contributor,
            &policy,
            &teams,
            &audit,
        )
        .unwrap();
        let err = add_team_member(
            &admin,
            &team.id,
            member.id,
            TeamRole::Viewer,
            &policy,
            &teams,
            &audit,
        )
        .unwrap_err();

        assert!(matches!(err, OrganizationTeamError::DuplicateMembership { .. }));
    }
}
