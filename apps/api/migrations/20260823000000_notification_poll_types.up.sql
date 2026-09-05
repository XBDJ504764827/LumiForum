-- The moderation migration (20260812000000) rewrote notifications_type_check
-- before the poll phase landed; poll notifications ('poll_voted',
-- 'poll_ended') therefore violate the constraint and never persist.
-- Re-align the constraint with NotificationType::as_str.

ALTER TABLE notifications DROP CONSTRAINT IF EXISTS notifications_type_check;
ALTER TABLE notifications
    ADD CONSTRAINT notifications_type_check
        CHECK (
            type IN (
                'post_liked',
                'comment_liked',
                'comment_created',
                'comment_replied',
                'topic_favorited',
                'user_followed',
                'mentioned',
                'system_message',
                'report_submitted',
                'report_processed',
                'content_hidden',
                'content_deleted',
                'topic_locked',
                'user_warned',
                'user_muted',
                'user_banned',
                'sanction_expiring',
                'sanction_revoked',
                'appeal_submitted',
                'appeal_approved',
                'appeal_rejected',
                'moderation_inbox',
                'poll_voted',
                'poll_ended'
            )
        );