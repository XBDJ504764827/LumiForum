-- Phase 13: community moderation system.
-- Reports lifecycle, moderation cases, content actions, sanctions, appeals,
-- auto-moderation rules, governance metrics. Forward-only additive changes.

-- =====================================================================
-- 1. Extend existing tables (additive; no column drops)
-- =====================================================================

-- reports: priority, duplicate merging, risk score, cancellation
ALTER TABLE reports DROP CONSTRAINT IF EXISTS reports_status_check;
ALTER TABLE reports
    ADD CONSTRAINT reports_status_check
        CHECK (status IN ('open', 'reviewing', 'resolved', 'rejected', 'duplicate', 'cancelled'));

ALTER TABLE reports DROP CONSTRAINT IF EXISTS reports_handled_check;
ALTER TABLE reports
    ADD CONSTRAINT reports_handled_check
        CHECK (
            (status IN ('resolved', 'rejected', 'duplicate')
                AND handler_id IS NOT NULL AND handled_at IS NOT NULL)
            OR (status IN ('open', 'reviewing') AND handled_at IS NULL)
            OR (status = 'cancelled' AND handled_at IS NULL)
        );

ALTER TABLE reports
    ADD COLUMN IF NOT EXISTS priority varchar(16) NOT NULL DEFAULT 'normal',
    ADD COLUMN IF NOT EXISTS risk_score integer NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS duplicate_of uuid REFERENCES reports(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS cancelled_at timestamptz;

ALTER TABLE reports DROP CONSTRAINT IF EXISTS reports_priority_check;
ALTER TABLE reports
    ADD CONSTRAINT reports_priority_check
        CHECK (priority IN ('low', 'normal', 'high', 'urgent'));

ALTER TABLE reports DROP CONSTRAINT IF EXISTS reports_risk_score_check;
ALTER TABLE reports
    ADD CONSTRAINT reports_risk_score_check
        CHECK (risk_score >= 0 AND risk_score <= 100);

CREATE INDEX IF NOT EXISTS reports_priority_status_idx
    ON reports (priority, status, created_at DESC);

-- topics: locking, sensitivity, interaction restrictions
ALTER TABLE topics
    ADD COLUMN IF NOT EXISTS is_locked boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS is_sensitive boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS restrict_interactions boolean NOT NULL DEFAULT false;

-- comments: hidden status, collapse, sensitivity, reply restriction
ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_status_check;
ALTER TABLE comments
    ADD CONSTRAINT comments_status_check
        CHECK (status IN ('published', 'hidden', 'deleted'));

ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_soft_delete_check;
ALTER TABLE comments
    ADD CONSTRAINT comments_soft_delete_check
        CHECK (
            (status = 'deleted' AND deleted_at IS NOT NULL)
            OR (status IN ('published', 'hidden') AND deleted_at IS NULL)
        );

ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS is_collapsed boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS is_sensitive boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS replies_locked boolean NOT NULL DEFAULT false;

-- =====================================================================
-- 2. moderation_cases (the central review unit for one target)
-- =====================================================================

CREATE TABLE moderation_cases (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    target_type varchar(32) NOT NULL,
    target_id uuid NOT NULL,
    status varchar(32) NOT NULL DEFAULT 'open',
    priority varchar(16) NOT NULL DEFAULT 'normal',
    risk_score integer NOT NULL DEFAULT 0,
    source varchar(16) NOT NULL DEFAULT 'report',
    assignee_id uuid REFERENCES users(id) ON DELETE SET NULL,
    opened_by uuid REFERENCES users(id) ON DELETE SET NULL,
    opened_at timestamptz NOT NULL DEFAULT now(),
    closed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT moderation_cases_target_type_check
        CHECK (target_type IN ('topic', 'comment', 'user')),
    CONSTRAINT moderation_cases_status_check
        CHECK (status IN ('open', 'reviewing', 'closed')),
    CONSTRAINT moderation_cases_priority_check
        CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    CONSTRAINT moderation_cases_source_check
        CHECK (source IN ('report', 'auto', 'manual')),
    CONSTRAINT moderation_cases_risk_check
        CHECK (risk_score >= 0 AND risk_score <= 100),
    CONSTRAINT moderation_cases_closed_check
        CHECK (
            (status = 'closed' AND closed_at IS NOT NULL)
            OR (status IN ('open', 'reviewing') AND closed_at IS NULL)
        )
);

-- only one open/reviewing case per target at a time
CREATE UNIQUE INDEX moderation_cases_target_open_idx
    ON moderation_cases (target_type, target_id)
    WHERE status IN ('open', 'reviewing');

CREATE INDEX moderation_cases_queue_idx
    ON moderation_cases (status, priority, created_at DESC);

CREATE INDEX moderation_cases_assignee_idx
    ON moderation_cases (assignee_id, status)
    WHERE assignee_id IS NOT NULL;

CREATE INDEX moderation_cases_created_idx
    ON moderation_cases (created_at DESC);

CREATE TRIGGER moderation_cases_set_updated_at
BEFORE UPDATE ON moderation_cases
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- link reports to cases
ALTER TABLE reports
    ADD COLUMN IF NOT EXISTS case_id uuid REFERENCES moderation_cases(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS reports_case_idx ON reports (case_id) WHERE case_id IS NOT NULL;

-- =====================================================================
-- 3. report_events (report lifecycle history)
-- =====================================================================

CREATE TABLE report_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id uuid NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    actor_type varchar(16) NOT NULL,
    actor_id uuid REFERENCES users(id) ON DELETE SET NULL,
    action varchar(32) NOT NULL,
    note text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT report_events_actor_type_check
        CHECK (actor_type IN ('reporter', 'moderator', 'system')),
    CONSTRAINT report_events_action_check
        CHECK (action IN (
            'created', 'assigned', 'released', 'transferred',
            'reviewing', 'resolved', 'rejected', 'duplicated', 'cancelled', 'note'
        )),
    CONSTRAINT report_events_note_check
        CHECK (note IS NULL OR char_length(note) <= 2000)
);

CREATE INDEX report_events_report_created_idx
    ON report_events (report_id, created_at);

-- =====================================================================
-- 4. moderation_actions (governance action history, queryable)
-- =====================================================================

CREATE TABLE moderation_actions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id uuid REFERENCES moderation_cases(id) ON DELETE SET NULL,
    action varchar(48) NOT NULL,
    target_type varchar(32) NOT NULL,
    target_id uuid NOT NULL,
    before_status varchar(64),
    after_status varchar(64),
    reason varchar(500),
    operator_id uuid REFERENCES users(id) ON DELETE SET NULL,
    report_id uuid REFERENCES reports(id) ON DELETE SET NULL,
    sanction_id uuid,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT moderation_actions_target_type_check
        CHECK (target_type IN ('topic', 'comment', 'user', 'case', 'sanction')),
    CONSTRAINT moderation_actions_reason_check
        CHECK (reason IS NULL OR char_length(reason) <= 500)
);

