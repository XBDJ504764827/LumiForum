-- Only remove seed categories that still have no topics.
DELETE FROM categories
WHERE slug IN (
    'announcements',
    'general',
    'guides',
    'feedback',
    'off-topic'
)
AND NOT EXISTS (
    SELECT 1
    FROM topics
    WHERE topics.category_id = categories.id
);
