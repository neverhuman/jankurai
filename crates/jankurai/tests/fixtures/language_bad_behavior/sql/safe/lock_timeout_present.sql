SET lock_timeout = '5s';
SET statement_timeout = '30s';
ALTER TABLE big_accounts DROP COLUMN legacy_flag;
