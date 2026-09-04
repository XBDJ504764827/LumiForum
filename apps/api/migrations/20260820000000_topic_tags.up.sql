-- Topic tags: a normalized tag table plus a many-to-many join with topics.
-- Tags are lowercase, unique, and limited to 32 chars; a topic can carry up to
-- 5 tags. The join is a simple composite primary key.

CREATE TABLE tags (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name varchar(32) NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tags_name_check
        CHECK (
            name = btrim(name)
            AND name = lower(name)
            AND char_length(name) BETWEEN 1 AND 32
            AND name ~ '^[a-z0-9][a-z0-9_-]*$'
        )
);

CREATE TABLE topic_tags (
    topic_id uuid NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    tag_id uuid NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (topic_id, tag_id)
);

CREATE INDEX topic_tags_tag_id_idx ON topic_tags (tag_id);
