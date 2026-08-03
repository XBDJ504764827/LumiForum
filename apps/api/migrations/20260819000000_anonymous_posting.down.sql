DELETE FROM categories WHERE slug = 'player-reports';

ALTER TABLE topics
    DROP COLUMN IF EXISTS is_anonymous;

ALTER TABLE categories
    DROP COLUMN IF EXISTS allow_anonymous;
