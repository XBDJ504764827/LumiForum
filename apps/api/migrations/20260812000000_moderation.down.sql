-- Phase 13: community moderation system (rollback).
-- WARNING: destroys all governance data. Reversible only if run before
-- governance records exist or when data loss is acceptable.

DROP TABLE IF EXISTS moderation_notes;
DROP TABLE IF EXISTS moderation_rule_hits;
DROP TABLE IF EXISTS moderation_rules;
DROP TABLE IF EXISTS appeal_events;
DROP TABLE IF EXISTS appeals;
DROP TABLE IF EXISTS user_sanctions;
DROP TABLE IF EXISTS content_snapshots;
DROP TABLE IF EXISTS moderation_actions;
DROP TABLE IF EXISTS report_events;

ALTER TABLE reports DROP COLUMN IF EXISTS case_id;
DROP TABLE IF EXISTS moderation_cases;

ALTER TABLE reports DROP COLUMN IF EXISTS cancelled_at;
ALTER TABLE reports DROP COLUMN IF EXISTS duplicate_of;
ALTER TABLE reports DROP COLUMN IF EXISTS risk_score;
ALTER TABLE reports DROP COLUMN IF EXISTS priority;

ALTER TABLE reports DROP CONSTRAINT IF EXISTS reports_status_check;
ALTER TABLE reports ADD CONSTRAINT reports_status_check
    CHECK (status IN ('open', 'reviewing', 'resolved', 'rejected'));
ALTER TABLE reports DROP CONSTRAINT IF EXISTS reports_handled_check;
ALTER TABLE reports ADD CONSTRAINT reports_handled_check
    CHECK (
        (status IN ('resolved', 'rejected') AND handler_id IS NOT NULL AND handled_at IS NOT NULL)
        OR (status IN ('open', 'reviewing') AND handled_at IS NULL)
    );

ALTER TABLE topics DROP COLUMN IF EXISTS restrict_interactions;
ALTER TABLE topics DROP COLUMN IF EXISTS is_sensitive;
ALTER TABLE topics DROP COLUMN IF EXISTS is_locked;

ALTER TABLE comments DROP COLUMN IF EXISTS replies_locked;
ALTER TABLE comments DROP COLUMN IF EXISTS is_sensitive;
ALTER TABLE comments DROP COLUMN IF EXISTS is_collapsed;
ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_status_check;
ALTER TABLE comments ADD CONSTRAINT comments_status_check
    CHECK (status IN ('published', 'deleted'));
ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_soft_delete_check;
ALTER TABLE comments ADD CONSTRAINT comments_soft_delete_check
    CHECK (
        (status = 'published' AND deleted_at IS NULL)
        OR (status = 'deleted' AND deleted_at IS NOT NULL)
    );

DROP INDEX IF EXISTS notifications_dedup_idx;
ALTER TABLE notifications DROP CONSTRAINT IF EXISTS notifications_type_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_type_check
    CHECK (
        type IN (
            'post_liked',
            'comment_liked',
            'comment_created',
            'comment_replied',
            'topic_favorited',
            'user_followed',
            'mentioned',
            'system_message'
        )
    );

-- RBAC: remove moderation permissions and senior_moderator role.
DELETE FROM role_permissions
WHERE permission_id IN (
    SELECT id FROM permissions WHERE code LIKE 'moderation.%'
);
DELETE FROM permissions WHERE code LIKE 'moderation.%';
DELETE FROM role_permissions
WHERE role_id = (SELECT id FROM roles WHERE code = 'senior_moderator');
DELETE FROM roles WHERE code = 'senior_moderator';
