ALTER TABLE files ADD COLUMN deleted_at TIMESTAMPTZ;

CREATE TABLE file_shares (
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    grantee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (file_id, grantee_id)
);

CREATE INDEX file_shares_grantee_created_idx ON file_shares (grantee_id, created_at DESC);
