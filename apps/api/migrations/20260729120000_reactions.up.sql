-- Phase 5: content interaction system (likes, favorites, follows).
-- Relation tables are the source of truth; denormalized counters stay on parents.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS followers_count bigint NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS following_count bigint NOT NULL DEFAULT 0;

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_follow_counters_check;

ALTER TABLE users
    ADD CONSTRAINT users_follow_counters_check
        CHECK (followers_count >= 0 AND following_count >= 0);

-- Ignore counter-only updates so follow actions do not bump profile updated_at.
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

DROP TRIGGER IF EXISTS users_set_updated_at ON users;

CREATE TRIGGER users_set_updated_at
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION set_user_updated_at();

CREATE TABLE topic_likes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    topic_id uuid NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT topic_likes_topic_user_uidx UNIQUE (topic_id, user_id)
);

CREATE INDEX topic_likes_user_created_idx
    ON topic_likes (user_id, created_at DESC, id DESC);

CREATE INDEX topic_likes_topic_created_idx
    ON topic_likes (topic_id, created_at DESC, id DESC);

CREATE TABLE comment_likes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    comment_id uuid NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT comment_likes_comment_user_uidx UNIQUE (comment_id, user_id)
);

CREATE INDEX comment_likes_user_created_idx
    ON comment_likes (user_id, created_at DESC, id DESC);

CREATE INDEX comment_likes_comment_created_idx
    ON comment_likes (comment_id, created_at DESC, id DESC);

CREATE TABLE favorites (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    topic_id uuid NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT favorites_topic_user_uidx UNIQUE (topic_id, user_id)
);

CREATE INDEX favorites_user_created_idx
    ON favorites (user_id, created_at DESC, id DESC);

CREATE INDEX favorites_topic_created_idx
    ON favorites (topic_id, created_at DESC, id DESC);

CREATE TABLE user_follows (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    follower_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT user_follows_pair_uidx UNIQUE (follower_id, following_id),
    CONSTRAINT user_follows_no_self_check CHECK (follower_id <> following_id)
);

CREATE INDEX user_follows_following_created_idx
    ON user_follows (following_id, created_at DESC, id DESC);

CREATE INDEX user_follows_follower_created_idx
    ON user_follows (follower_id, created_at DESC, id DESC);

INSERT INTO permissions (code, name, description)
VALUES
    ('topic.like', 'Like topics', 'Like and unlike published topics'),
    ('comment.like', 'Like comments', 'Like and unlike published comments'),
    ('topic.favorite', 'Favorite topics', 'Favorite and unfavorite published topics'),
    ('user.follow', 'Follow users', 'Follow and unfollow other users')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE
    permissions.code IN (
        'topic.like',
        'comment.like',
        'topic.favorite',
        'user.follow'
    )
    AND (
        roles.code = 'super_administrator'
        OR roles.code IN ('user', 'moderator', 'administrator')
    )
ON CONFLICT (role_id, permission_id) DO NOTHING;
