CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

CREATE TABLE roles (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code varchar(64) NOT NULL UNIQUE,
    name varchar(100) NOT NULL,
    description text,
    priority smallint NOT NULL DEFAULT 0,
    is_system boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT roles_code_format_check
        CHECK (code ~ '^[a-z][a-z0-9_]{1,63}$'),
    CONSTRAINT roles_name_length_check
        CHECK (char_length(btrim(name)) BETWEEN 1 AND 100),
    CONSTRAINT roles_priority_check
        CHECK (priority >= 0)
);

CREATE TRIGGER roles_set_updated_at
BEFORE UPDATE ON roles
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE permissions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code varchar(128) NOT NULL UNIQUE,
    name varchar(100) NOT NULL,
    description text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT permissions_code_format_check
        CHECK (code ~ '^[a-z][a-z0-9_.]*(\.[a-z][a-z0-9_]*)+(:[a-z][a-z0-9_]*)?$'),
    CONSTRAINT permissions_name_length_check
        CHECK (char_length(btrim(name)) BETWEEN 1 AND 100)
);

CREATE TRIGGER permissions_set_updated_at
BEFORE UPDATE ON permissions
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE role_permissions (
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id uuid NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (role_id, permission_id)
);

CREATE INDEX role_permissions_permission_id_idx
    ON role_permissions (permission_id);

CREATE TABLE users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username varchar(32) NOT NULL,
    email varchar(254) NOT NULL,
    password_hash text NOT NULL,
    avatar text,
    nickname varchar(64),
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE RESTRICT,
    status varchar(32) NOT NULL DEFAULT 'active',
    email_verified boolean NOT NULL DEFAULT false,
    email_verified_at timestamptz,
    auth_version integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT users_username_length_check
        CHECK (char_length(username) BETWEEN 3 AND 32),
    CONSTRAINT users_username_trimmed_check
        CHECK (username = btrim(username)),
    CONSTRAINT users_email_normalized_check
        CHECK (email = lower(btrim(email)) AND char_length(email) BETWEEN 3 AND 254),
    CONSTRAINT users_password_hash_check
        CHECK (password_hash LIKE '$argon2id$%'),
    CONSTRAINT users_nickname_check
        CHECK (
            nickname IS NULL
            OR (nickname = btrim(nickname) AND char_length(nickname) BETWEEN 1 AND 64)
        ),
    CONSTRAINT users_status_check
        CHECK (status IN ('active', 'pending', 'suspended', 'disabled')),
    CONSTRAINT users_email_verification_check
        CHECK (
            (email_verified = true AND email_verified_at IS NOT NULL)
            OR (email_verified = false AND email_verified_at IS NULL)
        ),
    CONSTRAINT users_auth_version_check
        CHECK (auth_version >= 0)
);

CREATE UNIQUE INDEX users_username_lower_uidx
    ON users (lower(username));

CREATE UNIQUE INDEX users_email_lower_uidx
    ON users (lower(email));

CREATE INDEX users_role_id_idx
    ON users (role_id);

CREATE INDEX users_non_active_status_idx
    ON users (status)
    WHERE status <> 'active';

CREATE TRIGGER users_set_updated_at
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE refresh_tokens (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id uuid NOT NULL,
    token_hash bytea NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_used_at timestamptz,
    revoked_at timestamptz,
    revocation_reason varchar(64),
    replaced_by_id uuid REFERENCES refresh_tokens(id) ON DELETE SET NULL,
    created_by_ip inet,
    user_agent varchar(512),
    CONSTRAINT refresh_tokens_hash_length_check
        CHECK (octet_length(token_hash) = 32),
    CONSTRAINT refresh_tokens_expiry_check
        CHECK (expires_at > created_at),
    CONSTRAINT refresh_tokens_last_used_check
        CHECK (last_used_at IS NULL OR last_used_at >= created_at),
    CONSTRAINT refresh_tokens_revocation_check
        CHECK (revoked_at IS NOT NULL OR revocation_reason IS NULL),
    CONSTRAINT refresh_tokens_replacement_check
        CHECK (replaced_by_id IS NULL OR revoked_at IS NOT NULL),
    CONSTRAINT refresh_tokens_not_self_replaced_check
        CHECK (replaced_by_id IS NULL OR replaced_by_id <> id)
);

CREATE INDEX refresh_tokens_user_created_idx
    ON refresh_tokens (user_id, created_at DESC);

CREATE INDEX refresh_tokens_family_created_idx
    ON refresh_tokens (family_id, created_at);

CREATE INDEX refresh_tokens_active_expiry_idx
    ON refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;

CREATE UNIQUE INDEX refresh_tokens_replaced_by_uidx
    ON refresh_tokens (replaced_by_id)
    WHERE replaced_by_id IS NOT NULL;

INSERT INTO roles (code, name, description, priority, is_system)
VALUES
    ('user', 'User', 'Standard authenticated user', 10, true),
    ('moderator', 'Moderator', 'Community moderation role', 20, true),
    ('administrator', 'Administrator', 'Application administration role', 30, true),
    ('super_administrator', 'Super Administrator', 'Unrestricted system administration role', 40, true);

INSERT INTO permissions (code, name, description)
VALUES
    ('user.profile.read:self', 'Read own profile', 'Read the authenticated user profile'),
    ('user.profile.update:self', 'Update own profile', 'Update the authenticated user profile'),
    ('user.role.assign', 'Assign user roles', 'Assign roles below the acting principal priority'),
    ('user.status.manage', 'Manage user status', 'Suspend, reactivate, or disable user accounts'),
    ('rbac.manage', 'Manage RBAC', 'Manage roles and permission assignments');

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE
    roles.code = 'super_administrator'
    OR permissions.code IN ('user.profile.read:self', 'user.profile.update:self')
    OR (
        roles.code IN ('moderator', 'administrator')
        AND permissions.code = 'user.status.manage'
    )
    OR (
        roles.code = 'administrator'
        AND permissions.code = 'user.role.assign'
    );
