-- rollback proof: companion down migration recreates sessions table
-- backup snapshot taken before deployment
DROP TABLE old_sessions CASCADE;
