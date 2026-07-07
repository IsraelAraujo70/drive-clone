ALTER TABLE files ADD COLUMN purge_claimed_at TIMESTAMPTZ;

CREATE INDEX files_purge_claim_idx
    ON files (deleted_at ASC, id ASC)
    WHERE deleted_at IS NOT NULL AND state = 'complete';