CREATE INDEX moderation_actions_case_idx
    ON moderation_actions (case_id, created_at);

CREATE INDEX moderation_actions_target_idx
    ON moderation_actions (target_type, target_id, created_at DESC);

CREATE INDEX moderation_actions_operator_idx
    ON moderation_actions (operator_id, created_at DESC)
    WHERE operator_id IS NOT NULL;

-- =====================================================================
-- 5. content_snapshots (evidence + restore support; 90-day retention)
-- =====================================================================

CREATE TABLE content_snapshots (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id uuid REFERENCES moderation_cases(id) ON DELETE SET NULL,
    target_type varchar(32) NOT NULL,
    target_id uuid NOT NULL,
    title varchar(200),
    content text,
    summary varchar(500),
    status varchar(32),
    reason varchar(500),
    created_by uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT content_snapshots_target_type_check
        CHECK (target_type IN ('topic', 'comment', 'user'))
);

CREATE INDEX content_snapshots_case_idx
    ON content_snapshots (case_id, created_at);

CREATE INDEX content_snapshots_target_idx
    ON content_snapshots (target_type, target_id, created_at DESC);

-- =====================================================================
-- 6. user_sanctions (penalties; restrictions array avoids extra table)
-- =====================================================================

CREATE TABLE user_sanctions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sanction_type varchar(32) NOT NULL,
    reason varchar(500) NOT NULL,
    user_visible_reason varchar(500),
    internal_note text,
    restrictions text[] NOT NULL DEFAULT '{}',
    starts_at timestamptz NOT NULL DEFAULT now(),
    ends_at timestamptz,
    is_permanent boolean NOT NULL DEFAULT false,
    status varchar(32) NOT NULL DEFAULT 'scheduled',
    issued_by uuid REFERENCES users(id) ON DELETE SET NULL,
    case_id uuid REFERENCES moderation_cases(id) ON DELETE SET NULL,
    report_id uuid REFERENCES reports(id) ON DELETE SET NULL,
    related_content_type varchar(32),
    related_content_id uuid,
    revoked_by uuid REFERENCES users(id) ON DELETE SET NULL,
    revoked_at timestamptz,
    revoke_reason varchar(500),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT user_sanctions_type_check
        CHECK (sanction_type IN (
            'warning', 'content_restriction', 'mute', 'suspension', 'ban'
        )),
    CONSTRAINT user_sanctions_status_check
        CHECK (status IN ('scheduled', 'active', 'expired', 'revoked')),
    CONSTRAINT user_sanctions_restrictions_check
        CHECK (restrictions <@ ARRAY['no_topics', 'no_comments', 'no_reports', 'no_uploads']::text[]),
    CONSTRAINT user_sanctions_period_check
        CHECK (
            (is_permanent AND ends_at IS NULL AND sanction_type = 'ban')
            OR (NOT is_permanent AND ends_at IS NOT NULL)
        ),
    CONSTRAINT user_sanctions_reason_check
        CHECK (char_length(btrim(reason)) BETWEEN 3 AND 500),
    CONSTRAINT user_sanctions_note_check
        CHECK (internal_note IS NULL OR char_length(internal_note) <= 2000),
    CONSTRAINT user_sanctions_revoke_check
        CHECK (
            (status = 'revoked' AND revoked_by IS NOT NULL AND revoked_at IS NOT NULL)
            OR (status <> 'revoked' AND revoked_at IS NULL)
        ),
    CONSTRAINT user_sanctions_no_self_issue_check
        CHECK (issued_by IS NULL OR issued_by <> user_id)
);

