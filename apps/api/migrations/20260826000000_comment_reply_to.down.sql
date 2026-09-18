-- Restore the original parent-rule trigger (no reply_to semantics).
DROP TRIGGER IF EXISTS comments_enforce_parent_rules ON comments;

CREATE OR REPLACE FUNCTION enforce_comment_parent_rules()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    parent_topic uuid;
    parent_parent uuid;
    parent_status text;
BEGIN
    IF NEW.parent_id IS NULL THEN
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

    RETURN NEW;
END;
$$;

CREATE TRIGGER comments_enforce_parent_rules
BEFORE INSERT OR UPDATE OF parent_id, topic_id ON comments
FOR EACH ROW
EXECUTE FUNCTION enforce_comment_parent_rules();

ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_reply_to_check;
DROP INDEX IF EXISTS comments_reply_to_idx;
ALTER TABLE comments DROP COLUMN IF EXISTS reply_to_comment_id;
