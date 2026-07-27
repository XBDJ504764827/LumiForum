DELETE FROM role_permissions
WHERE permission_id IN (
    SELECT id FROM permissions
    WHERE code IN ('upload.create', 'upload.read:self', 'upload.delete:self')
);

DELETE FROM permissions
WHERE code IN ('upload.create', 'upload.read:self', 'upload.delete:self');

DROP INDEX IF EXISTS users_avatar_upload_id_idx;
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_avatar_upload_id_fkey;
ALTER TABLE users DROP COLUMN IF EXISTS avatar_upload_id;
ALTER TABLE users RENAME COLUMN avatar_url TO avatar;

DROP TRIGGER IF EXISTS uploads_set_updated_at ON uploads;
DROP TABLE uploads;