CREATE INDEX user_sanctions_user_status_idx
    ON user_sanctions (user_id, status, created_at DESC);

CREATE INDEX user_sanctions_status_ends_idx
    ON user_sanctions (status, ends_at)
    WHERE status IN ('scheduled', 'active');

CREATE INDEX user_sanctions_issued_idx
    ON user_sanctions (issued_by, created_at DESC)
    WHERE issued_by IS NOT NULL;

CREATE INDEX user_sanctions_case_idx
    ON user_sanctions (case_id)
    WHERE case_id IS NOT NULL;

CREATE TRIGGER user_sanctions_set_updated_at
BEFORE UPDATE ON user_sanctions
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- =====================================================================
-- 7. appeals + appeal_events
-- =====================================================================

CREATE TABLE appeals (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    appeal_type varchar(16) NOT NULL,
    sanction_id uuid REFERENCES user_sanctions(id) ON DELETE SET NULL,
    content_type varchar(32),
    content_id uuid,
    reason varchar(2000) NOT NULL,
    details text,
    evidence jsonb NOT NULL DEFAULT '[]'::jsonb,
    status varchar(32) NOT NULL DEFAULT 'pending',
    reviewer_id uuid REFERENCES users(id) ON DELETE SET NULL,
    review_note varchar(1000),
    reviewed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT appeals_type_check
        CHECK (appeal_type IN ('sanction', 'content')),
    CONSTRAINT appeals_target_check
        CHECK (
            (sanction_id IS NOT NULL AND content_type IS NULL AND content_id IS NULL)
            OR (sanction_id IS NULL AND content_type IS NOT NULL AND content_id IS NOT NULL)
        ),
    CONSTRAINT appeals_content_type_check
        CHECK (content_type IS NULL OR content_type IN ('topic', 'comment')),
    CONSTRAINT appeals_status_check
        CHECK (status IN ('pending', 'reviewing', 'approved', 'rejected', 'cancelled')),
    CONSTRAINT appeals_reason_check
        CHECK (char_length(btrim(reason)) BETWEEN 3 AND 2000),
    CONSTRAINT appeals_review_check
        CHECK (
            (status IN ('approved', 'rejected') AND reviewer_id IS NOT NULL AND reviewed_at IS NOT NULL)
            OR (status IN ('pending', 'reviewing') AND reviewed_at IS NULL)
            OR (status = 'cancelled' AND reviewed_at IS NULL)
        ),
    CONSTRAINT appeals_no_self_review_check
        CHECK (reviewer_id IS NULL OR reviewer_id <> user_id)
);

