ALTER TABLE topics
    ADD COLUMN IF NOT EXISTS last_reply_user_id uuid REFERENCES users(id) ON DELETE SET NULL;

CREATE TABLE comments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    topic_id uuid NOT NULL REFERENCES topics(id) ON DELETE RESTRICT,
    author_id uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    parent_id uuid REFERENCES comments(id) ON DELETE RESTRICT,
    content text NOT NULL,
    status varchar(32) NOT NULL DEFAULT 'published',
    like_count bigint NOT NULL DEFAULT 0,
    reply_count bigint NOT NULL DEFAULT 0,
    edited_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz,
    CONSTRAINT comments_content_check
        CHECK (char_length(content) BETWEEN 1 AND 20000),
    CONSTRAINT comments_status_check
        CHECK (status IN ('published', 'deleted')),
    CONSTRAINT comments_soft_delete_check
        CHECK (
            (status = 'published' AND deleted_at IS NULL)
            OR (status = 'deleted' AND deleted_at IS NOT NULL)
        ),
    CONSTRAINT comments_counters_check
        CHECK (like_count >= 0 AND reply_count >= 0),
    CONSTRAINT comments_edited_check
        CHECK (edited_at IS NULL OR edited_at >= created_at)
);

CREATE OR REPLACE FUNCTION set_comment_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF (
        NEW.content,
        NEW.status,
        NEW.deleted_at
    ) IS DISTINCT FROM (
        OLD.content,
        OLD.status,
        OLD.deleted_at
    ) THEN
        NEW.updated_at = now();
    ELSE
        NEW.updated_at = OLD.updated_at;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER comments_set_updated_at
BEFORE UPDATE ON comments
FOR EACH ROW
EXECUTE FUNCTION set_comment_updated_at();

CREATE OR REPLACE FUNCTION enforce_comment_parent_rules()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    parent_topic uuid;
    parent_parent uuid;
    parent_status text;
BEGIN
    IF NEW.parent_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT topic_id, parent_id, status
    INTO parent_topic, parent_parent, parent_status
    FROM comments
    WHERE id = NEW.parent_id
    FOR SHARE;

    IF parent_topic IS NULL THEN
        RAISE EXCEPTION 'comment parent not found';
    END IF;
    IF parent_parent IS NOT NULL THEN
        RAISE EXCEPTION 'comment nesting deeper than 2 levels is not allowed';
    END IF;
    IF parent_topic <> NEW.topic_id THEN
        RAISE EXCEPTION 'comment parent must belong to the same topic';
    END IF;
    IF TG_OP = 'INSERT' AND parent_status <> 'published' THEN
        RAISE EXCEPTION 'cannot reply to a deleted comment';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER comments_enforce_parent_rules
BEFORE INSERT OR UPDATE OF parent_id, topic_id ON comments
FOR EACH ROW
EXECUTE FUNCTION enforce_comment_parent_rules();

CREATE INDEX comments_topic_roots_idx
    ON comments (topic_id, created_at ASC, id ASC)
    WHERE parent_id IS NULL AND status = 'published';

CREATE INDEX comments_parent_children_idx
    ON comments (parent_id, created_at ASC, id ASC)
    WHERE parent_id IS NOT NULL AND status = 'published';

CREATE INDEX comments_topic_created_idx
    ON comments (topic_id, created_at DESC, id DESC);

CREATE INDEX comments_author_idx
    ON comments (author_id, created_at DESC, id DESC);

CREATE INDEX comments_deleted_at_idx
    ON comments (deleted_at)
    WHERE deleted_at IS NOT NULL;

CREATE OR REPLACE FUNCTION set_topic_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF (
        NEW.category_id,
        NEW.author_id,
        NEW.title,
        NEW.slug,
        NEW.content,
        NEW.summary,
        NEW.status,
        NEW.is_pinned,
        NEW.is_featured,
        NEW.deleted_at
    ) IS DISTINCT FROM (
        OLD.category_id,
        OLD.author_id,
        OLD.title,
        OLD.slug,
        OLD.content,
        OLD.summary,
        OLD.status,
        OLD.is_pinned,
        OLD.is_featured,
        OLD.deleted_at
    ) THEN
        NEW.updated_at = now();
    ELSE
        NEW.updated_at = OLD.updated_at;
    END IF;
    RETURN NEW;
END;
$$;

INSERT INTO permissions (code, name, description)
VALUES
    ('comment.create', 'Create comments', 'Create root comments on published topics'),
    ('comment.reply', 'Reply to comments', 'Reply to root comments'),
    ('comment.update:self', 'Update own comments', 'Update comments authored by the acting user'),
    ('comment.update:any', 'Update any comment', 'Update comments regardless of author'),
    ('comment.delete:self', 'Delete own comments', 'Soft-delete comments authored by the acting user'),
    ('comment.delete:any', 'Delete any comment', 'Soft-delete comments regardless of author'),
    ('comment.restore', 'Restore comments', 'Restore soft-deleted comments')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE
    permissions.code IN (
        'comment.create',
        'comment.reply',
        'comment.update:self',
        'comment.update:any',
        'comment.delete:self',
        'comment.delete:any',
        'comment.restore'
    )
    AND (
        roles.code = 'super_administrator'
        OR permissions.code IN ('comment.create', 'comment.reply', 'comment.update:self', 'comment.delete:self')
        OR (
            roles.code IN ('moderator', 'administrator')
            AND permissions.code IN ('comment.update:any', 'comment.delete:any', 'comment.restore')
        )
        OR (
            roles.code = 'user'
            AND permissions.code IN ('comment.create', 'comment.reply', 'comment.update:self', 'comment.delete:self')
        )
    )
ON CONFLICT (role_id, permission_id) DO NOTHING;
