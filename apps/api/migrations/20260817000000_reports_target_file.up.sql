-- Phase 16.1: reports may target files.
ALTER TABLE reports
    DROP CONSTRAINT IF EXISTS reports_target_type_check;
ALTER TABLE reports
    ADD CONSTRAINT reports_target_type_check
        CHECK (target_type IN ('topic', 'comment', 'user', 'file'));

ALTER TABLE moderation_cases
    DROP CONSTRAINT IF EXISTS moderation_cases_target_type_check;
ALTER TABLE moderation_cases
    ADD CONSTRAINT moderation_cases_target_type_check
        CHECK (target_type IN ('topic', 'comment', 'user', 'file'));
