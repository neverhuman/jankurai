-- Migration 002: Auth/session shell.
--
-- Invariants enforced at DB level:
-- - Session IDs are stable primary keys.
-- - Raw session tokens are never stored; token_hash is mandatory.
-- - Token hashes are unique and non-empty.
-- - Every session belongs to an account.
-- - Expiry must be after creation.
-- - Revocation, when present, cannot predate creation.
--
-- Rollback:
-- DROP INDEX IF EXISTS idx_auth_sessions_active_account;
-- DROP INDEX IF EXISTS idx_auth_sessions_expires_at;
-- DROP TABLE IF EXISTS auth_sessions;

CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE CHECK (token_hash <> '' AND char_length(token_hash) >= 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    provider TEXT NOT NULL DEFAULT 'local-shell' CHECK (provider IN ('local-shell', 'oidc-shell', 'saml-shell', 'passkey-shell')),
    CHECK (expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX idx_auth_sessions_active_account
    ON auth_sessions(account_id)
    WHERE revoked_at IS NULL;

CREATE INDEX idx_auth_sessions_expires_at
    ON auth_sessions(expires_at);
