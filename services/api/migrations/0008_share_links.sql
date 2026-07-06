CREATE TABLE share_links (
    id          UUID PRIMARY KEY,
    file_id     UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    created_by  UUID NOT NULL REFERENCES users(id),
    token_hash  BYTEA NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX share_links_file_idx ON share_links (file_id);
