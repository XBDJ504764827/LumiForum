-- =============================================================================
-- Phase 16: moderation enhancements — pending review states, violation score,
-- user reputation.
-- =============================================================================

-- 1) Violation score & reputation on users
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS moderation_score integer NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS reputation varchar(16) NOT NULL DEFAULT 'normal';

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_moderation_score_check,
    DROP CONSTRAINT IF EXISTS users_reputation_check;
ALTER TABLE users
    ADD CONSTRAINT users_moderation_score_check CHECK (moderation_score >= 0),
    ADD CONSTRAINT users_reputation_check
        CHECK (reputation IN ('normal', 'watch', 'restricted'));

-- 2) Topic status gains pending_review (auto-flagged content awaiting review)
ALTER TABLE topics
    DROP CONSTRAINT IF EXISTS topics_status_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_status_check
        CHECK (status IN ('published', 'hidden', 'deleted', 'pending_review'));

ALTER TABLE comments
    DROP CONSTRAINT IF EXISTS comments_status_check;
ALTER TABLE comments
    ADD CONSTRAINT comments_status_check
        CHECK (status IN ('published', 'hidden', 'deleted', 'pending_review'));

-- 3) Reports may target files (reports.target_type is a free varchar, no DDL
--    change needed; the service validates the target exists).

-- 4) Soft-delete invariant must accept pending_review (content awaiting review
--    is live, not deleted).
ALTER TABLE topics
    DROP CONSTRAINT IF EXISTS topics_soft_delete_check;
ALTER TABLE topics
    ADD CONSTRAINT topics_soft_delete_check
        CHECK (
            (status = 'deleted' AND deleted_at IS NOT NULL)
            OR (status IN ('published', 'hidden', 'pending_review') AND deleted_at IS NULL)
        );

ALTER TABLE comments
    DROP CONSTRAINT IF EXISTS comments_soft_delete_check;
ALTER TABLE comments
    ADD CONSTRAINT comments_soft_delete_check
        CHECK (
            (status = 'deleted' AND deleted_at IS NOT NULL)
            OR (status IN ('published', 'hidden', 'pending_review') AND deleted_at IS NULL)
        );
