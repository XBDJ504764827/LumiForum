-- =============================================================================
-- Phase 14: Poll system — polls attach to topics (1:1), options, votes.
-- =============================================================================

CREATE TABLE polls (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    topic_id uuid NOT NULL UNIQUE REFERENCES topics(id) ON DELETE CASCADE,
    -- Denormalized author snapshot: notification + author checks without a join.
    author_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title varchar(200) NOT NULL,
    description text,
    -- Extension point: 'standard' today, future types (ranked, quiz) append here.
    poll_type varchar(16) NOT NULL DEFAULT 'standard',
    status varchar(16) NOT NULL DEFAULT 'active',
    multiple_choice boolean NOT NULL DEFAULT false,
    anonymous boolean NOT NULL DEFAULT false,
    max_choices integer NOT NULL DEFAULT 1,
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT polls_title_check
        CHECK (title = btrim(title) AND char_length(title) BETWEEN 1 AND 200),
    CONSTRAINT polls_description_check
        CHECK (
            description IS NULL
            OR (description = btrim(description) AND char_length(description) <= 2000)
        ),
    CONSTRAINT polls_poll_type_check
        CHECK (poll_type IN ('standard')),
    CONSTRAINT polls_status_check
        CHECK (status IN ('active', 'closed')),
    CONSTRAINT polls_max_choices_check
        CHECK (max_choices BETWEEN 1 AND 20),
    -- Single-choice polls always have max_choices = 1.
    CONSTRAINT polls_single_choice_max_check
        CHECK (multiple_choice = true OR max_choices = 1)
);

CREATE TRIGGER polls_set_updated_at
BEFORE UPDATE ON polls
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE INDEX polls_author_id_idx ON polls (author_id);
CREATE INDEX polls_status_expires_idx ON polls (status, expires_at);

CREATE TABLE poll_options (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    poll_id uuid NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    content varchar(500) NOT NULL,
    sort_order integer NOT NULL DEFAULT 0,
    vote_count integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT poll_options_content_check
        CHECK (content = btrim(content) AND char_length(content) BETWEEN 1 AND 500),
    CONSTRAINT poll_options_sort_order_check
        CHECK (sort_order BETWEEN 0 AND 1000),
    UNIQUE (poll_id, sort_order)
);

CREATE INDEX poll_options_poll_id_idx ON poll_options (poll_id, sort_order);

CREATE TABLE poll_votes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    poll_id uuid NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    option_id uuid NOT NULL REFERENCES poll_options(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    -- Backstop against duplicate votes (single choice additionally serialized by
    -- the service via SELECT ... FOR UPDATE on the poll row).
    CONSTRAINT poll_votes_unique_vote UNIQUE (poll_id, user_id, option_id)
);

CREATE INDEX poll_votes_poll_id_idx ON poll_votes (poll_id);
CREATE INDEX poll_votes_option_id_idx ON poll_votes (option_id);
CREATE INDEX poll_votes_user_id_idx ON poll_votes (user_id);

-- ---------------------------------------------------------------------------
-- Permissions
-- ---------------------------------------------------------------------------

INSERT INTO permissions (code, name, description)
VALUES
    ('poll.vote', 'Vote on polls', 'Cast and cancel votes on poll topics'),
    ('poll.manage', 'Manage polls', 'Close or delete any poll regardless of author')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE
    roles.code IN ('moderator', 'administrator', 'super_administrator')
    AND permissions.code IN ('poll.vote', 'poll.manage')
   OR permissions.code = 'poll.vote';
