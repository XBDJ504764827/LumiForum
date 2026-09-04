DELETE FROM role_permissions
WHERE permission_id IN (SELECT id FROM permissions WHERE code IN ('dm.read:self', 'dm.write'));

DELETE FROM permissions WHERE code IN ('dm.read:self', 'dm.write');