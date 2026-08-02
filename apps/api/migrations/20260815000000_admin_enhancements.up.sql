-- =============================================================================
-- Phase 15: Admin enhancement — system settings + settings.manage permission.
-- =============================================================================

CREATE TABLE system_settings (
    key varchar(64) PRIMARY KEY,
    value jsonb NOT NULL,
    description varchar(200),
    updated_by uuid REFERENCES users(id) ON DELETE SET NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO system_settings (key, value, description)
VALUES
    ('site_name', '"LumiForum"', '论坛名称'),
    ('site_description', '"现代化社区论坛 — 讨论、分享与协作"', '论坛描述'),
    ('registration_enabled', 'true', '是否开放注册'),
    ('topic_create_enabled', 'true', '是否允许发帖'),
    ('comment_enabled', 'true', '是否允许评论'),
    ('upload_enabled', 'true', '是否允许上传'),
    ('upload_max_bytes', '10485760', '单文件大小上限（字节）')
ON CONFLICT (key) DO NOTHING;

INSERT INTO permissions (code, name, description)
VALUES
    ('settings.manage', 'Manage system settings', 'Read and update system settings')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code = 'settings.manage'
  AND roles.code IN ('administrator', 'super_administrator')
ON CONFLICT DO NOTHING;
