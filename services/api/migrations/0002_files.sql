CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
    checksum_sha256 TEXT,
    object_key TEXT NOT NULL UNIQUE,
    state TEXT NOT NULL CHECK (state IN ('pending', 'complete')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    CHECK (checksum_sha256 IS NULL OR checksum_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK ((state = 'pending' AND completed_at IS NULL) OR (state = 'complete' AND completed_at IS NOT NULL))
);

CREATE INDEX files_owner_completed_idx ON files (owner_id, completed_at DESC, id DESC) WHERE state = 'complete';
CREATE INDEX files_owner_state_idx ON files (owner_id, state);
