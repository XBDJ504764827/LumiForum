CREATE TABLE uploads (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    filename varchar(255) NOT NULL,
    original_filename varchar(255) NOT NULL,
    storage_provider varchar(32) NOT NULL,
    storage_key varchar(1024) NOT NULL UNIQUE,
    mime_type varchar(128) NOT NULL,
    file_size bigint NOT NULL,
    category varchar(32) NOT NULL,
    url text,
    thumbnail_storage_key varchar(1024) UNIQUE,
    thumbnail_url text,
    width integer,
    height integer,
    status varchar(32) NOT NULL DEFAULT 'pending',
    deleted_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uploads_filename_check
        CHECK (char_length(btrim(filename)) BETWEEN 1 AND 255),
    CONSTRAINT uploads_original_filename_check
        CHECK (char_length(btrim(original_filename)) BETWEEN 1 AND 255),
    CONSTRAINT uploads_storage_provider_check
        CHECK (storage_provider IN ('local', 's3')),
    CONSTRAINT uploads_storage_key_check
        CHECK (char_length(storage_key) BETWEEN 1 AND 1024 AND storage_key !~ '(^|/)\.\.(/|$)'),
    CONSTRAINT uploads_mime_type_check
        CHECK (mime_type ~ '^[a-z0-9][a-z0-9.+-]*/[a-z0-9][a-z0-9.+-]*$'),
    CONSTRAINT uploads_file_size_check
        CHECK (file_size > 0 AND file_size <= 20971520),
    CONSTRAINT uploads_category_check
        CHECK (category IN ('avatar', 'topic_image', 'comment_image', 'attachment')),
    CONSTRAINT uploads_status_check
        CHECK (status IN ('pending', 'ready', 'deleting', 'deleted', 'failed')),
    CONSTRAINT uploads_dimensions_check
        CHECK ((width IS NULL AND height IS NULL) OR (width > 0 AND height > 0)),
    CONSTRAINT uploads_ready_url_check
        CHECK (status <> 'ready' OR url IS NOT NULL),
    CONSTRAINT uploads_deleted_at_check
        CHECK ((status = 'deleted') = (deleted_at IS NOT NULL))
);

CREATE INDEX uploads_user_created_idx
    ON uploads (user_id, created_at DESC);

CREATE INDEX uploads_user_category_ready_idx
    ON uploads (user_id, category, created_at DESC)
    WHERE status = 'ready';

CREATE INDEX uploads_cleanup_idx
    ON uploads (status, updated_at)
    WHERE status IN ('pending', 'failed', 'deleting');

CREATE TRIGGER uploads_set_updated_at
BEFORE UPDATE ON uploads
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

ALTER TABLE users RENAME COLUMN avatar TO avatar_url;
ALTER TABLE users ADD COLUMN avatar_upload_id uuid;
ALTER TABLE users
    ADD CONSTRAINT users_avatar_upload_id_fkey
    FOREIGN KEY (avatar_upload_id) REFERENCES uploads(id) ON DELETE SET NULL;
CREATE INDEX users_avatar_upload_id_idx ON users (avatar_upload_id)
    WHERE avatar_upload_id IS NOT NULL;

INSERT INTO permissions (code, name, description)
VALUES
    ('upload.create', 'Upload files', 'Upload files owned by the authenticated user'),
    ('upload.read:self', 'Read own uploads', 'Read upload metadata owned by the authenticated user'),
    ('upload.delete:self', 'Delete own uploads', 'Delete uploads owned by the authenticated user');

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE roles.code IN ('user', 'moderator', 'administrator', 'super_administrator')
  AND permissions.code IN ('upload.create', 'upload.read:self', 'upload.delete:self');
