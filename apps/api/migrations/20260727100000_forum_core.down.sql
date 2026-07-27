DROP TABLE IF EXISTS topics;
DROP TABLE IF EXISTS categories;

DELETE FROM permissions
WHERE code IN (
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
);
