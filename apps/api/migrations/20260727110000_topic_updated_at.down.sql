DROP TRIGGER IF EXISTS topics_set_updated_at ON topics;
DROP FUNCTION IF EXISTS set_topic_updated_at();

CREATE TRIGGER topics_set_updated_at
BEFORE UPDATE ON topics
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();
