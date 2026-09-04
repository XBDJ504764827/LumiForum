-- Private messaging permissions: send and manage own conversations.

INSERT INTO permissions (code, name, description)
VALUES
    ('dm.read:self', 'Read own private messages', 'List own conversations and messages'),
    ('dm.write', 'Send private messages', 'Send messages to other users')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE
    permissions.code IN ('dm.read:self', 'dm.write')
    AND (
        roles.code = 'super_administrator'
        OR roles.code IN ('user', 'moderator', 'administrator')
    )
ON CONFLICT (role_id, permission_id) DO NOTHING;