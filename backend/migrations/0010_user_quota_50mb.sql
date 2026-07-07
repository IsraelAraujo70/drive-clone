SET LOCAL lock_timeout = '3s';
SET LOCAL statement_timeout = '10s';

ALTER TABLE users
    ALTER COLUMN storage_quota_bytes SET DEFAULT 52428800;

UPDATE users
SET storage_quota_bytes = 52428800
WHERE storage_quota_bytes <> 52428800;
