CREATE TABLE folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    parent_folder_id UUID REFERENCES folders(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    deleted_by_folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    CHECK (length(trim(name)) > 0),
    CHECK (char_length(name) <= 255),
    CHECK (name NOT IN ('.', '..')),
    CHECK (position('/' in name) = 0),
    CHECK (position(E'\\' in name) = 0),
    CHECK (parent_folder_id IS NULL OR parent_folder_id <> id)
);

ALTER TABLE files
    ADD COLUMN parent_folder_id UUID REFERENCES folders(id) ON DELETE RESTRICT,
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN deleted_by_folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

CREATE INDEX folders_owner_parent_active_idx
    ON folders(owner_id, parent_folder_id, name, id)
    WHERE deleted_at IS NULL;

CREATE INDEX folders_owner_deleted_idx
    ON folders(owner_id, deleted_at DESC, id DESC)
    WHERE deleted_at IS NOT NULL;

CREATE INDEX folders_parent_idx ON folders(parent_folder_id);

CREATE INDEX files_owner_parent_completed_active_idx
    ON files(owner_id, parent_folder_id, completed_at DESC, id DESC)
    WHERE state = 'complete' AND deleted_at IS NULL;

CREATE INDEX files_owner_deleted_idx
    ON files(owner_id, deleted_at DESC, id DESC)
    WHERE deleted_at IS NOT NULL;
