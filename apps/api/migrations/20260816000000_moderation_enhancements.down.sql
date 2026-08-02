ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_moderation_score_check,
    DROP CONSTRAINT IF EXISTS users_reputation_check;
ALTER TABLE users
    DROP COLUMN IF EXISTS moderation_score,
    DROP COLUMN IF EXISTS reputation;

ALTER TABLE topics
    DROP CONSTRAINT IF EXISTS topics_status_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_status_check
        CHECK (status IN ('published', 'hidden', 'deleted'));

ALTER TABLE comments
    DROP CONSTRAINT IF EXISTS comments_status_check;
ALTER TABLE comments
    ADD CONSTRAINT comments_status_check
        CHECK (status IN ('published', 'hidden', 'deleted'));

ALTER TABLE topics
    DROP CONSTRAINT IF EXISTS topics_soft_delete_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_soft_delete_check
        CHECK (
            (status = 'deleted' AND deleted_at IS NOT NULL)
            OR (status IN ('published', 'hidden') AND deleted_at IS NULL)
        );

ALTER TABLE comments
    DROP CONSTRAINT IF EXISTS comments_soft_delete_check;
ALTER TABLE comments
    ADD CONSTRAINT comments_soft_delete_check
        CHECK (
            (status = 'deleted' AND deleted_at IS NOT NULL)
            OR (status IN ('published', 'hidden') AND deleted_at IS NULL)
        );
