DELETE FROM role_permissions
WHERE permission_id IN (
    SELECT id FROM permissions
    WHERE code IN ('topic.like', 'comment.like', 'topic.favorite', 'user.follow')
);

DELETE FROM permissions
WHERE code IN ('topic.like', 'comment.like', 'topic.favorite', 'user.follow');

DROP TABLE IF EXISTS user_follows;
DROP TABLE IF EXISTS favorites;
DROP TABLE IF EXISTS comment_likes;
DROP TABLE IF EXISTS topic_likes;

DROP TRIGGER IF EXISTS users_set_updated_at ON users;

CREATE TRIGGER users_set_updated_at
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

DROP FUNCTION IF EXISTS set_user_updated_at();

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_follow_counters_check;

ALTER TABLE users
    DROP COLUMN IF EXISTS following_count,
    DROP COLUMN IF EXISTS followers_count;
