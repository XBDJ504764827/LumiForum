-- Phase 16.3: anonymous posting + player report category.

-- Categories may opt into anonymous posting (e.g. player report board).
ALTER TABLE categories
    ADD COLUMN IF NOT EXISTS allow_anonymous boolean NOT NULL DEFAULT false;

-- Topics carry an anonymity flag; the author display is masked at read time.
ALTER TABLE topics
    ADD COLUMN IF NOT EXISTS is_anonymous boolean NOT NULL DEFAULT false;

-- Player report board: normal users may post, anonymous posting enabled.
INSERT INTO categories (slug, name, description, icon, sort_order, is_visible, restricted_posting, allow_anonymous)
SELECT 'player-reports', '玩家举报', '匿名举报其他玩家的违规行为，保护你的隐私', 'shield-alert', 90, true, false, true
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'player-reports');
