DELETE FROM role_permissions rp
USING permissions p
WHERE rp.permission_id = p.id AND p.code = 'settings.manage';

DELETE FROM permissions WHERE code = 'settings.manage';

DROP TABLE IF EXISTS system_settings;
