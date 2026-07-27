DELETE FROM role_permissions
WHERE permission_id IN (
    SELECT id FROM permissions
    WHERE code IN (
        'admin.access',
        'user.manage',
        'topic.manage',
        'comment.manage',
        'file.manage',
        'report.manage',
        'report.create',
        'system.manage'
    )
);

DELETE FROM permissions
WHERE code IN (
    'admin.access',
    'user.manage',
    'topic.manage',
    'comment.manage',
    'file.manage',
    'report.manage',
    'report.create',
    'system.manage'
);

DROP TRIGGER IF EXISTS reports_set_updated_at ON reports;
DROP TABLE IF EXISTS admin_logs;
DROP TABLE IF EXISTS reports;

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
        NEW.auth_version
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
        OLD.auth_version
    ) THEN
        NEW.updated_at = now();
    ELSE
        NEW.updated_at = OLD.updated_at;
    END IF;
    RETURN NEW;
END;
$$;

UPDATE topics SET status = 'published', deleted_at = NULL WHERE status = 'hidden';

ALTER TABLE topics DROP CONSTRAINT IF EXISTS topics_soft_delete_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_soft_delete_check
        CHECK (
            (status = 'deleted' AND deleted_at IS NOT NULL)
            OR (status = 'published' AND deleted_at IS NULL)
        );

ALTER TABLE topics DROP CONSTRAINT IF EXISTS topics_status_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_status_check
        CHECK (status IN ('published', 'deleted'));

DROP INDEX IF EXISTS users_last_login_at_idx;
ALTER TABLE users DROP COLUMN IF EXISTS last_login_at;
