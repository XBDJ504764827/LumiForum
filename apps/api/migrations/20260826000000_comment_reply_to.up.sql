-- Flat two-level replies with explicit @ reference.
-- Storage stays root + direct children (parent_id always points at the root);
-- reply_to_comment_id records which comment in the thread is actually addressed.

ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS reply_to_comment_id uuid REFERENCES comments(id) ON DELETE SET NULL;

-- Roots never carry a reply target; children may optionally reference a
-- sibling (or the root itself, though the API normalizes direct root replies
-- to NULL).
ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_reply_to_check;
ALTER TABLE comments
    ADD CONSTRAINT comments_reply_to_check
        CHECK (
            (parent_id IS NULL AND reply_to_comment_id IS NULL)
            OR (parent_id IS NOT NULL)
        );

CREATE INDEX IF NOT EXISTS comments_reply_to_idx
    ON comments (reply_to_comment_id)
    WHERE reply_to_comment_id IS NOT NULL;

CREATE OR REPLACE FUNCTION enforce_comment_parent_rules()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    parent_topic uuid;
    parent_parent uuid;
    parent_status text;
    target_topic uuid;
    target_parent uuid;
BEGIN
    IF NEW.parent_id IS NULL THEN
        IF NEW.reply_to_comment_id IS NOT NULL THEN
            RAISE EXCEPTION 'root comments cannot reference a reply target';
        END IF;
        RETURN NEW;
    END IF;

    SELECT topic_id, parent_id, status
    INTO parent_topic, parent_parent, parent_status
    FROM comments
    WHERE id = NEW.parent_id
    FOR SHARE;

    IF parent_topic IS NULL THEN
        RAISE EXCEPTION 'comment parent not found';
    END IF;
    IF parent_parent IS NOT NULL THEN
        RAISE EXCEPTION 'comment nesting deeper than 2 levels is not allowed';
    END IF;
    IF parent_topic <> NEW.topic_id THEN
        RAISE EXCEPTION 'comment parent must belong to the same topic';
    END IF;
    IF TG_OP = 'INSERT' AND parent_status <> 'published' THEN
        RAISE EXCEPTION 'cannot reply to a deleted comment';
    END IF;

    -- reply_to is informational (@ reference) and may point at a soft-deleted
    -- sibling: it must exist, live in the same topic, and belong to the same
    -- thread (the root itself or a direct child of the root).
    IF NEW.reply_to_comment_id IS NOT NULL THEN
        SELECT topic_id, parent_id
        INTO target_topic, target_parent
        FROM comments
        WHERE id = NEW.reply_to_comment_id
        FOR SHARE;

        IF target_topic IS NULL THEN
            RAISE EXCEPTION 'reply target not found';
        END IF;
        IF target_topic <> NEW.topic_id THEN
            RAISE EXCEPTION 'reply target must belong to the same topic';
        END IF;
        IF NOT (NEW.reply_to_comment_id = NEW.parent_id OR target_parent = NEW.parent_id) THEN
            RAISE EXCEPTION 'reply target must be in the same thread';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS comments_enforce_parent_rules ON comments;
CREATE TRIGGER comments_enforce_parent_rules
BEFORE INSERT OR UPDATE OF parent_id, topic_id, reply_to_comment_id ON comments
FOR EACH ROW
EXECUTE FUNCTION enforce_comment_parent_rules();
