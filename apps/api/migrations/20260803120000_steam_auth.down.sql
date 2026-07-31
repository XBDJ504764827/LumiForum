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

DROP INDEX IF EXISTS users_steam_id_uidx;
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_steam_id_format_check;
ALTER TABLE users
    DROP COLUMN IF EXISTS steam_profile_url,
    DROP COLUMN IF EXISTS steam_avatar_full,
    DROP COLUMN IF EXISTS steam_avatar_medium,
    DROP COLUMN IF EXISTS steam_persona_name,
    DROP COLUMN IF EXISTS steam_id;

-- Restore non-null password requirement for remaining rows.
UPDATE users
SET password_hash = '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'
WHERE password_hash IS NULL;

ALTER TABLE users DROP CONSTRAINT IF EXISTS users_password_hash_check;
ALTER TABLE users ALTER COLUMN password_hash SET NOT NULL;
ALTER TABLE users
    ADD CONSTRAINT users_password_hash_check
        CHECK (password_hash LIKE '$argon2id$%');
