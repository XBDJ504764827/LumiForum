CREATE OR REPLACE FUNCTION set_user_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF (
        NEW.username,
        NEW.email,
        NEW.password_hash,
        NEW.avatar,
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
        OLD.avatar,
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
