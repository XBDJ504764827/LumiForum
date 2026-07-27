-- Phase 7: PostgreSQL full-text search foundation.
-- simple config keeps Latin tokenization usable; pg_trgm covers CJK substring match.
-- Future engines (Elasticsearch/Meilisearch) can replace SearchRepository without API churn.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

ALTER TABLE topics
    ADD COLUMN IF NOT EXISTS search_vector tsvector;

ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS search_vector tsvector;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS search_vector tsvector;

CREATE OR REPLACE FUNCTION topics_search_vector(title text, content text, summary text)
RETURNS tsvector
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT
        setweight(to_tsvector('simple', coalesce(title, '')), 'A')
        || setweight(to_tsvector('simple', coalesce(summary, '')), 'B')
        || setweight(to_tsvector('simple', coalesce(content, '')), 'C');
$$;

CREATE OR REPLACE FUNCTION comments_search_vector(content text)
RETURNS tsvector
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT setweight(to_tsvector('simple', coalesce(content, '')), 'A');
$$;

CREATE OR REPLACE FUNCTION users_search_vector(username text, nickname text)
RETURNS tsvector
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT
        setweight(to_tsvector('simple', coalesce(username, '')), 'A')
        || setweight(to_tsvector('simple', coalesce(nickname, '')), 'B');
$$;

CREATE OR REPLACE FUNCTION topics_refresh_search_vector()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.search_vector := topics_search_vector(NEW.title, NEW.content, NEW.summary);
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION comments_refresh_search_vector()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.search_vector := comments_search_vector(NEW.content);
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION users_refresh_search_vector()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.search_vector := users_search_vector(NEW.username, NEW.nickname);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS topics_search_vector_trigger ON topics;
CREATE TRIGGER topics_search_vector_trigger
BEFORE INSERT OR UPDATE OF title, content, summary ON topics
FOR EACH ROW
EXECUTE FUNCTION topics_refresh_search_vector();

DROP TRIGGER IF EXISTS comments_search_vector_trigger ON comments;
CREATE TRIGGER comments_search_vector_trigger
BEFORE INSERT OR UPDATE OF content ON comments
FOR EACH ROW
EXECUTE FUNCTION comments_refresh_search_vector();

DROP TRIGGER IF EXISTS users_search_vector_trigger ON users;
CREATE TRIGGER users_search_vector_trigger
BEFORE INSERT OR UPDATE OF username, nickname ON users
FOR EACH ROW
EXECUTE FUNCTION users_refresh_search_vector();

UPDATE topics
SET search_vector = topics_search_vector(title, content, summary)
WHERE search_vector IS NULL;

UPDATE comments
SET search_vector = comments_search_vector(content)
WHERE search_vector IS NULL;

UPDATE users
SET search_vector = users_search_vector(username, nickname)
WHERE search_vector IS NULL;

ALTER TABLE topics
    ALTER COLUMN search_vector SET NOT NULL;

ALTER TABLE comments
    ALTER COLUMN search_vector SET NOT NULL;

ALTER TABLE users
    ALTER COLUMN search_vector SET NOT NULL;

CREATE INDEX IF NOT EXISTS topics_search_vector_idx
    ON topics USING GIN (search_vector);

CREATE INDEX IF NOT EXISTS comments_search_vector_idx
    ON comments USING GIN (search_vector);

CREATE INDEX IF NOT EXISTS users_search_vector_idx
    ON users USING GIN (search_vector);

CREATE INDEX IF NOT EXISTS topics_title_trgm_idx
    ON topics USING GIN (title gin_trgm_ops);

CREATE INDEX IF NOT EXISTS topics_summary_trgm_idx
    ON topics USING GIN (summary gin_trgm_ops)
    WHERE summary IS NOT NULL;

CREATE INDEX IF NOT EXISTS comments_content_trgm_idx
    ON comments USING GIN (content gin_trgm_ops);

CREATE INDEX IF NOT EXISTS users_username_trgm_idx
    ON users USING GIN (username gin_trgm_ops);

CREATE INDEX IF NOT EXISTS users_nickname_trgm_idx
    ON users USING GIN (nickname gin_trgm_ops)
    WHERE nickname IS NOT NULL;
