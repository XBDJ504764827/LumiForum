DROP TRIGGER IF EXISTS comments_enforce_parent_rules ON comments;
DROP TRIGGER IF EXISTS comments_set_updated_at ON comments;
DROP FUNCTION IF EXISTS enforce_comment_parent_rules();
DROP FUNCTION IF EXISTS set_comment_updated_at();
DROP TABLE IF EXISTS comments;

ALTER TABLE topics DROP COLUMN IF EXISTS last_reply_user_id;

DELETE FROM permissions
WHERE code IN (
    'comment.create',
    'comment.reply',
    'comment.update:self',
    'comment.update:any',
    'comment.delete:self',
    'comment.delete:any',
    'comment.restore'
);
