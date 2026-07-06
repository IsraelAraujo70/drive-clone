ALTER TABLE files DROP CONSTRAINT files_state_check;
ALTER TABLE files ADD CONSTRAINT files_state_check CHECK (state IN ('pending', 'complete', 'expired'));

ALTER TABLE files DROP CONSTRAINT files_check;
ALTER TABLE files ADD CONSTRAINT files_state_completed_at_check
    CHECK ((state IN ('pending', 'expired') AND completed_at IS NULL) OR (state = 'complete' AND completed_at IS NOT NULL));

ALTER TABLE files
    ADD COLUMN upload_kind TEXT NOT NULL DEFAULT 'direct',
    ADD COLUMN multipart_upload_id TEXT,
    ADD COLUMN upload_expires_at TIMESTAMPTZ,
    ADD COLUMN part_size_bytes BIGINT,
    ADD CONSTRAINT files_upload_kind_check CHECK (upload_kind IN ('direct', 'resumable')),
    ADD CONSTRAINT files_resumable_upload_shape_check CHECK (
        (upload_kind = 'direct' AND multipart_upload_id IS NULL AND part_size_bytes IS NULL)
        OR
        (upload_kind = 'resumable' AND multipart_upload_id IS NOT NULL AND upload_expires_at IS NOT NULL AND part_size_bytes IS NOT NULL AND part_size_bytes > 0)
    );

CREATE TABLE upload_parts (
    file_id UUID NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    part_number INT NOT NULL CHECK (part_number BETWEEN 1 AND 10000),
    size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
    etag TEXT NOT NULL CHECK (length(trim(etag)) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (file_id, part_number)
);

CREATE INDEX upload_parts_file_order_idx ON upload_parts (file_id, part_number);
CREATE INDEX files_resumable_expiry_idx ON files (upload_expires_at, id)
    WHERE upload_kind = 'resumable' AND state = 'pending';
