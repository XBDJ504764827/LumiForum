DROP INDEX IF EXISTS users_nickname_trgm_idx;
DROP INDEX IF EXISTS users_username_trgm_idx;
DROP INDEX IF EXISTS comments_content_trgm_idx;
DROP INDEX IF EXISTS topics_summary_trgm_idx;
DROP INDEX IF EXISTS topics_title_trgm_idx;
DROP INDEX IF EXISTS users_search_vector_idx;
DROP INDEX IF EXISTS comments_search_vector_idx;
DROP INDEX IF EXISTS topics_search_vector_idx;

DROP TRIGGER IF EXISTS users_search_vector_trigger ON users;
DROP TRIGGER IF EXISTS comments_search_vector_trigger ON comments;
DROP TRIGGER IF EXISTS topics_search_vector_trigger ON topics;

DROP FUNCTION IF EXISTS users_refresh_search_vector();
DROP FUNCTION IF EXISTS comments_refresh_search_vector();
DROP FUNCTION IF EXISTS topics_refresh_search_vector();
DROP FUNCTION IF EXISTS users_search_vector(text, text);
DROP FUNCTION IF EXISTS comments_search_vector(text);
DROP FUNCTION IF EXISTS topics_search_vector(text, text, text);

ALTER TABLE users DROP COLUMN IF EXISTS search_vector;
ALTER TABLE comments DROP COLUMN IF EXISTS search_vector;
ALTER TABLE topics DROP COLUMN IF EXISTS search_vector;
