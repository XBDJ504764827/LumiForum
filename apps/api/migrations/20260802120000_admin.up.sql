-- Phase 9: admin panel foundation (RBAC, reports, audit logs, moderation fields).

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS last_login_at timestamptz;

CREATE INDEX IF NOT EXISTS users_last_login_at_idx
    ON users (last_login_at DESC NULLS LAST);

ALTER TABLE topics DROP CONSTRAINT IF EXISTS topics_status_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_status_check
        CHECK (status IN ('published', 'hidden', 'deleted'));

ALTER TABLE topics DROP CONSTRAINT IF EXISTS topics_soft_delete_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_soft_delete_check
        CHECK (
            (status = 'deleted' AND deleted_at IS NOT NULL)
            OR (status IN ('published', 'hidden') AND deleted_at IS NULL)
        );

CREATE OR REPLACE FUNCTION set_user_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF (
        NEW.username,
        NEW.email,
        NEW.password_hash,
        NEW.avatar_url,
        NEW.avatar_upload_id,
        NEW.nickname,
        NEW.role_id,
        NEW.status,
        NEW.email_verified,
        NEW.email_verified_at,
        NEW.auth_version,
        NEW.last_login_at
    ) IS DISTINCT FROM (
        OLD.username,
        OLD.email,
        OLD.password_hash,
        OLD.avatar_url,
        OLD.avatar_upload_id,
        OLD.nickname,
        OLD.role_id,
        OLD.status,
        OLD.email_verified,
        OLD.email_verified_at,
        OLD.auth_version,
        OLD.last_login_at
    ) THEN
        NEW.updated_at = now();
    ELSE
        NEW.updated_at = OLD.updated_at;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TABLE reports (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type varchar(32) NOT NULL,
    target_id uuid NOT NULL,
    reason varchar(500) NOT NULL,
    details text,
    status varchar(32) NOT NULL DEFAULT 'open',
    handler_id uuid REFERENCES users(id) ON DELETE SET NULL,
    resolution_note text,
    handled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT reports_target_type_check
        CHECK (target_type IN ('topic', 'comment', 'user')),
    CONSTRAINT reports_reason_check
        CHECK (char_length(btrim(reason)) BETWEEN 3 AND 500),
    CONSTRAINT reports_details_check
        CHECK (details IS NULL OR char_length(details) <= 2000),
    CONSTRAINT reports_status_check
        CHECK (status IN ('open', 'reviewing', 'resolved', 'rejected')),
    CONSTRAINT reports_handled_check
        CHECK (
            (status IN ('resolved', 'rejected') AND handler_id IS NOT NULL AND handled_at IS NOT NULL)
            OR (status IN ('open', 'reviewing') AND handled_at IS NULL)
        )
);

CREATE INDEX reports_status_created_idx
    ON reports (status, created_at DESC);

CREATE INDEX reports_target_idx
    ON reports (target_type, target_id);

CREATE INDEX reports_reporter_created_idx
    ON reports (reporter_id, created_at DESC);

CREATE TRIGGER reports_set_updated_at
BEFORE UPDATE ON reports
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE admin_logs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    action varchar(64) NOT NULL,
    target_type varchar(32) NOT NULL,
    target_id uuid,
    summary text NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    ip_address inet,
    user_agent varchar(512),
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT admin_logs_action_check
        CHECK (char_length(btrim(action)) BETWEEN 2 AND 64),
    CONSTRAINT admin_logs_target_type_check
        CHECK (char_length(btrim(target_type)) BETWEEN 2 AND 32),
    CONSTRAINT admin_logs_summary_check
        CHECK (char_length(btrim(summary)) BETWEEN 1 AND 1000)
);

CREATE INDEX admin_logs_created_idx
    ON admin_logs (created_at DESC);

CREATE INDEX admin_logs_admin_created_idx
    ON admin_logs (admin_id, created_at DESC);

CREATE INDEX admin_logs_target_idx
    ON admin_logs (target_type, target_id);

INSERT INTO permissions (code, name, description)
VALUES
    ('admin.access', 'Access admin panel', 'Access the administrative panel and /admin APIs'),
    ('user.manage', 'Manage users', 'List, search, suspend, disable, and soft-delete users'),
    ('topic.manage', 'Manage topics', 'List and moderate all topics from admin panel'),
    ('comment.manage', 'Manage comments', 'List and moderate all comments from admin panel'),
    ('file.manage', 'Manage files', 'List and delete uploaded files from admin panel'),
    ('report.manage', 'Manage reports', 'Review and resolve user reports'),
    ('report.create', 'Create reports', 'Report topics, comments, or users'),
    ('system.manage', 'Manage system', 'View dashboard aggregates and admin operation logs')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code IN (
    'admin.access',
    'user.manage',
    'topic.manage',
    'comment.manage',
    'file.manage',
    'report.manage',
    'system.manage'
)
AND roles.code IN ('administrator', 'super_administrator')
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code = 'report.create'
  AND roles.code IN ('user', 'moderator', 'administrator', 'super_administrator')
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code = 'user.role.assign'
  AND roles.code = 'super_administrator'
ON CONFLICT DO NOTHING;
