-- rollback proof: downgrade recreates the table
-- backup snapshot taken before deployment
DROP TABLE old_sessions;