CREATE INDEX appeals_queue_idx
    ON appeals (status, created_at);

CREATE INDEX appeals_user_created_idx
    ON appeals (user_id, created_at DESC);

CREATE INDEX appeals_sanction_idx
    ON appeals (sanction_id)
    WHERE sanction_id IS NOT NULL;

CREATE TRIGGER appeals_set_updated_at
BEFORE UPDATE ON appeals
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE appeal_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    appeal_id uuid NOT NULL REFERENCES appeals(id) ON DELETE CASCADE,
    actor_type varchar(16) NOT NULL,
    actor_id uuid REFERENCES users(id) ON DELETE SET NULL,
    action varchar(32) NOT NULL,
    note text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT appeal_events_actor_type_check
        CHECK (actor_type IN ('user', 'moderator', 'system')),
    CONSTRAINT appeal_events_action_check
        CHECK (action IN (
            'submitted', 'reviewing', 'approved', 'rejected', 'cancelled', 'note'
        )),
    CONSTRAINT appeal_events_note_check
        CHECK (note IS NULL OR char_length(note) <= 2000)
);

CREATE INDEX appeal_events_appeal_created_idx
    ON appeal_events (appeal_id, created_at);

-- =====================================================================
-- 8. moderation_rules + moderation_rule_hits (auto-moderation)
-- =====================================================================

CREATE TABLE moderation_rules (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name varchar(100) NOT NULL,
    rule_type varchar(32) NOT NULL,
    target_type varchar(16) NOT NULL DEFAULT 'all',
    priority integer NOT NULL DEFAULT 0,
    enabled boolean NOT NULL DEFAULT true,
    risk_score integer NOT NULL DEFAULT 5,
    action varchar(16) NOT NULL DEFAULT 'flag',
    config jsonb NOT NULL DEFAULT '{}'::jsonb,
    hit_count bigint NOT NULL DEFAULT 0,
    created_by uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT moderation_rules_type_check
        CHECK (rule_type IN (
            'keyword', 'url_domain', 'rate', 'duplicate', 'new_user', 'high_frequency'
        )),
    CONSTRAINT moderation_rules_target_check
        CHECK (target_type IN ('topic', 'comment', 'user', 'all')),
    CONSTRAINT moderation_rules_action_check
        CHECK (action IN ('allow', 'flag', 'queue', 'collapse', 'hide', 'reject', 'rate_limit')),
    CONSTRAINT moderation_rules_name_check
        CHECK (name = btrim(name) AND char_length(name) BETWEEN 1 AND 100),
    CONSTRAINT moderation_rules_risk_check
        CHECK (risk_score BETWEEN 1 AND 100),
    CONSTRAINT moderation_rules_priority_check
        CHECK (priority >= 0)
);

CREATE INDEX moderation_rules_enabled_idx
    ON moderation_rules (enabled, priority DESC, rule_type);

CREATE TRIGGER moderation_rules_set_updated_at
BEFORE UPDATE ON moderation_rules
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE moderation_rule_hits (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id uuid NOT NULL REFERENCES moderation_rules(id) ON DELETE CASCADE,
    target_type varchar(16) NOT NULL,
    target_id uuid,
    user_id uuid REFERENCES users(id) ON DELETE SET NULL,
    content_snippet varchar(500),
    risk_score integer NOT NULL DEFAULT 0,
    action varchar(16) NOT NULL DEFAULT 'flag',
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT moderation_rule_hits_target_check
        CHECK (target_type IN ('topic', 'comment', 'user', 'all')),
    CONSTRAINT moderation_rule_hits_action_check
        CHECK (action IN ('allow', 'flag', 'queue', 'collapse', 'hide', 'reject', 'rate_limit'))
);

CREATE INDEX moderation_rule_hits_rule_idx
    ON moderation_rule_hits (rule_id, created_at DESC);

CREATE INDEX moderation_rule_hits_target_idx
    ON moderation_rule_hits (target_type, target_id, created_at DESC)
    WHERE target_id IS NOT NULL;

CREATE INDEX moderation_rule_hits_user_idx
    ON moderation_rule_hits (user_id, created_at DESC)
    WHERE user_id IS NOT NULL;

-- =====================================================================
-- 9. moderation_notes (internal notes on cases)
-- =====================================================================

