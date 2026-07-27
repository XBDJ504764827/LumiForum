-- Phase 6: unified in-app notification center.
-- Single table + type/metadata keeps the model extensible without per-type tables.

CREATE TABLE notifications (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actor_id uuid REFERENCES users(id) ON DELETE SET NULL,
    type varchar(64) NOT NULL,
    title varchar(200) NOT NULL,
    content text NOT NULL DEFAULT '',
    target_type varchar(32),
    target_id uuid,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    is_read boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT notifications_type_check
        CHECK (
            type IN (
                'post_liked',
                'comment_liked',
                'comment_created',
                'comment_replied',
                'topic_favorited',
                'user_followed',
                'mentioned',
                'system_message'
            )
        ),
    CONSTRAINT notifications_title_check
        CHECK (title = btrim(title) AND char_length(title) BETWEEN 1 AND 200),
    CONSTRAINT notifications_content_check
        CHECK (char_length(content) <= 2000),
    CONSTRAINT notifications_target_type_check
        CHECK (
            target_type IS NULL
            OR target_type IN ('topic', 'comment', 'user', 'system')
        ),
    CONSTRAINT notifications_target_pair_check
        CHECK (
            (target_type IS NULL AND target_id IS NULL)
            OR (target_type IS NOT NULL AND target_id IS NOT NULL)
        ),
    CONSTRAINT notifications_no_self_actor_check
        CHECK (actor_id IS NULL OR actor_id <> user_id)
);

CREATE INDEX notifications_user_created_idx
    ON notifications (user_id, created_at DESC, id DESC);

CREATE INDEX notifications_user_unread_idx
    ON notifications (user_id, created_at DESC, id DESC)
    WHERE is_read = false;

CREATE INDEX notifications_user_type_idx
    ON notifications (user_id, type, created_at DESC, id DESC);

CREATE INDEX notifications_target_idx
    ON notifications (target_type, target_id)
    WHERE target_id IS NOT NULL;

INSERT INTO permissions (code, name, description)
VALUES
    ('notification.read:self', 'Read own notifications', 'List and read notifications for the acting user'),
    ('notification.update:self', 'Update own notifications', 'Mark own notifications as read')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE
    permissions.code IN ('notification.read:self', 'notification.update:self')
    AND (
        roles.code = 'super_administrator'
        OR roles.code IN ('user', 'moderator', 'administrator')
    )
ON CONFLICT (role_id, permission_id) DO NOTHING;
