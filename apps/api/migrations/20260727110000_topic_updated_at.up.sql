CREATE OR REPLACE FUNCTION set_topic_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF (
        NEW.category_id,
        NEW.author_id,
        NEW.title,
        NEW.slug,
        NEW.content,
        NEW.summary,
        NEW.status,
        NEW.is_pinned,
        NEW.is_featured,
        NEW.deleted_at
    ) IS DISTINCT FROM (
        OLD.category_id,
        OLD.author_id,
        OLD.title,
        OLD.slug,
        OLD.content,
        OLD.summary,
        OLD.status,
        OLD.is_pinned,
        OLD.is_featured,
        OLD.deleted_at
    ) THEN
        NEW.updated_at = now();
    ELSE
        NEW.updated_at = OLD.updated_at;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER topics_set_updated_at ON topics;

CREATE TRIGGER topics_set_updated_at
BEFORE UPDATE ON topics
FOR EACH ROW
EXECUTE FUNCTION set_topic_updated_at();