CREATE TABLE moderation_notes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id uuid NOT NULL REFERENCES moderation_cases(id) ON DELETE CASCADE,
    author_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note varchar(2000) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT moderation_notes_note_check
        CHECK (note = btrim(note) AND char_length(note) BETWEEN 1 AND 2000)
);

CREATE INDEX moderation_notes_case_created_idx
    ON moderation_notes (case_id, created_at);

-- =====================================================================
-- 10. notifications: moderation types + dedup index
-- =====================================================================

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
                'moderation_inbox'
            )
        );

-- idempotent notifications: dedup_key identifies a unique logical notification
CREATE UNIQUE INDEX notifications_dedup_idx
    ON notifications (user_id, type, (metadata->>'dedup_key'))
    WHERE metadata->>'dedup_key' IS NOT NULL;

-- =====================================================================
-- 11. RBAC: senior_moderator role + moderation.* permissions
-- =====================================================================

INSERT INTO roles (code, name, description, priority, is_system)
VALUES ('senior_moderator', 'Senior Moderator', 'Advanced community moderation role', 25, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO permissions (code, name, description)
VALUES
    ('moderation.report.read', 'Read reports', 'View the moderation report and case queues'),
    ('moderation.report.review', 'Review reports', 'Resolve, reject, and duplicate reports'),
    ('moderation.report.assign', 'Assign reports', 'Claim, release, and transfer moderation tasks'),
    ('moderation.content.hide', 'Hide content', 'Hide topics and comments from public view'),
    ('moderation.content.restore', 'Restore content', 'Restore hidden or deleted content'),
    ('moderation.content.delete', 'Delete content', 'Soft-delete topics and comments'),
    ('moderation.topic.lock', 'Lock topics', 'Lock and unlock topics'),
    ('moderation.topic.move', 'Move topics', 'Move topics between categories'),
    ('moderation.user.warn', 'Warn users', 'Issue warnings to users'),
    ('moderation.user.mute', 'Mute users', 'Restrict posting for a period'),
    ('moderation.user.suspend', 'Suspend users', 'Temporarily suspend user accounts'),
    ('moderation.user.ban', 'Ban users', 'Permanently ban user accounts'),
    ('moderation.sanction.revoke', 'Revoke sanctions', 'Revoke active sanctions'),
    ('moderation.appeal.read', 'Read appeals', 'View the appeal queue and details'),
    ('moderation.appeal.review', 'Review appeals', 'Approve or reject appeals'),
    ('moderation.rule.manage', 'Manage rules', 'Manage auto-moderation rules, keywords, and domains'),
    ('moderation.audit.read', 'Read audit logs', 'View governance audit logs'),
    ('moderation.metrics.read', 'Read metrics', 'View governance statistics')
ON CONFLICT (code) DO NOTHING;

-- admin.access granted to moderator roles so they can reach /admin/moderation/*.
-- Non-moderation admin routes still enforce their own granular permissions.
INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code = 'admin.access'
  AND roles.code IN ('moderator', 'senior_moderator')
ON CONFLICT DO NOTHING;

-- moderator: day-to-day moderation
INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code IN (
    'moderation.report.read',
    'moderation.report.review',
    'moderation.content.hide',
    'moderation.content.restore',
    'moderation.topic.lock',
    'moderation.user.warn',
    'moderation.user.mute',
    'moderation.appeal.read',
    'moderation.metrics.read'
)
AND roles.code = 'moderator'
ON CONFLICT DO NOTHING;

-- senior moderator: everything a moderator can do plus advanced operations
INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code IN (
    'moderation.report.read',
    'moderation.report.review',
    'moderation.report.assign',
    'moderation.content.hide',
    'moderation.content.restore',
    'moderation.content.delete',
    'moderation.topic.lock',
    'moderation.topic.move',
    'moderation.user.warn',
    'moderation.user.mute',
    'moderation.user.suspend',
    'moderation.sanction.revoke',
    'moderation.appeal.read',
    'moderation.appeal.review',
    'moderation.metrics.read'
)
AND roles.code = 'senior_moderator'
ON CONFLICT DO NOTHING;

-- administrator / super administrator: full governance access
INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE permissions.code LIKE 'moderation.%'
  AND roles.code IN ('administrator', 'super_administrator')
ON CONFLICT DO NOTHING;
