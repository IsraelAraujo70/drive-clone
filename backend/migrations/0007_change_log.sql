ALTER TABLE users ADD COLUMN change_seq BIGINT NOT NULL DEFAULT 0;

CREATE TABLE change_log (
    owner_id    UUID NOT NULL REFERENCES users(id),
    seq         BIGINT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('file', 'folder')),
    entity_id   UUID NOT NULL,
    op          TEXT NOT NULL CHECK (op IN ('upsert', 'delete')),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, seq)
);
