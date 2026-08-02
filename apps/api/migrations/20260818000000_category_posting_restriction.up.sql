-- Phase 16.2: announcements-style categories restrict posting to staff.
ALTER TABLE categories
    ADD COLUMN IF NOT EXISTS restricted_posting boolean NOT NULL DEFAULT false;

-- Announcements: only moderators and above may post there.
UPDATE categories
SET restricted_posting = true
WHERE slug = 'announcements';
