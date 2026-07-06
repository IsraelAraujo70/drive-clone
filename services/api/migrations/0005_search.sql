CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX files_active_filename_trgm_idx
    ON files USING gin (lower(filename) gin_trgm_ops)
    WHERE state = 'complete' AND deleted_at IS NULL;

CREATE INDEX files_deleted_filename_trgm_idx
    ON files USING gin (lower(filename) gin_trgm_ops)
    WHERE state = 'complete' AND deleted_at IS NOT NULL;

CREATE INDEX file_shares_grantee_file_idx
    ON file_shares (grantee_id, file_id);
