-- Steam OpenID authentication fields (coexist with password auth).

-- Allow Steam-only accounts without a local password.
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_password_hash_check;
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
ALTER TABLE users
    ADD CONSTRAINT users_password_hash_check
        CHECK (
            password_hash IS NULL
            OR password_hash LIKE '$argon2id$%'
        );

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS steam_id varchar(32),
    ADD COLUMN IF NOT EXISTS steam_persona_name varchar(128),
    ADD COLUMN IF NOT EXISTS steam_avatar_medium text,
    ADD COLUMN IF NOT EXISTS steam_avatar_full text,
    ADD COLUMN IF NOT EXISTS steam_profile_url text;

ALTER TABLE users
    ADD CONSTRAINT users_steam_id_format_check
        CHECK (steam_id IS NULL OR steam_id ~ '^[0-9]{17}$');

CREATE UNIQUE INDEX IF NOT EXISTS users_steam_id_uidx
    ON users (steam_id)
    WHERE steam_id IS NOT NULL;

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
        NEW.last_login_at,
        NEW.steam_id,
        NEW.steam_persona_name,
        NEW.steam_avatar_medium,
        NEW.steam_avatar_full,
        NEW.steam_profile_url
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
        OLD.last_login_at,
        OLD.steam_id,
        OLD.steam_persona_name,
        OLD.steam_avatar_medium,
        OLD.steam_avatar_full,
        OLD.steam_profile_url
    ) THEN
        NEW.updated_at = now();
    ELSE
        NEW.updated_at = OLD.updated_at;
    END IF;
    RETURN NEW;
END;
$$;
