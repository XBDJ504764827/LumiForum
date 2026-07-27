DELETE FROM role_permissions
WHERE permission_id IN (
    SELECT id FROM permissions
    WHERE code IN ('notification.read:self', 'notification.update:self')
);

DELETE FROM permissions
WHERE code IN ('notification.read:self', 'notification.update:self');

DROP TABLE IF EXISTS notifications;
