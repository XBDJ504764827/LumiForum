CREATE TABLE categories (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug varchar(64) NOT NULL UNIQUE,
    name varchar(100) NOT NULL,
    description text,
    icon varchar(64),
    sort_order integer NOT NULL DEFAULT 0,
    is_visible boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT categories_slug_format_check
        CHECK (
            char_length(slug) BETWEEN 2 AND 64
            AND slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
        ),
    CONSTRAINT categories_name_check
        CHECK (name = btrim(name) AND char_length(name) BETWEEN 1 AND 100),
    CONSTRAINT categories_description_check
        CHECK (
            description IS NULL
            OR (description = btrim(description) AND char_length(description) <= 2000)
        ),
    CONSTRAINT categories_icon_check
        CHECK (
            icon IS NULL
            OR (icon = btrim(icon) AND char_length(icon) BETWEEN 1 AND 64)
        ),
    CONSTRAINT categories_sort_order_check
        CHECK (sort_order BETWEEN -1000000 AND 1000000)
);

CREATE TRIGGER categories_set_updated_at
BEFORE UPDATE ON categories
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE INDEX categories_public_order_idx
    ON categories (sort_order, name, id)
    WHERE is_visible = true;

CREATE INDEX categories_sort_order_idx
    ON categories (sort_order, name, id);

CREATE TABLE topics (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id uuid NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    author_id uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    title varchar(200) NOT NULL,
    slug varchar(220) NOT NULL UNIQUE,
    content text NOT NULL,
    summary varchar(500),
    status varchar(32) NOT NULL DEFAULT 'published',
    view_count bigint NOT NULL DEFAULT 0,
    reply_count bigint NOT NULL DEFAULT 0,
    like_count bigint NOT NULL DEFAULT 0,
    is_pinned boolean NOT NULL DEFAULT false,
    is_featured boolean NOT NULL DEFAULT false,
    last_reply_at timestamptz,
    deleted_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT topics_title_check
        CHECK (title = btrim(title) AND char_length(title) BETWEEN 3 AND 200),
    CONSTRAINT topics_slug_format_check
        CHECK (
            char_length(slug) BETWEEN 2 AND 220
            AND slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
        ),
    CONSTRAINT topics_content_check
        CHECK (char_length(content) BETWEEN 1 AND 100000),
    CONSTRAINT topics_summary_check
        CHECK (
            summary IS NULL
            OR (summary = btrim(summary) AND char_length(summary) <= 500)
        ),
    CONSTRAINT topics_status_check
        CHECK (status IN ('published', 'deleted')),
    CONSTRAINT topics_soft_delete_check
        CHECK (
            (status = 'published' AND deleted_at IS NULL)
            OR (status = 'deleted' AND deleted_at IS NOT NULL)
        ),
    CONSTRAINT topics_counters_check
        CHECK (view_count >= 0 AND reply_count >= 0 AND like_count >= 0),
    CONSTRAINT topics_last_reply_check
        CHECK (last_reply_at IS NULL OR last_reply_at >= created_at)
);

CREATE TRIGGER topics_set_updated_at
BEFORE UPDATE ON topics
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE INDEX topics_latest_idx
    ON topics (created_at DESC, id DESC)
    WHERE status = 'published';

CREATE INDEX topics_category_latest_idx
    ON topics (category_id, created_at DESC, id DESC)
    WHERE status = 'published';

CREATE INDEX topics_hot_idx
    ON topics (view_count DESC, created_at DESC, id DESC)
    WHERE status = 'published';

CREATE INDEX topics_category_hot_idx
    ON topics (category_id, view_count DESC, created_at DESC, id DESC)
    WHERE status = 'published';

CREATE INDEX topics_featured_idx
    ON topics (created_at DESC, id DESC)
    WHERE status = 'published' AND is_featured = true;

CREATE INDEX topics_pinned_idx
    ON topics (created_at DESC, id DESC)
    WHERE status = 'published' AND is_pinned = true;

CREATE INDEX topics_author_idx
    ON topics (author_id, created_at DESC, id DESC);

CREATE INDEX topics_deleted_at_idx
    ON topics (deleted_at)
    WHERE deleted_at IS NOT NULL;

INSERT INTO permissions (code, name, description)
VALUES
    ('category.read', 'Read categories', 'List and read visible forum categories'),
    ('category.manage', 'Manage categories', 'Create, update, and delete forum categories'),
    ('topic.read', 'Read topics', 'List and read published topics'),
    ('topic.create', 'Create topics', 'Create topics in usable categories'),
    ('topic.update:self', 'Update own topics', 'Update topics authored by the acting user'),
    ('topic.update:any', 'Update any topic', 'Update topics regardless of author'),
    ('topic.delete:self', 'Delete own topics', 'Soft-delete topics authored by the acting user'),
    ('topic.delete:any', 'Delete any topic', 'Soft-delete topics regardless of author'),
    ('topic.pin', 'Pin topics', 'Pin and unpin topics'),
    ('topic.feature', 'Feature topics', 'Feature and unfeature topics')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT roles.id, permissions.id
FROM roles
CROSS JOIN permissions
WHERE
    permissions.code IN (
        'category.read',
        'category.manage',
        'topic.read',
        'topic.create',
        'topic.update:self',
        'topic.update:any',
        'topic.delete:self',
        'topic.delete:any',
        'topic.pin',
        'topic.feature'
    )
    AND (
        roles.code = 'super_administrator'
        OR permissions.code IN ('category.read', 'topic.read')
        OR (
            roles.code IN ('user', 'moderator', 'administrator')
            AND permissions.code IN ('topic.create', 'topic.update:self', 'topic.delete:self')
        )
        OR (
            roles.code IN ('moderator', 'administrator')
            AND permissions.code IN ('topic.update:any', 'topic.delete:any', 'topic.pin', 'topic.feature')
        )
        OR (
            roles.code = 'administrator'
            AND permissions.code = 'category.manage'
        )
    )
ON CONFLICT (role_id, permission_id) DO NOTHING;
